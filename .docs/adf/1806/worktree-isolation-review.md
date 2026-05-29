# ADF Review: Worktree-Per-Agent Isolation

Issue: 1806
Flow: adf-worktree-isolation-review
Generated: 2026-05-29 20:20 BST
Status: Completed

## Executive Summary

ADF reviewed the current `terraphim_spawner` and orchestrator worktree-related code against issue #1806's checklist. The system already has meaningful worktree isolation plumbing in the orchestrator: `create_agent_worktree()` runs `git worktree add --detach`, `WorktreeGuard::for_managed()` removes managed worktrees, and `ManagedAgent` retains the guard for the active agent lifetime.

The current state is therefore **partially implemented but not yet issue-complete**. The strongest review finding is that the worktree path is written into the provider's `working_dir`, but the actual `AgentSpawner` process directory is selected from `SpawnContext` first. `spawn_agent_with_event()` builds `spawn_ctx` from project config after computing `agent_working_dir`, so project-bound agents may still execute in the project root rather than the isolated worktree unless the provider conversion path overrides config working_dir before `spawn_process()`.

## Checklist Evaluation

| Requirement | Current Evidence | Verdict |
|-------------|------------------|---------|
| `git worktree add` before each pi-rust agent spawn | `create_agent_worktree()` runs `git worktree add --detach <path> HEAD` before provider construction | Mostly met |
| Unique path per agent under `~/.cache/terraphim/agents/wt-<agent-id>-<timestamp>/` | Current path is `<repo>/.worktrees/<agent-name>-<uuid8>`, not the issue's cache-path convention | Partially met |
| Cleanup after completion or graceful shutdown | `WorktreeGuard::for_managed()` supports `git worktree remove --force`; `ManagedAgent` stores the guard | Mostly met |
| tmux session integration for attach/reprompt | No `tmux` references found in spawner or orchestrator source | Missing |
| Fallback if worktree creation fails | `create_agent_worktree()` fail-opens to shared working_dir | Met, but should become policy-controlled |
| Tests for concurrent isolation and cleanup on error | `worktree_guard` has cleanup tests; no proof found that spawned child processes actually run inside the worktree | Partially met |

## Highest-Risk Finding

The code appears to compute an isolated worktree path, then builds `spawn_ctx` independently from project config:

- `agent_working_dir = worktree_path.as_deref().unwrap_or(repo_dir).to_path_buf()`
- Provider `working_dir` receives `agent_working_dir`
- `spawn_ctx = build_spawn_context_for_agent(...)`
- `spawn_process()` chooses `ctx.working_dir` before `config.working_dir`

If `spawn_ctx.working_dir` is set to the project root, it can shadow the provider `working_dir` and defeat the isolation. This is exactly the kind of cross-boundary contract bug issue #1806 is intended to flush out.

## Recommended Design

1. After `agent_working_dir` is computed, layer it into `spawn_ctx` with `SpawnContext::with_working_dir(agent_working_dir.clone())` semantics so the process current directory cannot diverge from the provider working directory.
2. Add a regression test that spawns a simple local command and asserts `pwd` equals the created worktree path.
3. Convert the fail-open behaviour to explicit policy: `Abort`, `SharedWorkingDir`, or `ReadOnlySharedWorkingDir`.
4. Decide whether the issue's `~/.cache/terraphim/agents/...` path convention should replace the current repo-local `.worktrees/...` convention.
5. Add optional `tmux new-session -d -s <session-id>` wrapping at the same orchestration boundary, not inside low-level output capture.

## Acceptance Tests To Add

- A spawned implementation-tier agent reports `pwd` equal to the created worktree path.
- Two concurrently spawned agents write to different files and never touch each other's working directories.
- Failed worktree creation returns a structured degraded-mode result or hard error according to config.
- Dropping the guard after a simulated failed agent removes both filesystem path and git worktree registry entry.
- Successful run can preserve the worktree when configured for inspection.
- tmux session id is unique and stable enough for attach/reprompt.

## Implementation Slice

The smallest useful PR should avoid implementing full pi-rust orchestration. It should add only:

- spawn-context working directory override after worktree creation
- a current-directory regression test proving the child actually runs in the worktree
- policy-controlled fallback for worktree creation failure
- one concurrent isolation test

This would turn the existing partial implementation into executable isolation behaviour without overreaching into model routing or agent lifecycle policy.

## Evidence

See `.docs/adf/1806/worktree-evidence.md` for the source inventory captured by this flow.
