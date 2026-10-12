#!/usr/bin/env python3
"""Import or verify generated shared-editor assets; never commit large binaries."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import tempfile

ROOT = Path(__file__).resolve().parents[1]

def verify(directory, expected=None):
    directory = Path(directory).resolve()
    manifest = (directory / "host-assets.json").read_bytes()
    if expected is not None and manifest != Path(expected).read_bytes():
        raise ValueError("Asset manifest differs from recorded editor-build.json")
    data = json.loads(manifest)
    if data.get("protocol") != 1 or not isinstance(data.get("assets"), dict) or not data["assets"]:
        raise ValueError("Invalid or empty asset manifest")
    for name, digest in data["assets"].items():
        relative = PurePosixPath(name)
        if relative.is_absolute() or ".." in relative.parts or "\\" in name:
            raise ValueError("Unsafe asset path")
        path = directory.joinpath(*relative.parts)
        if path.is_symlink() or not path.resolve().is_relative_to(directory):
            raise ValueError("Symlink or escaped asset path")
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError("Asset hash mismatch: " + name)
    actual = {p.relative_to(directory).as_posix() for p in directory.rglob("*") if p.is_file()}
    if actual != set(data["assets"]) | {"host-assets.json"}:
        raise ValueError("Unlisted or missing asset files")
    return manifest, data

def import_assets(source, target, lock):
    source, target, lock = Path(source).resolve(), Path(target), Path(lock)
    manifest, data = verify(source)
    if target.exists():
        # Refuse overwriting a different existing build; preserve its artifacts.
        verify(target)
        if (target / "host-assets.json").read_bytes() != manifest:
            raise ValueError("Destination contains a different build; preserve/move it first")
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=target.parent, prefix=".editor-import-") as temporary:
            staged = Path(temporary) / "assets"
            shutil.copytree(source, staged)
            verify(staged)
            staged.rename(target)
    lock.write_bytes(manifest)
    return len(data["assets"])

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--from", dest="source", type=Path)
    args = parser.parse_args()
    target, lock = ROOT / "media/editor", ROOT / "media/editor-build.json"
    if args.source:
        count = import_assets(args.source, target, lock)
    else:
        _, data = verify(target, lock)
        count = len(data["assets"])
    print(f"Verified {count} shared editor assets")

if __name__ == "__main__":
    main()
