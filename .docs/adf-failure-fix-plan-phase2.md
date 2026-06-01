# Implementation Plan: Fix ADF Orchestrator Agent Failures

**Status**: Draft
**Research Doc**: `.docs/adf-failure-analysis-phase1.md`
**Author**: AI Agent (kimi-for-coding)
**Date**: 2026-06-01
**Estimated Effort**: 2-3 days

## Overview

### Summary
Fix the three dominant failure modes causing ADF agents to exit with code 1: (1) orchestrator tick-stall cascade killing child processes, (2) disabled agents still being spawned, and (3) missing agent stderr capture making root-cause analysis impossible.

### Approach
Phase the fixes by impact and dependency:
1. **Immediate (P0)**: Stop spawning disabled agents; move blocking git operations off the main reconcile thread.
2. **This week (P1)**: Capture agent stderr to log files; add tracing to build-runner bash script.
3. **Next sprint (P2)**: Fix provider health probes; increase wall-time limits for long-running agents.

### Scope
**In Scope:**
- `crates/terraphim_orchestrator/src/lib.rs` — tick loop refactoring
- `crates/terraphim_orchestrator/src/config.rs` — respect `enabled = false`
- `crates/terraphim_spawner/src/lib.rs` — stderr capture
- `scripts/build-runner-llm.sh` — error tracing and idempotency
- `crates/terraphim_orchestrator/src/health.rs` — provider probe fixes

**Out of Scope:**
- Rewriting the entire orchestrator (not needed)
- Replacing Claude CLI with a different tool
- Fixing individual agent logic (agents themselves are fine)

**Avoid At All Cost:**
- Adding a new message queue or event bus (overkill)
- Replacing systemd with custom process management
- Rewriting build-runner in Rust (bash script is correct level of abstraction)

## Architecture

### Component Diagram
```
Orchestrator (main thread)
  |
  +-- reconcile_tick() [now: < 2s target]
  |     |
  |     +-- spawn_agents() [non-blocking]
  |     +-- check_agent_health() [non-blocking]
  |     +-- post_status_updates() [non-blocking]
  |
  +-- BlockingOpsWorker (dedicated thread)
  |     |
  |     +-- git_worktree_add()
  |     +-- git_worktree_remove()
  |     +-- compound_review_run()
  |
  +-- AgentProcess
        |
        +-- stdout -> OutputCapture (existing)
        +-- stderr -> AgentLogFile (NEW)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Move git worktree ops to a dedicated blocking thread | Simplest way to prevent main thread stalls; git operations are inherently blocking I/O | Async git wrapper (complex, error-prone); skip worktrees (breaks agent isolation) |
| Capture stderr to per-agent log files | Minimal change to spawner; enables post-mortem debugging of bash-based agents | Stream stderr to journald (couples to systemd); add real-time WebSocket streaming (overkill) |
| Add `trap ERR` to build-runner instead of rewriting | One-line bash change gives exact line numbers for failures | Rewrite in Rust (unnecessary complexity) |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Replace all bash agents with Rust binaries | Massive scope increase; bash is correct for simple task scripts | Maintenance burden, longer build times |
| Add distributed tracing (OpenTelemetry) | Over-engineering for current scale; journalctl is sufficient | Complexity, dependency bloat |
| Rewrite spawner to use async process management | Current sync spawning is fine; async adds complexity without benefit | Harder to debug, more edge cases |

### Simplicity Check

> "Minimum code that solves the problem. Nothing speculative."

**What if this could be easy?**
- Tick stall: Spawn a thread, send commands via channel, check channel in tick loop.
- Disabled agents: Skip agents with `enabled = false` in the spawn loop.
- Stderr capture: Pipe stderr to a file path, rotate on agent exit.
- Build-runner: Add `trap ERR` and `set -x` fallback.

All fixes are under 50 lines each. No new dependencies.

**Senior Engineer Test**: A senior engineer would approve this design — it addresses the root causes without architectural heroics.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/lib.rs` | Move git worktree ops to blocking thread; add tick timing telemetry |
| `crates/terraphim_orchestrator/src/config.rs` | Respect `enabled = false` in agent configs |
| `crates/terraphim_spawner/src/lib.rs` | Pipe child stderr to agent log file |
| `scripts/build-runner-llm.sh` | Add `trap ERR`, idempotent `sed`, `git fetch` verification |
| `crates/terraphim_orchestrator/src/health.rs` | Fix provider probe command for Claude CLI |

### No New Files
All changes are additive modifications to existing files.

## API Design

### Public Types (no changes)
No new public types required. All changes are internal to existing modules.

### Internal Functions

```rust
// In crates/terraphim_orchestrator/src/lib.rs

/// Spawn a blocking git worktree operation on a dedicated thread.
/// Returns a channel receiver for the result.
fn spawn_git_worktree_op(
    working_dir: PathBuf,
    operation: WorktreeOp,
) -> mpsc::Receiver<Result<(), String>>;

/// Check if any pending blocking operations have completed.
/// Call inside reconcile_tick to collect results without blocking.
fn poll_blocking_ops(&mut self);
```

