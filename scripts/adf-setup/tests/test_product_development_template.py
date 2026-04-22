"""Verify the product-development agent template enforces the planning-only contract.

Acceptance criteria from issue #748:
- planning-only behaviour (roadmap, prioritisation, shaping)
- no branch / PR / implementation workflow
- no Lux or frontend/UI semantics
"""

import tomllib
from pathlib import Path

import pytest

TEMPLATE_PATH = Path(__file__).parent.parent / "agents" / "product-development.toml"


@pytest.fixture
def template() -> dict:
    assert TEMPLATE_PATH.exists(), f"Template not found: {TEMPLATE_PATH}"
    with TEMPLATE_PATH.open("rb") as f:
        return tomllib.load(f)


def test_template_exists():
    """product-development.toml must exist."""
    assert TEMPLATE_PATH.exists()


def test_agent_name(template: dict):
    """Agent name must be exactly product-development."""
    agents = template.get("agents", [])
    assert len(agents) == 1, "Expected exactly one agent definition"
    assert agents[0]["name"] == "product-development"


def test_planning_only_capabilities(template: dict):
    """Capabilities must be planning-only; no implementation or UI semantics."""
    caps = template["agents"][0].get("capabilities", [])
    forbidden = {
        "feature-implementation",
        "api-design",
        "typescript",
        "frontend",
        "ui",
        "lux",
    }
    lower_caps = {c.lower() for c in caps}
    overlap = lower_caps & forbidden
    assert not overlap, f"Forbidden capabilities found: {overlap}"
    assert "roadmap" in lower_caps or "prioritisation" in lower_caps, (
        "Expected at least one planning capability"
    )


def test_no_lux_persona(template: dict):
    """Persona must not be Lux."""
    persona = template["agents"][0].get("persona", "").lower()
    assert persona != "lux", "product-development must not use Lux persona"


def test_task_forbidden_patterns(template: dict):
    """Task script must not contain branch/PR creation or implementation triggers."""
    task = template["agents"][0].get("task", "")
    task_lower = task.lower()
    # Actionable forbidden patterns (not negated constraints like "Do NOT ...")
    forbidden_patterns = [
        "git checkout -b",
        "git push",
        "create pull request",
        "gtr create-pull",
        "@adf:lux",
    ]
    violations = []
    for p in forbidden_patterns:
        idx = task_lower.find(p)
        if idx != -1:
            # Check that the line doesn't start with a negation like "do not" or "must not"
            line_start = task_lower.rfind("\n", 0, idx) + 1
            line_prefix = task_lower[line_start:idx].strip().lower()
            if not (
                line_prefix.startswith("do not")
                or line_prefix.startswith("must not")
                or line_prefix.startswith("- do not")
            ):
                violations.append(p)
    assert not violations, f"Forbidden actionable patterns in task: {violations}"


def test_handoff_to_implementation_swarm(template: dict):
    """Task must reference @adf:implementation-swarm for handoff."""
    task = template["agents"][0].get("task", "")
    assert "@adf:implementation-swarm" in task, (
        "Task must hand off shaped work to @adf:implementation-swarm"
    )


def test_schedule_present(template: dict):
    """Cron schedule must be defined."""
    assert "schedule" in template["agents"][0], (
        "Schedule is required for Core-layer agents"
    )
