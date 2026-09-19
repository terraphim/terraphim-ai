#!/usr/bin/env python3
"""Contract tests for the central release coordinator state machine.

Covers: state-machine legality, compare-and-swap generation guards, atomic
temp-write+fsync+rename semantics, path safety, drift detection (missing /
extra / duplicate / tampered assets), idempotent retries, partial-failure
resume without rebuilding artifacts, approval-manifest-digest binding, and
zero-mutation behaviour before an approval exists.

Requires the same `jsonschema` dependency as
tests/release_manifest_validator_test.py (see
scripts/requirements-release-manifest.txt) because `verify` shells out to
scripts/validate-release-manifest.py.
"""

from __future__ import annotations

import copy
import hashlib
import io
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
COORDINATOR = ROOT / ".github/scripts/release/release_coordinator.py"
SCHEMA = ROOT / ".release/release-manifest.schema.json"

ZIPSIGN_BIN = shutil.which("zipsign")

SOURCES = {
    "terraphim-ai": {
        "gitea_sha": "1111111111111111111111111111111111111111",
        "github_sha": "1111111111111111111111111111111111111111",
        "tree_sha": "2222222222222222222222222222222222222222",
        "workflow_run_id": 33380001,
    },
    "terraphim-clients": {
        "gitea_sha": "3333333333333333333333333333333333333333",
        "github_sha": "3333333333333333333333333333333333333333",
        "tree_sha": "4444444444444444444444444444444444444444",
        "workflow_run_id": 33360001,
    },
}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write_json(path: Path, obj: Any) -> None:
    path.write_text(json.dumps(obj, indent=2), encoding="utf-8")


