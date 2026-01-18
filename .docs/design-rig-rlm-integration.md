# Design & Implementation Plan: Terraphim RLM Integration

**Version**: 1.3
**Date**: 2026-01-06
**Phase**: 2 (Disciplined Design)
**Based on**: research-rig-rlm-integration.md, specification-rig-rlm-integration.md (v1.2)
**Updated**:
- v1.1: Cross-checked against original Python RLM (github.com/alexzhang13/rlm)
- v1.2: Added sandbox alternatives (Docker/gVisor, E2B, Modal)
- v1.3: Architecture review + specification interview findings (firecracker-rust analysis, pool overflow, autoscaling, DNS security, session extension, KG strictness)

---

## 1. Summary of Target Behavior

After implementation, the terraphim-ai system will:

1. **Execute LLM-generated code** in isolated Firecracker VMs with sub-2 second boot times
2. **Support recursive LLM calls** where code inside VMs can invoke parent LLM via HTTP with session token validation
3. **Support batched LLM queries** via `llm_query_batched()` for concurrent sub-LLM calls
4. **Validate commands** against knowledge graph with configurable strictness (permissive/normal/strict)
5. **Manage sessions** with VM affinity, session extension API, and checkpoint-to-new-session capability
6. **Provide snapshot/rollback** capabilities for session state versioning (ignoring external state drift on restore)
7. **Enforce dual budgets** (tokens + time) to prevent runaway recursive execution
8. **Log execution trajectories** in JSONL format for debugging and analysis
9. **Expose 6 specialized MCP tools** for Claude Code integration with MCP-wrapped errors
10. **Support pool autoscaling** triggered by queue depth with configurable thresholds
11. **Spawn overflow VMs** (up to 3 concurrent) when pool exhausted, with lower-priority cleanup
12. **Enforce DNS allowlist** blocking non-approved domains with forensic logging of blocked attempts
13. **Support dynamic disk expansion** for OverlayFS (100MB → 2GB) with overlay-always-wins package priority
14. **Provide auto-remediation** for common failures with alert escalation after 3+ failures in 5 minutes
15. **Use preference + fallback backend selection** where users specify preferred backend AND fallback order

**Key Architectural Change**: New `terraphim_rlm` crate that bypasses rig-core entirely, using `terraphim_service::GenericLlmClient` directly for all LLM operations. Clear state ownership: SessionManager owns session state, VmManager owns VM health/lifecycle.

---

## 2. Key Invariants and Acceptance Criteria

### 2.1 Invariants (Must Always Hold)

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| INV-1 | VM isolation: code cannot access host filesystem | Firecracker VM with no host mounts |
| INV-2 | Budget enforcement: execution terminates when either token or time budget exceeded | BudgetTracker checked on every LLM call and command |
| INV-3 | Session affinity: same conversation always routes to same VM | SessionManager maintains conversation→VM mapping |
| INV-4 | Snapshot consistency: restore returns exact state at snapshot time | Firecracker snapshot includes full VM state |
| INV-5 | Cancellation propagates: parent cancel terminates all children | CancellationToken passed through execution tree |
| INV-6 | State ownership: SessionManager owns session state, VmManager owns VM lifecycle | Clear struct boundaries, no cross-ownership |
| INV-7 | DNS security: blocked domains logged but never allowed | DNS proxy with allowlist enforcement |
| INV-8 | Fail-fast: executor never retries, errors propagate to client | No retry logic in executor layer |
| INV-9 | Overflow limit: max 3 concurrent overflow VMs | AtomicU32 counter with CAS operations |

### 2.2 Acceptance Criteria (Testable)

| ID | Criterion | Test |
|----|-----------|------|
| AC-1 | `execute_code("print('hello')")` returns `"hello\n"` within 3s | Integration test |
| AC-2 | `execute_command("ls /")` returns directory listing | Integration test |
| AC-3 | Recursive `query_llm()` from VM invokes host LLM | Integration test with mock LLM |
| AC-4 | Unknown KG term after 3 retries returns `KgEscalationRequired` | Unit test |
| AC-5 | Token budget exceeded returns `TokenBudgetExceeded` error | Unit test |
| AC-6 | Snapshot create→modify→restore returns original state | Integration test |
| AC-7 | All 6 MCP tools callable via protocol | MCP integration test |
| AC-8 | VM allocation from pool < 500ms (p95) | Benchmark test |
| AC-9 | `llm_query_batched(["a","b","c"])` returns 3 results concurrently | Integration test |
| AC-10 | `FINAL_VAR(variable_name)` returns variable value from REPL | Unit test |
| AC-11 | First iteration prevents immediate FINAL() (safeguard) | Unit test |
| AC-12 | Trajectory log written to JSONL after execution | Integration test |
| AC-13 | `DockerExecutor` runs code with gVisor when available | Integration test |
| AC-14 | `E2bExecutor` creates/executes/destroys sandbox via REST | Integration test |
| AC-15 | Auto-backend selection chooses appropriate executor | Unit test |
| AC-16 | Pool overflow spawns up to 3 concurrent VMs when exhausted | Integration test |
| AC-17 | VM crash returns error immediately (fail-fast) | Unit test |
| AC-18 | Queue depth > 5 triggers pool scale-up | Integration test |
| AC-19 | DNS query to non-allowlist domain is blocked + logged | Integration test |
| AC-20 | LLM bridge validates session token before processing | Unit test |
| AC-21 | `extend_session()` adds 30 min, max 3 extensions | Unit test |
| AC-22 | Checkpoint-to-new-session creates new session from snapshot | Integration test |
| AC-23 | OverlayFS expands from 100MB to max 2GB dynamically | Integration test |
| AC-24 | Session-installed packages shadow base packages | Integration test |
| AC-25 | 3+ failures in 5 min triggers alert webhook | Integration test |
| AC-26 | KG strictness `permissive` mode warns but doesn't block | Unit test |
| AC-27 | MCP tool errors wrapped in MCP error format | Unit test |
| AC-28 | Backend preference list honored with fallback | Unit test |

