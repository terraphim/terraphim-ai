#!/usr/bin/env python3
r"""Central release coordinator: approval-gated publish state machine.

Owns the single lifecycle for a release across two central channels
(github_release, r2_stable_manifest) and three downstream channels
(homebrew_tap_pr, aur_terraphim_clients_bin, omarchy_terraphim_clients_bin):

    planned -> staged -> verified -> promoting -> promoted
                                              \-> failed
    {planned,staged,verified,promoting,failed} -> superseded

This module is dependency-light (Python stdlib only). Schema validation of
the release manifest is delegated to scripts/validate-release-manifest.py
via subprocess so this module never imports jsonschema directly.

State is persisted under a caller-supplied --state-dir as:

    state.json               coordinator bookkeeping (status, generation, ...)
    manifest.json             frozen candidate/verified release manifest
    assets/<basename>         immutable staged artifact bytes (content-addressed)
    downstream-handoff.json   emitted once status == promoted
    .lock                     exclusive lock file guarding all mutations

All mutating commands are gated by an exclusive lock on .lock and support
optimistic compare-and-swap via --expected-generation. State files are
written with temp-write + fsync + rename + directory-fsync so a crash never
leaves a partially written state.json or asset on disk, and staged asset
bytes are never overwritten once written (drift is rejected, not silently
clobbered).
"""

from __future__ import annotations

import argparse
import contextlib
import fcntl
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[3]
DEFAULT_SCHEMA = ROOT / ".release/release-manifest.schema.json"
DEFAULT_VALIDATOR = ROOT / "scripts/validate-release-manifest.py"

SOURCE_REPOS = ("terraphim-ai", "terraphim-clients")
CENTRAL_CHANNELS = ("github_release", "r2_stable_manifest")
DOWNSTREAM_CHANNELS = (
    "homebrew_tap_pr",
    "aur_terraphim_clients_bin",
    "omarchy_terraphim_clients_bin",
)

# Which components each downstream channel actually consumes. Homebrew ships
# the server plus both client binaries; AUR and Omarchy package only the
# terraphim-clients-owned binaries (their channel names say "clients_bin").
# Assets are further filtered by os: none of these three channels take a
# Windows asset.
DOWNSTREAM_CHANNEL_COMPONENTS = {
    "homebrew_tap_pr": frozenset(
        {"terraphim-server", "terraphim-agent", "terraphim-grep"}
    ),
    "aur_terraphim_clients_bin": frozenset({"terraphim-agent", "terraphim-grep"}),
    "omarchy_terraphim_clients_bin": frozenset({"terraphim-agent", "terraphim-grep"}),
}
DOWNSTREAM_CHANNEL_OS = {
    "homebrew_tap_pr": frozenset({"linux", "macos"}),
    "aur_terraphim_clients_bin": frozenset({"linux"}),
    "omarchy_terraphim_clients_bin": frozenset({"linux"}),
}

SIGNATURE_EXEMPT_FORMATS = frozenset({"rb"})
SIGNED_MARKER = "zipsign-embedded-ed25519"

# zipsign (https://github.com/Kijewski/zipsign) only understands these two
# container formats -- it is not a general-purpose signer. The release
# manifest schema's `format` enum also allows deb/rpm/pkg.tar.zst/exe/dmg,
# none of which zipsign can sign or verify: `zipsign sign tar` embeds a
# signature by appending to the byte stream unconditionally, with no
# validation that the input is actually a gzipped tar, so pointing it at a
# .deb (an `ar` archive) silently produces a file `dpkg-deb` still opens
# but `ar t` reports as a malformed archive -- a corrupted "signed" asset
# that both zipsign and this coordinator would otherwise report as
# success. Formats outside this map must fail closed in sign-assets/verify
# rather than be routed through zipsign at all.
ZIPSIGN_FORMAT_SUBCOMMAND = {
    "tar.gz": "tar",
    "zip": "zip",
}

ACTIVE_STATUSES = ("planned", "staged", "verified", "promoting")
TERMINAL_STATUSES = ("promoted", "failed", "superseded")
ALL_STATUSES = ACTIVE_STATUSES + TERMINAL_STATUSES

COORDINATOR_SCHEMA_VERSION = "1.1.0"


class CoordinatorError(RuntimeError):
    """A fail-closed contract violation (bad transition, drift, stale CAS)."""


class StaleGenerationError(CoordinatorError):
    """The caller's --expected-generation no longer matches on-disk state."""


class LockHeldError(CoordinatorError):
    """Another process holds the exclusive lock on this state directory."""


# ---------------------------------------------------------------------------
# Atomic filesystem helpers
# ---------------------------------------------------------------------------


def _fsync_dir(directory: Path) -> None:
    dir_fd = os.open(str(directory), os.O_RDONLY)
    try:
        os.fsync(dir_fd)
    finally:
        os.close(dir_fd)


def atomic_write_bytes(path: Path, data: bytes) -> None:
    """Write data to path via temp-write + fsync + rename + directory-fsync."""
    directory = path.parent
    directory.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(dir=str(directory), prefix=".tmp-", suffix=".part")
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp_name, path)
    except BaseException:
        with contextlib.suppress(OSError):
            os.unlink(tmp_name)
        raise
    _fsync_dir(directory)


def atomic_write_json(path: Path, obj: Any) -> None:
    atomic_write_bytes(
        path, (json.dumps(obj, indent=2, sort_keys=True) + "\n").encode("utf-8")
    )


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_of_file(path: Path) -> tuple[str, int]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
            size += len(chunk)
    return digest.hexdigest(), size


def atomic_copy_file(src: Path, dst: Path) -> tuple[str, int]:
    """Stream-copy src into dst atomically, returning (sha256, size) of src."""
    directory = dst.parent
    directory.mkdir(parents=True, exist_ok=True)
    digest = hashlib.sha256()
    size = 0
    fd, tmp_name = tempfile.mkstemp(dir=str(directory), prefix=".tmp-", suffix=".part")
    try:
        with os.fdopen(fd, "wb") as out, src.open("rb") as inp:
            for chunk in iter(lambda: inp.read(1024 * 1024), b""):
                digest.update(chunk)
                size += len(chunk)
                out.write(chunk)
            out.flush()
            os.fsync(out.fileno())
        os.replace(tmp_name, dst)
    except BaseException:
        with contextlib.suppress(OSError):
            os.unlink(tmp_name)
        raise
    _fsync_dir(directory)
    return digest.hexdigest(), size


def safe_asset_name(name: str) -> str:
    """Reject any name that is not a bare, non-traversing basename."""
    if not name or name in (".", ".."):
        raise CoordinatorError(f"unsafe asset name: {name!r}")
    if os.path.basename(name) != name:
        raise CoordinatorError(f"unsafe asset name (must be a bare basename): {name!r}")
    return name


