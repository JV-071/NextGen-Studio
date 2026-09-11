"""Package only the install tree; never copy client files or developer settings."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import zipfile

parser = argparse.ArgumentParser()
parser.add_argument("--platform", required=True)
parser.add_argument("--config", choices=["Debug", "Release"], required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent
stage = root / "staging"
qt_path = os.environ.get("QT_ROOT_DIR") or os.environ.get("Qt6_DIR")
if not qt_path:
    raise SystemExit("Qt SDK path missing")
qt = Path(qt_path)
if not qt.is_dir():
    raise SystemExit("Qt SDK path missing")
licenses = stage / "licenses" / "Qt"
licenses.mkdir(parents=True, exist_ok=True)
sources = [qt / "LICENSES", qt / "licenses", qt.parent / "LICENSES", qt / "doc" / "global"]
copied = 0
for source in sources:
    if source.is_dir():
        for f in source.rglob("*"):
            if f.is_file() and f.stat().st_size < 2_000_000 and ("license" in f.name.lower() or f.suffix in {".txt", ".html"}):
                target = licenses / f.relative_to(source)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(f, target)
                copied += 1
# Qt binary archives differ in how licenses are laid out. Fetch exact-version texts
# from the official upstream if the archive did not provide them.
if copied == 0:
    import urllib.request
    for name in ["LGPL-3.0-only.txt", "GPL-3.0-only.txt", "Qt-GPL-exception-1.0.txt"]:
        url = "https://raw.githubusercontent.com/qt/qtbase/v6.8.3/LICENSES/" + name
        with urllib.request.urlopen(url, timeout=30) as response:
            (licenses / name).write_bytes(response.read())
commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
(stage / "BUILD.json").write_text(json.dumps({
    "version": "0.1.0", "commit": commit, "platform": args.platform,
    "configuration": args.config, "qt": "6.8.3", "compiler_cache": True,
    "note": "Linux requires glibc 2.39+ and system graphics libraries. Windows Debug requires MSVC debug runtime."
}, indent=2) + "\n", encoding="utf-8")
out = root / "dist"
out.mkdir(exist_ok=True)
name = f"NextGen-Studio-0.1.0-{args.platform}-{args.config}"
archive = out / (name + ".zip")
with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED, compresslevel=3) as z:
    for f in sorted(stage.rglob("*")):
        if f.is_file():
            z.write(f, Path(name) / f.relative_to(stage))
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
(out / (archive.name + ".sha256")).write_text(f"{digest}  {archive.name}\n", encoding="ascii")
print(f"{archive.name}: {archive.stat().st_size / 1024 / 1024:.1f} MiB")
