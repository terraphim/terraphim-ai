# Research Document: ADF PR Gate Reconciliation (Gitea #1122)

**Status**: Draft
**Date**: 2026-05-01
**Related Issues**: #1122, #1066, #1092
**Evidence PR**: #1099

## Executive Summary

ADF PRs get stuck because the orchestrator writes `pending` commit statuses but never reads them back. Branch protection requires `adf/build` and `adf/pr-reviewer`, but if an agent crashes before posting its final status, the PR stays blocked indefinitely with no remediation. The fix requires a canonical gate reconciler that reads statuses, classifies PR state, and takes deterministic action.

## Problem Statement

### Description
PR #1099 is `mergeable: true` at git level, has accumulated 45 review comments, but cannot progress because:
1. Head commit `9c287d68` lacks both required status-check contexts (`adf/build`, `adf/pr-reviewer`)
2. ADF logs report `confidence 3/5 below auto-merge threshold 5/5` repeatedly
3. No deterministic remediation task is created
4. The `adf-orchestrator.service` was observed inactive on bigbox

### Impact
- All agent-authored PRs can get permanently stuck
- Human intervention required to manually toggle branch protection
- ADF agents spin, burning LLM tokens without advancing PRs
- No audit trail explaining why a PR is blocked

### Success Criteria (from issue #1122)
1. ADF can report required context state for every open PR head
2. Missing required contexts enqueue the correct agent
3. `adf/build` and `adf/pr-reviewer` are posted on PR head SHAs
4. Status-post failures become `FactoryFault` or blocking issues, not silent
5. PRs with all green contexts proceed to auto-merge evaluation
6. Low-confidence PRs create one clear remediation path
7. Duplicate remediation issues are not created
8. PR #1099 reaches a deterministic final state

## Current State Analysis

### Architecture: PR Lifecycle (ROC v1)

```
Step A: PR opened/pushed (webhook or poll)
Step B: Fan-out agents_on_pr_open (pr-reviewer, build-runner, etc.)
Step C: Post pending commit status for each agent context
Step D: Agent runs, posts output as PR comment
        *** Agent posts final commit status via bash/curl ***
Step E: (implicit) Agent exits
Step F: poll_pending_reviews reads comments, evaluates verdict
Step G: If verdict passes -> handle_auto_merge
Step H: Post-merge: cargo test --workspace, revert on failure
```

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| PR dispatch | `pr_dispatch.rs` (209 lines) | Builds review tasks, PR env vars |
| PR poller | `pr_poller.rs` (443 lines) | Polls comments, evaluates verdicts |
| PR review | `pr_review.rs` (311 lines) | Pure verdict parser + merge criteria |
| Post-merge gate | `post_merge_gate.rs` (641 lines) | cargo test + git revert |
| Config | `config.rs:228-251` | `PrDispatchEntry` with name + context |
| Pending status post | `lib.rs:2436-2497` | Posts `pending` for each agent context |
| Fan-out handler | `lib.rs:1959-2013` | Spawns agents + posts pending |
| Auto-merge | `lib.rs:4104-4341` | Re-check, merge, open issue on failure |
| Poll reviews | `lib.rs:3897-4102` | Reads comments, enqueues auto-merge |
| Reconcile tick | `lib.rs:5138-5350` | Main loop, 18 steps |
| Tracker API | `terraphim_tracker/src/gitea.rs` | `set_commit_status` only, no read |
| Output poster | `output_poster.rs` (524 lines) | Posts agent output as comments |
| Project control | `project_control.rs` (288 lines) | Pause flags + circuit breaker |

### Data Flow: Current (Broken)

```
PR opened -> Fan-out agents -> Post pending status
                                |
                                v
                        Agent runs (external process)
                                |
                    +-----------+-----------+
                    |                       |
                Agent succeeds          Agent crashes
                    |                       |
            Posts success via curl    Status stays "pending" forever
                    |                       |
            Poller reads comments     PR stuck under branch protection
                    |                  No remediation, no timeout
            Evaluates verdict
                    |
            If confidence >= 5/5
            AND p0=0, p1=0, all criteria met
                    |
            Auto-merge
```