def is_orphan_temp_name(name: str) -> bool:
    """Crash-leftover temp files from atomic_write_bytes/atomic_copy_file."""
    return name.startswith(".tmp-") and name.endswith(".part")


def clean_orphan_temp_files(directory: Path) -> list[str]:
    """Unlink crash-leftover .tmp-*.part files. Must be called under the lock."""
    if not directory.is_dir():
        return []
    removed = []
    for entry in directory.iterdir():
        if (
            entry.is_file()
            and not entry.is_symlink()
            and is_orphan_temp_name(entry.name)
        ):
            entry.unlink()
            removed.append(entry.name)
    return removed


@contextlib.contextmanager
def exclusive_lock(state_dir: Path, timeout_seconds: float = 0.0):
    state_dir.mkdir(parents=True, exist_ok=True)
    lock_path = state_dir / ".lock"
    deadline = time.monotonic() + timeout_seconds
    with open(lock_path, "a+") as handle:
        try:
            while True:
                try:
                    fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except BlockingIOError:
                    if time.monotonic() >= deadline:
                        raise LockHeldError(
                            f"lock held by another process: {lock_path}"
                        ) from None
                    time.sleep(0.05)
            yield
        finally:
            with contextlib.suppress(OSError):
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


# ---------------------------------------------------------------------------
# State load/save
# ---------------------------------------------------------------------------


def state_path(state_dir: Path) -> Path:
    return state_dir / "state.json"


def manifest_path(state_dir: Path) -> Path:
    return state_dir / "manifest.json"


def assets_dir(state_dir: Path) -> Path:
    return state_dir / "assets"


def downstream_handoff_path(state_dir: Path) -> Path:
    return state_dir / "downstream-handoff.json"


def manifest_signature_path(state_dir: Path) -> Path:
    return state_dir / "manifest.json.sig"


def load_state(state_dir: Path) -> dict[str, Any]:
    path = state_path(state_dir)
    if not path.exists():
        raise CoordinatorError(
            f"no coordinator state at {state_dir} (run 'plan' first)"
        )
    state = read_json(path)
    # Forward-fill R1 coordinator artifacts so an interrupted promotion can
    # be resumed after this schema upgrade without re-staging immutable
    # bytes.  This is in-memory until the next successful mutation.
    phases = state.setdefault("phases", {})
    phases.setdefault(
        "source_landing",
        {
            "status": "complete"
            if set(state.get("staged_source_repos", [])) == set(SOURCE_REPOS)
            else "pending",
            "sources": {},
        },
    )
    phases.setdefault(
        "ci",
        {
            "status": "verified" if state.get("manifest_sha256") else "pending",
            "manifest_sha256": state.get("manifest_sha256"),
        },
    )
    phases.setdefault("release_creation", {"status": "pending"})
    phases.setdefault("package_production", {"status": "pending"})
    phases.setdefault("release_reconciliation", {"status": "pending"})
    phases.setdefault(
        "promotion",
        {
            "status": "in_progress"
            if state.get("status") in ("promoting", "promoted")
            else "pending"
        },
    )
    phases.setdefault("publication", {"status": "pending", "channels": {}})
    phases.setdefault("verification", {"status": "pending", "channels": {}})
    state["coordinator_schema_version"] = COORDINATOR_SCHEMA_VERSION
    return state


def save_state(state_dir: Path, state: dict[str, Any]) -> None:
    atomic_write_json(state_path(state_dir), state)


def check_generation(state: dict[str, Any], expected_generation: int | None) -> None:
    if expected_generation is not None and state["generation"] != expected_generation:
        raise StaleGenerationError(
            f"stale generation: expected {expected_generation}, current {state['generation']}"
        )


def append_history(
    state: dict[str, Any], command: str, from_status: str, to_status: str
) -> None:
    state["generation"] += 1
    state["updated_at"] = now_iso()
    state["history"].append(
        {
            "generation": state["generation"],
            "command": command,
            "from": from_status,
            "to": to_status,
            "at": state["updated_at"],
        }
    )


def now_iso() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def require_status(
    state: dict[str, Any], allowed: tuple[str, ...], command: str
) -> None:
    if state["status"] not in allowed:
        raise CoordinatorError(
            f"illegal transition: {command!r} requires status in {allowed}, "
            f"current status is {state['status']!r}"
        )


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------


def cmd_plan(args: argparse.Namespace) -> dict[str, Any]:
    state_dir = Path(args.state_dir)
    sources = read_json(Path(args.sources_file))
    if set(sources) != {"terraphim-ai", "terraphim-clients"}:
        raise CoordinatorError(
            f"sources must contain exactly terraphim-ai and terraphim-clients, got {sorted(sources)}"
        )
    for repo, source in sources.items():
        for field in ("gitea_sha", "github_sha", "tree_sha", "workflow_run_id"):
            if field not in source:
                raise CoordinatorError(
                    f"sources.{repo} missing required field {field!r}"
                )

    with exclusive_lock(state_dir, args.lock_timeout):
        path = state_path(state_dir)
        if path.exists():
            existing = read_json(path)
            frozen = {
                "release_version": existing["release_version"],
                "release_tag": existing["release_tag"],
                "sources": existing["sources"],
            }
            candidate = {
                "release_version": args.release_version,
                "release_tag": args.release_tag,
                "sources": sources,
            }
            if frozen != candidate:
                raise CoordinatorError(
                    "plan drift: existing coordinator state was frozen with different "
                    "release_version/release_tag/sources; refusing to re-plan in place "
                    "(use a new --state-dir or 'supersede' the old coordination record)"
                )
            return existing

        state = {
            "coordinator_schema_version": COORDINATOR_SCHEMA_VERSION,
            "generation": 0,
            "status": "planned",
            "release_version": args.release_version,
            "release_tag": args.release_tag,
            "sources": sources,
            "manifest_sha256": None,
            "staged_source_repos": [],
            "approval": None,
            "central_channels": {},
            "downstream_channels": {},
            "downstream_handoff": None,
            # Keep the lifecycle dimensions independent.  The top-level
            # status is only the coarse state-machine cursor; these records
            # are the durable evidence for each materially different phase.
            "phases": {
                "source_landing": {"status": "pending", "sources": {}},
                "ci": {"status": "pending", "manifest_sha256": None},
                "release_creation": {"status": "pending"},
                "package_production": {"status": "pending"},
                "release_reconciliation": {"status": "pending"},
                "promotion": {"status": "pending"},
                "publication": {"status": "pending", "channels": {}},
                "verification": {"status": "pending", "channels": {}},
            },
            "failure": None,
            "superseded": None,
            "created_at": now_iso(),
            "updated_at": now_iso(),
            "history": [
                {
                    "generation": 0,
                    "command": "plan",
                    "from": None,
                    "to": "planned",
                    "at": now_iso(),
                }
            ],
        }
        save_state(state_dir, state)
        return state