def run(args: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    # Compatibility helper for the original happy-path tests: the R2 review
    # requires publication and terminal verification to be distinct durable
    # events, so an old logical "record-channel success" now executes both
    # real CLI commands in order. New regression tests call them separately.
    if args and args[0] == "record-channel":
        publication_args = ["record-publication", *args[1:]]
        publication = run(publication_args, cwd=cwd)
        if publication.returncode != 0 or "failure" in args:
            return publication
        verification_args = ["record-verification", *args[1:]]
        return run(verification_args, cwd=cwd)
    return subprocess.run(
        [sys.executable, str(COORDINATOR), *args],
        cwd=str(cwd) if cwd else None,
        check=False,
        text=True,
        capture_output=True,
        timeout=30,
    )


def parse_stdout(result: subprocess.CompletedProcess[str]) -> dict[str, Any]:
    return json.loads(result.stdout)


class CoordinatorFixture(unittest.TestCase):
    """Builds a scratch workspace with two per-producer artifact dirs (matching
    the workflow's real split-download layout: terraphim-ai's assets are
    downloaded separately from terraphim-clients') plus a candidate manifest
    and a real zipsign Ed25519 keypair."""

    def setUp(self) -> None:
        if ZIPSIGN_BIN is None:
            self.skipTest(
                "zipsign binary not on PATH; install with `cargo install zipsign`"
            )

        self.tmp = tempfile.TemporaryDirectory()
        self.work = Path(self.tmp.name)
        self.state_dir = self.work / "state"
        self.artifact_dir_ai = self.work / "artifacts" / "terraphim-ai"
        self.artifact_dir_clients = self.work / "artifacts" / "terraphim-clients"
        self.artifact_dir_ai.mkdir(parents=True)
        self.artifact_dir_clients.mkdir(parents=True)
        self.sources_file = self.work / "sources.json"
        write_json(self.sources_file, SOURCES)

        self.asset_bytes = {
            "terraphim-server-1.2.3-linux-x86_64.tar.gz": b"server-payload",
            "terraphim-agent-1.2.3-linux-x86_64.tar.gz": b"agent-payload",
            "terraphim-grep-1.2.3-linux-x86_64.tar.gz": b"grep-payload",
        }
        # terraphim-server is built by terraphim-ai; agent/grep by terraphim-clients.
        (
            self.artifact_dir_ai / "terraphim-server-1.2.3-linux-x86_64.tar.gz"
        ).write_bytes(self.asset_bytes["terraphim-server-1.2.3-linux-x86_64.tar.gz"])
        for name in (
            "terraphim-agent-1.2.3-linux-x86_64.tar.gz",
            "terraphim-grep-1.2.3-linux-x86_64.tar.gz",
        ):
            (self.artifact_dir_clients / name).write_bytes(self.asset_bytes[name])

        self.manifest_file = self.work / "manifest.json"
        write_json(self.manifest_file, self.build_manifest())

        self.private_key_file = self.work / "zipsign-private.key"
        self.public_key_file = self.work / "zipsign-public.key"
        gen = subprocess.run(
            [
                ZIPSIGN_BIN,
                "gen-key",
                str(self.private_key_file),
                str(self.public_key_file),
            ],
            check=False,
            text=True,
            capture_output=True,
            timeout=30,
        )
        assert gen.returncode == 0, gen.stderr

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def artifact_dir_for(self, source_repo: str) -> Path:
        return (
            self.artifact_dir_ai
            if source_repo == "terraphim-ai"
            else self.artifact_dir_clients
        )

    def build_manifest(self) -> dict[str, Any]:
        def asset(name: str, component: str, repo: str, sha: str) -> dict[str, Any]:
            data = self.asset_bytes[name]
            return {
                "name": name,
                "component": component,
                "format": "tar.gz",
                "target": "x86_64-unknown-linux-gnu",
                "arch": "x86_64",
                "os": "linux",
                "source_repo": repo,
                "source_sha": sha,
                "sha256": sha256_bytes(data),
                "size_bytes": len(data),
                "signature": f"{name}.sig",
            }

        ai_sha = SOURCES["terraphim-ai"]["gitea_sha"]
        clients_sha = SOURCES["terraphim-clients"]["gitea_sha"]
        return {
            "schema_version": "1.0.0",
            "release_version": "1.2.3",
            "release_tag": "v1.2.3",
            "sources": copy.deepcopy(SOURCES),
            "assets": [
                asset(
                    "terraphim-server-1.2.3-linux-x86_64.tar.gz",
                    "terraphim-server",
                    "terraphim-ai",
                    ai_sha,
                ),
                asset(
                    "terraphim-agent-1.2.3-linux-x86_64.tar.gz",
                    "terraphim-agent",
                    "terraphim-clients",
                    clients_sha,
                ),
                asset(
                    "terraphim-grep-1.2.3-linux-x86_64.tar.gz",
                    "terraphim-grep",
                    "terraphim-clients",
                    clients_sha,
                ),
            ],
            "required_channels": [
                "github_release",
                "r2_stable_manifest",
                "homebrew_tap_pr",
                "aur_terraphim_clients_bin",
                "omarchy_terraphim_clients_bin",
            ],
            "central_channels": ["github_release", "r2_stable_manifest"],
            "downstream_channels": [
                "homebrew_tap_pr",
                "aur_terraphim_clients_bin",
                "omarchy_terraphim_clients_bin",
            ],
            "created_at": "2026-09-18T10:00:00Z",
        }

    def plan(self) -> subprocess.CompletedProcess[str]:
        return run(
            [
                "plan",
                "--state-dir",
                str(self.state_dir),
                "--release-version",
                "1.2.3",
                "--release-tag",
                "v1.2.3",
                "--sources-file",
                str(self.sources_file),
            ]
        )

    def stage(
        self,
        manifest: Path | None = None,
        artifact_dir: Path | None = None,
        source_repo: str = "terraphim-ai",
    ) -> subprocess.CompletedProcess[str]:
        return run(
            [
                "stage",
                "--state-dir",
                str(self.state_dir),
                "--manifest",
                str(manifest or self.manifest_file),
                "--artifact-dir",
                str(artifact_dir or self.artifact_dir_for(source_repo)),
                "--source-repo",
                source_repo,
            ]
        )

    def stage_both_producers(self, manifest: Path | None = None) -> None:
        """Exactly the workflow's real sequence: one `stage` call per
        producer directory (terraphim-ai, then terraphim-clients)."""
        first = self.stage(manifest=manifest, source_repo="terraphim-ai")
        self.assertEqual(first.returncode, 0, first.stderr)
        second = self.stage(manifest=manifest, source_repo="terraphim-clients")
        self.assertEqual(second.returncode, 0, second.stderr)

    def sign(self) -> subprocess.CompletedProcess[str]:
        return run(
            [
                "sign-assets",
                "--state-dir",
                str(self.state_dir),
                "--private-key-file",
                str(self.private_key_file),
            ]
        )

    def verify(self) -> subprocess.CompletedProcess[str]:
        return run(
            [
                "verify",
                "--state-dir",
                str(self.state_dir),
                "--private-key-file",
                str(self.private_key_file),
            ]
        )

    def inspect(self) -> dict[str, Any]:
        result = run(["inspect", "--state-dir", str(self.state_dir)])
        self.assertEqual(result.returncode, 0, result.stderr)
        return parse_stdout(result)

    def plan_stage_verify(self) -> str:
        self.assertEqual(self.plan().returncode, 0)
        self.stage_both_producers()
        signed = self.sign()
        self.assertEqual(signed.returncode, 0, signed.stderr)
        result = self.verify()
        self.assertEqual(result.returncode, 0, result.stderr)
        return parse_stdout(result)["manifest_sha256"]

    def approve(
        self, digest: str, approved_by: str = "alex"
    ) -> subprocess.CompletedProcess[str]:
        return run(
            [
                "approve",
                "--state-dir",
                str(self.state_dir),
                "--approved-by",
                approved_by,
                "--manifest-sha256",
                digest,
            ]
        )

    def snapshot_tree(self) -> set[str]:
        """Relative paths of every file under the whole scratch workspace."""
        paths = set()
        for root, _dirs, files in os.walk(self.work):
            for name in files:
                paths.add(str(Path(root, name).relative_to(self.work)))
        return paths


class StateMachineContract(CoordinatorFixture):
    def test_happy_path_reaches_promoted_and_handoff(self) -> None:
        digest = self.plan_stage_verify()
        self.assertEqual(self.approve(digest).returncode, 0)
        self.assertEqual(
            run(["promote-begin", "--state-dir", str(self.state_dir)]).returncode, 0
        )
        self.assertEqual(
            run(
                [
                    "record-channel",
                    "--state-dir",
                    str(self.state_dir),
                    "--channel",
                    "github_release",
                    "--outcome",
                    "success",
                ]
            ).returncode,
            0,
        )
        result = run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "r2_stable_manifest",
                "--outcome",
                "success",
            ]
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(parse_stdout(result)["status"], "promoted")
        state = parse_stdout(result)
        self.assertEqual(state["phases"]["source_landing"]["status"], "complete")
        self.assertEqual(state["phases"]["ci"]["status"], "verified")
        self.assertEqual(state["phases"]["promotion"]["status"], "complete")
        self.assertEqual(state["phases"]["publication"]["status"], "complete")
        self.assertEqual(state["phases"]["verification"]["status"], "complete")

        handoff = run(["handoff-downstream", "--state-dir", str(self.state_dir)])
        self.assertEqual(handoff.returncode, 0, handoff.stderr)
        payload = json.loads((self.state_dir / "downstream-handoff.json").read_text())
        self.assertEqual(
            {c["channel"] for c in payload["channels"]},
            {
                "homebrew_tap_pr",
                "aur_terraphim_clients_bin",
                "omarchy_terraphim_clients_bin",
            },
        )

    def test_illegal_transitions_fail_closed(self) -> None:
        # verify before stage
        self.assertEqual(self.plan().returncode, 0)
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("illegal transition", result.stderr)

        # promote-begin before verify
        self.assertEqual(self.stage().returncode, 0)
        result = run(["promote-begin", "--state-dir", str(self.state_dir)])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("illegal transition", result.stderr)

        # record-channel before promoting
        result = run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "success",
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("illegal transition", result.stderr)

    def test_no_downstream_handoff_before_central_success(self) -> None:
        digest = self.plan_stage_verify()
        self.approve(digest)

        # Not yet promoting.
        result = run(["handoff-downstream", "--state-dir", str(self.state_dir)])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("illegal transition", result.stderr)

        run(["promote-begin", "--state-dir", str(self.state_dir)])
        # Promoting, but no central channel recorded yet.
        result = run(["handoff-downstream", "--state-dir", str(self.state_dir)])
        self.assertNotEqual(result.returncode, 0)

        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "success",
            ]
        )
        # Only one of two central channels succeeded.
        result = run(["handoff-downstream", "--state-dir", str(self.state_dir)])
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.inspect()["status"], "promoting")

    def test_fail_blocked_from_terminal_states(self) -> None:
        digest = self.plan_stage_verify()
        self.approve(digest)
        run(["promote-begin", "--state-dir", str(self.state_dir)])
        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "success",
            ]
        )
        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "r2_stable_manifest",
                "--outcome",
                "success",
            ]
        )
        self.assertEqual(self.inspect()["status"], "promoted")

        result = run(
            ["fail", "--state-dir", str(self.state_dir), "--reason", "too late"]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("illegal transition", result.stderr)

    def test_supersede_blocked_from_promoted(self) -> None:
        digest = self.plan_stage_verify()
        self.approve(digest)
        run(["promote-begin", "--state-dir", str(self.state_dir)])
        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "success",
            ]
        )
        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "r2_stable_manifest",
                "--outcome",
                "success",
            ]
        )

        result = run(
            [
                "supersede",
                "--state-dir",
                str(self.state_dir),
                "--by",
                "v1.2.4",
                "--reason",
                "newer release",
            ]
        )
        self.assertNotEqual(result.returncode, 0)

    def test_supersede_allowed_from_active_and_failed_states(self) -> None:
        self.assertEqual(self.plan().returncode, 0)
        result = run(
            [
                "supersede",
                "--state-dir",
                str(self.state_dir),
                "--by",
                "v1.2.4",
                "--reason",
                "abandoned",
            ]
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(parse_stdout(result)["status"], "superseded")


class CompareAndSwapContract(CoordinatorFixture):
    def test_stale_generation_fails_closed_without_mutating(self) -> None:
        self.assertEqual(self.plan().returncode, 0)
        before = self.inspect()

        result = run(
            [
                "stage",
                "--state-dir",
                str(self.state_dir),
                "--manifest",
                str(self.manifest_file),
                "--artifact-dir",
                str(self.artifact_dir_for("terraphim-ai")),
                "--source-repo",
                "terraphim-ai",
                "--expected-generation",
                "999",
            ]
        )
        self.assertEqual(result.returncode, 3)
        self.assertIn("stale generation", result.stderr)

        after = self.inspect()
        self.assertEqual(before, after)

    def test_matching_generation_succeeds_and_advances(self) -> None:
        self.assertEqual(self.plan().returncode, 0)
        gen = self.inspect()["generation"]
        result = run(
            [
                "stage",
                "--state-dir",
                str(self.state_dir),
                "--manifest",
                str(self.manifest_file),
                "--artifact-dir",
                str(self.artifact_dir_for("terraphim-ai")),
                "--source-repo",
                "terraphim-ai",
                "--expected-generation",
                str(gen),
            ]
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertGreater(self.inspect()["generation"], gen)


class AtomicityAndPathSafetyContract(CoordinatorFixture):
    def test_state_file_is_always_valid_json_after_every_command(self) -> None:
        self.plan()
        self.assertTrue(json.loads(self.state_dir_state_json()))
        self.stage_both_producers()
        self.assertTrue(json.loads(self.state_dir_state_json()))
        self.sign()
        self.assertTrue(json.loads(self.state_dir_state_json()))
        self.verify()
        self.assertTrue(json.loads(self.state_dir_state_json()))

    def state_dir_state_json(self) -> str:
        return (self.state_dir / "state.json").read_text(encoding="utf-8")

    def test_atomic_write_leaves_no_tmp_files_behind(self) -> None:
        self.plan()
        self.stage_both_producers()
        self.sign()
        self.verify()
        leftovers = [p for p in self.state_dir.rglob(".tmp-*")]
        self.assertEqual(leftovers, [], f"leaked temp files: {leftovers}")

    def test_path_traversal_asset_name_is_rejected(self) -> None:
        self.plan()
        manifest = self.build_manifest()
        manifest["assets"][0]["name"] = "../../etc/passwd"
        malicious = self.work / "malicious-manifest.json"
        write_json(malicious, manifest)

        result = self.stage(manifest=malicious)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsafe asset name", result.stderr)
        self.assertFalse((self.state_dir / "assets" / "passwd").exists())
        self.assertFalse((self.work / "etc").exists())

    def test_absolute_path_asset_name_is_rejected(self) -> None:
        self.plan()
        manifest = self.build_manifest()
        manifest["assets"][0]["name"] = "/etc/passwd"
        malicious = self.work / "malicious-manifest.json"
        write_json(malicious, manifest)

        result = self.stage(manifest=malicious)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsafe asset name", result.stderr)

    def test_symlinked_artifact_file_is_rejected(self) -> None:
        self.plan()
        name = "terraphim-server-1.2.3-linux-x86_64.tar.gz"
        real = self.artifact_dir_for("terraphim-ai") / name
        real.unlink()
        target = self.work / "outside-target.bin"
        target.write_bytes(self.asset_bytes[name])
        os.symlink(target, real)

        result = self.stage()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing regular file", result.stderr)


class DriftAndDuplicationContract(CoordinatorFixture):
    def test_declared_sha256_mismatch_is_rejected(self) -> None:
        self.plan()
        manifest = self.build_manifest()
        manifest["assets"][0]["sha256"] = "f" * 64
        tampered = self.work / "tampered-manifest.json"
        write_json(tampered, manifest)

        result = self.stage(manifest=tampered)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match actual", result.stderr)

    def test_staged_bytes_are_never_overwritten_on_drift(self) -> None:
        self.plan()
        self.assertEqual(self.stage().returncode, 0)

        name = "terraphim-server-1.2.3-linux-x86_64.tar.gz"
        staged_path = self.state_dir / "assets" / name
        original_bytes = staged_path.read_bytes()

        # A second stage call presents different bytes under the same name.
        (self.artifact_dir_for("terraphim-ai") / name).write_bytes(
            b"tampered-different-bytes"
        )
        manifest = self.build_manifest()
        manifest["assets"][0]["sha256"] = sha256_bytes(b"tampered-different-bytes")
        manifest["assets"][0]["size_bytes"] = len(b"tampered-different-bytes")
        drifted = self.work / "drifted-manifest.json"
        write_json(drifted, manifest)

        result = self.stage(manifest=drifted)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("never overwritten", result.stderr)
        self.assertEqual(staged_path.read_bytes(), original_bytes)

    def test_missing_staged_file_at_verify_is_rejected(self) -> None:
        self.plan()
        self.stage_both_producers()
        self.assertEqual(self.sign().returncode, 0)
        name = "terraphim-grep-1.2.3-linux-x86_64.tar.gz"
        (self.state_dir / "assets" / name).unlink()

        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing", result.stderr)

    def test_extra_staged_file_at_verify_is_rejected(self) -> None:
        self.plan()
        self.stage_both_producers()
        self.assertEqual(self.sign().returncode, 0)
        (self.state_dir / "assets" / "unexpected-extra-file.bin").write_bytes(b"nope")

        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("extra files", result.stderr)

    def test_source_run_id_drift_between_plan_and_manifest_is_rejected(self) -> None:
        self.plan()
        manifest = self.build_manifest()
        manifest["sources"]["terraphim-ai"]["workflow_run_id"] = 999999
        drifted = self.work / "drifted-sources-manifest.json"
        write_json(drifted, manifest)

        result = self.stage(manifest=drifted)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("do not match frozen plan sources", result.stderr)


class IdempotencyAndResumeContract(CoordinatorFixture):
    def test_repeated_plan_with_identical_inputs_is_a_noop(self) -> None:
        first = self.plan()
        self.assertEqual(first.returncode, 0)
        gen_after_first = parse_stdout(first)["generation"]

        second = self.plan()
        self.assertEqual(second.returncode, 0)
        self.assertEqual(parse_stdout(second)["generation"], gen_after_first)

    def test_plan_with_conflicting_sources_is_rejected(self) -> None:
        self.plan()
        conflicting_sources = copy.deepcopy(SOURCES)
        conflicting_sources["terraphim-ai"]["workflow_run_id"] = 1
        conflicting_file = self.work / "conflicting-sources.json"
        write_json(conflicting_file, conflicting_sources)

        result = run(
            [
                "plan",
                "--state-dir",
                str(self.state_dir),
                "--release-version",
                "1.2.3",
                "--release-tag",
                "v1.2.3",
                "--sources-file",
                str(conflicting_file),
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("plan drift", result.stderr)

    def test_repeated_stage_with_identical_inputs_is_a_noop(self) -> None:
        self.plan()
        first = self.stage()
        gen_after_first = parse_stdout(first)["generation"]
        second = self.stage()
        self.assertEqual(second.returncode, 0)
        self.assertEqual(parse_stdout(second)["generation"], gen_after_first)

    def test_partial_central_failure_resumes_without_rebuild(self) -> None:
        digest = self.plan_stage_verify()
        self.approve(digest)
        run(["promote-begin", "--state-dir", str(self.state_dir)])

        staged_assets_before = {
            p.name: p.read_bytes() for p in (self.state_dir / "assets").iterdir()
        }
        manifest_before = (self.state_dir / "manifest.json").read_bytes()

        failed = run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "r2_stable_manifest",
                "--outcome",
                "failure",
                "--detail",
                "transient network error",
            ]
        )
        self.assertEqual(failed.returncode, 0)
        self.assertEqual(self.inspect()["status"], "promoting")

        retried = run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "r2_stable_manifest",
                "--outcome",
                "success",
            ]
        )
        self.assertEqual(retried.returncode, 0)

        completed = run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "success",
            ]
        )
        self.assertEqual(parse_stdout(completed)["status"], "promoted")

        staged_assets_after = {
            p.name: p.read_bytes() for p in (self.state_dir / "assets").iterdir()
        }
        self.assertEqual(staged_assets_before, staged_assets_after)
        self.assertEqual(
            manifest_before, (self.state_dir / "manifest.json").read_bytes()
        )

    def test_approve_is_idempotent_after_promotion_begins_for_same_digest(self) -> None:
        digest = self.plan_stage_verify()
        self.assertEqual(self.approve(digest).returncode, 0)
        self.assertEqual(
            run(["promote-begin", "--state-dir", str(self.state_dir)]).returncode, 0
        )
        before = self.inspect()
        resumed = self.approve(digest, approved_by="resume-operator")
        self.assertEqual(resumed.returncode, 0, resumed.stderr)
        self.assertEqual(before["generation"], parse_stdout(resumed)["generation"])
        self.assertEqual(parse_stdout(resumed)["status"], "promoting")

    def test_resume_reports_next_actions_for_every_status(self) -> None:
        self.plan()
        result = run(["resume", "--state-dir", str(self.state_dir)])
        self.assertEqual(result.returncode, 0, result.stderr)
        payload = parse_stdout(result)
        self.assertEqual(payload["status"], "planned")
        self.assertIn("stage", payload["next_actions"])


