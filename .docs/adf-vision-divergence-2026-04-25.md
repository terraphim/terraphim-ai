# ADF Vision vs Reality: Divergence Analysis

**Date**: 2026-04-25
**Sources**: `~/cto-executive-system/agents/`, `bigbox-deployment/docs/ARCHITECTURE.md`, `blog-post-deploying-ai-dark-factory.md`, `.docs/adf-architecture.md`

---

## Executive Summary

Three meaningful divergences from the original vision. One is a confirmed regression with a known fix (review tier priority). One is a name preserved over a replaced function (upstream-synchronizer). One is a structural gap -- the meta-cortex observation layer -- that was central to the original design and is absent in the current fleet.

Everything else (Python → Rust, 4 GB → 128 GB, 14 agents → 17 agents) is maturation, not drift.

---

## Divergence 1: Review Tier Priority Regression (Confirmed Bug)

**Severity**: High -- silent wrong routing affects all review-tier agents daily.

### What the original design specified

The blog post `blog-post-deploying-ai-dark-factory.md` documents a specific bug fix made on first deployment:

> "We lowered the review tier priority from 60 to 40, below the implementation tier at 50. Now 'review' matches review tier, but implementation tier wins on priority when both match."

The fix was committed in `73849826`. The three-tier priority ordering was: Planning (80) > Implementation (50) > Review (40).

### What the file contains now

```
review_tier.md:        priority:: 60
implementation_tier.md: priority:: 50
planning_tier.md:      priority:: 80
```

**Review tier is 60 -- above implementation at 50 again.** The bug that was diagnosed, documented, and fixed in production has been silently re-introduced.

### Impact

Any agent whose capabilities description contains "review" (e.g. `product-development` has capability `code-review`, `quality-coordinator` has `quality-gate`) can be matched to the review tier at priority 60, pulling it above the implementation tier at 50. This means some implementation agents may route to `review_tier` (anthropic/haiku) instead of `implementation_tier` (anthropic/sonnet). The routing is degraded, not broken -- the agents still run -- but they run on a less capable model than intended.

### Fix

```
docs/taxonomy/routing_scenarios/adf/review_tier.md
```
Change `priority:: 60` to `priority:: 40`.

This is a one-line fix. It should be done immediately and a regression test added: `cargo test` in `terraphim_orchestrator` should have a case asserting review_tier priority < implementation_tier priority.

---

## Divergence 2: upstream-synchronizer -- Name Kept, Function Replaced

**Severity**: Medium -- creates a misleading contract; original function is unimplemented.

### What the original agent did

`~/cto-executive-system/agents/upstream-synchronizer/agent.py`:

```python
"""
Upstream Synchronizer Agent for AI Dark Factory

Keeps forks in sync with upstream repositories,
handles cherry-picks, merge conflict detection.
"""

# Repository configurations
self.repositories = [
    {
        'name': 'gitea-fork',
        'local_path': '/root/gitea',
        'upstream_remote': 'upstream',
        'upstream_url': 'https://github.com/go-gitea/gitea.git',
        ...
    }
]
```

The original was a **fork-sync watchdog**: it monitored `go-gitea/gitea.git`, detected divergence between the terraphim gitea fork and upstream, identified cherry-pick candidates, and alerted on merge conflicts. This was a specific operational need for the gitea fork project.

### What the current agent does

The current `upstream-synchronizer` is an **infrastructure health checker**: disk usage, Docker images, memory state, GitHub Actions runner status, Rust `target/` directory size, `cargo outdated`. It produces `[Infra]` Gitea issues like "#902: target/ at 483 GB" and "#893: All 5 CI runners STOPPED".

These are the responsibilities of the original `runtime-guardian`:

```python
# runtime-guardian/agent.py
"""
Runtime Guardian Agent for AI Dark Factory
Monitors system performance, resource usage, and optimizes runtime.
"""
```

### Why this matters

The name `upstream-synchronizer` creates a false expectation for anyone reading the config. The fork-sync function -- the one that detects whether the terraphim gitea fork has drifted from `go-gitea/gitea.git` -- is **not implemented anywhere in the current fleet**. Given that the gitea fork is how Gitea extensions (the Robot API, PageRank, dependency tracking) are maintained, undetected upstream drift is a real operational risk.

### Recommended action

Two separate things needed:
1. Rename the current infra-health agent to `runtime-guardian` (matching the original taxonomy) or `infra-monitor`. Update conf.d and templates.
2. Create a real `upstream-synchronizer` agent with a task that actually runs `git fetch upstream && git log HEAD..upstream/main --oneline` against the gitea fork repo at `~/gitea` on bigbox.

---

## Divergence 3: The Meta-Cortex Layer Is Missing