def _load_candidate_manifest(
    manifest_file: Path, state: dict[str, Any]
) -> dict[str, Any]:
    candidate = read_json(manifest_file)
    if not isinstance(candidate, dict):
        raise CoordinatorError("manifest must be a JSON object")
    if candidate.get("release_version") != state["release_version"]:
        raise CoordinatorError(
            f"manifest release_version {candidate.get('release_version')!r} does not match "
            f"frozen plan {state['release_version']!r}"
        )
    if candidate.get("release_tag") != state["release_tag"]:
        raise CoordinatorError(
            f"manifest release_tag {candidate.get('release_tag')!r} does not match "
            f"frozen plan {state['release_tag']!r}"
        )
    if candidate.get("sources") != state["sources"]:
        raise CoordinatorError(
            "manifest sources do not match frozen plan sources (SHAs/tree/run-id drift)"
        )
    return candidate


def cmd_stage(args: argparse.Namespace) -> dict[str, Any]:
    state_dir = Path(args.state_dir)
    artifact_dir = Path(args.artifact_dir)
    manifest_file = Path(args.manifest)
    source_repo = args.source_repo
    if source_repo not in SOURCE_REPOS:
        raise CoordinatorError(
            f"--source-repo must be one of {SOURCE_REPOS}, got {source_repo!r}"
        )

    if not artifact_dir.is_dir() or artifact_dir.is_symlink():
        raise CoordinatorError(
            f"artifact-dir must be an existing, non-symlink directory: {artifact_dir}"
        )

    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)
        require_status(state, ("planned", "staged"), "stage")

        candidate = _load_candidate_manifest(manifest_file, state)
        all_assets = candidate.get("assets")
        if not isinstance(all_assets, list) or not all_assets:
            raise CoordinatorError("manifest.assets must be a non-empty array")

        # Producer artifacts are downloaded into one directory per source
        # repo (terraphim-ai, terraphim-clients); a single `stage` call only
        # ever inspects the one directory it was given, so it must only
        # require the subset of manifest assets that repo actually owns.
        assets = [
            (index, asset)
            for index, asset in enumerate(all_assets)
            if isinstance(asset, dict) and asset.get("source_repo") == source_repo
        ]
        if not assets:
            raise CoordinatorError(
                f"manifest.assets contains no assets with source_repo == {source_repo!r}"
            )

        target_dir = assets_dir(state_dir)
        target_dir.mkdir(parents=True, exist_ok=True)
        clean_orphan_temp_files(target_dir)

        for index, asset in assets:
            if not isinstance(asset, dict):
                raise CoordinatorError(f"assets[{index}] must be an object")
            name = safe_asset_name(str(asset.get("name", "")))
            declared_sha256 = asset.get("sha256")
            declared_size = asset.get("size_bytes")

            source_path = artifact_dir / name
            if source_path.is_symlink() or not source_path.is_file():
                raise CoordinatorError(
                    f"assets[{index}] ({name}): missing regular file in artifact-dir"
                )

            actual_sha256, actual_size = sha256_of_file(source_path)
            if declared_sha256 is not None and actual_sha256 != declared_sha256:
                raise CoordinatorError(
                    f"assets[{index}] ({name}): declared sha256 {declared_sha256} does not "
                    f"match actual {actual_sha256}"
                )
            if declared_size is not None and actual_size != declared_size:
                raise CoordinatorError(
                    f"assets[{index}] ({name}): declared size_bytes {declared_size} does not "
                    f"match actual {actual_size}"
                )

            staged_path = target_dir / name
            if staged_path.exists():
                if staged_path.is_symlink():
                    raise CoordinatorError(
                        f"staged asset must not be a symlink: {name}"
                    )
                staged_sha256, staged_size = sha256_of_file(staged_path)
                if staged_sha256 != actual_sha256 or staged_size != actual_size:
                    raise CoordinatorError(
                        f"assets[{index}] ({name}): staged bytes already exist and differ "
                        "from the newly presented file (fail closed, never overwritten)"
                    )
                # Identical bytes already staged: idempotent no-op for this asset.
                continue

            copied_sha256, _ = atomic_copy_file(source_path, staged_path)
            if copied_sha256 != actual_sha256:  # pragma: no cover - defensive
                raise CoordinatorError(
                    f"assets[{index}] ({name}): copy integrity check failed"
                )

        previous_manifest_bytes = (
            manifest_path(state_dir).read_bytes()
            if manifest_path(state_dir).exists()
            else None
        )
        new_manifest_bytes = manifest_file.read_bytes()
        manifest_changed = previous_manifest_bytes != new_manifest_bytes
        if manifest_changed:
            atomic_write_bytes(manifest_path(state_dir), new_manifest_bytes)

        already_staged_repos = set(state.get("staged_source_repos", []))
        newly_staged_repo = source_repo not in already_staged_repos
        state["staged_source_repos"] = sorted(already_staged_repos | {source_repo})
        landing = state["phases"]["source_landing"]
        landing["sources"][source_repo] = {
            "status": "landed",
            "asset_names": sorted(asset["name"] for _, asset in assets),
            "recorded_at": now_iso(),
        }
        if set(state["staged_source_repos"]) == set(SOURCE_REPOS):
            landing["status"] = "complete"

        if state["status"] == "planned":
            append_history(state, f"stage:{source_repo}", "planned", "staged")
            state["status"] = "staged"
            save_state(state_dir, state)
        elif manifest_changed or newly_staged_repo:
            append_history(state, f"stage:{source_repo}", "staged", "staged")
            save_state(state_dir, state)
        # else: fully idempotent repeat, no mutation, no generation bump.
        return state


