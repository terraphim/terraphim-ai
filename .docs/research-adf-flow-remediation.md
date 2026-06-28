# Research Document: ADF Flow Remediation

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-06-09
**Reviewers**: Alex

## Executive Summary

ADF orchestrator on bigbox is running but the end-to-end ADF lifecycle is not fully closed-loop. The system must reliably progress from issue selection through research, design, implementation, PR review, remediation, re-review, status gates, and merge. Current faults are: (1) review agents run but do not reliably produce orchestrator-parseable verdict comments, (2) `implementation-swarm` can enter a 5-minute no-work spawn loop, (3) some repo branch protection is missing or mismatched, (4) duplicate auto-merge failure issues are generated, and (5) runner/process hygiene needs tightening. Native Gitea CI is present and executing real builds; ADF bash build-runner agents must remain retired.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Hot-loop waste, PR log spam, and broken CI gates are actively harmful |
| Leverages strengths? | Yes | We understand ADF configs and Gitea API |
| Meets real need? | Yes | ~12 spawns/hour burning LLM credits; 200+ duplicate issues; 6 terraform repos unmaintained |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
The ADF orchestrator on bigbox is running but its flow has degraded due to configuration issues introduced during recent refactors. Four distinct problems need remediation.

### Impact
- LLM credits wasted: `implementation-swarm` spawns every 5 min, runs 30s, exits 0 with no work (<changing>12 times/hour)
- Auto-merge pipeline blocked: 9 stale PRs across 3 projects, plus 6 repos with missing branch protection
- Issue tracker spam: ~20 duplicate "Auto-merge failed" issues per PR per day
- 6 terraphim sub-project repos have no agent-driven maintenance

### Success Criteria
1. `implementation-swarm` spawns no more than hourly, only when work exists
2. Branch protection rules exist on all configured terraphim repos
3. Stale PR vetos resolved and duplicate issue cascade stopped
4. Sub-project repos use native Gitea CI plus ADF PR agents; retired ADF bash build-runners remain disabled
5. Review findings are remediated by implementation agents and then re-reviewed until clean or escalated

## Current State Analysis

### Orchestrator Configuration

**Main config** (`/opt/ai-dark-factory/orchestrator.toml`):
- `tick_interval_secs = 30` -- reconcile tick rate
- `eval_interval_secs = 300` -- agent evaluation interval (5 min)
- `gate_reconcile_interval_ticks = 20` -- auto-merge gate every 20 ticks (~10 min)
- `max_dispatches_per_tick = 3` -- max agent spawns per tick
- `max_remediation_attempts = 3` -- auto-merge retries
- Working dir: `/data/projects/terraphim/terraphim-ai`

**Agent configs**: 55 agents across 14 `conf.d/*.toml` files, 0 in main orchestrator.toml.

### Problem 1: implementation-swarm Hot-Loop

**Spawning pattern** (last 3 hours):
```
Time    Spawn  Exit   Wall  Model
19:40   yes    19:40  29.9s openai/gpt-5.4
19:45   yes    19:45  29.9s openai/gpt-5.4
19:50   yes    19:50  30.0s openai/gpt-5.4
19:55   yes    19:55  30.0s openai/gpt-5.4
20:00   yes    20:00  29.6s openai/gpt-5.3-codex
20:35   yes    20:35  29.9s openai/gpt-5.4
21:30   yes    21:30  29.9s openai/gpt-5.4
21:35   yes    21:35  29.9s openai/gpt-5.4
```

**Current config**:
```toml
[[agents]]
name = "implementation-swarm"
schedule = "0 * * * *"          # Intended: once per hour
cli_tool = "/home/alex/.local/bin/pi-rust"
model = "zai-coding-plan/glm-5.1"  # NOT used -- KG router overrides
# MISSING: max_cpu_seconds, max_ticks, grace_period_secs
```

**Root cause hypothesis**: The orchestrator's `eval_interval_secs = 300` evaluates all agents every 5 minutes. When `implementation-swarm` completes in ~30s with exit 0, at the next evaluation the orchestrator finds no running instance and re-dispatches. The `schedule` cron field appears to NOT gate re-dispatch after completion -- it only gates initial scheduling. Without `max_ticks` or `max_cpu_seconds` to enforce a cooldown, the agent is perpetually re-spawned.

**Evidence**: Spawn times (19:40, 19:45, 19:50, 19:55) match `eval_interval_secs = 300` (5 min) pattern, NOT the cron `"0 * * * *"` pattern (which would fire only on the hour).

### Problem 2: Branch Protection Missing

**Projects skipping gate reconciliation**:
| Project | Error | HTTP |
|---------|-------|------|
| atomic-server | Not Found | 404 |
| better-auth-rust | Not Found | 404 |
| digital-twins | Not Found | 404 |
| gitea-robot | Not Found | 404 |
| gitea | Not Found | 404 |
| odilo | Forbidden (zestic-ai repo) | 403 |