```rust
// In crates/terraphim_spawner/src/lib.rs

/// Open a log file for agent stderr and return the file handle.
/// Log path: /opt/ai-dark-factory/logs/agents/{agent_name}-{timestamp}.stderr
fn open_agent_stderr_log(agent_name: &str) -> std::fs::File;
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_disabled_agent_skipped` | `config.rs` | Verify agents with `enabled = false` are not spawned |
| `test_stderr_captured_to_file` | `spawner/tests/` | Verify child stderr is written to agent log file |
| `test_tick_duration_telemetry` | `lib.rs` | Verify tick timing is logged when > 5s |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_build_runner_with_bad_sha` | `scripts/` | Verify build-runner prints line number on failure |
| `test_provider_probe_claude` | `health.rs` | Verify Claude provider probe uses correct `--allowedTools=` format |

### Manual Verification
1. Restart orchestrator, verify no merge-coordinator spawns.
2. Push a commit with formatting issues, verify build-runner logs exact failure line.
3. Run `cargo test --workspace`, verify test-guardian completes without timeout.

## Implementation Steps

### Step 1: Respect `enabled = false`
**Files:** `crates/terraphim_orchestrator/src/config.rs`
**Description:** Add a check in the agent spawn loop to skip agents where `enabled = false`.
**Tests:** Unit test `test_disabled_agent_skipped`
**Estimated:** 30 minutes

```rust
// In agent spawn loop:
if agent.enabled == Some(false) {
    tracing::debug!(agent = %agent.name, "skipping disabled agent");
    continue;
}
```

### Step 2: Move Blocking Git Operations Off Main Thread
**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:** Spawn git worktree add/remove on a dedicated blocking thread. Use a channel to communicate results back to the main tick loop.
**Tests:** Manual verification + tick timing telemetry
**Dependencies:** Step 1
**Estimated:** 2 hours

```rust
// New module or inline in lib.rs:
use std::sync::mpsc;

enum WorktreeOp {
    Add { path: PathBuf, ref_name: String },
    Remove { path: PathBuf },
}

fn spawn_blocking_op(op: WorktreeOp) -> mpsc::Receiver<Result<(), String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = match op {
            WorktreeOp::Add { path, ref_name } => {
                // existing git worktree add logic
            }
            WorktreeOp::Remove { path } => {
                // existing git worktree remove logic
            }
        };
        let _ = tx.send(result);
    });
    rx
}
```

### Step 3: Capture Agent Stderr to Log Files
**Files:** `crates/terraphim_spawner/src/lib.rs`
**Description:** Open a stderr log file for each spawned agent and redirect child stderr to it.
**Tests:** Unit test for stderr capture
**Dependencies:** Step 2
**Estimated:** 1 hour

```rust
// In spawn_process():
let stderr_log_path = format!("/opt/ai-dark-factory/logs/agents/{}-{}.stderr", agent_name, timestamp);
let stderr_file = std::fs::File::create(&stderr_log_path)?;
cmd.stderr(Stdio::from(stderr_file));
```

### Step 4: Add Error Tracing to Build-Runner
**Files:** `scripts/build-runner-llm.sh`
**Description:** Add `trap ERR` to print the failing line number, make `sed` idempotent, verify `git fetch` retrieves the SHA.
**Tests:** Integration test with bad SHA
**Estimated:** 30 minutes

```bash
set -euo pipefail
trap 'echo "build-runner: FAILED at line $LINENO" >&2' ERR

# Verify git fetch succeeded:
git fetch --depth=1 gitea "$ADF_PUSH_REF" || {
    echo "build-runner: git fetch failed for $ADF_PUSH_REF" >&2
    exit 1
}
```

### Step 5: Fix Provider Probe Command
**Files:** `crates/terraphim_orchestrator/src/health.rs`
**Description:** Update the Claude provider probe to use `--allowedTools=Bash,Read,Write,Edit,Glob,Grep` (with `=`) instead of separate args.
**Tests:** Manual verification
**Estimated:** 15 minutes

```rust
// Update probe command:
let probe = format!("{} -p 'test' --allowedTools=Bash,Read,Write,Edit,Glob,Grep --output-format text", claude_path);
```

### Step 6: Add Tick Timing Telemetry
**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:** Log a warning when reconcile_tick exceeds 5 seconds, including which phase took the longest.
**Tests:** Manual verification
**Estimated:** 30 minutes

```rust
let tick_start = Instant::now();
// ... existing tick logic ...
let tick_elapsed = tick_start.elapsed();
if tick_elapsed > Duration::from_secs(5) {
    tracing::warn!(elapsed_ms = %tick_elapsed.as_millis(), "reconcile_tick exceeded 5s threshold");
}
```

### Step 7: Enable and Verify
**Description:** Re-enable the orchestrator, monitor for 24 hours, verify failure rates drop.
**Estimated:** 15 minutes + monitoring time

## Rollback Plan

If issues discovered:
1. Stop orchestrator: `sudo systemctl stop adf-orchestrator`
2. Revert to previous commit: `git revert HEAD` (all changes are in one PR)
3. Restart orchestrator: `sudo systemctl start adf-orchestrator`

## Dependencies

### No New Dependencies
All changes use existing stdlib and crate APIs.

### Dependency Updates
None required.

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| reconcile_tick duration | < 2s (p99) | Log telemetry |
| Agent spawn latency | < 1s | Existing audit logs |
| Stderr log write overhead | < 5ms per line | Negligible (file I/O) |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Restore product-owner Gitea collaborator access | Human action required | Alex |
| Verify opencode/kimi provider health | Needs investigation | TBD |
| Increase test-guardian wall-time limit | Config change | TBD |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
