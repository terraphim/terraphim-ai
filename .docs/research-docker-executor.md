# Research Document: DockerExecutor Implementation

**Status**: In Review
**Author**: Alex
**Date**: 2026-05-14

## Executive Summary

Implement `DockerExecutor` to provide container-based isolation for terraphim_rlm on macOS/Linux where Firecracker/KVM is unavailable. Docker containers provide namespace isolation (PID, NET, IPC, Mount) without requiring hardware virtualization. This enables sandboxed code execution using the existing `ExecutionEnvironment` trait and `bollard` crate already in dependencies.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | macOS dev workflow hampered by lack of isolation backend |
| Leverages strengths? | Yes | bollard already dependency, Docker available on mac |
| Meets real need? | Yes | RLM cannot use Firecracker on mac, Local is untrusted |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
terraphim_rlm lacks a working container-based execution backend. The `select_executor` function detects Docker is available but falls back to `LocalExecutor` because `DockerExecutor` is not implemented. This leaves macOS users without sandboxed execution.

### Impact
- macOS developers cannot test RLM with isolation
- Untrusted code runs directly on host via LocalExecutor
- Feature gap vs Firecracker on Linux

### Success Criteria
1. `cargo test -p terraphim_rlm` passes with DockerExecutor tests
2. `BackendType::Docker` selected when Docker available, no KVM
3. Code execution in Docker containers with proper cleanup

## Current State Analysis

### Existing Implementation

| Component | Location | Purpose |
|-----------|----------|---------|
| ExecutionEnvironment trait | `executor/trait.rs` | Interface for all backends |
| FirecrackerExecutor | `executor/firecracker.rs` | Full VM isolation (902 lines) |
| LocalExecutor | `executor/local.rs` | No isolation (250 lines) |
| select_executor | `executor/mod.rs:78-143` | Backend selection logic |
| ExecutionContext | `executor/context.rs` | Shared types |
| Capability enum | `executor/context.rs:281-303` | Backend capabilities |

### Code Locations - Key Methods to Implement

```
executor/mod.rs:123-129  → DockerExecutor placeholder (tried.push("docker (not yet implemented)"))
executor/mod.rs:135-137  → LocalExecutor fallback
executor/local.rs:130-209 → LocalExecutor ExecutionEnvironment impl (reference)
executor/firecracker.rs:481-782 → FirecrackerExecutor ExecutionEnvironment impl (reference)
```

### Data Flow

```
TerraphimRlm → select_executor() → [Firecracker | Docker | Local]
                               Docker branch currently returns error
                               with "not yet implemented"
```

### Integration Points

| Point | Integration |
|-------|-------------|
| `ExecutionEnvironment` trait | Implement 13 methods |
| `BackendType::Docker` enum | Add to config selection |
| `select_executor()` | Wire up Docker availability check |
| `Capability` enum | Add ContainerIsolation capability |
| bollard crate | Already in Cargo.toml |

## Constraints

### Technical Constraints
- **bollard 0.20.2** already in dependency tree
- **Docker Desktop/Orbstack** on mac - supports runc runtime
- **No gVisor** on mac (not available via Homebrew)
- **Session affinity** - Docker containers replace VMs conceptually

### Non-Functional Requirements
| Requirement | Target |
|-------------|--------|
| Container startup | < 2 seconds |
| Execution latency | < 100ms overhead vs local |
| Cleanup | Container removal on session destroy |
| Isolation | Separate PID/Network/Mount namespaces |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Must implement ExecutionEnvironment trait | API contract with RLM core | trait.rs defines 13 methods |
| Must clean up containers on session destroy | Resource leak prevention | FirecrackerImpl shows session_to_vm tracking |
| Must support both Python and bash execution | Core RLM functionality | LocalExecutor shows python_path and bash command patterns |

