"""Tests for product-development agent template.

Validates the planning-only contract:
- TOML parses correctly
- Contains required fields
- No implementation workflow (no branch/PR/build commands)
- Capabilities are planning-only
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
TEMPLATE = Path(__file__).parent.parent / "agents" / "product-development.toml"

# Keywords that indicate implementation work (should NOT appear in task).
IMPLEMENTATION_KEYWORDS = [
    "git checkout -b",
    "git add -A",
    "git commit",
    "git push",
    "create-pull",
    "cargo build",
    "cargo test",
    "cargo check",
    "cargo clippy",
]

# Required capabilities for planning-only agent.
REQUIRED_CAPABILITIES = [
    "product-development",
    "roadmap-prioritization",
    "feature-prioritization",
    "backlog-shaping",
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


def test_agent_name_is_product_development():
    """Agent name must be product-development."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)
    assert config["agents"][0]["name"] == "product-development"


def test_layer_is_core():
    """Layer must be Core for planning agent."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)
    assert config["agents"][0]["layer"] == "Core"


def test_schedule_is_daily():
    """Schedule must be daily at 02:00."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)
    assert config["agents"][0]["schedule"] == "0 2 * * *"


def test_persona_is_carthos():
    """Persona must be Carthos (not Lux or Ferrox)."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)
    assert config["agents"][0]["persona"] == "Carthos"


def test_capabilities_are_planning_only():
    """Capabilities must include planning keywords and exclude implementation."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)

    capabilities = config["agents"][0]["capabilities"]
    for cap in REQUIRED_CAPABILITIES:
        assert cap in capabilities, f"Missing required capability: {cap}"

    # Must NOT contain implementation keywords.
    implementation_caps = ["implementation", "rust-development", "frontend", "ui"]
    for cap in implementation_caps:
        assert cap not in capabilities, f"Capability must not include implementation: {cap}"


def test_task_has_no_implementation_workflow():
    """Task must not contain branch/PR/build commands."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)

    task = config["agents"][0]["task"]
    task_lower = task.lower()

    for keyword in IMPLEMENTATION_KEYWORDS:
        assert keyword not in task, f"Task contains implementation keyword: {keyword}"


def test_task_has_planning_workflow():
    """Task must contain planning-specific workflow steps."""
    with open(TEMPLATE, "rb") as fh:
        config = tomllib.load(fh)

    task = config["agents"][0]["task"]
    planning_indicators = [
        "prioritization",
        "roadmap",
        "ready issues",
        "rank",
        "sequencing",
    ]
    found = [ind for ind in planning_indicators if ind in task.lower()]
    assert len(found) >= 3, f"Task missing planning indicators. Found: {found}"


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
