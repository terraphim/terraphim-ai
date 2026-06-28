# Research Document: Native Gitea runner reliability (#2185)

**Status**: Draft
**Author**: ADF orchestration work
**Date**: 2026-06-04
**Reviewers**: Alex (pending)

## Executive Summary

The three reported failures resolve into **two real bugs, one likely
misdiagnosis, and one latent bug**. (1) Stuck push-runs are a real
runner/Gitea version-timing race -- cleanly fixable runner-side. (2) The
"double-fetch" is almost certainly a misdiagnosis: Gitea's task claim is
optimistic-concurrency-guarded, so two runners cannot win the same job; the
observed same-SHA double-checkout was two *distinct* runs (push +
workflow_dispatch). (3) Starvation is a real capacity/scoping gap (terraphim-ai
now uses native-ci -- design R-6 is stale). Plus a latent orphan bug: a
post-fetch `active_repos` skip drops an already-claimed task.

## Essential Questions Check
| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Reliable polyrepo CI underpins the whole #1910 autonomous-maintenance vision. |
| Leverages strengths? | Yes | We own both the runner (Rust) and the Gitea fork (Go) -- can fix either side. |
| Meets real need? | Yes | Stuck runs already blocked landing #2174; starvation throttles polyrepo throughput. |

**Proceed**: Yes (3/3).

## Problem Statement

### Description
The native runner (`terraphim_gitea_runner`, 2 org-scoped instances on bigbox)
exhibits: (1) push-runs that sit "Waiting to run" and are never dispatched
despite idle runners; (2) apparent double-pickup of one task by both
instances; (3) terraphim-ai's CI volume starving polyrepo CI.