### Eliminated from Scope
| Item | Why Eliminated |
|------|----------------|
| Snapshot support with Docker commit | Complexity; not critical for v1 |
| gVisor/runc runtime selection | mac Docker uses runc by default |
| Container pooling | Over-engineering for initial implementation |
| Network isolation (dns_allowlist) | Future enhancement |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_rlm::executor::trait | Must implement exactly | Low - well-documented trait |
| terraphim_rlm::executor::context | Use ExecutionContext, ExecutionResult | Low - simple types |
| bollard crate | HTTP to Docker daemon | Low - mature crate |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| bollard | 0.20.2 | Low | None needed |
| tokio | workspace | None | Already using |
| async-trait | workspace | None | Already using |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Docker daemon not running | Medium | High | is_docker_available() check already exists |
| Container cleanup failures | Low | Medium | Delete on Drop pattern |
| Port conflicts if container networking used | Low | Low | Use container exec, no port mapping |

### Open Questions
1. **Container image**: Should we use `python:3.11-slim` or require user to pre-configure? → Use `python:3.11-slim` as default
2. **Session mapping**: Should DockerExecutor track session→container 1:1 like Firecracker? → Yes, HashMap<SessionId, ContainerId>
3. **Timeout handling**: Docker exec has built-in timeout support? → Use bollard's CreateContainerOptions and exec start with timeout

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong |
|------------|-------|---------------|
| Docker daemon accessible via unix:///var/run/docker.sock | Standard macOS Docker Desktop location | Docker Desktop may use different socket path |
| bollard API stable for container exec operations | Version 0.20.2 is recent | Minor - API changes rare |
| Container removal is synchronous and reliable | Docker design assumption | Containers may get stuck in removing state |

## Research Findings

### Key Insights
1. **FirecrackerExecutor pattern**: Use `session_to_vm: HashMap<SessionId, String>` for container affinity
2. **LocalExecutor pattern**: Use `python_path` field, `build_command()` and `build_python_command()` methods
3. **bollard API**: Container creation → exec creation → exec start → stream output
4. **Cleanup pattern**: Firecracker uses `cleanup()` method, Docker should use `delete_container()` on session end

### Relevant Prior Art
- **FirecrackerExecutor** (902 lines): Best reference for session-to-resource mapping
- **LocalExecutor** (250 lines): Best reference for simple execution pattern
- **bollard examples**: Container exec pattern well-documented in crate

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| bollard container exec test | Verify API for exec-in-container | 30 minutes |
| Docker socket path discovery | Confirm macOS path works | 15 minutes |

## Recommendations

### Proceed/No-Proceed
**Proceed** - Essential question check passes, clear path to implementation.

### Scope Recommendations
1. Implement DockerExecutor with basic container lifecycle
2. Support python and bash execution via docker exec
3. Track session→container mapping for affinity
4. Clean up containers on session destroy
5. Skip snapshot support (defer to v2)

### Risk Mitigation Recommendations
1. Use existing `is_docker_available()` check in select_executor
2. Add health_check that pings Docker daemon
3. Implement Drop for container cleanup as backup

## Next Steps

1. Proceed to Phase 2 (Design) - create implementation plan
2. Human review of research document
3. Specification interview (Phase 2.5) to confirm edge cases

## Appendix

### Reference: bollard Container Exec Pattern

```rust
// Pseudo-code for Docker exec pattern (from bollard docs)
let config = CreateContainerConfig { image: "python:3.11-slim", cmd: vec!["sleep", "3600"] };
let container = docker.create_container(None, config).await?;
let exec = docker.create_exec(&container.id, ExecConfig { attach_stdout: true, cmd: vec!["python", "-c", "print('hi')"] }).await?;
docker.start_exec(&exec.id, StartExecResults::Attached).await?;
```

### Reference: FirecrackerExecutor Session Tracking

```rust
// From firecracker.rs - session to VM mapping pattern
session_to_vm: parking_lot::RwLock<HashMap<SessionId, String>>
current_snapshot: parking_lot::RwLock<HashMap<SessionId, String>>
snapshot_counts: parking_lot::RwLock<HashMap<SessionId, u32>>
```

### Reference: LocalExecutor Execution Pattern

```rust
// From local.rs - simple execution approach
async fn execute_code(&self, code: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error> {
    let cmd = self.build_python_command(code, ctx);
    self.run_command(cmd).await
}

fn run_command(&self, mut cmd: Command) -> RlmResult<ExecutionResult> {
    let output = timeout(Duration::from_millis(30000), cmd.output()).await;
    // ... handle result
}
```