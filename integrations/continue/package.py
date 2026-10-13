"""Create a separate Continue integration archive with an explicit file allowlist."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import zipfile

FILES = ['README.md', 'install.py', 'server.py', 'requirements.txt', 'integration.json', 'risk-concern.exported.json', 'test_install.py', 'test_transport.py', 'test_package.py', 'package.py', 'verify_source.py', 'test_source.py', '.gitignore']


def package(destination):
    root = Path(__file__).resolve().parent
    manifest = json.loads((root / 'integration.json').read_text())
    if not re.fullmatch(r'0|[1-9][0-9]*', manifest['version'].split('.')[0]) or not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', manifest['version']):
        raise ValueError('Invalid integration version')
    try:
        head = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], stderr=subprocess.DEVNULL, text=True).strip()
        clean = not subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain', '--untracked-files=all'], stderr=subprocess.DEVNULL, text=True).strip()
    except subprocess.CalledProcessError:
        head, clean = None, False
    qualification = {'sourceHead': head, 'cleanTree': clean, 'fileHashes': {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in FILES}}
    destination.mkdir(parents=True, exist_ok=True)
    target = destination / f"terraphim-continue-integration-{manifest['version']}.zip"
    with zipfile.ZipFile(target, 'x', zipfile.ZIP_DEFLATED) as archive:
        for name in FILES:
            path = root / name
            if path.is_symlink() or not path.is_file():
                raise ValueError('Package input is not an ordinary file')
            entry = zipfile.ZipInfo('terraphim-continue/' + name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = zipfile.ZIP_DEFLATED
            entry.external_attr = 0o100644 << 16
            archive.writestr(entry, path.read_bytes())
        entry = zipfile.ZipInfo('terraphim-continue/source.json', date_time=(1980, 1, 1, 0, 0, 0))
        entry.compress_type = zipfile.ZIP_DEFLATED
        entry.external_attr = 0o100644 << 16
        archive.writestr(entry, json.dumps(qualification, sort_keys=True) + '\n')
    checksum = destination / 'terraphim-continue-SHA256SUMS'
    with checksum.open('x') as stream:
        stream.write(hashlib.sha256(target.read_bytes()).hexdigest() + '  ' + target.name + '\n')


if __name__ == '__main__':
    args = argparse.ArgumentParser()
    args.add_argument('destination', type=Path)
    package(args.parse_args().destination)
