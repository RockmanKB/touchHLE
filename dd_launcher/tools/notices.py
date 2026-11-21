# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
"""Writes the third-party notices for the Double Dragon launcher: every crate
compiled into the Windows build, with its license and its own license files."""
import json, os, subprocess, sys

manifest = os.path.join(os.path.dirname(__file__), "..", "Cargo.toml")
out_path = sys.argv[1]
meta = json.loads(subprocess.check_output([
    "cargo", "metadata", "--format-version", "1", "--manifest-path", manifest,
    "--filter-platform", "x86_64-pc-windows-msvc",
]))
used = {n["id"] for n in meta["resolve"]["nodes"]}
pkgs = sorted((p for p in meta["packages"] if p["id"] in used and p["source"]),
              key=lambda p: (p["name"], p["version"]))
lines = [
    "Third-party software in \"Double Dragon.exe\" (the launcher)",
    "=" * 60, "",
    "The launcher is built with the Rust crates below. Each crate's license",
    "files follow the list.", "",
]
for p in pkgs:
    lines.append(f"{p['name']} {p['version']}  --  {p.get('license') or 'see license file'}"
                 + (f"  --  {p['repository']}" if p.get("repository") else ""))
lines.append("")
for p in pkgs:
    d = os.path.dirname(p["manifest_path"])
    files = sorted(f for f in os.listdir(d)
                   if f.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))
                   and os.path.isfile(os.path.join(d, f)))
    if p.get("license_file") and os.path.basename(p["license_file"]) not in files:
        files.append(p["license_file"])
    lines += ["", "-" * 60, f"{p['name']} {p['version']}", "-" * 60]
    if not files:
        lines.append(f"License: {p.get('license')} (no license file shipped with this crate)")
    for f in files:
        with open(os.path.join(d, f), encoding="utf-8", errors="replace") as fh:
            lines += ["", f"[{f}]", fh.read().rstrip()]
# License files for crates that don't ship one, and for the bundled
# Microsoft WebView2 loader.
extra = os.path.join(os.path.dirname(__file__), "extra_licenses")
for f in sorted(os.listdir(extra)):
    with open(os.path.join(extra, f), encoding="utf-8") as fh:
        lines += ["", "-" * 60, f[:-4], "-" * 60, "", fh.read().rstrip()]
with open(out_path, "w", encoding="utf-8", newline="\r\n") as fh:
    fh.write("\n".join(lines) + "\n")
print(f"{len(pkgs)} crates")
