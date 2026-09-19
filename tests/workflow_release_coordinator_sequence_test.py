"""Executable literal-sequence test for .github/workflows/release-coordinator.yml.

Loads the real workflow YAML, extracts the exact `run:` shell text (plus
each step's declared `env:` block) for a chosen sequence of step ids inside
`plan-stage-verify`, and executes them in order against real files -- not a
hand-copied re-implementation of what the workflow "should" do. GITHUB_ENV
and GITHUB_OUTPUT are wired the same way the real Actions runner wires
them: each step's `env:` entries referencing `steps.<id>.outputs.<name>`
are resolved from the *actual* `$GITHUB_OUTPUT` lines the previous step
wrote, and `$GITHUB_ENV` lines are sourced into subsequent steps' shells.

This is what proves (not merely asserts) two of the reviewer's findings are
fixed at the workflow level, not just inside the CLI in isolation:

  * P0-3: the workflow's real two-call `stage --source-repo ai` then
    `stage --source-repo terraphim-clients` sequence (with real CAS
    generation threading between steps) succeeds end to end.
  * P1-1: the real "Determine resume mode" step's shell logic correctly
    computes skip_stage_verify for every coordinator status, including
    the reviewer's exact REPRO B scenario (status == 'promoting').
"""

from __future__ import annotations

import copy
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/release-coordinator.yml"
COORDINATOR = ROOT / ".github/scripts/release/release_coordinator.py"

ZIPSIGN_BIN = shutil.which("zipsign")

try:
    import yaml
except ImportError:  # pragma: no cover
    yaml = None


def load_job_steps(job_name: str) -> list[dict[str, Any]]:
    assert yaml is not None, (
        "PyYAML required for this test (see scripts/requirements-release-manifest.txt)"
    )
    doc = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    return doc["jobs"][job_name]["steps"]


def load_plan_stage_verify_steps() -> list[dict[str, Any]]:
    return load_job_steps("plan-stage-verify")


def step_by_name(steps: list[dict[str, Any]], name: str) -> dict[str, Any]:
    for step in steps:
        if step.get("name") == name:
            return step
    raise AssertionError(f"step not found: {name!r}")


ENV_EXPR_RE = re.compile(r"^\$\{\{\s*(.+?)\s*\}\}$")
STEPS_OUTPUT_RE = re.compile(r"^steps\.([A-Za-z0-9_-]+)\.outputs\.([A-Za-z0-9_-]+)$")


class WorkflowStepRunner:
    """Runs a sequence of real workflow steps, wiring env/outputs the way
    the Actions runner does, entirely against local files (no network)."""

    def __init__(self, work: Path) -> None:
        self.work = work
        self.step_outputs: dict[str, dict[str, str]] = {}
        self.step_outcomes: dict[str, str] = {}
        self.base_env = dict(os.environ)
        self.base_env["RUNNER_TEMP"] = str(work / "runner-temp")
        self.base_env["STATE_DIR"] = str(work / "state")
        self.base_env["COORDINATOR"] = str(COORDINATOR)
        Path(self.base_env["RUNNER_TEMP"]).mkdir(parents=True, exist_ok=True)

    def resolve_env_value(self, expr: str) -> str:
        match = (
            ENV_EXPR_RE.match(expr.strip())
            if isinstance(expr, str) and "${{" in expr
            else None
        )
        if match is None:
            return str(expr)
        inner = match.group(1)
        steps_match = STEPS_OUTPUT_RE.match(inner)
        if steps_match:
            step_id, output_name = steps_match.groups()
            return self.step_outputs.get(step_id, {}).get(output_name, "")
        if inner.startswith(("inputs.", "secrets.", "github.")):
            # The test's `extra_env` overrides these immediately after;
            # this placeholder is only reached if a step needs an
            # input/secret this test harness doesn't wire up.
            return ""
        raise AssertionError(f"unsupported expression in test harness: {expr!r}")

    def run(
        self, step: dict[str, Any], extra_env: dict[str, str] | None = None
    ) -> subprocess.CompletedProcess[str]:
        env = dict(self.base_env)
        for key, value in (step.get("env") or {}).items():
            env[key] = self.resolve_env_value(value)
        if extra_env:
            env.update(extra_env)

        github_output = self.work / "github_output.txt"
        github_output.write_text("")
        github_env = self.work / "github_env.txt"
        github_env.write_text("")
        env["GITHUB_OUTPUT"] = str(github_output)
        env["GITHUB_ENV"] = str(github_env)

        script = step["run"]
        for step_id, outcome in self.step_outcomes.items():
            expression = (
                "${{ steps."
                + step_id
                + ".outcome == 'success' && 'success' || 'failure' }}"
            )
            script = script.replace(
                expression, "success" if outcome == "success" else "failure"
            )
        result = subprocess.run(
            ["bash", "-c", script],
            cwd=str(self.work),
            env=env,
            check=False,
            text=True,
            capture_output=True,
            timeout=120,
        )

        step_id = step.get("id")
        if step_id:
            self.step_outcomes[step_id] = (
                "success" if result.returncode == 0 else "failure"
            )
            outputs: dict[str, str] = {}
            for line in github_output.read_text().splitlines():
                if "=" in line:
                    key, _, value = line.partition("=")
                    outputs[key] = value
            self.step_outputs[step_id] = outputs

        for line in github_env.read_text().splitlines():
            if "=" in line:
                key, _, value = line.partition("=")
                self.base_env[key] = value

        return result


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


