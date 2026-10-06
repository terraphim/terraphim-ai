"""Dependency-free contracts for the narrow terraphim-lsp pre-release workflow.

The asset names are a contract with zed-terraphim provisioning (design step 6,
AC11); every cargo job must run on a self-hosted runner because the private
registry is reachable only from the tailnet (design section 9).
"""

from __future__ import annotations

import fnmatch
import hashlib
import os
import shutil
import re
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/release-lsp.yml"
COMPREHENSIVE = ROOT / ".github/workflows/release-comprehensive.yml"
TOOLCHAIN = ROOT / "rust-toolchain.toml"

LINUX_TARGETS = {
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
}
DARWIN_TARGETS = {"aarch64-apple-darwin", "x86_64-apple-darwin"}
RAW_ASSETS = sorted(
    [f"terraphim-lsp-{t}" for t in LINUX_TARGETS | DARWIN_TARGETS]
    + ["terraphim-lsp-universal-apple-darwin"]
)


def jobs(text: str) -> dict[str, str]:
    body = text[text.index("\njobs:\n") + len("\njobs:\n") :]
    parts = re.split(r"(?m)^  ([A-Za-z0-9_-]+):\n", body)
    return dict(zip(parts[1::2], parts[2::2]))


def runs_on(job: str) -> str:
    match = re.search(r"(?m)^    runs-on: (.+)$", job)
    assert match, "job has no runs-on"
    return match.group(1)


def step(job: str, name: str) -> str:
    match = re.search(rf"(?m)^      - name: {re.escape(name)}\n", job)
    assert match, f"step not found: {name!r}"
    rest = job[match.end() :]
    end = re.search(r"(?m)^      - name: ", rest)
    return rest[: end.start()] if end else rest


def step_run(job: str, name: str) -> str:
    block = step(job, name)
    match = re.search(r"(?m)^        run: \|\n", block)
    assert match, f"step has no run block: {name!r}"
    return textwrap.dedent(block[match.end() :])


def shell_command(script: str, prefix: str) -> str:
    """Return one shell command, joining backslash-continued lines."""
    start = script.index(prefix)
    lines = []
    for line in script[start:].splitlines():
        lines.append(line.rstrip("\\ "))
        if not line.rstrip().endswith("\\"):
            break
    return " ".join(part.strip() for part in lines)


def tag_patterns(text: str) -> list[str]:
    tags = re.search(r"(?m)^    tags:\n((?:      - .+\n)+)", text)
    assert tags, "no push tags"
    return [p.strip().strip("'\"") for p in re.findall(r"- (.+)", tags.group(1))]


class ReleaseLspWorkflowContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text(encoding="utf-8")
        cls.jobs = jobs(cls.text)

    def test_triggers_are_the_lsp_tag_and_a_dry_run_dispatch(self) -> None:
        self.assertEqual(tag_patterns(self.text), ["terraphim_lsp-v*"])
        dispatch = self.text[self.text.index("  workflow_dispatch:") :]
        dispatch = dispatch[: dispatch.index("\npermissions:")]
        self.assertRegex(dispatch, r"tag:\n(?:        .+\n)*?        type: string")
        self.assertRegex(
            dispatch,
            r"dry_run:\n(?:        .+\n)*?        default: true\n        type: boolean",
        )

    def test_comprehensive_release_does_not_match_lsp_tags(self) -> None:
        comprehensive = tag_patterns(COMPREHENSIVE.read_text(encoding="utf-8"))
        self.assertIn("v*", comprehensive)
        for tag in ("terraphim_lsp-v1.22.0", "terraphim_lsp-v1.22.0-rc.1"):
            with self.subTest(tag=tag):
                self.assertTrue(fnmatch.fnmatchcase(tag, "terraphim_lsp-v*"))
                for pattern in comprehensive:
                    self.assertFalse(fnmatch.fnmatchcase(tag, pattern), pattern)

    def test_every_cargo_job_runs_on_a_self_hosted_runner(self) -> None:
        cargo_jobs = set()
        for name, job in self.jobs.items():
            if re.search(r"\b(cargo|cross|rustup)\b", job):
                cargo_jobs.add(name)
                self.assertIn("self-hosted", runs_on(job), name)
            else:
                self.assertEqual(runs_on(job), "ubuntu-latest", name)
        self.assertEqual(cargo_jobs, {"build-linux", "build-macos"})
        self.assertEqual(
            runs_on(self.jobs["build-linux"]), "[self-hosted, Linux, X64, bigbox]"
        )
        self.assertEqual(runs_on(self.jobs["build-macos"]), "[self-hosted, macOS]")
        for name in ("build-linux", "build-macos"):
            self.assertIn(
                "CARGO_REGISTRIES_TERRAPHIM_TOKEN: "
                "${{ secrets.CARGO_REGISTRIES_TERRAPHIM_TOKEN }}",
                self.jobs[name],
            )

    def test_linux_lanes_use_a_lane_local_cargo_home_and_target_dir(self) -> None:
        linux = self.jobs["build-linux"]
        lane = step_run(linux, "Use a lane-local Cargo home and target dir")
        self.assertIn('LANE="$RUNNER_TEMP/cargo-lane-$TARGET"', lane)
        self.assertIn('rm -rf "$LANE"', lane)
        self.assertIn('echo "CARGO_HOME=$LANE/home"', lane)
        self.assertIn('echo "CARGO_TARGET_DIR=$LANE/target"', lane)
        self.assertIn('>> "$GITHUB_ENV"', lane)
        self.assertIn('echo "$LANE/home/bin" >> "$GITHUB_PATH"', lane)
        # The lane exists before cross is installed, crates are resolved or
        # anything builds, and every built path comes from the lane target dir.
        order = [
            linux.index("- name: Use a lane-local Cargo home and target dir"),
            linux.index("- name: Install cross"),
            linux.index("- name: Fetch dependencies on the host"),
            linux.index("- name: Build terraphim-lsp\n"),
            linux.index("- name: Build terraphim-lsp (cross)"),
        ]
        self.assertEqual(order, sorted(order))
        self.assertNotIn('"target/$TARGET', linux)
        self.assertIn('BIN="$CARGO_TARGET_DIR/$TARGET/release/terraphim-lsp"', linux)

    def test_pinned_toolchain_never_becomes_the_default(self) -> None:
        channel = re.search(
            r'(?m)^channel = "([^"]+)"', TOOLCHAIN.read_text(encoding="utf-8")
        ).group(1)
        self.assertIn(f"RUST_TOOLCHAIN: '{channel}'", self.text)
        self.assertNotIn("dtolnay/rust-toolchain", self.text)
        self.assertNotRegex(self.text, r"rustup (default|override)")
        for line in self.text.splitlines():
            if re.search(r"\b(cargo|cross) (build|fetch|install)\b", line):
                self.assertIn('rustup run "$RUST_TOOLCHAIN" ', line)

    def test_built_targets_and_raw_asset_names(self) -> None:
        linux = self.jobs["build-linux"]
        self.assertEqual(
            set(re.findall(r"(?m)^          - target: (\S+)$", linux)), LINUX_TARGETS
        )
        self.assertIn(
            'cp "$CARGO_TARGET_DIR/$TARGET/release/terraphim-lsp" '
            '"dist/terraphim-lsp-$TARGET"',
            linux,
        )
        mac = self.jobs["build-macos"]
        build = step_run(mac, "Build terraphim-lsp for both architectures")
        self.assertIn(
            "for target in aarch64-apple-darwin x86_64-apple-darwin; do", build
        )
        self.assertIn('--target "$target" -p terraphim_lsp --bin terraphim-lsp', build)
        self.assertIn("-output dist/terraphim-lsp-universal-apple-darwin", mac)
        self.assertNotIn("pc-windows", self.text)

    def test_signing_never_blocks_the_macos_build(self) -> None:
        mac = self.jobs["build-macos"]
        self.assertIn("continue-on-error: true", step(mac, "Install 1Password CLI"))
        sign = step_run(mac, "Sign and notarise the universal binary (best effort)")
        self.assertIn("! command -v op", sign)
        self.assertNotIn("set -e", sign)
        self.assertNotIn("exit 1", sign)
        self.assertEqual(sign.count('echo "signing=unsigned"'), 3)

    def test_version_is_checked_against_the_built_binary(self) -> None:
        self.assertIn(
            '[[ "$ACTUAL" != "terraphim-lsp $VERSION" ]]', self.jobs["build-linux"]
        )
        self.assertIn(
            '[[ "$ACTUAL" != "terraphim-lsp $VERSION" ]]', self.jobs["build-macos"]
        )

    def test_release_is_a_token_scoped_prerelease_only_when_publishing(self) -> None:
        self.assertRegex(self.text, r"(?m)^permissions:\n  contents: read$")
        self.assertEqual(self.text.count("contents: write"), 1)
        release = self.jobs["release"]
        self.assertIn("contents: write", release)
        self.assertIn("if: needs.resolve.outputs.publish == 'true'", release)
        self.assertIn("GH_TOKEN: ${{ github.token }}", release)
        script = step_run(release, "Create or update the pre-release")
        for command in ("gh release create", "gh release edit"):
            with self.subTest(command=command):
                invocation = shell_command(script, command)
                self.assertIn("--prerelease", invocation)
                self.assertIn('--title "$TAG"', invocation)
        self.assertIn("--verify-tag", shell_command(script, "gh release create"))
        # A personal token would let the publication trigger `release:` workflows.
        self.assertNotRegex(self.text, r"secrets\.(GH_|GITHUB_|.*PAT)")


