# Research Document: DockerExecutor Robustness Fixes

**Status**: Approved
**Author**: opencode (glm-5.1)
**Date**: 2026-05-15
**Reviewers**: Alex

## Executive Summary

The DockerExecutor has three P2 findings that affect production reliability: stale container references when create-but-not-start fails, a Drop implementation that creates N separate tokio runtimes (one per container), and no timeout enforcement on exec operations. LocalExecutor already demonstrates the timeout pattern. All three fixes are localised to `docker.rs` with no cross-file impact.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Container leaks and unbounded exec are real production failure modes |
| Leverages strengths? | Yes | Directly improves the DockerExecutor we just built |
| Meets real need? | Yes | Structural review identified these as the three remaining issues |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

Three robustness gaps in `DockerExecutor` (`crates/terraphim_rlm/src/executor/docker.rs`):

1. **Stale container on start failure**: `create_container()` creates then starts. If `start_container()` fails, the created (but stopped) container leaks. Subsequent calls to `ensure_container()` attempt re-creation, hitting a name collision (`container name "terraphim-rlm-{id}" already in use`).

2. **N tokio runtimes in Drop**: The `Drop` impl spawns one `std::thread` per container, each creating its own `tokio::runtime::Runtime`. With 100 sessions, that is 100 simultaneous runtimes -- excessive resource usage for cleanup.

3. **No timeout enforcement on exec**: `ExecutionContext.timeout_ms` (default 30s) is ignored. A hung exec blocks the caller indefinitely.

### Impact

- Stale containers consume Docker resources (disk, namespace slots) until manually cleaned
- Drop with many sessions creates resource spikes during shutdown
- Hung exec blocks the RLM query loop, making the system unresponsive

### Success Criteria

- Create-but-start-fail cleans up the created container
- Drop uses a single runtime or fire-and-forget approach that does not scale with session count
- Exec operations respect `ExecutionContext.timeout_ms` and return `ExecutionResult::timeout()` on expiry

## Current State Analysis

### Existing Implementation

| Component | Location | Purpose |
|-----------|----------|---------|
| `DockerExecutor` | `docker.rs:36-42` | Struct with `Docker` client, session map, image |
| `create_container` | `docker.rs:90-121` | Creates + starts container |
| `ensure_container` | `docker.rs:77-88` | Get-or-create with session affinity |
| `exec_in_container` | `docker.rs:123-212` | Create exec, stream output, inspect exit code |
| `cleanup` | `docker.rs:296-317` | Parallel remove via `join_all` |
| `Drop` | `docker.rs:320-343` | Spawns N threads with N runtimes |
| `LocalExecutor::run_command` | `local.rs:86-121` | Reference: uses `tokio::time::timeout` |

### Data Flow

```
execute_code(code, ctx)
  -> ensure_container(session_id)
       -> create_container(session_id)
            -> docker.create_container()   [FAIL: container does not exist]
            -> docker.start_container()    [FAIL: container exists but stopped]
  -> exec_in_container(container_id, cmd)
       -> docker.create_exec()
       -> docker.start_exec()             [NO TIMEOUT: can hang forever]
       -> docker.inspect_exec()           [exit code]
```

### Integration Points

- `ExecutionEnvironment` trait (`trait.rs`) -- no changes needed
- `ExecutionContext.timeout_ms` (`context.rs:50`) -- the field already exists, just not consumed
- `ExecutionResult::timeout()` (`context.rs:193`) -- factory method exists, not used by DockerExecutor
- `select_executor` (`mod.rs:80-143`) -- no changes needed

## Constraints

### Technical Constraints

- **bollard 0.20.2 API**: `start_container()`, `remove_container()`, `stop_container()` all available
- **Docker is Clone**: Can be cloned cheaply (Arc underneath)
- **No async Drop**: Rust does not support async Drop; must use sync workarounds
- **parking_lot::RwLock**: Not reentrant; avoid holding write lock across `.await` points

### Pattern Constraints

