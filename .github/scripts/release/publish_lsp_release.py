#!/usr/bin/env python3
"""Qualify and publish immutable LSP bytes; never move tags or replace assets.

The resumable draft/readback policy follows the existing VS Code publisher.
Source provenance is the actual checkout, including its repository. This helper
cannot bridge a private source commit into a public tag: that integration must
be reviewed separately, and a missing or different public tag fails closed.
"""
from __future__ import annotations

import argparse
import hashlib
import hmac
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[3]
SHA = re.compile(r"[0-9a-f]{40}")
TAG = re.compile(r"terraphim_lsp-v([0-9]+\.[0-9]+\.[0-9]+)(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?")
REPOSITORY = "terraphim/terraphim-ai"
SOURCE_REPOSITORIES = {f"https://{host}/{REPOSITORY}" for host in ("github.com", "git.terraphim.cloud")}
RAW_ASSETS = tuple(sorted("terraphim-lsp-" + target for target in (
    "aarch64-apple-darwin", "aarch64-unknown-linux-musl", "universal-apple-darwin",
    "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl",
)))
MANIFEST = "lsp-release-manifest.json"
RELEASE_FILES = (*RAW_ASSETS, "checksums.txt", MANIFEST)
COMMANDS = (
    ("cargo", "fmt", "-p", "terraphim_lsp_core", "-p", "terraphim_lsp", "--", "--check"),
    ("cargo", "clippy", "--locked", "-p", "terraphim_lsp_core", "-p", "terraphim_lsp",
     "--all-targets", "--all-features", "--", "-D", "warnings"),
    ("cargo", "test", "--locked", "-p", "terraphim_lsp_core", "-p", "terraphim_lsp", "--all-features"),
)


def run(*args, cwd=None):
    return subprocess.check_output(list(args), cwd=cwd, text=True, timeout=300).strip()


def git(root, *args):
    return run("git", *args, cwd=root)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)


