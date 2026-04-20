"""Tests for migrate-to-confd.py

Treats the script as a black box -- invoked via subprocess.
No mocks used.
"""

import subprocess
import sys
import tempfile
from pathlib import Path

# Absolute path to the script under test.
SCRIPT = Path(__file__).parent.parent / "migrate-to-confd.py"
FIXTURES = Path(__file__).parent / "fixtures"


def run_migration(*extra_args, check=False, cwd=None):
    """Invoke migrate-to-confd.py via uv run and return CompletedProcess."""
    cmd = ["uv", "run", str(SCRIPT)] + list(extra_args)
    return subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        check=check,
        cwd=cwd,
    )


# ---------------------------------------------------------------------------
# Test 1: Round-trip -- fixture input produces expected output structure
# ---------------------------------------------------------------------------

def test_round_trip_structure():
    """Running the migration on fixtures produces correct [[projects]], [[agents]], [[flows]]."""
    try:
        import tomllib
    except ImportError:
        import tomli as tomllib  # type: ignore[no-redef]

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        confd_dir = tmp_path / "conf.d"
        base_out = tmp_path / "orchestrator.toml"

        result = run_migration(
            "--input", str(FIXTURES / "orchestrator.toml"),
            "--input", str(FIXTURES / "odilo-orchestrator.toml"),
            "--output-dir", str(confd_dir),
            "--base-output", str(base_out),
        )
        assert result.returncode == 0, f"Script failed:\n{result.stderr}"

        # --- base orchestrator.toml checks ---
        with open(base_out, "rb") as fh:
            base = tomllib.load(fh)

        assert "include" in base, "base must have 'include' key"
        assert base["include"] == ["conf.d/*.toml"], f"unexpected include: {base['include']}"
        assert "agents" not in base, "base must not contain agents"
        assert "flows" not in base, "base must not contain flows"
        assert "projects" not in base, "base must not contain projects"
        assert "working_dir" in base, "base must have working_dir"
        assert "nightwatch" in base, "base must have nightwatch"
        assert "compound_review" in base, "base must have compound_review"

        # --- terraphim.toml checks ---
        terraphim_path = confd_dir / "terraphim.toml"
        assert terraphim_path.exists(), "terraphim.toml not created"
        with open(terraphim_path, "rb") as fh:
            terraphim = tomllib.load(fh)

        projects = terraphim.get("projects", [])
        assert len(projects) == 1, f"expected 1 project, got {len(projects)}"
        assert projects[0]["id"] == "terraphim"
        assert projects[0]["working_dir"] == "/home/alex/terraphim-ai"

        agents = terraphim.get("agents", [])
        assert len(agents) == 3, f"expected 3 agents, got {len(agents)}"
        for agent in agents:
            assert agent.get("project") == "terraphim", (
                f"agent '{agent['name']}' missing project='terraphim'"
            )

        flows = terraphim.get("flows", [])
        assert len(flows) == 1, f"expected 1 flow, got {len(flows)}"
        assert flows[0]["project"] == "terraphim"
        assert flows[0]["name"] == "security-audit-flow"

        # --- odilo.toml checks ---
        odilo_path = confd_dir / "odilo.toml"
        assert odilo_path.exists(), "odilo.toml not created"
        with open(odilo_path, "rb") as fh:
            odilo = tomllib.load(fh)

        o_projects = odilo.get("projects", [])
        assert len(o_projects) == 1
        assert o_projects[0]["id"] == "odilo"
        assert o_projects[0]["working_dir"] == "/home/alex/projects/odilo"

        o_agents = odilo.get("agents", [])
        assert len(o_agents) == 2
        for agent in o_agents:
            assert agent.get("project") == "odilo", (
                f"odilo agent '{agent['name']}' missing project='odilo'"
            )

        # odilo has no flows -- key should be absent or empty
        assert odilo.get("flows", []) == []


# ---------------------------------------------------------------------------
# Test 2: Idempotence -- running twice produces byte-identical output
# ---------------------------------------------------------------------------

