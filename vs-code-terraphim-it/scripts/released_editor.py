#!/usr/bin/env python3
"""Import the immutable public editor release; no private Cargo access required."""
import argparse
import hashlib
import hmac
import io
import json
from pathlib import Path, PurePosixPath
import tarfile
import tempfile
import urllib.request

from editor_assets import ROOT, import_assets, verify

LOCK = ROOT / "editor-release.json"


def checked(data, digest):
    if not hmac.compare_digest(hashlib.sha256(data).hexdigest(), digest):
        raise ValueError("Released editor SHA-256 mismatch")
    return data


def materialize(archive, manifest, bridge, destination, source):
    """Validate the entire archive before writing only regular admitted files."""
    assets = json.loads(manifest)["assets"]
    files = {}
    total = 0
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as bundle:
        for entry in bundle:
            name = entry.name
            path = PurePosixPath(name)
            if (not entry.isfile() or path.is_absolute() or ".." in path.parts
                    or "\\" in name or path.as_posix() != name or name in files):
                raise ValueError("Unsafe or duplicate released editor entry")
            total += entry.size
            if total > 64 * 1024 * 1024 or name not in assets:
                raise ValueError("Unlisted or oversized released editor entry")
            files[name] = checked(bundle.extractfile(entry).read(), assets[name])
    if set(files) != set(assets):
        raise ValueError("Missing released editor assets")
    if "host-session.js" in files:
        raise ValueError("Unexpected host bridge in browser release")
    files["host-session.js"] = bridge
    destination.mkdir()
    for name, data in files.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    lock = {"protocol": 1, "sourceHead": source["sourceCommit"],
            "release": source["release"],
            "archiveSha256": source["archive"]["sha256"],
            "assets": {name: hashlib.sha256(data).hexdigest()
                       for name, data in sorted(files.items())},
            "qualification": "Released editor assets; host acceptance is separate"}
    (destination / "host-assets.json").write_text(json.dumps(lock, indent=2) + "\n")
    verify(destination)


def fetch(item, cache):
    name = item["name"]
    if cache:
        return checked((cache / name).read_bytes(), item["sha256"])
    request = urllib.request.Request(item["url"], headers={"User-Agent": "terraphim-extension-ci"})
    with urllib.request.urlopen(request, timeout=60) as response:
        data = response.read(64 * 1024 * 1024 + 1)
    if len(data) > 64 * 1024 * 1024:
        raise ValueError("Released editor download exceeds size limit")
    return checked(data, item["sha256"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, help="Read already downloaded, hash-checked inputs")
    args = parser.parse_args()
    source = json.loads(LOCK.read_text())
    inputs = {key: fetch(source[key], args.cache) for key in ("archive", "manifest", "bridge")}
    with tempfile.TemporaryDirectory(prefix="terraphim-released-editor-") as temporary:
        staged = Path(temporary) / "assets"
        materialize(inputs["archive"], inputs["manifest"], inputs["bridge"], staged, source)
        count = import_assets(staged, ROOT / "media/editor", ROOT / "media/editor-build.json")
    print(f"Imported {count} verified assets from {source['release']}")


if __name__ == "__main__":
    main()
