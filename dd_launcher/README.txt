Double Dragon (2010 iOS) launcher - unofficial
==============================================

A Windows launcher and a modified build of touchHLE (an iPhone OS emulator)
for playing the 2010 iOS version of Double Dragon on a PC, with controller
and keyboard support, local 2-player co-op and some game fixes.

THE GAME IS NOT INCLUDED. You need your own legally obtained copy of the
game's .ipa file.

Getting started
---------------
1. Put your DoubleDragon.ipa in the "game" folder.
2. Double-click "Double Dragon.exe".
3. Set up your keys and controllers, then press Play 1 player or Play 2 players.
   Settings are saved automatically (in the "config" folder).

Needs Windows 10 or 11 (uses the built-in Microsoft Edge WebView2).

Folders
  engine     the modified touchHLE and its files (don't change these)
  game       your .ipa goes here
  config     your saved controls
  saves      save data: player1 is the main game, player2 is the second co-op window
  licenses   license texts and copyright notices

Moving old save data over:
  copy everything inside an old "touchHLE_sandbox" folder into saves\player1,
  and everything inside "touchHLE_sandbox_p2" into saves\player2.

Not affiliated
--------------
This is an unofficial fan project. It is not made, endorsed or supported by
the touchHLE project, Arc System Works, Bow Mobile, Apple or Microsoft.
Double Dragon is a trademark of its respective owner. iPhone and iOS are
trademarks of Apple Inc.

Please do not report problems with this build to the touchHLE project: it
contains changes they have not made or reviewed. The changes were written
with the help of an AI assistant; the touchHLE project does not accept
AI-written code, so they will not be offered upstream.

Licenses and source code
------------------------
- touchHLE: source code under the Mozilla Public License 2.0; the touchHLE
  executable (engine\touchHLE.exe) is distributed under the GNU General
  Public License, version 3 or later, as with official touchHLE builds.
  See licenses\touchHLE - GPL-3.0 (COPYING).txt,
  licenses\touchHLE source - MPL-2.0.txt and
  licenses\touchHLE - copyright and third-party licenses.txt.
- Bundled iPhone OS libraries and fonts: see engine\touchHLE_dylibs and
  engine\touchHLE_fonts (each folder has its own license files and a
  README with links to their source code).
- The launcher ("Double Dragon.exe"): Mozilla Public License 2.0. Its
  third-party components are listed in
  licenses\Launcher - third-party licenses.txt.

Complete source code for everything in this download (touchHLE with the
changes, and the launcher) is available at:
  https://github.com/RockmanKB/touchHLE
The Double Dragon changes are kept in dd_patch.zip in that repository and
unpacked over the touchHLE source before building; see
.github/workflows/touchHLE_release.yml for the exact build steps.

The touchHLE changes (in src/) include: GameKit local link for co-op,
controller/keyboard mapping and menu navigation, screen-aware controls,
window options (scale, border, split screen, full screen swap), memory
patches applied at startup (loading-freeze fix, game speed, dash timing,
classic combos, hiding the on-screen pad) and small compatibility fixes.