def test_idempotent():
    """Running the migration twice produces byte-identical output files."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)

        def run_once(run_id: int):
            confd_dir = tmp_path / f"run{run_id}" / "conf.d"
            base_out = tmp_path / f"run{run_id}" / "orchestrator.toml"
            result = run_migration(
                "--input", str(FIXTURES / "orchestrator.toml"),
                "--input", str(FIXTURES / "odilo-orchestrator.toml"),
                "--output-dir", str(confd_dir),
                "--base-output", str(base_out),
            )
            assert result.returncode == 0, f"Run {run_id} failed:\n{result.stderr}"
            return base_out, confd_dir

        base1, confd1 = run_once(1)
        base2, confd2 = run_once(2)

        # Compare base files.
        assert base1.read_bytes() == base2.read_bytes(), "base orchestrator.toml differs between runs"

        # Compare each conf.d file.
        for name in ["terraphim.toml", "odilo.toml"]:
            b1 = (confd1 / name).read_bytes()
            b2 = (confd2 / name).read_bytes()
            assert b1 == b2, f"conf.d/{name} differs between runs"


# ---------------------------------------------------------------------------
# Test 3: C1 rejection -- banned-provider input exits non-zero with agent name
# ---------------------------------------------------------------------------

def test_banned_provider_rejected():
    """Script must exit non-zero when an agent uses a banned provider prefix."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        result = run_migration(
            "--input", str(FIXTURES / "banned-provider.toml"),
            "--output-dir", str(tmp_path / "conf.d"),
            "--base-output", str(tmp_path / "orchestrator.toml"),
        )
        assert result.returncode != 0, "Expected non-zero exit for banned provider"
        assert "banned" in result.stderr.lower() or "ERROR" in result.stderr, (
            f"Expected error message in stderr, got:\n{result.stderr}"
        )
        assert "bad-agent" in result.stderr, (
            f"Expected agent name 'bad-agent' in error, got:\n{result.stderr}"
        )
        assert "opencode/" in result.stderr, (
            f"Expected banned value 'opencode/' in error, got:\n{result.stderr}"
        )


# ---------------------------------------------------------------------------
# Test 4: Flow project injection -- flows get project field added
# ---------------------------------------------------------------------------

def test_flow_project_injection():
    """Each flow in the output has the correct project field."""
    try:
        import tomllib
    except ImportError:
        import tomli as tomllib  # type: ignore[no-redef]

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        confd_dir = tmp_path / "conf.d"
        base_out = tmp_path / "orchestrator.toml"

        result = run_migration(
            "--input", str(FIXTURES / "orchestrator.toml"),
            "--output-dir", str(confd_dir),
            "--base-output", str(base_out),
        )
        assert result.returncode == 0, f"Script failed:\n{result.stderr}"

        terraphim_path = confd_dir / "terraphim.toml"
        with open(terraphim_path, "rb") as fh:
            doc = tomllib.load(fh)

        flows = doc.get("flows", [])
        assert len(flows) >= 1, "Expected at least one flow in terraphim.toml"
        for flow in flows:
            assert "project" in flow, f"Flow '{flow.get('name')}' missing project field"
            assert flow["project"] == "terraphim", (
                f"Flow '{flow.get('name')}' has wrong project: {flow['project']!r}"
            )


# ---------------------------------------------------------------------------
# Test 5: Dry-run -- no files written
# ---------------------------------------------------------------------------

def test_dry_run_writes_nothing():
    """With --dry-run, no output files are created."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        confd_dir = tmp_path / "conf.d"
        base_out = tmp_path / "orchestrator.toml"

        result = run_migration(
            "--dry-run",
            "--input", str(FIXTURES / "orchestrator.toml"),
            "--output-dir", str(confd_dir),
            "--base-output", str(base_out),
        )
        assert result.returncode == 0, f"Script failed:\n{result.stderr}"
        assert not base_out.exists(), "base file should not be written in dry-run"
        assert not confd_dir.exists(), "conf.d dir should not be created in dry-run"
        assert "dry-run" in result.stdout.lower(), "Expected dry-run notice in stdout"


# ---------------------------------------------------------------------------
# Test 6: github-copilot/ prefix also rejected
# ---------------------------------------------------------------------------

def test_github_copilot_banned():
    """github-copilot/ prefix is also a banned provider."""
    # Write a minimal inline TOML fixture as a plain string -- no need for
    # external serialisation library in the test itself.
    fixture_toml = """\
working_dir = "/tmp/test"
restart_cooldown_secs = 300
max_restart_count = 3
tick_interval_secs = 30

[nightwatch]
eval_interval_secs = 300
minor_threshold = 0.1
moderate_threshold = 0.2
severe_threshold = 0.4
critical_threshold = 0.7

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/test"

[[agents]]
name = "copilot-agent"
layer = "Core"
cli_tool = "/usr/bin/gh"
model = "github-copilot/gpt-4o"
task = "Do something."
"""

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        fixture_path = tmp_path / "copilot-orchestrator.toml"
        fixture_path.write_text(fixture_toml, encoding="utf-8")

        result = run_migration(
            "--input", str(fixture_path),
            "--output-dir", str(tmp_path / "conf.d"),
            "--base-output", str(tmp_path / "orchestrator.toml"),
        )
        assert result.returncode != 0, "Expected non-zero exit for github-copilot provider"
        assert "copilot-agent" in result.stderr
        assert "github-copilot/" in result.stderr
