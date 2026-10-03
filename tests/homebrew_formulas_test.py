"""Contract tests for the Homebrew downstream outbox (Gitea #3381, MP5a).

Covers deterministic rendering of the tap formulas from the canonical release
manifest, the fail-closed asset checks required before any PR is opened, and
the idempotence of the dry-run outbox.
"""

import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GENERATOR = ROOT / "scripts" / "generate-homebrew-formulas.py"
OUTBOX = ROOT / "scripts" / "homebrew-outbox.sh"
FIXTURE = ROOT / "tests" / "fixtures" / "homebrew" / "release-manifest-v1.21.16.json"
EXPECTED = ROOT / "tests" / "fixtures" / "homebrew" / "expected"
FORMULAS = ("terraphim-agent.rb", "terraphim-grep.rb")


def load_module():
    spec = importlib.util.spec_from_file_location("generate_homebrew_formulas", GENERATOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


GEN = load_module()


def manifest():
    return json.loads(FIXTURE.read_text())


class HomebrewFormulaTests(unittest.TestCase):
    def test_renders_the_tap_bytes_for_v1_21_16(self):
        with tempfile.TemporaryDirectory() as tmp:
            GEN.generate(manifest(), Path(tmp))
            for name in FORMULAS:
                self.assertEqual(
                    (Path(tmp) / name).read_text(),
                    (EXPECTED / name).read_text(),
                    name,
                )

    def test_generation_is_byte_idempotent(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
            GEN.generate(manifest(), Path(a))
            GEN.generate(manifest(), Path(b))
            for name in FORMULAS:
                self.assertEqual((Path(a) / name).read_bytes(), (Path(b) / name).read_bytes())

    def test_stale_detection(self):
        with tempfile.TemporaryDirectory() as tmp:
            GEN.generate(manifest(), Path(tmp))
            self.assertEqual(GEN.stale_formulas(manifest(), Path(tmp)), [])
            target = Path(tmp) / "terraphim-agent.rb"
            target.write_text(target.read_text() + "\n")
            self.assertEqual(
                [path.name for path in GEN.stale_formulas(manifest(), Path(tmp))],
                ["terraphim-agent.rb"],
            )

    def test_components_without_a_spec_are_ignored(self):
        with tempfile.TemporaryDirectory() as tmp:
            written = GEN.generate(manifest(), Path(tmp))
            self.assertEqual(sorted(path.name for path in written), sorted(FORMULAS))

    def _reject(self, mutate):
        data = manifest()
        mutate(data)
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(GEN.FormulaError):
                GEN.generate(data, Path(tmp))

    def test_missing_asset_is_rejected(self):
        def mutate(data):
            data["assets"] = [
                asset
                for asset in data["assets"]
                if asset["name"] != "terraphim-agent-1.21.16-universal-apple-darwin.tar.gz"
            ]

        self._reject(mutate)

    def test_duplicate_asset_name_is_rejected(self):
        def mutate(data):
            data["assets"].append(dict(data["assets"][2]))

        self._reject(mutate)

    def test_zero_size_asset_is_rejected(self):
        def mutate(data):
            for asset in data["assets"]:
                if asset["name"] == "terraphim-grep-1.21.16-x86_64-unknown-linux-gnu.tar.gz":
                    asset["size_bytes"] = 0

        self._reject(mutate)

    def test_malformed_sha_is_rejected(self):
        def mutate(data):
            for asset in data["assets"]:
                if asset["name"] == "terraphim-agent-1.21.16-aarch64-unknown-linux-musl.tar.gz":
                    asset["sha256"] = "not-a-sha"

        self._reject(mutate)

    def test_mismatched_platform_is_rejected(self):
        def mutate(data):
            for asset in data["assets"]:
                if asset["name"] == "terraphim-agent-1.21.16-universal-apple-darwin.tar.gz":
                    asset["arch"] = "x86_64"

        self._reject(mutate)

    def test_mismatched_release_tag_is_rejected(self):
        def mutate(data):
            data["release_tag"] = "v9.9.9"

        self._reject(mutate)


class HomebrewOutboxTests(unittest.TestCase):
    def test_dry_run_is_idempotent(self):
        with tempfile.TemporaryDirectory() as tmp:
            tap = Path(tmp) / "tap"
            (tap / "Formula").mkdir(parents=True)
            git = ["git", "-c", "user.email=t@example.invalid", "-c", "user.name=t"]
            subprocess.run(["git", "init", "-q"], cwd=tap, check=True)
            subprocess.run([*git, "add", "-A"], cwd=tap, check=True)
            subprocess.run([*git, "commit", "--allow-empty", "-qm", "init"], cwd=tap, check=True)

            def run_outbox():
                return subprocess.run(
                    ["bash", str(OUTBOX), "--manifest", str(FIXTURE), "--tap-dir", str(tap)],
                    capture_output=True,
                    text=True,
                    check=True,
                    env=dict(os.environ, HOME=tmp),
                )

            first = run_outbox()
            self.assertIn("formulas changed", first.stdout)

            subprocess.run([*git, "add", "-A"], cwd=tap, check=True)
            subprocess.run([*git, "commit", "-qm", "formulas"], cwd=tap, check=True)

            second = run_outbox()
            self.assertIn("no-op", second.stdout)
            for name in FORMULAS:
                self.assertEqual(
                    (tap / "Formula" / name).read_text(),
                    (EXPECTED / name).read_text(),
                    name,
                )


if __name__ == "__main__":
    unittest.main()
