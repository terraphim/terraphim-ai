# Implementation Plan: Native Gitea runner reliability (#2185)

**Status**: Draft
**Research Doc**: `.docs/research-2185-native-runner-reliability.md`
**Author**: ADF orchestration work
**Date**: 2026-06-04
**Estimated Effort**: ~0.5 day (code) + ~0.5 day (verify spike + ops rollout decision)

## Overview

### Summary
Fix the two real native-runner bugs with surgical, runner-side changes, verify
the double-fetch is a non-issue, and make a capacity/scoping decision for
starvation. No Gitea fork change.

### Approach
- **Fix A (stuck runs):** stop caching `tasks_version`; poll with `0` every
  iteration so Gitea always runs `PickTask` and any Waiting job is dispatched
  within one poll interval.
- **Fix B (orphan-on-skip):** when a fetched task's repo is not in
  `active_repos`, report it cancelled via `UpdateTask` (it was already claimed
  server-side) instead of silently dropping it.
- **Decision C (starvation):** operational, not a code bug -- recommend
  short-term capacity + medium-term label-tier isolation; documented below.
- **Verify (double-fetch):** an integration test against the fake Gitea server
  proving one Waiting job is handed to exactly one of two concurrent pollers;
  add the run/task id to runner logs to disambiguate distinct same-SHA runs.

### Scope
**In scope:** poller.rs Fix A + Fix B; the verification test + log line; the
Decision C recommendation (ops doc, no code).
**Out of scope:** Gitea scheduler changes; intra-instance task concurrency;
Firecracker (#2076); act_runner.
**Avoid at all cost (5/25):**
- Editing the Gitea fork's FetchTask/version logic (heavier; forks upstream further; runner-side fix suffices).
- A configurable "sweep cadence" knob (speculative; always-0 is simplest and lowest-latency).
- Intra-instance worker pool / async multi-task (big change; capacity via instances instead).
- "Fixing" double-fetch (it is not broken).
- Rewriting active_repos into Gitea label-scoping in this PR (operational; Decision C).

## Architecture

### Data flow (after Fix A)
```
run_forever: loop { poll_once(state, 0); sleep(interval) }   # always 0, not carried
poll_once: fetch_task(state, 0) -> Gitea: 0 != latestVersion -> PickTask ALWAYS
  -> task? -> if !accepts_repo(repo): update_task(cancelled) [Fix B]; return
            -> else: TaskWorker.run(task)
```

### Key design decisions
| Decision | Rationale | Alternatives rejected |
|----------|-----------|-----------------------|
| Always send `tasks_version=0` | Forces PickTask each poll -> no stuck-run race; PickTask cost is negligible (~40 q/min for 2 runners @3s) | Periodic sweep (reintroduces latency + a knob); fix Gitea (heavier) |
| Report skipped task cancelled | The task is already claimed (StatusRunning, RunnerID) on fetch; releasing it prevents orphan-until-timeout | Silent drop (current bug); pre-filter before fetch (impossible -- claim happens in fetch) |
| Starvation = ops decision, not code | The pick order is Gitea-side FIFO; code can't fairly reorder without server change | Build a fairness scheduler in the runner (large; speculative) |

### Eliminated options
| Rejected | Why | Risk if included |
|----------|-----|------------------|
| Gitea-side fix (bump version when job->Waiting, or FOR UPDATE) | Runner-side `tasks_version=0` fully fixes it without touching the fork | Fork drift; deploy risk on the whole Gitea instance |
| Per-repo-scoped runners now | Decision C; needs label/runs-on changes per repo -- separate operational change | Scope creep into this code PR |

### Simplicity check
**What if this could be easy?** It is: Fix A is a one-line change (pass `0`),
Fix B is ~5 lines (call `update_task` on the skip path). The starvation
"decision" is config/ops, not code. Nothing speculative.
**Senior-engineer test:** not overcomplicated -- two tiny, well-targeted edits.
**Nothing-speculative checklist:** [x] no new features [x] no knobs [x] no new
abstractions [x] no premature optimisation.

## File Changes

### Modified
| File | Change |
|------|--------|
| `crates/terraphim_gitea_runner/src/poller.rs` | Fix A: `run_forever` calls `self.poll_once(state, 0).await` each loop (drop the carried `tasks_version`); doc-comment the rationale. Fix B: in `poll_once`, on the `!accepts_repo` branch, call `self.client.update_task(state, cancel_request(task.id))` before returning; log `run/task id`. Add a small `fn cancel_request(task_id) -> UpdateTaskRequest`. |
| `crates/terraphim_gitea_runner/src/poller.rs` (logs) | When a task is fetched, `log::info!` includes `task.id` (disambiguates same-SHA runs -- double-fetch verification aid). |

### New
| File | Purpose |
|------|---------|
| `crates/terraphim_gitea_runner/tests/poller_reliability.rs` | Integration tests against a fake Gitea axum server (no mocks): stuck-run-now-picked; skip-reports-cancel; two concurrent pollers, one job, exactly one wins. |

No new deps (axum/tokio already used by the M1 fake-server tests).

## API Design

```rust
// poller.rs -- Fix A (behaviour change, signature unchanged)
pub async fn run_forever(&self, state: &RunnerState) -> Result<()> {
    loop {
        // Always poll with version 0 so Gitea's FetchTask runs PickTask every
        // time (it gates PickTask on `tasks_version != latestVersion`; the
        // version is bumped at run creation, before the job is Waiting, so a
        // cached version can permanently mask a now-Waiting job -- #2185).
        if let Err(e) = self.poll_once(state, 0).await {
            log::error!("poll error: {e}");
        }
        tokio::time::sleep(self.config.poll_interval).await;
    }
}

// poller.rs -- Fix B (skip path)
if !self.config.accepts_repo(name) {
    log::info!("releasing task {} for repo `{name}` (not in active_repos)", task.id);
    // The task was already claimed by FetchTask; cancel it so Gitea does not
    // leave it Running/orphaned until the zombie timeout.
    let _ = self.client.update_task(state, cancel_request(task.id)).await;
    return Ok(resp.tasks_version);
}

/// Minimal UpdateTask payload marking a task cancelled (result code per the
/// Connect protocol: 1=success, 2=failure, 3=cancelled).
fn cancel_request(task_id: i64) -> UpdateTaskRequest { /* TaskState{ id: task_id, result: Cancelled, .. } */ }
```
(Confirm the cancelled result code against `types.rs TaskResult` during impl; types already model a cancelled state.)

## Test Strategy (no mocks; real fake-Gitea axum server)

| Test | Purpose |
|------|---------|
| `stuck_waiting_job_is_picked_after_version_cache` | Fake server: first FetchTask bumps+returns version V with no task (simulates the race window); a job becomes Waiting; assert the next `poll_once(_, 0)` (Fix A) still receives + runs it. With the old carried-version behaviour it would NOT. |
| `skipped_repo_task_is_reported_not_orphaned` | Fake server hands a task for a repo not in `active_repos`; assert the runner calls `UpdateTask(cancelled)` for that `task.id` and runs nothing. |
| `two_pollers_one_job_single_winner` | Fake server models the guarded claim (hand the job to the first FetchTask, return empty to the second); assert exactly one poller runs it -- documents/locks the double-fetch non-issue at the runner boundary. |
| existing M1 tests | Must stay green (register/declare/fetch/execute/logs). |

E2E (post-merge, bigbox): create a polyrepo push run while both runners are idle; assert it dispatches within one poll interval with NO restart (the original stuck-run repro).

## Implementation Steps
1. **Fix A** in `poller.rs` `run_forever` (pass `0`); doc-comment. Test: `stuck_waiting_job_is_picked_after_version_cache`.
2. **Fix B** in `poll_once` skip path + `cancel_request` helper (verify cancelled code in `types.rs`). Test: `skipped_repo_task_is_reported_not_orphaned`.
3. **Log** `task.id` on fetch. Test: `two_pollers_one_job_single_winner`.
4. Quality gate (crate builds independently): `cargo fmt --all -- --check`, `cargo clippy -- -D warnings`, `cargo test -p terraphim_gitea_runner`. PR -> gate -> merge -> rebuild/redeploy both runner instances (systemd --user restart).
5. **Verify spike** (bigbox, post-deploy): the stuck-run E2E above; confirm double-fetch does not recur with the run-id logging.

## Decision C -- starvation (operational recommendation; NOT in the code PR)
Recommended rollout, in order:
1. **Short term:** add 1-2 more org runner instances. With Fix A they are immediately effective (each free runner picks the oldest Waiting job). Cheapest; reduces queue wait. Does not isolate polyrepos from terraphim-ai volume.
2. **Medium term (isolation):** split by **runs-on label tier** -- terraphim-ai's `native-ci.yml` uses `runs-on: terraphim-native-ai`; polyrepos keep `runs-on: terraphim-native`. Register a dedicated runner for the `-ai` label and keep >=1 for `terraphim-native`. Gitea label-matches at PickTask, so polyrepo jobs never queue behind terraphim-ai. Avoids the active_repos post-filter entirely (so Fix B's skip path stops being exercised). Cost: per-repo `runs-on` edit + runner registration.
3. **Rejected:** per-individual-repo runners (too many to manage); moving terraphim-ai off native-ci (it deliberately adopted native-ci as its gate -- R-6 is stale).