SOURCES = {
    "terraphim-ai": {
        "gitea_sha": "1" * 40,
        "github_sha": "1" * 40,
        "tree_sha": "2" * 40,
        "workflow_run_id": 111,
    },
    "terraphim-clients": {
        "gitea_sha": "3" * 40,
        "github_sha": "3" * 40,
        "tree_sha": "4" * 40,
        "workflow_run_id": 222,
    },
}


def build_manifest(asset_bytes: dict[str, bytes]) -> dict[str, Any]:
    def asset(name: str, component: str, repo: str, sha: str) -> dict[str, Any]:
        data = asset_bytes[name]
        return {
            "name": name,
            "component": component,
            "format": "tar.gz",
            "target": "x86_64-unknown-linux-gnu",
            "arch": "x86_64",
            "os": "linux",
            "source_repo": repo,
            "source_sha": sha,
            "sha256": sha256_bytes(data),
            "size_bytes": len(data),
            "signature": f"{name}.sig",
        }

    ai_sha = SOURCES["terraphim-ai"]["gitea_sha"]
    clients_sha = SOURCES["terraphim-clients"]["gitea_sha"]
    return {
        "schema_version": "1.0.0",
        "release_version": "1.2.3",
        "release_tag": "v1.2.3",
        "sources": copy.deepcopy(SOURCES),
        "assets": [
            asset(
                "terraphim-server-1.2.3-linux-x86_64.tar.gz",
                "terraphim-server",
                "terraphim-ai",
                ai_sha,
            ),
            asset(
                "terraphim-agent-1.2.3-linux-x86_64.tar.gz",
                "terraphim-agent",
                "terraphim-clients",
                clients_sha,
            ),
            asset(
                "terraphim-grep-1.2.3-linux-x86_64.tar.gz",
                "terraphim-grep",
                "terraphim-clients",
                clients_sha,
            ),
        ],
        "required_channels": [
            "github_release",
            "r2_stable_manifest",
            "homebrew_tap_pr",
            "aur_terraphim_clients_bin",
            "omarchy_terraphim_clients_bin",
        ],
        "central_channels": ["github_release", "r2_stable_manifest"],
        "downstream_channels": [
            "homebrew_tap_pr",
            "aur_terraphim_clients_bin",
            "omarchy_terraphim_clients_bin",
        ],
        "created_at": "2026-09-18T10:00:00Z",
    }


