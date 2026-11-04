/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Local stubs (not upstream) for GameKit classes: Game Center achievements
//! and UI, plus Bluetooth peer-to-peer. Game Center is reported unavailable
//! (`+[GKLocalPlayer localPlayer]` is nil), so these should rarely be used.

use crate::objc::{id, nil, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation GKAchievement: NSObject
- (id)initWithIdentifier:(id)_identifier { this }
- (())setPercentComplete:(f64)_pc {}
- (())reportAchievementWithCompletionHandler:(id)_handler {}
@end

@implementation GKAchievementViewController: NSObject
- (id)init { nil }
@end

@implementation GKLeaderboardViewController: NSObject
- (id)init { nil }
@end

};
