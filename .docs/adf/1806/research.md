# Phase 1 Research: Spawn-Context Isolation Bug

**Skill**: disciplined-research
**Issue**: 1806
**Date**: 2026-05-29

---

## Bug Confirmation

**YES - the bug was real.** It is present in the committed code at `HEAD~1` and the fix is in the current uncommitted diff.

### Evidence

**Root-cause line (pre-fix):**
`crates/terraphim_orchestrator/src/lib.rs:2404-2405` — `build_spawn_context_for_agent` constructs `spawn_ctx` with `working_dir = project.working_dir` (the project root), as seen at `lib.rs:643`:

```rust
let mut ctx = SpawnContext::with_working_dir(project.working_dir.clone())
    .with_env("ADF_PROJECT_ID", pid)
    .with_env("ADF_WORKING_DIR", working_dir_str); // project root, not worktree
```

**Priority rule (spawner):**
`crates/terraphim_spawner/src/lib.rs:657-662` comments "Priority: ctx override > config working_dir > spawner default":

```rust
let working_dir = ctx
    .working_dir
    .as_ref()
    .or(config.working_dir.as_ref())
    .unwrap_or(&self.default_working_dir);
```

**Worktree path correctly placed on the Provider (not overriding ctx):**
`lib.rs:2329` computes `agent_working_dir` = worktree path if available, else `repo_dir`.
`lib.rs:2338` places `agent_working_dir` into `ProviderType::Agent { working_dir }`.
`crates/terraphim_spawner/src/config.rs:59` maps that into `AgentConfig::working_dir`.

So pre-fix, the conflict was:
- `spawn_ctx.working_dir` = project root (from `build_spawn_context_for_agent`)
- `config.working_dir` = worktree path (from `Provider.working_dir` → `AgentConfig`)
- Spawner picks `ctx.working_dir` first → **agent ran in project root**

**The fix (current uncommitted diff, `lib.rs:2406-2410`):**

```rust
spawn_ctx.working_dir = Some(agent_working_dir.clone());
spawn_ctx = spawn_ctx.with_env(
    "ADF_WORKING_DIR",
    agent_working_dir.to_string_lossy().into_owned(),
);
```

This overwrites the project-root value in `spawn_ctx` with `agent_working_dir` (the worktree), so `ctx.working_dir` now also points to the worktree.

---

## Root Cause

Two-layer inconsistency introduced by the interaction between `build_spawn_context_for_agent` and the worktree allocation logic:

1. **`build_spawn_context_for_agent` (lib.rs:643)** sets `spawn_ctx.working_dir = project.working_dir` — the project root — because at that point the worktree path has not yet been computed and injected.

2. **`spawn_process` (spawner/src/lib.rs:658)** gives `ctx.working_dir` the highest priority, so the project-root value silently wins over the correctly set `Provider.working_dir` (worktree).

3. **`ADF_WORKING_DIR` env var (lib.rs:645)** was also set to the project root string by `build_spawn_context_for_agent`, so the agent's shell scripts that rely on `$ADF_WORKING_DIR` also received the wrong directory.

The worktree path IS computed correctly at `lib.rs:2319-2329`; the bug is that it was put on the `Provider` but not subsequently patched back onto `spawn_ctx`.

---

## Impact

**What actually happens (pre-fix):**

- `create_agent_worktree` creates an isolated git worktree at e.g. `/tmp/project/.worktrees/agent-abc123`.
- The worktree path is stored in `agent_working_dir` and placed on `Provider.working_dir`.
- `build_spawn_context_for_agent` creates `spawn_ctx` with `working_dir = /tmp/project` (root) and `ADF_WORKING_DIR=/tmp/project`.
- `spawn_process` selects `ctx.working_dir` = `/tmp/project` as the `current_dir` for the child process.
- The spawned agent process starts in the project root, not in the isolated worktree.
- The agent may read/write files in the live project, defeating the isolation guarantee.
- `$ADF_WORKING_DIR` inside the agent shell also points to the project root.

**What should happen (post-fix):**

- `spawn_ctx.working_dir` = worktree path.
- `ADF_WORKING_DIR` env var = worktree path.
- Agent process starts in the isolated worktree.
- Changes are contained; the worktree guard handles cleanup after the agent exits.

---

## Existing Safeguards

1. **WorktreeGuard (lib.rs:2321)**: Created alongside the worktree. Ensures cleanup on drop. It does NOT prevent the process from running in the wrong directory — it only cleans up the worktree after the agent finishes. So isolation failure is silent: the guard fires correctly but the agent already ran in the wrong place.

2. **Haiku agents skip isolation (lib.rs:2298)**: Review-tier (haiku) agents set `needs_isolation = false`, so they never enter the worktree path at all. This is intentional and correct — it means the bug only affected implementation-tier (non-haiku) agents.

3. **Provider `working_dir` (lib.rs:2338)**: The worktree path was correctly stored on the `Provider`, so it would have been used if `spawn_ctx.working_dir` had been `None`. The bug is specifically that `spawn_ctx.working_dir` was non-`None` (set to project root) and thus took precedence.

---

## Recommendation

Replace the stale `spawn_ctx.working_dir` produced by `build_spawn_context_for_agent` with `agent_working_dir` immediately after the call, and update `ADF_WORKING_DIR` accordingly — exactly as the uncommitted diff at `lib.rs:2406-2410` does.
