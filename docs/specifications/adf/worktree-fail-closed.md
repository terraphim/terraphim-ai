# Specification: Worktree Fail-Closed Behaviour

**Status**: Authoritative
**Source**: `crates/terraphim_orchestrator/src/worktree_guard.rs`
**Issue**: #1924 (re-scoped from PR #1788 Slice 8)
**Date**: 2026-06-01

---

## Overview

Agent worktrees are isolated git working tree checkouts used to prevent concurrent
agents from interfering with each other. `WorktreeGuard` is an RAII struct that
unconditionally removes a worktree on `Drop` unless the caller explicitly calls
`keep()`. This is the "fail-closed" policy: if the agent crashes, panics, or exits
abnormally, the worktree is cleaned up automatically.

---

## Behaviour

### Default State: Cleanup Armed

`WorktreeGuard::new(path)` creates a guard with `should_cleanup = true`.
When the guard is dropped, `cleanup()` runs.

### Disarming: `keep()`

`guard.keep()` sets `should_cleanup = false` and logs a DEBUG trace that the guard is
disarmed. The guard then drops without cleaning up. Callers use this when the agent
succeeds and the worktree should be preserved for inspection or merge.

### Two Constructors

| Constructor | Purpose |
|-------------|---------|
| `WorktreeGuard::new(path)` | Filesystem-only cleanup. Removes the directory with `fs::remove_dir_all`. |
| `WorktreeGuard::for_managed(repo_path, worktree_path)` | Git-aware cleanup. Runs `git -C <repo_path> worktree remove --force <worktree_path>` first to reconcile the git admin registry at `<repo>/.git/worktrees/<name>`, then falls back to filesystem removal on non-zero exit or if git CLI is not invokable. |

### Cleanup Algorithm

1. If `should_cleanup` is `false`, return immediately.
2. If `path` does not exist, log DEBUG and return.
3. If `repo_path` is `Some`:
   a. Run `git -C <repo_path> worktree remove --force <path>` (synchronous, via `std::process::Command`).
   b. If successful, return.
   c. On non-zero exit or IO error, log WARN and fall through to step 4.
4. Run `fs::remove_dir_all(path)`.
   - On success, log INFO.
   - On error, log WARN and attempt `fs::remove_dir(path)` as a final fallback.

The synchronous `std::process::Command` is intentional — `Drop` cannot be async, and
`git worktree remove` completes in sub-second time.

---

## Invariants

| # | Invariant | Source |
|---|-----------|--------|
| I1 | A guard created with `new` or `for_managed` always attempts cleanup on drop unless `keep()` was called. | `Drop::drop` calls `cleanup()` |
| I2 | Cleanup is idempotent: if the path no longer exists, cleanup returns without error. | `if !self.path.exists() { return; }` |
| I3 | `keep()` consumes `self`; it cannot be called after the guard has moved. | Ownership model |
| I4 | `for_managed` cleanup reconciles the git admin registry as well as the working directory. | `git worktree remove --force` before `remove_dir_all` |
| I5 | Git CLI unavailability does not prevent cleanup; fs removal is always attempted as fallback. | `Err(e) => { warn!(...); }` → fallthrough |

---

## Failure Modes

| Failure | Observable Effect | Recovery |
|---------|-------------------|---------|
| `git` not on PATH | WARN logged; fs fallback runs | Install git, or use `new()` instead of `for_managed()` |
| fs remove fails (permissions) | WARN logged; directory may remain | Fix permissions; run `git worktree prune` manually |
| `keep()` not called on agent success | Worktree is removed | Ensure success path calls `keep()` |
| Guard dropped before agent process exits | Worktree removed while process still running | Ensure guard scope encompasses the entire agent process lifetime |
| Stale git worktree admin entry after fs removal | `git worktree list` shows dangling entry | Run `git worktree prune` |

---

## Scoped Helpers

Two convenience wrappers are provided for callers that want to run a closure with
automatic cleanup:

```rust
// Synchronous
with_worktree_guard(path, |guard| { ... });

// Async
with_worktree_guard_async(path, async { ... }).await;
```

Both use `WorktreeGuard::new` (filesystem-only cleanup).

---

## Verification Note

All invariants are covered by unit tests verified on `gitea/main` as of 2026-06-01:

```bash
cargo test -p terraphim_orchestrator worktree_guard -- --nocapture
```

Tests verified:
- `test_worktree_guard_cleanup` — guard removes directory on drop
- `test_worktree_guard_keep` — `keep()` prevents removal
- `test_worktree_guard_already_removed` — no panic when directory pre-removed
- `test_with_worktree_guard` — scoped closure cleans up
- `test_managed_guard_invokes_git_remove` — git admin registry reconciled
- `test_managed_guard_fallback_on_git_failure` — fs fallback when git fails
- `test_managed_guard_keep_disarms` — `keep()` prevents managed cleanup
