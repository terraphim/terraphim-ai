"""Verify one exact registry version against the qualified extension payload."""
import argparse
import hashlib
import hmac
import json
import re
import subprocess
import sys
import tempfile
import time
import zipfile
from datetime import UTC, datetime
from pathlib import Path
from urllib.parse import urlparse
from xml.etree.ElementTree import ParseError

import verify_vsix
from publish_release import SHA_RE, anonymous_download, digest

GALLERY_URLS = {
    "marketplace": "https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery",
    "openvsx": "https://open-vsx.org/vscode/gallery/extensionquery",
    "cursor": "https://marketplace.cursorapi.com/_apis/public/gallery/extensionquery",
    "windsurf": "https://marketplace.windsurf.com/vscode/gallery/extensionquery",
}
VSIX_ASSET = "Microsoft.VisualStudio.Services.VSIXPackage"
PRERELEASE_PROPERTY = "Microsoft.VisualStudio.Code.PreRelease"


class NotIndexed(ValueError):
    """The exact qualified version is not yet offered in this catalog."""


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


def require_trusted_publication(metadata):
    """Open VSX must attest OIDC publication; byte equality alone is insufficient."""
    if metadata.get("publishedWithTrustedPublishing") is not True:
        raise ValueError("Open VSX does not attest trusted publishing for this exact version")


def gallery_query(manifest):
    """Use the same exact-ID query and version/assets flags as Gallery clients."""
    publisher, name, _ = identity(manifest)
    return {"filters": [{"criteria": [{"filterType": 7, "value": f"{publisher}.{name}"}],
                         "pageNumber": 1, "pageSize": 100}], "flags": 147}


def catalog_version(metadata, manifest, registry):
    """Reject missing, ambiguous, wrong-channel or non-public catalog entries."""
    publisher, name, version = identity(manifest)
    matches = [entry for result in metadata["results"] for entry in result["extensions"]
               if (entry["publisher"]["publisherName"], entry["extensionName"]) == (publisher, name)]
    if not matches:
        raise NotIndexed("Exact extension is absent from catalog")
    if len(matches) != 1:
        raise ValueError("Ambiguous exact extension in catalog")
    extension = matches[0]
    versions = [item for item in extension["versions"] if item["version"] == version
                and item.get("targetPlatform") in (None, "universal")]
    if not versions:
        raise NotIndexed("Exact universal version is absent from catalog")
    if len(versions) != 1:
        raise ValueError("Ambiguous exact universal version in catalog")
    selected = versions[0]
    properties = [item["value"] for item in selected["properties"] if item["key"] == PRERELEASE_PROPERTY]
    if properties != ["true"]:
        raise ValueError("Catalog version is not the qualified prerelease channel")
    if registry == "marketplace":
        if not {"public", "validated"} <= {flag.strip() for flag in extension.get("flags", "").split(",")}:
            raise NotIndexed("Microsoft extension has not passed public validation")
        if "validated" not in {flag.strip() for flag in selected.get("flags", "").split(",")}:
            raise NotIndexed("Microsoft version has not passed validation")
    return selected


def catalog_download(selected, manifest, registry):
    """Only admit the registry's actual exact-version VSIX URL on known origins."""
    publisher, name, version = identity(manifest)
    sources = [item["source"] for item in selected["files"] if item["assetType"] == VSIX_ASSET]
    if len(sources) != 1:
        raise ValueError("Catalog must advertise exactly one VSIX download")
    url = sources[0]
    parsed = urlparse(url)
    if (parsed.scheme != "https" or parsed.username or parsed.password or parsed.query or parsed.fragment):
        raise ValueError("Unsafe catalog download URL")
    prefix = f"/api/{publisher}/{name}/{version}/file/"
    origins = {
        "openvsx": {"open-vsx.org": [prefix]},
        "cursor": {"marketplace.cursorapi.com": ["/open-vsx-mirror" + prefix]},
        "windsurf": {"open-vsx.org": [prefix], "marketplace.windsurf.com": [prefix, "/open-vsx.org" + prefix]},
    }
    if registry == "marketplace":
        allowed = (parsed.netloc == f"{publisher}.gallerycdn.vsassets.io"
                   and re.fullmatch(rf"/extensions/{re.escape(publisher)}/{re.escape(name)}/{re.escape(version)}/[0-9]+/{re.escape(VSIX_ASSET)}", parsed.path))
    else:
        allowed = any(parsed.path.startswith(value) and parsed.path[len(value):].endswith(".vsix")
                      and re.fullmatch(r"[A-Za-z0-9_.-]+", parsed.path[len(value):])
                      for value in origins.get(registry, {}).get(parsed.netloc, []))
    if not allowed:
        raise ValueError("Unexpected catalog VSIX origin or version path")
    return url


