"""Contract tests for the managed-release rehearsal aggregator (Gitea #3382).

The aggregator references channel-owned evidence and never duplicates it; a
complete candidate yields the approval digest, while missing, tampered or
unbound evidence fails closed and a deferred channel withholds approval.
"""

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "rehearse-managed-release.py"

CHANNELS = ["homebrew_tap_pr", "aur_terraphim_clients_bin", "omarchy_terraphim_clients_bin"]
VERIFICATIONS = ["deb_rpm_native", "updater_zero_network_write"]


class RehearsalFixture(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        root = Path(self._tmp.name)
        self.state_dir = root / "state"
        self.state_dir.mkdir()
        self.evidence_root = root / "evidence"
        self.evidence_root.mkdir()

        self.manifest = {
            "schema_version": "1.0.0",
            "release_version": "1.21.16",
            "release_tag": "v1.21.16",
            "downstream_channels": CHANNELS,
            "required_channels": [
                "github_release",
                "r2_stable_manifest",
                "homebrew_tap_pr",
                "aur_terraphim_clients_bin",
                "omarchy_terraphim_clients_bin",
            ],
            "central_channels": ["github_release", "r2_stable_manifest"],
        }
        manifest_path = self.state_dir / "manifest.json"
        manifest_path.write_text(json.dumps(self.manifest, sort_keys=True))
        self.digest = hashlib.sha256(manifest_path.read_bytes()).hexdigest()

        self.state = {
            "status": "verified",
            "release_version": "1.21.16",
            "release_tag": "v1.21.16",
            "manifest_sha256": self.digest,
        }
        (self.state_dir / "state.json").write_text(json.dumps(self.state, sort_keys=True))

        self.evidence = {
            "schema_version": "1.0.0",
            "release_tag": "v1.21.16",
            "manifest_sha256": self.digest,
            "channels": {},
            "verifications": {},
        }
        self._populate()

    def tearDown(self):
        self._tmp.cleanup()

    def _write(self, rel: str, content: bytes) -> str:
        path = self.evidence_root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return hashlib.sha256(content).hexdigest()

    def _populate(self):
        for index, channel in enumerate(CHANNELS):
            rel = f"{channel}/evidence-{index}.bin"
            digest = self._write(rel, f"DO-NOT-EMBED-{index}".encode())
            self.evidence["channels"][channel] = [
                {"name": f"{channel}.bin", "path": rel, "sha256": digest}
            ]
        for lane in VERIFICATIONS:
            rel = f"{lane}/evidence.bin"
            digest = self._write(rel, f"DO-NOT-EMBED-{lane}".encode())
            self.evidence["verifications"][lane] = [
                {"name": f"{lane}.bin", "path": rel, "sha256": digest}
            ]

    def bundle(self, mutate=None) -> Path:
        data = json.loads(json.dumps(self.evidence))
        if mutate:
            mutate(data)
        path = Path(self._tmp.name) / "evidence.json"
        path.write_text(json.dumps(data, sort_keys=True))
        return path

    def run_agg(self, *extra, mutate=None):
        output = self.state_dir / "rehearsal-report.json"
        proc = subprocess.run(
            [
                sys.executable, str(SCRIPT),
                "--state-dir", str(self.state_dir),
                "--evidence", str(self.bundle(mutate)),
                "--evidence-root", str(self.evidence_root),
                "--output", str(output),
                *extra,
            ],
            capture_output=True, text=True, check=False,
        )
        return proc, output


class CompleteRehearsalTests(RehearsalFixture):
    def test_complete_rehearsal_passes_and_yields_approval(self):
        proc, output = self.run_agg()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        report = json.loads(output.read_text())
        self.assertEqual(report["status"], "pass")
        self.assertEqual(report["approval"], self.digest)
        self.assertEqual(report["state_status"], "verified")
        self.assertEqual(
            report["correlation_id"],
            hashlib.sha256(f"v1.21.16:{self.digest}".encode()).hexdigest(),
        )
        self.assertEqual(sorted(report["channels"]), sorted(CHANNELS))
        for channel in CHANNELS:
            self.assertEqual(report["channels"][channel]["status"], "verified")
        for entry in report["channels"]["homebrew_tap_pr"]["evidence"]:
            self.assertGreater(entry["size_bytes"], 0)
            self.assertEqual(len(entry["sha256"]), 64)

    def test_report_references_evidence_and_does_not_embed_bytes(self):
        _, output = self.run_agg()
        self.assertNotIn("DO-NOT-EMBED", output.read_text())

    def test_output_is_byte_idempotent(self):
        _, first = self.run_agg()
        rendered = first.read_text()
        second_proc, second = self.run_agg()
        self.assertEqual(second_proc.returncode, 0, second_proc.stderr)
        self.assertEqual(second.read_text(), rendered)

    def test_check_passes_when_current_and_fails_on_drift(self):
        _, output = self.run_agg()
        proc, _ = self.run_agg("--check")
        self.assertEqual(proc.returncode, 0, proc.stderr)
        output.write_text(output.read_text() + "\n")
        proc, _ = self.run_agg("--check")
        self.assertNotEqual(proc.returncode, 0)


class FailClosedTests(RehearsalFixture):
    def test_missing_channel_evidence_fails(self):
        def drop(data):
            del data["channels"]["omarchy_terraphim_clients_bin"]
        proc, _ = self.run_agg(mutate=drop)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("omarchy_terraphim_clients_bin", proc.stderr)

    def test_missing_verification_evidence_fails(self):
        def drop(data):
            del data["verifications"]["deb_rpm_native"]
        proc, _ = self.run_agg(mutate=drop)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("deb_rpm_native", proc.stderr)

    def test_tampered_evidence_fails(self):
        (self.evidence_root / "homebrew_tap_pr" / "evidence-0.bin").write_bytes(b"tampered")
        proc, _ = self.run_agg()
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("digest mismatch", proc.stderr)

    def test_unbound_manifest_digest_fails(self):
        def mutate(data):
            data["manifest_sha256"] = "0" * 64
        proc, _ = self.run_agg(mutate=mutate)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("manifest_sha256 does not match", proc.stderr)

    def test_unbound_release_tag_fails(self):
        def mutate(data):
            data["release_tag"] = "v9.9.9"
        proc, _ = self.run_agg(mutate=mutate)
        self.assertNotEqual(proc.returncode, 0)

    def test_state_must_be_verified(self):
        self.state["status"] = "staged"
        (self.state_dir / "state.json").write_text(json.dumps(self.state, sort_keys=True))
        proc, _ = self.run_agg()
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("must be 'verified'", proc.stderr)

    def test_duplicate_evidence_name_fails(self):
        def mutate(data):
            data["channels"]["homebrew_tap_pr"].append(
                dict(data["channels"]["homebrew_tap_pr"][0])
            )
        proc, _ = self.run_agg(mutate=mutate)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("duplicate evidence name", proc.stderr)

    def test_unsafe_relative_path_fails(self):
        def mutate(data):
            data["channels"]["homebrew_tap_pr"][0]["path"] = "../escape.bin"
        proc, _ = self.run_agg(mutate=mutate)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("safe relative path", proc.stderr)

    def test_unknown_channel_fails(self):
        def mutate(data):
            data["channels"]["not_a_channel"] = [{"name": "x", "path": "x", "sha256": "0" * 64}]
        proc, _ = self.run_agg(mutate=mutate)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("unknown channels", proc.stderr)


class DeferredChannelTests(RehearsalFixture):
    def test_deferred_channel_withholds_approval(self):
        proc, output = self.run_agg(
            "--deferred-channel", "aur_terraphim_clients_bin", "AUR registration paused"
        )
        self.assertEqual(proc.returncode, 0, proc.stderr)
        report = json.loads(output.read_text())
        self.assertEqual(report["status"], "deferred")
        self.assertIsNone(report["approval"])
        self.assertEqual(report["channels"]["aur_terraphim_clients_bin"]["status"], "deferred")
        self.assertIn("paused", report["channels"]["aur_terraphim_clients_bin"]["reason"])

    def test_require_complete_fails_on_deferred(self):
        proc, _ = self.run_agg(
            "--require-complete",
            "--deferred-channel", "aur_terraphim_clients_bin", "AUR registration paused",
        )
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("incomplete", proc.stderr)

    def test_cannot_defer_unknown_channel(self):
        proc, _ = self.run_agg("--deferred-channel", "not_a_channel", "nope")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("unknown channel", proc.stderr)


if __name__ == "__main__":
    unittest.main()