def write_new(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8") as output:
        output.write(json.dumps(value, indent=2, sort_keys=True) + "\n")


def version(root):
    crate = tomllib.loads((root / "crates/terraphim_lsp/Cargo.toml").read_text())["package"]["version"]
    if crate == {"workspace": True}:
        crate = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    return crate


def validate_tag(tag, crate_version):
    match = TAG.fullmatch(tag)
    if not match or match[1] != crate_version:
        raise ValueError("Exact LSP tag and crate version must match")


def source_repository(root):
    url = git(root, "remote", "get-url", "origin").removesuffix(".git").rstrip("/")
    if url.startswith("git@"):
        host, path = url[4:].split(":", 1)
        url = f"https://{host}/{path}"
    if url not in SOURCE_REPOSITORIES:
        raise ValueError("Source origin is not an approved Terraphim source repository")
    return url


def source_identity(root, tag, source, require_tag=True):
    validate_tag(tag, version(root))
    if not SHA.fullmatch(source) or git(root, "rev-parse", "HEAD") != source:
        raise ValueError("Checkout does not match exact source commit")
    if git(root, "status", "--porcelain", "--untracked-files=no"):
        raise ValueError("Tracked source is dirty")
    # Fail closed if reviewed main history was not fetched or the source is not
    # merged there. A matching version string alone is never source approval.
    subprocess.run(["git", "merge-base", "--is-ancestor", source, "origin/main"],
                   cwd=root, check=True, timeout=30)
    local = subprocess.run(["git", "rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}"],
                           cwd=root, text=True, capture_output=True, timeout=30)
    if local.returncode == 0:
        if local.stdout.strip() != source:
            raise ValueError("Local release tag differs from exact source")
    elif require_tag:
        raise ValueError("Publishing requires the existing exact source tag")
    return {"repository": source_repository(root), "commit": source,
            "tag": tag, "trackedTreeDirty": False}


def toolchain(root):
    return tomllib.loads((root / "rust-toolchain.toml").read_text())["toolchain"]["channel"]


def qualification_commands(root):
    return [["rustup", "run", toolchain(root), *command] for command in COMMANDS]


def qualify(root, tag, source, output, require_tag):
    if output.exists():
        raise ValueError("Choose a new qualification directory")
    identity = source_identity(root, tag, source, require_tag)
    output.mkdir(parents=True)
    result = {"schema": 1, "status": "FAIL", "source": identity, "commands": []}
    try:
        for index, command in enumerate(qualification_commands(root)):
            path = output / f"command-{index}.log"
            with path.open("xb") as log:
                # Execute the real crate tests, including save/export/stdio tests.
                status = subprocess.run(command, cwd=root, stdout=log, stderr=subprocess.STDOUT,
                                        timeout=1800, check=False)
            result["commands"].append({"argv": command, "exitCode": status.returncode,
                                       "log": path.name, "sha256": digest(path)})
            if status.returncode:
                raise ValueError(f"Qualification command {index} failed; inspect {path.name}")
        source_identity(root, tag, source, require_tag)
        result["status"] = "PASS"
    finally:
        write_new(output / "qualification.json", result)


def verify_qualification(root, folder, identity):
    report = read_json(folder / "qualification.json")
    commands = report.get("commands", [])
    expected = qualification_commands(root)
    if (type(report.get("schema")) is not int or report.get("schema") != 1 or report.get("status") != "PASS"
            or report.get("source") != identity or len(commands) != len(expected)):
        raise ValueError("Qualification does not identify this successful exact source")
    for index, (record, argv) in enumerate(zip(commands, expected)):
        log = folder / f"command-{index}.log"
        if (record.get("argv") != argv or type(record.get("exitCode")) is not int or record.get("exitCode") != 0
                or record.get("log") != log.name or not log.is_file() or log.is_symlink()
                or not isinstance(record.get("sha256"), str)
                or not hmac.compare_digest(record["sha256"], digest(log))):
            raise ValueError("Qualification command or saved log differs")
    return report


def asset_paths(folder, names):
    if folder.is_symlink() or not folder.is_dir():
        raise ValueError("Assets must be a regular directory")
    if set(p.name for p in folder.iterdir()) != set(names):
        raise ValueError("Unexpected or missing release assets")
    paths = [folder / name for name in names]
    if any(p.is_symlink() or not p.is_file() or p.stat().st_size == 0 for p in paths):
        raise ValueError("Release assets must be nonempty regular files")
    return paths


def asset_records(paths):
    return [{"name": p.name, "sha256": digest(p), "bytes": p.stat().st_size} for p in paths]


def verify_sums(folder):
    expected = "".join(f"{digest(folder / name)}  {name}\n" for name in RAW_ASSETS)
    if (folder / "checksums.txt").read_text(encoding="ascii") != expected:
        raise ValueError("Checksums differ from the exact six raw binaries")


def assemble(root, folder, qualification, tag, source, signing, require_tag):
    identity = source_identity(root, tag, source, require_tag)
    report = verify_qualification(root, qualification, identity)
    paths = asset_paths(folder, (*RAW_ASSETS, "checksums.txt"))
    verify_sums(folder)
    if signing not in ("signed", "unsigned"):
        raise ValueError("Unknown universal macOS signing state")
    result = {"schema": 1, "component": "terraphim-lsp", "source": identity,
              "version": version(root), "toolchain": toolchain(root),
              "qualification": report, "universalMacosSigning": signing,
              "assets": asset_records(paths)}
    write_new(folder / MANIFEST, result)
    return result


def verify_bundle(root, folder, qualification, tag, source):
    identity = source_identity(root, tag, source)
    report = verify_qualification(root, qualification, identity)
    paths = asset_paths(folder, RELEASE_FILES)
    manifest = read_json(folder / MANIFEST)
    verify_sums(folder)
    if (manifest.get("schema") != 1 or manifest.get("component") != "terraphim-lsp"
            or manifest.get("source") != identity or manifest.get("version") != version(root)
            or manifest.get("toolchain") != toolchain(root) or manifest.get("qualification") != report
            or manifest.get("universalMacosSigning") not in ("signed", "unsigned")
            or manifest.get("assets") != asset_records(paths[:-1])):
        raise ValueError("Manifest differs from qualified source or frozen assets")
    return manifest, paths


def gh(*args):
    return run("gh", *args)


def api(path):
    return json.loads(gh("api", "--hostname", "github.com", path), object_pairs_hook=unique_object)


def verify_remote_tag(repo, tag, source):
    value = api(f"repos/{repo}/git/ref/tags/{tag}")["object"]
    for _ in range(5):
        if value["type"] == "commit" and value["sha"] == source:
            return
        if value["type"] != "tag" or not SHA.fullmatch(value["sha"]):
            break
        value = api(f"repos/{repo}/git/tags/{value['sha']}")["object"]
    raise ValueError("Public tag does not identify the qualified source; no source bridge is configured")


def select_release(pages, tag):
    matches = [item for page in pages for item in page if item.get("tag_name") == tag]
    if len(matches) > 1:
        raise ValueError("Ambiguous release tag")
    return matches[0] if matches else None


def release(repo, tag):
    # Listing includes private drafts, unlike the anonymous exact-tag endpoint.
    pages = json.loads(gh("api", "--hostname", "github.com", "--paginate", "--slurp",
                          f"repos/{repo}/releases?per_page=100"))
    return select_release(pages, tag)


def ownership(files):
    manifest = next(p for p in files if p.name == MANIFEST)
    return f"<!-- terraphim-lsp manifest-sha256={digest(manifest)} -->"


def resume_plan(current, tag, files):
    body = current.get("body")
    if not isinstance(body, str):
        raise ValueError("Existing release has no ownership marker")
    if (current.get("tag_name") != tag or type(current.get("prerelease")) is not bool or not current["prerelease"]
            or type(current.get("draft")) is not bool
            or ownership(files) not in body.replace("\r", "").splitlines()):
        raise ValueError("Existing release does not belong to this exact candidate manifest")
    assets = current["assets"]
    names = [item["name"] for item in assets]
    wanted = {p.name: p for p in files}
    if len(names) != len(set(names)) or not set(names).issubset(wanted):
        raise ValueError("Unexpected or duplicate release assets")
    for item in assets:
        path = wanted[item["name"]]
        if (item.get("state") != "uploaded" or item.get("size") != path.stat().st_size
                or item.get("digest") not in (None, "sha256:" + digest(path))):
            raise ValueError("Existing asset differs or is incomplete; refusing replacement")
    missing = [p for p in files if p.name not in names]
    if missing and not current["draft"]:
        raise ValueError("Published releases are immutable; missing assets require a new version")
    return missing


def verify_bytes(expected, downloaded):
    if downloaded.is_symlink() or not downloaded.is_file() or expected.read_bytes() != downloaded.read_bytes():
        raise ValueError(f"Downloaded {expected.name} differs; refusing replacement")


def anonymous_download(url, destination):
    parsed = urlparse(url)
    if (parsed.scheme != "https" or parsed.hostname != "github.com" or parsed.username
            or parsed.password or parsed.query or parsed.fragment or destination.exists()
            or destination.is_symlink()):
        raise ValueError("Expected a new anonymous HTTPS GitHub download")
    # No curl config, credentials, cookies or netrc; redirect protocols stay HTTPS.
    status = subprocess.run([
        "curl", "--disable", "--fail", "--silent", "--show-error", "--location",
        "--proto", "=https", "--proto-redir", "=https", "--max-redirs", "5",
        "--connect-timeout", "15", "--max-time", "300", "--max-filesize", str(128 * 1024 * 1024),
        "--retry", "2", "--output", str(destination), "--write-out", "%{http_code}", url,
    ], text=True, capture_output=True, timeout=1000)
    if status.returncode or status.stdout != "200":
        raise ValueError(f"Anonymous readback failed: HTTP {status.stdout}; curl {status.returncode}")


def recovered_release(repo, tag):
    # Draft creation/listing is eventually consistent. Bounded reads recover the
    # original draft; never retry create blindly or replace an existing release.
    for attempt in range(5):
        current = release(repo, tag)
        if current is not None:
            return current
        if attempt < 4:
            time.sleep(2)
    raise ValueError("Created draft is not yet readable; rerun the same qualified artifact")


def publish(repo, tag, files, source, notes):
    if repo != REPOSITORY or not TAG.fullmatch(tag) or not SHA.fullmatch(source):
        raise ValueError("Unexpected public release repository, tag or source")
    metadata = api(f"repos/{repo}")
    if type(metadata.get("private")) is not bool or metadata["private"] or metadata.get("full_name") != repo:
        raise ValueError("Exact public repository is required")
    verify_remote_tag(repo, tag, source)
    current = release(repo, tag)
    with tempfile.TemporaryDirectory(prefix="lsp-publication-") as folder:
        temp = Path(folder)
        if current is None:
            note_file = temp / "notes.md"
            note_file.write_text(ownership(files) + "\n\n" + notes, encoding="utf-8")
            gh("release", "create", tag, "--repo", f"github.com/{repo}", "--verify-tag",
               "--draft", "--prerelease", "--latest=false", "--title", tag, "--notes-file", str(note_file))
            current = recovered_release(repo, tag)
        missing = resume_plan(current, tag, files)
        # Verify all existing bytes before any missing-asset write. Matching
        # metadata alone is insufficient, even when GitHub supplies a digest.
        for path in files:
            if path not in missing:
                gh("release", "download", tag, "--repo", f"github.com/{repo}",
                   "--pattern", path.name, "--dir", str(temp))
                verify_bytes(path, temp / path.name)
        for path in missing:
            gh("release", "upload", tag, str(path), "--repo", f"github.com/{repo}")
            gh("release", "download", tag, "--repo", f"github.com/{repo}",
               "--pattern", path.name, "--dir", str(temp))
            verify_bytes(path, temp / path.name)
        current = recovered_release(repo, tag)
        if resume_plan(current, tag, files):
            raise ValueError("Draft asset set is incomplete")
        verify_remote_tag(repo, tag, source)
        if current["draft"]:
            gh("release", "edit", tag, "--repo", f"github.com/{repo}", "--verify-tag", "--draft=false")
        published = recovered_release(repo, tag)
        if published["draft"] or resume_plan(published, tag, files):
            raise ValueError("Release is not public with the exact asset set")
        verify_remote_tag(repo, tag, source)
        for path in files:
            destination = temp / ("public-" + path.name)
            anonymous_download(f"https://github.com/{repo}/releases/download/{tag}/{path.name}", destination)
            verify_bytes(path, destination)
    return {"schema": 1, "status": "PUBLISHED_PUBLIC_VERIFIED", "repository": repo,
            "tag": tag, "sourceCommit": source, "releaseId": published["id"], "assets": asset_records(files)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("source", "qualify", "assemble", "publish"))
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--require-tag", action="store_true")
    parser.add_argument("--assets", type=Path)
    parser.add_argument("--qualification", type=Path)
    parser.add_argument("--signing", choices=("signed", "unsigned"))
    parser.add_argument("--repo", default=REPOSITORY)
    parser.add_argument("--notes", type=Path)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    result = {"schema": 1, "status": "FAIL"}
    try:
        if args.command == "source":
            source_identity(args.root, args.tag, args.source, args.require_tag)
        elif args.command == "qualify":
            qualify(args.root, args.tag, args.source, args.qualification, args.require_tag)
        elif args.command == "assemble":
            assemble(args.root, args.assets, args.qualification, args.tag, args.source,
                     args.signing, args.require_tag)
        else:
            if not args.receipt or args.receipt.exists() or args.receipt.is_symlink():
                raise ValueError("Choose a new publication receipt path")
            verify_bundle(args.root, args.assets, args.qualification, args.tag, args.source)
            # Upload only a revalidated snapshot; later workspace edits cannot
            # replace bytes between qualification and upload.
            with tempfile.TemporaryDirectory(prefix="lsp-frozen-") as folder:
                frozen = Path(folder) / "assets"
                shutil.copytree(args.assets, frozen, symlinks=True)
                _, paths = verify_bundle(args.root, frozen, args.qualification, args.tag, args.source)
                result = publish(args.repo, args.tag, paths, args.source, args.notes.read_text())
            write_new(args.receipt, result)
        return 0
    except (ValueError, KeyError, TypeError, OSError, subprocess.SubprocessError) as error:
        result["error"] = str(error)
        if args.command == "publish" and args.receipt and not args.receipt.exists() and not args.receipt.is_symlink():
            write_new(args.receipt, result)
        print(f"publish_lsp_release: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