- **LocalExecutor precedent**: Uses `tokio::time::timeout(Duration::from_millis(30000), cmd.output())` at `local.rs:89`
- **FirecrackerExecutor precedent**: Uses `tokio::time::timeout(Duration::from_secs(30), ...)` at `firecracker.rs:401`
- **FirecrackerExecutor cleanup**: Simply clears maps, no Drop impl at all

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Must not leak containers | Container resource exhaustion crashes Docker | Stale container on start failure |
| Exec must timeout | Hung exec blocks RLM loop | `timeout_ms` field exists but unused |
| Drop must not scale with sessions | 100 sessions = 100 runtimes is unacceptable | Current Drop impl |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Container pooling / reuse | Over-engineering for v1 |
| Image pre-pull / pull-on-create | Separate concern, tracked elsewhere |
| Snapshot support for Docker | Explicitly deferred to v2 |
| `ListContainers` for orphan cleanup | Separate concern (operational hygiene) |
| Container health monitoring | YAGNI for v1 |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `ExecutionEnvironment` trait | No changes needed | None |
| `ExecutionContext.timeout_ms` | Already exists | None |
| `ExecutionResult::timeout()` | Already exists | None |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| bollard | 0.20.2 | None -- all APIs confirmed | N/A |
| tokio | (workspace) | None | N/A |
| parking_lot | (workspace) | None | N/A |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `remove_container` in start-fail path also fails | Low | Low | Log warning, container is stopped (low resource impact) |
| Timeout fires before Docker exec completes | Low | Medium | Use `ctx.timeout_ms` (30s default), documented behaviour |
| Drop fire-and-forget misses container | Low | Low | Best-effort cleanup; `cleanup()` is the primary path |

### Open Questions

None -- all APIs confirmed via bollard source inspection.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `Docker` is cheaply Clone-able | bollard source: Arc-based transport | Minor perf hit | Yes |
| `remove_container(force)` works on stopped containers | Docker API semantics | Container leaked | Yes |
| `tokio::time::timeout` works with bollard streams | Streams are `Future`-compatible | Timeout does not fire | Yes |

## Research Findings

### Key Insights

1. **LocalExecutor already has the timeout pattern** at `local.rs:89` -- copy this approach exactly
2. **FirecrackerExecutor has no Drop impl** -- it relies on `cleanup()` being called explicitly. DockerExecutor should follow this pattern and simplify Drop.
3. **`Docker` is Clone** (Arc-based transport) -- can share a single Docker client across cleanup threads
4. **bollard provides `stop_container`** for graceful stop before `remove_container`, but `remove_container(force=true)` handles both in one call

### Relevant Prior Art

- **LocalExecutor `run_command`** (`local.rs:86-121`): Canonical timeout pattern in this codebase
- **FirecrackerExecutor timeout** (`firecracker.rs:401`): Uses `tokio::time::timeout` for SSH wait
- **Rust async Drop patterns**: The standard approach is either (a) fire-and-forget with `tokio::spawn`, or (b) best-effort sync. Option (a) is cleaner.

### Technical Spikes

| Spike | Purpose | Result |
|-------|---------|--------|
| bollard Drop patterns | How to clean up in sync context | `Docker` is Clone, use `tokio::spawn` fire-and-forget |
| timeout with bollard streams | Does `tokio::time::timeout` work? | Yes, bollard streams implement `Future` via `StreamExt::next()` |

## Recommendations

### Proceed/No-Proceed

**Proceed**. All three fixes are well-understood, localised to one file, and follow existing codebase patterns.

### Scope Recommendations

Fix all three in a single commit since they share the same file and are interrelated (Drop cleanup and stale container handling overlap).

### Risk Mitigation Recommendations

- Write a test for the stale-container scenario (mock Docker or use a container name that will fail to start)
- Write a test for timeout (exec a `sleep 60` with a short timeout)

## Next Steps

If approved:
1. Create implementation plan (Phase 2)
2. Implement all three fixes
3. Run full test suite
4. Push and update PR #1485
