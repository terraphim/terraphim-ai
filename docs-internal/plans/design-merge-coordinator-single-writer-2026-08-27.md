# Design: Merge-Coordinator Single-Writer (cross-process singleton lease + pre-merge fresh-state precondition)

**Status**: APPROVED FOR IMPLEMENTATION — B1 resolved by
`terraphim/terraphim-agents#136` (KLS-approved design `09e619bb`); B2 resolved by
merging PR #3130 as `0ee58c491`. The coordinated merge/deploy order is frozen in
§13. No production code changed in the design phase.
**Date**: 2026-08-27
**Issue**: terraphim/terraphim-ai `#3295` (producer for digital-twins `#165`)
**Evidence base**: `23445b9e1862104a1d68d6a155b44ef7533483df`
**Implementation foundation**: `0ee58c491b681790c0be25130b3dfed175f51d21` (PR #3130 merged)
**Author**: design/research agent (session-scoped, no-commit)

---

## 1. Problem Statement

Two distinct defects are delivered under one coherent issue:

**P1 — LLM-agent cross-process duplicate spawn.**
`AgentOrchestrator.active_agents` is a process-local `HashMap<String, ManagedAgent>`
keyed by agent *name only* (`crates/terraphim_orchestrator/src/lib.rs:243`). When two
orchestrator processes are configured for the same project (e.g. the systemd
`adf-orchestrator.service` instance plus a second instance — same fleet TOML, same
cron), each process independently passes its own "already active?" filter
(`crates/terraphim_orchestrator/src/scheduling_impl.rs:30-33`,
`crates/terraphim_orchestrator/src/reconcile_impl.rs:1472`) and each spawns its own
`merge-coordinator` LLM agent. Both agents then race on Gitea: merging the same PRs,
closing the same issues, posting duplicate verdict comments. The agent's only dedup is
a 6-hour self-cooldown read from Gitea comments
(`scripts/adf-setup/agents/merge-coordinator.toml:85-92`) — a TOCTOU check both
duplicates pass before either has posted.

**P2 — Standalone binary: global lock + stale-merge window.**
The cron-invoked `merge-coordinator` binary takes a single global lock at
`/tmp/merge-coordinator.lock` (`crates/terraphim_merge_coordinator/src/main.rs:23`).
This (a) is *not* acquired by the LLM agents of P1, so it cannot prevent P1, and
(b) serializes unrelated `owner/repo` targets because the path carries no project
identity. Worse, the lock uses timestamp-based forced stealing
(`crates/terraphim_merge_coordinator/src/pid_lock.rs:62-65`): a legitimate run longer
than `LOCK_STALE_SECS = 30` causes the next invocation to *block indefinitely* on
`FileExt::lock_exclusive` (pile-up), and the world-writable `/tmp` location invites
unlink/squat attacks that defeat the lock entirely (see §8). Separately,
`evaluate_all` snapshots PR state and the later `merge_and_close` call trusts that
snapshot: `merge_and_close` checks only the previously computed verdict
(`crates/terraphim_merge_coordinator/src/evaluator.rs:157-178`) with no fresh
open/mergeable/head-SHA precondition, so a PR merged, closed, or force-pushed between
evaluation and merge is merged anyway (double-merge attempt, merge of a moved head).

**Relationship**: P1 is about *who is allowed to write* (single writer per key);
P2 is about *what state a writer may act on* (fresh precondition) plus the binary's
own lock scoping. One delivery: a shared kernel-held lease-lock primitive fixes the
singleton problem for both surfaces, and a fresh-state precondition makes every
merge attempt idempotent-safe regardless of which writer wins.

---

## 2. Evidence (file:line at base `23445b9e1`)

| # | Location | Fact |
|---|----------|------|
| E1 | `crates/terraphim_orchestrator/src/lib.rs:243` | `active_agents: HashMap<String, ManagedAgent>` — name-keyed, process-local |
| E2 | `crates/terraphim_orchestrator/src/lib.rs:377-384` | `agent_key(def) -> (String, String)` — canonical `(project_id, name)` already exists (used for restart counts) |
| E3 | `crates/terraphim_orchestrator/src/lib.rs:187-215` | `ManagedAgent` struct — natural home for a lease guard field (RAII) |
| E4 | `crates/terraphim_orchestrator/src/spawn_impl.rs:36-54` | project pause gate — *file-based cross-process gate precedent* |
| E5 | `crates/terraphim_orchestrator/src/spawn_impl.rs:491-505` | concurrency gate — process-local `acquire_any` |
| E6 | `crates/terraphim_orchestrator/src/spawn_impl.rs:545-552` | actual child spawn (`spawn_with_fallback`) — lease must be held *before* this |
| E7 | `crates/terraphim_orchestrator/src/spawn_impl.rs:568-589` | `active_agents.insert` — guard moves into `ManagedAgent` here |
| E8 | `crates/terraphim_orchestrator/src/reconcile_impl.rs:500` | `poll_agent_exits` — main exit path |
| E9 | `crates/terraphim_orchestrator/src/reconcile_impl.rs:412, :535, :1094`; `src/lib.rs:1949` (`stop_agent`), `src/lib.rs:1990` (test helper) | every `active_agents.remove` site — guard Drop releases lease automatically; only the D1 re-key touches them (no lease-specific edits) |
| E10 | `crates/terraphim_orchestrator/src/project_control.rs:24,34-39` | `DEFAULT_PAUSE_DIR = /opt/ai-dark-factory/data/pause`, sentinel-file precedent |
| E11 | `crates/terraphim_orchestrator/src/config.rs:163-173` | `pause_dir: Option<PathBuf>` config precedent; `config.rs:1600` `validate()` for load-time checks |
| E12 | `crates/terraphim_orchestrator/src/config.rs:717-827` | `AgentDefinition` (serde, no `deny_unknown_fields`) — new `#[serde(default)]` fields are TOML-backward-compatible |
| E13 | `crates/terraphim_orchestrator/src/lib.rs:2064` | `pause_dir_for_test()` — test-accessor precedent |
| E14 | `crates/terraphim_orchestrator/src/lib.rs:2121-2143` | `requires_isolated_worktree` — `model=None` + non-LLM cli (e.g. `bash`) ⇒ no worktree in tests |
| E15 | `scripts/adf-setup/agents/merge-coordinator.toml:12-21` | LLM agent: `layer="Growth"`, `schedule="0 */4 * * *"`, `project="terraphim"` |
| E16 | `scripts/adf-setup/agents/merge-coordinator.toml:166-167` | `gtr merge-pull` with no head/state precondition |
| E17 | `crates/terraphim_merge_coordinator/src/main.rs:23-24` | `LOCK_PATH="/tmp/merge-coordinator.lock"`, `LOCK_STALE_SECS=30` |
| E18 | `crates/terraphim_merge_coordinator/src/main.rs:51-64` | lock acquisition; `LockHeld ⇒ ExitCode::Critical` |
| E19 | `crates/terraphim_merge_coordinator/src/pid_lock.rs:45-67` | `try_lock_exclusive`; stale-steal path ends in *blocking* `lock_exclusive` |
| E20 | `crates/terraphim_merge_coordinator/src/evaluator.rs:31-43` | `evaluate_all` snapshot |
| E21 | `crates/terraphim_merge_coordinator/src/evaluator.rs:16-27,46-82` | `PrEvaluation` has no `head_sha`; `evaluate_one` reads `pr.head_sha` only for CI classification |
| E22 | `crates/terraphim_merge_coordinator/src/evaluator.rs:157-178` | `merge_and_close` acts on stale verdict; `gitea.merge_pr` at `:178` |
| E23 | `crates/terraphim_merge_coordinator/src/gitea.rs:94-107` | `merge_pr` body is `{"Do":"merge"}` only; client has **no** `get_pr` |
| E24 | `crates/terraphim_merge_coordinator/src/types.rs:155-163` | `LockHeld { pid, age_secs }`; `MergeOutcome::Skipped(String)` at `:64-65` |
| E25 | `crates/terraphim_orchestrator/src/pr_handlers_impl.rs:517,721,1045` | PR-gate agents insert into `active_agents` via a *separate* path (per-PR concurrency by design) |
| E26 | `crates/terraphim_orchestrator/src/lib_tests.rs:587-617` | `test_reconcile_detects_agent_exit` — lifecycle test pattern to reuse |
| E27 | merged PR #3130, reviewed head `eab0785b2`, main merge `0ee58c491` | `GiteaOperations` trait + stateful `FakeGitea` foundation landed (project treats stateful fakes as non-mocks) |
| E28 | `adf-orchestrator.service:45-46` | `ProtectSystem=strict` + `ReadWritePaths=/opt/ai-dark-factory ...` — a lock dir under `/opt/ai-dark-factory` is writable by the service |
| E29 | `crates/terraphim_spawner/src/lib.rs:1044` + `infer_args("bash")` | task string runs as `bash -c "<task>"` — enables marker-file/counting test agents |

flock semantics relied upon (Linux `flock(2)`): locks are attached to the open file
description; a second `open` + `try_lock` **in the same process** is denied; the
kernel releases the lock on last fd close (process crash, `SIGKILL`, host reboot).

---

## 3. Design Decisions

### D1 — Canonical identity `(project_id, agent_name)`; mandatory `active_agents` re-key

**Mandatory re-key.** `AgentOrchestrator.active_agents` is re-keyed from name-only
to the canonical pair via `agent_key(def)` (E2) —
`HashMap<(String, String), ManagedAgent>` — and **every access site is re-keyed**:
the declaration (`lib.rs:243`), the activity filters (`scheduling_impl.rs:30-33`,
`reconcile_impl.rs:1472`), the spawn insert (`spawn_impl.rs:568-589`), every
remove/stop site (E9), the PR-handler insert path (E25), and the test helpers
(`lib.rs:1990`). This is *mandatory* so same-name agents in different projects
(e.g. `terraphim/merge-coordinator` and `digital-twins/merge-coordinator`) can
coexist **inside one process** and across processes — the name-keyed map silently
filters the second one as "already active".

**Identity derivation.** Reuse `agent_key(def)` (E2):
`project_id = def.project.unwrap_or("__global__")`
(`crates/terraphim_orchestrator/src/dispatcher.rs:10`). Derived lease file name:

```
<agent_lock_dir>/agent-{project}--{agent_name}.lock
```

**Reserved-id mapping.** The reserved legacy id `__global__` cannot satisfy the
charset below; it maps to the safe disk component `global` before any path is
built — `<agent_lock_dir>/agent-global--{agent_name}.lock`. Only that one
reserved value is mapped; every non-reserved project, agent, and explicit-key
component must satisfy `^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$` (same rule for the
binary's `owner`/`repo` path components). Violations for singleton agents are
rejected at config load (`validate()`, E11) — startup failure, not a runtime
surprise. This kills path-traversal (`../`, `/`, NUL) through operator config.

Optional per-agent override `singleton_key: Option<String>` replaces the derived
`agent-…` component verbatim (validated against the same regex), enabling
cross-subsystem mutual exclusion: the LLM agent can be pinned to the *same* lease
file the standalone binary uses for its repo (`merge-coordinator-{owner}--{repo}.lock`),
so the binary and the LLM agent can never run concurrently for the same repo even
though they are configured in different files.

### D2 — Which agents are singleton

Opt-in per agent: `singleton: bool = false` on `AgentDefinition` (E12). Only
`scripts/adf-setup/agents/merge-coordinator.toml` sets `singleton = true` in this
delivery. Everything else is untouched (INV7). Safety/Core/Growth layers are not
auto-singleton — per-PR gate agents must stay concurrent (E25).

### D3 — Kernel-held advisory lease, no timestamp stealing, no unlink

- **Acquire**: `create_dir_all(dir)` → `OpenOptions::read+write+create+truncate(false)`
  → `fs4::FileExt::try_lock_exclusive`. Non-blocking, single attempt.
- **Contention** (`ErrorKind::WouldBlock`): benign skip — read payload best-effort for
  the holder pid, log, return `Ok(())` from the spawn gate (same skip-not-error
  contract as the concurrency gate, E5).
- **Success**: rewrite payload (`set_len(0)`, seek 0, `pid=<pid> acquired=<unix_secs>`) — informational only, never used for correctness.
- **Never steal**: the `stale_after_secs` machinery (E19) is deleted. Crash recovery
  is exclusively kernel fd-close. This also removes the blocking-pile-up bug.
- **Never unlink** lock files (no `/tmp`-style unlink race: A holds inode X; an
  unlinker makes path point to inode Y; B and C both lock Y while A believes it is
  exclusive). Residual files are a few bytes and inert.

### D4 — Lifetime ownership (acquire-before-spawn, RAII release)

Acquire in `spawn_agent_with_event` immediately after the pause gate (E4), with an
in-process fast path (`active_agents.contains_key(&agent_key(def))` ⇒ skip) in
front. The guard lives in a local binding; on *any* early return (budget,
pre-check, spawn failure at E6) Rust drops it and the lease is released — no
double-hold on failed spawns. On success the guard is moved into
`ManagedAgent.singleton_lease` (E3/E7). Every exit path (E9) removes/drops
`ManagedAgent`, so the lease releases with zero *lease-specific* changes to the
remove sites — their only edit is the D1 re-key of the lookup. Orchestrator death
releases via kernel.

### D5 — Standalone binary lock scoping

```
path = <MERGE_COORDINATOR_LOCK_DIR or "/opt/ai-dark-factory/data/locks"> /
       merge-coordinator-{owner}--{repo}.lock
```

Same primitive as D3 (shared crate, component C1). `LockHeld` maps to
`ExitCode::Success` with a `lock.held` jsonlog event (was `Critical`, E18):
single-flight overlap is benign — another runner *is* doing the work — and the old
Critical mapping generated cron noise for normal overlap. Unwritable/invalid lock dir
is a hard `Critical` (fail-closed, distinct from contention).

### D6 — Fresh-state pre-merge precondition (P2)

`PrEvaluation` gains `head_sha: Option<String>` (captured in `evaluate_one`, E21).
`merge_and_close` (E22), after the verdict match and *before* `merge_pr`:

1. `fresh = gitea.get_pr(owner, repo, eval.pr_index).await?` — transport/API error
   propagates as `Err` (fail-closed: no merge on uncertain state). New
   `GiteaClient::get_pr` (E23) `GET /repos/{owner}/{repo}/pulls/{index}`.
2. `fresh.state != "open"` ⇒ `Skipped("state changed to '{state}' after evaluation")`.
3. `fresh.mergeable != Some(true)` ⇒ `Skipped("no longer mergeable after evaluation")`.
4. Head-SHA verification is **mandatory for automatic merge**: both the evaluated
   SHA (`eval.head_sha`) and the fresh SHA (`fresh.head_sha`) must be present;
   *either* one missing ⇒ `Skipped("head SHA unavailable for verification")`
   (fail-closed — no legacy `None` bypass). Both present and unequal ⇒
   `Skipped("head moved {old}->{new} after evaluation")`. Both present and equal
   is the only state that proceeds to merge.

The precise reason flows to the existing `pr.skipped` jsonlog event (main.rs:118-124)
unchanged. Optional hardening (follow-up, out of scope): pass Gitea's merge `head`
parameter if the deployed Gitea version supports it in `MergePullRequestOption`.

### D7 — Test seam for P2

Adopt the `GiteaOperations` trait + stateful fake direction of unmerged
`task/2892-gitea-operations-trait` (E27): tests mutate the fake's PR table *between*
`evaluate_all` and `merge_and_close` and assert `merge_call_count == 0`. If #2892 has
not landed when implementation starts, implement the minimal trait seam locally in
this task (same shape: `list_open_prs`, `list_pr_files`, `merge_pr`, `close_issue`,
`get_commit_status` + new `get_pr`) and coordinate the merge order (see Blockers).
No mocks: the fake holds real state and real counters.

---

## 4. Invariants

- **INV1** — At most one live instance per singleton key (`(project_id, agent_name)`
  derived, or explicit `singleton_key`) across *all* orchestrator processes sharing
  `agent_lock_dir`; enforced by exclusive `flock`.
- **INV2** — Lease lifetime == agent run lifetime: acquired before child spawn,
  released on every exit/remove path via guard Drop; failed spawns never leak a lease.
- **INV3** — No timestamp-based stealing; recovery is exclusively kernel fd-close
  semantics (survives `SIGKILL`, orchestrator crash, reboot).
- **INV4** — The software never unlinks lease files.
- **INV5** — Fail-closed: inability to create/open/lock the lease for a singleton
  agent aborts the spawn with `Err` (logged by callers); mere *contention* is a
  benign `Ok(())` skip. Binary: invalid/unwritable lock dir ⇒ `Critical`; contention
  ⇒ `Success` + `lock.held`.
- **INV6** — The binary issues no merge API call unless the freshly fetched PR is
  `open`, `mergeable == true`, and **both** the evaluated and fresh head SHAs are
  present and equal (either SHA missing ⇒ skip, fail-closed; unequal ⇒ skip as
  head moved).
- **INV7** — Non-singleton agents: zero behavior change; no lease files created.
- **INV8** — Every skip is precise and observable (structured reason strings; log
  events carry holder pid + lease path).

---

## 5. Components (≤5, scoped)

**C1 — `crates/terraphim_lockfile` (new tiny workspace crate).**
`LeaseLock::acquire(dir, key) -> Result<LeaseGuard, LeaseError>`; `LeaseError::{LockHeld{holder_pid}, Io}`; payload write; charset validator `validate_key_component`. RAII `LeaseGuard` (Drop = unlock, best-effort). Deps: `fs4` (0.13.1), `tracing`, `thiserror`; payload uses only `std::time`. ~150 LOC + unit tests. Replaces `pid_lock.rs` internals (that module becomes a thin wrapper or is deleted; its public surface shrinks to what `main.rs` needs).

**C2 — Orchestrator integration (dual-repo delivery).**
This component explicitly spans **canonical production orchestrator delivery in
the `terraphim-agents` repo** (where the deployed `adf` binary ships, AGENTS.md
Bigbox rule 3) **plus synchronized integration in `terraphim-ai`**; the two PRs
land as one coordinated delivery and **neither PR alone may claim readiness**
(see B1). `terraphim-ai` contents: `config.rs`: `agent_lock_dir: Option<PathBuf>`
(default `/opt/ai-dark-factory/data/locks`), `AgentDefinition.singleton: bool` +
`singleton_key: Option<String>` (both `#[serde(default)]`), load-time charset
validation in `validate()`. `lib.rs`: `AgentOrchestrator.agent_lock_dir: PathBuf`
(resolve next to `pause_dir`, E11-style), the D1 re-key of `active_agents` to
`agent_key(def)`, `ManagedAgent.singleton_lease: Option<LeaseGuard>`,
`agent_lock_dir_for_test()` accessor (E13 precedent), `#[cfg(test)] mod
singleton_lock_tests;`. `spawn_impl.rs`: singleton gate after pause gate (D4).
Remove sites are touched only by the D1 re-key (E9).

**C3 — merge-coordinator crate.**
`resolve_lock_path(dir, owner, repo)` + `MERGE_COORDINATOR_LOCK_DIR` env (D5); delete stale-steal; `LockHeld ⇒ Success` mapping (extract `fn exit_code_for_run_outcome` for unit testing); `PrEvaluation.head_sha`; `GiteaClient::get_pr`; fresh-state precondition in `merge_and_close` (D6); trait seam per D7 if #2892 absent.

**C4 — Fleet template.**
`scripts/adf-setup/agents/merge-coordinator.toml`: add `singleton = true` (and, if cross-subsystem exclusion with the cron binary is desired on the host, `singleton_key = "merge-coordinator-terraphim--terraphim-ai"`).

**C5 — Tests + docs.**
New `singleton_lock_tests.rs` (orchestrator), extended `evaluator.rs`/`pid_lock.rs`/new `lock_path` tests (merge_coordinator), this design doc, ops runbook section (§9).

---

## 6. Implementation Steps (ordered, TDD)

1. **C1 unit tests RED-first** (write the failing tests *before* the primitive —
   compile-RED at base, then behavioural RED once the crate skeleton exists — then
   implement to GREEN): acquire/payload/guard-drop; second-open-denied (same
   process); fd-close releases; `validate_key_component` accept/reject table
   (`../`, empty, >127, non-ASCII, `a.b-c_9` ok; raw `__global__` rejected as a
   component — only D1's mapping turns it into `global`).
2. **RED T1** (below) in orchestrator — fails to compile at base (no `singleton`
   field), then fails behaviorally once fields exist.
3. **C2 config + gate + guard field + D1 re-key of `active_agents` and every
   access site**; mechanical update of the 41 `AgentDefinition` literals / 19
   `OrchestratorConfig` literals in tests (`singleton: false`,
   `singleton_key: None`, `agent_lock_dir: None`).
4. **RED T2–T7** (incl. companion T2b); confirm existing `lib_tests` unchanged
   (INV7 via T6).
5. **RED M5–M7** in merge_coordinator (lock scoping + no-steal + exit mapping);
   replace removed steal test (`second_acquire_after_stale_threshold_steals_lock`).
6. **RED M1–M4** (fresh-state precondition) via D7 seam.
7. **C4** template flag.
8. Docs/runbook; full verification ladder (§11).

---

## 7. Test Plan (deterministic RED tests)

### 7.1 Orchestrator singleton (`crates/terraphim_orchestrator/src/singleton_lock_tests.rs`)

Shared harness: `tempfile::tempdir()` lock dir; agent `name="merge-coordinator"`,
`layer=Growth`, `cli_tool="bash"`, `model=None` (no worktree, E14), `project=Some(
"terraphim")`, `singleton=true`; task embeds an absolute marker path (E29:
`bash -c`). Two *independent* `AgentOrchestrator::new(config)` instances sharing the
dir. Long-lived tasks use `sleep 5` so the first holder is provably still running.

- **T1 (FIRST RED) `singleton_second_instance_same_key_skips_spawn`** —
  task `echo run >> {marker}; sleep 5`. A spawns ⇒ `is_agent_active` true; B spawns ⇒
  returns `Ok(())`, `!B.is_agent_active(...)`, and marker has **exactly 1** line after
  settling. (At base: compile-RED on `singleton`/`agent_lock_dir`; behaviorally B
  spawns a duplicate ⇒ 2 lines.)
- **T2 `singleton_same_name_different_projects_both_spawn_in_one_process`** — a
  *single* `AgentOrchestrator` instance spawns A(project `terraphim`) and B(project
  `digital-twins`), same agent name `merge-coordinator`: **both** are active
  simultaneously, both markers written, two distinct lease files exist under the
  dir. Proves the D1 re-key end-to-end: at base the name-keyed `active_agents`
  filters B as "already active" (behavioural RED).
- **T2b (companion, cross-process) `singleton_same_name_different_projects_both_spawn_across_processes`**
  — same pair, A and B in *separate* `AgentOrchestrator` processes sharing the
  lock dir: both spawn, both markers written, distinct lease files (cross-process
  half of the same-name/different-project guarantee).
- **T3 `singleton_lease_released_on_normal_exit`** — task `exit 0`; spawn in A;
  loop `poll_agent_exits()` (E8/E26 pattern) until inactive (deadline 5 s); then B
  spawns successfully.
- **T4 `singleton_lease_released_when_holder_fd_closes`** (kernel crash semantics) —
  test opens the lease path and `try_lock_exclusive`s it directly (simulating any
  holder); orchestrator spawn is skipped; `drop(file)`; orchestrator spawn now
  succeeds. (Optional Linux-only variant: `flock(1)` child killed with `SIGKILL`.)
- **T5 `singleton_same_process_second_spawn_skips`** — one instance, two
  `spawn_agent_for_test` calls; second `Ok(())`; marker count still 1. Extends the
  callers' existing activity filters (post-D1, keyed by `agent_key(def)`) down to
  the spawn choke point.
- **T6 `non_singleton_agents_create_no_lock_files`** — `singleton=false`: both
  instances spawn; lock dir remains **empty** (INV7).
- **T7 `singleton_lock_dir_unwritable_fails_closed`** — `agent_lock_dir` pointed at
  a path occupied by a regular *file*: `spawn_agent` returns `Err` (fail-closed,
  INV5), no child spawned.

### 7.2 Standalone binary lock (merge_coordinator)

- **M5 `resolve_lock_path_scopes_by_owner_repo`** — `(terraphim,terraphim-ai)` ≠
  `(other,repo)` paths; charset violations rejected.
- **M6 `lock_contention_returns_lock_held_without_steal`** — holder present, payload
  forged with a 100 s-old timestamp: acquire still `LockHeld` (stealing removed;
  replaces the deleted steal test).
- **M7 `lock_held_maps_to_success_exit_semantics`** — pure mapping fn:
  `LockHeld ⇒ ExitCode::Success`, `Io ⇒ Critical`.

### 7.3 Fresh-state precondition (merge_coordinator, via D7 fake)

- **M1 (FIRST RED for P2) `merge_and_close_skips_when_pr_closed_after_evaluation`** —
  PR 7 open/mergeable/head `aaa…`; `evaluate_all` ⇒ `Merge`; fake flips state to
  `closed`; `merge_and_close` ⇒ `Skipped` containing `closed`, fake
  `merge_call_count == 0`.
- **M2 `merge_and_close_skips_on_head_sha_drift`** — head `aaa…`→`bbb…` ⇒
  `Skipped` contains `head moved`; `merge_call_count == 0`.
- **M2b `merge_and_close_skips_when_head_sha_unavailable`** — evaluated or fresh
  SHA `None` (either side) ⇒ `Skipped` contains `head SHA unavailable`;
  `merge_call_count == 0` (fail-closed per D6.4/INV6).
- **M3 `merge_and_close_merges_when_fresh_state_matches`** (control) —
  `merge_call_count == 1`.
- **M4 `merge_and_close_skips_when_mergeability_lost`** — `mergeable: Some(false)`
  ⇒ `Skipped`; count 0.
- **M4b `merge_and_close_fails_closed_on_get_pr_error`** — fake returns transport
  error ⇒ `Err`, count 0.

Existing suites that must remain green: `lib_tests.rs` (unchanged behavior), current
`pid_lock` tests except the replaced steal test, current `evaluator`/`gitea`/
`types`/`lib` tests.

---

## 8. Security / Fail-Closed Behavior

- **/tmp removal**: the global `/tmp/merge-coordinator.lock` disappears. `/tmp` is
  world-writable: any local user could unlink+recreate the file (new inode) so
  concurrent runs both "hold" the lock, or squat it for DoS; payload timestamps were
  attacker-writable. New default dirs (`/opt/ai-dark-factory/data/locks`) sit inside
  the deployment tree the service already owns (`ReadWritePaths`, E28; `ProtectSystem=strict` keeps the rest of the FS read-only).
- **Fail-closed matrix**: unwritable dir / IO error / open failure ⇒ spawn `Err`
  (orchestrator) or `Critical` (binary); `get_pr` failure ⇒ no merge, `Err`;
  head SHA unverifiable (evaluated or fresh SHA missing — either side) ⇒
  `Skipped`; unequal SHAs ⇒ `Skipped` (head moved); charset violation ⇒
  config load failure. Contention (`WouldBlock`) is the *only* benign skip.
- **Payload trust**: pid/timestamp/instance are diagnostics only; no correctness
  decision ever reads them (no steal). No secrets in lease files (never tokens).
- **Advisory nature**: `flock` binds only cooperating processes (our orchestrators
  and binary). A hostile local process can hold a lease (DoS) — detectable via
  `lock.held` logs + payload pid (`ps -p`), recoverable by killing the rogue holder;
  kernel then releases. Host access control is the real boundary (unchanged).
- **NFS caveat**: `flock` is unreliable on NFS; `agent_lock_dir` must be local disk.
  Documented in config docs; no runtime enforcement (single-host deployment today).

---

## 9. Observability, Cleanup, Ops Runbook

- Events: `info!` `singleton lease acquired` / `singleton lock held elsewhere;
  skipping spawn` (fields: agent, project, lock_path, holder_pid); `debug!` on
  guard drop. Binary keeps jsonlog `lock.held` (now exit 0) and gains `lock.path`.
- Live inspection: `ls /opt/ai-dark-factory/data/locks/`; `cat` a lease ⇒ holder pid
  ⇒ `ps -p <pid>` identifies the owning adf/merge-coordinator process.
- Cleanup: **none automated** (INV4). Residual lease files are inert. Manual removal
  is safe only when no orchestrator/binary runs (documented), otherwise the unlink
  race is reintroduced by hand.

---

## 10. Rollout / Deployment Smoke & Rollback

**Rollout** (respect AGENTS.md bigbox rules — git only, no scp):
1. Merge to `main` (origin then gitea, mandatory sequence).
2. **Orchestrator binary ships from `/home/alex/projects/terraphim/terraphim-agents`**
   (AGENTS.md Bigbox rule 3) — sync/cherry-pick C1+C2 there, rebuild, redeploy
   `adf` (see Blockers B1).
3. Rebuild/deploy `merge-coordinator` binary; ensure
   `/opt/ai-dark-factory/data/locks` exists (service user-writable) or set
   `MERGE_COORDINATOR_LOCK_DIR` in the cron unit.
4. Add `singleton = true` to the deployed `merge-coordinator.toml`.

**Smoke (bigbox, ≤5 min)**:
- `sudo systemctl restart adf-orchestrator`; during the next merge-coordinator run:
  lease file exists, payload pid == an `adf` child pid.
- Run the binary twice concurrently: second exits **0**, journal shows `lock.held`
  with the first's pid; per-repo file name is
  `merge-coordinator-terraphim--terraphim-ai.lock`.
- Negative: `flock <lease> -c 'sleep 300'` then trigger `@adf:merge-coordinator` ⇒
  orchestrator logs the skip; kill the `flock` holder ⇒ next cron spawns normally.
- Grep journal for `singleton lease acquired` / `skipping spawn`.

**Rollback**:
- Orchestrator: redeploy previous `adf`. Old binary ignores the new TOML keys
  (`AgentDefinition`/`OrchestratorConfig` have no `deny_unknown_fields`, E12) —
  config need not be reverted. Per-agent kill switch: delete `singleton = true`
  without redeploy.
- Binary: redeploy previous build (restores `/tmp` global lock + steal semantics —
  accepted interim); stale lease files are harmless to both versions.
- No data migration; no state to unwind.

---

## 11. Verification Ladder

1. `cargo fmt --check` on touched crates.
2. `cargo clippy -p terraphim_lockfile -p terraphim_orchestrator -p terraphim_merge_coordinator --all-targets --all-features -- -D warnings`.
3. `cargo test -p terraphim_lockfile` (primitive).
4. `cargo test -p terraphim_orchestrator singleton` (T1–T7) + full `-p terraphim_orchestrator` (regression).
5. `cargo test -p terraphim_merge_coordinator` (M1–M7 + existing).
6. `cargo test --workspace --lib --no-fail-fast` (native-CI baseline).
7. Bigbox smoke (§10).

---

## 12. Out of Scope

- Lease *gating* of the `pr_handlers_impl.rs` spawn path (E25) — PR-gate agents
  are per-PR concurrent by design; none are singleton today. (Their
  `active_agents` insert sites are still re-keyed per D1.)
- Gitea merge-API `head` parameter conditioning (server-version dependent; follow-up
  hardening after D6 lands).
- Multi-host / distributed locking (flock is single-host; document, don't solve).
- Rewriting the LLM agent's shell task (verdict parsing, `recently_evaluated`
  TOCTOU) — indirectly protected by the lease; script content unchanged.
- `dual_mode.rs` alternate orchestrator; automatic lease-file garbage collection.
  (The `active_agents` re-key to `(project_id, agent_name)` is **in scope** —
  mandatory, D1.)
- Any change to remote-sync/push/tracker workflows.

---

## 13. Blockers / Coordination Risks

- **B1 — Deployment duality (RESOLVED FOR DESIGN)**: production `adf` is built
  from `terraphim-agents` (AGENTS.md Bigbox rule 3). Canonical child issue
  `terraphim/terraphim-agents#136` is linked to parent `#3295`. C2 lands there via
  git only (commit/PR/pull); **no uncommitted copying (scp/cp) between repos**.
  Deployment readiness still requires both reviewed exact heads.
- **B2 — Trait foundation (RESOLVED)**: PR #3130 was rebased, independently
  reviewed at exact head `eab0785b2`, passed required PR CI plus 43/43 tests and
  strict Clippy, and merged as `0ee58c491`; issue #2892 is closed. D7 now extends
  the landed `GiteaOperations`/stateful-fake seam.
- **B2a — Coordinated order (FROZEN)**: implement/review both legs; merge and deploy
  the terraphim-ai standalone binary + template first, then merge/deploy the canonical
  terraphim-agents `adf` leg. Claim end-to-end readiness only after both reviewed exact
  heads and the shared-lock smoke. Roll back in reverse order (`adf` first, standalone
  binary second).
- **B3 — Host layout assumption**: default lock dirs assume the
  `/opt/ai-dark-factory` deployment tree; cron contexts without it must set
  `MERGE_COORDINATOR_LOCK_DIR` (fail-closed otherwise — visible, not silent).
- **B4 — Monitoring semantics change**: binary `LockHeld` exit 2→0 (D5). Any alert
  keyed on exit code 2 for overlap must be re-pointed at the `lock.held` event.
- **B5 — Test churn**: 41 `AgentDefinition` + 19 `OrchestratorConfig` literals in
  orchestrator tests need mechanical field additions (no `..Default::default()` in
  most) — sized, mechanical, but review-noisy.

---

## 14. Summary of Exact Seams & First RED Tests

**Seams**
1. **D1 re-key of all active-agent accesses**: `active_agents` and every access
   site move from name-only to canonical `agent_key(def)` =
   `(project_id, agent_name)` — declaration `lib.rs:243`, activity filters
   `scheduling_impl.rs:30-33` + `reconcile_impl.rs:1472`, spawn insert
   `spawn_impl.rs:568-589`, remove/stop sites (E9: `reconcile_impl.rs:412/535/1094`,
   `lib.rs:1949/1990`), PR-handler inserts (E25).
2. `spawn_impl.rs:46+` — singleton gate (after pause gate), local guard → moved into `ManagedAgent` at `spawn_impl.rs:568`.
3. `lib.rs:187-215` — `ManagedAgent.singleton_lease`; releases free at the (re-keyed) remove sites.
4. `config.rs:163-173` — `agent_lock_dir`; `config.rs:717-827` — `singleton`/`singleton_key`; `config.rs:1600` — charset validation + `__global__`→`global` reserved mapping.
5. `main.rs:23-24,51-64` + `pid_lock.rs` — binary lock scoping/no-steal/exit mapping.
6. `evaluator.rs:157-178` + `gitea.rs` (`get_pr`) + `types.rs` (`PrEvaluation.head_sha`) — fresh-state precondition with mandatory exact heads.
7. `scripts/adf-setup/agents/merge-coordinator.toml` — `singleton = true`.

**First RED tests**
- P1: `singleton_lock_tests.rs::singleton_second_instance_same_key_skips_spawn` (T1).
- P2: `evaluator` tests `merge_and_close_skips_when_pr_closed_after_evaluation` (M1).
