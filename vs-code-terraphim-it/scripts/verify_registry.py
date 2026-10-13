#!/usr/bin/env python3
"""Verify one exact registry version against the qualified extension payload."""
import argparse
import hashlib
import hmac
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from urllib.parse import urlparse
import zipfile
from xml.etree.ElementTree import ParseError

from publish_release import SHA_RE, anonymous_download, digest
import verify_vsix

IDENTIFIER_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]*")


def identity(manifest):
    publisher, name, version = (manifest[key] for key in ("publisher", "name", "version"))
    if (not IDENTIFIER_RE.fullmatch(publisher) or not IDENTIFIER_RE.fullmatch(name)
            or not re.fullmatch(r"\d+\.\d+\.\d+", version)):
        raise ValueError("Invalid exact extension identity")
    return publisher, name, version


def payload(package):
    with zipfile.ZipFile(package) as archive:
        expected = json.loads(archive.read("extension/package.json"))
        lock = archive.read("extension/media/editor-build.json")
        # Reuse the package gate, including duplicate paths and extraction aliases.
        report = verify_vsix.verify(package, expected, lock)
        files = {entry.filename: hashlib.sha256(archive.read(entry)).hexdigest()
                 for entry in archive.infolist()
                 if entry.filename.startswith("extension/") and not entry.is_dir()}
    identity(expected)
    return expected, report, files


def compare_payloads(qualified, downloaded):
    expected, _, wanted = payload(qualified)
    actual, _, found = payload(downloaded)
    if identity(actual) != identity(expected):
        raise ValueError("Registry returned a different extension or version")
    if wanted != found:
        changed = sorted(name for name in wanted.keys() | found.keys() if wanted.get(name) != found.get(name))
        raise ValueError("Registry extension payload differs: " + ", ".join(changed[:5]))
    # Signing/container metadata outside extension/* is intentionally not compared.
    return wanted


def registry_url(registry, manifest):
    publisher, name, version = identity(manifest)
    if registry == "marketplace":
        # Microsoft's GalleryRestClient.getAssetByName public route.
        return (f"https://marketplace.visualstudio.com/_apis/public/gallery/publisher/{publisher}"
                f"/extension/{name}/{version}/assetbyname/Microsoft.VisualStudio.Services.VSIXPackage")
    if registry == "openvsx":
        # Open VSX Registry.getMetadata(namespace, extension, target, version).
        return f"https://open-vsx.org/api/{publisher}/{name}/universal/{version}"
    raise ValueError("Unsupported registry")


def openvsx_download(metadata, manifest):
    publisher, name, version = identity(manifest)
    if (metadata.get("namespace"), metadata.get("name"), metadata.get("version")) != (publisher, name, version):
        raise ValueError("Open VSX metadata differs from exact qualified identity")
    if metadata.get("targetPlatform") != "universal":
        raise ValueError("Open VSX returned a different target platform")
    url = metadata["files"]["download"]
    parsed = urlparse(url)
    if (parsed.scheme != "https" or parsed.netloc != "open-vsx.org" or parsed.username
            or parsed.password or parsed.query or parsed.fragment
            or not parsed.path.startswith(f"/api/{publisher}/{name}/")):
        raise ValueError("Unexpected Open VSX download URL")
    return url


def qualify(package, manifest):
    report = json.loads(manifest.read_text())
    expected, actual, _ = payload(package)
    if any(report.get(key) != value for key, value in actual.items()):
        raise ValueError("Local package differs from qualification report")
    if not SHA_RE.fullmatch(report.get("sourceCommit", "")) or report.get("sourceTreeDirty") is not False:
        raise ValueError("Registry verification requires clean source qualification")
    return expected, report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", choices=["marketplace", "openvsx"], required=True)
    parser.add_argument("--vsix", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    result = {"schema": 1, "registry": args.registry, "status": "FAILED"}
    try:
        if args.receipt.exists() or args.receipt.is_symlink():
            raise ValueError("Choose a new receipt path to retain prior registry evidence")
        expected, report = qualify(args.vsix, args.manifest)
        url = registry_url(args.registry, expected)
        with tempfile.TemporaryDirectory(prefix="terraphim-registry-") as folder:
            root = Path(folder)
            # Hold the qualified bytes stable throughout network verification.
            local = root / "qualified.vsix"
            local.write_bytes(args.vsix.read_bytes())
            if not hmac.compare_digest(digest(local), report["sha256"]):
                raise ValueError("Qualified package changed before registry verification")
            if args.registry == "openvsx":
                metadata = json.loads(anonymous_download(url, root / "registry.json"))
                url = openvsx_download(metadata, expected)
            remote = root / "published.vsix"
            anonymous_download(url, remote)
            files = compare_payloads(local, remote)
            result = {"schema": 1, "status": "REGISTRY_PAYLOAD_VERIFIED", "registry": args.registry,
                      "extension": report["extension"], "version": report["version"],
                      "sourceCommit": report["sourceCommit"], "downloadUrl": url,
                      "qualifiedSha256": report["sha256"], "registrySha256": digest(remote),
                      "comparison": "Every extension/* file; outer signing/container metadata may differ",
                      "payloadFiles": files}
    except (ValueError, KeyError, TypeError, OSError, zipfile.BadZipFile, ParseError, subprocess.CalledProcessError) as error:
        result["error"] = str(error)
        print(f"verify_registry: {error}", file=sys.stderr)
    if not args.receipt.exists() and not args.receipt.is_symlink():
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        with args.receipt.open("x") as output:
            output.write(json.dumps(result, indent=2) + "\n")
    return 0 if result["status"] == "REGISTRY_PAYLOAD_VERIFIED" else 1


if __name__ == "__main__":
    sys.exit(main())
