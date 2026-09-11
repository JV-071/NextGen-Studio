"""Refresh third-party notices from exact upstream release tags; never execute downloaded files."""
import concurrent.futures
import hashlib
import json
from pathlib import Path, PurePosixPath
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'licenses' / 'Qt'
BASE = 'https://raw.githubusercontent.com/qt/qtbase/v6.8.3/'

def get(url):
    request = urllib.request.Request(url, headers={'User-Agent': 'NextGen-Studio-notices'})
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()

tree = json.loads(get('https://api.github.com/repos/qt/qtbase/git/trees/v6.8.3?recursive=1'))
if tree.get('truncated'):
    raise SystemExit('Incomplete Qt source inventory')
paths = {item['path'] for item in tree['tree'] if item['type'] == 'blob'}
metadata = sorted(p for p in paths if p.endswith('qt_attribution.json'))
selected = {p for p in paths if p.startswith('LICENSES/')}
selected.update(p for p in paths if p.startswith('src/3rdparty/') and any(token in PurePosixPath(p).name.upper() for token in ['LICENSE', 'COPYING', 'COPYRIGHT', 'NOTICE']))
selected = {p for p in selected if PurePosixPath(p).name not in {'no-copyright', 'update-copyright', 'update-copyright-year'}}
data = {}
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    for name, content in zip(metadata, pool.map(lambda p: get(BASE + p), metadata)):
        data[name] = content
        entries = json.loads(content, strict=False)
        if isinstance(entries, dict):
            entries = [entries]
        for entry in entries:
            license_file = entry.get('LicenseFile')
            if isinstance(license_file, str):
                candidate = str(PurePosixPath(name).parent / license_file)
                if candidate in paths:
                    selected.add(candidate)
    names = sorted(selected - data.keys())
    for name, content in zip(names, pool.map(lambda p: get(BASE + p), names)):
        data[name] = content
inventory = []
for name, content in sorted(data.items()):
    destination = OUT / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(content)
    inventory.append({'path': name, 'sha256': hashlib.sha256(content).hexdigest(), 'source': BASE + name})
OUT.mkdir(parents=True, exist_ok=True)
(OUT / 'inventory.json').write_text(json.dumps(inventory, indent=2) + '\n', encoding='utf-8')
icu = ROOT / 'licenses' / 'ICU'
icu.mkdir(parents=True, exist_ok=True)
(icu / 'LICENSE').write_bytes(get('https://raw.githubusercontent.com/unicode-org/icu/release-73-2/icu4c/LICENSE'))
print(f'Collected {len(inventory)} Qt notice files and ICU license')