def read_catalog(registry, manifest, root):
    """Anonymous, bounded read-only POST; never inherit curl config or credentials."""
    request = root / "query.json"
    response = root / "catalog.json"
    request.write_text(json.dumps(gallery_query(manifest)))
    result = subprocess.run([
        "curl", "--disable", "--fail", "--silent", "--show-error", "--proto", "=https",
        "--connect-timeout", "15", "--max-time", "60", "--max-filesize", str(8 * 1024 * 1024),
        "--request", "POST", "--header", "Content-Type: application/json",
        "--header", "Accept: application/json;api-version=7.2-preview.1",
        "--data-binary", "@" + str(request), "--output", str(response),
        "--write-out", "%{http_code}", GALLERY_URLS[registry],
    ], text=True, capture_output=True, timeout=70, check=False)
    if result.returncode or result.stdout != "200":
        raise ValueError(f"Anonymous catalog query failed: HTTP {result.stdout}; curl {result.returncode}")
    raw = response.read_bytes()
    metadata = json.loads(raw)
    return metadata, {"url": GALLERY_URLS[registry], "request": gallery_query(manifest),
                      "responseSha256": hashlib.sha256(raw).hexdigest(), "responseBytes": len(raw),
                      "responseBody": raw.decode("utf-8")}


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
    parser.add_argument("--registry", choices=list(GALLERY_URLS), required=True)
    parser.add_argument("--vsix", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--attempts", type=int, default=1, help="Bounded catalog-index attempts (1–20)")
    parser.add_argument("--delay", type=int, default=30, help="Seconds between absent-index reads (0–30)")
    parser.add_argument("--require-trusted-publishing", action="store_true",
                        help="Require Open VSX to attest OIDC for this exact version")
    args = parser.parse_args()
    result = {"schema": 2, "registry": args.registry, "status": "FAILED", "attempts": [],
              "scope": "Exact catalog version and downloaded payload; no native host/functionality claim"}
    try:
        if args.receipt.exists() or args.receipt.is_symlink():
            raise ValueError("Choose a new receipt path to retain prior registry evidence")
        if not 1 <= args.attempts <= 20 or not 0 <= args.delay <= 30:
            raise ValueError("Catalog retry bounds exceeded")
        if args.require_trusted_publishing and args.registry != "openvsx":
            raise ValueError("Trusted-publication metadata is supported only by Open VSX")
        expected, report = qualify(args.vsix, args.manifest)
        result.update(extension=report["extension"], version=report["version"],
                      sourceCommit=report["sourceCommit"], qualifiedSha256=report["sha256"])
        with tempfile.TemporaryDirectory(prefix="terraphim-registry-") as folder:
            root = Path(folder)
            local = root / "qualified.vsix"
            local.write_bytes(args.vsix.read_bytes())
            if not hmac.compare_digest(digest(local), report["sha256"]):
                raise ValueError("Qualified package changed before registry verification")
            for attempt in range(args.attempts):
                directory = root / str(attempt)
                directory.mkdir()
                metadata, evidence = read_catalog(args.registry, expected, directory)
                evidence["checkedAt"] = datetime.now(UTC).isoformat()
                result["attempts"].append(evidence)
                try:
                    selected = catalog_version(metadata, expected, args.registry)
                except NotIndexed as error:
                    evidence.update(status="NOT_INDEXED", error=str(error))
                    if attempt + 1 == args.attempts:
                        raise
                    time.sleep(args.delay)
                    continue
                evidence.update(status="INDEXED", version=selected)
                break
            url = catalog_download(selected, expected, args.registry)
            if args.registry == "openvsx":
                metadata = json.loads(anonymous_download(registry_url(args.registry, expected), root / "registry.json"))
                result["registryMetadata"] = metadata
                result["publishedWithTrustedPublishing"] = metadata.get("publishedWithTrustedPublishing")
                if args.require_trusted_publishing:
                    require_trusted_publication(metadata)
                if openvsx_download(metadata, expected) != url:
                    raise ValueError("Open VSX catalog and exact metadata downloads differ")
            remote = root / "published.vsix"
            anonymous_download(url, remote)
            files = compare_payloads(local, remote)
            result.update(status="REGISTRY_PAYLOAD_VERIFIED", catalogIndexed=True,
                          downloadUrl=url, registrySha256=digest(remote),
                          comparison="Every extension/* file; outer signing/container metadata may differ",
                          payloadFiles=files)
    except (ValueError, KeyError, TypeError, AttributeError, IndexError, OSError, zipfile.BadZipFile, ParseError, subprocess.SubprocessError) as error:
        result["error"] = str(error)
        print(f"verify_registry: {error}", file=sys.stderr)
    if not args.receipt.exists() and not args.receipt.is_symlink():
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        with args.receipt.open("x") as output:
            output.write(json.dumps(result, indent=2) + "\n")
    return 0 if result["status"] == "REGISTRY_PAYLOAD_VERIFIED" else 1


if __name__ == "__main__":
    sys.exit(main())
