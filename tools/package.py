"""Package native Rust binary, integration and dependency notices. No SDK DLLs."""
import argparse, hashlib, json, os, shutil, subprocess, tomllib, zipfile
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument("--platform",required=True)
p.add_argument("--config",choices=["Debug","Release"],required=True)
a=p.parse_args()
root=Path(__file__).resolve().parent.parent
version=tomllib.loads((root/"Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
stage=root/"staging"
if stage.exists():
    raise SystemExit("Install tree already exists; use a clean staging directory")
(stage/"bin").mkdir(parents=True)
profile="release" if a.config=="Release" else "debug"
exe="nextgen-studio.exe" if a.platform.startswith("Windows") else "nextgen-studio"
shutil.copy2(root/"target"/profile/exe,stage/"bin"/exe)
pdb=root/"target"/profile/"nextgen_studio.pdb"
if a.config=="Debug" and pdb.exists():shutil.copy2(pdb,stage/"bin"/pdb.name)
for f in ["README.md","LICENSE","THIRD_PARTY.md","Cargo.lock"]:
    shutil.copy2(root/f,stage/f)
for directory in ["examples","integration"]:
    shutil.copytree(root/directory,stage/directory)
metadata=json.loads(subprocess.check_output(["cargo","metadata","--locked","--format-version","1"],cwd=root,text=True))
notices=stage/"licenses";notices.mkdir()
entries=[]
for package in metadata["packages"]:
    if package["name"]=="nextgen-studio":continue
    base=Path(package["manifest_path"]).parent
    destination=notices/(package["name"]+"-"+package["version"])
    destination.mkdir()
    copied=[]
    candidates=list(base.iterdir())
    for extra in ["LICENSES","licenses"]:
        if (base/extra).is_dir():candidates.extend((base/extra).rglob("*"))
    for file in candidates:
        if file.is_file() and file.stat().st_size<2_000_000 and any(file.name.upper().startswith(n) for n in ["LICENSE","COPYING","NOTICE","COPYRIGHT","UNLICENSE"]):
            name=str(file.relative_to(base)).replace(os.sep,"_")
            shutil.copyfile(file,destination/name);copied.append(name)
    entries.append({"name":package["name"],"version":package["version"],"license":package["license"],"repository":package["repository"],"authors":package["authors"],"notice_files":copied})
(notices/"DEPENDENCIES.json").write_text(json.dumps(entries,indent=2)+"\n",encoding="utf-8")
commit=subprocess.check_output(["git","rev-parse","HEAD"],cwd=root,text=True).strip()
(stage/"BUILD.json").write_text(json.dumps({"version":version,"commit":commit,"platform":a.platform,"configuration":a.config,"rust":"1.98.1","renderer":"egui + glow/OpenGL","note":"Linux baseline Ubuntu 24.04. GTK3/OpenGL system libraries required. Windows uses static CRT."},indent=2)+"\n",encoding="utf-8")
out=root/"dist";out.mkdir(exist_ok=True)
name=f"NextGen-Studio-{version}-{a.platform}-{a.config}"
archive=out/(name+".zip")
with zipfile.ZipFile(archive,"w",zipfile.ZIP_DEFLATED,compresslevel=3) as z:
    for f in sorted(stage.rglob("*")):
        if f.is_file():z.write(f,Path(name)/f.relative_to(stage))
(out/(archive.name+".sha256")).write_text(hashlib.sha256(archive.read_bytes()).hexdigest()+"  "+archive.name+"\n",encoding="ascii")
print(f"{archive.name}: {archive.stat().st_size/1024/1024:.2f} MiB")