def _run_validator(manifest_file: Path, schema: Path, validator: Path) -> None:
    result = subprocess.run(
        [sys.executable, str(validator), "--schema", str(schema), str(manifest_file)],
        check=False,
        text=True,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise CoordinatorError(
            "release manifest failed schema/business validation: "
            + (result.stderr.strip() or result.stdout.strip())
        )


def _require_zipsign_bin(zipsign_bin: str) -> str:
    resolved = shutil.which(zipsign_bin) or (
        zipsign_bin if Path(zipsign_bin).is_file() else None
    )
    if resolved is None:
        raise CoordinatorError(
            f"zipsign binary not found (fail closed, no signing/verification performed): {zipsign_bin!r}"
        )
    return resolved


def _extract_verifying_key(
    zipsign_bin: str, private_key_file: Path, out_dir: Path
) -> Path:
    if not private_key_file.is_file():
        raise CoordinatorError(
            f"signing private key file not found: {private_key_file}"
        )
    verifying_key = out_dir / "extracted-verifying.key"
    result = subprocess.run(
        [
            zipsign_bin,
            "gen-key",
            "--extract",
            "-f",
            str(private_key_file),
            str(verifying_key),
        ],
        check=False,
        text=True,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise CoordinatorError(
            "failed to extract verifying key from signing private key (fail closed): "
            + (result.stderr.strip() or result.stdout.strip())
        )
    return verifying_key


def _zipsign_subcommand_for_format(index: int, name: str, fmt: str) -> str:
    """Map a manifest asset's declared format to the zipsign subcommand
    that actually understands its container. zipsign only supports .zip
    and gzipped .tar -- routing any other format (deb/rpm/pkg.tar.zst/
    exe/dmg) through it does not fail loudly: `zipsign sign tar` embeds a
    signature by appending to the byte stream unconditionally, silently
    producing a corrupted asset (e.g. a .deb that `ar t` reports as a
    malformed archive) while both zipsign and this coordinator would
    otherwise consider it a successfully signed, verified asset."""
    subcommand = ZIPSIGN_FORMAT_SUBCOMMAND.get(fmt)
    if subcommand is None:
        raise CoordinatorError(
            f"assets[{index}] ({name}): zipsign has no signing/verification support for "
            f"format {fmt!r} (only {sorted(ZIPSIGN_FORMAT_SUBCOMMAND)} are supported); "
            "fail closed rather than corrupt this asset or fabricate a signature -- add a "
            "format-appropriate signing mechanism to the coordinator before including this "
            "asset format, or list it as signature-exempt if it carries its own out-of-band "
            "provenance"
        )
    return subcommand


def _zipsign_is_signed(
    zipsign_bin: str, subcommand: str, asset_path: Path, verifying_key: Path
) -> bool:
    result = subprocess.run(
        [zipsign_bin, "verify", subcommand, str(asset_path), str(verifying_key), "-q"],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=60,
    )
    return result.returncode == 0


def _sign_manifest_detached(
    zipsign_bin: str, manifest_file: Path, signature_file: Path, private_key_file: Path
) -> None:
    """Create a deterministic detached Ed25519 signature for manifest.json."""
    with tempfile.TemporaryDirectory(prefix="manifest-signature-") as tmp:
        generated = Path(tmp) / "manifest.json.sig"
        result = subprocess.run(
            [
                zipsign_bin,
                "sign",
                "separate",
                "--context",
                "terraphim-release-manifest-v1",
                "--output",
                str(generated),
                "--force",
                str(manifest_file),
                str(private_key_file),
            ],
            check=False,
            text=True,
            capture_output=True,
            timeout=30,
        )
        if result.returncode != 0:
            raise CoordinatorError(
                "failed to create detached manifest signature (fail closed): "
                + (result.stderr.strip() or result.stdout.strip())
            )
        atomic_write_bytes(signature_file, generated.read_bytes())


def _verify_manifest_detached(
    zipsign_bin: str, manifest_file: Path, signature_file: Path, verifying_key: Path
) -> None:
    if signature_file.is_symlink() or not signature_file.is_file():
        raise CoordinatorError(
            "detached manifest signature is missing or not a regular file"
        )
    result = subprocess.run(
        [
            zipsign_bin,
            "verify",
            "separate",
            "--context",
            "terraphim-release-manifest-v1",
            "--quiet",
            str(manifest_file),
            str(signature_file),
            str(verifying_key),
        ],
        check=False,
        text=True,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise CoordinatorError(
            "detached manifest signature failed cryptographic verification (fail closed): "
            + (result.stderr.strip() or result.stdout.strip())
        )


def cmd_sign_assets(args: argparse.Namespace) -> dict[str, Any]:
    """Embed a real Ed25519 signature (zipsign) into each staged asset that
    requires one, then rewrite the frozen manifest's declared sha256/size to
    the post-signature bytes so that verify freezes -- and approval binds
    to -- the exact bytes that will be published."""
    state_dir = Path(args.state_dir)
    private_key_file = Path(args.private_key_file)
    zipsign_bin = _require_zipsign_bin(args.zipsign_bin)

    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)
        require_status(state, ("staged",), "sign-assets")

        manifest_file = manifest_path(state_dir)
        if not manifest_file.exists():
            raise CoordinatorError(
                "no staged manifest.json to sign (run 'stage' first)"
            )
        manifest = read_json(manifest_file)
        assets = manifest.get("assets")
        if not isinstance(assets, list) or not assets:
            raise CoordinatorError("manifest.assets must be a non-empty array")

        target_dir = assets_dir(state_dir)
        clean_orphan_temp_files(target_dir)

        with tempfile.TemporaryDirectory(prefix="zipsign-verifying-key-") as tmp:
            verifying_key = _extract_verifying_key(
                zipsign_bin, private_key_file, Path(tmp)
            )

            changed = False
            for index, asset in enumerate(assets):
                fmt = asset.get("format")
                if fmt in SIGNATURE_EXEMPT_FORMATS:
                    continue
                name = safe_asset_name(str(asset["name"]))
                subcommand = _zipsign_subcommand_for_format(index, name, fmt)
                path = target_dir / name
                if path.is_symlink() or not path.is_file():
                    raise CoordinatorError(
                        f"assets[{index}] ({name}): staged file missing or not a regular file"
                    )

                if _zipsign_is_signed(zipsign_bin, subcommand, path, verifying_key):
                    # Crash recovery: a prior invocation may have atomically
                    # landed the signed asset and died before rewriting the
                    # manifest. Reconcile the BOM to those verified bytes.
                    signed_sha256, signed_size = sha256_of_file(path)
                    if (
                        asset.get("sha256") != signed_sha256
                        or asset.get("size_bytes") != signed_size
                        or asset.get("signature") != SIGNED_MARKER
                    ):
                        asset["sha256"] = signed_sha256
                        asset["size_bytes"] = signed_size
                        asset["signature"] = SIGNED_MARKER
                        changed = True
                    continue

                # zipsign writes in place. Sign a same-basename temporary copy
                # and only atomically replace the staged object after its
                # signature verifies, so interruption never leaves half-signed
                # package bytes behind.
                with tempfile.TemporaryDirectory(prefix="zipsign-asset-") as sign_tmp:
                    signed_candidate = Path(sign_tmp) / name
                    shutil.copyfile(path, signed_candidate)
                    result = subprocess.run(
                        [
                            zipsign_bin,
                            "sign",
                            subcommand,
                            str(signed_candidate),
                            str(private_key_file),
                            "-f",
                        ],
                        check=False,
                        text=True,
                        capture_output=True,
                        timeout=120,
                    )
                    if result.returncode != 0:
                        raise CoordinatorError(
                            f"assets[{index}] ({name}): zipsign signing failed (fail closed): "
                            + (result.stderr.strip() or result.stdout.strip())
                        )
                    if not _zipsign_is_signed(
                        zipsign_bin, subcommand, signed_candidate, verifying_key
                    ):
                        raise CoordinatorError(
                            f"assets[{index}] ({name}): signature did not verify immediately after signing"
                        )
                    atomic_write_bytes(path, signed_candidate.read_bytes())

                signed_sha256, signed_size = sha256_of_file(path)
                asset["sha256"] = signed_sha256
                asset["size_bytes"] = signed_size
                asset["signature"] = SIGNED_MARKER
                changed = True

        if changed:
            atomic_write_bytes(
                manifest_path(state_dir),
                (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode("utf-8"),
            )
        # The stable manifest is a release artifact in its own right.  Sign
        # its final bytes after all embedded asset signatures have updated
        # the BOM, using a filename-independent context so retries reproduce
        # byte-for-byte identical detached signatures.
        signature_file = manifest_signature_path(state_dir)
        previous_signature = (
            signature_file.read_bytes() if signature_file.exists() else None
        )
        _sign_manifest_detached(
            zipsign_bin, manifest_path(state_dir), signature_file, private_key_file
        )
        signature_changed = previous_signature != signature_file.read_bytes()

        package_phase = state["phases"]["package_production"]
        phase_changed = package_phase.get("status") != "complete"
        if phase_changed:
            package_phase.update(
                {
                    "status": "complete",
                    "asset_names": sorted(asset["name"] for asset in assets),
                    "manifest_signature": signature_file.name,
                    "recorded_at": now_iso(),
                }
            )
        if changed or signature_changed or phase_changed:
            append_history(state, "sign-assets", "staged", "staged")
            save_state(state_dir, state)
        return state


def cmd_verify(args: argparse.Namespace) -> dict[str, Any]:
    state_dir = Path(args.state_dir)
    schema = Path(args.schema)
    validator = Path(args.validator)
    zipsign_bin = _require_zipsign_bin(args.zipsign_bin)

    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)
        require_status(state, ("staged", "verified"), "verify")

        manifest_file = manifest_path(state_dir)
        if not manifest_file.exists():
            raise CoordinatorError(
                "no staged manifest.json to verify (run 'stage' first)"
            )

        _run_validator(manifest_file, schema, validator)

        manifest = read_json(manifest_file)
        if manifest.get("release_version") != state["release_version"]:
            raise CoordinatorError("manifest release_version drifted from frozen plan")
        if manifest.get("release_tag") != state["release_tag"]:
            raise CoordinatorError("manifest release_tag drifted from frozen plan")
        if manifest.get("sources") != state["sources"]:
            raise CoordinatorError(
                "manifest sources drifted from frozen plan (SHA/run-id mismatch)"
            )

        assets = manifest["assets"]
        target_dir = assets_dir(state_dir)
        clean_orphan_temp_files(target_dir)
        staged_names = set()
        if target_dir.is_dir():
            staged_names = {p.name for p in target_dir.iterdir()}

        with tempfile.TemporaryDirectory(prefix="zipsign-verifying-key-") as tmp:
            verifying_key = (
                _extract_verifying_key(
                    zipsign_bin, Path(args.private_key_file), Path(tmp)
                )
                if args.private_key_file
                else None
            )

            expected_names = set()
            for index, asset in enumerate(assets):
                name = safe_asset_name(asset["name"])
                expected_names.add(name)
                path = target_dir / name
                if path.is_symlink() or not path.is_file():
                    raise CoordinatorError(
                        f"assets[{index}] ({name}): staged file missing or not a regular file"
                    )
                actual_sha256, actual_size = sha256_of_file(path)
                if actual_sha256 != asset["sha256"]:
                    raise CoordinatorError(
                        f"assets[{index}] ({name}): staged sha256 does not match manifest"
                    )
                if actual_size != asset["size_bytes"]:
                    raise CoordinatorError(
                        f"assets[{index}] ({name}): staged size does not match manifest"
                    )

                if asset.get("format") not in SIGNATURE_EXEMPT_FORMATS:
                    subcommand = _zipsign_subcommand_for_format(
                        index, name, asset.get("format")
                    )
                    if verifying_key is None:
                        raise CoordinatorError(
                            f"assets[{index}] ({name}): requires signature verification but no "
                            "--private-key-file was provided (fail closed)"
                        )
                    if not _zipsign_is_signed(
                        zipsign_bin, subcommand, path, verifying_key
                    ):
                        raise CoordinatorError(
                            f"assets[{index}] ({name}): embedded zipsign signature failed cryptographic "
                            "verification (fail closed)"
                        )

            if verifying_key is None:
                raise CoordinatorError(
                    "detached manifest signature verification requires --private-key-file (fail closed)"
                )
            _verify_manifest_detached(
                zipsign_bin,
                manifest_file,
                manifest_signature_path(state_dir),
                verifying_key,
            )

        extra = {n for n in staged_names if not is_orphan_temp_name(n)} - expected_names
        if extra:
            raise CoordinatorError(
                f"staged assets directory has extra files not in manifest: {sorted(extra)}"
            )
        missing = expected_names - staged_names
        if missing:
            raise CoordinatorError(
                f"staged assets directory is missing manifest files: {sorted(missing)}"
            )

        manifest_sha256 = hashlib.sha256(manifest_file.read_bytes()).hexdigest()

        if state["status"] == "verified":
            if state["manifest_sha256"] != manifest_sha256:
                raise CoordinatorError(
                    "verified manifest digest changed on re-verify; this should be "
                    "impossible once verified and indicates drift"
                )
            return state

        state["manifest_sha256"] = manifest_sha256
        state["phases"]["ci"] = {
            "status": "verified",
            "manifest_sha256": manifest_sha256,
            "manifest_signature_sha256": sha256_of_file(
                manifest_signature_path(state_dir)
            )[0],
            "recorded_at": now_iso(),
        }
        append_history(state, "verify", "staged", "verified")
        state["status"] = "verified"
        save_state(state_dir, state)
        return state


def cmd_approve(args: argparse.Namespace) -> dict[str, Any]:
    state_dir = Path(args.state_dir)
    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)
        require_status(state, ("verified", "promoting"), "approve")

        if args.manifest_sha256 != state["manifest_sha256"]:
            raise CoordinatorError(
                "approval not bound to current manifest digest: "
                f"approval digest {args.manifest_sha256} != verified digest {state['manifest_sha256']}"
            )

        existing = state.get("approval")
        if state["status"] == "promoting":
            if not existing or existing.get("manifest_sha256") != args.manifest_sha256:
                raise CoordinatorError(
                    "cannot replace or create approval after promotion has begun"
                )
            return state  # durable approval already authorized this exact digest
        if (
            existing
            and existing["approved_by"] == args.approved_by
            and existing["manifest_sha256"] == args.manifest_sha256
            and existing["note"] == args.note
        ):
            return state  # identical idempotent re-approval, no mutation

        state["approval"] = {
            "approved_by": args.approved_by,
            "manifest_sha256": args.manifest_sha256,
            "approved_at": now_iso(),
            "note": args.note,
        }
        state["phases"]["promotion"] = {
            "status": "approved",
            "manifest_sha256": args.manifest_sha256,
            "approved_by": args.approved_by,
            "recorded_at": now_iso(),
        }
        append_history(state, "approve", "verified", "verified")
        save_state(state_dir, state)
        return state


def cmd_promote_begin(args: argparse.Namespace) -> dict[str, Any]:
    state_dir = Path(args.state_dir)
    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)

        if state["status"] == "promoting":
            return state  # idempotent resume

        require_status(state, ("verified",), "promote-begin")
        approval = state.get("approval")
        if not approval or approval.get("manifest_sha256") != state["manifest_sha256"]:
            raise CoordinatorError(
                "promote-begin requires an approval bound to the current verified manifest digest"
            )

        # Defensive: never clobber a central_channels record that a fresh
        # 'verified' state didn't already have (should always be {} coming
        # from a clean plan/verify, but never discard a non-empty record).
        state.setdefault("central_channels", {})
        state["phases"]["promotion"].update(
            {"status": "in_progress", "started_at": now_iso()}
        )
        append_history(state, "promote-begin", "verified", "promoting")
        state["status"] = "promoting"
        save_state(state_dir, state)
        return state


