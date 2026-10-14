"""Direct OIDC policy tests; no substitute registry or subprocess implementation."""
import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import publish_openvsx as publish


class OpenVsxPublicationTest(unittest.TestCase):
    def test_only_oidc_and_removes_inherited_pat_overrides(self):
        original = {"PATH": "/bin", "OVSX_PAT": "example-old-token", "OVSX_BOOTSTRAP_PAT": "example-new-token"}
        self.assertEqual(publish.publication_command(Path("qualified.vsix")),
                         ["npm", "exec", "--", "ovsx", "publish", str(Path("qualified.vsix").resolve()),
                          "--pre-release", "--trusted-publishing"])
        self.assertEqual(publish.publication_environment(original), {"PATH": "/bin"})
        self.assertIn("OVSX_PAT", original)

    def test_bootstrap_cli_option_is_refused_before_network(self):
        result = subprocess.run([sys.executable, publish.__file__, "--bootstrap", "--vsix", "absent",
                                 "--manifest", "absent"], capture_output=True, text=True, timeout=10, check=False)
        self.assertEqual(result.returncode, 2)
        self.assertIn("unrecognized arguments: --bootstrap", result.stderr)


if __name__ == "__main__":
    unittest.main()
