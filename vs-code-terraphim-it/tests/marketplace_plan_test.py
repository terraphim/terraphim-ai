"""Execute the real publication planner with isolated files and no network."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/plan_marketplaces.py"
FLAGS = ("VSCODE_PUBLISH_VISUAL_STUDIO_MARKETPLACE", "VSCODE_PUBLISH_OPEN_VSX")


class MarketplacePlanTests(unittest.TestCase):
    def run_plan(self, flags):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            environment = {key: value for key, value in os.environ.items() if key not in FLAGS}
            environment.update({"GITHUB_SHA": "a" * 40, "GITHUB_REF_NAME": "terraphim_vscode-v0.1.1",
                                "GITHUB_OUTPUT": str(folder / "outputs"),
                                "GITHUB_STEP_SUMMARY": str(folder / "summary"), **flags})
            result = subprocess.run([sys.executable, str(SCRIPT), "--receipt", str(folder / "receipt.json")],
                                    env=environment, capture_output=True, text=True)
            return (result, json.loads((folder / "receipt.json").read_text()),
                    (folder / "outputs").read_text(), (folder / "summary").read_text())

    def test_absent_or_false_flags_skip_both_with_explicit_not_published_receipt(self):
        for flags in ({}, {FLAGS[0]: "false", FLAGS[1]: ""}):
            with self.subTest(flags=flags):
                result, receipt, outputs, summary = self.run_plan(flags)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(receipt["registries"], [])
                self.assertEqual(outputs, "registries=[]\n")
                self.assertTrue(all(target["status"] == "NOT_PUBLISHED" for target in receipt["targets"]))
                self.assertEqual(summary.count("NOT_PUBLISHED"), 2)

    def test_each_exact_true_selects_only_its_fixed_registry(self):
        for index, expected in enumerate(("visual-studio-marketplace", "open-vsx")):
            with self.subTest(expected=expected):
                result, receipt, outputs, _ = self.run_plan({FLAGS[index]: "true"})
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(receipt["registries"], [expected])
                self.assertEqual(json.loads(outputs.removeprefix("registries=")), [expected])
                self.assertEqual(receipt["targets"][index]["status"], "PENDING_PUBLICATION")
                self.assertEqual(receipt["targets"][1-index]["status"], "NOT_PUBLISHED")

    def test_both_enabled_remain_pending_until_separate_live_readback(self):
        result, receipt, _, summary = self.run_plan(dict.fromkeys(FLAGS, "true"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(receipt["registries"], ["visual-studio-marketplace", "open-vsx"])
        self.assertTrue(all(target["status"] == "PENDING_PUBLICATION" for target in receipt["targets"]))
        self.assertNotIn("PUBLISHED_PUBLIC_VERIFIED", summary)

    def test_malformed_flag_refuses_entire_route_without_echoing_value(self):
        for value in ("TRUE", "yes", "1", "true\nregistries=foreign"):
            with self.subTest(value=value):
                result, receipt, outputs, summary = self.run_plan({FLAGS[0]: "true", FLAGS[1]: value})
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(receipt["registries"], [])
                self.assertEqual(outputs, "registries=[]\n")
                self.assertTrue(all(target["status"] == "NOT_PUBLISHED" for target in receipt["targets"]))
                self.assertNotIn(value, result.stdout + result.stderr + summary)

    def test_existing_bootstrap_switch_cannot_enable_marketplace_routing(self):
        result, receipt, _, _ = self.run_plan({"OVSX_BOOTSTRAP_FIRST_PUBLISH": "true",
                                              "OVSX_BOOTSTRAP_PAT": "not-a-real-secret"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(receipt["registries"], [])

    def test_wrong_tag_or_source_refuses_enabled_targets(self):
        for identity in ({"GITHUB_REF_NAME": "main"},
                         {"GITHUB_REF_NAME": "terraphim_vscode-v0.1.0"},
                         {"GITHUB_SHA": "not-a-commit"}):
            with self.subTest(identity=identity):
                result, receipt, outputs, _ = self.run_plan({**dict.fromkeys(FLAGS, "true"), **identity})
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(outputs, "registries=[]\n")
                self.assertTrue(all(target["status"] == "NOT_PUBLISHED" for target in receipt["targets"]))

    def test_existing_receipt_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            receipt = Path(directory) / "receipt.json"
            receipt.write_text("earlier publication evidence\n")
            result = subprocess.run([sys.executable, str(SCRIPT), "--receipt", str(receipt)],
                                    env={**os.environ, "GITHUB_SHA": "a" * 40,
                                         "GITHUB_REF_NAME": "terraphim_vscode-v0.1.1"},
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(receipt.read_text(), "earlier publication evidence\n")

    def test_workflow_keeps_github_release_independent_and_routes_only_planned_targets(self):
        sys.path.insert(0, str(ROOT.parent / "tests"))
        from workflow_release_recovery_contract_test import has_need, job_block
        workflow = (ROOT.parent / ".github/workflows/vscode-release.yml").read_text()
        release = job_block(workflow, "release")
        plan = job_block(workflow, "marketplace-plan")
        marketplace = job_block(workflow, "marketplace")
        self.assertTrue(has_need(release, "qualify"))
        self.assertNotIn("marketplace", release)
        self.assertTrue(has_need(plan, "release"))
        self.assertNotIn("id-token: write", plan)
        self.assertNotIn("secrets.", plan)
        self.assertTrue(has_need(marketplace, "release"))
        self.assertTrue(has_need(marketplace, "marketplace-plan"))
        self.assertIn("needs.marketplace-plan.outputs.registries != '[]'", marketplace)
        self.assertIn("fromJSON(needs.marketplace-plan.outputs.registries)", marketplace)
        self.assertIn("--azure-credential", marketplace)
        self.assertIn("verify_registry.py", marketplace)
        for flag in FLAGS:
            self.assertIn("vars." + flag, plan)


if __name__ == "__main__":
    unittest.main()