def _record_central_event(args: argparse.Namespace, event: str) -> dict[str, Any]:
    """Record publication or terminal verification without conflating them."""
    state_dir = Path(args.state_dir)
    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)
        require_status(state, ("promoting", "promoted"), f"record-{event}")
        if (
            event == "verification"
            and args.outcome == "success"
            and (
                state["phases"]["publication"]["channels"]
                .get(args.channel, {})
                .get("outcome")
                != "success"
            )
        ):
            raise CoordinatorError(
                f"cannot record successful verification for {args.channel!r} before publication success"
            )

        channel_events = state["central_channels"].setdefault(args.channel, {})
        existing = channel_events.get(event)
        candidate = {
            "outcome": args.outcome,
            "detail": args.detail,
            "recorded_at": now_iso(),
        }
        if existing and existing.get("outcome") == "success":
            if args.outcome != "success":
                raise CoordinatorError(
                    f"immutable {event} success for {args.channel!r} cannot be downgraded"
                )
            return state
        channel_events[event] = candidate

        phase = state["phases"][event]
        phase["channels"][args.channel] = candidate
        if all(
            phase["channels"].get(channel, {}).get("outcome") == "success"
            for channel in CENTRAL_CHANNELS
        ):
            phase["status"] = "complete"
        elif args.outcome == "failure":
            phase["status"] = "incomplete"
        else:
            phase["status"] = "in_progress"

        previous_status = state["status"]
        append_history(
            state,
            f"record-{event}:{args.channel}:{args.outcome}",
            previous_status,
            previous_status,
        )

        all_verified = all(
            state["central_channels"]
            .get(channel, {})
            .get("verification", {})
            .get("outcome")
            == "success"
            for channel in CENTRAL_CHANNELS
        )
        if all_verified:
            state["history"][-1]["to"] = "promoted"
            state["status"] = "promoted"
            state["phases"]["promotion"]["status"] = "complete"
            state["phases"]["promotion"]["completed_at"] = now_iso()
        save_state(state_dir, state)
        return state


