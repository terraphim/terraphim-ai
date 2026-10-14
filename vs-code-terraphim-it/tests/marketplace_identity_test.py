"""Pure identity fixtures and real local child processes; no cloud authentication."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import marketplace_identity as identity

CLIENT = "11111111-2222-3333-4444-555555555555"
TENANT = "22222222-3333-4444-5555-666666666666"
PROFILE = "33333333-4444-5555-6666-777777777777"
OTHER = "44444444-5555-6666-7777-888888888888"
CANARY = "PRIVATE_OUTPUT_MUST_NOT_ESCAPE"


class MarketplaceIdentityTests(unittest.TestCase):
    def environment(self):
        version = json.loads((ROOT / "package.json").read_text())["version"]
        return {"GITHUB_REPOSITORY": "terraphim/terraphim-ai", "GITHUB_REF_TYPE": "tag",
                "GITHUB_REF_NAME": "terraphim_vscode-v" + version, "GITHUB_SHA": "a" * 40,
                "VSCE_AZURE_CLIENT_ID": CLIENT, "VSCE_AZURE_TENANT_ID": TENANT}

    def test_only_exact_release_context_and_well_formed_configuration_admitted(self):
        env = self.environment()
        version = json.loads((ROOT / "package.json").read_text())["version"]
        self.assertEqual(identity.configuration(env, version), (CLIENT, TENANT, None))
        for key, value in (("GITHUB_REPOSITORY", "elsewhere/repo"), ("GITHUB_REF_TYPE", "branch"),
                           ("GITHUB_REF_NAME", "main"), ("GITHUB_SHA", "b" * 39),
                           ("VSCE_AZURE_CLIENT_ID", ""), ("VSCE_AZURE_TENANT_ID", CANARY),
                           ("VSCE_MARKETPLACE_PROFILE_ID", " ")):
            with self.subTest(key=key), self.assertRaises(identity.Refusal) as error:
                identity.configuration({**env, key: value}, version)
            self.assertNotIn(CANARY, str(error.exception))

    def test_identifier_refuses_null_zero_whitespace_and_control_characters(self):
        for value in (None, [], {}, 3, "", "0" * 32, "00000000-0000-0000-0000-000000000000",
                      " " + CLIENT, CLIENT + "\n", CLIENT + "?token=" + CANARY):
            with self.subTest(value=type(value).__name__), self.assertRaises(identity.Refusal):
                identity.identifier(value)
        self.assertEqual(identity.identifier("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"),
                         "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")

    def test_pure_account_fixture_must_match_service_principal_tenant_and_client(self):
        account = {"tenantId": TENANT, "userName": CLIENT, "userType": "servicePrincipal"}
        identity.verify_account(account, CLIENT, TENANT)
        for changed in (None, [], {}, {**account, "userType": "user"},
                        {**account, "userName": OTHER}, {**account, "tenantId": OTHER},
                        {**account, "userName": None}, {**account, "accessToken": CANARY}):
            with self.subTest(changed=type(changed).__name__), self.assertRaises(identity.Refusal) as error:
                identity.verify_account(changed, CLIENT, TENANT)
            self.assertNotIn(CANARY, str(error.exception))

    def test_profile_fixture_is_not_ready_without_explicit_matching_expected_id(self):
        actual = identity.profile_id({"id": PROFILE})
        self.assertEqual(actual, PROFILE)
        with self.assertRaisesRegex(identity.Refusal, "MEMBERSHIP_CONFIRMATION_REQUIRED"):
            identity.confirm_profile(actual, None)
        with self.assertRaisesRegex(identity.Refusal, "PROFILE_MISMATCH"):
            identity.confirm_profile(actual, OTHER)
        identity.confirm_profile(actual, PROFILE)
        for value in (None, [], {}, {"id": None}, {"id": PROFILE, "token": CANARY}):
            with self.subTest(value=type(value).__name__), self.assertRaises(identity.Refusal):
                identity.profile_id(value)

    def test_metadata_parser_refuses_duplicate_keys_types_and_oversized_output(self):
        for data in (b'{"id":"first","id":"second"}', b'[]', b'null', b'not-json',
                     b'\xff', b' ' * 65537, CANARY):
            with self.subTest(dataType=type(data).__name__), self.assertRaises(identity.Refusal) as error:
                identity.parse_metadata(data)
            self.assertEqual(str(error.exception), "INVALID_METADATA")

    def test_real_child_process_json_transport_does_not_expose_stderr(self):
        command = [sys.executable, "-c", "import sys; print('{\"transport\":true}'); "
                   "print('" + CANARY + "',file=sys.stderr)"]
        self.assertEqual(identity.run_metadata(command), {"transport": True})

    def test_real_child_process_failures_timeout_and_invalid_json_are_sanitized(self):
        cases = [([sys.executable, "-c", "import sys; print('" + CANARY + "'); "
                   "print('" + CANARY + "',file=sys.stderr); sys.exit(7)"], 5, "IDENTITY_QUERY_FAILED"),
                 ([sys.executable, "-c", "import time; time.sleep(5)"], .02, "IDENTITY_QUERY_TIMEOUT"),
                 ([sys.executable, "-c", "print('" + CANARY + "')"], 5, "INVALID_METADATA"),
                 (["/nonexistent/metadata-reader"], 5, "IDENTITY_QUERY_UNAVAILABLE")]
        for command, timeout, expected in cases:
            with self.subTest(expected=expected), self.assertRaises(identity.Refusal) as error:
                identity.run_metadata(command, timeout)
            self.assertEqual(str(error.exception), expected)

    def test_invalid_configuration_cli_writes_only_fixed_failure_without_azure(self):
        with tempfile.TemporaryDirectory() as directory:
            receipt = Path(directory) / "identity.json"
            result = subprocess.run([sys.executable, str(ROOT / "scripts/marketplace_identity.py"),
                                     "--receipt", str(receipt)], capture_output=True, text=True, timeout=10,
                                    env={**os.environ, **self.environment(), "VSCE_AZURE_CLIENT_ID": CANARY})
            self.assertEqual(result.returncode, 1)
            report = json.loads(receipt.read_text())
            self.assertEqual(report["status"], "NOT_READY")
            self.assertEqual(report["errorCode"], "INVALID_IDENTITY_IDENTIFIER")
            self.assertNotIn(CANARY, result.stdout + result.stderr + receipt.read_text())
            self.assertNotIn("profileId", report)

    def test_existing_and_symlink_receipts_are_not_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            old = root / "old.json"; old.write_text("original receipt")
            link = root / "link.json"; link.symlink_to(root / "absent.json")
            for receipt in (old, link):
                result = subprocess.run([sys.executable, str(ROOT / "scripts/marketplace_identity.py"),
                                         "--receipt", str(receipt)], capture_output=True, text=True, timeout=10,
                                        env={**os.environ, **self.environment()})
                self.assertEqual(result.returncode, 1)
                self.assertIn("RECEIPT_UNAVAILABLE", result.stderr)
            self.assertEqual(old.read_text(), "original receipt")
            self.assertTrue(link.is_symlink())
            self.assertFalse((root / "absent.json").exists())

    def test_fixed_commands_request_only_expected_identity_metadata(self):
        self.assertEqual(identity.ACCOUNT_COMMAND[:3], ["az", "account", "show"])
        self.assertEqual(identity.PROFILE_COMMAND[:4], ["az", "rest", "--method", "get"])
        self.assertIn(identity.PROFILE_URL, identity.PROFILE_COMMAND)
        self.assertIn("{id:id}", identity.PROFILE_COMMAND)
        self.assertIn("499b84ac-1321-427f-aa17-267ca6975798", identity.PROFILE_COMMAND)
        for command in (identity.ACCOUNT_COMMAND, identity.PROFILE_COMMAND):
            self.assertIn("--only-show-errors", command)
            for forbidden in ("get-access-token", "--debug", "--verbose", "--headers", "--password"):
                self.assertNotIn(forbidden, command)

    def test_workflow_gate_runs_after_login_before_publish_and_retains_separate_receipt(self):
        workflow = (ROOT.parent / ".github/workflows/vscode-release.yml").read_text()
        login = workflow.index("uses: azure/login@")
        gate = workflow.index("run: python3 scripts/marketplace_identity.py")
        publish = workflow.index("run: npm exec -- vsce publish")
        self.assertLess(login, gate)
        self.assertLess(gate, publish)
        block = workflow[workflow.rfind("      - name:", 0, login):workflow.rfind("      - name:", 0, publish)]
        self.assertNotIn("continue-on-error: true", block)
        self.assertIn("vars.VSCE_MARKETPLACE_PROFILE_ID", block)
        self.assertIn("path: vs-code-terraphim-it/marketplace-identity.json", workflow)
        self.assertIn("vscode-marketplace-identity-${{ github.sha }}-${{ github.run_attempt }}", workflow)
        self.assertNotIn("VSCE_PAT", workflow)


if __name__ == "__main__":
    unittest.main()
