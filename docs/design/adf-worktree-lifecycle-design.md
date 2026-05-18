# ADF Worktree Lifecycle Hardening -- Design Document

Gitea epic: `terraphim/terraphim-ai#1567`
Phase: 2 of disciplined development (Design)
Prerequisite: `docs/design/adf-worktree-lifecycle-research.md` (Phase 1, 1045 lines)
Author: claude-opus-4-7 (design agent)
Date: 2026-05-17

> This document defines the implementation plan for the four-layer
> worktree-lifecycle hardening epic. No code is changed; no PR is opened.
> Implementation (Phase 3) starts only after human approval lands on
> epic #1567.

---

## 1. Overview and approach

The bigbox storm of 2026-05-17 (research section 1.1) compounded two
independently-correctable defects into a feedback loop that produced
692 leaked git worktrees occupying 48 GiB. The fix is layered. No
single patch is sufficient; each layer addresses a failure mode the
others cannot.

| Layer | Issue | Defends against | Process context |
|------:|------:|-----------------|-----------------|
| 0 | #1562 | Schedule cursor never advancing under reconcile-timeout cancellation. Caps the per-storm fire rate from "every 90 s" to "every cron occurrence". | orchestrator process |
| 1 | #1569 | `await`-after-cleanup pattern. RAII `WorktreeGuard` runs cleanup from `Drop` on every exit path including future cancellation. | orchestrator process |
| 2 | #1570 | Residue from `SIGKILL` / OOM / panic-across-runtime that cleared the orchestrator process before any `Drop` could fire. | orchestrator process (user `alex`) |
| 3 | #1571 | Root-owned files inside worktrees that the orchestrator user cannot delete. | systemd `ExecStartPre`, root |

The layers compose: Layer 0 reduces blast radius for any Layer 1
regression by ~200x (research section 6.1); Layer 1 plugs the live
leak; Layer 2 cleans residue at every restart; Layer 3 cleans residue
the unprivileged sweep cannot. Each layer is independently mergeable
and each carries its own acceptance test.

This design treats the eight open questions in research section 5 as
**resolved** per the decisions captured in the task brief and section
2 below. Where additional sub-decisions emerged during design they
are recorded inline with the section that introduces them.

The design is **conservative on every axis**: synchronous `Drop`
(established precedent at `worktree_guard.rs:69`), the existing
`std::process::Command` pattern (already used by
`scope.rs:316-337`), no new crate dependencies, no async-trait
machinery, no new schedulers. The single new concurrency primitive
is `tokio::task::JoinSet` from the existing
`tokio = { features = ["full"] }` in
`crates/terraphim_orchestrator/Cargo.toml:23`.

---

## 2. Scope

### 2.1 In scope

