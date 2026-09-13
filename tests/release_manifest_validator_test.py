#!/usr/bin/env python3
"""Contract tests for the canonical release-manifest validator."""

from __future__ import annotations

import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "scripts/validate-release-manifest.py"
SCHEMA = ROOT / ".release/release-manifest.schema.json"
FIXTURES = ROOT / "tests/fixtures/release-manifest"


def run_validator(path: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(VALIDATOR), "--schema", str(SCHEMA), str(path)],
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=5,
    )


def load_fixture(name: str = "valid-v1.json") -> dict[str, Any]:
    return json.JSONDecoder().decode((FIXTURES / name).read_text(encoding="utf-8"))


def valid_manifest() -> dict[str, Any]:
    return copy.deepcopy(load_fixture())


def run_manifest(data: dict[str, Any]) -> subprocess.CompletedProcess[str]:
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "release-manifest.json"
        path.write_text(json.dumps(data, sort_keys=True, indent=2), encoding="utf-8")
        return run_validator(path)


class ReleaseManifestValidatorTest(unittest.TestCase):
    def assert_accepted(self, data: dict[str, Any]) -> None:
        result = run_manifest(data)
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)

    def assert_rejected(self, data: dict[str, Any], expected: str) -> None:
        result = run_manifest(data)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(expected, result.stderr + result.stdout)

    def test_schema_self_check_passes_with_canonical_id(self) -> None:
        schema = load_fixture_schema()
        self.assertEqual(schema["$id"], "https://terraphim.ai/schemas/release-manifest-v1.json")

        for flag in ("--self-check", "--self-check-schema"):
            result = subprocess.run(
                [sys.executable, str(VALIDATOR), "--schema", str(SCHEMA), flag],
                cwd=ROOT,
                check=False,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=5,
            )
            self.assertEqual(result.returncode, 0, result.stderr + result.stdout)

    def test_valid_fixtures_pass(self) -> None:
        for name in ("valid-v1.json", "valid-homebrew-rb-empty-signature.json"):
            with self.subTest(name=name):
                result = run_validator(FIXTURES / name)
                self.assertEqual(result.returncode, 0, result.stderr + result.stdout)

    def test_invalid_fixtures_fail(self) -> None:
        invalid_names = sorted(path.name for path in FIXTURES.glob("invalid-*.json"))
        self.assertGreaterEqual(len(invalid_names), 6)

        for name in invalid_names:
            with self.subTest(name=name):
                result = run_validator(FIXTURES / name)
                self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_valid_fixture_uses_canonical_contract_shape(self) -> None:
        manifest = valid_manifest()

        self.assertEqual(
            list(manifest),
            [
                "schema_version",
                "release_version",
                "release_tag",
                "sources",
                "assets",
                "required_channels",
                "central_channels",
                "downstream_channels",
                "created_at",
            ],
        )
        self.assertEqual(manifest["schema_version"], "1.0.0")
        self.assertEqual(manifest["release_tag"], f"v{manifest['release_version']}")
        self.assertEqual(
            set(manifest["required_channels"]),
            {
                "github_release",
                "r2_stable_manifest",
                "homebrew_tap_pr",
                "aur_terraphim_clients_bin",
                "omarchy_terraphim_clients_bin",
            },
        )
        self.assertEqual(set(manifest["central_channels"]), {"github_release", "r2_stable_manifest"})
        self.assertEqual(
            set(manifest["downstream_channels"]),
            {
                "homebrew_tap_pr",
                "aur_terraphim_clients_bin",
                "omarchy_terraphim_clients_bin",
            },
        )

        for asset in manifest["assets"]:
            self.assertEqual(
                set(asset),
                {
                    "name",
                    "component",
                    "format",
                    "target",
                    "arch",
                    "os",
                    "source_repo",
                    "source_sha",
                    "sha256",
                    "size_bytes",
                    "signature",
                },
            )

    def test_channel_sets_are_unordered_exact_sets(self) -> None:
        manifest = valid_manifest()
        manifest["required_channels"] = [
            "omarchy_terraphim_clients_bin",
            "aur_terraphim_clients_bin",
            "homebrew_tap_pr",
            "r2_stable_manifest",
            "github_release",
        ]
        manifest["central_channels"] = ["r2_stable_manifest", "github_release"]
        manifest["downstream_channels"] = [
            "omarchy_terraphim_clients_bin",
            "aur_terraphim_clients_bin",
            "homebrew_tap_pr",
        ]
        self.assert_accepted(manifest)

        manifest = valid_manifest()
        manifest["required_channels"][4] = "github_release"
        self.assert_rejected(manifest, "non-unique")

        manifest = valid_manifest()
        manifest["downstream_channels"].pop()
        self.assert_rejected(manifest, "too short")

        manifest = valid_manifest()
        manifest["central_channels"].append("homebrew_tap_pr")
        self.assert_rejected(manifest, "too long")

        manifest = valid_manifest()
        manifest["required_channels"][0] = "unknown_channel"
        self.assert_rejected(manifest, "unknown_channel")

    def test_optional_true_tag_peel_fields_are_accepted_and_false_is_rejected(self) -> None:
        manifest = valid_manifest()
        for source in manifest["sources"].values():
            source["tag_peel_matches_gitea"] = True
            source["tag_peel_matches_github"] = True
        self.assert_accepted(manifest)

        manifest = valid_manifest()
        manifest["sources"]["terraphim-ai"]["tag_peel_matches_gitea"] = False
        self.assert_rejected(manifest, "tag_peel_matches_gitea")

        manifest = valid_manifest()
        manifest["sources"]["terraphim-clients"]["tag_peel_matches_github"] = False
        self.assert_rejected(manifest, "tag_peel_matches_github")

    def test_prerelease_and_build_semver_is_accepted(self) -> None:
        manifest = valid_manifest()
        manifest["release_version"] = "1.2.3-rc.1+build.7"
        manifest["release_tag"] = "v1.2.3-rc.1+build.7"
        self.assert_accepted(manifest)

        manifest = valid_manifest()
        manifest["release_version"] = "v1.2.3"
        self.assert_rejected(manifest, "release_version")

        manifest = valid_manifest()
        manifest["release_version"] = "1.2.3-01"
        manifest["release_tag"] = "v1.2.3-01"
        self.assert_rejected(manifest, "does not match")

        manifest = valid_manifest()
        manifest["release_tag"] = "v1.2.3+other"
        self.assert_rejected(manifest, "release_tag")

    def test_signature_rules_are_format_sensitive(self) -> None:
        for asset_format in ("tar.gz", "deb", "rpm", "pkg.tar.zst", "exe", "dmg", "zip"):
            with self.subTest(format=asset_format):
                manifest = valid_manifest()
                manifest["assets"][0]["format"] = asset_format
                manifest["assets"][0]["signature"] = ""
                self.assert_rejected(manifest, "non-rb assets require a signature")

        manifest = load_fixture("valid-homebrew-rb-empty-signature.json")
        self.assert_accepted(manifest)

    def test_duplicate_json_object_keys_are_rejected_at_any_depth(self) -> None:
        top_level = run_validator(FIXTURES / "invalid-duplicate-keys.json")
        self.assertNotEqual(top_level.returncode, 0)
        self.assertIn("duplicate JSON object key 'schema_version'", top_level.stderr + top_level.stdout)

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "nested-duplicate.json"
            path.write_text(
                """
{
  "schema_version": "1.0.0",
  "release_version": "1.2.3",
  "release_tag": "v1.2.3",
  "sources": {
    "terraphim-ai": {
      "gitea_sha": "1111111111111111111111111111111111111111",
      "gitea_sha": "1111111111111111111111111111111111111111",
      "github_sha": "1111111111111111111111111111111111111111",
      "tree_sha": "2222222222222222222222222222222222222222",
      "workflow_run_id": 33380001
    },
    "terraphim-clients": {
      "gitea_sha": "3333333333333333333333333333333333333333",
      "github_sha": "3333333333333333333333333333333333333333",
      "tree_sha": "4444444444444444444444444444444444444444",
      "workflow_run_id": 33360001
    }
  },
  "assets": [],
  "required_channels": [],
  "central_channels": [],
  "downstream_channels": [],
  "created_at": "2026-09-12T10:00:00Z"
}
""",
                encoding="utf-8",
            )
            nested = run_validator(path)
        self.assertNotEqual(nested.returncode, 0)
        self.assertIn("duplicate JSON object key 'gitea_sha'", nested.stderr + nested.stdout)

    def test_required_source_fields_and_hashes_are_enforced(self) -> None:
        manifest = valid_manifest()
        manifest["sources"]["terraphim-ai"]["workflow_run_id"] = 0
        self.assert_rejected(manifest, "workflow_run_id")

        manifest = valid_manifest()
        manifest["sources"]["terraphim-clients"]["tree_sha"] = "not-a-sha"
        self.assert_rejected(manifest, "tree_sha")

        manifest = valid_manifest()
        manifest["sources"]["terraphim-ai"]["github_sha"] = "5555555555555555555555555555555555555555"
        self.assert_rejected(manifest, "Gitea SHA must match GitHub SHA")

    def test_source_member_ownership_and_asset_sha_are_enforced(self) -> None:
        manifest = valid_manifest()
        manifest["sources"]["terraphim_server"] = manifest["sources"].pop("terraphim-ai")
        self.assert_rejected(manifest, "terraphim_server")

        manifest = valid_manifest()
        manifest["assets"][0]["source_repo"] = "terraphim-clients"
        manifest["assets"][0]["source_sha"] = manifest["sources"]["terraphim-clients"]["gitea_sha"]
        self.assert_rejected(manifest, "does not own component")

        manifest = valid_manifest()
        manifest["assets"][0]["source_sha"] = "5555555555555555555555555555555555555555"
        self.assert_rejected(manifest, "source SHA mismatch")

    def test_asset_field_drift_is_rejected_without_inventory_matrix_validation(self) -> None:
        manifest = valid_manifest()
        manifest["assets"][0]["format"] = "bin"
        self.assert_rejected(manifest, "bin")

        manifest = valid_manifest()
        manifest["assets"][0]["name"] = ""
        self.assert_rejected(manifest, "name")

        manifest = valid_manifest()
        manifest["assets"][0]["size"] = manifest["assets"][0].pop("size_bytes")
        self.assert_rejected(manifest, "size")

        manifest = valid_manifest()
        manifest["assets"][0]["assets"] = []
        self.assert_rejected(manifest, "assets[0]")

    def test_legacy_products_shape_is_rejected(self) -> None:
        result = run_validator(FIXTURES / "invalid-legacy-products.json")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("products", result.stderr + result.stdout)


def load_fixture_schema() -> dict[str, Any]:
    return json.JSONDecoder().decode(SCHEMA.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
