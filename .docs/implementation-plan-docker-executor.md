# Implementation Plan: DockerExecutor

**Status**: Draft
**Research Doc**: `.docs/research-docker-executor.md`
**Author**: Alex
**Date**: 2026-05-14
**Estimated Effort**: 8-12 hours

## Overview

### Summary
Implement `DockerExecutor` as a container-based isolation backend for terraphim_rlm. Docker provides namespace isolation (PID, NET, IPC, Mount) without requiring KVM, enabling sandboxed code execution on macOS.

### Approach
Follow the FirecrackerExecutor pattern for session-to-resource tracking, LocalExecutor pattern for simple execution, and use bollard for Docker daemon communication.

### Scope

**In Scope:**
- `DockerExecutor` struct implementing `ExecutionEnvironment` trait
- Container lifecycle (create, exec, delete)
- Session-to-container affinity mapping
- Python and bash execution via `docker exec`
- Container cleanup on session destroy
- `Capability::ContainerIsolation` support
- Update `select_executor()` to use DockerExecutor

**Out of Scope:**
- Container snapshot/commit support
- Container pooling (pre-warmed containers)
- Network isolation / DNS allowlist
- gVisor runtime selection

**Avoid At All Cost:**
- Pre-mature optimization for container startup time
- Complex container networking setup

## Architecture

### Component Diagram

```
select_executor()
     │
     ├── KVM available? → FirecrackerExecutor
     │
     ├── E2B configured? → E2bExecutor (not implemented)
     │
     ├── Docker available? → DockerExecutor ← NEW
     │                              │
     │                              ├── bollard (HTTP to Docker daemon)
     │                              │
     │                              └── Container per session
     │
     └── Fallback → LocalExecutor
```

### Data Flow

```
TerraphimRlm.execute_code(session_id, code)
    → ExecutionContext { session_id, timeout_ms, env_vars }
    → DockerExecutor.execute_code(code, ctx)
    → Ensure container exists for session
    → docker exec <container> python -c <code>
    → ExecutionResult { stdout, stderr, exit_code, execution_time_ms }
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| One container per session | Simplifies isolation tracking, matches Firecracker model | Container reuse across sessions (too complex for v1) |
| Use `python:3.11-slim` image | Minimal size, reliable, pre-installed Python | User-provided image (requires config), ubuntu (too large) |
| Use `docker exec` not API exec | Simpler output streaming, matches bash -c pattern | Full API exec with streaming (over-complicated) |
| Delete container on session destroy | Ensures resources freed, matches Firecracker cleanup | Reuse containers (state pollution risk) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Container snapshot via commit | Not required for v1, adds complexity | Could confuse users expecting Firecracker-level snapshots |
| Pre-warmed container pool | Over-engineering for initial implementation | Startup latency acceptable for dev use case |
| gVisor runtime with runsc | Not available on mac Docker | Complex setup with no benefit on mac |

### Simplicity Check

> "What if this could be easy?"

A simple approach: `docker run --rm python:3.11-slim python -c "code"` for each execution. No persistent containers, no session affinity, no cleanup complexity.

**Why rejected**: High overhead (image pull every time), no session state persistence, slower than container reuse.

**Chosen design**: Session affinity with persistent container, exec for code execution. Balances simplicity with functionality.

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_rlm/src/executor/docker.rs` | DockerExecutor implementation |

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_rlm/src/executor/mod.rs` | Add `mod docker;`, wire up DockerExecutor in select_executor, add `is_docker_available()` to health checks |
| `crates/terraphim_rlm/src/executor/context.rs` | Already has `Capability::ContainerIsolation` |
| `crates/terraphim_rlm/Cargo.toml` | Already has `bollard` as optional dep |

### Files Not Changed
| File | Reason |
|------|--------|
| `crates/terraphim_rlm/src/lib.rs` | Already exports all executor types via `pub use executor::*` |
| `crates/terraphim_rlm/src/config.rs` | BackendType::Docker already exists |

## API Design

### DockerExecutor Struct

```rust
/// Docker-based execution backend using container isolation.
///
/// Uses Docker containers to provide namespace isolation (PID, NET, IPC, Mount)
/// without requiring KVM. Each session gets its own container.
pub struct DockerExecutor {
    /// Configuration for the executor.
    config: RlmConfig,

    /// Docker client for daemon communication.
    docker: bollard::Docker,

    /// Session to container ID mapping.
    session_to_container: parking_lot::RwLock<HashMap<SessionId, String>>,