- **Layer 0 (#1562)**: New cursor field
  `last_compound_review_fired_at` on `Orchestrator`; cursor advance
  inside the fire branch at `lib.rs:7148-7161`; gate against the
  cursor on subsequent iterations.
- **Layer 1 (#1569)**: New git-aware `WorktreeGuard` constructor;
  `WorktreeManager::create_worktree` returns the guard; `compound.rs`
  swarm spawn refactored to `JoinSet` with `kill_on_drop(true)`;
  guard declared before the `JoinSet` so Drop ordering is provable.
- **Layer 2 (#1570)**: New synchronous
  `WorktreeManager::sweep_stale` method invoked from
  `Orchestrator::new`; sweep covers both
  `<worktree_root>/review-*` and `/tmp/adf-worktrees/*` roots.
- **Layer 3 (#1571)**: New POSIX shell script
  `scripts/adf-setup/adf-cleanup.sh`; corresponding shell-test under
  `scripts/adf-setup/tests/`; `docs/operations/` snippet for the
  systemd `ExecStartPre=` line.
- **Cross-cutting**: One new constant `WORKTREE_REVIEW_PREFIX` in
  `scope.rs` so Layer 3 and Layer 2 reference the same literal; new
  tracing fields (`swept_count`, `backlog_count`,
  `root_owned_skipped`); updated `WorktreeGuard` unit tests; new
  `compound.rs` cancellation property test; new shell test for
  `adf-cleanup.sh`.

### 2.2 Out of scope

- Migrating `WorktreeGuard` to async `Drop` (unstable; research 3.1).
- Replacing the `cron` crate (research 7.2).
- Moving compound review out of `reconcile_tick` (research 7.1).
- Per-agent worktree namespacing changes (research 7.3).
- `sccache` lifecycle hooks (research 7.4).
- Compound-review single-flight backpressure beyond the cursor fix
  (research 7.5).
- Reworking the per-agent path in `lib.rs:5392-5438` to use the new
  git-aware guard constructor. That path already disarms its guard
  on success and explicitly calls `remove_agent_worktree`; folding
  it in expands review surface and risks regressing a working path.
  Recommendation: file a follow-up sibling issue if desired
  (research 7.6).

### 2.3 Avoid at all cost

- **Async `Drop`**: would require nightly and break the stable-Rust
  guarantee.
- **`tokio::spawn` from inside `Drop`**: brittle at shutdown when the
  runtime handle may be gone (research 3.2, strategy D).
- **Detached cleanup queue**: adds a moving part that needs separate
  visibility and complicates shutdown drain semantics (research 3.2,
  strategy C). The synchronous strategy is precedented and
  sufficient.
- **Changing the worktree path convention**: would invalidate all
  three downstream layers at once (research 6.3). The literal
  `review-` and `/tmp/adf-worktrees/` are frozen by this design.
- **Implicit reliance on `kill_on_drop`** as the sole subprocess kill
  path. The design uses `kill_on_drop(true)` **and**
  `JoinSet::abort` on the wrapping tasks, in that order, and
  documents why both are needed.

### 2.4 Decisions made on research section 5 open questions

The task brief resolves the open questions. Recorded here for the
reader who reviews this document in isolation:

| Research question | Resolution | Reference |
|-------------------|------------|-----------|
| 5.7 `git worktree remove --force` vs filesystem removal? | Hybrid: synchronous `git worktree remove --force` first, `std::fs::remove_dir_all` as fallback on non-zero exit. Layer 2's `sweep_stale` then runs `git worktree prune`. | Step 4 |
| 5.8 Cancellation propagation? | `kill_on_drop(true)` on `Command` at `compound.rs:490` **and** `JoinSet` replaces the bare `tokio::spawn` loop. Aborting the JoinSet drops the wrapping tasks, which drop the `Child` handles, which now kill the subprocesses thanks to `kill_on_drop`. | Steps 5-7 |
| 5.3 Keep `WorktreeGuard::keep()`? | Yes. Cost is one method; future PR-review and product-development agents may need worktree hand-off semantics. YAGNI-allowed. | Step 3 |
| 5.2 Sweep scope: `review-*` only? | Both prefixes: `<worktree_root>/review-*` AND `/tmp/adf-worktrees/*`. Bigbox `git worktree prune --verbose` showed stale `.git/worktrees/sentinel-*` admin refs. | Step 10 |
| 5.1 / 5.6 `adf-cleanup.sh` source-of-truth location? | `scripts/adf-setup/adf-cleanup.sh`. Matches the existing `scripts/adf-setup/` deployment vocabulary (TOML configs plus Python migrators live there today). | Step 13 |
| 5.4 Sweep timing? | From `Orchestrator::new` (synchronous), before construction returns. The sweep finishes before any tick thread is spawned (which happens later, in `run()` at `lib.rs:1218-1225`). Acceptance: zero `review-*` directories visible immediately after `Orchestrator::new` returns. | Step 11 |

### 2.5 Note on the brief's `start_dual_mode` reference

The task brief mentions `Orchestrator::start_dual_mode` as the
post-condition surface for Layer 2. No such method exists in
`crates/terraphim_orchestrator/src/lib.rs` (verified by grep). The
analogous async entry point is `Orchestrator::run` at `lib.rs:1053`.
The design proceeds against `Orchestrator::new` (sync) for the sweep
call site, which is strictly stronger: sweep completes before any
async work begins.

---

## 3. Step-by-step implementation plan

Each step is intended to map to one reviewable commit. Steps within
a layer must merge in the order shown; layers themselves can land
independently (see section 7 rollout sequence).

### Layer 0 (issue #1562) -- schedule cursor

**Step 1.** Add cursor field to `Orchestrator` struct.

- File: `crates/terraphim_orchestrator/src/lib.rs`.
- Edit at `:241` (after `last_cron_fire`): add
  `last_compound_review_fired_at: Option<chrono::DateTime<chrono::Utc>>`.
- Initialise at `:817` (alongside `last_tick_time`): `None`.
- Dependencies: none.
- Tests: covered by Step 2's test.
- Rollback: delete the field; the rest of the codebase has no other
  reference yet.

**Step 2.** Advance cursor inside fire branch at
`lib.rs:7137-7161`; gate subsequent iterations against it.

- File: `crates/terraphim_orchestrator/src/lib.rs`.
- Replace the `should_fire` block with a fire-time computation
  mirroring the per-agent loop at `:7110-7130`.
- Cursor is updated **before** `handle_schedule_event` is awaited,
  guaranteeing the next iteration sees the new value even if the
  reconcile-timeout wrapper cancels the future mid-await.
- Dependencies: Step 1.
- Tests: new `test_compound_review_cursor_advances_on_cancellation`
  in `lib.rs` test module. Plant a `last_tick_time` value in the
  past via the existing `set_last_tick_time` helper at `:7712`;
  install a cron expression that fires "1 second ago"; call
  `check_cron_schedules` once; verify
  `last_compound_review_fired_at` is `Some(_)`; call again and
  verify the field did not change.
- Rollback: revert to the previous `should_fire` block; cursor field
  becomes dead.

### Layer 1 (issue #1569) -- RAII `WorktreeGuard` in compound

**Step 3.** Add `WorktreeGuard::for_managed` constructor and extend
internal `cleanup` to use git-aware teardown.

- File: `crates/terraphim_orchestrator/src/worktree_guard.rs`.
- Add a new field `repo_path: Option<PathBuf>` to the struct. When
  `Some`, `cleanup` runs
  `git -C <repo> worktree remove --force <path>` synchronously via
  `std::process::Command::new("git").status()`, falling back to
  `std::fs::remove_dir_all` on non-zero exit or
  executable-not-found.
- Add `pub fn for_managed(repo_path: impl AsRef<Path>, worktree_path:
  impl AsRef<Path>) -> Self`.
- Keep `pub fn new(path: impl AsRef<Path>) -> Self` semantically
  unchanged: `repo_path = None`, falls through to the existing
  filesystem-only removal path. The per-agent caller in
  `lib.rs:5392-5438` is unaffected.
- Keep `pub fn keep(mut self)`: still sets `should_cleanup = false`.
  Documented for future hand-off cases per research 5.3.
- Dependencies: none.
- Tests: extend the `mod tests` block:
  - `test_managed_guard_invokes_git_remove`.
  - `test_managed_guard_fallback_on_git_failure`.
  - `test_managed_guard_keep_disarms`.
- Rollback: delete `for_managed` and `repo_path`; the existing
  per-agent path is unchanged.

**Step 4.** Change `WorktreeManager::create_worktree` to return the
guard.

- File: `crates/terraphim_orchestrator/src/scope.rs`.
- New return type: `Result<WorktreeGuard, std::io::Error>`.
- The existing implementation at `:260-301` is preserved; the only
  change is the return value.
- Add `pub const WORKTREE_REVIEW_PREFIX: &str = "review-";` at module
  scope. Single source of truth for Layer 2 and Layer 3.
- Dependencies: Step 3.
- Tests: update existing `test_create_worktree`,
  `test_remove_worktree`, `test_cleanup_all` tests to receive a
  guard and call `.keep()` where the test wants to inspect the
  worktree before removal.
- Rollback: revert the return type.

**Step 5.** Refactor `compound.rs:296-370` to hold the guard
declaratively and use `JoinSet` for the swarm.

- File: `crates/terraphim_orchestrator/src/compound.rs`.
- The guard is declared **first** (so it drops **last**).
- A `JoinSet<AgentResult>` is declared **second** (so it drops
  **first** on cancellation; aborts every task before the guard's
  Drop runs).
- The bare `tokio::spawn` loop at `:319-330` becomes
  `tasks.spawn(async move { run_single_agent(...).await })`.
- The collection loop at `:341-365` becomes
  `tokio::time::timeout_at(collect_deadline, tasks.join_next())`.
  `Ok(Some(Err(join_err)))` logs at `warn` and continues;
  `Ok(None)` exits the loop normally.
- The explicit cleanup at `:367-370` is **removed**; the guard's
  `Drop` performs it.
- Dependencies: Step 4.
- Tests: see Step 8.
- Rollback: revert to the previous explicit cleanup and switch back
  to `tokio::spawn`.

**Step 6.** Add `kill_on_drop(true)` to the `Command` builder at
`compound.rs:490-528`.

- File: `crates/terraphim_orchestrator/src/compound.rs`.
- Single-line change: `cmd.kill_on_drop(true);` immediately after
  the `tokio::process::Command::new(cli_tool)` constructor.
- Without it, aborting the JoinSet wrapping task drops `Child` but
  does not kill the underlying process (research 3.1).
- Dependencies: Step 5.
- Rollback: remove the single line.

**Step 7.** Document the Drop ordering invariant in `compound.rs`.

- File: `crates/terraphim_orchestrator/src/compound.rs`.
- Add a multi-line comment immediately above the guard / JoinSet
  declarations. Inverting the order recreates the bigbox storm race.
- Dependencies: Steps 5-6.
- Tests: review-time documentation; not testable.
- Rollback: remove the comment.

**Step 8.** Add cancellation-property integration test.

- New file:
  `crates/terraphim_orchestrator/tests/compound_cancellation_test.rs`.
- Test outline:
  1. Set up a real git repo in a `TempDir` (re-use
     `scope.rs::tests::setup_git_repo`, promoted to `pub(crate)`).
  2. Construct a minimal `SwarmConfig` whose single review group
     invokes `cli_tool = "/bin/sleep"`. Gives a long-lived
     subprocess that does not exit on its own.
  3. Spawn `workflow.run("HEAD", "HEAD~1")` inside a
     `tokio::spawn`; capture its `JoinHandle`.
  4. Wait 200 ms for the worktree to be created and the agent
     subprocess to be live.
  5. Capture the agent subprocess PID by listing PIDs whose `cwd`
     is the worktree.
  6. Call `handle.abort()` on the outer task; await `handle`.
  7. Within 2 s, assert:
     - The worktree directory under `<base>/review-*` is gone.
     - The `.git/worktrees/review-*` admin entry is gone.
     - The agent PID is no longer alive (`kill(pid, 0)` returns
       `ESRCH`).
- Variant: deliberately reintroduce the Layer 0 cursor bug (call
  `check_cron_schedules` twice without advancing `last_tick_time`);
  assert no worktree directory exists at the end.
- Dependencies: Steps 5-7.
- Rollback: delete the file.

### Layer 2 (issue #1570) -- startup sweep

**Step 9.** Add `WorktreeManager::sweep_stale` method.

- File: `crates/terraphim_orchestrator/src/scope.rs`.
- Synchronous signature:
  `pub fn sweep_stale(&self, extra_roots: &[PathBuf]) -> SweepReport`.
- `SweepReport`: `pub struct` with `swept_count`, `failed_count`,
  `root_owned_skipped`, `prune_succeeded`, `duration_ms`.
- Behaviour:
  1. For `self.worktree_base`, walk direct children whose name
     starts with `WORKTREE_REVIEW_PREFIX`. For each `extra_root`,
     walk **all** direct children. Invoke
     `git -C <repo> worktree remove --force <child>`; fall back to
     `std::fs::remove_dir_all` on non-zero exit.
  2. After walking, run `git -C <repo> worktree prune --verbose`.
  3. `EACCES` / `EPERM` increments `root_owned_skipped` without
     marking the sweep failed. Layer 3 catches these on the next
     service restart.
- Emit a single `info!` line at the end with all `SweepReport`
  fields plus `backlog_count = swept_count + root_owned_skipped`.
- Dependencies: Step 4.
- Tests:
  - `test_sweep_stale_empty_dir`.
  - `test_sweep_stale_no_base`.
  - `test_sweep_stale_removes_review_prefix`.
  - `test_sweep_stale_preserves_non_review_prefix`.
  - `test_sweep_stale_runs_prune`.
  - `test_sweep_stale_skips_root_owned` (Linux + root only;
    gated by `#[cfg_attr(not(target_os = "linux"), ignore)]` and
    runtime `getuid() == 0` check).
- Rollback: delete the method and struct.

**Step 10.** Wire `sweep_stale` into `Orchestrator::new`.

- File: `crates/terraphim_orchestrator/src/lib.rs`.
- Insert sweep call immediately after `compound_workflow` is
  constructed, before the `Ok(Self { ... })` block at `:786-867`.
- `extra_roots = vec![PathBuf::from("/tmp/adf-worktrees")]`. Literal
  matches `lib.rs:5393`.
- Add `pub fn worktree_manager(&self) -> &WorktreeManager` accessor
  on `CompoundReviewWorkflow` if it does not exist; trivial.
- Dependencies: Step 9.
- Tests:
  `crates/terraphim_orchestrator/tests/sweep_on_startup_test.rs`
  pre-seeds three `review-*` dirs under a `TempDir`, calls
  `Orchestrator::new`, asserts the directories are gone before
  `new` returns.
- Rollback: remove the call.

### Layer 3 (issue #1571) -- root-privileged `ExecStartPre`

**Step 11.** Add `scripts/adf-setup/adf-cleanup.sh`.

- New file, POSIX `sh` (not bash). Idempotent. Walk
  `<WORKTREE_ROOT>/review-*` and `/tmp/adf-worktrees/*`, invoke
  `git worktree remove --force` per entry (fall back to recursive
  directory removal), then `git worktree prune` once. Emit one
  summary line to stdout.
- Env vars: `ADF_REPO_PATH`, `ADF_WORKTREE_ROOT`,
  `ADF_AGENT_TMP_ROOT` (all overridable via systemd
  `Environment=`).
- The literal prefix `review-` cross-references
  `WORKTREE_REVIEW_PREFIX` in `scope.rs`.
- Dependencies: Step 4.
- Tests: see Step 12.
- Rollback: delete the file.

**Step 12.** Add `scripts/adf-setup/tests/test_adf_cleanup.sh`.

- POSIX shell test driver. Pre-seeds three `review-*` directories
  plus a `keep-me/` directory, runs `adf-cleanup.sh`, asserts:
  - `review-*` gone.
  - `keep-me/` preserved.
  - `git worktree list` clean.
  - Second run is a no-op.
- Dependencies: Step 11.
- Rollback: delete the file.

**Step 13.** Add deployment doc snippet.

- New file: `docs/operations/adf-orchestrator-systemd.md` containing
  the `[Service]` fragment, install commands, and a note that an
  in-tree installer is not yet present.
- Dependencies: Step 11.
- Rollback: delete the file.

### Cross-cutting

**Step 14.** Add new tracing fields to existing sweep and cleanup
events.

- Files: `crates/terraphim_orchestrator/src/scope.rs`,
  `crates/terraphim_orchestrator/src/worktree_guard.rs`.
- Fields per research 4.2: `swept_count`, `failed_count`,
  `root_owned_skipped`, `duration_ms` on sweep summary;
  `cancellation_reason` (optional) on guard's Drop.
- Dependencies: Steps 3, 9.
- Rollback: remove the new fields.

**Step 15.** Update epic body in Gitea after approval.

- Not a code change. Comment on epic #1567 confirming the design is
  approved and link to this document.
- Dependencies: human approval.

---

## 4. Per-layer file change manifest

The diff sketches below are illustrative. Exact line numbers and
imports may shift during implementation; the structural shape is
the review surface.

### 4.1 Layer 0 (#1562) -- schedule cursor

```rust
// crates/terraphim_orchestrator/src/lib.rs -- struct, near :241

    /// Per-agent last cron fire timestamp to prevent re-triggering
    /// within same schedule window.
    last_cron_fire: HashMap<String, chrono::DateTime<chrono::Utc>>,

+   /// Last compound-review fire time, used to gate the compound
+   /// schedule independently of `last_tick_time`. Mirrors the
+   /// `last_cron_fire` pattern for per-agent crons (#1562).
+   last_compound_review_fired_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Lazy-initialised Gitea tracker for gitea-issue pre-check.
    pre_check_tracker: Option<terraphim_tracker::GiteaTracker>,
```

```rust
// crates/terraphim_orchestrator/src/lib.rs -- Self { ... }, near :817

    last_cron_fire: HashMap::new(),
+   last_compound_review_fired_at: None,
    pre_check_tracker: None,
```

```rust
// crates/terraphim_orchestrator/src/lib.rs -- :7137-7161

    // Also check compound review schedule
    if let Some(compound_sched) = self.scheduler.compound_review_schedule() {
        debug!(
            last_tick = %self.last_tick_time,
            now = %now,
            "checking compound review schedule"
        );

-       let upcoming: Vec<_> = compound_sched.after(&self.last_tick_time).take(3).collect();
-       debug!(upcoming = ?upcoming, "compound schedule upcoming times");
-
-       let should_fire = compound_sched
-           .after(&self.last_tick_time)
-           .take_while(|t| *t <= now)
-           .next()
-           .is_some();
-
-       debug!(should_fire = should_fire, "compound review fire check");
-
-       if should_fire {
-           info!("compound review schedule fired, starting review");
-           self.handle_schedule_event(ScheduleEvent::CompoundReview)
-               .await;
-       }
+       // Compute the latest occurrence <= now that we have not yet
+       // recorded as fired. Cursor `last_compound_review_fired_at`
+       // is per-occurrence (not per-tick) so reconcile-timeout
+       // cancellation no longer re-fires the same occurrence on
+       // the next iteration. Mirrors the per-agent shape at
+       // :7110-7130.
+       let next_fire = compound_sched
+           .after(&self.last_tick_time)
+           .take_while(|t| *t <= now)
+           .next();
+
+       if let Some(fire_time) = next_fire {
+           let already_fired = self
+               .last_compound_review_fired_at
+               .map(|prev| fire_time <= prev)
+               .unwrap_or(false);
+
+           if !already_fired {
+               // Record fire time BEFORE await so cancellation does
+               // not re-trigger the same occurrence on next tick.
+               self.last_compound_review_fired_at = Some(fire_time);
+               info!(
+                   fire_time = %fire_time,
+                   "compound review schedule fired, starting review"
+               );
+               self.handle_schedule_event(ScheduleEvent::CompoundReview)
+                   .await;
+           }
+       }
    }
```

The cursor advance is **synchronous** and runs **before** any
`.await`. If `handle_schedule_event` is cancelled by the 90 s
reconcile-timeout wrapper, the cursor has already moved past the
fired occurrence; the next iteration's
`take_while(|t| *t <= now)` still returns the same `fire_time` but
the `already_fired` gate short-circuits the call.

### 4.2 Layer 1 (#1569) -- `WorktreeGuard` and `compound.rs`

```rust
// crates/terraphim_orchestrator/src/worktree_guard.rs -- struct

 #[derive(Debug)]
 pub struct WorktreeGuard {
     path: PathBuf,
     should_cleanup: bool,
+    /// When `Some`, `Drop` runs `git -C <repo_path> worktree remove
+    /// --force <path>` first and falls back to a filesystem-only
+    /// removal on non-zero exit. When `None`, only the filesystem
+    /// path runs (existing per-agent caller, unchanged).
+    repo_path: Option<PathBuf>,
 }

 impl WorktreeGuard {
     pub fn new<P: AsRef<Path>>(path: P) -> Self {
         let path = path.as_ref().to_path_buf();
         debug!(path = %path.display(), "worktree guard created");
         Self {
             path,
             should_cleanup: true,
+            repo_path: None,
         }
     }

+    /// Create a managed guard whose `Drop` invokes `git worktree
+    /// remove --force` against `repo_path` before falling back to
+    /// directory removal. Use this when the worktree was created
+    /// via `WorktreeManager::create_worktree` so the git admin
+    /// registry at `.git/worktrees/<name>` is reconciled.
+    pub fn for_managed<R: AsRef<Path>, P: AsRef<Path>>(
+        repo_path: R,
+        worktree_path: P,
+    ) -> Self {
+        let path = worktree_path.as_ref().to_path_buf();
+        let repo = repo_path.as_ref().to_path_buf();
+        debug!(
+            repo_path = %repo.display(),
+            worktree_path = %path.display(),
+            "managed worktree guard created"
+        );
+        Self {
+            path,
+            should_cleanup: true,
+            repo_path: Some(repo),
+        }
+    }
```

```rust
// crates/terraphim_orchestrator/src/worktree_guard.rs -- cleanup

     fn cleanup(&self) {
         if !self.should_cleanup {
             return;
         }

         if !self.path.exists() {
             debug!(path = %self.path.display(), "worktree already removed");
             return;
         }

+        // Managed path: try `git worktree remove --force` first.
+        if let Some(ref repo) = self.repo_path {
+            let start = std::time::Instant::now();
+            let status = std::process::Command::new("git")
+                .arg("-C")
+                .arg(repo)
+                .arg("worktree")
+                .arg("remove")
+                .arg("--force")
+                .arg(&self.path)
+                .env_remove("GIT_INDEX_FILE")
+                .status();
+
+            match status {
+                Ok(s) if s.success() => {
+                    info!(
+                        path = %self.path.display(),
+                        duration_ms = start.elapsed().as_millis() as u64,
+                        "worktree cleaned up via git"
+                    );
+                    return;
+                }
+                Ok(s) => {
+                    warn!(
+                        path = %self.path.display(),
+                        exit_code = ?s.code(),
+                        "git worktree remove failed, falling back to fs"
+                    );
+                }
+                Err(e) => {
+                    warn!(
+                        path = %self.path.display(),
+                        error = %e,
+                        "git CLI not invokable, falling back to fs"
+                    );
+                }
+            }
+        }
+
+        // Fallback / unmanaged path: filesystem-only removal.
         match std::fs::remove_dir_all(&self.path) {
             Ok(_) => {
                 info!(path = %self.path.display(), "worktree cleaned up");
             }
             Err(e) => {
                 warn!(
                     path = %self.path.display(),
                     error = %e,
                     "failed to remove worktree"
                 );
                 if let Err(e2) = std::fs::remove_dir(&self.path) {
                     debug!(
                         path = %self.path.display(),
                         error = %e2,
                         "failed to remove worktree dir"
                     );
                 }
             }
         }
     }
```

```rust
// crates/terraphim_orchestrator/src/scope.rs -- module scope

 use std::path::{Path, PathBuf};
 use std::collections::HashSet;
 use serde::{Deserialize, Serialize};
 use tracing::{debug, error, info, warn};
 use uuid::Uuid;
+
+use crate::worktree_guard::WorktreeGuard;
+
+/// Directory name prefix for compound-review worktrees. Single
+/// source of truth for Layer 2 sweep and the Layer 3 shell script.
+/// Changes here must be mirrored in
+/// `scripts/adf-setup/adf-cleanup.sh`.
+pub const WORKTREE_REVIEW_PREFIX: &str = "review-";
```

```rust
// crates/terraphim_orchestrator/src/scope.rs -- create_worktree

     pub async fn create_worktree(
         &self,
         name: &str,
         git_ref: &str,
-    ) -> Result<PathBuf, std::io::Error> {
+    ) -> Result<WorktreeGuard, std::io::Error> {
         let worktree_path = self.worktree_base.join(name);
         // ... unchanged until success branch ...
         info!(name = %name, path = %worktree_path.display(), "worktree created");
-        Ok(worktree_path)
+        Ok(WorktreeGuard::for_managed(&self.repo_path, worktree_path))
     }
```

```rust
// crates/terraphim_orchestrator/src/compound.rs -- run, replacing :296-370

         // Order matters. `_guard` is declared BEFORE `tasks` so on
         // `Drop` the JoinSet aborts every wrapping task (and via
         // `kill_on_drop` every subprocess) BEFORE the guard's
         // synchronous `git worktree remove` runs. Inverting this
         // order recreates the storm race documented in the
         // worktree-lifecycle research doc section 3.3.
-        let worktree_name = format!("review-{}", correlation_id);
-        let worktree_path = self
+        let worktree_name = format!(
+            "{}{}",
+            crate::scope::WORKTREE_REVIEW_PREFIX,
+            correlation_id
+        );
+        let _guard = self
             .worktree_manager
             .create_worktree(&worktree_name, git_ref)
             .await
             .map_err(|e| {
                 OrchestratorError::CompoundReviewFailed(format!(
                     "failed to create worktree: {}",
                     e
                 ))
             })?;
+        let worktree_path = _guard.path().to_path_buf();
+
+        let mut tasks: tokio::task::JoinSet<AgentResult> =
+            tokio::task::JoinSet::new();

-        // Channel for collecting agent outputs
-        let (tx, mut rx) = mpsc::channel::<AgentResult>(active_groups.len().max(1));
-
-        // Spawn agents in parallel
         let mut spawned_count = 0;
         for group in active_groups {
-            let tx = tx.clone();
             let group = group.clone();
             let worktree_path = worktree_path.clone();
             let changed_files = changed_files.clone();
             let timeout = self.config.timeout;
             let cli_tool = group.cli_tool.clone();
-            tokio::spawn(async move {
-                let result = run_single_agent(...).await;
-                let _ = tx.send(result).await;
-            });
+            tasks.spawn(async move {
+                run_single_agent(
+                    &group,
+                    &worktree_path,
+                    &changed_files,
+                    correlation_id,
+                    timeout,
+                    &cli_tool,
+                )
+                .await
+            });
             spawned_count += 1;
         }

-        drop(tx);
         let mut agent_outputs = Vec::new();
         let mut failed_count = 0;
         let collect_deadline =
             tokio::time::Instant::now() + self.config.timeout + Duration::from_secs(10);

         loop {
-            match tokio::time::timeout_at(collect_deadline, rx.recv()).await {
-                Ok(Some(result)) => match result {
+            match tokio::time::timeout_at(collect_deadline, tasks.join_next()).await {
+                Ok(Some(Ok(result))) => match result {
                     AgentResult::Success(output) => { ... }
                     AgentResult::Failed { agent_name, reason } => { ... }
                 },
+                Ok(Some(Err(join_err))) => {
+                    warn!(error = %join_err, "agent task aborted or panicked");
+                    failed_count += 1;
+                }
                 Ok(None) => break,
                 Err(_) => {
                     warn!("collection deadline exceeded, using partial results");
                     break;
                 }
             }
         }

-        // Cleanup worktree
-        if let Err(e) = self.worktree_manager.remove_worktree(&worktree_name).await {
-            warn!(error = %e, "failed to cleanup worktree");
-        }
+        // No explicit cleanup: `_guard` is dropped at end of scope,
+        // invoking `git worktree remove --force` synchronously.
```

```rust
// crates/terraphim_orchestrator/src/compound.rs -- run_single_agent, near :490

     let mut cmd = tokio::process::Command::new(cli_tool);
+
+    // Ensure that dropping the `Child` handle kills the underlying
+    // subprocess. Without this, JoinSet::abort() + Child::drop()
+    // leaves the subprocess running until its own timeout (default
+    // 30 minutes; see SwarmConfig::timeout). Combined with the
+    // JoinSet abort, this gives cooperative-then-forceful shutdown
+    // on cancellation.
+    cmd.kill_on_drop(true);
```

Note also: the `use tokio::sync::mpsc;` line at `compound.rs:4` can
be removed once the channel-based collection is gone (clippy
unused-import).

### 4.3 Layer 2 (#1570) -- startup sweep

```rust
// crates/terraphim_orchestrator/src/scope.rs -- new public API

+ /// Summary of a sweep_stale invocation. Emitted via structured
+ /// tracing so Quickwit can compute backlog gauges and alert on
+ /// large residue.
+ #[derive(Debug, Clone, Default)]
+ pub struct SweepReport {
+     pub swept_count: usize,
+     pub failed_count: usize,
+     pub root_owned_skipped: usize,
+     pub prune_succeeded: bool,
+     pub duration_ms: u64,
+ }

 impl WorktreeManager {
+    pub fn sweep_stale(&self, extra_roots: &[PathBuf]) -> SweepReport {
+        let start = std::time::Instant::now();
+        let mut report = SweepReport::default();
+
+        // 1. Primary base: only `WORKTREE_REVIEW_PREFIX`-prefixed
+        //    entries.
+        if self.worktree_base.is_dir() {
+            for entry in std::fs::read_dir(&self.worktree_base).into_iter().flatten() {
+                let entry = match entry {
+                    Ok(e) => e,
+                    Err(_) => continue,
+                };
+                let name = match entry.file_name().into_string() {
+                    Ok(n) => n,
+                    Err(_) => continue,
+                };
+                if !name.starts_with(WORKTREE_REVIEW_PREFIX) {
+                    continue;
+                }
+                self.sweep_one(&entry.path(), &mut report);
+            }
+        }
+
+        // 2. Extra roots (typically `/tmp/adf-worktrees`):
+        //    all direct children, regardless of prefix.
+        for root in extra_roots {
+            if !root.is_dir() {
+                continue;
+            }
+            for entry in std::fs::read_dir(root).into_iter().flatten() {
+                let entry = match entry {
+                    Ok(e) => e,
+                    Err(_) => continue,
+                };
+                self.sweep_one(&entry.path(), &mut report);
+            }
+        }
+
+        // 3. Reconcile git's admin registry.
+        let prune = std::process::Command::new("git")
+            .arg("-C")
+            .arg(&self.repo_path)
+            .arg("worktree")
+            .arg("prune")
+            .arg("--verbose")
+            .env_remove("GIT_INDEX_FILE")
+            .output();
+        report.prune_succeeded = matches!(&prune, Ok(o) if o.status.success());
+        if let Ok(out) = prune {
+            if !out.status.success() {
+                let stderr = String::from_utf8_lossy(&out.stderr);
+                warn!(stderr = %stderr, "git worktree prune failed");
+            }
+        }
+
+        report.duration_ms = start.elapsed().as_millis() as u64;
+        info!(
+            swept_count = report.swept_count,
+            failed_count = report.failed_count,
+            root_owned_skipped = report.root_owned_skipped,
+            prune_succeeded = report.prune_succeeded,
+            duration_ms = report.duration_ms,
+            backlog_count = report.swept_count + report.root_owned_skipped,
+            "worktree sweep_stale complete"
+        );
+        report
+    }
+
+    fn sweep_one(&self, path: &Path, report: &mut SweepReport) {
+        let status = std::process::Command::new("git")
+            .arg("-C")
+            .arg(&self.repo_path)
+            .arg("worktree")
+            .arg("remove")
+            .arg("--force")
+            .arg(path)
+            .env_remove("GIT_INDEX_FILE")
+            .status();
+
+        if matches!(&status, Ok(s) if s.success()) {
+            report.swept_count += 1;
+            return;
+        }
+
+        match std::fs::remove_dir_all(path) {
+            Ok(_) => report.swept_count += 1,
+            Err(e) if matches!(e.kind(), std::io::ErrorKind::PermissionDenied) => {
+                warn!(
+                    path = %path.display(),
+                    "skipping root-owned worktree -- Layer 3 will clean"
+                );
+                report.root_owned_skipped += 1;
+            }
+            Err(e) => {
+                warn!(path = %path.display(), error = %e, "sweep failed");
+                report.failed_count += 1;
+            }
+        }
+    }
+ }
```

```rust
// crates/terraphim_orchestrator/src/lib.rs -- Orchestrator::new, near :786

         // ... existing construction code, including `compound_workflow` ...

+        // Sweep any worktree residue left by a previous instance
+        // before we start servicing schedules. Synchronous; finishes
+        // before any tick thread is spawned in `run()`.
+        // Per-agent root `/tmp/adf-worktrees` is hard-coded to match
+        // `create_agent_worktree` at lib.rs:5392-5438.
+        let sweep_report = compound_workflow
+            .worktree_manager()
+            .sweep_stale(&[std::path::PathBuf::from("/tmp/adf-worktrees")]);
+        if sweep_report.swept_count + sweep_report.root_owned_skipped > 10 {
+            warn!(
+                swept_count = sweep_report.swept_count,
+                root_owned_skipped = sweep_report.root_owned_skipped,
+                "large worktree backlog at startup -- prior crash storm likely"
+            );
+        }

         Ok(Self {
             // ... existing fields ...
         })
```

### 4.4 Layer 3 (#1571) -- root-privileged sweep

```bash
# scripts/adf-setup/adf-cleanup.sh
#!/bin/sh
# adf-cleanup.sh -- pre-start sweep of stale ADF worktrees.
#
# Invoked by systemd as `ExecStartPre=` for adf-orchestrator.service.
# Runs as root so it can reclaim worktree contents owned by
# sub-process container builds and other elevated agents.
#
# Cross-reference: WORKTREE_REVIEW_PREFIX in
# crates/terraphim_orchestrator/src/scope.rs. The literal "review-"
# below must stay in sync with that constant.

set -eu
umask 022

ADF_REPO_PATH="${ADF_REPO_PATH:-/data/projects/terraphim/terraphim-ai}"
ADF_WORKTREE_ROOT="${ADF_WORKTREE_ROOT:-${ADF_REPO_PATH}/.worktrees}"
ADF_AGENT_TMP_ROOT="${ADF_AGENT_TMP_ROOT:-/tmp/adf-worktrees}"

swept=0
failed=0

sweep_one() {
    target="$1"
    if [ ! -e "$target" ]; then
        return 0
    fi
    if git -C "$ADF_REPO_PATH" worktree remove --force "$target" >/dev/null 2>&1; then
        swept=$((swept + 1))
        return 0
    fi
    # Fallback: recursive removal of the worktree directory tree.
    if /bin/rm -rf -- "$target"; then
        swept=$((swept + 1))
        return 0
    fi
    failed=$((failed + 1))
    return 0
}

# 1. Compound review residue under ${ADF_WORKTREE_ROOT}/review-*.
if [ -d "$ADF_WORKTREE_ROOT" ]; then
    for entry in "$ADF_WORKTREE_ROOT"/review-*; do
        [ -e "$entry" ] || continue
        sweep_one "$entry"
    done
fi

# 2. Per-agent residue under /tmp/adf-worktrees/*.
if [ -d "$ADF_AGENT_TMP_ROOT" ]; then
    for entry in "$ADF_AGENT_TMP_ROOT"/*; do
        [ -e "$entry" ] || continue
        sweep_one "$entry"
    done
fi

# 3. Reconcile git's admin registry. Failure here is not fatal.
git -C "$ADF_REPO_PATH" worktree prune --verbose 2>&1 || true

printf 'adf-cleanup: swept=%d failed=%d repo=%s\n' \
    "$swept" "$failed" "$ADF_REPO_PATH"

exit 0
```

```bash
# scripts/adf-setup/tests/test_adf_cleanup.sh
#!/bin/sh
set -eu

THIS_DIR="$(cd "$(dirname "$0")" && pwd)"
CLEANUP_SH="${THIS_DIR}/../adf-cleanup.sh"

TMP="$(mktemp -d)"
trap '/bin/rm -rf "$TMP"' EXIT

REPO="${TMP}/repo"
WT_ROOT="${REPO}/.worktrees"
mkdir -p "$REPO"
git -C "$REPO" init -q
git -C "$REPO" commit --allow-empty -m "seed" -q

mkdir -p "$WT_ROOT/keep-me"

for i in 1 2 3; do
    git -C "$REPO" worktree add -q "${WT_ROOT}/review-test-${i}" HEAD
done

[ -d "${WT_ROOT}/review-test-1" ] || { echo "setup failed"; exit 1; }

ADF_REPO_PATH="$REPO" \
ADF_WORKTREE_ROOT="$WT_ROOT" \
ADF_AGENT_TMP_ROOT="${TMP}/agent-tmp-absent" \
    "$CLEANUP_SH"

for i in 1 2 3; do
    if [ -e "${WT_ROOT}/review-test-${i}" ]; then
        echo "FAIL: review-test-${i} still present"
        exit 1
    fi
done

[ -d "${WT_ROOT}/keep-me" ] || { echo "FAIL: keep-me removed"; exit 1; }

# Idempotency: second run.
ADF_REPO_PATH="$REPO" \
ADF_WORKTREE_ROOT="$WT_ROOT" \
ADF_AGENT_TMP_ROOT="${TMP}/agent-tmp-absent" \
    "$CLEANUP_SH"

echo "PASS: test_adf_cleanup"
```

```toml
# docs/operations/adf-orchestrator-systemd.md (drop-in snippet)
# Add to /etc/systemd/system/adf-orchestrator.service.d/cleanup.conf:

[Service]
Environment=ADF_REPO_PATH=/data/projects/terraphim/terraphim-ai
Environment=ADF_WORKTREE_ROOT=/data/projects/terraphim/terraphim-ai/.worktrees
ExecStartPre=/opt/ai-dark-factory/bin/adf-cleanup.sh
```

Deployment commands (manual until a proper installer lands):

```bash
sudo install -m 750 -o root -g root \
    scripts/adf-setup/adf-cleanup.sh \
    /opt/ai-dark-factory/bin/adf-cleanup.sh
sudo systemctl daemon-reload
sudo systemctl restart adf-orchestrator
```

---

## 5. Test strategy

### 5.1 Layer 0 (#1562) test strategy

Two test surfaces:

1. **Unit test** in `lib.rs` test module
   `test_compound_review_cursor_advances_on_cancellation`. Uses the
   existing `set_last_tick_time` test helper (`lib.rs:7712`) to
   plant a `last_tick_time` value in the past. Calls
   `check_cron_schedules` once and verifies
   `last_compound_review_fired_at` is `Some(_)`. Calls again and
   verifies the field did not change.
2. **Property test** colocated with Step 8's
   `compound_cancellation_test.rs`. Forces the storm shape: plant a
   past `last_tick_time`, call the schedule check, cancel the
   resulting future before the cursor advance would have happened
   under the **buggy** code path, verify the cursor still moved.
   This test deliberately re-introduces the bug shape so it acts as
   a regression bell.

### 5.2 Layer 1 (#1569) test strategy

Three test surfaces:

1. **`worktree_guard.rs` unit tests**, augmenting the existing
   four-test module:
   - `test_managed_guard_invokes_git_remove`.
   - `test_managed_guard_fallback_on_git_failure`.
   - `test_managed_guard_keep_disarms`.
2. **`scope.rs` test updates** for the new return type. Existing
   tests at `:609-786` use `worktree_path` directly; update to take
   the guard and `.keep()` when inspection follows.
3. **Cancellation property test**
   (`compound_cancellation_test.rs`, Step 8). The headline
   acceptance test. It encodes the full property:
   - Real git repo, real worktree, real subprocess (no mocks).
   - Parent task aborted at an arbitrary `.await` point.
   - Within 2 s: worktree dir gone, `.git/worktrees/<name>` gone,
     subprocess PID dead.

Variant of the cancellation test with Layer 0 deliberately broken
(planted past `last_tick_time` without a cursor field) asserts the
storm shape -- repeated firing -- produces **zero** leaked
worktrees with Layer 1 in place. This is the property the user's
runbook will exercise on bigbox.

### 5.3 Layer 2 (#1570) test strategy

1. **`scope.rs` unit tests** for `sweep_stale`:
   - `test_sweep_stale_empty_dir`.
   - `test_sweep_stale_no_base`.
   - `test_sweep_stale_removes_review_prefix`.
   - `test_sweep_stale_preserves_non_review_prefix`.
   - `test_sweep_stale_runs_prune`.
   - `test_sweep_stale_skips_root_owned` (Linux + root only;
     gated by `#[cfg_attr(not(target_os = "linux"), ignore)]` and
     a runtime `getuid() == 0` check that skips with explanation
     if not root).
2. **Integration test** in
   `tests/sweep_on_startup_test.rs`: pre-seed three `review-*`
   dirs under a `TempDir`-rooted config; call `Orchestrator::new`;
   assert dirs are gone before `new` returns.

### 5.4 Layer 3 (#1571) test strategy

1. **`scripts/adf-setup/tests/test_adf_cleanup.sh`** (Step 12): the
   shell test driver described above. Run from CI via a Makefile
   target or directly.
2. **CI invocation**: add the shell test to whatever CI step runs
   the Python tests under `scripts/adf-setup/tests/` (currently
   `test_migrate.py` and friends).
3. **Manual bigbox verification** (section 8 runbook): pre-seed a
   stale `review-*` directory, run `systemctl restart
   adf-orchestrator`, confirm the `ExecStartPre` line in the
   unit's journald output includes a non-zero `swept` count.

### 5.5 Cross-cutting property tests

One **acceptance property** must hold across all four layers and
is verified by the cancellation test in Step 8 plus the bigbox
runbook:

> With the schedule-cursor bug deliberately re-introduced (i.e.
> Layer 0's cursor field removed and the old `should_fire` block
> reverted), the parent task being cancelled at any `.await` point
> must leave **zero** worktree directories on disk and **zero**
> agent subprocesses alive.

This property is what the storm violated. It is encoded as a
deterministic test rather than relying on bigbox to surface
regressions days later.

---

## 6. Traceability matrix

| Step | Layer | Gitea issue | Acceptance criterion (epic #1567) | Verifying test(s) |
|-----:|------:|------------:|------------------------------------|--------------------|
| 1 | 0 | #1562 | "Cursor field exists on `Orchestrator`." | unit: field-init test |
| 2 | 0 | #1562 | "Compound schedule fires at most once per cron occurrence under reconcile-timeout cancellation." | unit: `test_compound_review_cursor_advances_on_cancellation`; property variant in Step 8 |
| 3 | 1 | #1569 | "`WorktreeGuard` invokes `git worktree remove --force` for managed worktrees." | unit: `test_managed_guard_invokes_git_remove`, `test_managed_guard_fallback_on_git_failure` |
| 4 | 1 | #1569 | "`WorktreeManager::create_worktree` returns the guard; constant `WORKTREE_REVIEW_PREFIX` exists." | existing scope tests updated; compile-time check on the constant |
| 5 | 1 | #1569 | "Compound swarm uses `JoinSet`; explicit cleanup removed; guard owned by `run`." | property: `compound_cancellation_test::test_cancellation_leaves_no_worktree` |
| 6 | 1 | #1569 | "`kill_on_drop(true)` on agent `Command`." | property: same test, subprocess-PID assertion |
| 7 | 1 | #1569 | "Drop ordering documented in code." | review-time only |
| 8 | 1 | #1569 | "Cancellation leaves zero worktrees and zero subprocesses." | `compound_cancellation_test.rs` |
| 9 | 2 | #1570 | "`sweep_stale` exists, covers both roots, returns a report." | unit: five-plus `test_sweep_stale_*` tests in `scope.rs` |
| 10 | 2 | #1570 | "Sweep runs from `Orchestrator::new` before any await." | integration: `sweep_on_startup_test.rs` |
| 11 | 3 | #1571 | "`adf-cleanup.sh` cleans `review-*` and `/tmp/adf-worktrees/*` idempotently." | shell: `test_adf_cleanup.sh` |
| 12 | 3 | #1571 | "Shell test covers happy path + idempotency." | shell: `test_adf_cleanup.sh` |
| 13 | 3 | #1571 | "Deployment doc exists; ops team has copy/paste commands." | review-time only |
| 14 | x-cut | #1567 | "Tracing fields present for Quickwit ingest." | review-time + manual log inspection |
| 15 | x-cut | #1567 | "Epic comment posted with approval link." | manual |

---

## 7. Rollout sequence and dependencies

Each layer is independently mergeable. Recommended commit / merge
order, with rationale:

### 7.1 Order

1. **Layer 0 (#1562)** -- first.
2. **Layer 3 (#1571)** -- second.
3. **Layer 1 (#1569)** -- third.
4. **Layer 2 (#1570)** -- last.

### 7.2 Rationale

- **Layer 0 first** because it caps blast radius. From the moment
  Layer 0 is deployed to bigbox, even an unrelated regression that
  drops a guard prematurely leaks **one** worktree per cron
  occurrence (24 h for nightly) rather than one per 90 s. The
  on-call team gets 200x more time-to-detect for everything that
  follows.
- **Layer 3 second** because it is hot-shippable defence: a shell
  script and a systemd `ExecStartPre=` line. It does not depend on
  Layer 1 or Layer 2 code changes and survives a worst-case
  regression of those. The script alone reduces the recovery
  runbook from research section 4.3 to a single
  `systemctl restart`.
- **Layer 1 third** because it is the primary technical fix and the
  one most worth careful review. Landing it after Layer 0 means its
  test surface (the cancellation property test in Step 8) operates
  in a regime where the storm cannot happen even if the test
  regresses.
- **Layer 2 last** because it depends on the prefix constant from
  Layer 1 (Step 4) and on the guard cleaning up the registry
  (Layers 1 and 2 share the `WorktreeGuard` API and the
  `WORKTREE_REVIEW_PREFIX` literal).

### 7.3 Inter-dependencies summary

| Layer | Depends on |
|------:|------------|
| 0 | nothing |
| 3 | nothing in tree; reads `review-` literal from the script's own comment |
| 1 | Layer 0 not required but recommended (test isolation); uses `WORKTREE_REVIEW_PREFIX` |
| 2 | Layer 1's `WORKTREE_REVIEW_PREFIX` constant; Layer 1's guard API |

The only **hard** in-tree dependency is Layer 2 on Layer 1's
`WORKTREE_REVIEW_PREFIX`. If schedule pressure forces an
out-of-order merge, Layer 2 can either inline the literal or land
the constant in a tiny prerequisite commit.

---

## 8. Verification on bigbox post-deploy

After each layer's PR is merged and the orchestrator is redeployed
to bigbox, run the corresponding verification step.

### 8.1 Layer 0 verification

```bash
# 1. Confirm the new struct field is reachable in the binary.
ssh bigbox 'strings /opt/ai-dark-factory/bin/adf | grep -c last_compound_review_fired_at'
# expected: >= 1

# 2. Tail the journal and look for the "compound review schedule
#    fired" line with the new `fire_time` field.
ssh bigbox 'journalctl -u adf-orchestrator -n 200 -f' | grep "compound review"
# expected: at most ONE log line per cron occurrence, not per 90 s.

# 3. Confirm cursor is persisted across reconcile-timeout
#    cancellations.
```

### 8.2 Layer 1 verification

```bash
# 1. Pre-seed the system with a known-good condition: zero
#    review-* directories on disk.
ssh bigbox 'ls /data/projects/terraphim/terraphim-ai/.worktrees/review-* 2>/dev/null | wc -l'
# expected: 0

# 2. Trigger a compound review manually (or wait for the nightly
#    cron). Watch the journal.
ssh bigbox 'journalctl -u adf-orchestrator -n 0 -f' | grep -E "(worktree|compound)"

# 3. Confirm the worktree is created AND removed. Look for
#    `worktree cleaned up via git` at the end of the review, with
#    `duration_ms` < 5000.
# expected: 1 created, 1 cleaned up

# 4. Forced-cancellation test: simulate a reconcile timeout by
#    sending SIGSTOP to the orchestrator for 100 s during a review.
ssh bigbox 'sudo kill -STOP $(pidof adf) && sleep 100 && sudo kill -CONT $(pidof adf)'
# Then check: 0 leaked worktrees on disk, 0 stray `claude` /
# `opencode` PIDs in the cgroup.
ssh bigbox 'systemctl status adf-orchestrator | grep "Tasks:"'
# expected: Tasks well below 100, not 2 413.
```

### 8.3 Layer 2 verification

```bash
# 1. Pre-seed three fake review-* directories before restart.
ssh bigbox 'sudo -u alex mkdir -p \
    /data/projects/terraphim/terraphim-ai/.worktrees/review-fake-{1,2,3}'

# 2. Restart the orchestrator.
ssh bigbox 'sudo systemctl restart adf-orchestrator'

# 3. Within 5 s, confirm the directories are gone and the journal
#    shows the `worktree sweep_stale complete` event with
#    swept_count=3.
ssh bigbox 'sleep 5 && ls /data/projects/terraphim/terraphim-ai/.worktrees/review-fake-* 2>/dev/null | wc -l'
# expected: 0
ssh bigbox 'journalctl -u adf-orchestrator -n 50 | grep sweep_stale'
# expected: one line with swept_count=3 prune_succeeded=true
```

### 8.4 Layer 3 verification

```bash
# 1. Pre-seed a root-owned residue (simulates the bigbox failure
#    mode that Layers 1 and 2 cannot reach).
ssh bigbox 'sudo mkdir -p \
    /data/projects/terraphim/terraphim-ai/.worktrees/review-rootowned/target && \
    sudo chown -R root:root \
    /data/projects/terraphim/terraphim-ai/.worktrees/review-rootowned'

# 2. Restart.
ssh bigbox 'sudo systemctl restart adf-orchestrator'

# 3. Confirm the `ExecStartPre` line in the journal.
ssh bigbox 'journalctl -u adf-orchestrator -n 50 | grep adf-cleanup'
# expected: "adf-cleanup: swept=1 failed=0 repo=/data/..."

# 4. Confirm the residue is gone.
ssh bigbox 'ls /data/projects/terraphim/terraphim-ai/.worktrees/review-rootowned 2>/dev/null'
# expected: empty
```

### 8.5 Whole-epic acceptance

After all four layers are deployed, run the user's stated
acceptance criterion:

```bash
# Manually introduce a bunch of stale review-* directories.
ssh bigbox 'for i in $(seq 1 20); do sudo mkdir -p \
    /data/projects/terraphim/terraphim-ai/.worktrees/review-storm-${i}; done'
ssh bigbox 'sudo systemctl restart adf-orchestrator'
ssh bigbox 'sleep 10 && ls /data/projects/terraphim/terraphim-ai/.worktrees/review-* 2>/dev/null | wc -l'
# expected: 0 (all reclaimed by a single systemctl restart, no
# human-typed cleanup commands)
```

---

## 9. Risk register

Top risks ranked by severity. Each carries an explicit mitigation
captured in the design.

| # | Risk | Severity | Mitigation |
|--:|------|----------|------------|
| 1 | Synchronous `Drop` invoking `git worktree remove` parks a tokio worker for ~1.5 s. With 692 leaked trees a startup sweep could park the whole pool. | Medium | Layer 2's `sweep_stale` is **synchronous** and runs from `Orchestrator::new` -- explicitly outside any tokio runtime. Layer 1's per-call `Drop` parks one worker per cancelled review at most. Sustained worker starvation is not possible under steady-state operation. Documented in research section 3.2. |
| 2 | `git worktree remove --force` races with a still-live subprocess writing into the worktree. Subprocess may see file-not-found errors or write into a now-anonymous inode. | Low (on Linux) | Drop ordering (Step 7): JoinSet aborts **before** the guard's git-remove runs. `kill_on_drop(true)` then sends SIGKILL to each subprocess when the runtime polls the aborted tasks. On Linux a recursive directory removal unlinks dentries; subprocesses with open file descriptors continue writing to anonymous inodes that are reclaimed when the descriptor closes. State this explicitly in code comments and PR description. |
| 3 | `sweep_stale` removes a worktree that another orchestrator instance is using (e.g. during a botched dual-deploy). | Low | The systemd unit has `Type=simple` with no instance multiplexing; on bigbox only one orchestrator runs at a time (mutex enforced by the unit). Documented in research section 3.5.3. If multi-instance is ever needed, sweep gains a pidfile-aware liveness check; not required today. |
| 4 | `kill_on_drop(true)` interacts badly with a deliberate worktree hand-off case (e.g. future PR-review agents that hand off the worktree to a follow-on subprocess). | Low | The hand-off pattern is not implemented today. When it is implemented, the relevant call site uses `WorktreeGuard::keep()` to disarm the guard **and** does not rely on `Child` being held in a JoinSet wrapper. Documented as a YAGNI-allowed surface (section 2.2). |
| 5 | Layer 3's shell script's literal `review-` drifts from `WORKTREE_REVIEW_PREFIX` in `scope.rs`. | Low | The constant carries a documentation comment requiring synchronised changes; the shell script carries the same comment in reverse direction. A future test could `grep -q '"review-"' scope.rs` from the shell test to catch drift mechanically; recommended but not in scope. |
| 6 | `Orchestrator::new` blocks longer than systemd's `TimeoutStartSec` if `sweep_stale` finds thousands of stale entries. | Low | The cardinality is bounded by disk capacity; even 1 000 entries at 1.5 s each is ~25 minutes -- worse than systemd's default 90 s `TimeoutStartSec`. Mitigation: emit a `warn!` if `swept_count > 100` and document raising the unit's `TimeoutStartSec` for the first post-storm restart only. After the first restart, residue is bounded by Layer 1's effectiveness. |

---

## 10. Out of scope (deferred)

Carrying forward from research section 7 and adding items that
emerged during design:

- **Async `Drop`** (research 7.1). Unstable; revisit if Rust
  stabilises `AsyncDrop`.
- **Switching the `cron` crate** (research 7.2).
- **Per-agent worktree namespacing** (research 7.3).
- **`sccache` lifecycle hooks** (research 7.4).
- **Compound-review backpressure beyond cursor advance**
  (research 7.5).
- **Existing per-agent guard registry leak** (research 7.6; Step
  3 rationale): folding the per-agent path at `lib.rs:5392-5438`
  onto the new `for_managed` constructor would close the registry
  leak but expands review surface for #1569. Recommended as a
  sibling follow-up issue.
- **In-tree systemd installer**: `scripts/adf-setup/` deploys TOML
  and Python today; introducing a real install script is a larger
  ops investment. Deferred to a dedicated issue, documented in
  Step 13.
- **Sweep-vs-shell-script literal drift test**: a grep-based check
  that the shell script's `review-*` and `scope.rs`'s
  `WORKTREE_REVIEW_PREFIX` match. Recommended but adds shell-test
  complexity; deferred.
- **CI sweep test as non-root**: a robust permission-denied test
  for `sweep_stale` is hard to write in a portable CI environment
  without root. Manual verification on bigbox covers it (section
  8.4).

---

## 11. Approval gate

Implementation (Phase 3) must not begin until human approval lands
on epic **terraphim/terraphim-ai#1567**.

Approval is a comment on the epic confirming:

1. The decisions in section 2.4 (research section 5 resolutions)
   are accepted.
2. The rollout order in section 7 is accepted (Layer 0 -> Layer 3
   -> Layer 1 -> Layer 2).
3. The acceptance property in section 5.5 is the right contract.
4. The risk register in section 9 is acceptable (in particular
   risk #2 on the unlink-vs-live-write race, which is benign on
   Linux but non-obvious).

On approval, Phase 3 (Implementation) proceeds step-by-step per
section 3, one commit per step, with each commit referencing the
relevant Gitea issue:

- Steps 1-2 -> `Refs #1562`.
- Steps 3-8 -> `Refs #1569`.
- Steps 9-10 -> `Refs #1570`.
- Steps 11-13 -> `Refs #1571`.
- Steps 14-15 -> `Refs #1567`.

The cancellation property test (Step 8) is the headline acceptance
artefact; reviewers should expect to spend most of their review
time there.

---

## Phase 2 deliverable

This document is the Phase 2 deliverable. It captures the
four-layer design, file-level diffs, test plans, traceability,
rollout order, risk register, and the post-deploy verification
runbook. No code is changed; no PR is opened.

Next step: human review and approval comment on epic
terraphim/terraphim-ai#1567 at https://git.terraphim.cloud.