Decision needed from operator: accept (1) now and schedule (2), or jump to (2).

## Rollback
- Code: revert the poller PR; rebuild/redeploy. (Fix A/B are isolated to poller.rs.)
- If Fix A's always-PickTask measurably loads Gitea (it will not at this scale), reintroduce a large sweep interval as a follow-up.
- Decision C steps are config/registration -- reversible by reverting labels + deregistering runners.

## Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `tasks_version=0` mis-signals (old-Gitea sentinel) | Low | Med | runner.go increments latest to >=1 then `0 != latest` -> PickTask; covered by the stuck-run test |
| `cancel_request` wrong result code | Low | Low | verify against types.rs TaskResult; test asserts UpdateTask called |
| Fix A increases Gitea DB load | Low | Low | ~40 PickTask q/min for 2 runners; negligible; sweep fallback documented |
| Double-fetch is real after all | Low | Med | the two-poller test + run-id logging will catch it; Gitea guard analysed (task.go:327) |

## Open Items
| Item | Status |
|------|--------|
| Confirm cancelled result code in types.rs TaskResult | impl step 2 |
| Operator decision on Decision C (1 vs 2) | pending |
| Confirm a job->Waiting transition never bumps the version (shrinks the race but Fix A covers it regardless) | optional spike |

## Approval
- [ ] Technical review
- [ ] Decision C choice (capacity now vs label-tier isolation)
- [ ] Human approval
