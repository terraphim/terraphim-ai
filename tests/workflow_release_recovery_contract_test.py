"""Dependency-free static contracts for the v1.21.3 release recovery workflows."""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RELEASE_WORKFLOW = ROOT / ".github/workflows/release-comprehensive.yml"
DOCKER_WORKFLOW = ROOT / ".github/workflows/docker-multiarch.yml"
PYPI_WORKFLOW = ROOT / ".github/workflows/publish-pypi.yml"
NPM_WORKFLOW = ROOT / ".github/workflows/publish-npm.yml"
BUN_WORKFLOW = ROOT / ".github/workflows/publish-bun.yml"
TAURI_WORKFLOW = ROOT / ".github/workflows/publish-tauri.yml"
CONTRACT_WORKFLOW = ROOT / ".github/workflows/release-manifest-contract.yml"
AGENT_EVAL_WORKFLOW = ROOT / ".github/workflows/agent-eval.yml"
WORKFLOW_DIR = ROOT / ".github/workflows"

GITHUB_RELEASE_WRITER_PATTERN = re.compile(
    r"uses:\s*actions/create-release\b|"
    r"uses:\s*[\"']actions/create-release\b|"
    r"softprops/action-gh-release|gh release (?:create|edit|upload)|"
    r"repos\.(?:create|update)Release|createRelease|updateRelease|"
    r"github-publish-release\.sh",
    re.IGNORECASE,
)


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


def top_level_mapping_keys(text: str, parent: str) -> list[str]:
    body = section(text, parent)
    return re.findall(r"^  ([A-Za-z0-9_-]+):\n", body, re.MULTILINE)


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
    dispatch = section(text, "workflow_dispatch") or section(text, "workflow_call")
    pattern = re.compile(rf"^(?P<indent>\s*){re.escape(input_name)}:\n", re.MULTILINE)
    match = pattern.search(dispatch)
    if not match:
        return ""
    start = match.end()
    base_indent = len(match.group("indent"))
    for line_match in re.finditer(r"^.*(?:\n|$)", dispatch[start:], re.MULTILINE):
        line = line_match.group(0)
        if line.strip() and indent_of(line) <= base_indent:
            return dispatch[start : start + line_match.start()]
    return dispatch[start:]


def checkout_blocks(job: str) -> list[str]:
    blocks: list[str] = []
    lines = job.splitlines()
    for index, line in enumerate(lines):
        if "uses: actions/checkout@" not in line:
            continue
        block_lines = [line]
        base = (
            indent_of(lines[index - 1])
            if index > 0 and "- name:" in lines[index - 1]
            else indent_of(line)
        )
        for later in lines[index + 1 :]:
            if later.strip() and indent_of(later) <= base:
                break
            block_lines.append(later)
        blocks.append("\n".join(block_lines))
    return blocks


def step_run_block(job: str, step_name: str) -> str:
    marker = re.compile(rf"^\s+- name: {re.escape(step_name)}\n", re.MULTILINE)
    match = marker.search(job)
    if not match:
        return ""
    start = match.start()
    for later in re.finditer(r"^\s+- name: .*\n", job[match.end() :], re.MULTILINE):
        return job[start : match.end() + later.start()]
    return job[start:]


def step_if_condition(job: str, step_name: str) -> str:
    """Return a named step's single-line GitHub Actions condition."""
    step = step_run_block(job, step_name)
    match = re.search(r"^\s+if:\s*(.+)$", step, re.MULTILINE)
    return match.group(1).strip() if match else ""


def has_need(job: str, dependency: str) -> bool:
    return bool(
        re.search(rf"^\s+needs:\s*{re.escape(dependency)}\s*$", job, re.MULTILINE)
        or re.search(
            rf"^\s+needs:\s*\[[^\]]*\b{re.escape(dependency)}\b", job, re.MULTILINE
        )
        or re.search(rf"^\s+-\s*{re.escape(dependency)}\s*$", job, re.MULTILINE)
    )


def job_if_condition(text: str, job_name: str) -> str:
    """Return the folded scalar expression of a job's `if: >-` block."""
    job = job_block(text, job_name)
    match = re.search(r"^    if: >-\n((?:      .*\n?)*)", job, re.MULTILINE)
    if not match:
        return ""
    return " ".join(match.group(1).split())


def evaluate_condition(
    condition: str,
    *,
    event_name: str = "push",
    ref: str = "",
    results: dict[str, str] | None = None,
    inputs: dict[str, str | bool] | None = None,
    outputs: dict[tuple[str, str], str] | None = None,
) -> bool:
    """Evaluate a GitHub Actions `if:` expression under a simulated state.

    Supports the constructs used by the release workflows: always(),
    !cancelled(), startsWith(github.ref), github.event_name comparisons,
    typed boolean and bare negated inputs, needs.<job>.result comparisons,
    && and ||.
    """
    results = results or {}
    inputs = inputs or {}
    outputs = outputs or {}
    expr = " ".join(condition.split())
    expr = expr.replace("always()", "True").replace("!cancelled()", "True")

    def repl_starts_with_ref(match: re.Match[str]) -> str:
        return str(ref.startswith(match.group(1)))

    expr = re.sub(r"startsWith\(github\.ref, '([^']+)'\)", repl_starts_with_ref, expr)

    def repl_event(match: re.Match[str]) -> str:
        operator, expected = match.groups()
        matches = event_name == expected
        return str(matches if operator == "==" else not matches)

    expr = re.sub(r"github\.event_name (==|!=) '([a-z_]+)'", repl_event, expr)

    def repl_boolean_input(match: re.Match[str]) -> str:
        name, expected_literal = match.groups()
        actual = inputs.get(name, False)
        if isinstance(actual, str):
            actual = actual.lower() == "true"
        return str(bool(actual) == (expected_literal == "true"))

    expr = re.sub(r"inputs\.([A-Za-z0-9_]+) == (true|false)", repl_boolean_input, expr)

    def repl_input(match: re.Match[str]) -> str:
        negate = "not " if match.group(1) else ""
        value = inputs.get(match.group(2), "")
        return f"({negate}{value!r})"

    expr = re.sub(r"(!)?inputs\.([A-Za-z0-9_]+)", repl_input, expr)

    def repl_need(match: re.Match[str]) -> str:
        job = match.group(1)
        return f"({results.get(job, 'skipped')!r} == {match.group(2)!r})"

    expr = re.sub(r"needs\.([A-Za-z0-9_-]+)\.result == '([a-z]+)'", repl_need, expr)

    def repl_output_not_equal(match: re.Match[str]) -> str:
        job, name, expected = match.groups()
        return str(outputs.get((job, name), "") != expected)

    expr = re.sub(
        r"needs\.([A-Za-z0-9_-]+)\.outputs\.([A-Za-z0-9_-]+) != '([^']+)'",
        repl_output_not_equal,
        expr,
    )

    expr = expr.replace(" && ", " and ").replace(" || ", " or ")
    if not re.fullmatch(r"[A-Za-z0-9_'(), .=]+", expr):
        raise AssertionError(f"unsupported condition construct: {expr}")
    return bool(eval(expr, {"__builtins__": {}, "bool": bool}, {}))


class ReleaseRecoveryWorkflowContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.release_text = RELEASE_WORKFLOW.read_text(encoding="utf-8")
        cls.docker_text = DOCKER_WORKFLOW.read_text(encoding="utf-8")
        cls.pypi_text = PYPI_WORKFLOW.read_text(encoding="utf-8")
        cls.npm_text = NPM_WORKFLOW.read_text(encoding="utf-8")
        cls.bun_text = BUN_WORKFLOW.read_text(encoding="utf-8")
        cls.tauri_text = TAURI_WORKFLOW.read_text(encoding="utf-8")
        cls.contract_text = CONTRACT_WORKFLOW.read_text(encoding="utf-8")

    def test_exhaustive_github_release_writer_inventory_is_single_publisher_safe(
        self,
    ) -> None:
        writers = {
            path.name
            for path in WORKFLOW_DIR.glob("*.y*ml")
            if GITHUB_RELEASE_WRITER_PATTERN.search(path.read_text(encoding="utf-8"))
        }
        self.assertEqual(
            writers,
            {
                "publish-npm.yml",
                "publish-pypi.yml",
                "publish-bun.yml",
                "publish-tauri.yml",
                "release-comprehensive.yml",
                "release-coordinator.yml",
            },
            "every workflow that writes GitHub releases must be classified",
        )

        # Standard releases in the comprehensive producer/recovery workflow
        # are explicitly barred; component releases remain available.
        for job_name in ("create-release", "upload-recovered-release-assets"):
            self.assertIn(
                "needs.resolve-release-source.outputs.is_standard_release != 'true'",
                job_block(self.release_text, job_name),
                job_name,
            )

        npm_release = step_run_block(
            job_block(self.npm_text, "publish"), "Create GitHub Release"
        ) or step_run_block(
            job_block(self.npm_text, "publish-npm"), "Create GitHub Release"
        )
        self.assertIn("refs/tags/nodejs-", npm_release)

        pypi_release = step_run_block(
            job_block(self.pypi_text, "publish-pypi"), "Create GitHub Release"
        )
        self.assertIn("refs/tags/python-v", pypi_release)
        self.assertIn("refs/tags/pypi-v", pypi_release)
        self.assertNotIn("startsWith(github.ref, 'refs/tags/')", pypi_release)

        bun_release = step_run_block(
            job_block(self.bun_text, "publish-to-bun"),
            "Create Bun-specific GitHub Release",
        )
        self.assertIn("uses: actions/create-release@v1", bun_release)
        self.assertIn("refs/tags/bun-v", bun_release)
        self.assertNotIn("startsWith(github.ref, 'refs/tags/')", bun_release)
        self.assertNotIn("inputs.dry_run != 'true'", bun_release)

        tauri_release = step_run_block(
            job_block(self.tauri_text, "publish-tauri"),
            "Upload to GitHub Releases",
        )
        self.assertIn("refs/tags/app-v", tauri_release)
        self.assertIn("refs/tags/desktop-v", tauri_release)

    def test_release_writer_detector_accepts_quoted_and_unquoted_action_syntax(
        self,
    ) -> None:
        for syntax in (
            "uses: actions/create-release@v1",
            "uses:\tactions/create-release@main",
            'uses: "actions/create-release@v1"',
            "uses: 'actions/create-release@v1'",
            "USES: actions/create-release@v1",
        ):
            with self.subTest(syntax=syntax):
                self.assertRegex(syntax, GITHUB_RELEASE_WRITER_PATTERN)

    def test_bun_release_writer_allows_only_component_tag_and_non_dry_dispatch(
        self,
    ) -> None:
        condition = step_if_condition(
            job_block(self.bun_text, "publish-to-bun"),
            "Create Bun-specific GitHub Release",
        )
        self.assertTrue(condition)
        cases = (
            ("push", "refs/tags/bun-v1.2.3", False, True),
            ("release", "refs/tags/bun-v1.2.3", False, True),
            ("workflow_dispatch", "refs/tags/bun-v1.2.3", False, True),
            ("workflow_dispatch", "refs/tags/bun-v1.2.3", True, False),
            ("workflow_dispatch", "refs/tags/v1.2.3", False, False),
            ("workflow_dispatch", "refs/heads/main", False, False),
            ("push", "refs/tags/v1.2.3", False, False),
        )
        for event_name, ref, dry_run, expected in cases:
            with self.subTest(event_name=event_name, ref=ref, dry_run=dry_run):
                self.assertEqual(
                    evaluate_condition(
                        condition,
                        event_name=event_name,
                        ref=ref,
                        inputs={"dry_run": dry_run},
                    ),
                    expected,
                )

    def test_pypi_bare_tag_still_publishes_package_but_not_github_release(
        self,
    ) -> None:
        self.assertIn("- 'v*'", self.pypi_text)
        publish_job = job_block(self.pypi_text, "publish-pypi")
        publish_package = step_run_block(publish_job, "Run publish script")
        self.assertIn("./scripts/publish-pypi.sh", publish_package)
        self.assertNotIn("refs/tags/python-v", publish_package)
        github_release = step_run_block(publish_job, "Create GitHub Release")
        self.assertIn("refs/tags/python-v", github_release)
        self.assertIn("refs/tags/pypi-v", github_release)

    def test_validate_version_steps_use_runner_env_not_inline_expressions(
        self,
    ) -> None:
        """Shell comparisons must see real values, not ${{ }} literals (SC2193)."""
        for text, label in (
            (self.npm_text, "publish-npm"),
            (self.pypi_text, "publish-pypi"),
            (self.bun_text, "publish-bun"),
        ):
            step = step_run_block(job_block(text, "validate"), "Validate version format")
            self.assertTrue(step, label)
            self.assertNotIn('"${{ github.event_name }}"', step, label)
            self.assertNotIn('"${{ github.ref }}"', step, label)
            self.assertIn('"$GITHUB_EVENT_NAME" == "push"', step, label)
            self.assertIn('"$GITHUB_REF" == refs/tags/*', step, label)

    def test_strategy_steps_bind_inputs_via_env_and_quote_github_output(
        self,
    ) -> None:
        for text, label, job_name in (
            (self.npm_text, "publish-npm", "publish"),
            (self.bun_text, "publish-bun", "publish-to-bun"),
        ):
            step = step_run_block(
                job_block(text, job_name), "Determine publishing strategy"
            )
            self.assertTrue(step, label)
            self.assertNotIn('"${{ github.event_name }}"', step, label)
            self.assertNotIn('"${{ github.ref }}"', step, label)
            self.assertNotIn('"${{ inputs.version }}"', step, label)
            self.assertNotIn('"${{ inputs.tag }}"', step, label)
            self.assertIn("INPUT_VERSION: ${{ inputs.version }}", step, label)
            self.assertIn("INPUT_TAG: ${{ inputs.tag }}", step, label)
            self.assertNotIn(">> $GITHUB_OUTPUT", step, label)

    def test_bun_strategy_groups_github_output_appends(self) -> None:
        step = step_run_block(
            job_block(self.bun_text, "publish-to-bun"), "Determine publishing strategy"
        )
        self.assertIn('} >> "$GITHUB_OUTPUT"', step)

    def test_pypi_publish_script_invocation_uses_quoted_arg_array(self) -> None:
        publish_package = step_run_block(
            job_block(self.pypi_text, "publish-pypi"), "Run publish script"
        )
        self.assertIn('"${args[@]}"', publish_package)
        self.assertNotIn(" $ARGS", publish_package)
        self.assertIn('args=(--version "$VERSION"', publish_package)
        self.assertIn("PYPI_TOKEN", publish_package)

    def test_npm_and_bun_verify_steps_quote_package_spec(self) -> None:
        for text, label, job_name, step_name in (
            (self.npm_text, "publish-npm", "publish", "Verify published package"),
            (
                self.bun_text,
                "publish-bun",
                "publish-to-bun",
                "Verify package on GitHub Packages",
            ),
        ):
            step = step_run_block(job_block(text, job_name), step_name)
            self.assertTrue(step, label)
            self.assertIn('npm view "$PACKAGE_NAME@$PACKAGE_VERSION"', step, label)

    def test_node_globs_cannot_be_misread_as_ls_options(self) -> None:
        for text, label, job_name, step_name in (
            (self.npm_text, "publish-npm", "test-macos", "Rename universal binary for NAPI"),
            (self.npm_text, "publish-npm", "create-universal-macos", "Create universal binary"),
            (self.bun_text, "publish-bun", "create-universal-macos-bun", "Create universal binary"),
            (self.bun_text, "publish-bun", "publish-to-bun", "Prepare package for Bun publishing"),
        ):
            step = step_run_block(job_block(text, job_name), step_name)
            self.assertTrue(step, f"{label}:{step_name}")
            self.assertNotIn("ls -la *.node", step, label)
            self.assertIn("ls -la ./*.node", step, label)

    def test_single_publisher_contract_is_wired_into_ci_paths_and_execution(
        self,
    ) -> None:
        for path in (
            ".github/workflows/release-comprehensive.yml",
            ".github/workflows/publish-npm.yml",
            ".github/workflows/publish-pypi.yml",
            ".github/workflows/publish-bun.yml",
            ".github/workflows/publish-tauri.yml",
            "tests/release_control_mutation_test.py",
            "tests/workflow_release_recovery_contract_test.py",
            "tests/updater_r2_manifest_contract_test.py",
        ):
            self.assertEqual(
                self.contract_text.count(f'- "{path}"'),
                2,
                f"{path} must trigger both pull_request and push contracts",
            )
        self.assertIn(
            "python -m unittest -v tests.workflow_release_recovery_contract_test",
            self.contract_text,
        )
        self.assertIn(
            "python -m unittest -v tests.updater_r2_manifest_contract_test",
            self.contract_text,
        )
        contract_job = job_block(self.contract_text, "release-coordinator-contract")
        for step_name in (
            "actionlint (static workflow analysis)",
            "Parse workflow YAML",
        ):
            self.assertIn(
                ".github/workflows/publish-bun.yml",
                step_run_block(contract_job, step_name),
                step_name,
            )

    def test_manual_dispatch_requires_recovery_tag_and_expected_source_sha(
        self,
    ) -> None:
        release_tag = input_block(self.release_text, "release_tag")
        expected_sha = input_block(self.release_text, "expected_source_sha")

        self.assertIn("required: true", release_tag)
        self.assertIn("type: string", release_tag)
        self.assertIn("required: true", expected_sha)
        self.assertIn("type: string", expected_sha)

        resolver = job_block(self.release_text, "resolve-release-source")
        self.assertIn("^[0-9a-f]{40}$", resolver)

    def test_dispatch_recovery_accepts_only_component_tags(self) -> None:
        """P1-3: the manual recovery path must be reachable under a safe
        component-tag contract and fail closed for bare vX.Y.Z tags.

        Bare-tag recovery is owned exclusively by release-coordinator.yml, so
        the producer workflow's dispatch input must REQUIRE a component prefix
        (making `is_standard_release` structurally false on dispatch) instead
        of accepting only bare tags that can never satisfy the recovery guard.
        """
        resolver = job_block(self.release_text, "resolve-release-source")
        dispatch_branch = resolver.split('if [[ "$EVENT_NAME" == "workflow_dispatch" ]]')[
            1
        ].split("else", 1)[0]

        component_pattern = r"^[A-Za-z0-9_.-]+-v[0-9]+\.[0-9]+\.[0-9]+$"
        self.assertIn(
            component_pattern,
            dispatch_branch,
            "dispatch must require a component-prefixed tag so recovery is reachable",
        )
        regex = re.compile(component_pattern)
        # Reachable recovery inputs:
        self.assertTrue(regex.fullmatch("terraphim_server-v1.21.3"))
        self.assertTrue(regex.fullmatch("terraphim_grep-v1.21.3"))
        # Bare tags fail closed: single-writer ownership by the coordinator.
        self.assertFalse(regex.fullmatch("v1.21.3"))
        # Garbage fails closed.
        self.assertFalse(regex.fullmatch("../../etc"))
        self.assertFalse(regex.fullmatch(""))

        # The resolver marks component tags as non-standard so the recovery
        # job guard (`is_standard_release != 'true'`) can actually pass.
        self.assertIn('IS_STANDARD_RELEASE="false"', resolver)
        self.assertIn('VERSION="${RELEASE_TAG##*-v}"', resolver)

        # The advertised input documents the component-tag contract.
        release_tag = input_block(self.release_text, "release_tag")
        self.assertIn("component", release_tag.lower())
        self.assertIn("release-coordinator", release_tag)

    def test_recovery_job_is_reachable_for_component_tag_dispatch(self) -> None:
        """P1-3: with a component tag the dispatch guard must actually
        evaluate to true (previously unreachable under every valid input)."""
        condition_expr = job_if_condition(
            self.release_text, "upload-recovered-release-assets"
        )
        self.assertTrue(condition_expr)
        happy = {
            "verify-versions": "success",
            "sign-and-notarize-macos": "success",
            "verify-release-assets": "success",
            "build-server-managed-packages": "success",
        }
        self.assertTrue(
            evaluate_condition(
                condition_expr,
                event_name="workflow_dispatch",
                results=happy,
                inputs={"test_run": False},
                outputs={("resolve-release-source", "is_standard_release"): "false"},
            ),
            "component-tag dispatch recovery must be reachable",
        )
        # Bare (standard) releases stay coordinator-only even on dispatch.
        self.assertFalse(
            evaluate_condition(
                condition_expr,
                event_name="workflow_dispatch",
                results=happy,
                inputs={"test_run": False},
                outputs={("resolve-release-source", "is_standard_release"): "true"},
            ),
            "standard-release recovery must not bypass the coordinator",
        )
        # test_run never recovers.
        self.assertFalse(
            evaluate_condition(
                condition_expr,
                event_name="workflow_dispatch",
                results=happy,
                inputs={"test_run": True},
                outputs={("resolve-release-source", "is_standard_release"): "false"},
            ),
            "test_run dispatch must not mutate releases",
        )

    def test_resolver_has_read_only_contents_permission(self) -> None:
        resolver = job_block(self.release_text, "resolve-release-source")
        self.assertIn("permissions:", resolver)
        self.assertIn("contents: read", resolver)

    def test_resolver_runs_before_source_dependent_jobs(self) -> None:
        self.assertIn(
            "resolve-release-source", top_level_mapping_keys(self.release_text, "jobs")
        )

        for job_name in [
            "verify-versions",
            "build-binaries",
            "sign-and-notarize-macos",
            "build-debian-packages",
            "build-server-managed-packages",
            "verify-release-assets",
            "create-release",
            "upload-recovered-release-assets",
            "trigger-desktop-release",
            "trigger-clients-release",
            "build-docker",
        ]:
            self.assertTrue(
                has_need(
                    job_block(self.release_text, job_name), "resolve-release-source"
                ),
                job_name,
            )

    def test_source_dependent_checkouts_use_resolved_source_sha(self) -> None:
        for job_name in [
            "verify-versions",
            "build-binaries",
            "sign-and-notarize-macos",
            "build-debian-packages",
            "build-server-managed-packages",
            "verify-release-assets",
            "create-release",
            "upload-recovered-release-assets",
        ]:
            checkouts = checkout_blocks(job_block(self.release_text, job_name))
            self.assertTrue(checkouts, job_name)
            for checkout in checkouts:
                self.assertIn(
                    "ref: ${{ needs.resolve-release-source.outputs.source_sha }}",
                    checkout,
                    job_name,
                )
                self.assertIn("fetch-depth: 1", checkout, job_name)

    def test_release_mutation_and_waits_use_resolved_release_tag(self) -> None:
        self.assertIn(
            "tag_name: ${{ needs.resolve-release-source.outputs.release_tag }}",
            self.release_text,
        )
        self.assertNotRegex(
            self.release_text,
            r"tag_name:\s*\$\{\{\s*github\.ref_name\s*\}\}",
        )

        for forbidden in [
            'TAG="${{ github.ref_name }}"',
            "context.ref.replace('refs/tags/', '')",
            "context.ref.replace('refs/tags/v', '')",
            "VERSION=${GITHUB_REF#refs/tags/v}",
        ]:
            self.assertNotIn(forbidden, self.release_text)

    def test_critical_asset_patterns_are_preserved(self) -> None:
        for pattern in [
            r"terraphim_server-.*-x86_64-unknown-linux-gnu\\.tar\\.gz",
            r"terraphim_server-.*-x86_64-unknown-linux-musl\\.tar\\.gz",
            r"terraphim_server-.*-aarch64-unknown-linux-musl\\.tar\\.gz",
        ]:
            self.assertIn(pattern, self.release_text)

    def test_workflow_and_source_refs_are_distinct_outputs(self) -> None:
        resolver = job_block(self.release_text, "resolve-release-source")
        for output_name in [
            "source_sha",
            "source_ref",
            "workflow_ref",
            "version_series",
        ]:
            self.assertIn(f"{output_name}:", resolver)
        self.assertIn("source_sha=${SOURCE_SHA}", self.release_text)
        self.assertIn("version_series=${VERSION%.*}", self.release_text)
        self.assertIn("workflow_ref=${WORKFLOW_SHA}", self.release_text)
        self.assertIn("WORKFLOW_SHA: ${{ github.sha }}", resolver)
        self.assertNotIn("workflow_ref=${GITHUB_REF}", self.release_text)

    def test_component_version_parsing_prefers_component_separator(self) -> None:
        resolver = job_block(self.release_text, "resolve-release-source")
        component_index = resolver.index('if [[ "$RELEASE_TAG" == *"-v"* ]]')
        standard_index = resolver.index('VERSION="${RELEASE_TAG#v}"')
        self.assertLess(component_index, standard_index)
        self.assertIn('VERSION="${RELEASE_TAG##*-v}"', resolver)
        self.assertIn("^([A-Za-z0-9_.-]+-)?v[0-9]+\\.[0-9]+\\.[0-9]+$", resolver)

    def test_recovery_checksums_merge_existing_release_digests(self) -> None:
        recovery = job_block(self.release_text, "upload-recovered-release-assets")
        self.assertIn("Snapshot existing release asset digests", recovery)
        self.assertIn("releases/tags/${RELEASE_TAG}", recovery)
        self.assertIn('digest_re = re.compile(r"^sha256:([0-9a-f]{64})$")', recovery)
        self.assertIn('if name == "checksums.txt"', recovery)
        self.assertIn(
            "merged[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()",
            recovery,
        )
        self.assertIn("for name in sorted(merged)", recovery)
        self.assertNotIn("sha256sum * > checksums.txt", recovery)

    def test_recovery_checksum_merge_executes_and_fails_without_existing_digest(
        self,
    ) -> None:
        step = step_run_block(
            job_block(self.release_text, "upload-recovered-release-assets"),
            "Merge recovered and existing checksums",
        )
        embedded = step.split("python3 - <<'PY'\n", 1)[1].rsplit("\n          PY", 1)[0]
        script = textwrap.dedent(embedded)

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            assets = root / "assets"
            assets.mkdir()
            recovered = assets / "recovered.bin"
            recovered.write_bytes(b"recovered")
            old_digest = hashlib.sha256(b"existing").hexdigest()
            release_json = root / "release.json"
            release_json.write_text(
                json.dumps(
                    {
                        "assets": [
                            {"name": "existing.bin", "digest": f"sha256:{old_digest}"},
                            {"name": "checksums.txt", "digest": "sha256:" + "0" * 64},
                        ]
                    }
                )
            )
            env = dict(os.environ)
            env.update(
                RELEASE_ASSETS=str(assets),
                EXISTING_RELEASE_JSON=str(release_json),
            )
            result = subprocess.run(
                ["python3", "-c", script],
                env=env,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            checksums = (assets / "checksums.txt").read_text().splitlines()
            self.assertIn(f"{old_digest}  existing.bin", checksums)
            self.assertIn(
                f"{hashlib.sha256(b'recovered').hexdigest()}  recovered.bin",
                checksums,
            )

            release_json.write_text(
                json.dumps({"assets": [{"name": "existing.bin", "digest": None}]})
            )
            failed = subprocess.run(
                ["python3", "-c", script],
                env=env,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn(
                "invalid existing release asset digest metadata", failed.stderr
            )

    def test_no_tag_moving_commands_exist(self) -> None:
        combined = f"{self.release_text}\n{self.docker_text}"
        self.assertNotRegex(combined, r"\bgit\s+tag\s+-f\b")
        self.assertNotRegex(combined, r"\bgit\s+push\s+--force\b")
        self.assertNotRegex(combined, r"\bgit\s+push\b.*:refs/tags/")
        self.assertNotRegex(combined, r"\bgit\s+push\b.*--delete\b")

    def test_self_hosted_release_jobs_disable_rust_wrappers_before_toolchain(
        self,
    ) -> None:
        for job_name in [
            "build-binaries",
            "build-debian-packages",
            "build-server-managed-packages",
        ]:
            job = job_block(self.release_text, job_name)
            disable = step_run_block(
                job, "Disable Rust wrappers for self-hosted release builds"
            )
            for var in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"]:
                self.assertIn("unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER", disable)
                self.assertIn(f'echo "{var}="', disable)
            self.assertLess(
                job.index("Disable Rust wrappers for self-hosted release builds"),
                job.index("Install Rust toolchain"),
                job_name,
            )
        self.assertNotIn("Sanitize unavailable Rust wrappers", self.release_text)

    def test_cross_probe_is_non_fatal(self) -> None:
        install_cross = step_run_block(
            job_block(self.release_text, "build-binaries"), "Install cross"
        )
        self.assertIn("command -v cross", install_cross)
        self.assertIn("cross --version || true", install_cross)
        self.assertNotIn("cross --version\n            exit 0", install_cross)

    def test_docker_reusable_workflow_separates_source_and_build_recipe_refs(
        self,
    ) -> None:
        source_ref = input_block(self.docker_text, "source_ref")
        recipe_ref = input_block(self.docker_text, "build_recipe_ref")
        for block in [source_ref, recipe_ref]:
            self.assertIn("required: true", block)
            self.assertIn("type: string", block)

        frontend_checkouts = checkout_blocks(
            job_block(self.docker_text, "build-frontend")
        )
        self.assertEqual(len(frontend_checkouts), 1)
        self.assertIn("ref: ${{ inputs.source_ref }}", frontend_checkouts[0])

        build = job_block(self.docker_text, "build-and-push")
        build_checkouts = checkout_blocks(build)
        self.assertEqual(len(build_checkouts), 2)
        self.assertIn("ref: ${{ inputs.source_ref }}", build_checkouts[0])
        self.assertIn("ref: ${{ inputs.build_recipe_ref }}", build_checkouts[1])
        self.assertIn(
            "sparse-checkout: docker/Dockerfile.multiarch", build_checkouts[1]
        )
        overlay = step_run_block(build, "Overlay reviewed Docker build recipe")
        self.assertIn(
            "cp .release-recipe/docker/Dockerfile.multiarch docker/Dockerfile.multiarch",
            overlay,
        )

        caller = job_block(self.release_text, "build-docker")
        self.assertIn(
            "build_recipe_ref: ${{ needs.resolve-release-source.outputs.workflow_ref }}",
            caller,
        )

    def test_docker_reusable_workflow_uses_required_resolver_inputs_for_tags(
        self,
    ) -> None:
        for input_name in ["tag", "version", "version_series", "publish_latest"]:
            block = input_block(self.docker_text, input_name)
            self.assertIn("required: true", block, input_name)

        for tag_expr in [
            "type=raw,value=${{ inputs.tag }}-ubuntu${{ matrix.ubuntu-version }}",
            "type=raw,value=${{ inputs.version }}-ubuntu${{ matrix.ubuntu-version }}",
            "type=raw,value=${{ inputs.version_series }}-ubuntu${{ matrix.ubuntu-version }},enable=${{ inputs.publish_latest }}",
            "type=raw,value=latest-ubuntu${{ matrix.ubuntu-version }},enable=${{ inputs.publish_latest }}",
        ]:
            self.assertEqual(self.docker_text.count(tag_expr), 2, tag_expr)

        self.assertNotIn("type=semver", self.docker_text)
        self.assertNotIn("github.ref", self.docker_text)
        self.assertIn("if: inputs.push && !inputs.test_run", self.docker_text)

        caller = job_block(self.release_text, "build-docker")
        self.assertIn(
            "tag: ${{ needs.resolve-release-source.outputs.release_tag }}", caller
        )
        self.assertIn(
            "version: ${{ needs.resolve-release-source.outputs.version }}", caller
        )
        self.assertIn(
            "version_series: ${{ needs.resolve-release-source.outputs.version_series }}",
            caller,
        )
        self.assertIn(
            "publish_latest: ${{ github.event_name == 'push' && needs.resolve-release-source.outputs.is_standard_release == 'true' && !inputs.test_run }}",
            caller,
        )

    def test_docker_reusable_job_is_standard_release_only(self) -> None:
        caller = job_block(self.release_text, "build-docker")
        self.assertIn("needs.verify-versions.result == 'success'", caller)
        self.assertIn(
            "needs.resolve-release-source.outputs.is_standard_release == 'true'",
            caller,
        )
        self.assertIn("push: ${{ !inputs.test_run }}", caller)

    def test_docker_recovery_publishes_immutable_manifests_without_moving_tags(
        self,
    ) -> None:
        manifests = job_block(self.docker_text, "publish-release-manifests")
        ghcr = step_run_block(manifests, "Publish release manifests for GHCR")
        dockerhub = step_run_block(manifests, "Publish release manifests for DockerHub")

        self.assertIn("if: inputs.push && !inputs.test_run", manifests)
        self.assertNotIn(
            "if: inputs.push && !inputs.test_run && inputs.publish_latest", manifests
        )
        self.assertIn(
            "--tag ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ inputs.tag }}", ghcr
        )
        self.assertIn(
            "${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ inputs.tag }}-ubuntu22.04",
            ghcr,
        )
        self.assertIn(
            "--tag ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ inputs.version }}",
            ghcr,
        )
        self.assertIn(
            "${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ inputs.version }}-ubuntu22.04",
            ghcr,
        )
        self.assertIn("--tag ${{ env.DOCKERHUB_IMAGE }}:${{ inputs.tag }}", dockerhub)
        self.assertIn(
            "--tag ${{ env.DOCKERHUB_IMAGE }}:${{ inputs.version }}", dockerhub
        )

    def test_docker_moving_manifest_tags_are_publish_latest_gated(self) -> None:
        manifests = job_block(self.docker_text, "publish-release-manifests")
        ghcr = step_run_block(manifests, "Publish release manifests for GHCR")
        dockerhub = step_run_block(manifests, "Publish release manifests for DockerHub")

        for block in [ghcr, dockerhub]:
            moving_gate = block.index(
                'if [[ "${{ inputs.publish_latest }}" == "true" ]]; then'
            )
            self.assertLess(moving_gate, block.index(":${{ inputs.version_series }}"))
            self.assertLess(moving_gate, block.index(":latest"))

        self.assertIn(
            "${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ inputs.version_series }}-ubuntu22.04",
            ghcr,
        )
        self.assertIn(
            "${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:latest-ubuntu22.04", ghcr
        )
        self.assertIn(
            "${{ env.DOCKERHUB_IMAGE }}:${{ inputs.version_series }}-ubuntu22.04",
            dockerhub,
        )
        self.assertIn("${{ env.DOCKERHUB_IMAGE }}:latest-ubuntu22.04", dockerhub)

    def test_docker_summary_passes_source_ref_through_env(self) -> None:
        summary = step_run_block(
            job_block(self.docker_text, "build-and-push"),
            "Document transient BuildKit EOF recovery",
        )
        self.assertIn("SOURCE_REF: ${{ inputs.source_ref }}", summary)
        self.assertIn("source_ref remains $SOURCE_REF", summary)
        self.assertNotIn("source_ref remains '${{ inputs.source_ref }}'", summary)

    def test_docker_buildx_has_real_bounded_retry_or_explicit_manual_rerun(
        self,
    ) -> None:
        has_command_retry = all(
            marker in self.docker_text
            for marker in [
                "docker buildx build",
                "for attempt in",
                "MAX_BUILDX_ATTEMPTS",
                "docker buildx rm",
            ]
        )
        has_operational_manual_rerun = (
            "Known transient BuildKit EOF" in self.docker_text
            and "rerun the failed Docker job manually" in self.docker_text
            and "retry:" not in self.docker_text
        )
        self.assertTrue(has_command_retry or has_operational_manual_rerun)

    def test_universal_macos_is_gated_by_source_resolution_and_complete_builds(
        self,
    ) -> None:
        self.assertIn(
            "needs: [resolve-release-source, build-binaries]", self.release_text
        )
        self.assertIn(
            "needs.resolve-release-source.result == 'success'", self.release_text
        )
        self.assertIn("needs.build-binaries.result == 'success'", self.release_text)
        self.assertNotIn(
            "needs.build-binaries.result != 'cancelled'", self.release_text
        )

    def test_test_run_skips_all_release_mutation_jobs(self) -> None:
        for job_name in [
            "create-release",
            "upload-recovered-release-assets",
        ]:
            self.assertIn(
                "!inputs.test_run", job_block(self.release_text, job_name), job_name
            )

        for job_name in ["trigger-desktop-release", "trigger-clients-release"]:
            job = job_block(self.release_text, job_name)
            self.assertIn("github.event_name == 'push'", job, job_name)
            self.assertIn(
                "needs.resolve-release-source.outputs.is_standard_release == 'true'",
                job,
                job_name,
            )

        self.assertIn(
            "push: ${{ !inputs.test_run }}",
            job_block(self.release_text, "build-docker"),
        )
        self.assertIn("if: inputs.push && !inputs.test_run", self.docker_text)

    def test_push_release_creation_is_separate_from_manual_recovery_upload(
        self,
    ) -> None:
        create_release = job_block(self.release_text, "create-release")
        recovery = job_block(self.release_text, "upload-recovered-release-assets")

        self.assertIn("github.event_name == 'push'", create_release)
        self.assertNotIn(
            "needs.resolve-release-source.outputs.is_standard_release == 'true'",
            create_release,
        )
        self.assertIn("make_latest: true", create_release)
        self.assertIn("body: |", create_release)
        self.assertNotIn("gh release upload", create_release)

        self.assertIn("github.event_name == 'workflow_dispatch'", recovery)
        self.assertIn(
            'gh release upload "$RELEASE_TAG" release-assets/* --repo "$GITHUB_REPOSITORY" --clobber',
            recovery,
        )
        self.assertNotIn("softprops/action-gh-release", recovery)
        self.assertNotIn("make_latest", recovery)
        self.assertNotIn("body: |", recovery)

    def test_clients_and_desktop_dispatch_only_on_standard_tag_push(self) -> None:
        for job_name in ["trigger-desktop-release", "trigger-clients-release"]:
            job = job_block(self.release_text, job_name)
            self.assertIn("github.event_name == 'push'", job, job_name)
            self.assertIn(
                "needs.resolve-release-source.outputs.is_standard_release == 'true'",
                job,
                job_name,
            )

        clients = job_block(self.release_text, "trigger-clients-release")
        self.assertIn("async function resolveTagCommit", clients)
        self.assertIn("github.rest.git.getRef", clients)
        self.assertIn("github.rest.git.getTag", clients)
        self.assertIn(
            "const expectedSourceSha = await resolveTagCommit('terraphim', 'terraphim-clients', releaseTag);",
            clients,
        )
        self.assertIn("source_ref: releaseTag", clients)
        self.assertIn("expected_source_sha: expectedSourceSha", clients)

    def test_dead_legacy_client_wait_and_homebrew_jobs_are_removed(self) -> None:
        """P2-1: `wait-for-client-binaries`/`update-homebrew` were permanently
        dead (they required `create-release` success while `create-release` is
        barred from standard releases). The coordinator owns Homebrew
        downstream dispatch, so the producer workflow must not carry a second,
        contradictory ownership story."""
        for job_name in ["wait-for-client-binaries", "update-homebrew"]:
            self.assertEqual(
                job_block(self.release_text, job_name),
                "",
                f"{job_name} must be removed from the producer workflow",
            )
            self.assertNotIn(job_name, self.release_text)

    def test_pypi_release_creation_uses_boolean_dry_run_comparison(self) -> None:
        """P2-3: `type: boolean` inputs must be compared to booleans, not to
        the string 'true' (which is truthy-mismatched under GitHub's loose
        comparison and would publish during a dry run)."""
        publish_job = job_block(self.pypi_text, "publish-pypi")
        github_release = step_run_block(publish_job, "Create GitHub Release")
        self.assertIn("inputs.dry_run == false", github_release)
        self.assertNotIn("inputs.dry_run != 'true'", github_release)

        verify_step = step_run_block(publish_job, "Verify published packages")
        self.assertIn("inputs.dry_run == false", verify_step)
        self.assertNotIn("inputs.dry_run != 'true'", verify_step)

        condition = step_if_condition(publish_job, "Create GitHub Release")
        self.assertTrue(condition)
        cases = (
            ("push", "refs/tags/python-v1.2.3", False, True),
            ("push", "refs/tags/pypi-v1.2.3", False, True),
            ("push", "refs/tags/v1.2.3", False, False),
            ("workflow_dispatch", "refs/tags/python-v1.2.3", True, False),
            ("workflow_dispatch", "refs/tags/python-v1.2.3", False, True),
        )
        for event_name, ref, dry_run, expected in cases:
            with self.subTest(event_name=event_name, ref=ref, dry_run=dry_run):
                self.assertEqual(
                    evaluate_condition(
                        condition,
                        event_name=event_name,
                        ref=ref,
                        inputs={"dry_run": dry_run},
                    ),
                    expected,
                )

    def test_managed_packages_are_release_and_recovery_dependencies(self) -> None:
        for job_name in ["create-release", "upload-recovered-release-assets"]:
            job = job_block(self.release_text, job_name)
            self.assertTrue(has_need(job, "build-server-managed-packages"), job_name)

    def test_managed_packages_job_runs_on_tag_push_or_explicit_dispatch(self) -> None:
        managed = job_block(self.release_text, "build-server-managed-packages")
        self.assertIn("github.event_name == 'push'", managed)
        self.assertIn(
            "(github.event_name == 'workflow_dispatch' && inputs.managed_packages == 'enabled')",
            managed,
        )
        # The dispatch opt-in must not gate the plain tag-push path.
        condition = managed[managed.index("if: >-") : managed.index("runs-on:")]
        self.assertLess(
            condition.index("github.event_name == 'push'"),
            condition.index("workflow_dispatch"),
        )

        condition_expr = job_if_condition(
            self.release_text, "build-server-managed-packages"
        )
        self.assertTrue(condition_expr)
        happy = {
            "verify-versions": "success",
            "build-binaries": "success",
        }
        self.assertTrue(
            evaluate_condition(condition_expr, event_name="push", results=happy),
            "tag push must run the managed package job",
        )
        self.assertTrue(
            evaluate_condition(
                condition_expr,
                event_name="workflow_dispatch",
                results=happy,
                inputs={"managed_packages": "enabled"},
            ),
            "dispatch with managed_packages=enabled must run the managed package job",
        )
        self.assertFalse(
            evaluate_condition(
                condition_expr,
                event_name="workflow_dispatch",
                results=happy,
                inputs={"managed_packages": "disabled"},
            ),
            "dispatch with managed_packages=disabled must not run the managed package job",
        )
        self.assertFalse(
            evaluate_condition(
                condition_expr,
                event_name="push",
                results={"verify-versions": "success", "build-binaries": "failure"},
            ),
            "a failed build matrix must not run the managed package job",
        )

    def test_release_creation_requires_managed_success_never_skipped(self) -> None:
        create_release = job_block(self.release_text, "create-release")
        self.assertIn(
            "needs.build-server-managed-packages.result == 'success'",
            create_release,
        )
        # create-release must never accept a skipped (or via the skipped
        # clause, failed-then-skipped) managed producer.
        self.assertNotIn(
            "needs.build-server-managed-packages.result == 'skipped'",
            create_release,
        )
        self.assertIn(
            "Tag-push publication requires the managed package producer",
            create_release,
        )

    def test_release_creation_cannot_publish_when_managed_skipped_or_failed(
        self,
    ) -> None:
        condition_expr = job_if_condition(self.release_text, "create-release")
        self.assertTrue(condition_expr)
        happy = {
            "verify-versions": "success",
            "sign-and-notarize-macos": "success",
            "verify-release-assets": "success",
            "build-server-managed-packages": "success",
        }
        self.assertFalse(
            evaluate_condition(
                condition_expr,
                event_name="push",
                results=happy,
                outputs={("resolve-release-source", "is_standard_release"): "true"},
            ),
            "a standard release must remain producer-only for the coordinator",
        )
        self.assertTrue(
            evaluate_condition(
                condition_expr,
                event_name="push",
                results=happy,
                outputs={("resolve-release-source", "is_standard_release"): "false"},
            ),
            "component-prefixed releases remain outside the coordinator contract",
        )
        for blocked in ["skipped", "failure", "cancelled"]:
            results = dict(happy, **{"build-server-managed-packages": blocked})
            self.assertFalse(
                evaluate_condition(
                    condition_expr,
                    event_name="push",
                    results=results,
                    outputs={
                        ("resolve-release-source", "is_standard_release"): "false"
                    },
                ),
                f"create-release must not publish when the managed job is {blocked}",
            )
        # The recovery-only relaxation (explicit skipped acceptance) must
        # not be reachable from create-release on a tag push.
        recovery_expr = job_if_condition(
            self.release_text, "upload-recovered-release-assets"
        )
        self.assertTrue(recovery_expr)
        self.assertIn(
            "needs.build-server-managed-packages.result == 'skipped'", recovery_expr
        )

    def test_recovery_explicitly_accepts_skipped_managed_producer(self) -> None:
        recovery = job_block(self.release_text, "upload-recovered-release-assets")
        self.assertIn(
            "needs.build-server-managed-packages.result == 'success' || needs.build-server-managed-packages.result == 'skipped'",
            recovery,
        )
        condition_expr = job_if_condition(
            self.release_text, "upload-recovered-release-assets"
        )
        happy = {
            "verify-versions": "success",
            "sign-and-notarize-macos": "success",
            "verify-release-assets": "success",
            "build-server-managed-packages": "skipped",
        }
        self.assertFalse(
            evaluate_condition(
                condition_expr,
                event_name="workflow_dispatch",
                results=happy,
                outputs={("resolve-release-source", "is_standard_release"): "true"},
            ),
            "standard-release recovery must not bypass the coordinator",
        )

    def test_release_and_recovery_download_all_managed_matrix_artifacts(self) -> None:
        managed = job_block(self.release_text, "build-server-managed-packages")
        self.assertIn("name: server-managed-packages-${{ matrix.target }}", managed)
        self.assertIn("path: server-managed-packages/${{ matrix.target }}/*", managed)

        for job_name in ["create-release", "upload-recovered-release-assets"]:
            job = job_block(self.release_text, job_name)
            self.assertIn("pattern: server-managed-packages-*", job, job_name)
            self.assertIn("merge-multiple: false", job, job_name)
            self.assertIn(
                "if: needs.build-server-managed-packages.result == 'success'",
                job,
                job_name,
            )

    def test_release_inventory_is_gated_by_assemble_step(self) -> None:
        for job_name in ["create-release", "upload-recovered-release-assets"]:
            job = job_block(self.release_text, job_name)
            self.assertIn("assemble-release-inventory.sh", job, job_name)
            self.assertIn("--managed-target x86_64-unknown-linux-musl", job, job_name)
            self.assertIn("--managed-target aarch64-unknown-linux-musl", job, job_name)

    def test_release_inventory_call_sites_are_managed_only(self) -> None:
        """The authoritative release paths must never merge legacy and managed DEBs.

        The legacy host-native cargo-deb package and the managed x86_64 DEB
        share the canonical basename terraphim-server_<version>-1_amd64.deb,
        so passing both stages to the assembler would make every normal tag
        push fail closed on duplicate rejection. Both call sites therefore
        stay managed-only: no debian-packages download, no legacy-deb
        staging directory, no --legacy flag, and no legacy resurrection in
        the artifact-download retry path.
        """
        assemble_step = (
            "Assemble release inventory (managed all-or-nothing, duplicate rejection)"
        )
        for job_name in ["create-release", "upload-recovered-release-assets"]:
            job = job_block(self.release_text, job_name)
            self.assertNotIn("--legacy", job, job_name)
            self.assertNotIn("path: legacy-deb", job, job_name)
            self.assertNotIn("name: debian-packages", job, job_name)
            assemble = step_run_block(job, assemble_step)
            self.assertTrue(assemble, job_name)
            self.assertIn("--managed-staging managed-staging", assemble, job_name)
            self.assertNotIn("--legacy", assemble, job_name)

        retry = step_run_block(
            job_block(self.release_text, "create-release"),
            "Retry artifact download if needed",
        )
        self.assertTrue(retry)
        self.assertNotIn("legacy-deb", retry)
        self.assertNotIn("debian-packages", retry)

        # The legacy cargo-deb producer survives only as non-authoritative
        # validation; nothing downstream may consume its artifact.
        legacy_producer = job_block(self.release_text, "build-debian-packages")
        self.assertIn("continue-on-error: true", legacy_producer)
        self.assertIn("name: debian-packages", legacy_producer)

    def test_release_inventory_assembler_rejects_duplicates_and_partial_matrix(
        self,
    ) -> None:
        assembler = ROOT / ".github/scripts/release/assemble-release-inventory.sh"
        self.assertTrue(assembler.exists(), "assemble-release-inventory.sh must exist")

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            out = root / "release-assets"
            out.mkdir()
            (out / "terraphim_server-universal-apple-darwin").write_bytes(b"bin")
            legacy = root / "legacy-deb"
            legacy.mkdir()
            (legacy / "terraphim-server_1.0.0-1_amd64.deb").write_bytes(
                b"legacy-native"
            )
            staging = root / "managed-staging"
            for target, arch in [
                ("x86_64-unknown-linux-musl", "amd64"),
                ("aarch64-unknown-linux-musl", "arm64"),
            ]:
                target_dir = staging / f"server-managed-packages-{target}"
                target_dir.mkdir(parents=True)
                (target_dir / f"terraphim-server_1.0.0-1_{arch}.deb").write_bytes(
                    b"managed"
                )
                (
                    target_dir
                    / f"terraphim-server-1.0.0-1.{arch.replace('amd64', 'x86_64')}.rpm"
                ).write_bytes(b"managed")
                (
                    target_dir
                    / f"terraphim-server-1.0.0-{target}.package-sha256sums.txt"
                ).write_bytes(b"managed")

            command = [
                str(assembler),
                "--output",
                str(out),
                "--legacy",
                str(legacy),
                "--managed-staging",
                str(staging),
                "--managed-target",
                "x86_64-unknown-linux-musl",
                "--managed-target",
                "aarch64-unknown-linux-musl",
            ]

            # The legacy amd64 cargo-deb basename collides with the managed
            # amd64 DEB; the assembler must fail closed on the conflict.
            conflicted = subprocess.run(
                command, check=False, capture_output=True, text=True
            )
            self.assertNotEqual(conflicted.returncode, 0, conflicted.stdout)
            self.assertIn("duplicate release asset basename", conflicted.stderr)
            self.assertIn("terraphim-server_1.0.0-1_amd64.deb", conflicted.stderr)
            self.assertFalse((out / "terraphim-server_1.0.0-1_amd64.deb").exists())

            # A partial managed matrix is all-or-nothing.
            (legacy / "terraphim-server_1.0.0-1_amd64.deb").unlink()
            shutil.rmtree(
                staging / "server-managed-packages-aarch64-unknown-linux-musl"
            )
            partial = subprocess.run(
                command, check=False, capture_output=True, text=True
            )
            self.assertNotEqual(partial.returncode, 0, partial.stdout)
            self.assertIn("managed package matrix incomplete", partial.stderr)
            self.assertIn("aarch64-unknown-linux-musl", partial.stderr)

            # A normal managed-only inventory (the workflow's exact
            # call shape: no --legacy stage at all) succeeds.
            managed_only_command = [
                str(assembler),
                "--output",
                str(out),
                "--managed-staging",
                str(staging),
                "--managed-target",
                "x86_64-unknown-linux-musl",
                "--managed-target",
                "aarch64-unknown-linux-musl",
            ]
            shutil.rmtree(staging)
            for target, deb_arch, rpm_arch in [
                ("x86_64-unknown-linux-musl", "amd64", "x86_64"),
                ("aarch64-unknown-linux-musl", "arm64", "aarch64"),
            ]:
                target_dir = staging / f"server-managed-packages-{target}"
                target_dir.mkdir(parents=True)
                (target_dir / f"terraphim-server_1.0.0-1_{deb_arch}.deb").write_bytes(
                    b"managed"
                )
                (target_dir / f"terraphim-server-1.0.0-1.{rpm_arch}.rpm").write_bytes(
                    b"managed"
                )
                (
                    target_dir
                    / f"terraphim-server-1.0.0-{target}.package-sha256sums.txt"
                ).write_bytes(b"managed")
            merged = subprocess.run(
                managed_only_command, check=False, capture_output=True, text=True
            )
            self.assertEqual(merged.returncode, 0, merged.stderr)
            self.assertTrue((out / "terraphim-server_1.0.0-1_amd64.deb").exists())
            self.assertTrue((out / "terraphim-server_1.0.0-1_arm64.deb").exists())
            self.assertTrue((out / "terraphim-server-1.0.0-1.x86_64.rpm").exists())
            self.assertTrue((out / "terraphim-server-1.0.0-1.aarch64.rpm").exists())
            self.assertTrue(
                (
                    out
                    / "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt"
                ).exists()
            )
            self.assertTrue(
                (
                    out
                    / "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"
                ).exists()
            )

    def test_release_uploads_include_assembled_managed_inventory(self) -> None:
        create_release = job_block(self.release_text, "create-release")
        recovery = job_block(self.release_text, "upload-recovered-release-assets")

        # Both publishing paths upload everything assembled into
        # release-assets, which the assemble step extended with the managed
        # DEB/RPM outputs and their package checksum manifests.
        self.assertIn("files: release-assets/*", create_release)
        self.assertIn(
            'gh release upload "$RELEASE_TAG" release-assets/* --repo "$GITHUB_REPOSITORY" --clobber',
            recovery,
        )
        # Checksum inventories are computed over the assembled directory.
        self.assertIn("working-directory: release-assets", create_release)

    def test_managed_parity_cargo_deb_is_per_target_from_qualified_musl_bytes(
        self,
    ) -> None:
        managed = job_block(self.release_text, "build-server-managed-packages")
        # Parity is built per matrix target from the exact qualified MUSL
        # input staged into the cargo-deb assets path, never from a host
        # native cargo-deb build.
        self.assertIn("cargo deb -p terraphim_server", managed)
        self.assertIn("--no-build --no-strip", managed)
        self.assertIn('--target "$TARGET"', managed)
        self.assertIn('--output "cargo-deb-parity/${TARGET}"', managed)
        self.assertIn('cp "$BIN" "target/${TARGET}/release/terraphim_server"', managed)
        self.assertIn('--cargo-deb-dir "cargo-deb-parity/${TARGET}"', managed)
        self.assertNotIn("name: debian-packages", managed)
        self.assertNotIn("cargo-deb-artifact", self.release_text)
        self.assertNotIn("needs.build-debian-packages.result == 'success'", managed)

    def test_native_gate_require_install_matches_runner_arch(self) -> None:
        gate = step_run_block(
            job_block(self.release_text, "build-server-managed-packages"),
            "Run native managed-package lifecycle gate",
        )
        self.assertIn("REQUIRE_INSTALL:", gate)
        self.assertIn(
            "runner.arch == 'X64' && matrix.target == 'x86_64-unknown-linux-musl'",
            gate,
        )
        self.assertIn(
            "runner.arch == 'ARM64' && matrix.target == 'aarch64-unknown-linux-musl'",
            gate,
        )
        self.assertIn("&& '1' || '0'", gate)


class HostileInputContract(unittest.TestCase):
    TAG_RE = re.compile(r"^v[0-9]+\.[0-9]+\.[0-9]+$")
    SHA_RE = re.compile(r"^[0-9a-f]{40}$")

    def test_release_tag_contract(self) -> None:
        accepted = ["v1.21.3"]
        rejected = [
            "main",
            "release/v1.21.3",
            "v1.21.3;echo bad",
            "v1.21.3 $(echo bad)",
            "../v1.21.3",
            "terraphim_server-v1.21.3",
            "v1.21",
            "v1.21.3-rc.1",
            "4a1d9f24c99f1504fdb2476667aa1087b698d33c",
        ]

        for tag in accepted:
            self.assertRegex(tag, self.TAG_RE)
        for tag in rejected:
            self.assertNotRegex(tag, self.TAG_RE)

    def test_expected_source_sha_contract(self) -> None:
        self.assertRegex("4a1d9f24c99f1504fdb2476667aa1087b698d33c", self.SHA_RE)
        for sha in [
            "",
            "4a1d9f24",
            "4A1D9F24C99F1504FDB2476667AA1087B698D33C",
            "4a1d9f24c99f1504fdb2476667aa1087b698d33z",
            "4a1d9f24c99f1504fdb2476667aa1087b698d33c;echo bad",
            "../4a1d9f24c99f1504fdb2476667aa1087b698d33c",
        ]:
            self.assertNotRegex(sha, self.SHA_RE)


class AgentEvalWorkflowContract(unittest.TestCase):
    """agent-eval.yml runs Cargo against the private terraphim registry in
    both baseline and candidate captures, so the job must bind the registry
    token from secrets (GitHub job 105824356527 failed with "no token found
    for terraphim" without it). The token must stay a job-level env binding:
    never inlined into run scripts and never logged."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.text = AGENT_EVAL_WORKFLOW.read_text(encoding="utf-8")

    def test_eval_job_binds_terraphim_registry_token_from_secrets(self) -> None:
        job = job_block(self.text, "eval")
        self.assertTrue(job)
        self.assertIn(
            "CARGO_REGISTRIES_TERRAPHIM_TOKEN: ${{ secrets.CARGO_REGISTRIES_TERRAPHIM_TOKEN }}",
            job,
        )

    def test_token_binding_is_job_level_env_covering_both_captures(self) -> None:
        job = job_block(self.text, "eval")
        env_index = job.find("CARGO_REGISTRIES_TERRAPHIM_TOKEN:")
        steps_index = job.find("steps:")
        self.assertNotEqual(env_index, -1)
        self.assertNotEqual(steps_index, -1)
        self.assertLess(env_index, steps_index, "binding must be job-level env")
        self.assertIn("evaluate-agent.sh --mode baseline", job)
        self.assertIn("--mode candidate", job)

    def test_token_is_never_inlined_into_scripts_or_logged(self) -> None:
        # Exactly two occurrences: the env key and the secrets.* reference.
        self.assertEqual(self.text.count("CARGO_REGISTRIES_TERRAPHIM_TOKEN"), 2)


if __name__ == "__main__":
    unittest.main()
