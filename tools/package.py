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
notices = root / 'licenses'
if not (notices / 'Qt' / 'inventory.json').is_file():
    raise SystemExit('Dependency notices missing; run tools/update_notices.py')
shutil.copytree(notices, stage / 'licenses', dirs_exist_ok=True)
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