All `terraphim/*` repos except `terraphim-ai` are missing branch protection on `main`. The `odilo` repo under `zestic-ai` uses a token without admin write access.

### Problem 3: Stale PR Vetoes and Duplicate Issue Cascade

**Stale PR verdicts** (SHA mismatch, awaiting fresh review):
| Project | PR# | Reviewed SHA | Current Head |
|---------|-----|-------------|--------------|
| terraphim-ai | #2128 | 954ce5be | 95e9c4ef |
| terraphim-ai | #2129 | 4dc7c164 | fedb78e2 |
| terraphim-ai | #2151 | abb6ad9 | 5c33e4da |
| terraphim-ai | #2155 | 0505ed4 | 33c0a41d |
| terraphim-ai | #2156 | 4df24991 | f7298767 |
| terraphim-ai | #2170 | e347e09 | d2f74891 |
| atomic-server | #4 | 6e6cd53 | e1d49845 |
| atomic-server | #24 | 63c553f0 | 8d38efe0 |
| odilo | #318 | e5b8a18 | 9379a57f |

**Review parse failure**: PR #2146 in terraphim-ai has a review comment without the required "Inline Findings" section, causing perpetual parse failure every gate reconciliation.

**Duplicate issue cascade**: Auto-merge failures for PRs #1970, #2056, #2149 are creating 3 duplicate issues every tick (~20 min). Issues #2294-#2313 (20+ issues) are all "Auto-merge failed for PR #X". This is a bug: the auto-merge failure handler lacks idempotency/dedup logic.

