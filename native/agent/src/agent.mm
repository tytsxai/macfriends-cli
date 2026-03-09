#import "adapter.hpp"
#import <Foundation/Foundation.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
#include <thread>
#include <atomic>

static std::atomic<bool> g_running(false);
static int g_server_fd = -1;
static char g_socket_path[104] = {0};

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
    write(client_fd, data.bytes, data.length);
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
    g_server_fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (g_server_fd < 0) {
        logLine(@"socket create failed");
        return;
    }

    sockaddr_un addr {};
    addr.sun_family = AF_UNIX;
    strlcpy(addr.sun_path, g_socket_path, sizeof(addr.sun_path));

    if (bind(g_server_fd, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) != 0) {
        logLine(@"socket bind failed");
        close(g_server_fd);
        g_server_fd = -1;
        return;
    }
    if (listen(g_server_fd, 5) != 0) {
        logLine(@"socket listen failed");
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
            if (memchr(chunk, '\n', (size_t)read_len) != nullptr) {
                break;
            }
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
        strlcpy(g_socket_path, socketEnv, sizeof(g_socket_path));
        if (g_socket_path[0] == '\0') {
            logLine(@"socket path empty");
            return;
        }
        logLine([NSString stringWithFormat:@"agent init path=%s", g_socket_path]);
        std::thread(serverLoop).detach();
    }
}