def cmd_record_publication(args: argparse.Namespace) -> dict[str, Any]:
    return _record_central_event(args, "publication")


def cmd_record_verification(args: argparse.Namespace) -> dict[str, Any]:
    return _record_central_event(args, "verification")


def cmd_record_phase(args: argparse.Namespace) -> dict[str, Any]:
    """Record release creation and package production as independent phases."""
    state_dir = Path(args.state_dir)
    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)
        require_status(state, ("promoting",), "record-phase")

        if args.phase == "release_reconciliation" and (
            state["phases"]["release_creation"].get("outcome") != "success"
        ):
            raise CoordinatorError(
                "release reconciliation cannot complete before release creation succeeds"
            )
        record = state["phases"][args.phase]
        if record.get("outcome") == "success":
            if args.outcome != "success":
                raise CoordinatorError(
                    f"immutable {args.phase} success cannot be downgraded"
                )
            return state
        record.update(
            {
                "status": "complete" if args.outcome == "success" else "incomplete",
                "outcome": args.outcome,
                "detail": args.detail,
                "evidence": args.evidence,
                "recorded_at": now_iso(),
            }
        )
        append_history(
            state,
            f"record-phase:{args.phase}:{args.outcome}",
            "promoting",
            "promoting",
        )
        save_state(state_dir, state)
        return state


def cmd_handoff_downstream(args: argparse.Namespace) -> dict[str, Any]:
    state_dir = Path(args.state_dir)
    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)
        require_status(state, ("promoted",), "handoff-downstream")

        manifest = read_json(manifest_path(state_dir))

        def assets_for_channel(channel: str) -> list[dict[str, Any]]:
            components = DOWNSTREAM_CHANNEL_COMPONENTS[channel]
            oses = DOWNSTREAM_CHANNEL_OS[channel]
            return [
                asset
                for asset in manifest["assets"]
                if asset.get("component") in components and asset.get("os") in oses
            ]

        payload = {
            "schema_version": "1.0.0",
            "release_tag": state["release_tag"],
            "release_version": state["release_version"],
            "manifest_sha256": state["manifest_sha256"],
            "correlation_id": hashlib.sha256(
                f"{state['release_tag']}:{state['manifest_sha256']}".encode()
            ).hexdigest(),
            "channels": [
                {"channel": channel, "assets": assets_for_channel(channel)}
                for channel in DOWNSTREAM_CHANNELS
            ],
            "emitted_at": now_iso(),
        }

        handoff_file = downstream_handoff_path(state_dir)
        existing_emitted = state.get("downstream_handoff")
        if existing_emitted and handoff_file.exists():
            existing = read_json(handoff_file)
            if (
                existing.get("schema_version") == payload["schema_version"]
                and existing.get("release_tag") == payload["release_tag"]
                and existing.get("manifest_sha256") == payload["manifest_sha256"]
                and existing.get("correlation_id") == payload["correlation_id"]
            ):
                return state  # idempotent: already emitted for this exact manifest

        atomic_write_json(handoff_file, payload)
        state["downstream_handoff"] = {
            "emitted": True,
            "emitted_at": payload["emitted_at"],
            "path": str(handoff_file),
        }
        append_history(state, "handoff-downstream", "promoted", "promoted")
        save_state(state_dir, state)
        return state


