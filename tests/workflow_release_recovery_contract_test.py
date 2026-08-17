#!/usr/bin/env python3
"""Dependency-free static contracts for the v1.21.3 release recovery workflows."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RELEASE_WORKFLOW = ROOT / ".github/workflows/release-comprehensive.yml"
DOCKER_WORKFLOW = ROOT / ".github/workflows/docker-multiarch.yml"


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
        base = indent_of(lines[index - 1]) if index > 0 and "- name:" in lines[index - 1] else indent_of(line)
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


def has_need(job: str, dependency: str) -> bool:
    return bool(
        re.search(rf"^\s+needs:\s*{re.escape(dependency)}\s*$", job, re.MULTILINE)
        or re.search(rf"^\s+needs:\s*\[[^\]]*\b{re.escape(dependency)}\b", job, re.MULTILINE)
        or re.search(rf"^\s+-\s*{re.escape(dependency)}\s*$", job, re.MULTILINE)
    )


class ReleaseRecoveryWorkflowContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.release_text = RELEASE_WORKFLOW.read_text(encoding="utf-8")
        cls.docker_text = DOCKER_WORKFLOW.read_text(encoding="utf-8")

    def test_manual_dispatch_requires_recovery_tag_and_expected_source_sha(self) -> None:
        release_tag = input_block(self.release_text, "release_tag")
        expected_sha = input_block(self.release_text, "expected_source_sha")

        self.assertIn("required: true", release_tag)
        self.assertIn("type: string", release_tag)
        self.assertIn("required: true", expected_sha)
        self.assertIn("type: string", expected_sha)

        resolver = job_block(self.release_text, "resolve-release-source")
        self.assertIn("^v[0-9]+\\.[0-9]+\\.[0-9]+$", resolver)
        self.assertIn("^[0-9a-f]{40}$", resolver)

    def test_resolver_runs_before_source_dependent_jobs(self) -> None:
        self.assertIn("resolve-release-source", top_level_mapping_keys(self.release_text, "jobs"))

        for job_name in [
            "verify-versions",
            "build-binaries",
            "sign-and-notarize-macos",
            "build-debian-packages",
            "verify-release-assets",
            "create-release",
            "upload-recovered-release-assets",
            "trigger-desktop-release",
            "trigger-clients-release",
            "build-docker",
            "wait-for-client-binaries",
            "update-homebrew",
        ]:
            self.assertTrue(
                has_need(job_block(self.release_text, job_name), "resolve-release-source"),
                job_name,
            )

    def test_source_dependent_checkouts_use_resolved_source_sha(self) -> None:
        for job_name in [
            "verify-versions",
            "build-binaries",
            "sign-and-notarize-macos",
            "build-debian-packages",
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
        for output_name in ["source_sha", "source_ref", "workflow_ref", "version_series"]:
            self.assertIn(f"{output_name}:", resolver)
        self.assertIn("source_sha=${SOURCE_SHA}", self.release_text)
        self.assertIn("version_series=${VERSION%.*}", self.release_text)
        self.assertIn("workflow_ref=${GITHUB_REF}", self.release_text)

    def test_component_version_parsing_prefers_component_separator(self) -> None:
        resolver = job_block(self.release_text, "resolve-release-source")
        component_index = resolver.index('if [[ "$RELEASE_TAG" == *"-v"* ]]')
        standard_index = resolver.index('VERSION="${RELEASE_TAG#v}"')
        self.assertLess(component_index, standard_index)
        self.assertIn('VERSION="${RELEASE_TAG##*-v}"', resolver)

    def test_no_tag_moving_commands_exist(self) -> None:
        combined = f"{self.release_text}\n{self.docker_text}"
        self.assertNotRegex(combined, r"\bgit\s+tag\s+-f\b")
        self.assertNotRegex(combined, r"\bgit\s+push\s+--force\b")
        self.assertNotRegex(combined, r"\bgit\s+push\b.*:refs/tags/")
        self.assertNotRegex(combined, r"\bgit\s+push\b.*--delete\b")

    def test_self_hosted_release_jobs_disable_rust_wrappers_before_toolchain(self) -> None:
        for job_name in ["build-binaries", "build-debian-packages"]:
            job = job_block(self.release_text, job_name)
            disable = step_run_block(
                job, "Disable Rust wrappers for self-hosted release builds"
            )
            for var in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"]:
                self.assertIn(f"unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER", disable)
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

    def test_docker_reusable_workflow_checks_out_source_ref(self) -> None:
        source_ref = input_block(self.docker_text, "source_ref")
        self.assertIn("required: true", source_ref)
        self.assertIn("type: string", source_ref)

        for job_name in ["build-frontend", "build-and-push"]:
            checkouts = checkout_blocks(job_block(self.docker_text, job_name))
            self.assertTrue(checkouts, job_name)
            for checkout in checkouts:
                self.assertIn("ref: ${{ inputs.source_ref }}", checkout)
                self.assertIn("fetch-depth: 1", checkout)

    def test_docker_reusable_workflow_uses_required_resolver_inputs_for_tags(self) -> None:
        for input_name in ["tag", "version", "version_series", "publish_latest"]:
            block = input_block(self.docker_text, input_name)
            self.assertIn("required: true", block, input_name)

        for tag_expr in [
            "type=raw,value=${{ inputs.tag }}-ubuntu${{ matrix.ubuntu-version }}",
            "type=raw,value=${{ inputs.version }}-ubuntu${{ matrix.ubuntu-version }}",
            "type=raw,value=${{ inputs.version_series }}-ubuntu${{ matrix.ubuntu-version }}",
            "type=raw,value=latest-ubuntu${{ matrix.ubuntu-version }},enable=${{ inputs.publish_latest }}",
        ]:
            self.assertEqual(self.docker_text.count(tag_expr), 2, tag_expr)

        self.assertNotIn("type=semver", self.docker_text)
        self.assertNotIn("github.ref", self.docker_text)
        self.assertIn("if: inputs.push && !inputs.test_run && inputs.publish_latest", self.docker_text)

        caller = job_block(self.release_text, "build-docker")
        self.assertIn("tag: ${{ needs.resolve-release-source.outputs.release_tag }}", caller)
        self.assertIn("version: ${{ needs.resolve-release-source.outputs.version }}", caller)
        self.assertIn("version_series: ${{ needs.resolve-release-source.outputs.version_series }}", caller)
        self.assertIn("publish_latest: ${{ github.event_name == 'push' && needs.resolve-release-source.outputs.is_standard_release == 'true' && !inputs.test_run }}", caller)

    def test_docker_summary_passes_source_ref_through_env(self) -> None:
        summary = step_run_block(
            job_block(self.docker_text, "build-and-push"),
            "Document transient BuildKit EOF recovery",
        )
        self.assertIn("SOURCE_REF: ${{ inputs.source_ref }}", summary)
        self.assertIn("source_ref remains $SOURCE_REF", summary)
        self.assertNotIn("source_ref remains '${{ inputs.source_ref }}'", summary)

    def test_docker_buildx_has_real_bounded_retry_or_explicit_manual_rerun(self) -> None:
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

    def test_universal_macos_is_gated_by_source_resolution_and_complete_builds(self) -> None:
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
            "wait-for-client-binaries",
            "update-homebrew",
        ]:
            self.assertIn("!inputs.test_run", job_block(self.release_text, job_name), job_name)

        for job_name in ["trigger-desktop-release", "trigger-clients-release"]:
            job = job_block(self.release_text, job_name)
            self.assertIn("github.event_name == 'push'", job, job_name)
            self.assertIn("needs.resolve-release-source.outputs.is_standard_release == 'true'", job, job_name)

        self.assertIn("push: ${{ !inputs.test_run }}", job_block(self.release_text, "build-docker"))
        self.assertIn("if: inputs.push && !inputs.test_run && inputs.publish_latest", self.docker_text)

    def test_push_release_creation_is_separate_from_manual_recovery_upload(self) -> None:
        create_release = job_block(self.release_text, "create-release")
        recovery = job_block(self.release_text, "upload-recovered-release-assets")

        self.assertIn("github.event_name == 'push'", create_release)
        self.assertIn("make_latest: true", create_release)
        self.assertIn("body: |", create_release)
        self.assertNotIn("gh release upload", create_release)

        self.assertIn("github.event_name == 'workflow_dispatch'", recovery)
        self.assertIn('gh release upload "$RELEASE_TAG" release-assets/* --repo "$GITHUB_REPOSITORY" --clobber', recovery)
        self.assertNotIn("softprops/action-gh-release", recovery)
        self.assertNotIn("make_latest", recovery)
        self.assertNotIn("body: |", recovery)

    def test_clients_and_desktop_dispatch_only_on_standard_tag_push(self) -> None:
        for job_name in ["trigger-desktop-release", "trigger-clients-release"]:
            job = job_block(self.release_text, job_name)
            self.assertIn("github.event_name == 'push'", job, job_name)
            self.assertIn("needs.resolve-release-source.outputs.is_standard_release == 'true'", job, job_name)

        clients = job_block(self.release_text, "trigger-clients-release")
        self.assertIn("async function resolveTagCommit", clients)
        self.assertIn("github.rest.git.getRef", clients)
        self.assertIn("github.rest.git.getTag", clients)
        self.assertIn("const expectedSourceSha = await resolveTagCommit('terraphim', 'terraphim-clients', releaseTag);", clients)
        self.assertIn("source_ref: releaseTag", clients)
        self.assertIn("expected_source_sha: expectedSourceSha", clients)

    def test_wait_and_homebrew_are_tag_push_only_and_wait_does_not_need_trigger_clients(self) -> None:
        wait = job_block(self.release_text, "wait-for-client-binaries")
        homebrew = job_block(self.release_text, "update-homebrew")

        self.assertTrue(has_need(wait, "create-release"))
        self.assertFalse(has_need(wait, "trigger-clients-release"))
        for job in [wait, homebrew]:
            self.assertIn("github.event_name == 'push'", job)
            self.assertIn("!inputs.test_run", job)
            self.assertIn("needs.resolve-release-source.outputs.is_standard_release == 'true'", job)


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
        self.assertRegex(
            "4a1d9f24c99f1504fdb2476667aa1087b698d33c", self.SHA_RE
        )
        for sha in [
            "",
            "4a1d9f24",
            "4A1D9F24C99F1504FDB2476667AA1087B698D33C",
            "4a1d9f24c99f1504fdb2476667aa1087b698d33z",
            "4a1d9f24c99f1504fdb2476667aa1087b698d33c;echo bad",
            "../4a1d9f24c99f1504fdb2476667aa1087b698d33c",
        ]:
            self.assertNotRegex(sha, self.SHA_RE)


if __name__ == "__main__":
    unittest.main()
