# Phase 2 Design: Spawn-Context Isolation Fix

**Skill**: disciplined-design
**Issue**: 1806
**Date**: 2026-05-29

---

## 5/25 Rule: Explicitly Out of Scope

| # | Thing not fixed | Reason |
|---|----------------|--------|
| 1 | `build_spawn_context_for_agent` signature | Adding a `working_dir` parameter would cascade changes across all 9 call sites and change a stable internal API; the two-line patch after the call is the minimal, contained fix |
| 2 | `Provider.working_dir` redundancy | After the fix, `agent_working_dir` is stored in both `spawn_ctx.working_dir` and `Provider.working_dir`; removing the Provider field would touch the spawner config path and risk breaking fallback behaviour for non-project agents |
| 3 | Haiku/review-tier worktree isolation | Review agents intentionally skip isolation (`needs_isolation = false`); this is a correct design choice, not a bug; changing it would increase resource consumption and is out of scope |

---

## Problem Summary

`build_spawn_context_for_agent` constructs a `SpawnContext` whose `working_dir` field is set to the project root (`project.working_dir`) because at that point the worktree path has not yet been computed. Immediately after that call, the orchestrator allocates a git worktree and stores the resulting path in `agent_working_dir`, then places it on `Provider.working_dir`. However, the spawner's priority rule gives `spawn_ctx.working_dir` the highest precedence over `config.working_dir`, so the stale project-root value silently wins and the agent process is launched in the project root instead of the isolated worktree. The env var `ADF_WORKING_DIR` received the same stale value, so shell scripts inside the agent also operated on the live repository.

---

## Architecture

### Before (bug)

```
build_spawn_context_for_agent()
  +-- spawn_ctx.working_dir = project_root   <-- stale, set too early
  +-- ADF_WORKING_DIR      = project_root

create_agent_worktree()
  +-- agent_working_dir    = /.../.worktrees/agent-xyz

Provider.working_dir      = agent_working_dir   (correct)
spawn_ctx.working_dir     = project_root        (wins over Provider)
                                 |
                                 v
spawner: ctx.working_dir wins  --> agent runs in PROJECT ROOT  [WRONG]
```

### After (fix)

```
build_spawn_context_for_agent()
  +-- spawn_ctx.working_dir = project_root   (temporary, will be overwritten)

create_agent_worktree()
  +-- agent_working_dir    = /.../.worktrees/agent-xyz

spawn_ctx.working_dir     = agent_working_dir   [OVERWRITE - 1 line]
ADF_WORKING_DIR           = agent_working_dir   [OVERWRITE - 3 lines total]
                                 |
                                 v
spawner: ctx.working_dir wins  --> agent runs in WORKTREE    [CORRECT]
```

---

## File Changes

| File | Change | Lines |
|------|--------|-------|
| `crates/terraphim_orchestrator/src/lib.rs` | Overwrite `spawn_ctx.working_dir` and `ADF_WORKING_DIR` with `agent_working_dir` after the worktree is allocated, before `spawn_with_fallback` | ~3 |

No other file is modified.

---

## Proposed Change

```rust
// Before (line 2404-2405, pre-fix state at HEAD~1):
let mut spawn_ctx =
    build_spawn_context_for_agent(&self.config, def, self.output_poster.as_ref());
// ADF_WORKING_DIR and spawn_ctx.working_dir both point to project root here.
// spawn_with_fallback follows immediately.

// After (lines 2404-2410, current committed fix):
let mut spawn_ctx =
    build_spawn_context_for_agent(&self.config, def, self.output_poster.as_ref());
spawn_ctx.working_dir = Some(agent_working_dir.clone());          // line 2406
spawn_ctx = spawn_ctx.with_env(                                   // line 2407
    "ADF_WORKING_DIR",                                            // line 2408
    agent_working_dir.to_string_lossy().into_owned(),             // line 2409
);                                                                // line 2410
```

The insertion point is after line 2405 (after `build_spawn_context_for_agent` returns) and before the `if let Some(event) = synthetic_event` block at line 2411. `agent_working_dir` is in scope at this point, having been resolved at line 2329.

---

## Test Plan

| Test | What it verifies |
|------|-----------------|
| `test_spawn_ctx_working_dir_set_to_agent_working_dir` (lib.rs:11956) | Simulates the before-state (project root in spawn_ctx), applies the two-statement fix, then asserts (1) `spawn_ctx.working_dir` equals the worktree path and (2) `ADF_WORKING_DIR` env override equals the worktree path string |

The test is a synchronous unit test with no I/O, no mocks, and no external dependencies. It directly exercises the mutation logic and can be run with:

```
cargo test -p terraphim_orchestrator test_spawn_ctx_working_dir_set_to_agent_working_dir
```

---

## Eliminated Options

| Option | Why rejected |
|--------|-------------|
| Modify `build_spawn_context_for_agent` to accept `agent_working_dir` as a parameter | Would require changing the function signature and all call sites. The function is called in multiple paths; this refactoring is disproportionate to the single-path fix needed for worktree isolation. |
| Move worktree allocation before `build_spawn_context_for_agent` | Worktree allocation is async and depends on model-tier resolution (`needs_isolation`), which itself requires the agent definition and config. Reordering these steps would interleave unrelated logic and widen the diff. |
| Change spawner priority to prefer `config.working_dir` over `ctx.working_dir` | The spawner priority rule (`ctx > config > default`) is the correct design; callers that want to override the directory are expected to set `ctx.working_dir`. Inverting the priority would break every other caller that deliberately sets `ctx.working_dir`. |

---

## Rollback

Revert the addition of lines 2406-2410 in `lib.rs`. The test at line 11956 should be removed in the same revert. No other files are touched.