class ReleaseLspExecutionContract(unittest.TestCase):
    """Runs the workflow's own shell and Python snippets on real files."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.jobs = jobs(WORKFLOW.read_text(encoding="utf-8"))

    def run_checksums(self, names: list[str]) -> subprocess.CompletedProcess[str]:
        script = step_run(
            self.jobs["checksums"], "Check the asset set and write checksums.txt"
        )
        self.work = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.work)
        for index, name in enumerate(names):
            (self.work / name).write_bytes(f"binary {index} {name}\n".encode())
        return subprocess.run(
            ["bash", "-c", script], cwd=self.work, capture_output=True, text=True
        )

    def test_checksums_cover_exactly_the_raw_assets_in_sha256sum_format(self) -> None:
        script = step_run(
            self.jobs["checksums"], "Check the asset set and write checksums.txt"
        )
        self.assertIn("sha256sum -- * > checksums.txt", script)
        result = self.run_checksums(RAW_ASSETS)
        self.assertEqual(result.returncode, 0, result.stderr)
        lines = (self.work / "checksums.txt").read_text().splitlines()
        expected = [
            f"{hashlib.sha256((self.work / n).read_bytes()).hexdigest()}  {n}"
            for n in RAW_ASSETS
        ]
        self.assertEqual(lines, expected)

    def test_checksums_refuse_a_missing_or_extra_asset(self) -> None:
        for names in (
            RAW_ASSETS[1:],
            RAW_ASSETS + ["terraphim-lsp-x86_64-pc-windows-msvc.exe"],
        ):
            with self.subTest(count=len(names)):
                result = self.run_checksums(names)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((self.work / "checksums.txt").exists())

    def run_version_check(
        self, tag: str
    ) -> tuple[subprocess.CompletedProcess[str], str]:
        script = step_run(
            self.jobs["resolve"], "Check tag version against the crate version"
        )
        with tempfile.NamedTemporaryFile("r", suffix=".out") as out:
            env = dict(os.environ, TAG=tag, GITHUB_OUTPUT=out.name)
            result = subprocess.run(
                ["bash", "-c", script],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
            )
            return result, Path(out.name).read_text()

    def crate_version(self) -> str:
        cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
        workspace_package = cargo[cargo.index("[workspace.package]") :]
        return re.search(r'(?m)^version = "([^"]+)"', workspace_package).group(1)

    def test_tag_version_must_match_the_crate_version(self) -> None:
        version = self.crate_version()
        for tag in (f"terraphim_lsp-v{version}", f"terraphim_lsp-v{version}-rc.1"):
            with self.subTest(tag=tag):
                result, output = self.run_version_check(tag)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(output, f"version={version}\n")
        major, minor, patch = version.split(".")
        for tag in (
            f"terraphim_lsp-v{major}.{minor}.{int(patch) + 1}",
            f"terraphim_lsp-v{major}.{minor}.{int(patch) + 1}-rc.1",
        ):
            with self.subTest(tag=tag):
                result, output = self.run_version_check(tag)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(output, "")


if __name__ == "__main__":
    unittest.main()
