#import "adapter.hpp"
#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#include <errno.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/stat.h>
#include <string.h>
#include <unistd.h>
#include <thread>
#include <atomic>

static std::atomic<bool> g_running(false);
static int g_server_fd = -1;
static char g_socket_path[104] = {0};
static const NSUInteger kMaxRequestBytes = 64 * 1024;
static const unsigned long long kMaxLogBytes = 10ULL * 1024ULL * 1024ULL;
static const NSUInteger kDebugClassLimit = 40;
static const unsigned int kDebugMethodLimit = 30;

static bool writeAll(int fd, const void *buffer, size_t size) {
    const char *cursor = static_cast<const char *>(buffer);
    size_t total = 0;
    while (total < size) {
        ssize_t written = write(fd, cursor + total, size - total);
        if (written < 0) {
            if (errno == EINTR) {
                continue;
            }
            return false;
        }
        if (written == 0) {
            return false;
        }
        total += static_cast<size_t>(written);
    }
    return true;
}

static void logLine(NSString *message) {
    const char *logPath = getenv("MACFRIENDS_LOG_FILE");
    if (!logPath || !message) {
        return;
    }
    NSString *path = [NSString stringWithUTF8String:logPath];
    NSString *line = [NSString stringWithFormat:@"%@ %@\n", [[NSISO8601DateFormatter new] stringFromDate:[NSDate date]], message];
    NSData *data = [line dataUsingEncoding:NSUTF8StringEncoding];
    NSFileManager *manager = [NSFileManager defaultManager];
    NSString *directory = [path stringByDeletingLastPathComponent];
    if (directory.length > 0) {
        [manager createDirectoryAtPath:directory withIntermediateDirectories:YES attributes:nil error:nil];
    }
    NSDictionary *attributes = [manager attributesOfItemAtPath:path error:nil];
    NSNumber *fileSize = attributes[NSFileSize];
    if (fileSize && fileSize.unsignedLongLongValue >= kMaxLogBytes) {
        NSString *backupPath = [path stringByAppendingString:@".1"];
        [manager removeItemAtPath:backupPath error:nil];
        [manager moveItemAtPath:path toPath:backupPath error:nil];
    }
    if (![manager fileExistsAtPath:path]) {
        [data writeToFile:path atomically:YES];
        return;
    }
    NSFileHandle *handle = [NSFileHandle fileHandleForWritingAtPath:path];
    if (!handle) {
        [data writeToFile:path atomically:YES];
        return;
    }
    @try {
        [handle seekToEndOfFile];
        [handle writeData:data];
    } @catch (__unused NSException *exception) {
    } @finally {
        [handle closeFile];
    }
}

static void logErrno(NSString *prefix) {
    NSString *detail = [NSString stringWithUTF8String:strerror(errno)];
    logLine([NSString stringWithFormat:@"%@: %@", prefix ?: @"error", detail ?: @"unknown"]);
}

static bool ensureSocketDirectory(void) {
    NSString *path = [NSString stringWithUTF8String:g_socket_path];
    if (!path || path.length == 0) {
        return false;
    }
    NSString *directory = [path stringByDeletingLastPathComponent];
    if (directory.length == 0) {
        return true;
    }
    NSError *error = nil;
    BOOL ok = [[NSFileManager defaultManager] createDirectoryAtPath:directory withIntermediateDirectories:YES attributes:nil error:&error];
    if (!ok) {
        logLine([NSString stringWithFormat:@"socket directory create failed: %@", error.localizedDescription ?: @"unknown"]);
        return false;
    }
    if (chmod(directory.fileSystemRepresentation, 0700) != 0) {
        logErrno(@"socket directory chmod failed");
        return false;
    }
    return true;
}