    /// Container configurations per session (for reuse).
    container_configs: parking_lot::RwLock<HashMap<SessionId, ContainerConfig>>,

    /// Default image to use for containers.
    image: String,

    /// Capabilities supported by this executor.
    capabilities: Vec<Capability>,
}
```

### Public Constructors

```rust
impl DockerExecutor {
    /// Create a new Docker executor.
    ///
    /// # Errors
    /// Returns error if Docker daemon is not accessible.
    pub fn new(config: RlmConfig) -> Result<Self, RlmError> {
        // Connect to Docker daemon via bollard
        // Initialize docker client
        // Set up session_to_container HashMap
    }

    /// Create with custom Docker endpoint (for testing).
    pub fn with_docker_endpoint(config: RlmConfig, endpoint: &str) -> Result<Self, RlmError>;

    /// Create with custom image.
    pub fn with_image(config: RlmConfig, image: &str) -> Result<Self, RlmError>;
}
```

### ExecutionEnvironment Trait Implementation

```rust
#[async_trait]
impl ExecutionEnvironment for DockerExecutor {
    type Error = RlmError;

    async fn execute_code(&self, code: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error>;
    async fn execute_command(&self, cmd: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error>;
    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error>;
    async fn create_snapshot(&self, session_id: &SessionId, name: &str) -> Result<SnapshotId, Self::Error>;
    async fn restore_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error>;
    async fn list_snapshots(&self, session_id: &SessionId) -> Result<Vec<SnapshotId>, Self::Error>;
    async fn delete_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error>;
    async fn delete_session_snapshots(&self, session_id: &SessionId) -> Result<(), Self::Error>;
    fn capabilities(&self) -> &[Capability];
    fn backend_type(&self) -> BackendType;
    async fn health_check(&self) -> Result<bool, Self::Error>;
    async fn cleanup(&self) -> Result<(), Self::Error>;
}
```

### Error Types (reuse existing)

```rust
// Using existing RlmError variants:
RlmError::ExecutionFailed { message, exit_code, stdout, stderr }
RlmError::BackendInitFailed { backend, message }
RlmError::NoBackendAvailable { tried }
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_docker_executor_creation` | docker.rs | Verify executor can be created when Docker available |
| `test_docker_executor_no_docker` | docker.rs | Verify error when Docker unavailable |
| `test_execute_python_code` | docker.rs | Execute `print('hello')` and verify output |
| `test_execute_bash_command` | docker.rs | Execute `echo $HOME` and verify output |
| `test_execution_failure` | docker.rs | Execute `exit 1` and verify non-zero exit code |
| `test_execution_timeout` | docker.rs | Execute infinite loop, verify timeout |
| `test_container_cleanup` | docker.rs | Verify container deleted on session cleanup |
| `test_health_check` | docker.rs | Verify Docker daemon ping works |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_docker_executor_integration` | `tests/docker_validation.rs` | Full session lifecycle with Docker |
| `test_docker_executor_concurrent` | `tests/docker_validation.rs` | Multiple sessions, concurrent execution |

### Test Fixtures
- Docker daemon must be running (checked via `is_docker_available()`)
- Skip tests if Docker unavailable (same pattern as KVM tests)

### Mock Strategy
- Use a mock Docker daemon endpoint for unit tests
- Real integration tests against local Docker

## Implementation Steps

### Step 1: Cargo.toml verification
**Files:** `crates/terraphim_rlm/Cargo.toml`
**Description:** Verify bollard feature flag and dependency
**Tests:** None (just verification)
**Estimated:** 15 minutes

```toml
# Verify these lines exist:
[dependencies]
bollard = { version = "0.20", optional = true }

[features]
default = ["full"]
full = [..., "docker-backend", ...]
docker-backend = ["dep:bollard"]
```

### Step 2: Create docker.rs module skeleton
**Files:** `crates/terraphim_rlm/src/executor/docker.rs`
**Description:** Create empty module with struct and basic imports
**Tests:** Compile check
**Estimated:** 30 minutes

```rust
//! Docker execution backend using container isolation.

use async_trait::async_trait;
use std::collections::HashMap;

pub struct DockerExecutor { ... }

#[async_trait]
impl ExecutionEnvironment for DockerExecutor { ... }
```

### Step 3: Implement DockerExecutor::new()
**Files:** `crates/terraphim_rlm/src/executor/docker.rs`
**Description:** Connect to Docker daemon, initialize fields
**Tests:** Unit test for creation
**Estimated:** 1 hour

Key code:
```rust
pub fn new(config: RlmConfig) -> Result<Self, RlmError> {
    let docker = bollard::Docker::connect_with_local_defaults()
        .map_err(|e| RlmError::BackendInitFailed {
            backend: "docker".to_string(),
            message: format!("Failed to connect to Docker: {}", e),
        })?;

    Ok(Self {
        config,
        docker,
        session_to_container: parking_lot::RwLock::new(HashMap::new()),
        container_configs: parking_lot::RwLock::new(HashMap::new()),
        image: "python:3.11-slim".to_string(),
        capabilities: vec![
            Capability::ContainerIsolation,
            Capability::PythonExecution,
            Capability::BashExecution,
            Capability::FileOperations,
        ],
    })
}
```

### Step 4: Implement container lifecycle (ensure_container())
**Files:** `crates/terraphim_rlm/src/executor/docker.rs`
**Description:** Create container for session if not exists
**Tests:** Integration test with container creation
**Estimated:** 2 hours

Key code:
```rust
async fn ensure_container(&self, session_id: &SessionId) -> Result<String, RlmError> {
    // Check if container already exists for session
    // If yes, return container id
    // If no, create new container with python:3.11-slim
    // Store mapping session_id -> container_id
}
```

### Step 5: Implement execute_code()
**Files:** `crates/terraphim_rlm/src/executor/docker.rs`
**Description:** Use docker exec to run python code
**Tests:** Unit test for python execution
**Estimated:** 1 hour

Key code:
```rust
async fn execute_code(&self, code: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error> {
    let container_id = self.ensure_container(&ctx.session_id).await?;
    // Use docker exec with /bin/bash -c "python3 -c 'code'"
    // Capture stdout/stderr/exit code
}
```

### Step 6: Implement execute_command()
**Files:** `crates/terraphim_rlm/src/executor/docker.rs`
**Description:** Use docker exec to run bash command
**Tests:** Unit test for bash execution
**Estimated:** 1 hour

### Step 7: Implement remaining trait methods
**Files:** `crates/terraphim_rlm/src/executor/docker.rs`
**Description:** validate, create_snapshot, list_snapshots, delete_snapshot, etc.
**Tests:** Unit tests
**Estimated:** 2 hours

### Step 8: Wire up in select_executor()
**Files:** `crates/terraphim_rlm/src/executor/mod.rs`
**Description:** Add DockerExecutor to backend selection
**Tests:** Integration test
**Estimated:** 1 hour

Key code (change from line 123-129):
```rust
BackendType::Docker if is_docker_available() => {
    log::info!("Selected Docker backend");
    let executor = DockerExecutor::new(config.clone())?;
    return Ok(Box::new(executor));
}
```

### Step 9: Add health_check()
**Files:** `crates/terraphim_rlm/src/executor/docker.rs`
**Description:** Ping Docker daemon to verify connectivity
**Tests:** Unit test
**Estimated:** 30 minutes

### Step 10: Add cleanup()
**Files:** `crates/terraphim_rlm/src/executor/docker.rs`
**Description:** Delete all containers on shutdown
**Tests:** Integration test
**Estimated:** 30 minutes

### Step 11: Update SKILL.md
**Files:** `~/.config/opencode/skills/terraphim-rlm/references/docker-executor.md`
**Description:** Document DockerExecutor for skill users
**Tests:** None
**Estimated:** 30 minutes

## Rollback Plan

If issues discovered:
1. Revert changes to `executor/mod.rs` - DockerExecutor removed from select_executor
2. `LocalExecutor` becomes fallback again
3. Feature flag `docker-backend` can be disabled

Feature flag: `docker-backend = ["dep:bollard"]` in Cargo.toml

## Dependencies

### No New Dependencies
| Crate | Status |
|-------|--------|
| bollard | Already in Cargo.toml as optional |
| parking_lot | Already used in FirecrackerExecutor |
| async-trait | Already used in all executors |

### No Dependency Updates
All required dependencies already present.

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| Container creation | < 2s (first time, image pull) | Benchmark |
| Container reuse | < 100ms overhead | Benchmark |
| Execution latency | < 50ms vs LocalExecutor | Benchmark |
| Memory per container | ~50MB | Docker stats |

### Benchmarks to Add
```rust
#[bench]
fn bench_docker_execute_python(b: &mut Bencher) {
    // Reuse container across iterations
}
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Docker socket path on mac | Confirmed: `/var/run/docker.sock` or Orbstack equivalent | Alex |
| Container image pull on first use | May need to pre-pull or accept delay | Alex |
| Session timeout vs container timeout | Need to align with ExecutionContext.timeout_ms | Alex |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received