"""Tests for native PR gate agent templates (pr-reviewer, pr-validator, pr-verifier).

Validates Step 5 of .docs/adf/2334/research-design.md:
- TOML parses correctly
- No skill_chain (bounded orchestrator prompt only)
- No schedule (event-driven)
- Task forbids tools, comments, and status posts
- No producer-side shell or gtr usage
"""

from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore[no-redef]

AGENTS_DIR = Path(__file__).parent.parent / "agents"

GATE_TEMPLATES = {
    "pr-reviewer": AGENTS_DIR / "pr-reviewer.toml",
    "pr-validator": AGENTS_DIR / "pr-validator.toml",
    "pr-verifier": AGENTS_DIR / "pr-verifier.toml",
}

FORBIDDEN_TASK_KEYWORDS = [
    "gtr comment",
    "curl -",
    "source ~/.profile",
    "git fetch",
    "skill_chain",
    "structural-pr-review",
    "disciplined-validation",
    "disciplined-verification",
]

REQUIRED_FIELDS = [
    "name",
    "layer",
    "cli_tool",
    "model",
    "fallback_model",
    "persona",
    "max_cpu_seconds",
    "grace_period_secs",
    "task",
]


def _load_agent(path: Path) -> dict:
    with open(path, "rb") as fh:
        config = tomllib.load(fh)
    assert "agents" in config and config["agents"], f"No agents in {path}"
    return config["agents"][0]


def test_templates_exist():
    for name, path in GATE_TEMPLATES.items():
        assert path.exists(), f"Missing template for {name}: {path}"


def test_templates_parse():
    for path in GATE_TEMPLATES.values():
        _load_agent(path)


def test_agent_names_match_files():
    for expected_name, path in GATE_TEMPLATES.items():
        agent = _load_agent(path)
        assert agent["name"] == expected_name


def test_required_fields_present():
    for path in GATE_TEMPLATES.values():
        agent = _load_agent(path)
        for field in REQUIRED_FIELDS:
            assert field in agent, f"{path.name} missing field: {field}"


def test_no_skill_chain():
    for path in GATE_TEMPLATES.values():
        agent = _load_agent(path)
        assert "skill_chain" not in agent, f"{path.name} must not define skill_chain"


def test_no_schedule():
    for path in GATE_TEMPLATES.values():
        agent = _load_agent(path)
        assert "schedule" not in agent, f"{path.name} must be event-driven (no schedule)"


def test_task_contract():
    for path in GATE_TEMPLATES.values():
        agent = _load_agent(path)
        task = agent["task"].lower()
        assert "adf:gate-result" in task, f"{path.name} task must mention adf:gate-result"
        assert "do not call tools" in task, f"{path.name} task must forbid tools"
        assert "do not post comments" in task or "do not call tools, post comments" in task


def test_no_producer_side_shell_or_gtr():
    for path in GATE_TEMPLATES.values():
        agent = _load_agent(path)
        task = agent["task"]
        for keyword in FORBIDDEN_TASK_KEYWORDS:
            assert keyword not in task, f"{path.name} task must not contain: {keyword}"