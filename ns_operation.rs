/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSOperation`, `NSOperationQueue` and a few other minimal Foundation stubs.
//!
//! Local stubs (not upstream): enough for apps that create these objects
//! during startup (e.g. bundled Twitter/OAuth SDKs) without really using them.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, msg, nil, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSOperation: NSObject

- (id)init { this }
- (())start {
    () = msg![env; this main];
}
- (())main {}
- (())cancel {}
- (bool)isCancelled { false }
- (bool)isExecuting { false }
- (bool)isFinished { true }
- (bool)isConcurrent { false }
- (bool)isReady { true }

@end

@implementation NSOperationQueue: NSObject

- (id)init { this }
- (())addOperation:(id)op {
    log!("TODO: [NSOperationQueue addOperation:{:?}] (ignored)", op);
}
- (())setMaxConcurrentOperationCount:(NSInteger)_count {}
- (NSInteger)maxConcurrentOperationCount { 1 }
- (())cancelAllOperations {}
- (())waitUntilAllOperationsAreFinished {}
- (())setSuspended:(bool)_s {}
- (id)operations { nil }

@end

@implementation NSHTTPCookieStorage: NSObject
+ (id)sharedHTTPCookieStorage { nil }
@end

@implementation NSURLCredential: NSObject
@end

@implementation NSNumberFormatter: NSObject
- (id)init { this }
@end

@implementation NSDecimalNumber: NSObject
@end

@implementation NSException: NSObject
@end

};