static void debugDumpRuntimeMatches(void) {
    const char *filterEnv = getenv("MACFRIENDS_DEBUG_CLASS_FILTER");
    if (!filterEnv || filterEnv[0] == '\0') {
        return;
    }

    NSString *filter = [[NSString stringWithUTF8String:filterEnv] lowercaseString];
    int classCount = objc_getClassList(nullptr, 0);
    if (classCount <= 0) {
        logLine(@"debug runtime dump: no objc classes");
        return;
    }

    NSMutableArray *classes = [NSMutableArray arrayWithCapacity:(NSUInteger)classCount];
    Class *buffer = static_cast<Class *>(calloc((size_t)classCount, sizeof(Class)));
    if (!buffer) {
        logLine(@"debug runtime dump: class buffer alloc failed");
        return;
    }

    classCount = objc_getClassList(buffer, classCount);
    for (int index = 0; index < classCount; ++index) {
        Class candidate = buffer[index];
        if (!candidate) {
            continue;
        }
        NSString *name = [NSString stringWithUTF8String:class_getName(candidate)];
        if (!name || ![[name lowercaseString] containsString:filter]) {
            continue;
        }
        [classes addObject:name];
    }
    free(buffer);

    [classes sortUsingSelector:@selector(compare:)];
    NSUInteger logged = 0;
    for (NSString *className in classes) {
        if (logged >= kDebugClassLimit) {
            logLine([NSString stringWithFormat:@"debug runtime dump truncated at %lu classes", (unsigned long)kDebugClassLimit]);
            break;
        }
        Class cls = NSClassFromString(className);
        if (!cls) {
            continue;
        }

        logLine([NSString stringWithFormat:@"debug class %@", className]);

        unsigned int methodCount = 0;
        Method *methods = class_copyMethodList(cls, &methodCount);
        for (unsigned int methodIndex = 0; methods && methodIndex < methodCount && methodIndex < kDebugMethodLimit; ++methodIndex) {
            SEL selector = method_getName(methods[methodIndex]);
            logLine([NSString stringWithFormat:@"debug method -[%@ %s]", className, sel_getName(selector)]);
        }
        if (methods) {
            free(methods);
        }

        unsigned int classMethodCount = 0;
        Method *classMethods = class_copyMethodList(object_getClass(cls), &classMethodCount);
        for (unsigned int methodIndex = 0; classMethods && methodIndex < classMethodCount && methodIndex < kDebugMethodLimit; ++methodIndex) {
            SEL selector = method_getName(classMethods[methodIndex]);
            logLine([NSString stringWithFormat:@"debug method +[%@ %s]", className, sel_getName(selector)]);
        }
        if (classMethods) {
            free(classMethods);
        }

        logged += 1;
    }
}

static bool shouldStartAgentServer(NSDictionary *adapterManifest) {
    NSDictionary *status = MFBuildAdapterStatus(adapterManifest);
    BOOL fixtureEnabled = [status[@"fixture_enabled"] boolValue];
    BOOL targetSupported = [status[@"target_supported"] boolValue];
    NSString *bundleId = status[@"bundle_id"] == [NSNull null] ? @"<nil>" : status[@"bundle_id"];
    NSString *bundleVersion = status[@"bundle_version"] == [NSNull null] ? @"<nil>" : status[@"bundle_version"];
    NSString *reason = status[@"reason"] == [NSNull null] ? @"" : status[@"reason"];

    if (fixtureEnabled || targetSupported) {
        logLine([NSString stringWithFormat:@"agent activate bundle=%@ version=%@ fixture=%@ target_supported=%@",
                 bundleId,
                 bundleVersion,
                 fixtureEnabled ? @"1" : @"0",
                 targetSupported ? @"1" : @"0"]);
        return true;
    }

    logLine([NSString stringWithFormat:@"agent skip bundle=%@ version=%@ reason=%@",
             bundleId,
             bundleVersion,
             reason.length > 0 ? reason : @"unsupported_process"]);
    return false;
}

static NSData *jsonData(id object) {
    NSError *error = nil;
    NSData *data = [NSJSONSerialization dataWithJSONObject:object options:0 error:&error];
    if (!data) {
        NSString *fallback = @"{\"id\":\"agent\",\"ok\":false,\"error\":\"json_encode_failed\"}\n";
        return [fallback dataUsingEncoding:NSUTF8StringEncoding];
    }
    NSMutableData *buffer = [data mutableCopy];
    [buffer appendData:[@"\n" dataUsingEncoding:NSUTF8StringEncoding]];
    return buffer;
}

static NSDictionary *loadAdapter() {
    const char *adapterPath = getenv("MACFRIENDS_ADAPTER_PATH");
    if (!adapterPath) {
        return nil;
    }
    NSData *data = [NSData dataWithContentsOfFile:[NSString stringWithUTF8String:adapterPath]];
    if (!data) {
        return nil;
    }
    NSError *error = nil;
    id json = [NSJSONSerialization JSONObjectWithData:data options:0 error:&error];
    if (!json || ![json isKindOfClass:[NSDictionary class]]) {
        return nil;
    }
    return (NSDictionary *)json;
}

static NSDictionary *parseLine(const std::string &line) {
    NSData *data = [NSData dataWithBytes:line.data() length:line.size()];
    NSError *error = nil;
    id json = [NSJSONSerialization JSONObjectWithData:data options:0 error:&error];
    if (!json || ![json isKindOfClass:[NSDictionary class]]) {
        return nil;
    }
    return (NSDictionary *)json;
}