### Impact
Polyrepo PRs can wedge (stuck required check) until a manual restart/re-push;
polyrepo CI latency balloons behind terraphim-ai's queue. Both undermine the
enforced verdict gate and autonomous maintenance (#2203).

### Success criteria
- A queued job for any active repo is dispatched within one poll interval of a
  free, matching runner -- no manual restart needed.
- No task is ever assigned to two runners (or proven already impossible).
- Polyrepo jobs are not starved by terraphim-ai volume beyond an agreed bound.
- No task is silently orphaned.

## Current State Analysis

### Code locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Poll loop | `terraphim_gitea_runner/src/poller.rs` | `run_forever`: `tasks_version=0`, loop `poll_once` every `poll_interval` (3s). One task at a time, serial. |
| Repo filter | `poller.rs poll_once` + `config.rs accepts_repo` | Post-fetch `active_repos` allowlist; a non-accepted task is dropped (no UpdateTask). |
| Fetch RPC | `terraphim_gitea_runner/src/client.rs fetch_task` | Thin POST `FetchTask{tasks_version}`; no runner-side claim. |
| Gitea pick | `gitea/services/actions/task.go PickTask` -> `models/actions/task.go CreateTaskForRunner` | Tx; selects `task_id=0 AND status=Waiting` jobs, label-matches, claims via guarded UPDATE. |
| Gitea claim guard | `models/actions/task.go:327` | `UpdateRunJob(job, builder.Eq{"task_id":0})` then `if n != 1 { return nil,false,nil }` -- optimistic concurrency. |
| Gitea fetch gate | `routers/api/actions/runner/runner.go:158` | `if tasksVersion != latestVersion { PickTask }`; `latestVersion = GetTasksVersionByScope(OwnerID, RepoID)`. |
| Version bump | `models/actions/tasks_version.go IncreaseTaskVersion` | Bumps (0,0), (ownerID,0), (0,repoID). Called on **run creation** (`services/actions/run.go`) and job (`run_job.go`). |

### Data flow (current)
runner.run_forever(tasks_version=V) -> POST FetchTask{V} -> Gitea: latest =
TasksVersionByScope(owner, repoID=0 for org runner); if V != latest -> PickTask
(claims oldest Waiting label-matched job org-wide) -> returns {task?, latest}.
Runner caches latest; runs the task serially; sleeps; repeats.

## Root-cause analysis (the vital findings)

### (1) Stuck push-runs -- REAL (version-timing race)
`IncreaseTaskVersion` fires at run **creation**, but the run's job becomes
`StatusWaiting` slightly later (run preparation). Gitea only calls `PickTask`
when the runner's `tasksVersion != latestVersion` (runner.go:158). Sequence
that wedges a run:
1. Run created -> owner-scope version V -> V+1. Job not yet `Waiting`.
2. A runner polls in this window: `V != V+1` -> PickTask -> no Waiting job ->
   returns no task + `latestVersion = V+1`. Runner caches V+1.
3. Job transitions to `Waiting` -- **no new version bump**.
4. Runner now polls with `V+1`; server latest is `V+1` -> `equal` -> **PickTask
   skipped** -> the Waiting job is never offered.
5. Unstuck only by: another run bumping the version, or a runner **restart**
   (runner resets `tasks_version=0`, `0 != V+1` -> PickTask).

This exactly matches the evidence (stuck despite idle; workflow_dispatch
unstuck; restart unstuck; runner resets version to 0 on restart). It is a
Gitea design assumption (version-bump-at-creation as the "unassigned work"
signal) with a race window the runner's version-caching then locks in.

### (2) Double-fetch -- LIKELY MISDIAGNOSIS (Gitea guards it)
`CreateTaskForRunner` runs in a transaction and claims the job with
`UpdateRunJob(..., WHERE task_id = 0)` then bails if `n != 1` (task.go:327-331).
Under READ COMMITTED (Postgres), two concurrent picks of the same Waiting job
serialise on that guarded UPDATE: the first sets `task_id`, the second's UPDATE
matches 0 rows -> returns false -> rolls back its inserted task. **Two runners
cannot both win the same job.** The observed same-SHA double-checkout (14:28)
is fully explained by two *distinct* runs for that SHA -- the stuck push run
**and** the manual `workflow_dispatch` I triggered -- each correctly assigned to
one runner. Needs a confirming spike but is most likely a non-bug.

### (3) Starvation -- REAL (capacity/scoping)
`PickTask` selects the oldest Waiting job org-wide (`Asc("updated","id")`,
task.go:255) across all repos the org runner serves. terraphim-ai now uses
native-ci as its required gate (branch protection requires `native-ci / build
(push)`; it has `.gitea/workflows/native-ci.yml`) -- so **design R-6 ("terraphim-ai
uses GitHub Actions, no Gitea jobs") is STALE**. With an active swarm generating
many terraphim-ai runs/min and only 2 serial instances, the FIFO-ish queue is
dominated by terraphim-ai; isolated polyrepo jobs wait behind it. Not a bug --
a missing capacity/fairness/scoping mechanism.

### (4) Latent orphan bug -- REAL
`poll_once` filters `accepts_repo` **after** `fetch_task`. But FetchTask has
already CLAIMED the job server-side (StatusRunning, RunnerID set, token issued).
Skipping it without an `UpdateTask(state=failure/cancelled)` leaves the job
`Running` assigned to a runner that abandoned it -> orphaned until Gitea's
zombie/timeout sweep. Rare today (label-scoping means the org runner is mostly
offered only its repos), but real if a repo is removed from `active_repos` or
for the proof repo.

## Vital Few (Essentialism)

### Essential constraints (max 3)
| Constraint | Why vital | Evidence |
|------------|-----------|----------|
| A Waiting job must be dispatched without manual intervention | The stuck-run blocked a real merge | #2174 landing |
| No silent orphan / no double-run | Correctness of the gate | task.go guard; skip-drop path |
| Polyrepo jobs not starved unboundedly | Autonomous maintenance depends on it | terraphim-ai flood |

### Eliminated from scope
| Eliminated | Why |
|------------|-----|
| Rewriting Gitea's scheduler | Prefer a runner-side fix; Gitea change is heavier + forks upstream further |
| Firecracker execution (#2076) | Separate; not a reliability fix |
| act_runner migration | Explicitly rejected in the M1 design |
| Multi-task concurrency within an instance | Capacity can be added via instances; intra-instance concurrency is a bigger change |

## Risks and Unknowns

### Assumptions
| Assumption | Basis | Risk if wrong | Verified |
|------------|-------|---------------|----------|
| DB is Postgres, READ COMMITTED | memory: Gitea uses Postgres | If SQLite/serializable, guard still holds (stricter) | Partial |
| Job becomes Waiting after the creation version bump | run.go bumps on create; status transitions during prep | If bump also fires on Waiting, stuck-run theory weakens | No -- spike |
| Sending tasks_version=0 each poll forces PickTask safely | runner.go:158 logic | If 0 has special meaning, could mis-signal | No -- spike |

### Open questions
1. Does a job-status->Waiting transition ever bump the version? (Read run.go / run_job.go state machine.) If yes, the stuck window is smaller.
2. Confirm the 14:28 double-checkout was two distinct run ids (not one task id twice) -- check the Gitea action_task rows / runner logs with run id.
3. Is `tasks_version=0` treated specially anywhere (the `latestVersion==0 -> increase` branch suggests 0 is the "old Gitea" sentinel)?

## Research Findings -- key insights
1. The headline "double-fetch" is almost certainly NOT a bug -- Gitea's guarded claim prevents it. Re-scope #2185 around the stuck-run and starvation.
2. The stuck-run is fixable **entirely runner-side**, no Gitea change: defeat the version optimisation by sending `tasks_version=0` every poll (or a periodic forced sweep), so PickTask always runs and any Waiting job is picked immediately.
3. Starvation needs a capacity/scoping decision, not a bug fix: per-repo-scoped runners (register with RepoID so each only pulls its repo) vs more org instances vs reducing terraphim-ai Gitea volume.
4. The orphan-on-skip is a small, real correctness fix (UpdateTask the skipped task, or drop the post-filter in favour of Gitea scoping).

### Prior art
- Upstream Gitea act_runner: also caches tasks_version; relies on frequent runs + ample runners so the window rarely bites. Our low-volume isolated polyrepo jobs expose it.

## Recommendations
**Proceed to design.** Scope #2185 as:
- **Fix A (stuck-run, runner-side):** force PickTask via `tasks_version=0` poll (or every-Nth-poll sweep). Highest value, lowest risk, no Gitea fork change.
- **Fix B (orphan-on-skip):** report skipped tasks as cancelled/failed, or rely on Gitea repo/label scoping instead of the post-fetch allowlist.
- **Decision C (starvation):** choose capacity vs per-repo-scoped runners vs volume reduction (design phase).
- **Verify (double-fetch):** a spike/test to confirm the misdiagnosis; if confirmed, close that sub-problem with a log-disambiguation note only.

## Next steps (if approved)
1. `disciplined-design`: Fix A + Fix B implementation; Decision C options with a recommendation; the double-fetch verification spike.
