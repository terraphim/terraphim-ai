#!/usr/bin/env python3
"""Publish qualified VSIX bytes through a resumable draft; never replace assets or tags."""
import argparse
import hashlib
from itertools import islice
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
from urllib.parse import urlparse
import zipfile
from xml.etree.ElementTree import ParseError

import verify_vsix
import legacy_assets

REPO_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*")
SHA_RE = re.compile(r"[0-9a-f]{40}")
RELEASE_LOOKUP_DELAYS = (0, 2, 2, 2, 2)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def checksums(text):
    result = {}
    for line in text.splitlines():
        match = re.fullmatch(r"([0-9a-f]{64}) [ *]([A-Za-z0-9][A-Za-z0-9_.-]*)", line)
        if not match or match[2] in result:
            raise ValueError("Malformed or duplicate checksum entry")
        result[match[2]] = match[1]
    if not result:
        raise ValueError("Empty checksum manifest")
    return result


def git(root, *args):
    return subprocess.check_output(["git", *args], cwd=root, text=True).strip()


def check_source(root, report, tag):
    source = report.get("sourceCommit", "")
    if not SHA_RE.fullmatch(source) or report.get("sourceTreeDirty") is not False:
        raise ValueError("Qualification must identify a clean exact source commit")
    if git(root, "rev-parse", "HEAD") != source:
        raise ValueError("Current source differs from qualification")
    if git(root, "status", "--porcelain", "--untracked-files=no"):
        raise ValueError("Tracked source tree is dirty")
    if git(root, "rev-parse", f"refs/tags/{tag}^{{commit}}") != source:
        raise ValueError("Local release tag differs from qualification")
    return source


