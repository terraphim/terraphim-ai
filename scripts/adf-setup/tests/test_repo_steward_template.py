"""Tests for repo-steward agent template.

Validates the repository stewardship contract:
- TOML parses correctly
- Contains required fields
- Correct schedule and capabilities
- No dangerous flags (auto-merge, branch creation)
- Task contains two-panel analysis workflow
"""

import os
import subprocess
import sys
from pathlib import Path

import pytest

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore[no-redef]

# Path to the template under test.
TEMPLATE = Path(__file__).parent.parent / "agents" / "repo-steward.toml"

# Dangerous keywords that should NOT appear in task.
DANGEROUS_KEYWORDS = [
    "git checkout -b",
    "git add -A",
    "git commit",
    "git push",
    "merge-pull",
    "auto-merge",
    "cargo build",
    "cargo test",
]

# Required capabilities for repo-steward agent.
REQUIRED_CAPABILITIES = [
    "repo-stewardship",
    "stability-synthesis",
    "usefulness-synthesis",
    "backlog-prioritization",
]


def test_template_exists():
    """Template file must exist."""
    assert TEMPLATE.exists(), f"Template not found: {TEMPLATE}"


def test_template_parses():
    """TOML must parse without errors."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)
    assert "agents" in config, "Missing [[agents]] section"
    assert len(config["agents"]) >= 1, "No agents defined"


def test_agent_has_required_fields():
    """Agent definition must contain all required fields."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)

    agent = config["agents"][0]
    required_fields = [
        "name",
        "layer",
        "cli_tool",
        "persona",
        "terraphim_role",
        "skill_chain",
        "schedule",
        "max_cpu_seconds",
        "grace_period_secs",
        "capabilities",
        "task",
    ]
    for field in required_fields:
        assert field in agent, f"Missing required field: {field}"


def test_agent_name_is_repo_steward():
    """Agent name must be repo-steward."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)
    assert config["agents"][0]["name"] == "repo-steward"


def test_layer_is_growth():
    """Layer must be Growth for stewardship agent."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)
    assert config["agents"][0]["layer"] == "Growth"


def test_schedule_is_every_6_hours():
    """Schedule must be every 6 hours at :15."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)
    assert config["agents"][0]["schedule"] == "15 */6 * * *"


def test_persona_is_carthos():
    """Persona must be Carthos."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)
    assert config["agents"][0]["persona"] == "Carthos"


def test_capabilities_are_stewardship():
    """Capabilities must include stewardship keywords."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)

    capabilities = config["agents"][0]["capabilities"]
    for cap in REQUIRED_CAPABILITIES:
        assert cap in capabilities, f"Missing required capability: {cap}"


def test_task_has_no_dangerous_flags():
    """Task must not contain branch/PR/merge/build commands."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)

    task = config["agents"][0]["task"]

    for keyword in DANGEROUS_KEYWORDS:
        assert keyword not in task, f"Task contains dangerous keyword: {keyword}"


def test_task_has_two_panel_analysis():
    """Task must contain two-panel (stability + usefulness) workflow."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)

    task = config["agents"][0]["task"]
    panel_indicators = [
        "stability",
        "usefulness",
        "Theme-ID",
        "evidence",
        "panel",
    ]
    found = [ind for ind in panel_indicators if ind in task.lower()]
    assert len(found) >= 4, f"Task missing panel indicators. Found: {found}"


def test_task_has_deduplication():
    """Task must contain Theme-ID deduplication logic."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)

    task = config["agents"][0]["task"]
    assert "Theme-ID" in task, "Missing Theme-ID in task"
    assert "existing" in task.lower() or "duplicate" in task.lower(), "Missing duplicate check"


def test_task_has_learning_capture():
    """Task must capture learnings after run."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)

    task = config["agents"][0]["task"]
    assert "learn capture" in task, "Missing learning capture"


def test_pre_check_exists():
    """Agent should have a pre_check defined."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)
    assert "pre_check" in config["agents"][0], "Missing pre_check"


def test_model_constraints():
    """Must use subscription-only models (C1 constraint)."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)

    agent = config["agents"][0]
    model = agent.get("model", "")
    fallback = agent.get("fallback_model", "")

    banned_prefixes = ["opencode/", "github-copilot/", "minimax/"]
    for prefix in banned_prefixes:
        assert not model.startswith(prefix), f"Banned model prefix: {prefix}"
        assert not fallback.startswith(prefix), f"Banned fallback prefix: {prefix}"
