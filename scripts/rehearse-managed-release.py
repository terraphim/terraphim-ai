#!/usr/bin/env python3
"""Aggregate the end-to-end rehearsal evidence for a managed-package release.

Gitea terraphim-ai#3382 ([MP7]). The central coordinator
(.github/scripts/release/release_coordinator.py) owns the zero-mutation
rehearsal and the approval bound to the verified manifest digest. This script
owns the other half of the rehearsal: it aggregates the runtime evidence each
downstream channel and verification lane already produced, without re-running
any of it and without copying any evidence bytes.

Contract:
  * the coordinator state must be 'verified' (plan -> stage -> verify done,
    no central or downstream mutation);
  * the evidence bundle must be bound to the same release_tag and
    manifest_sha256 as the frozen coordinator state;
  * every channel in the manifest's downstream_channels and every required
    verification lane must have at least one evidence file, each a regular,
    non-empty file whose SHA-256 matches the declared digest;
  * a channel may be explicitly deferred (with a reason) -- the report then
    records the gap and withholds the approval digest, so a deferred
    rehearsal can never authorize promotion;
  * the report stores only references (relative path + SHA-256 + size), never
    evidence bytes, and is byte-identical on re-run.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

SCHEMA_VERSION = "1.0.0"
REQUIRED_VERIFICATIONS = ("deb_rpm_native", "updater_zero_network_write")
SHA256_HEX_LEN = 64


class RehearsalError(ValueError):
    """The rehearsal cannot produce a complete, verified candidate."""


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise RehearsalError(f"duplicate JSON object key {key!r}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        text = path.read_text()
    except OSError as exc:
        raise RehearsalError(f"cannot read {path}: {exc}") from exc
    try:
        return json.loads(text, object_pairs_hook=reject_duplicate_keys)
    except RehearsalError:
        raise
    except json.JSONDecodeError as exc:
        raise RehearsalError(f"cannot parse {path}: {exc}") from exc


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as exc:
        raise RehearsalError(f"cannot read evidence file {path}: {exc}") from exc
    return digest.hexdigest()


def correlation_id(release_tag: str, manifest_sha256: str) -> str:
    return hashlib.sha256(f"{release_tag}:{manifest_sha256}".encode()).hexdigest()


def validate_state(state: Any) -> dict[str, Any]:
    if not isinstance(state, dict):
        raise RehearsalError("coordinator state must be a JSON object")
    if state.get("status") != "verified":
        raise RehearsalError(
            "coordinator state must be 'verified' before aggregating rehearsal "
            f"evidence; got {state.get('status')!r}"
        )
    manifest_sha256 = state.get("manifest_sha256")
    if not isinstance(manifest_sha256, str) or len(manifest_sha256) != SHA256_HEX_LEN:
        raise RehearsalError("coordinator state has no verified manifest_sha256")
    release_tag = state.get("release_tag")
    if not isinstance(release_tag, str) or not release_tag:
        raise RehearsalError("coordinator state has no release_tag")
    return state


def validate_manifest(manifest: Any) -> list[str]:
    if not isinstance(manifest, dict):
        raise RehearsalError("frozen manifest must be a JSON object")
    channels = manifest.get("downstream_channels")
    if not isinstance(channels, list) or not channels:
        raise RehearsalError("frozen manifest has no downstream_channels")
    for channel in channels:
        if not isinstance(channel, str) or not channel:
            raise RehearsalError("downstream_channels entries must be non-empty strings")
    return channels


def resolve_evidence(
    entry: Any, evidence_root: Path, category: str, seen: set[str]
) -> dict[str, Any]:
    if not isinstance(entry, dict):
        raise RehearsalError(f"{category}: every evidence entry must be an object")
    name = entry.get("name")
    rel = entry.get("path")
    declared = entry.get("sha256")
    if not isinstance(name, str) or not name:
        raise RehearsalError(f"{category}: evidence entry needs a non-empty name")
    if name in seen:
        raise RehearsalError(f"{category}: duplicate evidence name {name!r}")
    seen.add(name)
    if not isinstance(rel, str) or not rel or rel.startswith("/") or ".." in Path(rel).parts:
        raise RehearsalError(f"{category}/{name}: path must be a safe relative path")
    if not isinstance(declared, str) or len(declared) != SHA256_HEX_LEN:
        raise RehearsalError(f"{category}/{name}: sha256 must be 64 lowercase hex")
    path = evidence_root / rel
    if not path.is_file():
        raise RehearsalError(f"{category}/{name}: evidence file not found: {path}")
    size = path.stat().st_size
    if size < 1:
        raise RehearsalError(f"{category}/{name}: evidence file is empty: {path}")
    actual = sha256_file(path)
    if actual != declared:
        raise RehearsalError(
            f"{category}/{name}: evidence digest mismatch: declared {declared}, actual {actual}"
        )
    return {"name": name, "path": rel, "sha256": actual, "size_bytes": size}


def aggregate(args: argparse.Namespace) -> dict[str, Any]:
    state_dir = Path(args.state_dir)
    state = validate_state(load_json(state_dir / "state.json"))
    manifest = load_json(state_dir / "manifest.json")
    channels = validate_manifest(manifest)

    evidence = load_json(Path(args.evidence))
    if not isinstance(evidence, dict):
        raise RehearsalError("evidence bundle must be a JSON object")
    if evidence.get("schema_version") != SCHEMA_VERSION:
        raise RehearsalError(
            f"evidence schema_version must be {SCHEMA_VERSION!r}, "
            f"got {evidence.get('schema_version')!r}"
        )
    if evidence.get("release_tag") != state["release_tag"]:
        raise RehearsalError("evidence release_tag does not match the frozen coordinator state")
    if evidence.get("manifest_sha256") != state["manifest_sha256"]:
        raise RehearsalError(
            "evidence manifest_sha256 does not match the verified coordinator digest"
        )

    deferred = {name: reason for name, reason in args.deferred_channel}
    for channel in deferred:
        if channel not in channels:
            raise RehearsalError(f"cannot defer unknown channel {channel!r}")

    for entry in manifest.get("deferred_channels") or []:
        channel = entry.get("channel")
        reason = entry.get("reason") or ""
        if channel not in channels:
            raise RehearsalError(f"manifest defers unknown channel {channel!r}")
        deferred.setdefault(channel, reason or "deferred in the release manifest")

    channels_evidence = evidence.get("channels") or {}
    verifications_evidence = evidence.get("verifications") or {}
    if not isinstance(channels_evidence, dict) or not isinstance(verifications_evidence, dict):
        raise RehearsalError("evidence 'channels' and 'verifications' must be objects")
    unknown = set(channels_evidence) - set(channels)
    if unknown:
        raise RehearsalError(f"evidence bundle names unknown channels: {sorted(unknown)}")
    unknown = set(verifications_evidence) - set(REQUIRED_VERIFICATIONS)
    if unknown:
        raise RehearsalError(f"evidence bundle names unknown verifications: {sorted(unknown)}")

    evidence_root = Path(args.evidence_root)
    report_channels: dict[str, Any] = {}
    all_verified = True
    for channel in channels:
        if channel in deferred:
            all_verified = False
            report_channels[channel] = {
                "status": "deferred",
                "reason": deferred[channel],
                "evidence": [],
            }
            continue
        entries = channels_evidence.get(channel)
        if not isinstance(entries, list) or not entries:
            raise RehearsalError(f"{channel}: no channel evidence supplied")
        seen: set[str] = set()
        report_channels[channel] = {
            "status": "verified",
            "evidence": [
                resolve_evidence(entry, evidence_root, channel, seen) for entry in entries
            ],
        }

    report_verifications: dict[str, Any] = {}
    for lane in REQUIRED_VERIFICATIONS:
        entries = verifications_evidence.get(lane)
        if not isinstance(entries, list) or not entries:
            raise RehearsalError(f"{lane}: no verification evidence supplied")
        seen = set()
        report_verifications[lane] = {
            "status": "verified",
            "evidence": [
                resolve_evidence(entry, evidence_root, lane, seen) for entry in entries
            ],
        }

    return {
        "schema_version": SCHEMA_VERSION,
        "release_version": state.get("release_version"),
        "release_tag": state["release_tag"],
        "manifest_sha256": state["manifest_sha256"],
        "correlation_id": correlation_id(state["release_tag"], state["manifest_sha256"]),
        "state_status": state["status"],
        "status": "pass" if all_verified else "deferred",
        "approval": state["manifest_sha256"] if all_verified else None,
        "channels": report_channels,
        "verifications": report_verifications,
    }


def render(report: dict[str, Any]) -> str:
    return json.dumps(report, indent=2, sort_keys=True) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--state-dir", required=True, type=Path)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--evidence-root", required=True, type=Path)
    parser.add_argument("--output", type=Path, default=None)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--require-complete", action="store_true")
    parser.add_argument(
        "--deferred-channel",
        action="append",
        nargs=2,
        default=[],
        metavar=("CHANNEL", "REASON"),
        help="record a channel as explicitly deferred; withholds the approval digest",
    )
    args = parser.parse_args(argv)
    output = args.output or (Path(args.state_dir) / "rehearsal-report.json")
    try:
        report = aggregate(args)
    except RehearsalError as exc:
        print(f"rehearsal error: {exc}", file=sys.stderr)
        return 1
    rendered = render(report)
    if args.check:
        current = output.read_text() if output.is_file() else None
        if current != rendered:
            print(f"rehearsal error: report is stale: {output}", file=sys.stderr)
            return 1
        return 0
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(rendered)
    print(rendered, end="")
    if args.require_complete and report["status"] != "pass":
        print("rehearsal incomplete: deferred channels present", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
