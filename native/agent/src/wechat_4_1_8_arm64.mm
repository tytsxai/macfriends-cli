#import "adapter.hpp"
#import <Foundation/Foundation.h>

static NSString *const kExpectedBundleId = @"com.tencent.xinWeChat";
static NSString *const kExpectedVersion = @"4.1.8";
static NSString *const kExpectedArch = @"arm64";
static NSString *const kAdapterName = @"wechat_4_1_8_arm64";

static NSDictionary *okResult(id result) {
    return @{ @"id": @"agent", @"ok": @YES, @"result": result ?: @{} };
}

static NSDictionary *errorResult(NSString *code) {
    return @{ @"id": @"agent", @"ok": @NO, @"error": code ?: @"unknown_error" };
}

static NSDictionary *primitiveResolution(BOOL fixtureEnabled, BOOL supported) {
    if (fixtureEnabled) {
        return @{ @"profile": @"fixture", @"contacts": @"fixture", @"scan": @"fixture" };
    }
    if (!supported) {
        return @{ @"profile": @"blocked", @"contacts": @"blocked", @"scan": @"blocked" };
    }
    return @{ @"profile": @"unresolved", @"contacts": @"unresolved", @"scan": @"unresolved" };
}

static NSString *currentArch() {
#if defined(__aarch64__) || defined(__arm64__)
    return @"arm64";
#else
    return @"unknown";
#endif
}

static NSDictionary *loadInfo(void) {
    NSBundle *bundle = [NSBundle mainBundle];
    NSDictionary *info = bundle.infoDictionary ?: @{};
    return @{
        @"bundle_id": info[@"CFBundleIdentifier"] ?: [NSNull null],
        @"bundle_version": info[@"CFBundleShortVersionString"] ?: [NSNull null],
        @"arch": currentArch()
    };
}

static NSDictionary *probeTarget(NSDictionary *adapterManifest) {
    NSDictionary *info = loadInfo();
    NSString *bundleId = info[@"bundle_id"] == [NSNull null] ? nil : info[@"bundle_id"];
    NSString *bundleVersion = info[@"bundle_version"] == [NSNull null] ? nil : info[@"bundle_version"];
    NSString *arch = info[@"arch"];

    NSString *manifestBundleId = adapterManifest[@"bundle_id"] ?: kExpectedBundleId;
    NSString *manifestVersion = adapterManifest[@"build_target"] ?: kExpectedVersion;
    NSString *manifestArch = adapterManifest[@"arch"] ?: kExpectedArch;

    BOOL bundleMatch = bundleId && [bundleId isEqualToString:manifestBundleId];
    BOOL versionMatch = bundleVersion && [bundleVersion isEqualToString:manifestVersion];
    BOOL archMatch = arch && [arch isEqualToString:manifestArch];
    BOOL supported = bundleMatch && versionMatch && archMatch;

    NSString *reason = nil;
    if (!bundleMatch) {
        reason = @"version_mismatch";
    } else if (!versionMatch) {
        reason = @"version_mismatch";
    } else if (!archMatch) {
        reason = @"adapter_not_loaded";
    }

    return @{
        @"bundle_id": bundleId ?: [NSNull null],
        @"bundle_version": bundleVersion ?: [NSNull null],
        @"target_supported": @(supported),
        @"adapter_name": kAdapterName,
        @"reason": reason ?: [NSNull null]
    };
}

static NSDictionary *mockProfile() {
    return @{
        @"wxid": @"wxid_mock_macfriends",
        @"nickname": @"Mock User",
        @"remark": [NSNull null],
        @"signature": @"fixture-mode"
    };
}

static NSArray *mockContacts() {
    return @[
        @{ @"wxid": @"wxid_a", @"nickname": @"Alice", @"remark": @"A" },
        @{ @"wxid": @"wxid_b", @"nickname": @"Bob", @"remark": [NSNull null] }
    ];
}

static NSString *iso8601Now() {
    NSISO8601DateFormatter *formatter = [[NSISO8601DateFormatter alloc] init];
    return [formatter stringFromDate:[NSDate date]];
}

static NSDictionary *mockScan() {
    NSString *now = iso8601Now();
    return @{
        @"source_version": kExpectedVersion,
        @"scanned_at": now,
        @"records": @[
            @{ @"wxid": @"wxid_a", @"nickname": @"Alice", @"remark": @"A", @"status": @"normal", @"status_code": @"0xB1", @"source_version": kExpectedVersion, @"scanned_at": now },
            @{ @"wxid": @"wxid_b", @"nickname": @"Bob", @"remark": [NSNull null], @"status": @"unknown", @"status_code": @"0x00", @"source_version": kExpectedVersion, @"scanned_at": now }
        ],
        @"summary": @{ @"normal": @1, @"unknown": @1 }
    };
}

NSDictionary *MFBuildAdapterStatus(NSDictionary *adapterManifest) {
    NSDictionary *probe = probeTarget(adapterManifest ?: @{});
    NSString *fixtureMode = [[[NSProcessInfo processInfo] environment] objectForKey:@"MACFRIENDS_ENABLE_FIXTURE"];
    BOOL fixtureEnabled = [fixtureMode isEqualToString:@"1"];
    BOOL supported = [probe[@"target_supported"] boolValue];
    NSDictionary *resolution = primitiveResolution(fixtureEnabled, supported);
    BOOL runtimeReady = supported && !fixtureEnabled
        && [resolution[@"profile"] isEqualToString:@"resolved"]
        && [resolution[@"contacts"] isEqualToString:@"resolved"]
        && [resolution[@"scan"] isEqualToString:@"resolved"];
    return @{
        @"connected": @YES,
        @"mode": @"single-version-adapter",
        @"bundle_id": probe[@"bundle_id"],
        @"bundle_version": probe[@"bundle_version"],
        @"adapter_loaded": probe[@"target_supported"],
        @"target_supported": probe[@"target_supported"],
        @"adapter_name": probe[@"adapter_name"],
        @"reason": probe[@"reason"],
        @"runtime_ready": @(runtimeReady),
        @"fixture_enabled": @(fixtureEnabled),
        @"primitive_resolution": resolution
    };
}

NSDictionary *MFHandleAdapterRequest(NSDictionary *adapterManifest, NSString *method, NSDictionary *params, BOOL fixtureEnabled) {
    (void)params;
    NSDictionary *probe = probeTarget(adapterManifest ?: @{});
    BOOL supported = [probe[@"target_supported"] boolValue];

    if ([method isEqualToString:@"status"]) {
        return okResult(MFBuildAdapterStatus(adapterManifest));
    }
    if ([method isEqualToString:@"stop"]) {
        return okResult(@{ @"message": @"agent 已停止" });
    }
    if (fixtureEnabled) {
        if ([method isEqualToString:@"profile"]) {
            return okResult(mockProfile());
        }
        if ([method isEqualToString:@"contacts"]) {
            return okResult(mockContacts());
        }
        if ([method isEqualToString:@"scan"]) {
            return okResult(mockScan());
        }
    }
    if (!supported) {
        NSString *reason = probe[@"reason"] == [NSNull null] ? @"adapter_not_loaded" : probe[@"reason"];
        return errorResult(reason);
    }
    if ([method isEqualToString:@"profile"]) {
        return errorResult(@"profile_primitive_unresolved");
    }
    if ([method isEqualToString:@"contacts"]) {
        return errorResult(@"contacts_primitive_unresolved");
    }
    if ([method isEqualToString:@"scan"]) {
        return errorResult(@"scan_primitive_unresolved");
    }
    return errorResult(@"unknown_method");
}