static NSDictionary *dispatchRequest(NSDictionary *request) {
    NSString *method = request[@"method"] ?: @"";
    NSDictionary *params = [request[@"params"] isKindOfClass:[NSDictionary class]] ? request[@"params"] : @{};
    NSString *fixtureMode = [[[NSProcessInfo processInfo] environment] objectForKey:@"MACFRIENDS_ENABLE_FIXTURE"];
    BOOL fixtureEnabled = [fixtureMode isEqualToString:@"1"];
    logLine([NSString stringWithFormat:@"request method=%@ fixture=%@", method, fixtureEnabled ? @"1" : @"0"]);
    return MFHandleAdapterRequest(loadAdapter(), method, params, fixtureEnabled);
}

static void respond(int client_fd, NSDictionary *response) {
    NSData *data = jsonData(response);
    if (!writeAll(client_fd, data.bytes, data.length)) {
        logLine(@"socket write failed");
    }
}

static void requestServerStop(void) {
    g_running.store(false);
    if (g_server_fd >= 0) {
        shutdown(g_server_fd, SHUT_RDWR);
        close(g_server_fd);
        g_server_fd = -1;
    }
}

static void serverLoop() {
    unlink(g_socket_path);
    if (!ensureSocketDirectory()) {
        return;
    }
    g_server_fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (g_server_fd < 0) {
        logErrno(@"socket create failed");
        return;
    }

    sockaddr_un addr {};
    addr.sun_family = AF_UNIX;
    if (strlcpy(addr.sun_path, g_socket_path, sizeof(addr.sun_path)) >= sizeof(addr.sun_path)) {
        logLine(@"socket path too long for sockaddr_un");
        close(g_server_fd);
        g_server_fd = -1;
        return;
    }

    if (bind(g_server_fd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) != 0) {
        logErrno(@"socket bind failed");
        close(g_server_fd);
        g_server_fd = -1;
        return;
    }
    if (listen(g_server_fd, 5) != 0) {
        logErrno(@"socket listen failed");
        close(g_server_fd);
        g_server_fd = -1;
        return;
    }
    g_running.store(true);
    logLine([NSString stringWithFormat:@"agent socket ready path=%s", g_socket_path]);

    while (g_running.load()) {
        int client_fd = accept(g_server_fd, nullptr, nullptr);
        if (client_fd < 0) {
            if (!g_running.load()) {
                break;
            }
            continue;
        }
        NSMutableData *buffer = [NSMutableData data];
        char chunk[1024];
        ssize_t read_len = 0;
        while ((read_len = read(client_fd, chunk, sizeof(chunk))) > 0) {
            [buffer appendBytes:chunk length:(NSUInteger)read_len];
            if (buffer.length > kMaxRequestBytes) {
                logLine(@"request_too_large");
                respond(client_fd, @{ @"id": @"agent", @"ok": @NO, @"error": @"request_too_large" });
                close(client_fd);
                client_fd = -1;
                break;
            }
            if (memchr(chunk, '\n', (size_t)read_len) != nullptr) {
                break;
            }
        }
        if (client_fd < 0) {
            continue;
        }
        NSString *line = [[NSString alloc] initWithData:buffer encoding:NSUTF8StringEncoding];
        NSDictionary *request = parseLine(std::string([[line stringByTrimmingCharactersInSet:[NSCharacterSet newlineCharacterSet]] UTF8String] ?: ""));
        if (!request) {
            logLine(@"invalid_json_request");
            respond(client_fd, @{ @"id": @"agent", @"ok": @NO, @"error": @"invalid_json_request" });
            close(client_fd);
            continue;
        }
        NSString *method = request[@"method"] ?: @"";
        NSDictionary *response = dispatchRequest(request);
        respond(client_fd, response);
        close(client_fd);
        if ([method isEqualToString:@"stop"]) {
            logLine(@"stop requested");
            requestServerStop();
        }
    }

    if (g_server_fd >= 0) {
        close(g_server_fd);
        g_server_fd = -1;
    }
    unlink(g_socket_path);
    logLine(@"agent socket stopped");
}

__attribute__((constructor)) static void macfriends_agent_init() {
    @autoreleasepool {
        const char *socketEnv = getenv("MACFRIENDS_AGENT_SOCKET");
        if (!socketEnv) {
            logLine(@"MACFRIENDS_AGENT_SOCKET missing");
            return;
        }
        size_t socketPathLen = strnlen(socketEnv, sizeof(g_socket_path));
        if (socketPathLen == 0) {
            logLine(@"socket path empty");
            return;
        }
        if (socketPathLen >= sizeof(g_socket_path)) {
            logLine(@"socket path too long");
            return;
        }
        NSDictionary *adapterManifest = loadAdapter();
        if (!shouldStartAgentServer(adapterManifest)) {
            return;
        }
        strlcpy(g_socket_path, socketEnv, sizeof(g_socket_path));
        logLine([NSString stringWithFormat:@"agent init path=%s", g_socket_path]);
        debugDumpRuntimeMatches();
        std::thread(serverLoop).detach();
    }
}
