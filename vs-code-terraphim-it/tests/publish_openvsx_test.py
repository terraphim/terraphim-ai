"""Direct policy tests: no substitute registry, host, or subprocess implementation."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import publish_openvsx as publish


class OpenVsxPublicationTest(unittest.TestCase):
    manifest = {"publisher": "applied-knowledge-systems", "name": "terraphim-it", "version": "0.1.0"}

    def test_only_exact_missing_extension_allows_bootstrap(self):
        missing = {"error": "Extension not found: applied-knowledge-systems.terraphim-it",
                   "deprecated": False, "downloadable": False}
        publish.require_first_publication(404, missing, self.manifest)
        for status, body in [(200, {"version": "0.0.4"}), (403, missing), (500, missing),
                             (404, {"error": "Namespace not found"}),
                             (404, {"error": "Extension not found: other.name"}), (404, []),
                             (404, {"error": "Extension not found: applied-knowledge-systems.terraphim-it.extra"})]:
            with self.subTest(status=status, body=body), self.assertRaises(ValueError):
                publish.require_first_publication(status, body, self.manifest)

    def test_default_is_oidc_and_removes_inherited_pat_override(self):
        original = {"PATH": "/bin", "OVSX_PAT": "example-old-token", "OVSX_BOOTSTRAP_PAT": "example-new-token"}
        command = publish.publication_command(Path("qualified.vsix"), False)
        child = publish.publication_environment(False, original)
        self.assertEqual(command, ["npm", "exec", "--", "ovsx", "publish", str(Path("qualified.vsix").resolve()),
                                   "--pre-release", "--trusted-publishing"])
        self.assertEqual(child, {"PATH": "/bin"})
        self.assertIn("OVSX_PAT", original)

    def test_explicit_bootstrap_uses_only_scoped_secret_without_cli_exposure(self):
        command = publish.publication_command(Path("qualified.vsix"), True)
        child = publish.publication_environment(True,
                {"PATH": "/bin", "OVSX_PAT": "unrelated-token", "OVSX_BOOTSTRAP_PAT": "example-bootstrap-token"})
        self.assertNotIn("--trusted-publishing", command)
        self.assertNotIn("example-bootstrap-token", " ".join(command))
        self.assertEqual(child, {"PATH": "/bin", "OVSX_PAT": "example-bootstrap-token"})

    def test_bootstrap_has_no_fallback_to_ambient_pat_or_oidc(self):
        for environment in [{}, {"OVSX_PAT": "unrelated-token"}, {"OVSX_BOOTSTRAP_PAT": "  "}]:
            with self.subTest(environment=environment), self.assertRaisesRegex(ValueError, "OVSX_BOOTSTRAP_PAT"):
                publish.publication_environment(True, environment)


if __name__ == "__main__":
    unittest.main()