def cmd_record_downstream_channel(args: argparse.Namespace) -> dict[str, Any]:
    """Record dispatch acceptance/failure without claiming terminal success."""
    if args.channel not in DOWNSTREAM_CHANNELS:
        raise CoordinatorError(
            f"record-downstream-channel is for downstream channels only, got {args.channel!r}"
        )

    state_dir = Path(args.state_dir)
    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)
        require_status(state, ("promoted",), "record-downstream-channel")

        downstream_channels = state.setdefault("downstream_channels", {})
        record = downstream_channels.setdefault(args.channel, {})
        existing = record.get("dispatch")
        if existing and existing.get("outcome") == "accepted":
            if args.outcome != "accepted":
                raise CoordinatorError(
                    f"accepted dispatch for {args.channel!r} cannot be downgraded"
                )
            return state
        record["dispatch"] = {
            "outcome": args.outcome,
            "detail": args.detail,
            "recorded_at": now_iso(),
        }
        append_history(
            state,
            f"record-downstream-channel:{args.channel}:{args.outcome}",
            "promoted",
            "promoted",
        )
        save_state(state_dir, state)
        return state


def cmd_verify_downstream_proof(args: argparse.Namespace) -> dict[str, Any]:
    """Verify a downstream-produced proof bound to this exact release."""
    proof = read_json(Path(args.proof))
    state_dir = Path(args.state_dir)
    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)
        require_status(state, ("promoted",), "verify-downstream-proof")
        dispatch = (
            state.get("downstream_channels", {})
            .get(args.channel, {})
            .get("dispatch", {})
        )
        if dispatch.get("outcome") != "accepted":
            raise CoordinatorError(
                f"cannot verify {args.channel!r}: no accepted dispatch is recorded"
            )
        handoff = read_json(downstream_handoff_path(state_dir))
        expected_entry = next(
            entry for entry in handoff["channels"] if entry["channel"] == args.channel
        )
        expected_assets = sorted(
            (
                {"name": asset["name"], "sha256": asset["sha256"]}
                for asset in expected_entry["assets"]
            ),
            key=lambda asset: asset["name"],
        )
        actual_assets = proof.get("assets")
        if not isinstance(actual_assets, list):
            raise CoordinatorError("downstream proof assets must be an array")
        actual_assets = sorted(
            (
                {
                    "name": str(asset.get("name", "")),
                    "sha256": str(asset.get("sha256", "")),
                }
                for asset in actual_assets
                if isinstance(asset, dict)
            ),
            key=lambda asset: asset["name"],
        )
        required = {
            "schema_version": "1.0.0",
            "channel": args.channel,
            "release_tag": state["release_tag"],
            "manifest_sha256": state["manifest_sha256"],
            "correlation_id": handoff["correlation_id"],
            "outcome": "success",
        }
        mismatches = {
            key: (value, proof.get(key))
            for key, value in required.items()
            if proof.get(key) != value
        }
        if mismatches or actual_assets != expected_assets:
            raise CoordinatorError(
                "downstream proof is not bound to the frozen release: "
                f"field_mismatches={mismatches}, assets_match={actual_assets == expected_assets}"
            )

        record = state["downstream_channels"].setdefault(args.channel, {})
        existing = record.get("verification")
        proof_sha256, _ = sha256_of_file(Path(args.proof))
        if existing and existing.get("outcome") == "success":
            if existing.get("proof_sha256") != proof_sha256:
                raise CoordinatorError(
                    f"immutable downstream verification for {args.channel!r} has different proof bytes"
                )
            return state
        record["verification"] = {
            "outcome": "success",
            "proof_sha256": proof_sha256,
            "recorded_at": now_iso(),
        }
        append_history(
            state,
            f"verify-downstream-proof:{args.channel}",
            "promoted",
            "promoted",
        )
        save_state(state_dir, state)
        return state


def cmd_fail(args: argparse.Namespace) -> dict[str, Any]:
    state_dir = Path(args.state_dir)
    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)

        if (
            state["status"] == "failed"
            and state.get("failure", {}).get("reason") == args.reason
        ):
            return state  # idempotent re-fail with identical reason

        require_status(state, ACTIVE_STATUSES, "fail")
        previous = state["status"]
        state["failure"] = {
            "reason": args.reason,
            "at_status": previous,
            "failed_at": now_iso(),
        }
        append_history(state, "fail", previous, "failed")
        state["status"] = "failed"
        save_state(state_dir, state)
        return state


def cmd_supersede(args: argparse.Namespace) -> dict[str, Any]:
    state_dir = Path(args.state_dir)
    with exclusive_lock(state_dir, args.lock_timeout):
        state = load_state(state_dir)
        check_generation(state, args.expected_generation)

        existing = state.get("superseded")
        if (
            state["status"] == "superseded"
            and existing
            and existing["by"] == args.by
            and existing["reason"] == args.reason
        ):
            return state  # idempotent re-supersede with identical target/reason

        require_status(state, ACTIVE_STATUSES + ("failed",), "supersede")

        previous = state["status"]
        state["superseded"] = {"by": args.by, "reason": args.reason, "at": now_iso()}
        append_history(state, "supersede", previous, "superseded")
        state["status"] = "superseded"
        save_state(state_dir, state)
        return state


def cmd_inspect(args: argparse.Namespace) -> dict[str, Any]:
    state_dir = Path(args.state_dir)
    return load_state(state_dir)


NEXT_ACTIONS = {
    "planned": ["stage"],
    "staged": ["stage (more assets)", "sign-assets", "verify"],
    "verified": ["approve", "promote-begin"],
    "promoting": [
        "record-phase",
        "record-publication",
        "record-verification",
        "fail",
    ],
    "promoted": ["handoff-downstream"],
    "failed": ["supersede"],
    "superseded": [],
}


def cmd_resume(args: argparse.Namespace) -> dict[str, Any]:
    state = cmd_inspect(args)
    state = dict(state)
    state["next_actions"] = NEXT_ACTIONS.get(state["status"], [])
    return state


