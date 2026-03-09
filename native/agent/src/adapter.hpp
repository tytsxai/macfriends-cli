#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

NSDictionary *MFBuildAdapterStatus(NSDictionary * _Nullable adapterManifest);
NSDictionary *MFHandleAdapterRequest(NSDictionary * _Nullable adapterManifest, NSString *method, NSDictionary *params, BOOL fixtureEnabled);

NS_ASSUME_NONNULL_END