@unittest.skipUnless(yaml is not None, "PyYAML not installed")
@unittest.skipUnless(ZIPSIGN_BIN, "zipsign binary not on PATH")
class LiteralWorkflowSequenceContract(unittest.TestCase):
    """Runs the workflow's own literal step bodies, not a re-implementation."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.work = Path(self.tmp.name)
        self.steps = load_plan_stage_verify_steps()
        self.runner = WorkflowStepRunner(self.work)
        # The workflow's step bodies address these fixture inputs via
        # $RUNNER_TEMP/<subdir>/..., and RUNNER_TEMP is runner.base_env's
        # own scratch dir (work/runner-temp) -- NOT self.work itself. They
        # must be created there, or every step that reads them fails with
        # FileNotFoundError (as opposed to a genuine coordinator/workflow
        # defect) and the whole point of running literal step bodies is
        # defeated.
        runner_temp = Path(self.runner.base_env["RUNNER_TEMP"])

        self.asset_bytes = {
            "terraphim-server-1.2.3-linux-x86_64.tar.gz": b"server-payload",
            "terraphim-agent-1.2.3-linux-x86_64.tar.gz": b"agent-payload",
            "terraphim-grep-1.2.3-linux-x86_64.tar.gz": b"grep-payload",
        }
        ai_dir = runner_temp / "producer-artifacts" / "terraphim-ai"
        clients_dir = runner_temp / "producer-artifacts" / "terraphim-clients"
        ai_dir.mkdir(parents=True)
        clients_dir.mkdir(parents=True)
        (ai_dir / "terraphim-server-1.2.3-linux-x86_64.tar.gz").write_bytes(
            self.asset_bytes["terraphim-server-1.2.3-linux-x86_64.tar.gz"]
        )
        for name in (
            "terraphim-agent-1.2.3-linux-x86_64.tar.gz",
            "terraphim-grep-1.2.3-linux-x86_64.tar.gz",
        ):
            (clients_dir / name).write_bytes(self.asset_bytes[name])

        manifest_dir = runner_temp / "manifest"
        manifest_dir.mkdir()
        (manifest_dir / "manifest.json").write_text(
            json.dumps(build_manifest(self.asset_bytes))
        )

        coordinator_inputs = runner_temp / "coordinator-inputs"
        coordinator_inputs.mkdir()
        (coordinator_inputs / "sources.json").write_text(json.dumps(SOURCES))

        self.zipsign_priv = self.work / "zipsign-private.key"
        self.zipsign_pub = self.work / "zipsign-public.key"
        for _ in range(64):
            gen = subprocess.run(
                [
                    ZIPSIGN_BIN,
                    "gen-key",
                    "-f",
                    str(self.zipsign_priv),
                    str(self.zipsign_pub),
                ],
                check=False,
                text=True,
                capture_output=True,
                timeout=30,
            )
            self.assertEqual(gen.returncode, 0, gen.stderr)
            if b"\x00" not in self.zipsign_priv.read_bytes():
                break

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_real_plan_stage_sign_verify_step_sequence_succeeds(self) -> None:
        """The workflow's actual literal step bodies, run in the exact
        order they appear in the YAML, with real CAS generation threading
        between them (matches P0-3's fix and P2's CAS-threading fix)."""
        plan = self.runner.run(
            step_by_name(self.steps, "Coordinator plan (freeze version/tag/sources)"),
            extra_env={"RELEASE_VERSION": "1.2.3", "RELEASE_TAG": "v1.2.3"},
        )
        self.assertEqual(plan.returncode, 0, plan.stderr)

        stage_ai = self.runner.run(
            step_by_name(
                self.steps, "Coordinator stage (terraphim-ai artifacts, no rebuild)"
            )
        )
        self.assertEqual(stage_ai.returncode, 0, stage_ai.stderr)

        stage_clients = self.runner.run(
            step_by_name(
                self.steps,
                "Coordinator stage (terraphim-clients artifacts, no rebuild)",
            )
        )
        self.assertEqual(stage_clients.returncode, 0, stage_clients.stderr)

        signing_key = self.runner.run(
            step_by_name(self.steps, "Fetch zipsign signing key"),
            extra_env={
                "ZIPSIGN_PRIVATE_KEY": self.zipsign_priv.read_bytes().decode(
                    "utf-8", errors="surrogateescape"
                )
            },
        )
        self.assertEqual(signing_key.returncode, 0, signing_key.stderr)

        sign = self.runner.run(
            step_by_name(
                self.steps, "Coordinator sign-assets (real zipsign Ed25519 signature)"
            )
        )
        self.assertEqual(sign.returncode, 0, sign.stderr)

        verify = self.runner.run(
            step_by_name(
                self.steps,
                "Coordinator verify (schema + business invariants + signature + digest freeze)",
            )
        )
        self.assertEqual(verify.returncode, 0, verify.stderr)

        state = json.loads((self.work / "state" / "state.json").read_text())
        self.assertEqual(state["status"], "verified")
        self.assertIsNotNone(state["manifest_sha256"])

    def test_stage_ai_alone_never_satisfies_the_full_manifest(self) -> None:
        """Documents why the split-call design is necessary: even the
        real workflow step body for one producer alone cannot make verify
        pass (proving the fix is genuinely split, not accidentally merged
        back into a single directory somewhere)."""
        self.runner.run(
            step_by_name(self.steps, "Coordinator plan (freeze version/tag/sources)"),
            extra_env={"RELEASE_VERSION": "1.2.3", "RELEASE_TAG": "v1.2.3"},
        )
        stage_ai = self.runner.run(
            step_by_name(
                self.steps, "Coordinator stage (terraphim-ai artifacts, no rebuild)"
            )
        )
        self.assertEqual(stage_ai.returncode, 0, stage_ai.stderr)

        staged = {p.name for p in (self.work / "state" / "assets").iterdir()}
        self.assertEqual(staged, {"terraphim-server-1.2.3-linux-x86_64.tar.gz"})

    def test_real_central_promote_recording_sequence_is_cas_safe(self) -> None:
        """Execute the literal central-promote approval, transition, phase,
        publication, and terminal-verification recorders. This catches empty
        skipped-step CAS values and proves promotion requires both verified
        channels rather than dispatch/publication acceptance alone."""
        for name, extra in [
            (
                "Coordinator plan (freeze version/tag/sources)",
                {"RELEASE_VERSION": "1.2.3", "RELEASE_TAG": "v1.2.3"},
            ),
            ("Coordinator stage (terraphim-ai artifacts, no rebuild)", None),
            ("Coordinator stage (terraphim-clients artifacts, no rebuild)", None),
            (
                "Fetch zipsign signing key",
                {
                    "ZIPSIGN_PRIVATE_KEY": self.zipsign_priv.read_bytes().decode(
                        "utf-8", errors="surrogateescape"
                    )
                },
            ),
            ("Coordinator sign-assets (real zipsign Ed25519 signature)", None),
            (
                "Coordinator verify (schema + business invariants + signature + digest freeze)",
                None,
            ),
        ]:
            result = self.runner.run(step_by_name(self.steps, name), extra_env=extra)
            self.assertEqual(result.returncode, 0, f"{name}: {result.stderr}")

        digest = json.loads((self.work / "state" / "state.json").read_text())[
            "manifest_sha256"
        ]
        central = load_job_steps("central-promote")
        approve = self.runner.run(
            step_by_name(central, "Record approval bound to verified manifest digest"),
            extra_env={"APPROVED_BY": "operator", "APPROVAL": digest},
        )
        self.assertEqual(approve.returncode, 0, approve.stderr)
        begin = self.runner.run(
            step_by_name(
                central,
                "Begin central promotion (fails closed without a bound approval)",
            )
        )
        self.assertEqual(begin.returncode, 0, begin.stderr)

        sequence = [
            ("release_create", "Record release creation outcome", {"APPROVAL": digest}),
            (
                "package_reconcile",
                "Record exact release reconciliation outcome",
                {"APPROVAL": digest},
            ),
            ("github_publish", "Record GitHub publication outcome", None),
            ("github_verify", "Record GitHub terminal verification outcome", None),
            ("r2_publish", "Record R2 publication outcome", None),
            ("r2_verify", "Record R2 terminal verification outcome", None),
        ]
        for outcome_id, name, extra in sequence:
            self.runner.step_outcomes[outcome_id] = "success"
            result = self.runner.run(step_by_name(central, name), extra_env=extra)
            self.assertEqual(result.returncode, 0, f"{name}: {result.stderr}")

        state = json.loads((self.work / "state" / "state.json").read_text())
        self.assertEqual(state["status"], "promoted")
        self.assertEqual(state["phases"]["publication"]["status"], "complete")
        self.assertEqual(state["phases"]["verification"]["status"], "complete")

    def test_final_promotion_gate_cannot_be_optimized_away(self) -> None:
        state_dir = self.work / "state"
        state_dir.mkdir(exist_ok=True)
        (state_dir / "state.json").write_text(
            json.dumps({"status": "promoting", "central_channels": {}})
        )
        gate = step_by_name(
            load_job_steps("central-promote"),
            "Fail the job if either central channel did not succeed",
        )
        result = self.runner.run(gate, extra_env={"PYTHONOPTIMIZE": "1"})
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("central promotion incomplete", result.stderr)

    def test_literal_dispatch_script_transmits_versioned_correlation_binding(
        self,
    ) -> None:
        node = shutil.which("node")
        self.assertIsNotNone(
            node, "node is required to execute github-script literally"
        )
        state_dir = self.work / "state"
        state_dir.mkdir(exist_ok=True)
        correlation_id = "c" * 64
        channels = [
            "homebrew_tap_pr",
            "aur_terraphim_clients_bin",
            "omarchy_terraphim_clients_bin",
        ]
        handoff = {
            "schema_version": "1.0.0",
            "release_tag": "v1.2.3",
            "manifest_sha256": "a" * 64,
            "correlation_id": correlation_id,
            "channels": [
                {
                    "channel": channel,
                    "assets": [{"name": f"{channel}.tar.gz", "sha256": "b" * 64}],
                }
                for channel in channels
            ],
        }
        (state_dir / "downstream-handoff.json").write_text(json.dumps(handoff))
        (state_dir / "state.json").write_text(
            json.dumps({"generation": 1, "downstream_channels": {}})
        )

        fake_bin = self.work / "fake-bin"
        fake_bin.mkdir()
        fake_python = fake_bin / "python3"
        fake_python.write_text("#!/usr/bin/env bash\nexit 0\n")
        fake_python.chmod(0o755)
        output = self.work / "dispatch-payloads.json"
        dispatch_step = step_by_name(
            load_job_steps("downstream-handoff"),
            "Dispatch downstream channel workflows (real, authenticated, digest-bound; fail closed if target/token absent)",
        )
        script = dispatch_step["with"]["script"]
        wrapper = f"""
        const fs = require('fs');
        const AsyncFunction = Object.getPrototypeOf(async function(){{}}).constructor;
        const payloads = [];
        const github = {{rest: {{repos: {{createDispatchEvent: async (value) => payloads.push(value)}}}}}};
        const core = {{setFailed: (message) => {{ throw new Error(message); }}}};
        (async () => {{
          const execute = new AsyncFunction('github', 'core', 'require', {json.dumps(script)});
          await execute(github, core, require);
          fs.writeFileSync({json.dumps(str(output))}, JSON.stringify(payloads));
        }})().catch((error) => {{ console.error(error); process.exit(1); }});
        """
        env = dict(os.environ)
        env.update(
            {
                "PATH": f"{fake_bin}{os.pathsep}{env['PATH']}",
                "STATE_DIR": str(state_dir),
                "COORDINATOR": str(COORDINATOR),
                "HOMEBREW_TAP_REPO": "terraphim/homebrew-terraphim",
                "AUR_TARGET_REPO": "terraphim/aur-packages",
                "OMARCHY_TARGET_REPO": "terraphim/omarchy",
                "DISPATCH_TOKEN_CONFIGURED": "true",
            }
        )
        result = subprocess.run(
            [node, "-e", wrapper],
            cwd=self.work,
            env=env,
            check=False,
            text=True,
            capture_output=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        payloads = json.loads(output.read_text())
        self.assertEqual(len(payloads), 3)
        for payload, channel in zip(payloads, channels, strict=True):
            self.assertEqual(payload["event_type"], "terraphim-release")
            client = payload["client_payload"]
            self.assertEqual(
                set(client),
                {
                    "schema_version",
                    "release_tag",
                    "manifest_sha256",
                    "correlation_id",
                    "channel",
                    "assets",
                },
            )
            self.assertEqual(client["schema_version"], "1.0.0")
            self.assertEqual(client["correlation_id"], correlation_id)
            self.assertEqual(client["channel"], channel)


class ResumeModeDecisionContract(unittest.TestCase):
    """Runs the real "Determine resume mode" step body against every
    coordinator status, proving the reviewer's REPRO B is fixed: a resumed
    'promoting' state must set skip_stage_verify=true so the workflow
    never re-attempts stage() and hits 'illegal transition'."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.work = Path(self.tmp.name)
        (self.work / "state").mkdir()
        steps = load_plan_stage_verify_steps()
        self.step = step_by_name(steps, "Determine resume mode")

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def run_resume_mode(self, status: str | None) -> dict[str, str]:
        state_dir = self.work / "state"
        if status is None:
            for p in state_dir.glob("*"):
                p.unlink()
        else:
            (state_dir / "state.json").write_text(json.dumps({"status": status}))

        github_output = self.work / "github_output.txt"
        github_output.write_text("")
        env = dict(os.environ)
        env["STATE_DIR"] = str(state_dir)
        env["GITHUB_OUTPUT"] = str(github_output)
        result = subprocess.run(
            ["bash", "-c", self.step["run"]],
            cwd=str(self.work),
            env=env,
            check=False,
            text=True,
            capture_output=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        outputs = {}
        for line in github_output.read_text().splitlines():
            if "=" in line:
                k, _, v = line.partition("=")
                outputs[k] = v
        return outputs

    def test_promoting_status_skips_stage_verify_repro_b(self) -> None:
        outputs = self.run_resume_mode("promoting")
        self.assertEqual(outputs["status"], "promoting")
        self.assertEqual(outputs["skip_stage_verify"], "true")

    def test_promoted_status_skips_stage_verify_for_downstream_proof_resume(
        self,
    ) -> None:
        for status in ("promoted",):
            with self.subTest(status=status):
                outputs = self.run_resume_mode(status)
                self.assertEqual(outputs["skip_stage_verify"], "true")

    def test_planned_and_staged_do_not_skip_but_verified_does(self) -> None:
        for status in ("planned", "staged"):
            with self.subTest(status=status):
                outputs = self.run_resume_mode(status)
                self.assertEqual(outputs["skip_stage_verify"], "false")
        outputs = self.run_resume_mode("verified")
        self.assertEqual(outputs["skip_stage_verify"], "true")

    def test_failed_and_superseded_are_rejected_as_terminal(self) -> None:
        for status in ("failed", "superseded"):
            with self.subTest(status=status):
                state_dir = self.work / "state"
                (state_dir / "state.json").write_text(json.dumps({"status": status}))
                env = dict(os.environ)
                env["STATE_DIR"] = str(state_dir)
                env["GITHUB_OUTPUT"] = str(self.work / "terminal-output.txt")
                result = subprocess.run(
                    ["bash", "-c", self.step["run"]],
                    cwd=str(self.work),
                    env=env,
                    check=False,
                    text=True,
                    capture_output=True,
                    timeout=30,
                )
                self.assertNotEqual(result.returncode, 0)

    def test_no_prior_state_does_not_skip(self) -> None:
        outputs = self.run_resume_mode(None)
        self.assertEqual(outputs["status"], "none")
        self.assertEqual(outputs["skip_stage_verify"], "false")


if __name__ == "__main__":
    unittest.main()
