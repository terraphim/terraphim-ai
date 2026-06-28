"""Tests for the meta-coordinator health-check in terraphim.toml.bigbox.

Validates the fix for issue #3537: the disk-usage and orphaned-worktree
alerts must carry a Theme-ID dedup guard so repeated cron runs comment on
an existing open alert instead of spawning duplicate issues.

These are static contract tests on the real config file -- no Gitea API
calls, no mocks. They assert the canonical dedup pattern already used by
security-sentinel / repo-steward / drift-detector / meta-learning cron
agents is also present in the meta-coordinator health-check.
"""

from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore[no-redef]

BIGBOX = Path(__file__).parent.parent.parent.parent / ".terraphim" / "terraphim.toml.bigbox"


def _load_meta_coordinator_task() -> str:
    """Return the meta-coordinator agent's task string from the bigbox config."""
    assert BIGBOX.exists(), f"Missing bigbox config: {BIGBOX}"
    with open(BIGBOX, "rb") as fh:
        config = tomllib.load(fh)
    agents = config.get("agents", [])
    for agent in agents:
        if agent.get("name") == "meta-coordinator":
            return agent["task"]
    raise AssertionError("No meta-coordinator agent found in bigbox config")


def test_bigbox_toml_parses():
    """The bigbox config must be valid TOML (regression guard for edits)."""
    with open(BIGBOX, "rb") as fh:
        config = tomllib.load(fh)
    assert "agents" in config and config["agents"], "bigbox has no agents"


def test_disk_usage_block_has_dedup_guard():
    """Section 3 must check for an existing open alert before create-issue.

    Without this guard the cron run creates a new issue every tick where
    worktree usage exceeds 10GB -- the root cause of the #3537 spam.
    """
    task = _load_meta_coordinator_task()
    assert "WORKTREE_SIZE" in task, "disk-usage measurement removed"
    assert "DISK_EXISTING=" in task, (
        "disk-usage block lost its dedup guard (DISK_EXISTING)"
    )
    # The guard must look for the same Theme-ID it emits.
    assert "Theme-ID: adf-health-alert" in task
    assert '"Worktree disk usage" in i.get("title","")' in task, (
        "dedup guard must match on the alert title, not just the Theme-ID"
    )


def test_disk_usage_block_comments_on_existing_alert():
    """When an open alert exists, the run must comment a recurrence, not create."""
    task = _load_meta_coordinator_task()
    assert 'gtr comment --owner terraphim --repo terraphim-ai --index "$DISK_EXISTING"' in task, (
        "disk-usage block must comment on the existing alert index"
    )
    assert "Recurrence" in task, "recurrence comment must be labelled"


def test_disk_usage_block_still_creates_when_no_duplicate():
    """The create-issue path must remain for the first occurrence of the alert."""
    task = _load_meta_coordinator_task()
    assert 'gtr create-issue' in task, "create-issue path removed entirely"
    assert '"[ADF] Worktree disk usage: $WORKTREE_SIZE MB"' in task, (
        "disk-usage create-issue title template changed"
    )


def test_orphan_block_detects_unregistered_worktrees():
    """Section 5 must detect dirs on disk that git no longer registers.

    This is the true orphan signal (the prior version only counted total
    dirs and echoed at >10, which missed stale-but-registered worktrees
    and flagged healthy multi-agent states).
    """
    task = _load_meta_coordinator_task()
    assert "ORPHAN_COUNT" in task, "orphan count variable removed"
    assert "comm -23" in task, (
        "orphan detection must diff filesystem vs git admin registry (comm -23)"
    )
    assert ".git/worktrees" in task, "must reference git's admin registry path"


def test_orphan_block_has_dedup_guard():
    """The orphan alert must also carry the Theme-ID dedup guard."""
    task = _load_meta_coordinator_task()
    assert "ORPHAN_EXISTING=" in task, (
        "orphan block lost its dedup guard (ORPHAN_EXISTING)"
    )
    assert '"orphaned worktree" in i.get("title","").lower()' in task, (
        "orphan dedup guard must match on the alert title (case-insensitive)"
    )


def test_orphan_block_comments_on_existing_alert():
    task = _load_meta_coordinator_task()
    assert 'gtr comment --owner terraphim --repo terraphim-ai --index "$ORPHAN_EXISTING"' in task, (
        "orphan block must comment on the existing alert index"
    )


def test_orphan_block_offers_safe_reclaim_path():
    """The alert body must point operators at the existing sweep script, not raw rm."""
    task = _load_meta_coordinator_task()
    assert "scripts/adf-setup/adf-cleanup.sh" in task, (
        "orphan alert must reference the manifest-guarded adf-cleanup.sh"
    )


def test_meta_coordinator_task_is_valid_bash(tmp_path, monkeypatch):
    """The embedded task must be syntactically valid bash.

    Guards against TOML-escape corruption (e.g. bare backslashes inside
    the multi-line basic string) that would break the cron run.
    """
    import shutil
    import subprocess

    bash = shutil.which("bash")
    if bash is None:
        # Non-bash CI host: skip rather than guess at the shell.
        import pytest

        pytest.skip("bash not available on this host")

    task = _load_meta_coordinator_task()
    script = tmp_path / "meta-coord-task.sh"
    script.write_text(task)
    result = subprocess.run(
        [bash, "-n", str(script)], capture_output=True, text=True
    )
    assert result.returncode == 0, (
        f"meta-coordinator task is not valid bash:\n{result.stderr}"
    )
