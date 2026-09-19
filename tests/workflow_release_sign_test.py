"""Regression tests for the read-only release signature verifier."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/release-sign.yml"
COORDINATOR_WORKFLOW = ROOT / ".github/workflows/release-coordinator.yml"
ZIPSIGN_BIN = shutil.which("zipsign")

TAG_GRAMMAR = r"^([A-Za-z0-9_.-]+-)?v[0-9]+\.[0-9]+\.[0-9]+$"


def extract_step_run(step_name: str) -> str:
    text = WORKFLOW.read_text(encoding="utf-8")
    marker = re.compile(rf"^      - name: {re.escape(step_name)}\n", re.MULTILINE)
    match = marker.search(text)
    if not match:
        raise AssertionError(f"step not found: {step_name!r}")
    rest = text[match.end() :]
    run_marker = re.search(r"^        run: \|\n", rest, re.MULTILINE)
    assert run_marker
    body_start = run_marker.end()
    next_step = re.search(r"^      - name: ", rest[body_start:], re.MULTILINE)
    body = (
        rest[body_start : body_start + next_step.start()]
        if next_step
        else rest[body_start:]
    )
    return "\n".join(line.removeprefix("        ") for line in body.splitlines()) + "\n"


class ReleaseSignOwnershipContract(unittest.TestCase):
    def test_workflow_is_read_only_and_has_no_sign_or_release_upload(self) -> None:
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("types: [published]", text)
        self.assertRegex(text, r"(?m)^permissions:\n  contents: read$")
        self.assertNotIn("gh release upload", text)
        self.assertNotIn("zipsign sign", text)
        self.assertNotIn("contents: write", text)

    def test_release_tag_is_taken_from_event_via_env_not_rewritten(self) -> None:
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn(
            "RELEASE_TAG: ${{ inputs.release_tag || github.event.release.tag_name }}",
            text,
        )
        self.assertIn('gh release view "$RELEASE_TAG"', text)
        self.assertNotIn("GITHUB_REF#refs/tags/v", text)

    def test_verifier_is_reachable_for_coordinator_published_releases(self) -> None:
        """P2-2: `release: [published]` never fires for GITHUB_TOKEN-published
        (coordinator) releases. The verifier must also accept explicit
        workflow_dispatch/workflow_call triggers carrying the release tag."""
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("types: [published]", text)
        self.assertIn("workflow_dispatch:", text)
        self.assertIn("workflow_call:", text)
        for trigger in ("workflow_dispatch:", "workflow_call:"):
            trigger_block = text.split(trigger, 1)[1]
            self.assertIn("release_tag:", trigger_block, trigger)
            self.assertIn("required: true", trigger_block, trigger)
            self.assertIn("type: string", trigger_block, trigger)

    def test_release_tag_input_is_validated_before_use(self) -> None:
        """The dispatch/call tag must match the producer tag grammar before it
        reaches any shell or gh invocation (fail closed, no interpolation)."""
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn(TAG_GRAMMAR, text)
        # The tag flows via an env var, never inline ${{ }} in a run block.
        self.assertNotIn('gh release view "${{', text)
        self.assertNotIn('gh release download "${{', text)

    def test_coordinator_dispatches_independent_verification_after_publish(
        self,
    ) -> None:
        """P2-2: the coordinator must invoke the read-only verifier for every
        promoted release via a reusable-workflow call (no new token
        permissions: the callee stays contents: read)."""
        text = COORDINATOR_WORKFLOW.read_text(encoding="utf-8")
        caller = re.search(
            r"(?ms)^  verify-release-signatures:\n(?:    .*\n?)+", text
        )
        self.assertIsNotNone(
            caller, "release-coordinator.yml must call release-sign.yml"
        )
        block = caller.group(0)
        self.assertIn("uses: ./.github/workflows/release-sign.yml", block)
        self.assertIn("release_tag: ${{ inputs.release_tag }}", block)
        self.assertIn("contents: read", block)
        self.assertNotIn("contents: write", block)
        # Only after a real promotion, never for rehearsals.
        self.assertIn("central-promote", block)
        self.assertIn("inputs.promote == true", block)
        # The call happens only after GitHub publication verification
        # succeeded inside central-promote (documented ordering anchor).
        self.assertIn("secrets: inherit", block)

    def test_signature_report_uses_single_grouped_redirect(self) -> None:
        run = extract_step_run("Generate signature report")
        self.assertIn("} > signature-report.md", run)
        self.assertNotIn(">> signature-report.md", run)
        # The artifact listing belongs inside the fenced report block, not
        # only on the runner's stdout.
        group = run.split("{", 1)[1].split("} > signature-report.md", 1)[0]
        self.assertIn("ls -lh artifacts/", group)

    def test_job_summary_append_redirect_is_quoted(self) -> None:
        run = extract_step_run("Add job summary")
        self.assertIn('cat signature-report.md >> "$GITHUB_STEP_SUMMARY"', run)
        self.assertNotIn(">> $GITHUB_STEP_SUMMARY", run)


@unittest.skipUnless(ZIPSIGN_BIN, "zipsign binary not on PATH")
class ReleaseSignExecutionContract(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.work = Path(self.tmp.name)
        self.artifacts = self.work / "artifacts"
        self.artifacts.mkdir()
        self.private = self.work / "private.key"
        self.public = self.work / "public.key"
        result = subprocess.run(
            [ZIPSIGN_BIN, "gen-key", str(self.private), str(self.public)],
            check=False,
            text=True,
            capture_output=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def run_verify(self) -> subprocess.CompletedProcess[str]:
        env = dict(os.environ)
        env["VERIFYING_KEY_FILE"] = str(self.public)
        return subprocess.run(
            ["bash", "-c", extract_step_run("Verify signatures")],
            cwd=str(self.work),
            env=env,
            check=False,
            text=True,
            capture_output=True,
            timeout=60,
        )

    def test_signed_tar_is_verified_without_mutation(self) -> None:
        archive = self.artifacts / "terraphim-server-1.2.3.tar.gz"
        payload = self.work / "payload"
        payload.write_bytes(b"payload")
        subprocess.run(
            ["tar", "czf", str(archive), "-C", str(self.work), "payload"], check=True
        )
        subprocess.run(
            [ZIPSIGN_BIN, "sign", "tar", str(archive), str(self.private), "-f"],
            check=True,
        )
        before = archive.read_bytes()
        result = self.run_verify()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(before, archive.read_bytes())

    def test_unsigned_tar_fails_closed_without_mutation(self) -> None:
        archive = self.artifacts / "terraphim-agent-1.2.3.tar.gz"
        payload = self.work / "payload2"
        payload.write_bytes(b"unsigned")
        subprocess.run(
            ["tar", "czf", str(archive), "-C", str(self.work), "payload2"], check=True
        )
        before = archive.read_bytes()
        result = self.run_verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(before, archive.read_bytes())


if __name__ == "__main__":
    unittest.main()
