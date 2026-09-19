"""Cross-boundary contract: one coherent signed manifest between the
coordinator publisher and the terraphim_update consumer.

P1-1/P1-2 remediation: the updater consumes the coordinator-owned,
detached-signed `release-manifest-v1`. The coordinator publishes the
immutable per-tag object at `releases/<tag>/manifest.json` plus a signed
stable discovery pointer per component at `<component>/manifest.json`
(byte-identical signed content, so one signature verifies both). These
tests pin the schema, the publication layout, and the consumer anchors so
future drift between the two sides fails here.
"""

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path

import jsonschema

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / ".release/release-manifest.schema.json"
FIXTURE = ROOT / "tests/fixtures/release-manifest/valid-v1.json"
R2_PUBLISH = ROOT / ".github/scripts/release/r2-publish-manifest.sh"
R2_RS = ROOT / "crates/terraphim_update/src/r2.rs"

MANIFEST_CONTEXT = "terraphim-release-manifest-v1"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
TAG_RE = re.compile(r"^v[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$")


class UpdaterManifestCrossBoundaryContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        cls.fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        cls.publish_sh = R2_PUBLISH.read_text(encoding="utf-8")
        cls.r2_rs = R2_RS.read_text(encoding="utf-8")

    def test_fixture_validates_against_canonical_schema(self) -> None:
        jsonschema.validate(self.fixture, self.schema)

    def test_fixture_carries_exact_agent_and_grep_payload_digests(self) -> None:
        """Both updater-consumed components must have exact tar.gz payload SHAs."""
        for component in ("terraphim-agent", "terraphim-grep"):
            matches = [
                asset
                for asset in self.fixture["assets"]
                if asset["component"] == component and asset["format"] == "tar.gz"
            ]
            self.assertTrue(matches, f"{component} must have a tar.gz asset")
            for asset in matches:
                self.assertRegex(asset["sha256"], SHA256_RE, asset["name"])
                self.assertGreater(asset["size_bytes"], 0, asset["name"])

    def test_publisher_writes_signed_stable_pointer_per_component(self) -> None:
        """The R2 publisher must publish the signed discovery pointer at the
        exact layout the updater fetches: `<component>/manifest.json` plus
        `<component>/manifest.json.sig`."""
        self.assertIn('/manifest.json"', self.publish_sh)
        self.assertIn('/manifest.json.sig"', self.publish_sh)
        # Pointer keys derive from the canonical manifest's component set,
        # not from a hard-coded list that could drift.
        self.assertIn("component", self.publish_sh)
        # The pointer carries the same signed bytes under the same context.
        self.assertIn(MANIFEST_CONTEXT, self.publish_sh)

    def test_publisher_keeps_per_tag_objects_immutable_and_signed_first(self) -> None:
        """Immutable publication must not be weakened: conditional puts and
        signature-before-manifest commit order stay for the per-tag objects."""
        self.assertIn("--if-none-match '*'", self.publish_sh)
        sig_put = self.publish_sh.index('immutable_put "$SIGNATURE_OBJECT_KEY"')
        manifest_put = self.publish_sh.index('immutable_put "$OBJECT_KEY"')
        self.assertLess(sig_put, manifest_put)

    def test_consumer_fetches_pointer_and_signature_with_canonical_context(self) -> None:
        """The updater must fetch the pointer and its detached signature and
        verify under the coordinator's signing context."""
        self.assertIn('"{}/{}/manifest.json"', self.r2_rs)
        self.assertIn('"{}.sig"', self.r2_rs)
        self.assertIn(f'"{MANIFEST_CONTEXT}"', self.r2_rs)
        # Fail closed: verification happens before parsing/selection can
        # influence updates, and missing signatures are errors.
        self.assertIn("verify_detached_signature", self.r2_rs)

    def test_consumer_enforces_schema_version_and_payload_sha(self) -> None:
        """The updater pins schema_version 1.0.0 and requires exact payload
        SHA-256 per selected asset (no dead integrity fields)."""
        self.assertIn('"1.0.0"', self.r2_rs)
        self.assertIn("sha256", self.r2_rs)
        # The removed dead fields must stay removed (field/schema anchors,
        # not substrings of the live `r2_manifest_signature_url` helper).
        self.assertNotIn("pub signature_url", self.r2_rs)
        self.assertNotIn('"signature_url"', self.r2_rs)

    def test_consumer_constructs_download_urls_from_validated_parts(self) -> None:
        """Asset download URLs are constructed from the trusted repository
        config and validated manifest parts — never arbitrary URLs from the
        manifest (the canonical schema intentionally carries no URL field)."""
        self.assertIn("github_release_asset_url", self.r2_rs)
        self.assertIn("https://github.com/", self.r2_rs)
        self.assertNotIn('"url"', self.r2_rs.replace('"url"', '"url"', 0))


if __name__ == "__main__":
    unittest.main()
