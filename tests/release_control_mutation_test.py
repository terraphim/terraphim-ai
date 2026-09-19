"""Exact mutation probes for release publication safety controls.

Each probe changes one control in an isolated temporary copy and proves the
targeted regression test turns red. The working tree is never modified.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_nth(text: str, old: str, new: str, occurrence: int = 1) -> str:
    starts = []
    offset = 0
    while True:
        index = text.find(old, offset)
        if index < 0:
            break
        starts.append(index)
        offset = index + len(old)
    if occurrence < 1 or occurrence > len(starts):
        raise AssertionError(
            f"mutation anchor occurrence {occurrence} unavailable; found {len(starts)}"
        )
    index = starts[occurrence - 1]
    return text[:index] + new + text[index + len(old) :]


class ReleaseControlMutationContract(unittest.TestCase):
    maxDiff = None

    def make_probe(self, paths: list[Path]) -> tuple[tempfile.TemporaryDirectory, Path]:
        temporary = tempfile.TemporaryDirectory(prefix="release-control-mutant-")
        probe = Path(temporary.name)
        for relative in paths:
            source = ROOT / relative
            destination = probe / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)
        return temporary, probe

    def mutate_and_run(
        self,
        *,
        name: str,
        paths: list[Path],
        target: Path,
        old: str,
        new: str,
        command: list[str],
        occurrence: int = 1,
        timeout: int = 120,
    ) -> None:
        temporary, probe = self.make_probe(paths)
        self.addCleanup(temporary.cleanup)
        mutation_target = probe / target
        mutation_target.write_text(
            replace_nth(
                mutation_target.read_text(encoding="utf-8"),
                old,
                new,
                occurrence,
            ),
            encoding="utf-8",
        )
        env = dict(os.environ)
        env["PYTHONDONTWRITEBYTECODE"] = "1"
        result = subprocess.run(
            command,
            cwd=probe,
            env=env,
            check=False,
            text=True,
            capture_output=True,
            timeout=timeout,
        )
        self.assertNotEqual(
            result.returncode,
            0,
            f"mutation survived: {name}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}",
        )

    def test_r2_and_github_shell_control_mutations_are_killed(self) -> None:
        r2_script = Path(".github/scripts/release/r2-publish-manifest.sh")
        r2_test = Path(".github/scripts/release/tests/test_r2_publish_manifest.sh")
        r2_mutations = [
            (
                "status authorization",
                '(state.get("status") in {"promoting", "promoted"},',
                "(True,",
            ),
            (
                "state digest authorization",
                '(state.get("manifest_sha256") == expected,',
                "(True,",
            ),
            (
                "approval digest authorization",
                '(approval.get("manifest_sha256") == expected,',
                "(True,",
            ),
            (
                "promotion authorization",
                '(promotion.get("status") in {"in_progress", "complete"},',
                "(True,",
            ),
            (
                "GitHub publication authorization",
                '(github.get("publication", {}).get("outcome") == "success",',
                "(True,",
            ),
            (
                "GitHub verification authorization",
                '(github.get("verification", {}).get("outcome") == "success",',
                "(True,",
            ),
            (
                "optimization-safe gate",
                "    if not allowed:\n        raise SystemExit(reason)",
                "    if False:\n        raise SystemExit(reason)",
            ),
            (
                "readback byte comparison",
                '  cmp -s -- "$expected_file" "$output" \\',
                "  true \\",
            ),
            (
                "AWS conditional-write capability preflight",
                "if ! LC_ALL=C \"$AWS_BIN\" s3api put-object help 2>&1 | grep -Fq -- '--if-none-match'; then",
                "if false; then",
            ),
        ]
        for name, old, new in r2_mutations:
            with self.subTest(mutation=name):
                self.mutate_and_run(
                    name=name,
                    paths=[r2_script, r2_test],
                    target=r2_script,
                    old=old,
                    new=new,
                    command=["bash", str(r2_test)],
                )

        github_script = Path(".github/scripts/release/github-publish-release.sh")
        github_test = Path(
            ".github/scripts/release/tests/test_github_publish_release.sh"
        )
        github_mutations = [
            (
                "CRLF ownership normalization",
                "  body=\"${body//$'\\r'/}\"\n",
                "",
                1,
            ),
            (
                "exact ownership marker",
                'grep -Fqx -- "$owner_marker"',
                'grep -Fq -- "$owner_marker"',
                1,
            ),
            (
                "published asset digest",
                '  [[ "$actual_sha" == "$expected_sha" ]] \\',
                "  true \\",
                1,
            ),
            (
                "post-reconcile exact inventory",
                '  cmp -s "$scratch_dir/expected-names" "$remote_names" \\',
                "  true \\",
                1,
            ),
            (
                "published-not-draft verification",
                '  [[ "$(jq -r \'.draft\' "$release_json")" == "false" ]] \\',
                "  true \\",
                2,
            ),
        ]
        for name, old, new, occurrence in github_mutations:
            with self.subTest(mutation=name):
                self.mutate_and_run(
                    name=name,
                    paths=[github_script, github_test],
                    target=github_script,
                    old=old,
                    new=new,
                    occurrence=occurrence,
                    command=["bash", str(github_test)],
                )

    def test_workflow_and_single_publisher_mutations_are_killed(self) -> None:
        coordinator_test = Path("tests/workflow_release_coordinator_contract_test.py")
        recovery_test = Path("tests/workflow_release_recovery_contract_test.py")
        coordinator_workflow = Path(".github/workflows/release-coordinator.yml")
        workflow_paths = [
            coordinator_test,
            recovery_test,
            coordinator_workflow,
            Path(".github/workflows/release-comprehensive.yml"),
            Path(".github/workflows/release-manifest-contract.yml"),
            Path(".github/workflows/release-sign.yml"),
            Path(".github/workflows/publish-npm.yml"),
            Path(".github/workflows/publish-pypi.yml"),
            Path(".github/workflows/publish-bun.yml"),
            Path(".github/workflows/publish-tauri.yml"),
            Path(".github/workflows/docker-multiarch.yml"),
            Path(".github/scripts/release/r2-publish-manifest.sh"),
        ]
        coordinator_mutations = [
            (
                "release-promotion environment",
                coordinator_workflow,
                "    environment: release-promotion\n",
                "",
                "tests.workflow_release_coordinator_contract_test.ReleaseCoordinatorWorkflowContract.test_central_promote_requires_release_promotion_environment",
            ),
            (
                "publish after reconciliation",
                coordinator_workflow,
                "        if: steps.package_reconcile.outcome == 'success'\n",
                "        if: always()\n",
                "tests.workflow_release_coordinator_contract_test.ReleaseCoordinatorWorkflowContract.test_github_publish_is_gated_on_successful_reconciliation",
            ),
            (
                "dispatch correlation binding",
                coordinator_workflow,
                "                      correlation_id: handoff.correlation_id,\n",
                "",
                "tests.workflow_release_coordinator_contract_test.ReleaseCoordinatorWorkflowContract.test_downstream_dispatch_acceptance_is_not_terminal_success",
            ),
            (
                "dispatch schema binding",
                coordinator_workflow,
                "                      schema_version: handoff.schema_version,\n",
                "",
                "tests.workflow_release_coordinator_contract_test.ReleaseCoordinatorWorkflowContract.test_downstream_dispatch_acceptance_is_not_terminal_success",
            ),
            (
                "proof artifact correlation naming",
                coordinator_workflow,
                "terraphim-release-proof-$correlation_id-$channel",
                "terraphim-release-proof-$channel",
                "tests.workflow_release_coordinator_contract_test.ReleaseCoordinatorWorkflowContract.test_downstream_dispatch_acceptance_is_not_terminal_success",
            ),
        ]
        for name, target, old, new, test_id in coordinator_mutations:
            with self.subTest(mutation=name):
                self.mutate_and_run(
                    name=name,
                    paths=workflow_paths,
                    target=target,
                    old=old,
                    new=new,
                    command=[sys.executable, "-m", "unittest", test_id],
                )

        recovery_mutations = [
            (
                "Bun bare-tag release guard widened",
                Path(".github/workflows/publish-bun.yml"),
                "if: startsWith(github.ref, 'refs/tags/bun-v') && (github.event_name != 'workflow_dispatch' || inputs.dry_run == false)",
                "if: startsWith(github.ref, 'refs/tags/') && (github.event_name != 'workflow_dispatch' || inputs.dry_run == false)",
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_bun_release_writer_allows_only_component_tag_and_non_dry_dispatch",
            ),
            (
                "Bun release guard removed",
                Path(".github/workflows/publish-bun.yml"),
                "        if: startsWith(github.ref, 'refs/tags/bun-v') && (github.event_name != 'workflow_dispatch' || inputs.dry_run == false)\n",
                "",
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_bun_release_writer_allows_only_component_tag_and_non_dry_dispatch",
            ),
            (
                "unquoted create-release writer detection",
                recovery_test,
                'r"uses:\\s*actions/create-release\\b|"',
                'r"uses:\\s*actions/not-a-release\\b|"',
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_exhaustive_github_release_writer_inventory_is_single_publisher_safe",
            ),
            (
                "quoted create-release writer detection",
                recovery_test,
                'r"uses:\\s*[\\"\']actions/create-release\\b|"',
                'r"uses:\\s*[\\"\']actions/not-a-release\\b|"',
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_release_writer_detector_accepts_quoted_and_unquoted_action_syntax",
            ),
            (
                "Bun writer expected classification",
                recovery_test,
                '                "publish-bun.yml",\n',
                "",
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_exhaustive_github_release_writer_inventory_is_single_publisher_safe",
            ),
            (
                "Bun pull-request path coverage",
                Path(".github/workflows/release-manifest-contract.yml"),
                '      - ".github/workflows/publish-bun.yml"\n',
                "",
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_single_publisher_contract_is_wired_into_ci_paths_and_execution",
            ),
            (
                "Bun push path coverage",
                Path(".github/workflows/release-manifest-contract.yml"),
                '      - ".github/workflows/publish-bun.yml"\n',
                "",
                2,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_single_publisher_contract_is_wired_into_ci_paths_and_execution",
            ),
            (
                "Bun actionlint scope",
                Path(".github/workflows/release-manifest-contract.yml"),
                ".github/workflows/publish-pypi.yml .github/workflows/publish-bun.yml .github/workflows/publish-tauri.yml",
                ".github/workflows/publish-pypi.yml .github/workflows/publish-tauri.yml",
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_single_publisher_contract_is_wired_into_ci_paths_and_execution",
            ),
            (
                "Bun YAML parse scope",
                Path(".github/workflows/release-manifest-contract.yml"),
                "'.github/workflows/publish-pypi.yml', '.github/workflows/publish-bun.yml', '.github/workflows/publish-tauri.yml'",
                "'.github/workflows/publish-pypi.yml', '.github/workflows/publish-tauri.yml'",
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_single_publisher_contract_is_wired_into_ci_paths_and_execution",
            ),
            (
                "PyPI bare-tag release guard",
                Path(".github/workflows/publish-pypi.yml"),
                "if: (startsWith(github.ref, 'refs/tags/python-v') || startsWith(github.ref, 'refs/tags/pypi-v')) && inputs.dry_run == false",
                "if: startsWith(github.ref, 'refs/tags/') && inputs.dry_run == false",
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_exhaustive_github_release_writer_inventory_is_single_publisher_safe",
            ),
            (
                "comprehensive create-release standard guard",
                Path(".github/workflows/release-comprehensive.yml"),
                "needs.resolve-release-source.outputs.is_standard_release != 'true'",
                "true",
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_exhaustive_github_release_writer_inventory_is_single_publisher_safe",
            ),
            (
                "comprehensive recovery standard guard",
                Path(".github/workflows/release-comprehensive.yml"),
                "needs.resolve-release-source.outputs.is_standard_release != 'true'",
                "true",
                2,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_exhaustive_github_release_writer_inventory_is_single_publisher_safe",
            ),
            (
                "deprecated Tauri standard guard",
                Path(".github/workflows/publish-tauri.yml"),
                "if: startsWith(github.ref, 'refs/tags/app-v') || startsWith(github.ref, 'refs/tags/desktop-v')",
                "if: always()",
                1,
                "tests.workflow_release_recovery_contract_test.ReleaseRecoveryWorkflowContract.test_exhaustive_github_release_writer_inventory_is_single_publisher_safe",
            ),
        ]
        for name, target, old, new, occurrence, test_id in recovery_mutations:
            with self.subTest(mutation=name):
                self.mutate_and_run(
                    name=name,
                    paths=workflow_paths,
                    target=target,
                    old=old,
                    new=new,
                    occurrence=occurrence,
                    command=[sys.executable, "-m", "unittest", test_id],
                )

    def test_state_machine_control_mutations_are_killed(self) -> None:
        coordinator = Path(".github/scripts/release/release_coordinator.py")
        coordinator_test = Path("tests/release_coordinator_test.py")
        paths = [
            coordinator,
            coordinator_test,
            Path(".release/release-manifest.schema.json"),
            Path("scripts/validate-release-manifest.py"),
        ]
        mutations = [
            (
                "detached signature verification",
                "            _verify_manifest_detached(\n                zipsign_bin,\n                manifest_file,\n                manifest_signature_path(state_dir),\n                verifying_key,\n            )",
                "            pass",
                1,
                "tests.release_coordinator_test.SigningContract.test_verify_rejects_tampered_detached_manifest_signature",
            ),
            (
                "verification after publication",
                '                != "success"\n            )\n        ):\n            raise CoordinatorError(\n                f"cannot record successful verification for {args.channel!r} before publication success"\n            )',
                '                != "success"\n            )\n            and False\n        ):\n            raise CoordinatorError(\n                f"cannot record successful verification for {args.channel!r} before publication success"\n            )',
                1,
                "tests.release_coordinator_test.PromotionOrderingContract.test_terminal_verification_cannot_precede_publication",
            ),
            (
                "reconciliation after creation",
                '        if args.phase == "release_reconciliation" and (',
                '        if False and args.phase == "release_reconciliation" and (',
                1,
                "tests.release_coordinator_test.PromotionOrderingContract.test_release_reconciliation_cannot_precede_release_creation",
            ),
            (
                "central success non-downgrade",
                '        if existing and existing.get("outcome") == "success":',
                "        if False:",
                1,
                "tests.release_coordinator_test.PromotionOrderingContract.test_successful_publication_record_cannot_be_downgraded",
            ),
            (
                "phase success non-downgrade",
                '        if record.get("outcome") == "success":',
                "        if False:",
                1,
                "tests.release_coordinator_test.PromotionOrderingContract.test_successful_phase_record_cannot_be_downgraded",
            ),
            (
                "accepted dispatch prerequisite",
                '        if dispatch.get("outcome") != "accepted":',
                "        if False:",
                1,
                "tests.release_coordinator_test.DownstreamDispatchRecordingContract.test_terminal_proof_requires_an_accepted_dispatch",
            ),
            (
                "proof correlation binding",
                "        if mismatches or actual_assets != expected_assets:",
                "        if False:",
                1,
                "tests.release_coordinator_test.DownstreamDispatchRecordingContract.test_first_terminal_proof_is_rejected_when_correlation_is_unbound",
            ),
            (
                "post-sign self-verification",
                '                    if not _zipsign_is_signed(\n                        zipsign_bin, subcommand, signed_candidate, verifying_key\n                    ):\n                        raise CoordinatorError(\n                            f"assets[{index}] ({name}): signature did not verify immediately after signing"\n                        )',
                "                    pass",
                1,
                "tests.release_coordinator_test.SigningContract.test_sign_assets_requires_immediate_post_sign_verification",
            ),
        ]
        for name, old, new, occurrence, test_id in mutations:
            with self.subTest(mutation=name):
                self.mutate_and_run(
                    name=name,
                    paths=paths,
                    target=coordinator,
                    old=old,
                    new=new,
                    occurrence=occurrence,
                    command=[sys.executable, "-m", "unittest", test_id],
                    timeout=180,
                )


if __name__ == "__main__":
    unittest.main()