---

## 3. High-Level Design and Boundaries

### 3.1 Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    NEW CRATE: terraphim_rlm                             │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ TerraphimRlm (public API)                                       │    │
│  │  - query(), execute_code(), execute_command()                   │    │
│  │  - create_session(), destroy_session()                          │    │
│  │  - create_snapshot(), restore_snapshot()                        │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                              │                                          │
│    ┌─────────────────────────┼─────────────────────────┐                │
│    ▼                         ▼                         ▼                │
│  ┌───────────────┐  ┌────────────────┐  ┌───────────────────┐           │
│  │ QueryLoop     │  │ SessionManager │  │ BudgetTracker     │           │
│  │ - Parse cmd   │  │ - VM affinity  │  │ - Token counting  │           │
│  │ - Execute     │  │ - Snapshots    │  │ - Time tracking   │           │
│  │ - Result      │  │ - Context vars │  │ - Recursion depth │           │
│  └───────┬───────┘  └───────┬────────┘  └─────────┬─────────┘           │
│          │                  │                     │                     │
│          ▼                  ▼                     ▼                     │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ KnowledgeGraphValidator                                         │    │
│  │  - Uses terraphim_automata for term matching                    │    │
│  │  - Retry logic with LLM rephrasing                              │    │
│  │  - Escalation to user after N failures                          │    │
│  └─────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌───────────────────┐ ┌───────────────────┐ ┌───────────────────┐
│ terraphim_        │ │ terraphim_        │ │ terraphim_        │
│ firecracker       │ │ service           │ │ agent_supervisor  │
│ (MODIFY)          │ │ (USE)             │ │ (USE)             │
│ + ExecutionEnv    │ │ GenericLlmClient  │ │ SupervisedAgent   │
│   trait impl      │ │ token counting    │ │ lifecycle         │
└───────────────────┘ └───────────────────┘ └───────────────────┘
```

### 3.2 Boundaries and Responsibilities

| Component | Responsibility | Does NOT Do |
|-----------|----------------|-------------|
| `terraphim_rlm` | RLM orchestration, session management, query loop | VM lifecycle, LLM calls |
| `terraphim_firecracker` | VM pool, execution, snapshots | LLM, session logic |
| `terraphim_service` | LLM providers, token counting | Execution, sessions |
| `terraphim_automata` | Text matching, KG validation | Execution, LLM |
| `terraphim_mcp_server` | MCP protocol, tool exposure | RLM logic |

### 3.3 Complected Areas to Avoid

| Risk | How Avoided |
|------|-------------|
| Mixing VM lifecycle with session logic | Clear trait boundary: `ExecutionEnvironment` only handles execution |
| Coupling KG validation to execution | Validator is separate component, called before execution |
| LLM provider details leaking to RLM | Use `GenericLlmClient` trait, no provider-specific code in RLM |

---

## 4. File/Module-Level Change Plan

### 4.1 New Crate: `terraphim_rlm`

| File | Action | Responsibility | Dependencies |
|------|--------|----------------|--------------|
| `crates/terraphim_rlm/Cargo.toml` | **Create** | Crate manifest | terraphim_firecracker, terraphim_service, terraphim_automata, terraphim_agent_supervisor |
| `crates/terraphim_rlm/src/lib.rs` | **Create** | Public API exports | All submodules |
| `crates/terraphim_rlm/src/rlm.rs` | **Create** | `TerraphimRlm` struct and public methods | All components |
| `crates/terraphim_rlm/src/session.rs` | **Create** | `SessionManager`: VM affinity, context, snapshots | terraphim_firecracker |
| `crates/terraphim_rlm/src/query_loop.rs` | **Create** | `QueryLoop`: command parsing, execution loop | session, executor |
| `crates/terraphim_rlm/src/budget.rs` | **Create** | `BudgetTracker`: token/time/depth tracking | None |
| `crates/terraphim_rlm/src/validator.rs` | **Create** | `KnowledgeGraphValidator`: KG validation | terraphim_automata |
| `crates/terraphim_rlm/src/command.rs` | **Create** | `Command` enum (RUN/FINAL/FINAL_VAR/RunCode) and parsing | None |
| `crates/terraphim_rlm/src/config.rs` | **Create** | `RlmConfig` struct | None |
| `crates/terraphim_rlm/src/error.rs` | **Create** | `RlmError` enum | thiserror |
| `crates/terraphim_rlm/src/types.rs` | **Create** | Shared types: SessionId, SnapshotId, QueryMetadata, etc. | uuid |
| `crates/terraphim_rlm/src/preamble.rs` | **Create** | LLM preamble/prompt constants, iteration safeguards | None |
| `crates/terraphim_rlm/src/llm_bridge.rs` | **Create** | HTTP endpoint for VM→host LLM calls, batched support, session token validation | hyper, terraphim_service |
| `crates/terraphim_rlm/src/logger.rs` | **Create** | `TrajectoryLogger`: JSONL logging of iterations | serde_json |
| `crates/terraphim_rlm/src/autoscaler.rs` | **Create** | Pool autoscaling: queue depth trigger, scale up/down logic | tokio |
| `crates/terraphim_rlm/src/dns_security.rs` | **Create** | DNS allowlist enforcement, blocked query logging | trust-dns-resolver |
| `crates/terraphim_rlm/src/operations.rs` | **Create** | Auto-remediation, alert escalation, failure tracking | reqwest (webhook) |
| `crates/terraphim_rlm/src/mcp_errors.rs` | **Create** | MCP error format wrapping for all RLM errors | serde_json |

### 4.2 Modify: `terraphim_firecracker`

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `src/lib.rs` | **Modify** | VM pool exports | + `ExecutionEnvironment` trait export | None |
| `src/executor.rs` | **Create** | - | `FirecrackerExecutor` impl of trait | pool, vm |
| `src/executor/mod.rs` | **Create** | - | Module structure for executor | None |
| `src/executor/trait.rs` | **Create** | - | `ExecutionEnvironment` trait definition | async-trait |
| `src/executor/context.rs` | **Create** | - | `ExecutionContext`, `ExecutionResult` | None |
| `src/pool/mod.rs` | **Modify** | Pool management | + overflow VM support, autoscaling hooks | vm |
| `src/pool/overflow.rs` | **Create** | - | Overflow VM spawning (max 3 concurrent) | AtomicU32 |
| `src/pool/autoscaler.rs` | **Create** | - | Queue depth monitoring, scale triggers | tokio::sync::watch |
| `src/vm/snapshot.rs` | **Create** | - | Snapshot create/restore operations | firecracker API |
| `src/vm/overlay.rs` | **Create** | - | OverlayFS management with dynamic expansion | nix |
| `src/network/audit.rs` | **Create** | - | Network audit logging | iptables |
| `src/network/dns_proxy.rs` | **Create** | - | DNS allowlist proxy, blocked query logging | trust-dns-resolver |

### 4.3 New: Alternative Executor Backends

| File | Action | Responsibility | Dependencies |
|------|--------|----------------|--------------|
| `crates/terraphim_rlm/src/executor/mod.rs` | **Create** | Executor trait + backend selection | None |
| `crates/terraphim_rlm/src/executor/firecracker.rs` | **Create** | `FirecrackerExecutor` wrapper | terraphim_firecracker |
| `crates/terraphim_rlm/src/executor/docker.rs` | **Create** | `DockerExecutor` with gVisor/runc support | bollard |
| `crates/terraphim_rlm/src/executor/e2b.rs` | **Create** | `E2bExecutor` REST client | reqwest |
| `crates/terraphim_rlm/src/executor/modal.rs` | **Create** | `ModalExecutor` (future) | reqwest |

### 4.4 Modify: `terraphim_mcp_server`

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `src/tools/mod.rs` | **Modify** | Existing tool exports | + RLM tool exports | None |
| `src/tools/rlm.rs` | **Create** | - | RLM tool implementations | terraphim_rlm |
| `src/tools/rlm/code.rs` | **Create** | - | `rlm_code` tool | terraphim_rlm |
| `src/tools/rlm/bash.rs` | **Create** | - | `rlm_bash` tool | terraphim_rlm |
| `src/tools/rlm/query.rs` | **Create** | - | `rlm_query` tool | terraphim_rlm |
| `src/tools/rlm/context.rs` | **Create** | - | `rlm_context` tool | terraphim_rlm |
| `src/tools/rlm/snapshot.rs` | **Create** | - | `rlm_snapshot` tool | terraphim_rlm |
| `src/tools/rlm/status.rs` | **Create** | - | `rlm_status` tool | terraphim_rlm |

### 4.4 Modify: Workspace

| File | Action | Change |
|------|--------|--------|
| `Cargo.toml` | **Modify** | Add `terraphim_rlm` to workspace members |

---

## 5. Step-by-Step Implementation Sequence

### Phase 1: Foundation (Steps 1-4)

| Step | Purpose | Files | Deployable? | Notes |
|------|---------|-------|-------------|-------|
| 1 | Create crate skeleton | `terraphim_rlm/Cargo.toml`, `lib.rs` | Yes | Empty crate, compiles |
| 2 | Define core types | `types.rs`, `error.rs`, `config.rs` | Yes | No logic yet |
| 3 | Define `ExecutionEnvironment` trait | `terraphim_firecracker/src/executor/trait.rs` | Yes | Trait only |
| 4 | Implement `FirecrackerExecutor` stub | `terraphim_firecracker/src/executor.rs` | Yes | Returns dummy results |

**Checkpoint**: Crate compiles, trait exists, stub executor works.

### Phase 2: Core Execution (Steps 5-9)

| Step | Purpose | Files | Deployable? | Notes |
|------|---------|-------|-------------|-------|
| 5 | Implement VM allocation in executor | `executor.rs` + pool integration | Yes | Real VM allocation |
| 6 | Implement `execute_command()` | `executor.rs` | Yes | Bash in VM |
| 7 | Implement `execute_code()` | `executor.rs` | Yes | Python in VM |
| 8 | Add output streaming to file | `executor.rs` | Yes | Large output handling |
| 9 | Implement `query_llm()` HTTP endpoint | `terraphim_rlm/src/llm_bridge.rs` | Yes | VM→host LLM calls |
| 9a | Implement `query_llm_batched()` endpoint | `terraphim_rlm/src/llm_bridge.rs` | Yes | Concurrent batch queries |

**Checkpoint**: Can execute code/commands in VM, recursive LLM works (single + batched).

### Phase 3: Session Management (Steps 10-14)

| Step | Purpose | Files | Deployable? | Notes |
|------|---------|-------|-------------|-------|
| 10 | Implement `SessionManager` | `session.rs` | Yes | Basic session create/destroy |
| 11 | Add VM affinity | `session.rs` | Yes | Session→VM mapping |
| 12 | Implement context variables | `session.rs` | Yes | get/set context |
| 13 | Implement snapshot create | `vm/snapshot.rs`, `session.rs` | Yes | Named snapshots |
| 14 | Implement snapshot restore | `vm/snapshot.rs`, `session.rs` | Yes | Rollback capability |

**Checkpoint**: Sessions persist across calls, snapshots work.

### Phase 4: Query Loop (Steps 15-18)

| Step | Purpose | Files | Deployable? | Notes |
|------|---------|-------|-------------|-------|
| 15 | Implement `Command` parsing (FINAL, FINAL_VAR, RUN) | `command.rs` | Yes | Parse LLM output |
| 16 | Implement `BudgetTracker` | `budget.rs` | Yes | Token/time/depth |
| 17 | Implement `QueryLoop` with iteration safeguard | `query_loop.rs` | Yes | Main execution loop |
| 17a | Add first-iteration safeguard | `preamble.rs`, `query_loop.rs` | Yes | Prevents immediate FINAL |
| 18 | Implement `TrajectoryLogger` | `logger.rs` | Yes | JSONL trajectory logging |
| 19 | Wire up `TerraphimRlm::query()` | `rlm.rs` | Yes | Full query pipeline |

**Checkpoint**: Full RLM query works end-to-end with logging.

### Phase 5: KG Validation (Steps 20-22)

| Step | Purpose | Files | Deployable? | Notes |
|------|---------|-------|-------------|-------|
| 20 | Implement `KnowledgeGraphValidator` | `validator.rs` | Yes | Basic validation |
| 21 | Add retry with rephrasing | `validator.rs` | Yes | LLM retry loop |
| 22 | Add user escalation | `validator.rs` | Yes | KgEscalationRequired error |

**Checkpoint**: KG validation complete with escalation.

### Phase 6: MCP Integration (Steps 23-26)

| Step | Purpose | Files | Deployable? | Notes |
|------|---------|-------|-------------|-------|
| 23 | Create MCP tool module | `terraphim_mcp_server/src/tools/rlm.rs` | Yes | Module structure |
| 24 | Implement `rlm_code`, `rlm_bash` | `rlm/code.rs`, `rlm/bash.rs` | Yes | Basic tools |
| 25 | Implement `rlm_query`, `rlm_context` | `rlm/query.rs`, `rlm/context.rs` | Yes | Advanced tools |
| 26 | Implement `rlm_snapshot`, `rlm_status` | `rlm/snapshot.rs`, `rlm/status.rs` | Yes | Management tools |

**Checkpoint**: All MCP tools functional.

### Phase 7: Hardening (Steps 27-30)

| Step | Purpose | Files | Deployable? | Notes |
|------|---------|-------|-------------|-------|
| 27 | Add network audit logging | `network/audit.rs` | Yes | iptables integration |
| 28 | Add overflow VM support | `pool/mod.rs` | Yes | Beyond pool limit |
| 29 | Add OverlayFS for packages | `vm/overlay.rs` | Yes | Session pip installs |
| 30 | Integrate with AgentSupervisor | `rlm.rs` | Yes | Crash recovery |

**Checkpoint**: Production-ready with all hardening.

### Phase 8: Alternative Backends (Steps 31-35)

| Step | Purpose | Files | Deployable? | Notes |
|------|---------|-------|-------------|-------|
| 31 | Implement `DockerExecutor` with bollard | `executor/docker.rs` | Yes | gVisor + runc runtime selection |
| 32 | Add Docker runtime detection | `executor/docker.rs` | Yes | Auto-detect gVisor availability |
| 33 | Implement `E2bExecutor` REST client | `executor/e2b.rs` | Yes | Create/exec/destroy sandbox |
| 34 | Add backend selection logic (preference + fallback) | `executor/mod.rs` | Yes | User-specified order with fallback |
| 35 | Integration tests for all backends | `tests/backend_test.rs` | Yes | CI-compatible test matrix |

**Checkpoint**: All execution backends functional with preference-based selection.

### Phase 9: Architecture Review Features (Steps 36-45)

| Step | Purpose | Files | Deployable? | Notes |
|------|---------|-------|-------------|-------|
| 36 | Implement pool overflow (max 3 concurrent) | `pool/overflow.rs` | Yes | AtomicU32 counter, CAS operations |
| 37 | Implement queue-depth autoscaling | `pool/autoscaler.rs` | Yes | Watch channel for scale signals |
| 38 | Add fail-fast error handling | `executor.rs` | Yes | No retry logic in executor |
| 39 | Implement DNS allowlist proxy | `network/dns_proxy.rs` | Yes | Block non-approved, log blocked |
| 40 | Add session token validation to LLM bridge | `llm_bridge.rs` | Yes | X-Session-Token header validation |
| 41 | Implement session extension API | `session.rs` | Yes | extend_session() with limits |
| 42 | Implement checkpoint-to-new-session | `session.rs`, `vm/snapshot.rs` | Yes | start_session_from_checkpoint() |
| 43 | Add dynamic OverlayFS expansion | `vm/overlay.rs` | Yes | 100MB → 2GB on demand |
| 44 | Implement auto-remediation + alerts | `operations.rs` | Yes | 3+ failures → webhook alert |
| 45 | Add configurable KG strictness | `validator.rs` | Yes | Permissive/Normal/Strict modes |

**Checkpoint**: All architecture review features implemented.

### Phase 10: MCP Error Wrapping (Steps 46-47)

| Step | Purpose | Files | Deployable? | Notes |
|------|---------|-------|-------------|-------|
| 46 | Implement MCP error format wrapper | `mcp_errors.rs` | Yes | Convert RlmError → MCP format |
| 47 | Update MCP tools to use wrapper | `tools/rlm/*.rs` | Yes | All tools return wrapped errors |

**Checkpoint**: Production-ready with comprehensive error handling.

**Backend Selection Logic** (preference + fallback):
```rust
pub fn select_executor(config: &RlmConfig) -> Result<Box<dyn ExecutionEnvironment>, RlmError> {
    // Use user-specified preference order, or default [Firecracker, E2B, Docker]
    let backends = if config.backend_preference.is_empty() {
        vec![BackendType::Firecracker, BackendType::E2B, BackendType::Docker]
    } else {
        config.backend_preference.clone()
    };

    for backend in backends {
        match backend {
            BackendType::Firecracker if is_kvm_available() => {
                return Ok(Box::new(FirecrackerExecutor::new(config)?));
            }
            BackendType::E2B if is_e2b_configured(&config) => {
                return Ok(Box::new(E2bExecutor::new(config)?));
            }
            BackendType::Docker if is_docker_available() => {
                let runtime = detect_docker_runtime(); // gVisor preferred
                return Ok(Box::new(DockerExecutor::new(config, runtime)?));
            }
            _ => continue, // Backend not available, try next
        }
    }

    Err(RlmError::NoBackendAvailable {
        tried: backends.iter().map(|b| b.to_string()).collect(),
    })
}

// Per-backend session model configuration
pub fn get_session_model(backend: BackendType, config: &RlmConfig) -> SessionModel {
    config.backend_configs
        .get(&backend)
        .map(|bc| bc.session_model)
        .unwrap_or_else(|| match backend {
            BackendType::Firecracker => SessionModel::Affinity,  // Stateful by default
            BackendType::E2B => SessionModel::Stateless,         // Stateless by default
            BackendType::Docker => SessionModel::Affinity,       // Configurable
        })
}
```

---

## 6. Testing & Verification Strategy

### 6.1 Unit Tests

| Acceptance Criterion | Test Location | Test Type |
|---------------------|---------------|-----------|
| AC-4: KG escalation after 3 retries | `terraphim_rlm/src/validator.rs` | Unit |
| AC-5: Token budget exceeded | `terraphim_rlm/src/budget.rs` | Unit |
| AC-10: FINAL_VAR returns variable value | `terraphim_rlm/src/command.rs` | Unit |
| AC-11: First iteration safeguard | `terraphim_rlm/src/query_loop.rs` | Unit |
| AC-15: Backend selection logic | `terraphim_rlm/src/executor/mod.rs` | Unit |
| AC-17: Fail-fast on crash | `terraphim_firecracker/src/executor.rs` | Unit |
| AC-20: Session token validation | `terraphim_rlm/src/llm_bridge.rs` | Unit |
| AC-21: Session extension limits | `terraphim_rlm/src/session.rs` | Unit |
| AC-26: KG strictness modes | `terraphim_rlm/src/validator.rs` | Unit |
| AC-27: MCP error wrapping | `terraphim_rlm/src/mcp_errors.rs` | Unit |
| AC-28: Backend preference order | `terraphim_rlm/src/executor/mod.rs` | Unit |
| Command parsing correctness | `terraphim_rlm/src/command.rs` | Unit |
| Config validation | `terraphim_rlm/src/config.rs` | Unit |

### 6.2 Integration Tests

| Acceptance Criterion | Test Location | Test Type |
|---------------------|---------------|-----------|
| AC-1: execute_code returns output | `terraphim_rlm/tests/execution_test.rs` | Integration |
| AC-2: execute_command returns output | `terraphim_rlm/tests/execution_test.rs` | Integration |
| AC-3: Recursive query_llm works | `terraphim_rlm/tests/recursive_test.rs` | Integration |
| AC-6: Snapshot create/restore | `terraphim_rlm/tests/snapshot_test.rs` | Integration |
| AC-7: MCP tools callable | `terraphim_mcp_server/tests/rlm_tools_test.rs` | Integration |
| AC-9: Batched query_llm concurrency | `terraphim_rlm/tests/batched_test.rs` | Integration |
| AC-12: Trajectory JSONL logging | `terraphim_rlm/tests/logger_test.rs` | Integration |
| AC-16: Pool overflow (max 3) | `terraphim_firecracker/tests/overflow_test.rs` | Integration |
| AC-18: Queue depth autoscaling | `terraphim_firecracker/tests/autoscaler_test.rs` | Integration |
| AC-19: DNS allowlist blocking | `terraphim_firecracker/tests/dns_security_test.rs` | Integration |
| AC-22: Checkpoint-to-new-session | `terraphim_rlm/tests/checkpoint_test.rs` | Integration |
| AC-23: Dynamic OverlayFS expansion | `terraphim_firecracker/tests/overlay_test.rs` | Integration |
| AC-24: Package shadowing | `terraphim_firecracker/tests/overlay_test.rs` | Integration |
| AC-25: Alert webhook trigger | `terraphim_rlm/tests/operations_test.rs` | Integration |

### 6.3 Benchmark Tests

| Acceptance Criterion | Test Location | Test Type |
|---------------------|---------------|-----------|
| AC-8: VM allocation < 500ms | `terraphim_firecracker/benches/allocation.rs` | Benchmark |
| P2: Cold boot < 2000ms | `terraphim_firecracker/benches/boot.rs` | Benchmark |
| P4: Snapshot create < 1000ms | `terraphim_firecracker/benches/snapshot.rs` | Benchmark |

### 6.4 Security Tests

| Invariant | Test Location | Test Type |
|-----------|---------------|-----------|
| INV-1: VM isolation | `terraphim_firecracker/tests/security_test.rs` | Security |
| S3: Timeout enforcement | `terraphim_rlm/tests/timeout_test.rs` | Security |
| S4: Resource limits | `terraphim_firecracker/tests/resource_test.rs` | Security |

---

## 7. Risk & Complexity Review

### 7.1 Risks from Phase 1 Research

| Risk | Mitigation in Design | Residual Risk |
|------|---------------------|---------------|
| **VM boot exceeds 2s with Python** | Pre-warmed pool + optimized images | Low: fallback to cold boot acceptable for first request |
| **Recursive LLM creates stack overflow** | Dual budget system (tokens + time) | Low: configurable limits |
| **Command parsing fails on edge cases** | terraphim_automata fuzzy matching + retry | Medium: LLM output unpredictable |
| **tokio runtime conflict** | Bypassed: no rig-core, use HTTP bridge | None: design avoids issue |
| **Ollama compatibility** | Bypassed: use terraphim_service directly | None: design avoids issue |

### 7.2 New Risks Identified

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| HTTP latency for recursive calls | Profile early, vsock in Phase 2 if needed | Medium: acceptable for MVP |
| Snapshot storage growth | Session TTL + auto-cleanup | Low: configurable limits |
| OverlayFS complexity | Start with simple tmpfs, overlay later | Low: incremental approach |
| DNS allowlist too restrictive | Configurable allowlist, start with common domains | Low: user can extend |
| Overflow VM thundering herd | Max 3 concurrent limit prevents runaway | Low: atomic counter |
| Auto-remediation infinite loop | 3-failure threshold + cooldown period | Low: bounded retries |
| Session extension abuse | Max 3 extensions, checkpoint-to-new-session for longer | Low: explicit limits |
| KG strictness misconfiguration | Clear mode descriptions, sensible defaults | Low: documentation |
| MCP error format changes | Version field in error format | Low: backwards compatible |
| Backend preference ignored | Fallback only when preferred unavailable | None: clear semantics |

### 7.3 Complexity Assessment

| Component | Complexity | Reason |
|-----------|------------|--------|
| `QueryLoop` | High | State machine, error handling, recursion |
| `SessionManager` | Medium | VM affinity, snapshot coordination, extension API |
| `FirecrackerExecutor` | Medium | VM lifecycle, output handling, fail-fast |
| `BudgetTracker` | Low | Simple arithmetic |
| `KnowledgeGraphValidator` | Medium | Retry logic, LLM interaction, strictness modes |
| `PoolOverflow` | Medium | Atomic counter, concurrent VM spawning |
| `Autoscaler` | Medium | Queue depth monitoring, scale triggers |
| `DnsProxy` | Medium | Allowlist enforcement, logging |
| `Operations` | Medium | Auto-remediation, failure tracking, webhooks |
| `McpErrors` | Low | Error format transformation |
| MCP Tools | Low | Thin wrappers over RLM API |

---

## 8. Open Questions / Decisions for Human Review

### 8.1 Decisions Needed Before Implementation

| Question | Options | Recommendation | Impact |
|----------|---------|----------------|--------|
| VM image location | Build in repo vs external registry | External registry (Docker Hub/ECR) | Build pipeline |
| Python version | 3.10, 3.11, 3.12 | 3.11 (stable, good performance) | VM image |
| Default budget values | Conservative vs permissive | Conservative (100K tokens, 5 min) | UX |

### 8.2 Decisions That Can Wait

| Question | When Needed | Current Approach |
|----------|-------------|------------------|
| vsock vs HTTP | After MVP if latency issue | HTTP (simpler) |
| Distributed VM pool | Scale beyond single node | Single node |
| Multi-language support | User demand | Python only |

### 8.3 Clarifications Requested

1. **VM image build process**: Should we add a GitHub Action for building/publishing VM images, or is manual build acceptable for MVP?

2. **KG vocabulary source**: Which existing knowledge graph files should `KnowledgeGraphValidator` use initially?

3. **Test infrastructure**: Are Firecracker integration tests expected to run in CI, or only locally (requires root)?

---

## Appendix A: Dependency Graph

```
terraphim_rlm
├── executor/ (execution backends)
│   ├── FirecrackerExecutor
│   │   └── terraphim_firecracker::VmPoolManager
│   ├── DockerExecutor
│   │   └── bollard (Docker API)
│   ├── E2bExecutor
│   │   └── reqwest (REST client)
│   └── ModalExecutor (future)
│       └── reqwest (REST client)
├── terraphim_service (LLM)
│   └── terraphim_service::GenericLlmClient
├── terraphim_automata (validation)
│   └── terraphim_automata::AutocompleteIndex
├── terraphim_agent_supervisor (lifecycle)
│   └── terraphim_agent_supervisor::SupervisedAgent
├── tokio (async runtime)
├── tokio_util (CancellationToken)
├── uuid (session/snapshot IDs)
├── thiserror (error types)
└── async-trait (trait definitions)

terraphim_mcp_server
├── terraphim_rlm (RLM functionality)
└── mcp-rust-sdk (MCP protocol)

Backend Selection Priority:
1. Firecracker (if KVM available)
2. E2B (if API key configured)
3. Docker+gVisor (if runsc available)
4. Docker+runc (fallback, weak security)
```

---

## Appendix B: File Count Summary

| Category | New Files | Modified Files | Total |
|----------|-----------|----------------|-------|
| terraphim_rlm (new crate) | 18 | 0 | 18 |
| terraphim_rlm/executor (backends) | 5 | 0 | 5 |
| terraphim_firecracker | 10 | 2 | 12 |
| terraphim_mcp_server | 7 | 1 | 8 |
| Workspace | 0 | 1 | 1 |
| **Total** | **40** | **4** | **44** |

*Updated after Python RLM cross-check: Added llm_bridge.rs, logger.rs*
*Updated after sandbox research: Added executor backends (docker.rs, e2b.rs, modal.rs)*
*Updated after architecture review v1.3: Added autoscaler.rs, dns_security.rs, operations.rs, mcp_errors.rs, pool/overflow.rs, pool/autoscaler.rs, network/dns_proxy.rs*

---

## Appendix C: Estimated LOC

| Component | Estimated LOC | Complexity |
|-----------|---------------|------------|
| terraphim_rlm core | ~1800 | High |
| FirecrackerExecutor + pool | ~1000 | Medium |
| Alternative backends (Docker, E2B) | ~600 | Medium |
| Pool overflow + autoscaling | ~400 | Medium |
| DNS security + network audit | ~300 | Medium |
| Operations + auto-remediation | ~250 | Medium |
| MCP tools + error wrapping | ~500 | Low |
| Tests | ~1500 | Medium |
| **Total** | **~6350** | - |

*Note: Estimates based on similar components in terraphim_github_runner and terraphim_multi_agent.*
*Updated after architecture review v1.3: Added pool management, DNS security, operations monitoring.*

---

## Appendix D: Python RLM Cross-Check Findings

**Source**: github.com/alexzhang13/rlm (original Python implementation)

### Features Incorporated from Python RLM

| Python Feature | Terraphim Implementation | Location |
|----------------|--------------------------|----------|
| `llm_query_batched()` | Concurrent batch queries via tokio | `llm_bridge.rs` |
| `FINAL_VAR(variable_name)` | Command variant returning variable by name | `command.rs` |
| `QueryMetadata` | Context length analysis pre-execution | `types.rs` |
| First iteration safeguard | Prevent immediate FINAL on iter 0 | `preamble.rs`, `query_loop.rs` |
| `RLMLogger` (JSONL) | `TrajectoryLogger` with JSONL output | `logger.rs` |
| `RLM_SYSTEM_PROMPT` | Comprehensive preamble with examples | `preamble.rs` |
| `build_user_prompt()` | Iteration-aware user prompts | `preamble.rs` |
| Safe builtins | Firecracker VM isolation (stronger) | N/A (VM provides) |

### Design Differences (Intentional)

| Aspect | Python RLM | Terraphim RLM | Rationale |
|--------|------------|---------------|-----------|
| Communication | Socket with 4-byte prefix | HTTP REST | Simpler, debuggable, vsock later |
| Security | Python sandbox + builtins whitelist | Firecracker VM isolation | Stronger security boundary |
| LLM providers | `other_backends` list | `GenericLlmClient` trait | Better abstraction via terraphim_service |
| Visualization | Node.js visualizer | Deferred to Phase 2 | Focus on core functionality first |
| Verbose output | Rich library | tracing + subscriber | Rust ecosystem standard |

### Not Implemented (Deferred)

| Python Feature | Status | Reason |
|----------------|--------|--------|
| Web visualizer | Deferred | Can use existing terraphim_tui |
| Verbose printer | Deferred | Use tracing subscriber |
| Multi-depth recursion | Deferred | max_depth=1 sufficient for MVP |

---

## Appendix E: Architecture Review Findings (v1.3)

**Review Date**: 2026-01-06
**Target Repository**: terraphim/firecracker-rust

### GitHub Issues Created

| Issue | Title | Priority | Phase |
|-------|-------|----------|-------|
| #14 | ExecutionEnvironment trait abstraction | High | Phase 1 |
| #15 | Pre-warmed VM pool with sub-500ms allocation | High | Phase 2 |
| #16 | OverlayFS support for session packages | Medium | Phase 7 |
| #17 | Network audit logging | Medium | Phase 7 |
| #18 | VM-to-host LLM bridge HTTP endpoint | High | Phase 2 |
| #19 | Output streaming to file for large outputs | Medium | Phase 2 |

### Key Architecture Decisions from Interview

| Decision | Spec Section | Impact |
|----------|--------------|--------|
| Parallel overflow spawn (limit 3) | 2.10.1 | Pool management, AC-16 |
| Fail fast on crash | 2.10.1 | Error handling, INV-8 |
| Queue depth autoscaling | 2.10.1 | Pool sizing, AC-18 |
| Hybrid state authority | 2.10.1 | SessionManager vs VmManager, INV-6 |
| DNS allowlist + block | 2.10.2 | Network security, AC-19 |
| Session token validation | 2.10.2 | LLM bridge auth, AC-20 |
| Explicit extension API | 2.10.3 | Session management, AC-21 |
| Checkpoint-to-new-session | 2.10.3 | Long sessions, AC-22 |
| Dynamic disk expansion | 2.10.4 | OverlayFS, AC-23 |
| Overlay always wins | 2.10.4 | Package priority, AC-24 |
| Alert escalation | 2.10.5 | Operations, AC-25 |
| Configurable KG strictness | 2.10.5 | Validation, AC-26 |
| MCP error wrapping | 2.10.6 | Error format, AC-27 |
| Preference + fallback | 2.10.6 | Backend selection, AC-28 |
| Per-backend session model | 2.10.6 | Affinity vs stateless |

### Reusable Components from firecracker-rust

| Component | Location | Use Case |
|-----------|----------|----------|
| VmManager | fcctl-core | VM lifecycle management |
| SnapshotManager | fcctl-core | Snapshot create/restore |
| FirecrackerClient | fcctl-core | Firecracker API communication |
| Session patterns | fcctl-repl | Session state management |
| SSH execution | fcctl-web | Command execution in VM |
| VmPoolManager (IP only) | fcctl-web | Pool IP allocation |

### Dependencies on firecracker-rust Issues

```
terraphim_rlm Phase 1 ──depends on──► firecracker-rust #14 (ExecutionEnvironment trait)
terraphim_rlm Phase 2 ──depends on──► firecracker-rust #15 (VM pool)
terraphim_rlm Phase 2 ──depends on──► firecracker-rust #18 (LLM bridge)
terraphim_rlm Phase 2 ──depends on──► firecracker-rust #19 (Output streaming)
terraphim_rlm Phase 7 ──depends on──► firecracker-rust #16 (OverlayFS)
terraphim_rlm Phase 7 ──depends on──► firecracker-rust #17 (Network audit)
```

*This appendix documents the architecture review conducted during Phase 2.5 (disciplined-specification) and its impact on the design.*