# ---------------------------------------------------------------------------
# CLI wiring
# ---------------------------------------------------------------------------


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    def add_common(p: argparse.ArgumentParser) -> None:
        p.add_argument("--state-dir", required=True)
        p.add_argument("--lock-timeout", type=float, default=0.0)

    def add_cas(p: argparse.ArgumentParser) -> None:
        p.add_argument("--expected-generation", type=int, default=None)

    p_plan = sub.add_parser(
        "plan", help="Freeze version/tag/sources for a new coordination record"
    )
    add_common(p_plan)
    p_plan.add_argument("--release-version", required=True)
    p_plan.add_argument("--release-tag", required=True)
    p_plan.add_argument(
        "--sources-file",
        required=True,
        help="JSON file with terraphim-ai/terraphim-clients source records",
    )
    p_plan.set_defaults(func=cmd_plan)

    p_stage = sub.add_parser(
        "stage",
        help="Stage an already-downloaded immutable producer artifact directory",
    )
    add_common(p_stage)
    add_cas(p_stage)
    p_stage.add_argument(
        "--manifest", required=True, help="Candidate release-manifest.json path"
    )
    p_stage.add_argument(
        "--artifact-dir", required=True, help="Immutable producer artifact directory"
    )
    p_stage.add_argument(
        "--source-repo",
        required=True,
        choices=SOURCE_REPOS,
        help="Which source repo's assets --artifact-dir contains (each producer is staged separately)",
    )
    p_stage.set_defaults(func=cmd_stage)

    p_sign = sub.add_parser(
        "sign-assets",
        help="Embed a real zipsign signature into each staged asset that requires one",
    )
    add_common(p_sign)
    add_cas(p_sign)
    p_sign.add_argument(
        "--private-key-file", required=True, help="zipsign Ed25519 private key file"
    )
    p_sign.add_argument("--zipsign-bin", default="zipsign")
    p_sign.set_defaults(func=cmd_sign_assets)

    p_verify = sub.add_parser(
        "verify",
        help="Validate the frozen manifest and staged assets, then freeze the digest",
    )
    add_common(p_verify)
    add_cas(p_verify)
    p_verify.add_argument("--schema", default=str(DEFAULT_SCHEMA))
    p_verify.add_argument("--validator", default=str(DEFAULT_VALIDATOR))
    p_verify.add_argument(
        "--private-key-file",
        default=None,
        help="zipsign key file used to cryptographically verify each signed asset (fails closed if omitted and a non-rb asset is present)",
    )
    p_verify.add_argument("--zipsign-bin", default="zipsign")
    p_verify.set_defaults(func=cmd_verify)

    p_approve = sub.add_parser(
        "approve", help="Record an approval bound to the verified manifest digest"
    )
    add_common(p_approve)
    add_cas(p_approve)
    p_approve.add_argument("--approved-by", required=True)
    p_approve.add_argument("--manifest-sha256", required=True)
    p_approve.add_argument("--note", default=None)
    p_approve.set_defaults(func=cmd_approve)

    p_promote = sub.add_parser(
        "promote-begin", help="Begin central promotion (requires bound approval)"
    )
    add_common(p_promote)
    add_cas(p_promote)
    p_promote.set_defaults(func=cmd_promote_begin)

    def add_central_record_parser(name: str, help_text: str, func: Any) -> None:
        record = sub.add_parser(name, help=help_text)
        add_common(record)
        add_cas(record)
        record.add_argument("--channel", required=True, choices=CENTRAL_CHANNELS)
        record.add_argument("--outcome", required=True, choices=("success", "failure"))
        record.add_argument("--detail", default=None)
        record.set_defaults(func=func)

    add_central_record_parser(
        "record-publication",
        "Record that a central channel publication was attempted/completed",
        cmd_record_publication,
    )
    add_central_record_parser(
        "record-verification",
        "Record terminal verification of a central channel",
        cmd_record_verification,
    )

    p_phase = sub.add_parser(
        "record-phase", help="Record release creation or exact-inventory reconciliation"
    )
    add_common(p_phase)
    add_cas(p_phase)
    p_phase.add_argument(
        "--phase", required=True, choices=("release_creation", "release_reconciliation")
    )
    p_phase.add_argument("--outcome", required=True, choices=("success", "failure"))
    p_phase.add_argument("--detail", default=None)
    p_phase.add_argument("--evidence", default=None)
    p_phase.set_defaults(func=cmd_record_phase)

    p_handoff = sub.add_parser(
        "handoff-downstream",
        help="Emit the downstream handoff record (requires promoted)",
    )
    add_common(p_handoff)
    add_cas(p_handoff)
    p_handoff.set_defaults(func=cmd_handoff_downstream)

    p_record_downstream = sub.add_parser(
        "record-downstream-channel",
        help="Record a downstream dispatch outcome (audit trail only, non-gating)",
    )
    add_common(p_record_downstream)
    add_cas(p_record_downstream)
    p_record_downstream.add_argument(
        "--channel", required=True, choices=DOWNSTREAM_CHANNELS
    )
    p_record_downstream.add_argument(
        "--outcome", required=True, choices=("accepted", "failure")
    )
    p_record_downstream.add_argument("--detail", default=None)
    p_record_downstream.set_defaults(func=cmd_record_downstream_channel)

    p_verify_downstream = sub.add_parser(
        "verify-downstream-proof",
        help="Verify a downstream terminal proof bound to the correlated release",
    )
    add_common(p_verify_downstream)
    add_cas(p_verify_downstream)
    p_verify_downstream.add_argument(
        "--channel", required=True, choices=DOWNSTREAM_CHANNELS
    )
    p_verify_downstream.add_argument("--proof", required=True)
    p_verify_downstream.set_defaults(func=cmd_verify_downstream_proof)

    p_fail = sub.add_parser("fail", help="Mark the coordination record failed")
    add_common(p_fail)
    add_cas(p_fail)
    p_fail.add_argument("--reason", required=True)
    p_fail.set_defaults(func=cmd_fail)

    p_supersede = sub.add_parser(
        "supersede", help="Mark this coordination record superseded by another"
    )
    add_common(p_supersede)
    add_cas(p_supersede)
    p_supersede.add_argument("--by", required=True)
    p_supersede.add_argument("--reason", required=True)
    p_supersede.set_defaults(func=cmd_supersede)

    p_inspect = sub.add_parser("inspect", help="Print the current state (read-only)")
    add_common(p_inspect)
    p_inspect.set_defaults(func=cmd_inspect)

    p_resume = sub.add_parser(
        "resume", help="Print the current state plus recommended next actions"
    )
    add_common(p_resume)
    p_resume.set_defaults(func=cmd_resume)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        result = args.func(args)
    except StaleGenerationError as exc:
        print(f"release-coordinator: {exc}", file=sys.stderr)
        return 3
    except LockHeldError as exc:
        print(f"release-coordinator: {exc}", file=sys.stderr)
        return 4
    except CoordinatorError as exc:
        print(f"release-coordinator: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