### Integration Points

| API | Used For | Gap |
|-----|----------|-----|
| `POST /repos/{owner}/{repo}/statuses/{sha}` | Write commit status | No read-back |
| `GET /repos/{owner}/{repo}/statuses/{sha}` | Read commit statuses | **Not implemented** |
| `GET /repos/{owner}/{repo}/branch_protections/{branch}` | Read required contexts | **Not implemented** |
| `GET /repos/{owner}/{repo}/pulls/{number}` | Read PR mergeable state | Partial (only lists) |
| `POST /repos/{owner}/{repo}/issues` | Create remediation issues | Exists, no dedup by gate key |

## Constraints

### Vital Constraints (Max 3)

| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Must not break existing reconcile_tick ordering | 18-step loop is delicate, other agents depend on it | lib.rs:5138-5350 |
| Must be backward compatible with existing PrDispatchEntry config | All deployed configs use `agents_on_pr_open` | config.rs:228 |
| Must work with existing tracker crate API | `GiteaTracker::set_commit_status` is the only write path | tracker/gitea.rs:1070 |

### Eliminated from Scope

| Item | Why Eliminated |
|------|---------------|
| Lowering auto-merge confidence threshold | Explicitly out of scope per issue |
| Replacing Gitea Actions workflows | Out of scope per issue |
| Making advisory contexts required | Status production not yet deterministic |
| Configurable AutoMergeCriteria via TOML | Separate concern, pre-existing gap |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_tracker` | Must add `list_commit_statuses` + `get_branch_protection` APIs | Low (straightforward HTTP) |
| `terraphim_orchestrator` config types | PrDispatchEntry defines expected contexts | Low (read-only) |
| `terraphim_orchestrator` dispatcher | Must enqueue remediation agents | Medium (priority scoring) |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Gitea commit status API | v1.22+ | Low | None needed |
| Gitea branch protection API | v1.22+ | Low | None needed |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Gitea API rate limits on status reads | Low | Medium | Batch per reconcile tick, not per PR |
| Agent bash scripts already post statuses | Medium | High | Reconciler must handle duplicate posts gracefully |
| Existing pending statuses from crashed runs | High | Medium | Timeout sweep: pending > N mins -> error |
| AutoMergeCriteria hard-coded to defaults | Low | Medium | Out of scope, but reconciler should expose it |

### Open Questions

1. Should the reconciler run every tick or on a separate interval? -- Every tick adds overhead; separate interval is cleaner
2. What timeout for stale pending statuses? -- 30 minutes is reasonable for most builds
3. Should FactoryFault issues auto-close when the blocker resolves? -- Yes, needs dedup key

### Assumptions

| Assumption | Basis | Risk if Wrong |
|------------|-------|---------------|
| Gitea `/statuses/{sha}` returns all contexts for a SHA | Standard Gitea API | Low - well-documented |
| Branch protection contexts match `PrDispatchEntry.context` values | Config convention | Medium - no validation exists |
| Agent task scripts post final statuses via curl | Observed in fixture configs | Medium - could be fragile |

## Research Findings

### Key Insights

1. **No commit status read-back exists** -- the orchestrator is blind to whether agents actually posted their final status
2. **No branch protection API** -- required contexts are configured out-of-band with no programmatic verification
3. **The poller only reads comments, not statuses** -- Step F is purely comment-based, unaware of the branch protection gate
4. **Stale pending statuses have no timeout** -- if an agent crashes, its pending status blocks the PR forever
5. **Remediation issues have no dedup** -- repeated ticks create duplicate issues (partially mitigated by `AutoMergeDedupeSet` for merge attempts)
6. **`AutoMergeCriteria` is hard-coded** -- always uses defaults (5/5 confidence, 0 P0, 0 P1, all criteria)
7. **Agent scripts post statuses via raw curl** -- the orchestrator has no visibility into whether this succeeded

### Relevant Prior Art

- `post_merge_gate.rs`: well-structured pure function + trait pattern that the reconciler should follow
- `pr_review.rs`: pure function module with zero I/O -- same pattern for the reconciler
- `project_control.rs`: circuit breaker + pause flag pattern for service faults