def verify_package(vsix, manifest, sums, tag, root=verify_vsix.ROOT):
    paths = [vsix, manifest, sums]
    if (len({p.name for p in paths}) != 3 or any(p.is_symlink() or not p.is_file() for p in paths)
            or manifest.name != "release-manifest.json" or sums.name != "SHA256SUMS"
            or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*\.vsix", vsix.name)):
        raise ValueError("Expected separate regular VSIX, release-manifest.json and SHA256SUMS files")
    expected = json.loads((root / "package.json").read_text())
    verify_vsix.check_tag(tag, expected["version"])
    report = json.loads(manifest.read_text())
    source = check_source(root, report, tag)
    actual = verify_vsix.verify(vsix, expected, (root / "media/editor-build.json").read_bytes(), legacy_assets.inputs(root))
    if any(report.get(key) != value for key, value in actual.items()):
        raise ValueError("Qualification report differs from actual VSIX or current source assets")
    wanted = {vsix.name: digest(vsix)}
    found = checksums(sums.read_text())
    if found not in (wanted, {**wanted, manifest.name: digest(manifest)}):
        raise ValueError("Checksum manifest differs from qualified package")
    return report, source, paths


def check_native(receipt, package_sha256):
    if receipt.get("status") != "PASS" or receipt.get("packageSha256") != package_sha256:
        raise ValueError("Native launcher did not pass on this exact VSIX")


def check_native_pair(receipts, package_sha256, platform):
    """Require each independently executed launcher for one exact OS/package."""
    expected = {"lifecycle", "retained-recovery"}
    if (not isinstance(platform, str) or platform not in {"linux", "darwin", "win32"}
            or not isinstance(receipts, list) or len(receipts) != len(expected)
            or any(not isinstance(receipt, dict) for receipt in receipts)):
        raise ValueError("Expected exactly two typed native launcher receipts")
    kinds = [receipt.get("kind") for receipt in receipts]
    if any(not isinstance(kind, str) or kind not in expected for kind in kinds) or len(set(kinds)) != len(expected):
        raise ValueError("Missing, duplicate or unexpected native launcher kind")
    for receipt in receipts:
        if receipt.get("platform") != platform:
            raise ValueError("Native launcher platform differs from its host")
        check_native(receipt, package_sha256)


def gh(*args):
    return subprocess.check_output(["gh", *args], text=True)


def api(path):
    return json.loads(gh("api", "--hostname", "github.com", path))


def select_release(pages, tag):
    found = [item for page in pages for item in page if item["tag_name"] == tag]
    if len(found) > 1:
        raise ValueError("Ambiguous release tag")
    return found[0] if found else None


def release(repo, tag):
    # Listing includes drafts; the public tag endpoint returns 404 for drafts.
    pages = json.loads(gh("api", "--hostname", "github.com", "--paginate", "--slurp",
                          f"repos/{repo}/releases?per_page=100"))
    return select_release(pages, tag)


def recover_visible_release(snapshots):
    """Consume at most five reads; only an absent draft permits another read."""
    for current in islice(snapshots, len(RELEASE_LOOKUP_DELAYS)):
        if current is not None:
            return current
    raise ValueError("Created draft is not yet readable; rerun the same qualified artifact")


def recovered_release(repo, tag):
    """Allow eight seconds of backoff for creation/listing consistency."""
    def snapshots():
        for delay in RELEASE_LOOKUP_DELAYS:
            if delay:
                time.sleep(delay)
            yield release(repo, tag)

    return recover_visible_release(snapshots())


def verify_remote_tag(repo, tag, source):
    value = api(f"repos/{repo}/git/ref/tags/{tag}")["object"]
    for _ in range(5):
        if value["type"] == "commit" and value["sha"] == source:
            return
        if value["type"] != "tag" or not SHA_RE.fullmatch(value["sha"]):
            break
        value = api(f"repos/{repo}/git/tags/{value['sha']}")["object"]
    raise ValueError("Remote tag does not resolve to the qualified source commit")


def resume_plan(current, tag, files):
    if current.get("tag_name") != tag or current.get("prerelease") is not True:
        raise ValueError("Release tag or preview channel differs")
    if type(current.get("draft")) is not bool:
        raise ValueError("Release draft state is missing")
    assets = current["assets"]
    names = [item["name"] for item in assets]
    wanted = {p.name: p for p in files}
    if len(names) != len(set(names)) or not set(names).issubset(wanted):
        raise ValueError("Unexpected or duplicate remote release assets")
    for item in assets:
        if item.get("state") != "uploaded" or item.get("size") != wanted[item["name"]].stat().st_size:
            raise ValueError("Existing asset is incomplete or differs in size; refusing replacement")
        if item.get("digest") not in (None, "sha256:" + digest(wanted[item["name"]])):
            raise ValueError("Existing remote asset digest differs; refusing replacement")
    missing = [p for p in files if p.name not in names]
    if missing and not current["draft"]:
        raise ValueError("Published release has missing assets; refusing repair in place")
    return missing


def anonymous_download(url, destination):
    """Anonymous HTTPS GET, without curl config, auth headers, cookies or netrc."""
    parsed = urlparse(url)
    if (parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password
            or parsed.query or parsed.fragment):
        raise ValueError("Anonymous download requires an ordinary HTTPS URL")
    if destination.exists() or destination.is_symlink():
        raise ValueError("Refusing to overwrite a download")
    result = subprocess.run([
        "curl", "--disable", "--fail", "--silent", "--show-error", "--location",
        "--proto", "=https", "--proto-redir", "=https", "--max-redirs", "5",
        "--connect-timeout", "15", "--max-time", "300", "--max-filesize", str(128 * 1024 * 1024),
        "--retry", "2", "--output", str(destination), "--write-out", "%{http_code}", url,
    ], text=True, capture_output=True)
    if result.returncode or result.stdout != "200":
        raise ValueError(f"Anonymous GET failed: HTTP {result.stdout}; curl {result.returncode}")
    return destination.read_bytes()


def publish(repo, tag, files, source):
    if not REPO_RE.fullmatch(repo):
        raise ValueError("Invalid GitHub repository")
    repository = api(f"repos/{repo}")
    if repository.get("private") is not False or repository.get("full_name", "").lower() != repo.lower():
        raise ValueError("Publication requires the exact public GitHub repository")
    verify_remote_tag(repo, tag, source)
    current = release(repo, tag)
    with tempfile.TemporaryDirectory(prefix="terraphim-publish-") as folder:
        temp = Path(folder)
        if current is None:
            notes = temp / "notes.md"
            notes.write_text(f"Terraphim VS Code preview {tag}; source `{source}`.\n\n"
                             "The VSIX, integrity manifest and checksums are attached. "
                             "Registry availability is verified separately.\n")
            gh("release", "create", tag, "--repo", f"github.com/{repo}", "--verify-tag",
               "--draft", "--prerelease", "--latest=false", "--title", tag, "--notes-file", str(notes))
            current = recovered_release(repo, tag)
        missing = resume_plan(current, tag, files)
        # Verify every existing asset before mutating an incomplete draft.
        for path in files:
            if path not in missing:
                gh("release", "download", tag, "--repo", f"github.com/{repo}",
                   "--pattern", path.name, "--dir", str(temp))
                if (temp / path.name).read_bytes() != path.read_bytes():
                    raise ValueError(f"Existing {path.name} differs; refusing replacement")
        for path in missing:
            gh("release", "upload", tag, str(path), "--repo", f"github.com/{repo}")
            gh("release", "download", tag, "--repo", f"github.com/{repo}",
               "--pattern", path.name, "--dir", str(temp))
            if (temp / path.name).read_bytes() != path.read_bytes():
                raise ValueError("Uploaded asset readback differs")
        current = release(repo, tag)
        if current is None or resume_plan(current, tag, files):
            raise ValueError("Release asset set is incomplete")
        verify_remote_tag(repo, tag, source)
        if current["draft"]:
            gh("release", "edit", tag, "--repo", f"github.com/{repo}", "--verify-tag", "--draft=false")
        published = release(repo, tag)
        if published is None or published["draft"] or resume_plan(published, tag, files):
            raise ValueError("Release is not publicly published with the exact asset set")
        verify_remote_tag(repo, tag, source)
        for path in files:
            url = f"https://github.com/{repo}/releases/download/{tag}/{path.name}"
            if anonymous_download(url, temp / ("public-" + path.name)) != path.read_bytes():
                raise ValueError("Anonymous release readback differs")
    return {"schema": 1, "status": "PUBLISHED_PUBLIC_VERIFIED", "repository": repo,
            "tag": tag, "sourceCommit": source, "releaseId": published["id"],
            "assets": [{"name": p.name, "sha256": digest(p), "bytes": p.stat().st_size} for p in files]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--vsix", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--sums", type=Path, required=True)
    parser.add_argument("--native-receipt", type=Path)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    result = {"schema": 1, "status": "FAILED"}
    try:
        if args.receipt.exists() or args.receipt.is_symlink():
            raise ValueError("Choose a new receipt path to retain prior publication evidence")
        report, source, files = verify_package(args.vsix, args.manifest, args.sums, args.tag)
        if args.native_receipt:
            check_native(json.loads(args.native_receipt.read_text()), report["sha256"])
        # Snapshot validated bytes: subsequent workspace edits cannot alter an upload.
        with tempfile.TemporaryDirectory(prefix="terraphim-qualified-") as folder:
            snapshots = []
            for path in files:
                snapshot = Path(folder) / path.name
                snapshot.write_bytes(path.read_bytes())
                snapshots.append(snapshot)
            verify_package(*snapshots, args.tag)
            result = publish(args.repo, args.tag, snapshots, source)
    except (ValueError, KeyError, TypeError, OSError, zipfile.BadZipFile, ParseError, subprocess.CalledProcessError) as error:
        result["error"] = str(error)
        print(f"publish_release: {error}", file=sys.stderr)
    if not args.receipt.exists() and not args.receipt.is_symlink():
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        with args.receipt.open("x") as output:
            output.write(json.dumps(result, indent=2) + "\n")
    return 0 if result["status"] == "PUBLISHED_PUBLIC_VERIFIED" else 1


if __name__ == "__main__":
    sys.exit(main())
