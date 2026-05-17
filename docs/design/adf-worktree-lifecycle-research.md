# ADF Worktree Lifecycle Hardening -- Research Document

Gitea epic: `terraphim/terraphim-ai#1567`
Phase: 1 of disciplined development (Research)
Author: claude-opus-4-7 (research agent)
Date: 2026-05-17

> This document concludes Phase 1. No code is changed; no design is committed.
> The four sibling issues (#1562 Layer 0, #1569 Layer 1, #1570 Layer 2,
> #1571 Layer 3) are referenced as scope anchors only.

---

## 1. Problem framing and storm timeline

### 1.1 What happened on bigbox

On 2026-05-17 (UTC) the `adf-orchestrator.service` host on bigbox entered a
self-reinforcing fire loop in which the compound review schedule fired
roughly every 90 seconds, spawning a fresh six-agent review swarm before the
previous one had a chance to post a verdict.

Observed end-state at intervention:

- Memory: 99.9 GiB / 100 GiB cgroup limit (OOM imminent).
- CPU: roughly four cores pinned for more than three hours.
- Active tasks in the service cgroup: 2 413.
- Leaked git worktrees on disk: 692, occupying 48 GiB under
  `/data/projects/terraphim/terraphim-ai/.worktrees/review-*`.
- Compound review issue #514 silent: the last verdict comment was timestamped
  `2026-05-13T10:01:45Z`; no posts appeared during the storm window.

Log signature in `journalctl -u adf-orchestrator`:

```
... compound review schedule fired, starting review
... reconcile_tick exceeded timeout, forcing continuation
... compound review schedule fired, starting review
... reconcile_tick exceeded timeout, forcing continuation
... (repeating ~every 90 s for >3 h)
```

Each `compound review schedule fired` line corresponds to a brand-new
`review-<uuid>` worktree created on disk, followed by six spawned agent
subprocesses, followed by tokio cancellation of the parent future before
either the cleanup line or the verdict-posting code ran. The 90 s cadence
matches the reconcile timeout: `Duration::from_secs(tick_interval_secs.max(30) * 3)`
at `crates/terraphim_orchestrator/src/lib.rs:1245` with the default
`tick_interval_secs = 30` (`crates/terraphim_orchestrator/src/config.rs:1096-1098`).

### 1.2 The two distinct root causes

Two bugs compound into the storm. They are tracked as separate issues so
that mitigation can land independently and so that each can be reasoned about
in isolation.

#### Cause A -- schedule cursor not advanced on cancellation (#1562, Layer 0)

The cron check at `crates/terraphim_orchestrator/src/lib.rs:7148-7152` reads:

```rust
let should_fire = compound_sched
    .after(&self.last_tick_time)
    .take_while(|t| *t <= now)
    .next()
    .is_some();
```

The cursor `self.last_tick_time` is only updated at step 14 of
`reconcile_tick`, at `crates/terraphim_orchestrator/src/lib.rs:5687-5689`:

```rust
// 14. Update last_tick_time and increment tick counter
self.last_tick_time = chrono::Utc::now();
self.tick_count = self.tick_count.wrapping_add(1);
```

The tokio safety wrapper at `crates/terraphim_orchestrator/src/lib.rs:1288-1296`
cancels `reconcile_tick` after `reconcile_timeout` (90 s by default) and
*continues the outer loop*. Because cancellation drops the future before line
5687 is reached, `last_tick_time` retains the value set by the previous
successful tick, so on the next iteration the same past cron occurrence
satisfies `should_fire` again. Result: the schedule re-fires every tick.

Proposed mitigation (in the body of #1562) is to advance the cursor inside
the fire branch, before the long-running `.await` on `handle_schedule_event`,
so it survives cancellation. This is documented here as **Layer 0** but the
detailed design is owned by #1562.

#### Cause B -- worktree cleanup is a line after `await`, not a `Drop` (this work)

The compound-review entry point creates a worktree at
`crates/terraphim_orchestrator/src/compound.rs:298-304`:

```rust
let worktree_name = format!("review-{}", correlation_id);
let worktree_path = self
    .worktree_manager
    .create_worktree(&worktree_name, git_ref)
    .await
    .map_err(|e| {
        OrchestratorError::CompoundReviewFailed(format!("failed to create worktree: {}", e))
    })?;
```

Cleanup occurs at `crates/terraphim_orchestrator/src/compound.rs:367-370`,
after the collection loop has either drained the channel or hit the
collection deadline:

```rust
// Cleanup worktree
if let Err(e) = self.worktree_manager.remove_worktree(&worktree_name).await {
    warn!(error = %e, "failed to cleanup worktree");
}
```

When the outer future `CompoundReviewWorkflow::run` is dropped at any of the
intermediate `.await` points -- and there are many: `create_worktree`,
`get_changed_files`, the inner `tokio::time::timeout_at` in the collection
loop -- the cleanup line is never reached. There is no `Drop` impl to run as
a fall-back. Cause A turns this from a once-per-storm leak into a
once-per-90s leak, which is how 692 worktrees accumulated in three hours.

### 1.3 Scope of this research

This document focuses on **Cause B**. It treats Cause A as a sequencing
prerequisite (because without #1562, even a perfect Layer 1 still creates
and tears down worktrees on every fire) and surfaces the layers that make
the cleanup robust under cancellation, panic, SIGKILL, and root-owned
artefacts.

### 1.4 Four-layer scope as established by the epic

| Layer | Issue | Purpose | Runs as |
|------|------|---------|---------|
| 0 | #1562 | Schedule cursor advances inside the fire branch before `.await`. | orchestrator process |
| 1 | #1569 | RAII guard so worktree cleanup runs from `Drop` on every exit path including future cancellation. | orchestrator process |
| 2 | #1570 | `WorktreeManager::sweep_stale` invoked from `Orchestrator::new` to clean residue from SIGKILL / OOM / panic-across-runtime. | orchestrator process (user `alex`) |
| 3 | #1571 | `adf-cleanup.sh` deployed as `ExecStartPre` to sweep stale `review-*` and `*-sentinel-*` worktrees as root, handling root-owned files Layers 1 and 2 cannot. | systemd, as root |

Layers 1, 2, 3 are defence-in-depth: each handles a failure mode the layer
above cannot. Layer 0 is upstream of all three; it reduces blast radius and
buys time-to-detect for any Layer 1 regression.

---

## 2. Existing implementation map

This section cites file paths and line numbers; it does not propose code.

### 2.1 `WorktreeManager` -- `crates/terraphim_orchestrator/src/scope.rs`

The manager is a thin wrapper around the `git worktree` CLI.

- `WorktreeManager` struct: `crates/terraphim_orchestrator/src/scope.rs:206-210`.
- Constructors `new` and `with_base`: `:216-242`. `with_base` resolves a
  relative `worktree_base` against `repo_path` to avoid CWD-dependent
  behaviour (see commit `622a78e88`).
- `create_worktree` (async): `:260-301`. Shells out to
  `git -C <repo> worktree add <path> <ref>`, with `GIT_INDEX_FILE` removed
  from the environment to defend against pre-commit-hook contamination of
  the index lock. Returns `io::Error` on git failure with stderr included.
- `remove_worktree` (async): `:306-356`. Shells out to
  `git -C <repo> worktree remove <path>`; falls back to
  `git worktree remove --force` on first failure. Also attempts to remove
  the empty parent directory, ignoring the error.
- `cleanup_all` (async): `:361-375`. Walks `list_worktrees()` and calls
  `remove_worktree` on each. Returns the count of successful removals.
- `list_worktrees` (sync): `:380-402`. Reads the worktree base directory and
  returns names of subdirectories that contain a `.git` file or directory.
  **Crucially, it does not consult `git worktree list` or
  `.git/worktrees/<name>` registry entries.** Thus an empty dir without
  `.git` is invisible, as is a stale registry entry whose worktree dir was
  manually removed by hand.
- `worktree_exists` (sync): `:405-407`. Checks whether
  `<base>/<name>/.git` exists.

Test coverage is thorough for the happy path -- `test_create_worktree`,
`test_remove_worktree`, `test_remove_nonexistent_worktree`,
`test_cleanup_all`, `test_list_worktrees`, `test_worktree_exists`,
`test_create_duplicate_worktree_fails` -- all in
`crates/terraphim_orchestrator/src/scope.rs:609-786`. There is no test that
exercises a partially-created worktree (dir present, `.git` missing) or a
registry-only orphan (no dir, `.git/worktrees/<name>` present).

### 2.2 `WorktreeGuard` already exists -- `crates/terraphim_orchestrator/src/worktree_guard.rs`

The `WorktreeGuard` module landed in commit `f324183e4` ("phase 1 - Exit
Classifier, worktree guard, ES bulk ingest") and is currently used **only**
for the per-agent worktree path in `lib.rs`, not for the compound-review
worktree in `compound.rs`. The full file is 183 lines.

Key facts:

- Struct: `crates/terraphim_orchestrator/src/worktree_guard.rs:25-29`.
  Holds a `PathBuf` and a `bool should_cleanup`.
- `keep` (disarm): `:48-51`. Sets `should_cleanup = false`. Takes `self` by
  value, which is fine for the existing caller pattern but worth noting --
  the guard becomes unusable after `keep()`.
- `cleanup` (private): `:59-81`. Calls **`std::fs::remove_dir_all`**, not
  `git worktree remove`. This is the most consequential finding in the
  existing implementation: the worktree directory is wiped, but the git
  internal registry under `.git/worktrees/<name>` is not. `git worktree
  prune` is required to reconcile.
- `Drop`: `:84-88`. Calls `cleanup()` synchronously from `Drop`.
- Helpers `with_worktree_guard` and `with_worktree_guard_async`:
  `:93-111`. The async variant takes a future by value but does **not**
  thread the guard into it; the guard is created in the outer scope and
  dropped when the helper returns. A cancellation of the outer future at
  the await still drops the guard via stack unwinding -- the guarantee is
  the standard one.

Tests exist for cleanup, `keep`, already-removed, and
`with_worktree_guard` -- `:113-182`. There is no test of cancellation
behaviour, nor of git registry residue.

### 2.3 Compound review entry point -- `crates/terraphim_orchestrator/src/compound.rs`

`CompoundReviewWorkflow` (struct `:231-235`) owns a `SwarmConfig` and a
`WorktreeManager`. The constructor `from_compound_config` at `:248-250`
builds the manager via `WorktreeManager::with_base(&config.repo_path,
&config.worktree_root)`.

`run` is the entry point -- `:261-402`. Step by step:

1. `:266-275` Generate `correlation_id`, log start.
2. `:277-294` Call `get_changed_files` (`:424-461`, shells out to
   `git diff --name-only base_ref git_ref`). Filter groups by
   `visual_only`.
3. `:296-304` Create worktree `review-<correlation_id>`. If this await is
   cancelled the worktree either never materialises or its creation error
   propagates out of `run` before any sender exists, so this single await
   is not in itself a leak source.
4. `:306-307` Channel for agent outputs.
5. `:309-332` Spawn each agent via `tokio::spawn`. Each task receives a
   cloned `tx`, a cloned `worktree_path`, a cloned `changed_files`, and
   a cloned `group`. `spawned_count` is incremented in the loop body.
   **The `JoinHandle` returned by `tokio::spawn` is dropped immediately.**
   This is critical for risk analysis (see 3.3).
6. `:335` `drop(tx)`. The outer holder of `tx` is dropped so that, when
   all spawned senders eventually go away, the receiver returns
   `Ok(None)` and the loop exits.
7. `:336-365` Collect with a deadline of
   `timeout + Duration::from_secs(10)`. The `tokio::time::timeout_at`
   returns `Err` on deadline, which logs `"collection deadline exceeded,
   using partial results"` and breaks the loop.
8. `:367-370` Cleanup. This is the line that does not run on cancellation.
9. `:373-401` Deduplicate findings, compute pass/fail, return result.

Per-agent execution lives in `run_single_agent` (`:476-554`). It builds a
`tokio::process::Command` with `current_dir(worktree_path)` (`:523`), runs
it under `tokio::time::timeout` (`:537`), and returns
`AgentResult::Success` or `AgentResult::Failed`. The timeout cancels the
`cmd.output()` future and the `Child` is dropped, but `kill_on_drop` is
not currently set on the `Command` (see `compound.rs:490-528`), so the
subprocess is orphaned and continues running until it exits of its own
accord. This is a hidden cost of the current code path and a key
constraint for any Phase 2 design (see 3.3).

### 2.4 Reconcile-loop wrapper -- `crates/terraphim_orchestrator/src/lib.rs:1280-1300`

The reconciliation loop body, with the 90 s safety wrapper:

```rust
Ok(LoopEvent::Tick) => {
    loop { /* drain coalesced events */ }
    match tokio::time::timeout(reconcile_timeout, self.reconcile_tick()).await {
        Ok(()) => {}
        Err(_) => {
            warn!(
                timeout_secs = reconcile_timeout.as_secs(),
                "reconcile_tick exceeded timeout, forcing continuation"
            );
        }
    }
}
```

`reconcile_timeout` is constructed at `:1245`:

```rust
let reconcile_timeout = Duration::from_secs(self.config.tick_interval_secs.max(30) * 3);
```

With `tick_interval_secs = 30`, the timeout is 90 s. The storm cadence
matches exactly.

### 2.5 Cron schedule check -- `crates/terraphim_orchestrator/src/lib.rs:7098-7162`

This is the call site that fires the storm. The relevant chunks:

- `:7099-7100` Read `now = Utc::now()`.
- `:7104-7125` Per-agent cron filter: skip if `active_agents.contains_key(name)`,
  compute `next_fire` using `schedule.after(&self.last_tick_time).next()`,
  short-circuit if `next_fire > now`, additionally short-circuit if
  `last_cron_fire[name] >= next_fire`. The last protection is what saves
  the per-agent cron path from the same bug -- it keeps an explicit
  per-agent fire cursor independent of `last_tick_time`.
- `:7127-7134` Fire loop. `last_cron_fire.insert(name, fire_time)` happens
  **before** `spawn_agent` is awaited, which is the pattern Layer 0 needs
  to replicate for compound review.
- `:7137-7161` Compound review check. Unlike the per-agent loop, it has no
  per-event cursor -- it reads `compound_sched.after(&self.last_tick_time)`
  and fires if that yields any time `<= now`. The "skip if active" check
  is absent.

The schedule type comes from the [`cron`](https://crates.io/crates/cron)
crate, used via `crates/terraphim_orchestrator/src/scheduler.rs:3` (`use
cron::Schedule;`). Iterator `Schedule::after(&datetime).take(n)` returns
the next `n` occurrences strictly after `datetime`. The cron expression
"runs nightly at 02:00" decomposes to occurrences spaced 24 h apart -- so
once a single past occurrence is on the wrong side of the cursor, every
subsequent tick (until the cursor advances past it) re-detects it.

### 2.6 Other call sites of `create_worktree`

Two distinct lifecycle paths exist:

1. **Compound-review path** (`crates/terraphim_orchestrator/src/compound.rs:298-304`).
   Worktree dir under `<config.worktree_root>/review-<full-uuid>`.
   Default `worktree_root` on bigbox is
   `/data/projects/terraphim/terraphim-ai/.worktrees/`.
2. **Per-agent path** (`crates/terraphim_orchestrator/src/lib.rs:5392-5438`
   defines `create_agent_worktree`; called at `:2086`). Worktree dir under
   `/tmp/adf-worktrees/<agent_name>-<8char-uuid>` (the root is hardcoded
   at `lib.rs:5393`). The corresponding `WorktreeGuard` is stored on
   `ManagedAgent::worktree_guard` (`lib.rs:182`) and disarmed on success
   at `lib.rs:6694-6697`.

The per-agent path already has guard coverage; the compound-review path
does not. Note also that the per-agent path produces worktree directory
names whose prefix is the agent's own name -- so a `pr-security-sentinel`
spawn produces `/tmp/adf-worktrees/pr-security-sentinel-<8>`. This is the
likely origin of the "sentinel-*" pattern in the user's brief: it is not a
separate code path; it is the set of `/tmp/adf-worktrees/*sentinel*`
directories.

### 2.7 Runtime mode

`#[tokio::main]` at `crates/terraphim_orchestrator/src/bin/adf.rs:246`
selects the **multi-threaded** runtime (the default). This affects the
viability of synchronous blocking inside `Drop` (see 3.2).

---

## 3. Constraints and risks

### 3.1 Tokio cancellation semantics

When an async function is dropped at an await point, the following hold:

- Stack-allocated values that the future owns are dropped in reverse
  construction order, including any `Drop` impls.
- `tokio::spawn`ed background tasks are **not** cancelled. Their
  `JoinHandle`s become "abandoned" but the tasks continue executing on the
  runtime until they complete or panic.
- A `tokio::process::Child` held in the cancelled future is dropped, but
  this **does not** terminate the subprocess. `kill_on_drop` is `false`
  by default on `tokio::process::Command`; the documented behaviour
  matches the standard library's `std::process::Child`. The child
  process continues running, detached from any parent handle, until it
  exits on its own.
- Subprocess termination therefore requires either explicit
  `kill_on_drop(true)` on the `Command` at spawn time, an explicit
  `.kill().await` before `Child::drop`, or a unix process-group approach
  (`setpgid` + `killpg`). The current `compound.rs:490-528` builder does
  none of these.

This means a `Drop` impl on `CompoundReviewWorkflow::run`'s state -- or
more practically on a guard owned by that state -- *will* fire when the
reconcile-loop wrapper cancels the future at 90 s. The remaining question
is what such a `Drop` is allowed to do; see 3.2.

Async `Drop` is unstable (still in nightly-only RFC state per the project's
stable-Rust constraint). Layer 1 must therefore use a synchronous `Drop`.

### 3.2 Sync `Drop` calling blocking git CLI -- the tradeoff matrix

Layer 1's `Drop` needs to remove a git worktree. Four implementation
strategies are worth enumerating in Phase 2; this document does not
choose between them.

| Strategy | Mechanism | Throughput cost | Failure modes |
|----------|-----------|----------------|---------------|
| A. `std::process::Command::new("git").status()` | Blocks the calling thread until `git worktree remove` returns. | On the multi-thread runtime, parks one worker for the duration; with 692 leaked trees, a startup sweep could park the whole pool. Per-Drop is small (single git invocation). On `current_thread` runtime, would deadlock. | git failure is observable (non-zero exit). |
| B. `tokio::task::block_in_place` | Tells the multi-thread runtime to move the current task elsewhere, then blocks; the runtime keeps making progress. | Same blocking work but does not starve the scheduler. Not available on `current_thread` runtime; would panic. | Same as A. |
| C. Detached cleanup queue | `Drop` sends a `PathBuf` to a long-lived `mpsc::UnboundedSender`; a dedicated worker drains it and runs `git worktree remove`. | Non-blocking inside `Drop`. Adds a moving part. Messages can be lost on process shutdown unless drained explicitly. | git failure is observed in the worker, not the calling task; need separate visibility. |
| D. Fire-and-forget `tokio::spawn` from `Drop` | `Drop` builds the command and spawns it. | Inherits "tokio::spawn from sync context needs a handle" -- only works if a runtime handle is in scope (via `tokio::runtime::Handle::try_current`). On shutdown the handle may be gone. | git failure is logged in the spawned task. |

The existing `WorktreeGuard::cleanup` at
`crates/terraphim_orchestrator/src/worktree_guard.rs:69` already uses a
relative of strategy A -- **synchronous filesystem ops**
(`std::fs::remove_dir_all`) -- so the precedent for "we block inside
`Drop`" is established. The filesystem variant differs from `git worktree
remove` in two ways:

- It does not invoke git, so it cannot reconcile `.git/worktrees/<name>`.
- It is bounded by inode count rather than git's internal locking, so
  long durations are rare.

A purely-filesystem cleanup in `Drop`, accompanied by a separate
`git worktree prune` in Layer 2's sweep, is one design point. Phase 2 must
weigh whether the registry residue between Drop and the next sweep is
acceptable (it is read-only metadata; git refuses to create a new worktree
at the same path, but new worktrees use uuids).

### 3.3 The detached `tokio::spawn` race -- highest risk to address in Phase 2

`crates/terraphim_orchestrator/src/compound.rs:319-330` spawns each agent
in a detached task:

```rust
tokio::spawn(async move {
    let result = run_single_agent( ... ).await;
    let _ = tx.send(result).await;
});
```

The `JoinHandle` is dropped at the end of the spawn statement. Consequences
when the outer `run` future is cancelled by the 90 s wrapper:

- Spawned tasks **continue executing**. Their inner
  `tokio::time::timeout(timeout, cmd.output())` at `:537` only cancels the
  child after `self.config.timeout` elapses; default
  `max_duration_secs = 1800` (30 min) gives an upper bound of 30 minutes
  during which up to six child processes are still running.
- Each running task still holds a clone of `worktree_path`, and its
  subprocess has `current_dir(worktree_path)` (`:523`). It may write into
  the worktree at any moment.
- If Layer 1's `Drop` deletes `worktree_path` while a spawned task is
  mid-write, the agent's subprocess will see file-not-found errors or, on
  some filesystems, may continue writing into a now-orphaned inode.
- The spawned task will then `tx.send(...).await` on a closed channel,
  log a `_ = ...` discarded error, and exit. No findings will be
  collected. The verdict for that swarm is lost.
- The agent subprocess itself continues running for up to
  `max_duration_secs` (30 minutes by default), holding CPU and memory
  in the orchestrator cgroup. This is the immediate consequence of
  3.1's `kill_on_drop = false` finding and is the direct mechanism by
  which the bigbox 2 413 task-count and 4-core CPU pin were reached:
  agent subprocesses from earlier review fires were still alive when
  the next fire spawned six more.

This is the **primary correctness concern** for Phase 2. Options to
enumerate (without choosing):

1. **`abort()` the join handles** held in a vector that `Drop` walks.
   Aborts each task at its next await point. By itself this does **not**
   kill the underlying subprocess (see 3.1); it merely cancels the
   wrapping tokio task and drops the `Child`. For this option to clean
   up subprocesses, the spawn site must additionally set
   `kill_on_drop(true)` on the `Command`.
2. **Propagate a `CancellationToken`** ([`tokio_util::sync::CancellationToken`])
   into `SwarmConfig` and into each spawned task. Each task selects on its
   work future vs. the token, and on receiving the cancel signal calls
   `child.kill().await` before exiting. More plumbing but cooperative
   and explicit about subprocess termination.
3. **Accept the race**. Rely on `git worktree remove --force` atomicity
   (it does `mv` + `rm`) and on agents being tolerant of disappearing
   CWDs. Highest risk; documented for completeness. Note that this
   option does not address the runaway-subprocess load problem at all.
4. **Hold the spawn handles in the guard.** When the guard is dropped,
   it `abort`s the handles, awaits a short grace period to allow
   subprocess teardown, then runs `git worktree remove --force`. This
   is a refinement of (1) and is the most defensible default, but
   again requires `kill_on_drop(true)` (or explicit `.kill()`) at the
   spawn site for the subprocess to actually die.

In all four cases, Layer 1's `Drop` must not depend on cooperative agent
shutdown for correctness; the cleanup must succeed even when agents are
hostile. **Phase 2 must explicitly plumb `kill_on_drop(true)` into the
spawn site at `compound.rs:490` or commit to an alternative kill
mechanism**; relying on drop semantics alone is incorrect under tokio's
documented default.

### 3.4 File-ownership constraint -- why Layer 3 must exist

On bigbox the post-incident inspection showed mixed file ownership inside
the leaked `review-*` worktrees:

- Source-tree files owned by `alex` (the orchestrator service user).
- `target/` artefacts owned by `alex` or `sccache`, depending on whether
  `RUSTC_WRAPPER=sccache` was active (`lib.rs:670-679` enables it when
  `sccache --version` succeeds at orchestrator startup).
- Sub-process workspaces and container artefacts owned by `root`. The
  exact origin is project-specific (some agents elevate into container
  builds, some shell out to docker), but the empirical observation is
  that `find /data/.../worktrees/ -uid 0` returns non-zero results.

Implication: Layer 1's `Drop` and Layer 2's sweep, both running as `alex`,
**cannot** remove root-owned files. `git worktree remove --force` will
fail with `Operation not permitted` partway through. The worktree directory
will remain on disk with a partial structure.

Layer 3 (`adf-cleanup.sh` as `ExecStartPre`) runs in the systemd unit
context with root privileges. It can recursively delete entries under
`<worktree_root>/review-*` (using `find ... -mindepth 1 -delete` or
equivalent) and then `git worktree prune`. This is the only layer that
closes the root-owned residue gap.

Two design choices that follow from this constraint and need Phase 2
attention:

- Layer 1 and Layer 2 should treat `EACCES` / `EPERM` as a **warning, not
  an error**. The orchestrator should log and move on; Layer 3 will clean
  up on the next service restart.
- Layer 3's source-of-truth path matters. It must live under version
  control so the deployed `ExecStartPre` line never drifts from the
  cleanup logic. See open question 5.1.

### 3.5 Race conditions in detail

Beyond the spawn race (3.3), several other races deserve enumeration.

#### 3.5.1 Sweep vs. live review

If `WorktreeManager::sweep_stale` runs in `Orchestrator::new` and the
process restarts during an in-flight compound review (e.g. via a deploy),
sweep will find one `review-*` directory whose owner is the just-killed
process. Sweep claims it as stale and deletes it.

Mitigation options:

- **Age threshold**: only sweep worktrees older than a configurable
  duration (e.g. 2 x `max_duration_secs`). Caveat: a fresh restart after
  an OOM will leave a worktree younger than the threshold, which sweep
  then skips -- defeats the purpose.
- **Process-liveness probe**: read the orchestrator's previous pidfile;
  if the previous instance is dead, sweep. Adds a pidfile.
- **Sweep before scheduler starts**: ensure sweep finishes before the
  scheduler's tick thread is spawned. This is sufficient because the
  scheduler is the only path that creates new `review-*` trees.

#### 3.5.2 Sweep vs. per-agent worktrees in `/tmp/adf-worktrees/`

If Layer 2 also sweeps `/tmp/adf-worktrees/*`, it must skip agents that
are still listed in `active_agents`. Phase 2 must decide whether sweep
scope is `review-*` only, or `review-*` plus `/tmp/adf-worktrees/*`. See
open question 5.2.

#### 3.5.3 Multiple orchestrator instances

The schema does not prevent two orchestrator processes from sharing the
same `worktree_root` -- e.g. during a hot-restart with `Type=notify` or
during a botched migration. Layer 2 has no lockfile. If two instances
sweep concurrently, both will see the same trees; the second sweep will
return "remove failed" warnings but is otherwise idempotent.

This is not a correctness bug but it is worth flagging because Layer 3
runs as `ExecStartPre`, and `ExecStartPre` runs **before** the main
`ExecStart`, so any overlap is bounded by the unit's restart semantics.

### 3.6 Idempotency requirements

Both `WorktreeManager::sweep_stale` and `adf-cleanup.sh` must be safe to
invoke when there is nothing to clean. Concretely:

- Empty `worktree_base` (no directory at all) is not an error.
- Empty `worktree_base/` is not an error.
- `git worktree prune` on a repo with no registry entries is a no-op.
- `find ... -mindepth 1 -delete` on an empty directory is a no-op.

Layer 1's `Drop`-fired cleanup is **inherently** idempotent because
`Drop` only runs once per guard instance; the concern there is correctness
under repeated process-level invocation, which is addressed by Layers 2
and 3.

### 3.7 `WorktreeGuard`'s existing gap -- registry residue

As noted in 2.2, the existing `WorktreeGuard` uses
`std::fs::remove_dir_all` rather than `git worktree remove`. This means
that today, even on the per-agent path that is supposedly already guarded:

- An agent panic or OOM removes the worktree dir.
- The git registry entry `.git/worktrees/<agent_name>-<uuid>` is not
  cleaned up.
- Over time the repo's `.git/worktrees/` directory grows.
- `git worktree list` shows zombie entries; `git worktree add` at the
  same path fails with "already exists" until `prune` runs.

The user has not reported this as a visible problem -- presumably because
agent worktrees use `uuid::Uuid::new_v4()` for uniqueness, so the
"already exists" path is never hit. But the registry size is a slow leak
and Phase 2 should address it.

### 3.8 Tracing and observability of cleanup

`scope.rs` already emits structured tracing on create/remove:

- `:272-277` `info!` on `create_worktree` entry.
- `:299` `info!` on success.
- `:292` `error!` on `git worktree add` failure.
- `:314` `info!` on `remove_worktree` entry.
- `:341` `error!` on `git worktree remove` failure.
- `:354` `info!` on success.

`worktree_guard.rs` emits at lower verbosity:

- `:38` `debug!` on guard creation.
- `:50` `debug!` on `keep()`.
- `:65` `debug!` on "already removed".
- `:71` `info!` on cleanup success.
- `:74` `warn!` on filesystem-remove failure.

`compound.rs:367-370` emits `warn!` on cleanup failure. There is no
`info!` on cleanup *success* in the compound path, and no metric tracking
the worktree backlog. Phase 2 should add at minimum:

- A counter or gauge of `worktrees_swept_total` / `worktrees_active`
  exposed to the Quickwit `adf-logs` index.
- A `warn!` log line if `sweep_stale` finds more than N residual
  worktrees at startup (a smoking gun for a recent crash storm).

---

## 4. Operational and observability requirements

### 4.1 What gets logged today

Cataloguing every `tracing::` call in the worktree lifecycle (see 3.8 for
the full list). The summary picture:

- **Create**: visible. `info!` with `repo_path`, `worktree_path`,
  `git_ref`.
- **Normal removal**: visible. `info!` with name and path.
- **Forced removal**: visible. The second invocation of `git worktree
  remove --force` is logged at `info!` on success and `error!` on
  failure.
- **Drop-fired cleanup**: visible but only at `debug` / `info`. `warn!`
  on filesystem failure.
- **Storm conditions**: no aggregate-level signal. The 692 leaked
  worktrees on bigbox were discovered by `du -sh` after the fact, not
  by an alert.

### 4.2 Required new signals for Phase 2

Three minimum additions:

1. **Backlog gauge**. On every `sweep_stale` invocation, emit
   `worktree_backlog{kind="review"} = N` and
   `worktree_backlog{kind="agent"} = M`. Quickwit's `adf-logs` index can
   alert when either exceeds a threshold (e.g. 10).
2. **Sweep summary event**. After `sweep_stale` finishes, emit one
   `info!` line with `swept_count`, `failed_count`, `duration_ms`,
   `root_owned_skipped`, etc. This lets storm postmortems compute "how
   long was the backlog growing" by reading sweep logs alone.
3. **Drop-fired cleanup event**. Each Layer 1 `Drop` cleanup should emit
   a single structured event including the worktree name, the agent (if
   bound), the cancellation reason if known, and the success status.
   Today's `info!` "worktree cleaned up" suffices on the happy path; the
   `warn!` on failure should also include the partial-progress state
   (which step failed: `git worktree remove`, filesystem delete, or
   `git worktree prune`).

### 4.3 Storm-recovery runbook -- current vs. target

**Today's recovery** requires a human to:

1. Stop the service: `sudo systemctl stop adf-orchestrator`.
2. Manually delete the residual worktree directories under
   `/data/.../worktrees/review-*` using elevated privileges.
3. Reconcile git's registry: `cd /data/.../terraphim-ai && git worktree prune`.
4. Sweep the per-agent root at `/tmp/adf-worktrees/` (typically
   `sudo find /tmp/adf-worktrees/ -mindepth 1 -delete`).
5. Restart: `sudo systemctl start adf-orchestrator`.

**Target (post-Layer-3) recovery**: a single `sudo systemctl restart
adf-orchestrator`. The unit's `ExecStartPre=/opt/adf/bin/adf-cleanup.sh`
runs as root before the main binary; the main binary's
`Orchestrator::new` calls `sweep_stale` for any residue not in scope of
the cleanup script.

Acceptance criterion for the epic: a manually-introduced bunch of stale
`review-*` directories is fully reclaimed by a single
`systemctl restart`, with no human-typed cleanup commands.

### 4.4 Quickwit `adf-logs` integration

The orchestrator already streams structured tracing into Quickwit via
`crates/terraphim_orchestrator/src/quickwit.rs` (573 lines; full review
out of scope here). Adding new fields to existing `info!` lines is
backwards-compatible. Adding a new event type (e.g.
`worktree_sweep_completed`) requires a Quickwit dashboard update but no
schema migration -- Quickwit's index is schema-on-read for non-mapped
fields.

---

## 5. Assumptions and open questions

Open questions identified during research. Phase 2 should resolve each
before committing to a design.

### 5.1 Where should the `adf-cleanup.sh` source of truth live?

The script is deployed to `/opt/ai-dark-factory/bin/adf-cleanup.sh` (path
inferred from the existing `adf-orchestrator.service`'s working directory
convention; not in tree). The repository today has no `deploy/`,
`systemd/`, or `ops/` directory (`ls` of these returns "No such file").
Three plausible locations:

- `scripts/adf-cleanup.sh` -- consistent with other operational scripts
  under `scripts/ci/`, `scripts/adf-setup/`, etc.
- `scripts/adf-setup/cleanup.sh` -- consistent with the existing
  `scripts/adf-setup/agents/` and `scripts/adf-setup/scripts/`
  directories.
- A new `deploy/systemd/adf-orchestrator/adf-cleanup.sh` alongside a
  fragment of the unit file -- captures the deployment intent in tree.

**Assumption**: the script lives at `scripts/adf-cleanup.sh` until the
ops team specifies otherwise. The deployed copy must be a literal copy
of this file (or a symlink during dev), with no in-place edits on the
host.

### 5.2 Sweep scope: `review-*` only, or also `*-sentinel-*`?

Section 2.6 establishes that the per-agent path produces worktrees at
`/tmp/adf-worktrees/<agent_name>-<8>`, so any agent whose name contains
`sentinel` (e.g. `pr-security-sentinel`, `security-sentinel`) yields a
"sentinel" path on disk. The brief mentions `sentinel-*` as a separate
prefix but no separate code path produces it.

Options:

A. **Compound only**. Sweep `<worktree_root>/review-*` only. Per-agent
   trees in `/tmp/adf-worktrees/` are left to the existing per-agent
   `WorktreeGuard` and to systemd's `RuntimeDirectory` cleanup. Smaller
   blast radius; relies on the per-agent guard already being correct.
B. **Compound + per-agent**. Sweep `<worktree_root>/review-*` AND
   `/tmp/adf-worktrees/*`. Two roots, similar logic. Catches the
   per-agent registry residue noted in 3.7.
C. **All `.worktrees/`**. Sweep everything under
   `<worktree_root>/`, including hypothetical future prefixes. Most
   defensible against drift but risks deleting in-use trees from
   other tools (no known case but cannot be ruled out).

**Assumption**: option B for Layer 2 (compound + per-agent), restricted
to `review-*` and entries created by `create_agent_worktree`. Layer 3's
shell-script equivalent should match. Documented as an open question
because the brief specified `review-*` and `sentinel-*` explicitly,
which is option B by intent but uses a different prefix vocabulary.

### 5.3 Is `WorktreeGuard::disarm` (currently `keep()`) needed?

The current callers either keep or drop the guard at deterministic
points. The disarm pattern is useful when:

- The agent succeeds and we want to inspect its worktree out-of-band.
- The agent's output is being post-processed by another tokio task
  that owns the worktree.

The current code path in `lib.rs:6690-6697` disarms the guard on agent
success ("Disarm worktree guard on success so it doesn't conflict with
the explicit cleanup below") and then calls `remove_agent_worktree`
explicitly. The disarm is essentially "use the explicit cleanup instead
of the implicit one because we want to await the result". For the
compound-review path, no out-of-band inspection currently happens, so
the guard always fires.

**Assumption**: a `disarm` (or `keep`) method is **kept** for symmetry
with the existing module and for future use, but is not exercised by
the compound-review path.

### 5.4 Must sweep run before any agent dispatch, or just before the first cron check?

Layer 2 inserts `sweep_stale` somewhere in `Orchestrator::new` or in
`Orchestrator::run`. The choice matters because:

- `Orchestrator::new` is `pub fn new(config: OrchestratorConfig) ->
  Result<Self, OrchestratorError>` -- synchronous. Calling an async
  sweep here requires `tokio::runtime::Handle::current().block_on(...)`
  or a `pollster`-style helper. Adds a runtime-coupling.
- `Orchestrator::run` is async (the long-running event loop). Sweeping
  at the start of `run`, before the tick thread is spawned, is
  natural and async-friendly.

**Assumption**: sweep runs at the start of `run`, before the tick
thread is spawned at `crates/terraphim_orchestrator/src/lib.rs:1218`.
The first cron check (compound or per-agent) cannot complete a fire
before sweep finishes.

### 5.5 Upper bound on worktree age for the sweep claim

Section 3.5.1 raised the conflict between sweep and a hypothetical
running review. Phase 2 must specify:

- Either an age threshold (e.g. `> max_duration_secs * 2`), or
- A liveness probe via pidfile, or
- An ordering guarantee (sweep before any new spawn) that obviates
  the threshold.

**Assumption**: ordering guarantee per 5.4 is sufficient; no age
threshold is required because sweep is one-shot at startup.

### 5.6 Layer 3 in-tree shape

Open: does `adf-cleanup.sh` belong with the unit file fragment, or
standalone? The repository has no unit-file fragments today. Two
non-binding options:

- Standalone `scripts/adf-cleanup.sh` + a documentation snippet for
  the systemd `ExecStartPre=` line in `docs/operations/adf.md`.
- A fully self-contained `deploy/systemd/adf-orchestrator/`
  directory holding both the unit and the script, that the ops team
  copies to `/etc/systemd/system/`.

**Assumption**: standalone script + docs snippet. Reduces churn against
the current `scripts/adf-setup/` convention.

### 5.7 Should Layer 1's `Drop` invoke `git worktree remove`, or just delete the directory?

Section 3.2's tradeoff table presented four strategies. The follow-up
question is which of them aligns with the existing precedent.

**Today**: `WorktreeGuard` uses `std::fs::remove_dir_all` (strategy A in
filesystem form). It does **not** invoke git, so `.git/worktrees/<name>`
remains.

**Possible Phase 2 direction**: keep `Drop` filesystem-only (fast,
already-precedented), but have Layer 2's `sweep_stale` always invoke
`git worktree prune` before walking directories. This pushes the
registry reconciliation to the slower sweep path and keeps the hot path
fast.

**Open**: whether the hot-path performance matters in practice (each
`git worktree remove` is sub-second; with 692 trees that is roughly 12
minutes of CPU). Phase 2 needs an empirical measurement, not a guess.

### 5.8 Cancellation propagation contract for spawned agents

Section 3.3's spawn race needs a Phase 2 decision: do we abort the
handles, propagate a token, or accept the race?

**Open**. No assumption is recorded; this is the single most consequential
design decision in #1569.

---

## 6. Cross-cutting concerns with #1562

### 6.1 Why Layer 0 limits Layer 1's blast radius

If #1569 ships *without* #1562, an unrelated regression that drops a
guard prematurely will leak one worktree per 90 s while the storm runs.
With #1562 in place, the same regression leaks one worktree per
scheduled fire window (24 h for the default nightly cron), giving an
operator 200-fold more time to notice the alert before disk fills up.

If #1562 ships *without* #1569, the cursor fix prevents the storm from
firing, but any *single* OOM or panic during a normal nightly review
still leaks. The bleed rate drops from "one per 90 s" to "one per crash"
-- typically one or two per week, which is below the threshold a human
would notice for months. This is in fact the regime the system was in
before 2026-05-17 (the 13 leaked trees from earlier dates that
co-existed with the storm's 692). #1569 is the long-tail fix.

### 6.2 Sequencing dependency

Both fixes can land independently. **Recommended order**:

1. Land #1562 first (small, well-scoped, contained in `lib.rs`).
2. Land #1569 second (touches `compound.rs`, `worktree_guard.rs`, and
   adds tests for cancellation).
3. Land #1570 third (touches `scope.rs`, adds `sweep_stale`).
4. Land #1571 last (out-of-tree systemd integration; can be
   simulated in tests but only takes full effect on bigbox).

Rationale: each layer's tests can be deterministic given the layers
beneath. Without #1562, #1569's tests would need to simulate cron-storm
behaviour to be useful; with #1562 the storm cannot start and #1569 can
be tested in isolation.

### 6.3 Risk of regressions across layers

The four layers share two surfaces:

- The `WorktreeManager` type, which Layers 1 and 2 both extend.
- The `<worktree_root>/review-*` directory contract, which Layers 1, 2,
  and 3 all assume.

A change to the worktree path convention (e.g. embedding a process PID
in the name, or using a subdirectory per pid) would invalidate all three
layers simultaneously. Phase 2 should freeze the path convention and
codify it in a constant -- `WORKTREE_REVIEW_PREFIX: &str = "review-"`
in `scope.rs` -- so Layer 3's shell script can reference the same
literal via a versioned source.

---

## 7. Out of scope

The following items came up during research and are deferred. They are
not blockers for the four-layer epic and would expand the change
surface meaningfully.

### 7.1 Moving compound review out of `reconcile_tick`

The compound-review entry path currently runs inline inside
`handle_schedule_event` (`crates/terraphim_orchestrator/src/lib.rs:7513-7567`),
which itself is called from `reconcile_tick`. A long-running review thus
blocks the entire reconciliation cycle. A natural refactor would be to
detach the review into a `tokio::spawn` background task with a
dedicated channel back into the orchestrator.

**Deferred** because:

- The detached-spawn pattern reintroduces the cancellation problem
  that this epic is *solving*.
- The fix would interact badly with the existing concurrency gate at
  `crates/terraphim_orchestrator/src/lib.rs:2154-2168`.
- It is orthogonal to the leak: a detached review still creates a
  worktree and still needs Layer 1 guarantees on cancellation.

### 7.2 Switching scheduler crate

The `cron` crate has known limitations (no native time-zone support,
iterator semantics that bit us in #1562). [`tokio-cron-scheduler`] and
[`apalis`] are alternatives. Migrating is a multi-week task and not
required for the leak fix.

**Deferred** to a dedicated tech-debt issue if the scheduling subsystem
accumulates further bugs.

### 7.3 Agent worktree namespacing

The per-agent path uses `/tmp/adf-worktrees/<agent_name>-<8char-uuid>`.
Two agents with the same name run by two orchestrator processes (e.g.
during deploys) would collide. The current uuid suffix avoids name
collision in practice but does not prevent it formally. A namespacing
scheme (`<orchestrator-pid>/<agent>-<uuid>` or
`/run/adf-<pid>/worktrees/<agent>-<uuid>`) would harden this, at the
cost of a different sweep contract.

**Deferred** because the practical collision probability is zero
(uuids) and the operational benefit is small.

### 7.4 sccache cleanup on worktree removal

`/data/.../worktrees/review-*` directories on bigbox contained large
`target/` trees (~70 MiB each) and the `sccache` daemon's mmap'd
caches were observed across them. `git worktree remove --force` will
delete the files, but if `sccache` is running it may hold open file
descriptors that delay disk reclamation. A dedicated `sccache --stop-server`
before the sweep would be belt-and-braces.

**Deferred** because Linux unlinked-file semantics handle this
correctly on filesystem level (deleted files are reclaimed when the
last fd closes). Disk-pressure alerts would catch the residue.

### 7.5 Compound review backpressure

If the compound review legitimately takes longer than the cron interval
(e.g. a daily review that takes >24 h on a large repo), Layer 0 advances
the cursor but the actual review still overruns. A proper fix needs a
"single-flight" guard akin to `last_cron_fire` for compound reviews.
This is *adjacent* to #1562 -- in fact #1562 likely should also
introduce a `last_compound_review_fire` field -- but Phase 1 cannot
make that call without seeing #1562's PR.

**Deferred** to be raised on #1562's PR if not already addressed.

### 7.6 Existing `WorktreeGuard` registry leak

Section 3.7 identifies that the existing per-agent path leaks git
registry entries even on the happy path. Fixing this is a small change
but is technically out-of-scope for #1569 (whose stated objective is
compound-review coverage). Phase 2 may choose to fold it in or split it
into a sibling issue.

**Deferred** with a recommendation to fold in if effort is < 1 day.

---

## 8. Essential questions check

Per the disciplined-research skill, the following three questions
gate Phase 2 entry.

### 8.1 Is this work energising?

Yes, on three counts:

- **Concrete production incident.** The 692 leaked worktrees on bigbox
  are visible, measurable, and have a clear cost (48 GiB disk, OOM
  risk, lost reviews). Closing the loop on a real incident is
  motivating in a way that abstract refactors are not.
- **Idiomatic Rust opportunity.** RAII via `Drop` is one of Rust's
  signature strengths; building a textbook-quality `WorktreeGuard` that
  survives tokio cancellation is the kind of work that has lasting
  pedagogical value for the team.
- **Tight feedback loop.** Each layer has a clear, testable acceptance
  criterion. Phase 2 can demonstrate progress in days, not weeks.

### 8.2 Does this leverage existing strengths?

Yes:

- The Rust+tokio expertise on the team is the right substrate for the
  cancellation-safety work. The team has prior experience with
  `kill_on_drop`, `block_in_place`, and cancellation tokens (cf. the
  existing `WorktreeGuard` itself).
- The orchestrator code is well-instrumented with `tracing::`; adding
  the new sweep events is a one-line-per-event change.
- The systemd `ExecStartPre` integration is standard Linux
  administration; no novel ops infrastructure is required.

### 8.3 Does it meet a real need?

Yes, by all four signals:

- **Production incident as evidence.** Section 1.1 cites bigbox state
  with specific numbers; there is no doubt the bug is real.
- **Storm postmortem timeline.** Compound review #514 silent from
  2026-05-13 to 2026-05-17 -- four days of missed feedback to humans
  during which the system thought it was working.
- **Recurrence likelihood.** The bug class (RAII not applied to a
  long-lived async future) is a category that will reproduce in any
  new long-running orchestrator path unless the pattern is internalised.
  Landing a documented, tested `WorktreeGuard` for compound review
  inoculates against the next case.
- **User-stated priority.** The epic is named, the four issues are
  filed, and the brief explicitly requests the disciplined-research
  workflow rather than ad-hoc fixes.

The evidence is convergent: the work is well-scoped, well-motivated,
and aligned with team capability.

---

## Phase 1 deliverable

This document is the Phase 1 deliverable. It captures problem framing,
existing-implementation map, constraints, observability gaps, open
questions, sequencing dependencies, and out-of-scope items, with
file:line citations throughout. No code has been changed.

Recommended next steps:

1. Review this document. Resolve sections 5.7 (Drop strategy) and
   5.8 (cancellation propagation) before Phase 2 begins -- they are
   the only blocking open questions.
2. Land #1562 to stop the bleed.
3. Enter Phase 2 (Design) for #1569 with the resolutions to 5.7
   and 5.8 in hand.
4. Phases 2 and 3 for #1570 and #1571 follow #1569.