**Open PRs on disk** (terraphim-ai): 30 open PRs. Many are `task/NNN-short-title` branches that have been pushed and are awaiting review. The actual stale ones from the logs (#2128, #2129, #2151) may already be closed/merged.

### Problem 4: Sub-Project Agent Coverage and Native CI

6 terraphim sub-project repos have **disabled** ADF `build-runner` agents by design:
- terraphim-agents, terraphim-core, terraphim-clients
- terraphim-config-persistence, terraphim-kg-agents, terraphim-service

Each conf file contains the same disabled build-runner pattern with comment "interim lane retired post native-runner cutover (terraphim-ai #2080)". This is correct and must not be reversed.

**Current native runner status**: `.gitea/workflows/native-ci.yml` exists in `terraphim-ai` and the 6 sub-project repos. `terraphim-gitea-runner` is running on bigbox and has executed real jobs:
- `terraphim-agents@d1271f24...` completed `native-ci` in 45,241ms
- `terraphim-ai@259f0dfc...` completed `native-ci` in 164,530ms

The Gitea UI can show 0s for cancelled runs (for example actions run #17864), but journal logs confirm real cargo execution for successful runs.

### Problem 5: Missing Closed-Loop ADF Flow

The prior plan treated individual gates separately. The actual target state is a lifecycle loop:

1. `implementation-swarm` selects one ready Gitea issue.
2. It performs disciplined research and disciplined design before implementation.
3. It implements on a task branch and opens a PR.
4. Native Gitea CI runs `.gitea/workflows/native-ci.yml` and posts `native-ci / build (push)`.
5. ADF PR agents (`pr-reviewer`, `pr-validator`, `pr-verifier`) review the PR and post parseable verdict comments and statuses.
6. If findings exist, remediation is dispatched back to implementation.
7. The implementation agent fixes review comments on the same branch.
8. Review agents re-review the updated head SHA.
9. The loop repeats until verdicts are clean or the attempt budget is exhausted.
10. Auto-merge merges only when native CI and all required ADF verdict statuses are green and fresh for the current head SHA.

## Constraints

### Technical Constraints
- ADF orchestrator source changes are now in scope because #2301 cannot be fixed reliably with config alone
- Config and binary changes require restart: `systemctl restart adf-orchestrator`
- Gitea API token (`5d663...`) has owner-level access to `terraphim/*` repos
- `zestic-ai/odilo` token lacks admin write (403 on branch protection)
- All 6 sub-project repos exist on disk at `/data/projects/terraphim/<name>`

### Business Constraints
- `implementation-swarm` uses expensive LLM models (gpt-5.4, gpt-5.3-codex)
- Cannot increase test timeouts
- Must use British English for all documentation

## Vital Few (Essential Constraints)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Stop LLM credit waste immediately | ~12 spawns/hour x gpt-5.4 = significant cost | Hot-loop observed in logs |
| Branch protection on terraphim repos | Auto-merge gate silently broken | 404 errors every 10 min |
| Stop duplicate issue creation | Tracker being flooded | 20+ dupes in last day |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Fix odilo branch protection (403) | zestic-ai org, different token scope |
| Modify orchestrator binary code | Config-only fix preferred |
| Full sub-project CI pipeline | Use native-ci runner; ADF handles review/merge only |
| Archive/close all stale PRs manually | Too many; use systematic approach |

## Dependencies

### Internal
| Dependency | Impact | Risk |
|------------|--------|------|
| orchestrator binary restart | 30s downtime | Low |
| conf.d/*.toml file format | Must match orchestrator schema | Med |
| Gitea API | Branch protection, issue management | Low |

### External
| Dependency | Version | Risk |
|------------|---------|------|
| Gitea instance | git.terraphim.cloud | Low (running) |
| Native Gitea runner | `terraphim-gitea-runner` on bigbox | Medium (running, but duplicate runner processes need cleanup) |

## Research Findings

### Finding 1: No cooldown mechanism in orchestrator for core agents
The orchestrator's `eval_interval_secs = 300` controls how often agents are evaluated, but there is no per-agent cooldown field (like `min_re_spawn_secs` or `sleep_after_exit_secs`). The `schedule` field appears to be a cron gate for FIRST spawn, not for RE-spawn after normal exit. The `max_ticks` field limits how many reconcile ticks the agent stays alive, but `implementation-swarm` has no `max_ticks` set, making it eligible for infinite re-spawn.

### Finding 2: Auto-merge failure handler has no dedup
Issues #2294-#2313 are created by the auto-merge flow every time reconciliation runs. Each failure for the same PR creates a new issue instead of updating an existing one. The ADF monitor bots (security-sentinel, meta-coordinator) also contribute to this with their "Recurrence" comment patterns.

### Finding 3: Branch protection API returns 404 for repos with no rules
The Gitea API returns 404 Not Found (not 200 empty) when no branch protection rules exist. The auto-merge code treats this as an error and skips the repo entirely.

### Finding 4: Native Gitea runner is the build path
A custom `terraphim-gitea-runner` binary exists at `/home/alex/.local/bin/terraphim-gitea-runner` and is running. It executes `.gitea/workflows/native-ci.yml` with label `terraphim-native`. The correct required build context for `terraphim-ai` is currently `native-ci / build (push)`, not the retired ADF bash build-runner path. Multiple long-lived runner processes exist, so cleanup is needed, but the runner itself is real and performs cargo work.

### Finding 5: Verdict posting is the real PR-gate prerequisite
Issue #2301 identifies the core PR-gate failure: review agents produce useful output but do not reliably post parseable verdict comments with `Confidence Score: N/5`, `Inline Findings`, and `Last reviewed commit: <head>`. Auto-merge, remediation, and stale-review detection depend on those parseable verdicts. Fixing branch protection or stale PRs without fixing verdict posting does not close the loop.

## Open Questions
1. Does the orchestrator support a `min_re_spawn_secs` or equivalent cooldown field? (Needs source code check)
2. Are PRs #2128, #2129, #2151 still open or were they closed? (Not in open PR list; may be closed)
3. Should sub-project repos get their own ADF agents or share terraphim-ai's? (Architecture decision)

## Recommendations

### Proceed/No-Proceed
**Proceed** with all four remediation areas. Each is independently fixable via config changes and Gitea API calls.

### Fix 1: Stop implementation-swarm hot-loop
**Recommendation**: Add `max_ticks = 1` and `grace_period_secs = 3600` to the agent config to enforce a 1-hour cooldown between spawns. If orchestrator doesn't support cooldown, add a file-based mutex lock in the task script itself.

### Fix 2: Branch protection
**Recommendation**: Create or align branch protection rules on `terraphim/*` repos via Gitea API, requiring the contexts those repos actually emit. For `terraphim-ai`, that means `native-ci / build (push)`, `adf/pr-reviewer`, `adf/validation`, and `adf/verification`.

### Fix 3: PR vetos and duplicate issues
**Recommendation**: (a) Close duplicate auto-merge failure issues with a batch comment, (b) re-trigger reviews on stale PRs by commenting `@adf:pr-reviewer re-review` on each, (c) verify PR #2146 review comment format and fix or close.

### Fix 4: Native CI and sub-project coverage
**Recommendation**: Keep retired ADF bash build-runners disabled. Use `terraphim-gitea-runner` and `.gitea/workflows/native-ci.yml` for build/test. Confirm `pr-reviewer`, `pr-validator`, and `pr-verifier` cover sub-projects through `extra_projects`; add missing `extra_projects` only if verification shows gaps.

### Fix 5: Closed-loop remediation
**Recommendation**: Treat each PR as a loop: review -> findings -> remediation dispatch -> fixes -> re-review -> merge. The orchestrator must track review attempt count by PR head SHA and only stop when verdicts are clean, not merely when an agent exits successfully.
