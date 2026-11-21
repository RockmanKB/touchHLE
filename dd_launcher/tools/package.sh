#!/bin/bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Assembles the "Double Dragon" app folder (run from the repository root on
# the Windows build machine, after touchHLE, its bundle and the launcher are built).
set -e
D="DoubleDragon_App/Double Dragon"
rm -rf DoubleDragon_App
mkdir -p "$D/engine" "$D/game" "$D/config" "$D/saves/player1" "$D/saves/player2" "$D/licenses"
cp dd_launcher/target/release/DoubleDragon.exe "$D/Double Dragon.exe"
cp -r touchHLE_windows_bundle/* "$D/engine/"
printf 'Put DoubleDragon.ipa in this folder.\r\n' > "$D/game/PUT DoubleDragon.ipa HERE.txt"
cp dd_launcher/README.txt "$D/README.txt"
cp dev-scripts/gpl-3.0.txt "$D/licenses/touchHLE - GPL-3.0 (COPYING).txt"
cp LICENSE "$D/licenses/touchHLE source - MPL-2.0.txt"
(cd "$D/engine" && ./touchHLE.exe --copyright 2>&1) > "$D/licenses/touchHLE - copyright and third-party licenses.txt"
rm -f "$D/engine/touchHLE_log.txt"
python dd_launcher/tools/notices.py "$D/licenses/Launcher - third-party licenses.txt"
echo "Packaged:"; find DoubleDragon_App -maxdepth 3 | sort
