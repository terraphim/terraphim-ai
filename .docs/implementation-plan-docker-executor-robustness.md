# Implementation Plan: DockerExecutor Robustness Fixes

**Status**: Draft
**Research Doc**: `.docs/research-docker-executor-robustness.md`
**Author**: opencode (glm-5.1)
**Date**: 2026-05-15
**Estimated Effort**: 1-2 hours

## Overview

### Summary

Fix three robustness issues in `DockerExecutor`: stale container cleanup on start failure, Drop runtime explosion, and missing exec timeout enforcement.

### Approach

Follow existing codebase patterns: `LocalExecutor` timeout pattern, `FirecrackerExecutor` cleanup-without-Drop pattern. All changes localised to `docker.rs`.

### Scope

**In Scope:**
- Rollback container creation on `start_container` failure
- Replace N-runtime Drop with single `tokio::spawn` fire-and-forget
- Wrap exec stream consumption with `tokio::time::timeout`
- Unit tests for all three fixes

**Out of Scope:**
- Container pooling
- Image pre-pull
- Snapshot support
- ListContainers orphan cleanup

**Avoid At All Cost:**
- Adding new dependencies
- Changing the `ExecutionEnvironment` trait
- Adding background reaper threads

## Architecture

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| `tokio::spawn` in Drop instead of N runtimes | Reuses existing tokio runtime, single allocation | Creating one Runtime per container (current), `spawn_blocking` with nested runtime |
| `tokio::time::timeout` wrapping entire exec | Matches LocalExecutor pattern, simple | Per-stream-item timeout (over-complex), Docker `ExecCreate` timeout (not supported) |
| Remove-on-start-fail in `create_container` | Keeps cleanup close to failure point | Separate cleanup method, `ListContainers` filter approach |

### Simplicity Check

**What if this could be easy?** Each fix is a small, localised change:

1. Add `remove_container` call in error path of `start_container`
2. Replace `std::thread::spawn + Runtime::new` with `tokio::spawn`
3. Wrap the output stream with `tokio::time::timeout`

No new types, no new modules, no trait changes. Minimum code that solves the problem.

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
| `crates/terraphim_rlm/src/executor/docker.rs` | Fix 1: cleanup on start fail. Fix 2: rewrite Drop. Fix 3: add timeout to exec_in_container. Add 3 new tests. |

No new files. No deleted files. No changes to other files.

## API Design

### No New Public Types

All fixes are internal to `DockerExecutor`. No public API changes.

### Modified Private Methods

```rust
impl DockerExecutor {
    /// Create and start a container. Rolls back creation if start fails.
    async fn create_container(&self, session_id: &SessionId) -> RlmResult<String>;

    /// Execute a command in a container with timeout enforcement.
    /// Respects `ctx.timeout_ms` (default 30s).
    async fn exec_in_container(
        &self,
        container_id: &str,
        cmd: Vec<&str>,
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult>;
}

impl Drop for DockerExecutor {
    /// Fire-and-forget cleanup using `tokio::spawn`.
    /// Does not create new runtimes.
    fn drop(&mut self);
}
```

