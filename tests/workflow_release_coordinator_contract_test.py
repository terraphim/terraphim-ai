#!/usr/bin/env python3
"""Static contract tests for .github/workflows/release-coordinator.yml.

These are dependency-free structural checks (no YAML parser required, to
stay consistent with tests/workflow_release_recovery_contract_test.py's
approach of regex-based section extraction) proving:

  * the rehearsal path (plan/stage/verify) always runs and is read-only
    (contents/actions read only, no write permissions) -- issue #3382's
    zero-mutation-before-approval contract;
  * central promotion only runs when explicitly requested (promote input)
    and records an approval bound to the manifest digest before any
    mutation step, and begins promotion before publishing;
  * downstream handoff only runs after central promotion succeeds, and is
    a separate job (so it structurally cannot run before both central
    channels are terminally verified inside the central-promote job itself;
  * partial failures are recorded (`if: always()`) rather than aborting the
    job outright, supporting resume without rebuilding artifacts.
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/release-coordinator.yml"
COMPREHENSIVE_WORKFLOW = ROOT / ".github/workflows/release-comprehensive.yml"
CONTRACT_WORKFLOW = ROOT / ".github/workflows/release-manifest-contract.yml"
SIGN_WORKFLOW = ROOT / ".github/workflows/release-sign.yml"
R2_PUBLISH_SCRIPT = ROOT / ".github/scripts/release/r2-publish-manifest.sh"


def indent_of(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def section(text: str, header: str) -> str:
    pattern = re.compile(rf"^(?P<indent>\s*){re.escape(header)}:\n", re.MULTILINE)
    match = pattern.search(text)
    if not match:
        return ""
    start = match.end()
    base_indent = len(match.group("indent"))
    for line_match in re.finditer(r"^.*(?:\n|$)", text[start:], re.MULTILINE):
        line = line_match.group(0)
        if not line.strip():
            continue
        if indent_of(line) <= base_indent and not line.lstrip().startswith("#"):
            return text[start : start + line_match.start()]
    return text[start:]


def job_block(text: str, job_name: str) -> str:
    jobs = section(text, "jobs")
    pattern = re.compile(rf"^  {re.escape(job_name)}:\n", re.MULTILINE)
    match = pattern.search(jobs)
    if not match:
        return ""
    start = match.end()
    for line_match in re.finditer(r"^.*(?:\n|$)", jobs[start:], re.MULTILINE):
        line = line_match.group(0)
        if line.strip() and indent_of(line) <= 2:
            return jobs[start : start + line_match.start()]
    return jobs[start:]


def input_block(text: str, input_name: str) -> str:
    dispatch = section(text, "workflow_dispatch")
    inputs = section(dispatch, "inputs")
    pattern = re.compile(rf"^(?P<indent>\s*){re.escape(input_name)}:\n", re.MULTILINE)
    match = pattern.search(inputs)
    if not match:
        return ""
    start = match.end()
    base_indent = len(match.group("indent"))
    for line_match in re.finditer(r"^.*(?:\n|$)", inputs[start:], re.MULTILINE):
        line = line_match.group(0)
        if line.strip() and indent_of(line) <= base_indent:
            return inputs[start : start + line_match.start()]
    return inputs[start:]


def step_names(job: str) -> list[str]:
    return re.findall(r"^\s+- name: (.+)\n", job, re.MULTILINE)


def step_run_block(job: str, step_name: str) -> str:
    marker = re.compile(rf"^\s+- name: {re.escape(step_name)}\n", re.MULTILINE)
    match = marker.search(job)
    if not match:
        return ""
    start = match.start()
    for later in re.finditer(r"^\s+- name: .*\n", job[match.end() :], re.MULTILINE):
        return job[start : match.end() + later.start()]
    return job[start:]


def has_need(job: str, dependency: str) -> bool:
    return bool(
        re.search(rf"^\s+needs:\s*{re.escape(dependency)}\s*$", job, re.MULTILINE)
        or re.search(
            rf"^\s+needs:\s*\[[^\]]*\b{re.escape(dependency)}\b", job, re.MULTILINE
        )
    )


class ReleaseCoordinatorWorkflowContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text(encoding="utf-8")

    def test_workflow_is_manual_dispatch_only(self) -> None:
        on_block = section(self.text, "on")
        self.assertIn("workflow_dispatch", on_block)
        self.assertNotIn("push", on_block)
        self.assertNotIn("pull_request", on_block)

    def test_required_freeze_inputs_are_declared(self) -> None:
        for name in [
            "release_version",
            "release_tag",
            "terraphim_ai_run_id",
            "terraphim_ai_sha",
            "terraphim_ai_tree_sha",
            "terraphim_clients_run_id",
            "terraphim_clients_sha",
            "terraphim_clients_tree_sha",
            "manifest_artifact_name",
        ]:
            block = input_block(self.text, name)
            self.assertIn("required: true", block, f"input {name} must be required")

    def test_promote_defaults_to_rehearsal_only(self) -> None:
        block = input_block(self.text, "promote")
        self.assertIn("type: boolean", block)
        self.assertIn("default: false", block)

    def test_approval_input_has_no_default_bypass(self) -> None:
        block = input_block(self.text, "approval")
        self.assertIn("default: ''", block)

    def test_top_level_permissions_are_read_only(self) -> None:
        top_permissions = section(self.text, "permissions")
        self.assertIn("contents: read", top_permissions)
        self.assertNotIn("write", top_permissions)

    def test_plan_stage_verify_job_is_unconditional_and_read_only(self) -> None:
        job = job_block(self.text, "plan-stage-verify")
        self.assertTrue(job)
        # No job-level `if:` gate -- rehearsal always runs regardless of
        # the promote input.
        self.assertNotRegex(
            job,
            r"^    if:",
        )
        job_permissions = section(job, "permissions")
        self.assertIn("contents: read", job_permissions)
        self.assertIn("actions: read", job_permissions)
        self.assertNotIn("write", job_permissions)

    def test_every_api_mode_artifact_download_job_has_actions_read(self) -> None:
        mapped_steps: list[tuple[str, str]] = []
        jobs = section(self.text, "jobs")
        for match in re.finditer(r"^  ([A-Za-z0-9_-]+):\n", jobs, re.MULTILINE):
            job_name = match.group(1)
            job = job_block(self.text, job_name)
            permissions = section(job, "permissions")
            for name in step_names(job):
                step = step_run_block(job, name)
                if "actions/download-artifact@" in step and "github-token:" in step:
                    mapped_steps.append((job_name, name))
                    self.assertIn(
                        "actions: read",
                        permissions,
                        f"{job_name}/{name} uses the Actions API but lacks actions: read",
                    )
        self.assertEqual(
            len(mapped_steps),
            5,
            f"all API-mode artifact downloads must be mapped: {mapped_steps}",
        )

    def test_plan_stage_verify_never_publishes_or_dispatches(self) -> None:
        job = job_block(self.text, "plan-stage-verify")
        for forbidden in (
            "release create",
            "release upload",
            "createWorkflowDispatch",
            "record-channel",
            "handoff-downstream",
        ):
            self.assertNotIn(forbidden, job)

    def test_central_promote_requires_promote_input(self) -> None:
        job = job_block(self.text, "central-promote")
        self.assertTrue(has_need(job, "plan-stage-verify"))
        condition_match = re.search(r"^    if: (.+)\n", job, re.MULTILINE)
        self.assertIsNotNone(condition_match)
        self.assertIn("inputs.promote", condition_match.group(1))

    def test_central_promote_requires_release_promotion_environment(self) -> None:
        job = job_block(self.text, "central-promote")
        self.assertRegex(job, r"(?m)^    environment: release-promotion$")

    def test_central_promote_records_approval_before_promote_begin_before_publish(
        self,
    ) -> None:
        job = job_block(self.text, "central-promote")
        names = step_names(job)

        def index_containing(substring: str) -> int:
            for i, name in enumerate(names):
                if substring in name:
                    return i
            raise AssertionError(f"no step name contains {substring!r} in: {names}")

        approve_idx = index_containing("Record approval")
        begin_idx = index_containing("Begin central promotion")
        publish_idx = index_containing("Publish GitHub release")
        self.assertLess(
            approve_idx, begin_idx, "approval must be recorded before promote-begin"
        )
        self.assertLess(
            begin_idx, publish_idx, "promote-begin must happen before any publish step"
        )

        approve_step = step_run_block(
            job, "Record approval bound to verified manifest digest"
        )
        self.assertIn("approve", approve_step)
        self.assertIn("--manifest-sha256", approve_step)
        self.assertIn("inputs.approval", approve_step)

    def test_central_promote_records_publication_and_verification_separately(
        self,
    ) -> None:
        job = job_block(self.text, "central-promote")
        names = step_names(job)
        expected = [
            "Publish GitHub release atomically from verified draft",
            "Record GitHub publication outcome",
            "Verify published GitHub release inventory and bytes",
            "Record GitHub terminal verification outcome",
            "Publish detached-signed R2 stable manifest",
            "Record R2 publication outcome",
            "Verify R2 readback and detached signature",
            "Record R2 terminal verification outcome",
        ]
        indexes = [names.index(name) for name in expected]
        self.assertEqual(indexes, sorted(indexes))
        self.assertIn("record-publication", step_run_block(job, expected[1]))
        self.assertIn("record-verification", step_run_block(job, expected[3]))
        self.assertIn("record-publication", step_run_block(job, expected[5]))
        self.assertIn("record-verification", step_run_block(job, expected[7]))

    def test_github_publish_is_gated_on_successful_reconciliation(self) -> None:
        job = job_block(self.text, "central-promote")
        publish = step_run_block(
            job, "Publish GitHub release atomically from verified draft"
        )
        self.assertIn("if: steps.package_reconcile.outcome == 'success'", publish)
        self.assertNotIn("if: always()", publish)

    def test_r2_publish_is_not_always_and_requires_github_terminal_verification(
        self,
    ) -> None:
        job = job_block(self.text, "central-promote")
        step = step_run_block(job, "Publish detached-signed R2 stable manifest")
        self.assertNotIn("if: always()", step)
        self.assertIn("if: steps.github_verify.outcome == 'success'", step)
        self.assertIn("STATE_FILE:", step)
        self.assertIn("MANIFEST_SIGNATURE_FILE:", step)

    def test_recorders_resolve_cas_from_durable_state_not_optional_step_outputs(
        self,
    ) -> None:
        job = job_block(self.text, "central-promote")
        for name in step_names(job):
            if name.startswith("Record ") and "approval" not in name:
                block = step_run_block(job, name)
                self.assertIn("['generation']", block, name)
                self.assertIn("--expected-generation", block, name)
                self.assertNotRegex(block, r"steps\.[^.]+\.outputs\.generation")

    def test_downstream_dispatch_acceptance_is_not_terminal_success(self) -> None:
        job = job_block(self.text, "downstream-handoff")
        dispatch = step_run_block(
            job,
            "Dispatch downstream channel workflows (real, authenticated, digest-bound; fail closed if target/token absent)",
        )
        self.assertIn("outcome = 'accepted'", dispatch)
        self.assertNotIn("outcome = 'success'", dispatch)
        self.assertIn("schema_version: handoff.schema_version", dispatch)
        self.assertIn("correlation_id: handoff.correlation_id", dispatch)
        self.assertIn("--expected-generation", dispatch)
        proof = step_run_block(job, "Verify terminal downstream publication proofs")
        self.assertIn("verify-downstream-proof", proof)
        self.assertIn("terraphim-release-proof-$correlation_id-$channel", proof)

    def test_central_promote_job_fails_unless_both_channels_promoted(self) -> None:
        job = job_block(self.text, "central-promote")
        gate = step_run_block(
            job, "Fail the job if either central channel did not succeed"
        )
        self.assertIn('state.get("status") != "promoted"', gate)
        self.assertIn("raise SystemExit", gate)
        self.assertNotRegex(gate, r"\bassert\b")

    def test_all_zipsign_install_sites_use_the_production_pin(self) -> None:
        install_lines: list[str] = []
        for workflow in (WORKFLOW, CONTRACT_WORKFLOW, SIGN_WORKFLOW):
            install_lines.extend(
                line.strip()
                for line in workflow.read_text(encoding="utf-8").splitlines()
                if "cargo install zipsign" in line
            )
        self.assertEqual(len(install_lines), 4, install_lines)
        self.assertEqual(set(install_lines), {"cargo install zipsign@0.2.1 --locked"})

    def test_runtime_python_security_gates_do_not_use_bare_assert(self) -> None:
        r2_script = R2_PUBLISH_SCRIPT.read_text(encoding="utf-8")
        self.assertNotRegex(r2_script, r"(?m)^assert\s")
        central = job_block(self.text, "central-promote")
        final_gate = step_run_block(
            central, "Fail the job if either central channel did not succeed"
        )
        self.assertNotRegex(final_gate, r"\bassert\b")
        self.assertIn("raise SystemExit", final_gate)

    def test_downstream_handoff_requires_central_promote_success(self) -> None:
        job = job_block(self.text, "downstream-handoff")
        self.assertTrue(has_need(job, "central-promote"))
        condition_match = re.search(r"^    if: (.+)\n", job, re.MULTILINE)
        self.assertIsNotNone(condition_match)
        self.assertIn(
            "needs.central-promote.result == 'success'", condition_match.group(1)
        )

    def test_downstream_handoff_calls_coordinator_before_dispatching(self) -> None:
        job = job_block(self.text, "downstream-handoff")
        names = step_names(job)
        handoff_idx = next(
            i for i, n in enumerate(names) if "Emit downstream handoff" in n
        )
        dispatch_idx = next(
            i for i, n in enumerate(names) if "Dispatch downstream" in n
        )
        self.assertLess(handoff_idx, dispatch_idx)
        emit_step = step_run_block(job, names[handoff_idx])
        self.assertIn("handoff-downstream", emit_step)

    def test_no_job_before_central_promote_can_reach_downstream_channels(self) -> None:
        plan_job = job_block(self.text, "plan-stage-verify")
        self.assertNotIn("homebrew", plan_job.lower())
        self.assertNotIn("aur", plan_job.lower())
        self.assertNotIn("omarchy", plan_job.lower())

    def test_immutable_tag_peel_is_verified_for_both_repos_before_planning(
        self,
    ) -> None:
        job = job_block(self.text, "plan-stage-verify")
        names = step_names(job)
        ai_peel_idx = next(
            i for i, n in enumerate(names) if "terraphim-ai" in n and "peel" in n
        )
        clients_peel_idx = next(
            i for i, n in enumerate(names) if "terraphim-clients" in n and "peel" in n
        )
        plan_idx = next(i for i, n in enumerate(names) if "Coordinator plan" in n)
        self.assertLess(ai_peel_idx, plan_idx)
        self.assertLess(clients_peel_idx, plan_idx)

    def test_stage_happens_before_verify_before_upload(self) -> None:
        job = job_block(self.text, "plan-stage-verify")
        names = step_names(job)

        def index_containing(substring: str) -> int:
            for i, name in enumerate(names):
                if substring in name:
                    return i
            raise AssertionError(f"no step name contains {substring!r} in: {names}")

        stage_ai_idx = index_containing("Coordinator stage (terraphim-ai")
        stage_clients_idx = index_containing("Coordinator stage (terraphim-clients")
        verify_idx = index_containing("Coordinator verify")
        upload_idx = index_containing("Upload frozen coordinator state")
        self.assertLess(stage_ai_idx, verify_idx)
        self.assertLess(stage_clients_idx, verify_idx)
        self.assertLess(verify_idx, upload_idx)

    def test_concurrency_is_scoped_per_release_tag(self) -> None:
        concurrency = section(self.text, "concurrency")
        self.assertIn("release-coordinator-", concurrency)
        self.assertIn("inputs.release_tag", concurrency)

    def test_legacy_release_pipeline_cannot_publish_standard_releases(self) -> None:
        text = COMPREHENSIVE_WORKFLOW.read_text(encoding="utf-8")
        for job_name in ("create-release", "upload-recovered-release-assets"):
            job = job_block(text, job_name)
            self.assertIn(
                "needs.resolve-release-source.outputs.is_standard_release != 'true'",
                job,
                f"{job_name} must not compete with the standard-release coordinator",
            )
        homebrew = job_block(text, "update-homebrew")
        self.assertTrue(has_need(homebrew, "wait-for-client-binaries"))
        wait = job_block(text, "wait-for-client-binaries")
        self.assertTrue(has_need(wait, "create-release"))


if __name__ == "__main__":
    unittest.main()
