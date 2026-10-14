"""Read an immutable public release for catalog checks; never build or publish."""
import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

from publish_release import (
    anonymous_download,
    api,
    checksums,
    digest,
    verify_remote_tag,
)
from verify_registry import qualify
from verify_vsix import check_tag

REPOSITORY = "terraphim/terraphim-ai"
ASSETS = {"terraphim-it.vsix", "release-manifest.json", "SHA256SUMS"}


def release_assets(metadata, tag):
    """Bind anonymous asset URLs to the exact public published preview release."""
    if (not re.fullmatch(r"terraphim_vscode-v\d+\.\d+\.\d+", tag)
            or metadata.get("tag_name") != tag or metadata.get("draft") is not False
            or metadata.get("prerelease") is not True):
        raise ValueError("Expected exact published VS Code prerelease")
    assets = metadata["assets"]
    if len(assets) != len(ASSETS) or {item["name"] for item in assets} != ASSETS:
        raise ValueError("Release must have exactly the three qualified assets")
    for item in assets:
        url = f"https://github.com/{REPOSITORY}/releases/download/{tag}/{item['name']}"
        if (item.get("browser_download_url") != url or item.get("state") != "uploaded"
                or type(item.get("size")) is not int or not 0 < item["size"] <= 128 * 1024 * 1024
                or not re.fullmatch(r"sha256:[0-9a-f]{64}", item.get("digest", ""))):
            raise ValueError("Release asset URL, state, size or GitHub digest is invalid")
    return assets


def verify_downloads(root, assets, tag):
    """Verify hashes, archive identity and manifest before returning downloaded bytes."""
    for item in assets:
        path = root / item["name"]
        if path.stat().st_size != item["size"] or "sha256:" + digest(path) != item["digest"]:
            raise ValueError("Downloaded release asset differs from GitHub digest")
    package, manifest = root / "terraphim-it.vsix", root / "release-manifest.json"
    expected, report = qualify(package, manifest)
    check_tag(tag, expected["version"])
    found = checksums((root / "SHA256SUMS").read_text())
    wanted = {package.name: digest(package)}
    if found not in (wanted, {**wanted, manifest.name: digest(manifest)}):
        raise ValueError("Release checksums differ from downloaded package")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--directory", type=Path, required=True)
    args = parser.parse_args()
    try:
        if not re.fullmatch(r"terraphim_vscode-v\d+\.\d+\.\d+", args.tag):
            raise ValueError("Invalid immutable VS Code release tag")
        if args.directory.exists() or args.directory.is_symlink():
            raise ValueError("Choose a new output directory")
        metadata = api(f"repos/{REPOSITORY}/releases/tags/{args.tag}")
        assets = release_assets(metadata, args.tag)
        with tempfile.TemporaryDirectory(prefix="terraphim-registry-release-") as directory:
            root = Path(directory)
            for item in assets:
                anonymous_download(item["browser_download_url"], root / item["name"])
            report = verify_downloads(root, assets, args.tag)
            verify_remote_tag(REPOSITORY, args.tag, report["sourceCommit"])
            args.directory.mkdir(parents=True)
            for item in assets:
                (args.directory / item["name"]).write_bytes((root / item["name"]).read_bytes())
            receipt = {"schema": 1, "status": "RELEASE_BYTES_VERIFIED", "repository": REPOSITORY,
                       "tag": args.tag, "sourceCommit": report["sourceCommit"], "assets": assets,
                       "scope": "Read-only published artifact recovery; no rebuild, publication or new native qualification"}
            (args.directory / "release-readback.json").write_text(json.dumps(receipt, indent=2) + "\n")
    except (ValueError, KeyError, TypeError, OSError, subprocess.SubprocessError) as error:
        print(f"download_registry_release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