Note: `exec_in_container` signature changes to accept `&ExecutionContext` for timeout value.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_create_container_cleanup_on_start_fail` | `docker.rs::tests` | Verify container removed when start fails (requires Docker) |
| `test_exec_timeout` | `docker.rs::tests` | Verify timeout fires on long-running exec (requires Docker) |
| `test_drop_does_not_create_runtimes` | `docker.rs::tests` | Verify Drop completes without spawning std::threads |

### Integration Tests (manual verification)

| Test | Purpose |
|------|---------|
| Run `sleep 60` with 2s timeout | Confirm `timed_out: true` in result |
| Create many sessions, drop executor | Confirm no runtime explosion in process metrics |

## Implementation Steps

### Step 1: Fix stale container on start failure

**File:** `docker.rs`, method `create_container`
**Description:** If `start_container()` fails, remove the created container before returning the error.
**Tests:** Manual verification with Docker (cannot mock bollard without mocks)
**Estimated:** 15 minutes

```rust
// In create_container, after start_container fails:
if let Err(e) = self.docker.start_container(&create_response.id, None).await {
    let container_id = &create_response.id;
    let remove_opts = RemoveContainerOptionsBuilder::new().force(true).build();
    if let Err(remove_err) = self.docker.remove_container(container_id, Some(remove_opts)).await {
        log::warn!(
            "Failed to remove container {} after start failure: {}",
            container_id, remove_err
        );
    }
    return Err(RlmError::BackendInitFailed {
        backend: "docker".to_string(),
        message: format!("Failed to start container: {}", e),
    });
}
```

### Step 2: Fix Drop to use tokio::spawn

**File:** `docker.rs`, `impl Drop for DockerExecutor`
**Description:** Replace `std::thread::spawn + Runtime::new` with `tokio::spawn` fire-and-forget. `tokio::spawn` returns a `JoinHandle` that we drop (detaches the task). The task will run on the existing tokio runtime.
**Tests:** Verify no `std::thread::spawn` calls during Drop
**Dependencies:** Step 1
**Estimated:** 15 minutes

```rust
impl Drop for DockerExecutor {
    fn drop(&mut self) {
        let containers: Vec<String> = self
            .session_to_container
            .write()
            .drain()
            .map(|(_, id)| id)
            .collect();

        if containers.is_empty() {
            return;
        }

        let docker = self.docker.clone();
        tokio::spawn(async move {
            let remove_opts = RemoveContainerOptionsBuilder::new().force(true).build();
            let futures: Vec<_> = containers
                .iter()
                .map(|id| docker.remove_container(id, Some(remove_opts.clone())))
                .collect();
            let results = futures::future::join_all(futures).await;
            for (i, result) in results.into_iter().enumerate() {
                if let Err(e) = result {
                    log::warn!("Drop: failed to remove container {}: {}", containers[i], e);
                }
            }
        });
    }
}
```

**Critical**: `tokio::spawn` requires a running tokio runtime. Since `DockerExecutor` is only used within the terraphim_rlm async context (created by `select_executor` which is async), a runtime will always exist when Drop is called. If Drop is called outside a runtime (edge case), `tokio::spawn` will panic. Add a guard:

```rust
let handle = tokio::runtime::Handle::try_current();
match handle {
    Ok(_) => { /* tokio::spawn as above */ }
    Err(_) => {
        log::warn!("DockerExecutor::drop called outside tokio runtime; containers not cleaned up");
    }
}
```

### Step 3: Add timeout to exec_in_container

**File:** `docker.rs`, method `exec_in_container`
**Description:** Wrap the `start_exec` output stream consumption with `tokio::time::timeout`. Use `ctx.timeout_ms` from `ExecutionContext`. Change method signature to accept `&ExecutionContext`.

This requires changing:
- `exec_in_container` signature to accept `&ExecutionContext`
- `execute_code` and `execute_command` to pass `ctx` through

**Tests:** Test with `sleep 60` and short timeout
**Dependencies:** Step 2
**Estimated:** 30 minutes

```rust
async fn exec_in_container(
    &self,
    container_id: &str,
    cmd: Vec<&str>,
    ctx: &ExecutionContext,
) -> RlmResult<ExecutionResult> {
    // ... create_exec, start_exec as before ...

    let timeout_duration = Duration::from_millis(ctx.timeout_ms);

    let exec_future = async {
        // ... stream consumption ...
    };

    match tokio::time::timeout(timeout_duration, exec_future).await {
        Ok(result) => result,
        Err(_) => {
            // Timeout elapsed
            Ok(ExecutionResult::timeout(stdout_so_far, stderr_so_far))
        }
    }
}
```

The challenge: we need to capture partial stdout/stderr before timeout. This requires restructuring the stream consumption so partial data is available in the timeout handler. Approach: collect stdout/stderr in outer variables, use a closure.

### Step 4: Update callers and add tests

**File:** `docker.rs`
**Description:** Update `execute_code` and `execute_command` to pass `ctx` to `exec_in_container`. Add test cases.
**Dependencies:** Step 3
**Estimated:** 15 minutes

## Rollback Plan

Each step is independent and can be reverted individually. All changes are in one file.

## Performance Considerations

| Metric | Before | After | Measurement |
|--------|--------|-------|-------------|
| Drop runtime count | N (one per session) | 0 (reuses existing) | Process thread count |
| Exec max duration | Unlimited | `ctx.timeout_ms` (default 30s) | Wall clock |
| Cleanup on start fail | None (container leaked) | Immediate | Docker container list |

No new benchmarks needed -- these are correctness/robustness fixes, not performance changes.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify `tokio::spawn` in Drop works with test runtime | Resolved: add `Handle::try_current()` guard | Implementation |
| Verify `RemoveContainerOptions` is Clone | Resolved: confirmed `#[derive(Debug, Clone, PartialEq, Serialize)]` in bollard-stubs | Implementation |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