**Severity**: Structural -- not a bug, but the absence of a core architectural concept.

### What the original vision specified

The original design had three distinct identity layers:

**Species layer** (`IDENTITY.md`): All agents are *Terraphim* -- "designed for spacecraft, edge devices... exists in superposition... forms meta-cortex with other agents." The meta-cortex is described as the connective tissue: agents learn from each other, not just from their own runs.

**Meta-learning agent** (`agents/meta-learning/agent.py`):
```python
"""
Meta-Learning Agent for AI Dark Factory
Analyzes patterns from all agents, suggests improvements,
learns from cross-agent interactions.
"""
```

**Mneme persona** (defined in `data/personas/mneme.toml`):
> "The memory of the fleet. Meta-cortex with all agents -- Mneme observes, correlates, and advises."
> Symbol: Palimpsest -- overwritten text where earlier writing remains visible.

**BigBox deployment architecture** assigned Meta-Learning 32 GB of RAM -- the largest single allocation -- and listed it as CRITICAL priority alongside Meta-Coordinator.

### What the current fleet has

Seventeen task agents. Each runs, does its work, posts to Gitea, writes a wiki learning page, exits. The `terraphim-agent learn` system captures per-session learnings. But there is no agent that:
- Reads all other agents' run records and exit classifications
- Identifies cross-agent patterns ("compliance-watchdog has been producing empty_success for 6 consecutive days -- something changed")
- Synthesises what the fleet learned this week
- Advises on which agents need their tasks updated

`repo-steward` is the closest substitute: it synthesises recurring Gitea themes. But it works from Gitea issue text, not from agent run records or the learning store. It cannot see that `spec-validator` has been timing out on Tuesdays, or that `test-guardian` consistently produces FAIL on `cargo test` before 02:00 UTC.

### Why this matters

Without a meta-learning layer, the fleet cannot improve itself. Today's ADF session fixes are done manually (this conversation). In the original vision, Mneme would have noticed the `upstream-synchronizer` OOM false-positive pattern after two consecutive runs and proposed the `exit_code=0` classifier fix autonomously.

### Recommended action

This is the highest-value unimplemented feature in the original design. The minimum viable form:

Assign Mneme to a new agent `meta-learning`, scheduled `0 3 * * *` (3am daily, after the overnight run completes). Task: read `/opt/ai-dark-factory/reports/` and the AgentRunRecord store from the last 24 hours, identify recurring patterns (confidence-0.5 classifications, repeated timeout agents, consistently empty_success), and post a synthesis to a single Gitea issue per day: `[ADF] Daily fleet learning report YYYY-MM-DD`. No code changes required -- just a new conf.d entry and task prompt.

---

## What Has NOT Diverged (Maturation, Not Drift)

| Original | Current | Assessment |
|----------|---------|------------|
| Python agents, OpenClaw runtime | Rust orchestrator | Correct maturation. Rust gives better process isolation, resource limits, and no Python interpreter overhead per agent. |
| 4 GB RAM target ("small spaces") | 128 GiB bigbox | Pragmatic. The overnight workload (cargo build, cargo test, cargo audit across 29 crates) needs RAM. The "small spaces" constraint applies to edge/client deployment, not to the build server. |
| 14 agents in original spec | 17 deployed | Focused expansion. Some originals (analyst, critic, executor, scribe) were placeholders without implementations. Current agents are all deployed and producing output. |
| Redis for KG + inter-agent comms | Gitea issues + wiki pages | Reasonable substitution. Redis requires a separate service; Gitea was already the source of truth. The tradeoff is latency (Gitea API calls vs in-memory) for durability and observability. |
| Edge (Kimiko) + BigBox hybrid | BigBox only | Pragmatic. The edge device "Kimiko" is not deployed. Hybrid adds sync complexity. Running everything on bigbox is simpler. Worth revisiting if edge autonomy becomes a requirement. |
| `nightwatch` monitoring loop | Not present | The blog post shows `nightwatch` was required but `enabled = false` was acceptable. It is currently absent from conf.d entirely. Low priority -- the drift-detector partially covers this role. |

---

## Action Items

| Priority | Item | Effort |
|----------|------|--------|
| Immediate | Fix `review_tier` priority from 60 → 40 | 1 line, 5 minutes |
| Soon | Add regression test: review_tier priority < implementation_tier priority | 30 minutes |
| Near-term | Rename current upstream-synchronizer → `runtime-guardian` in conf.d | 1 hour (conf.d edit + orchestrator restart) |
| Near-term | Create actual upstream-synchronizer with gitea fork sync task | 2 hours (new conf.d entry, task prompt) |
| Strategic | Implement `meta-learning` agent (Mneme) reading AgentRunRecords | 1 day (new conf.d entry, task that reads run records) |