class ApprovalBindingContract(CoordinatorFixture):
    def test_approval_must_match_verified_digest_exactly(self) -> None:
        digest = self.plan_stage_verify()
        wrong = "0" * 64
        result = self.approve(wrong)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not bound to current manifest digest", result.stderr)

        result = self.approve(digest)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_promote_begin_requires_bound_approval(self) -> None:
        self.plan_stage_verify()
        result = run(["promote-begin", "--state-dir", str(self.state_dir)])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("requires an approval", result.stderr)

    def test_zero_mutation_before_approval_rehearsal(self) -> None:
        """Plan -> stage -> verify (rehearsal) must not touch anything outside
        the state directory, and must leave promote/central/downstream state
        entirely untouched (issue #3382 pre-approval rehearsal contract)."""
        before = self.snapshot_tree()
        digest = self.plan_stage_verify()
        after = self.snapshot_tree()

        # Only files under state/ (plus the fixture's own manifest/sources
        # inputs, which already existed before rehearsal) should differ.
        new_paths = after - before
        self.assertTrue(new_paths, "rehearsal should have produced state artifacts")
        for path in new_paths:
            self.assertTrue(
                path.startswith("state/"), f"rehearsal wrote outside state dir: {path}"
            )

        state = self.inspect()
        self.assertEqual(state["status"], "verified")
        self.assertIsNone(state["approval"])
        self.assertEqual(state["central_channels"], {})
        self.assertIsNone(state["downstream_handoff"])
        self.assertEqual(digest, state["manifest_sha256"])

    def test_abort_after_rehearsal_leaves_zero_central_or_downstream_mutation(
        self,
    ) -> None:
        digest = self.plan_stage_verify()
        result = run(
            [
                "fail",
                "--state-dir",
                str(self.state_dir),
                "--reason",
                "operator aborted before approval",
            ]
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        state = self.inspect()
        self.assertEqual(state["status"], "failed")
        self.assertEqual(state["central_channels"], {})
        self.assertIsNone(state["downstream_handoff"])
        self.assertFalse((self.state_dir / "downstream-handoff.json").exists())
        self.assertEqual(state["manifest_sha256"], digest)


class PromotionOrderingContract(CoordinatorFixture):
    def begin_promotion(self) -> None:
        digest = self.plan_stage_verify()
        approved = self.approve(digest)
        self.assertEqual(approved.returncode, 0, approved.stderr)
        begun = run(["promote-begin", "--state-dir", str(self.state_dir)])
        self.assertEqual(begun.returncode, 0, begun.stderr)

    def test_terminal_verification_cannot_precede_publication(self) -> None:
        self.begin_promotion()
        result = run(
            [
                "record-verification",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "success",
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("before publication success", result.stderr)

    def test_release_reconciliation_cannot_precede_release_creation(self) -> None:
        self.begin_promotion()
        result = run(
            [
                "record-phase",
                "--state-dir",
                str(self.state_dir),
                "--phase",
                "release_reconciliation",
                "--outcome",
                "success",
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cannot complete before release creation", result.stderr)

    def test_successful_publication_record_cannot_be_downgraded(self) -> None:
        self.begin_promotion()
        succeeded = run(
            [
                "record-publication",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "success",
            ]
        )
        self.assertEqual(succeeded.returncode, 0, succeeded.stderr)
        downgraded = run(
            [
                "record-publication",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "failure",
            ]
        )
        self.assertNotEqual(downgraded.returncode, 0)
        self.assertIn("cannot be downgraded", downgraded.stderr)

    def test_successful_phase_record_cannot_be_downgraded(self) -> None:
        self.begin_promotion()
        succeeded = run(
            [
                "record-phase",
                "--state-dir",
                str(self.state_dir),
                "--phase",
                "release_creation",
                "--outcome",
                "success",
            ]
        )
        self.assertEqual(succeeded.returncode, 0, succeeded.stderr)
        downgraded = run(
            [
                "record-phase",
                "--state-dir",
                str(self.state_dir),
                "--phase",
                "release_creation",
                "--outcome",
                "failure",
            ]
        )
        self.assertNotEqual(downgraded.returncode, 0)
        self.assertIn("cannot be downgraded", downgraded.stderr)


class SplitProducerStagingContract(CoordinatorFixture):
    """Reproduces and fixes the reviewer's REPRO A: the workflow downloads
    terraphim-ai and terraphim-clients artifacts into two separate
    directories and calls `stage` once per directory. `cmd_stage` must only
    require the subset of manifest assets owned by the --source-repo it was
    given, not the full manifest."""

    def test_staging_only_one_producer_leaves_the_other_producers_assets_missing(
        self,
    ) -> None:
        self.plan()
        ai_only = self.stage(source_repo="terraphim-ai")
        self.assertEqual(ai_only.returncode, 0, ai_only.stderr)

        staged_names = {p.name for p in (self.state_dir / "assets").iterdir()}
        self.assertEqual(staged_names, {"terraphim-server-1.2.3-linux-x86_64.tar.gz"})
        self.assertNotIn("terraphim-agent-1.2.3-linux-x86_64.tar.gz", staged_names)
        self.assertNotIn("terraphim-grep-1.2.3-linux-x86_64.tar.gz", staged_names)

        # verify (even before signing) must fail closed: the manifest
        # requires all three assets and only one is staged.
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)

    def test_staging_each_producer_directory_separately_then_verify_succeeds(
        self,
    ) -> None:
        """The workflow's exact literal sequence: stage(terraphim-ai dir),
        then stage(terraphim-clients dir), each call only ever seeing its
        own producer's files on disk -- never the other producer's assets,
        and never a merged directory."""
        self.assertEqual(self.plan().returncode, 0)

        self.assertEqual(
            {p.name for p in self.artifact_dir_ai.iterdir()},
            {"terraphim-server-1.2.3-linux-x86_64.tar.gz"},
        )
        self.assertEqual(
            {p.name for p in self.artifact_dir_clients.iterdir()},
            {
                "terraphim-agent-1.2.3-linux-x86_64.tar.gz",
                "terraphim-grep-1.2.3-linux-x86_64.tar.gz",
            },
        )

        stage_ai = self.stage(source_repo="terraphim-ai")
        self.assertEqual(stage_ai.returncode, 0, stage_ai.stderr)
        stage_clients = self.stage(source_repo="terraphim-clients")
        self.assertEqual(stage_clients.returncode, 0, stage_clients.stderr)

        signed = self.sign()
        self.assertEqual(signed.returncode, 0, signed.stderr)
        verified = self.verify()
        self.assertEqual(verified.returncode, 0, verified.stderr)

        staged_names = {p.name for p in (self.state_dir / "assets").iterdir()}
        self.assertEqual(
            staged_names,
            {
                "terraphim-server-1.2.3-linux-x86_64.tar.gz",
                "terraphim-agent-1.2.3-linux-x86_64.tar.gz",
                "terraphim-grep-1.2.3-linux-x86_64.tar.gz",
            },
        )

    def test_stage_rejects_source_repo_with_no_matching_assets(self) -> None:
        self.plan()
        manifest = self.build_manifest()
        manifest["assets"] = [
            a for a in manifest["assets"] if a["source_repo"] != "terraphim-clients"
        ]
        only_ai = self.work / "only-ai-manifest.json"
        write_json(only_ai, manifest)

        result = self.stage(manifest=only_ai, source_repo="terraphim-clients")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no assets with source_repo", result.stderr)

    def test_stage_requires_a_valid_source_repo_choice(self) -> None:
        self.plan()
        result = run(
            [
                "stage",
                "--state-dir",
                str(self.state_dir),
                "--manifest",
                str(self.manifest_file),
                "--artifact-dir",
                str(self.artifact_dir_ai),
                "--source-repo",
                "not-a-real-repo",
            ]
        )
        self.assertNotEqual(result.returncode, 0)


class SigningContract(CoordinatorFixture):
    """Real zipsign Ed25519 signing/verification -- not a placeholder. No
    cryptographic check is skipped or mocked: these tests shell out to the
    real `zipsign` binary against a real generated keypair."""

    def test_sign_assets_embeds_a_real_signature_that_verify_accepts(self) -> None:
        self.plan()
        self.stage_both_producers()

        name = "terraphim-server-1.2.3-linux-x86_64.tar.gz"
        staged_path = self.state_dir / "assets" / name
        unsigned_bytes = staged_path.read_bytes()

        signed = self.sign()
        self.assertEqual(signed.returncode, 0, signed.stderr)

        signed_bytes = staged_path.read_bytes()
        self.assertNotEqual(
            unsigned_bytes,
            signed_bytes,
            "signing must actually mutate the staged bytes",
        )

        manifest = json.loads((self.state_dir / "manifest.json").read_text())
        asset = next(a for a in manifest["assets"] if a["name"] == name)
        self.assertEqual(asset["sha256"], hashlib.sha256(signed_bytes).hexdigest())
        self.assertEqual(asset["signature"], "zipsign-embedded-ed25519")

        verified = self.verify()
        self.assertEqual(verified.returncode, 0, verified.stderr)

        signature = self.state_dir / "manifest.json.sig"
        self.assertTrue(signature.is_file())
        detached = subprocess.run(
            [
                ZIPSIGN_BIN,
                "verify",
                "separate",
                "--context",
                "terraphim-release-manifest-v1",
                "--quiet",
                str(self.state_dir / "manifest.json"),
                str(signature),
                str(self.public_key_file),
            ],
            check=False,
            text=True,
            capture_output=True,
            timeout=30,
        )
        self.assertEqual(detached.returncode, 0, detached.stderr)

    def test_verify_rejects_tampered_detached_manifest_signature(self) -> None:
        self.plan()
        self.stage_both_producers()
        signed = self.sign()
        self.assertEqual(signed.returncode, 0, signed.stderr)
        (self.state_dir / "manifest.json.sig").write_bytes(b"tampered-signature")
        verified = self.verify()
        self.assertNotEqual(verified.returncode, 0)
        self.assertIn("detached manifest signature", verified.stderr)

    def test_sign_assets_requires_immediate_post_sign_verification(self) -> None:
        self.plan()
        self.stage_both_producers()
        wrapper = self.work / "zipsign-no-verify"
        wrapper.write_text(
            "#!/usr/bin/env bash\n"
            'if [[ "$1" == verify ]]; then exit 1; fi\n'
            f'exec {shlex.quote(ZIPSIGN_BIN)} "$@"\n'
        )
        wrapper.chmod(0o755)
        result = run(
            [
                "sign-assets",
                "--state-dir",
                str(self.state_dir),
                "--private-key-file",
                str(self.private_key_file),
                "--zipsign-bin",
                str(wrapper),
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "signature did not verify immediately after signing", result.stderr
        )

    def test_verify_fails_closed_on_unsigned_asset(self) -> None:
        self.plan()
        self.stage_both_producers()
        # Deliberately skip sign() -- staged bytes are still the raw,
        # unsigned producer output.
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("signature", result.stderr)

    def test_verify_fails_closed_without_private_key_file_when_signature_required(
        self,
    ) -> None:
        self.plan()
        self.stage_both_producers()
        self.assertEqual(self.sign().returncode, 0)
        result = run(["verify", "--state-dir", str(self.state_dir)])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no --private-key-file", result.stderr)

    def test_verify_fails_closed_when_zipsign_binary_is_missing(self) -> None:
        self.plan()
        self.stage_both_producers()
        self.assertEqual(self.sign().returncode, 0)
        result = run(
            [
                "verify",
                "--state-dir",
                str(self.state_dir),
                "--private-key-file",
                str(self.private_key_file),
                "--zipsign-bin",
                "/nonexistent/zipsign-binary-does-not-exist",
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("zipsign binary not found", result.stderr)

    def test_sign_assets_fails_closed_when_zipsign_binary_is_missing(self) -> None:
        self.plan()
        self.stage_both_producers()
        result = run(
            [
                "sign-assets",
                "--state-dir",
                str(self.state_dir),
                "--private-key-file",
                str(self.private_key_file),
                "--zipsign-bin",
                "/nonexistent/zipsign-binary-does-not-exist",
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("zipsign binary not found", result.stderr)

    def test_sign_assets_is_idempotent_on_repeat_invocation(self) -> None:
        self.plan()
        self.stage_both_producers()
        first = self.sign()
        self.assertEqual(first.returncode, 0, first.stderr)
        signed_bytes_after_first = {
            p.name: p.read_bytes() for p in (self.state_dir / "assets").iterdir()
        }

        second = self.sign()
        self.assertEqual(second.returncode, 0, second.stderr)
        signed_bytes_after_second = {
            p.name: p.read_bytes() for p in (self.state_dir / "assets").iterdir()
        }
        self.assertEqual(signed_bytes_after_first, signed_bytes_after_second)

    def test_verify_rejects_a_forged_signature_from_a_different_key(self) -> None:
        self.plan()
        self.stage_both_producers()
        self.assertEqual(self.sign().returncode, 0)

        other_priv = self.work / "other-private.key"
        other_pub = self.work / "other-public.key"
        gen = subprocess.run(
            [ZIPSIGN_BIN, "gen-key", str(other_priv), str(other_pub)],
            check=False,
            text=True,
            capture_output=True,
            timeout=30,
        )
        self.assertEqual(gen.returncode, 0, gen.stderr)

        result = run(
            [
                "verify",
                "--state-dir",
                str(self.state_dir),
                "--private-key-file",
                str(other_priv),
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("failed cryptographic verification", result.stderr)

    def test_sign_assets_fails_closed_for_a_zipsign_unsupported_format(self) -> None:
        """zipsign only understands .zip and gzipped .tar (confirmed via
        `zipsign --help`: "Sign and verify `.zip` and `.tar.gz` files").
        The manifest schema's format enum also allows deb/rpm/pkg.tar.zst/
        exe/dmg. Routing one of those through `zipsign sign tar` does not
        fail loudly -- it appends a signature to the byte stream
        unconditionally with no validation that the input is a real
        gzipped tar, so a real .deb (an `ar` archive) comes out the other
        side as a file `dpkg-deb` still opens but `ar t` reports as a
        malformed archive, while zipsign itself reports success. sign-assets
        must fail closed for any format it cannot correctly sign, rather
        than silently producing that corrupted, falsely-successful asset."""
        self.plan()
        manifest = self.build_manifest()
        manifest["assets"][0]["format"] = "deb"
        manifest["assets"][0]["signature"] = (
            "terraphim-server-1.2.3-linux-x86_64.deb.sig"
        )
        deb_manifest = self.work / "deb-format-manifest.json"
        write_json(deb_manifest, manifest)

        self.stage_both_producers(manifest=deb_manifest)
        staged_path = (
            self.state_dir / "assets" / "terraphim-server-1.2.3-linux-x86_64.tar.gz"
        )
        original_bytes = staged_path.read_bytes()

        result = self.sign()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no signing/verification support for format 'deb'", result.stderr)

        # Fail closed means untouched -- not silently corrupted like the
        # bug this test guards against.
        self.assertEqual(staged_path.read_bytes(), original_bytes)

    def test_verify_fails_closed_for_a_zipsign_unsupported_format(self) -> None:
        self.plan()
        manifest = self.build_manifest()
        manifest["assets"][0]["format"] = "rpm"
        manifest["assets"][0]["signature"] = (
            "terraphim-server-1.2.3-linux-x86_64.rpm.sig"
        )
        rpm_manifest = self.work / "rpm-format-manifest.json"
        write_json(rpm_manifest, manifest)

        self.stage_both_producers(manifest=rpm_manifest)

        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no signing/verification support for format 'rpm'", result.stderr)

    def test_sign_assets_still_works_for_the_zip_format_via_the_correct_zipsign_subcommand(
        self,
    ) -> None:
        """Guards the fix itself: zip-format assets must be routed through
        `zipsign sign zip`/`verify zip`, not the tar subcommand -- both are
        real zipsign container types, and using the wrong one for either
        fails outright (confirmed: `zipsign sign zip` on non-zip bytes
        errors "Could not find EOCD"; a real zip through `sign tar` is
        equally invalid). This must actually be a well-formed zip, not
        arbitrary bytes, because unlike zipsign's tar handling its zip
        handling does validate the container before signing."""
        self.plan()
        manifest = self.build_manifest()
        manifest["assets"][0]["format"] = "zip"
        manifest["assets"][0]["name"] = "terraphim-server-1.2.3-windows-x86_64.zip"

        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w") as zf:
            zf.writestr("terraphim-server.exe", b"fake-windows-binary-payload")
        zip_bytes = buf.getvalue()

        manifest["assets"][0]["sha256"] = sha256_bytes(zip_bytes)
        manifest["assets"][0]["size_bytes"] = len(zip_bytes)
        zip_manifest = self.work / "zip-format-manifest.json"
        write_json(zip_manifest, manifest)

        (self.artifact_dir_ai / "terraphim-server-1.2.3-linux-x86_64.tar.gz").unlink()
        (
            self.artifact_dir_ai / "terraphim-server-1.2.3-windows-x86_64.zip"
        ).write_bytes(zip_bytes)

        stage_ai = self.stage(manifest=zip_manifest, source_repo="terraphim-ai")
        self.assertEqual(stage_ai.returncode, 0, stage_ai.stderr)
        stage_clients = self.stage(
            manifest=zip_manifest, source_repo="terraphim-clients"
        )
        self.assertEqual(stage_clients.returncode, 0, stage_clients.stderr)

        signed = self.sign()
        self.assertEqual(signed.returncode, 0, signed.stderr)

        verified = self.verify()
        self.assertEqual(verified.returncode, 0, verified.stderr)


class CrashRecoveryContract(CoordinatorFixture):
    """P2: crash-leftover .tmp-*.part files must not permanently block verify."""

    def test_orphan_temp_file_in_assets_dir_is_cleaned_up_automatically(self) -> None:
        digest_deps = self.plan()
        self.assertEqual(digest_deps.returncode, 0)
        self.stage_both_producers()
        orphan = self.state_dir / "assets" / ".tmp-deadbeef.part"
        orphan.write_bytes(b"leftover from a killed process")
        self.assertTrue(orphan.exists())

        signed = self.sign()
        self.assertEqual(signed.returncode, 0, signed.stderr)
        self.assertFalse(
            orphan.exists(), "orphan temp file should be cleaned up during sign-assets"
        )

        verified = self.verify()
        self.assertEqual(verified.returncode, 0, verified.stderr)

    def test_orphan_temp_file_reintroduced_after_sign_is_cleaned_by_verify(
        self,
    ) -> None:
        self.plan()
        self.stage_both_producers()
        self.assertEqual(self.sign().returncode, 0)
        orphan = self.state_dir / "assets" / ".tmp-cafef00d.part"
        orphan.write_bytes(b"leftover from a killed process")

        verified = self.verify()
        self.assertEqual(verified.returncode, 0, verified.stderr)
        self.assertFalse(orphan.exists())


class DownstreamChannelFilteringContract(CoordinatorFixture):
    """P2: handoff must filter assets per channel instead of blasting the
    full asset list (including assets a channel cannot consume) at all
    three downstream channels."""

    def promote_to_handoff(self) -> dict[str, Any]:
        digest = self.plan_stage_verify()
        self.approve(digest)
        run(["promote-begin", "--state-dir", str(self.state_dir)])
        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "success",
            ]
        )
        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "r2_stable_manifest",
                "--outcome",
                "success",
            ]
        )
        handoff = run(["handoff-downstream", "--state-dir", str(self.state_dir)])
        self.assertEqual(handoff.returncode, 0, handoff.stderr)
        return json.loads((self.state_dir / "downstream-handoff.json").read_text())

    def test_homebrew_channel_receives_server_and_both_client_binaries(self) -> None:
        payload = self.promote_to_handoff()
        channel = next(
            c for c in payload["channels"] if c["channel"] == "homebrew_tap_pr"
        )
        names = {a["name"] for a in channel["assets"]}
        self.assertEqual(
            names,
            {
                "terraphim-server-1.2.3-linux-x86_64.tar.gz",
                "terraphim-agent-1.2.3-linux-x86_64.tar.gz",
                "terraphim-grep-1.2.3-linux-x86_64.tar.gz",
            },
        )

    def test_aur_and_omarchy_channels_receive_only_clients_binaries_not_the_server(
        self,
    ) -> None:
        payload = self.promote_to_handoff()
        for channel_name in (
            "aur_terraphim_clients_bin",
            "omarchy_terraphim_clients_bin",
        ):
            channel = next(
                c for c in payload["channels"] if c["channel"] == channel_name
            )
            names = {a["name"] for a in channel["assets"]}
            self.assertEqual(
                names,
                {
                    "terraphim-agent-1.2.3-linux-x86_64.tar.gz",
                    "terraphim-grep-1.2.3-linux-x86_64.tar.gz",
                },
            )
            self.assertNotIn("terraphim-server-1.2.3-linux-x86_64.tar.gz", names)


class DownstreamDispatchRecordingContract(CoordinatorFixture):
    """record-downstream-channel is a non-gating audit trail: it must never
    fabricate success, and must fail closed with an illegal transition
    before central promotion completes."""

    def promoted_handoff(self) -> tuple[str, dict[str, Any]]:
        digest = self.plan_stage_verify()
        self.assertEqual(self.approve(digest).returncode, 0)
        self.assertEqual(
            run(["promote-begin", "--state-dir", str(self.state_dir)]).returncode,
            0,
        )
        for channel in ("github_release", "r2_stable_manifest"):
            result = run(
                [
                    "record-channel",
                    "--state-dir",
                    str(self.state_dir),
                    "--channel",
                    channel,
                    "--outcome",
                    "success",
                ]
            )
            self.assertEqual(result.returncode, 0, result.stderr)
        emitted = run(["handoff-downstream", "--state-dir", str(self.state_dir)])
        self.assertEqual(emitted.returncode, 0, emitted.stderr)
        return digest, json.loads(
            (self.state_dir / "downstream-handoff.json").read_text()
        )

    def proof_for(
        self,
        digest: str,
        handoff: dict[str, Any],
        channel: str = "aur_terraphim_clients_bin",
    ) -> dict[str, Any]:
        entry = next(c for c in handoff["channels"] if c["channel"] == channel)
        return {
            "schema_version": "1.0.0",
            "channel": channel,
            "release_tag": "v1.2.3",
            "manifest_sha256": digest,
            "correlation_id": handoff["correlation_id"],
            "outcome": "success",
            "assets": [
                {"name": asset["name"], "sha256": asset["sha256"]}
                for asset in entry["assets"]
            ],
        }

    def test_record_downstream_channel_requires_promoted_status(self) -> None:
        self.plan()
        result = run(
            [
                "record-downstream-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "aur_terraphim_clients_bin",
                "--outcome",
                "accepted",
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("illegal transition", result.stderr)

    def test_record_downstream_channel_records_a_real_failure_honestly(self) -> None:
        digest = self.plan_stage_verify()
        self.approve(digest)
        run(["promote-begin", "--state-dir", str(self.state_dir)])
        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "success",
            ]
        )
        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "r2_stable_manifest",
                "--outcome",
                "success",
            ]
        )
        run(["handoff-downstream", "--state-dir", str(self.state_dir)])

        result = run(
            [
                "record-downstream-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "aur_terraphim_clients_bin",
                "--outcome",
                "failure",
                "--detail",
                "target repo/token not configured; fail-closed, no dispatch attempted",
            ]
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        state = self.inspect()
        self.assertEqual(
            state["downstream_channels"]["aur_terraphim_clients_bin"]["dispatch"][
                "outcome"
            ],
            "failure",
        )
        # Central promotion is unaffected by a downstream failure -- it was
        # already 'promoted' before any downstream dispatch was attempted.
        self.assertEqual(state["status"], "promoted")

    def test_terminal_proof_requires_an_accepted_dispatch(self) -> None:
        digest, handoff = self.promoted_handoff()
        proof_file = self.work / "proof-without-dispatch.json"
        write_json(proof_file, self.proof_for(digest, handoff))
        result = run(
            [
                "verify-downstream-proof",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "aur_terraphim_clients_bin",
                "--proof",
                str(proof_file),
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no accepted dispatch is recorded", result.stderr)

    def test_first_terminal_proof_is_rejected_when_correlation_is_unbound(
        self,
    ) -> None:
        digest, handoff = self.promoted_handoff()
        accepted = run(
            [
                "record-downstream-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "aur_terraphim_clients_bin",
                "--outcome",
                "accepted",
            ]
        )
        self.assertEqual(accepted.returncode, 0, accepted.stderr)
        proof = self.proof_for(digest, handoff)
        proof["correlation_id"] = "0" * 64
        proof_file = self.work / "first-unbound-proof.json"
        write_json(proof_file, proof)
        result = run(
            [
                "verify-downstream-proof",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "aur_terraphim_clients_bin",
                "--proof",
                str(proof_file),
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not bound to the frozen release", result.stderr)

    def test_terminal_proof_must_bind_channel_release_manifest_and_assets(self) -> None:
        digest = self.plan_stage_verify()
        self.approve(digest)
        run(["promote-begin", "--state-dir", str(self.state_dir)])
        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "github_release",
                "--outcome",
                "success",
            ]
        )
        run(
            [
                "record-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "r2_stable_manifest",
                "--outcome",
                "success",
            ]
        )
        run(["handoff-downstream", "--state-dir", str(self.state_dir)])
        accepted = run(
            [
                "record-downstream-channel",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "aur_terraphim_clients_bin",
                "--outcome",
                "accepted",
            ]
        )
        self.assertEqual(accepted.returncode, 0, accepted.stderr)
        handoff = json.loads((self.state_dir / "downstream-handoff.json").read_text())
        entry = next(
            c
            for c in handoff["channels"]
            if c["channel"] == "aur_terraphim_clients_bin"
        )
        proof = {
            "schema_version": "1.0.0",
            "channel": "aur_terraphim_clients_bin",
            "release_tag": "v1.2.3",
            "manifest_sha256": digest,
            "correlation_id": handoff["correlation_id"],
            "outcome": "success",
            "assets": [
                {"name": a["name"], "sha256": a["sha256"]} for a in entry["assets"]
            ],
        }
        proof_file = self.work / "proof.json"
        write_json(proof_file, proof)
        verified = run(
            [
                "verify-downstream-proof",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "aur_terraphim_clients_bin",
                "--proof",
                str(proof_file),
            ]
        )
        self.assertEqual(verified.returncode, 0, verified.stderr)
        state = self.inspect()
        self.assertEqual(
            state["downstream_channels"]["aur_terraphim_clients_bin"]["verification"][
                "outcome"
            ],
            "success",
        )

        proof["manifest_sha256"] = "0" * 64
        tampered = self.work / "tampered-proof.json"
        write_json(tampered, proof)
        rejected = run(
            [
                "verify-downstream-proof",
                "--state-dir",
                str(self.state_dir),
                "--channel",
                "aur_terraphim_clients_bin",
                "--proof",
                str(tampered),
            ]
        )
        self.assertNotEqual(rejected.returncode, 0)


class NoRebuildContract(unittest.TestCase):
    """The coordinator must never invoke a build tool; it only stages,
    validates, and copies already-produced, immutable producer artifacts."""

    FORBIDDEN_SUBSTRINGS = (
        "cargo build",
        "cargo install",
        "npm run build",
        "npm install",
        "yarn build",
        "docker build",
        "make ",
        "cc -o",
        "rustc ",
    )

    def test_coordinator_source_contains_no_build_invocations(self) -> None:
        text = COORDINATOR.read_text(encoding="utf-8")
        for forbidden in self.FORBIDDEN_SUBSTRINGS:
            self.assertNotIn(
                forbidden,
                text,
                f"coordinator must not invoke a build command: {forbidden!r}",
            )

    def test_coordinator_only_shells_out_to_the_schema_validator(self) -> None:
        text = COORDINATOR.read_text(encoding="utf-8")
        subprocess_calls = [
            line for line in text.splitlines() if "subprocess.run" in line
        ]
        self.assertTrue(subprocess_calls)
        # The only subprocess.run call site must target the validator script,
        # not an arbitrary shell/build command.
        body_start = text.index("def _run_validator")
        body_end = text.index("\n\n\n", body_start)
        validator_body = text[body_start:body_end]
        self.assertIn("validate-release-manifest", text)
        self.assertIn("subprocess.run", validator_body)


if __name__ == "__main__":
    unittest.main()
