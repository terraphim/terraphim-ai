# Design & Implementation Plan: PR #426 RLM Completion

## 1. Summary of Target Behavior

After implementation, the `terraphim_rlm` crate will be:
- **Secure**: Protected against path traversal, resource exhaustion, and invalid session access
- **Reliable**: Race-condition-free with atomic operations and proper timeout handling
- **CI-Compatible**: Compilable and testable without external fcctl-core dependency
- **Observable**: Comprehensive error context and memory limits enforced

## 2. Key Invariants and Acceptance Criteria

### Security Invariants
- [ ] Snapshot names validated: no `..`, `/`, `\`, or null bytes
- [ ] Code/command inputs limited to MAX_CODE_SIZE (1MB default)
- [ ] All MCP operations validate session existence before execution
- [ ] KG validation mandatory for rlm_code/rlm_bash (can be bypassed only with explicit config)

### Correctness Invariants
- [ ] Snapshot counter increments atomically with limit enforcement
- [ ] Query loop terminates within configured timeout (5 min default)
- [ ] MemoryBackend enforces MAX_MEMORY_EVENTS limit (10,000 default)
- [ ] All errors preserve full context with `#[source]` chaining

### CI/CD Invariants
- [ ] `cargo build --workspace` succeeds without fcctl-core
- [ ] `cargo test --workspace` passes with mock implementations
- [ ] VM-dependent tests gated by `FIRECRACKER_TESTS` env var

### Performance Invariants
- [ ] Parser enforces 10KB max input size
- [ ] Parser limits recursion depth to 100 levels
- [ ] Lock contention documented and minimized

## 3. High-Level Design and Boundaries

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     terraphim_rlm crate                          │
├─────────────────────────────────────────────────────────────────┤
│  Public API (rlm.rs)                                            │
│  ├─ TerraphimRlm::new()                                         │
│  ├─ create_session() / destroy_session()                        │
│  ├─ execute_code() / execute_command()                          │
│  └─ query_loop()                                                │
├─────────────────────────────────────────────────────────────────┤
│  Execution Abstraction Layer (NEW)                              │
│  ├─ ExecutionEnvironment trait                                  │
│  ├─ FirecrackerExecutor (feature = "firecracker")              │
│  └─ MockExecutor (default/test)                                 │
├─────────────────────────────────────────────────────────────────┤
│  Core Components                                                │
│  ├─ Command Parser (parser.rs) - with validation               │
│  ├─ Query Loop (query_loop.rs) - with timeout                  │
│  ├─ Session Manager (session.rs) - session validation          │
│  ├─ Trajectory Logger (logger.rs) - with limits                │
│  └─ KG Validator (validator.rs) - mandatory validation         │
├─────────────────────────────────────────────────────────────────┤
│  MCP Tools (feature = "mcp")                                    │
│  ├─ rlm_code - with input validation                           │
│  ├─ rlm_bash - with input validation                           │
│  └─ [4 other tools] - with session validation                  │
└─────────────────────────────────────────────────────────────────┘
```

### Component Boundaries

| Component | Responsibility | Boundary |
|-----------|---------------|----------|
| ExecutionEnvironment trait | Abstract VM operations | Firewall between core and fcctl-core |
| InputValidator | Centralized validation | All inputs pass through before processing |
| SessionValidator | Session existence checks | All operations validate session first |
| TimeoutManager | Query loop timeout | Enforces wall-clock limits |

## 4. File/Module-Level Change Plan

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `src/executor/mod.rs` | Create | - | ExecutionEnvironment trait definition | None |
| `src/executor/firecracker.rs` | Modify | Direct fcctl-core usage | Implements ExecutionEnvironment, adds validation | fcctl-core (gated) |
| `src/executor/mock.rs` | Create | - | Mock ExecutionEnvironment for testing | None |
| `src/validation.rs` | Create | - | Centralized input validation functions | None |
| `src/parser.rs` | Modify | No size/depth limits | Input size & recursion limits | validation.rs |
| `src/query_loop.rs` | Modify | No timeout | tokio::time::timeout integration | tokio::time |
| `src/session.rs` | Modify | Basic session mgmt | Session validation methods | validation.rs |
| `src/logger.rs` | Modify | Unbounded growth | MAX_MEMORY_EVENTS limit | parking_lot |
| `src/mcp_tools.rs` | Modify | No input validation | Input size + session validation | validation.rs, session.rs |
| `Cargo.toml` | Modify | fcctl-core required | Feature-gated fcctl-core | - |

## 5. Step-by-Step Implementation Sequence

### Phase A: Security Hardening (Priority 1)
1. **Create validation.rs**: Centralized input validation functions
   - `validate_snapshot_name()` - path traversal prevention
   - `validate_code_input()` - size limits
   - `validate_session_id()` - format validation
   - State: Deployable, adds no dependencies

2. **Fix firecracker.rs snapshot naming**: Apply `validate_snapshot_name()`
   - Location: Line 726
   - Add: Validation before any file operations
   - State: Deployable, security fix

3. **Fix firecracker.rs race condition**: Atomic snapshot counter
   - Location: Lines 692-693
   - Change: Use write() lock for check-and-increment
   - State: Deployable, correctness fix

4. **Add input validation to MCP tools**: 
   - Location: mcp_tools.rs lines 2625-2628
   - Add: MAX_CODE_SIZE constant and validation
   - State: Deployable, security fix

5. **Add session validation to MCP tools**:
   - Location: mcp_tools.rs line 2630
   - Add: Explicit session existence check
   - State: Deployable, security fix

### Phase B: Resource Management (Priority 2)
6. **Fix MemoryBackend memory leak**:
   - Location: logger.rs lines 1638-1640
   - Add: MAX_MEMORY_EVENTS limit with FIFO eviction
   - State: Deployable, reliability fix

7. **Add timeout to query loop**:
   - Location: query_loop.rs
   - Add: tokio::time::timeout wrapper
   - Config: QueryLoopConfig.timeout_duration
   - State: Deployable, reliability fix

8. **Add parser limits**:
   - Location: parser.rs
   - Add: MAX_INPUT_SIZE (10KB), MAX_RECURSION_DEPTH (100)
   - State: Deployable, reliability fix

### Phase C: CI Compatibility (Priority 3)
9. **Create ExecutionEnvironment trait**:
   - File: src/executor/mod.rs
   - Methods: execute_code(), execute_command(), create_snapshot(), etc.
   - State: Deployable, abstraction layer

10. **Implement MockExecutor**:
    - File: src/executor/mock.rs
    - State: Deployable, enables CI testing

11. **Refactor firecracker.rs**:
    - Change: Implement ExecutionEnvironment trait
    - Gate: Behind "firecracker" feature
    - State: Deployable, maintains existing functionality

12. **Update Cargo.toml**:
    - Change: fcctl-core becomes optional
    - Add: "firecracker" feature flag
    - Update: "full" feature set
    - State: Deployable, CI compatibility

### Phase D: Error Handling (Priority 4)
13. **Enhance error types**:
    - Add: `#[source]` attributes to RlmError variants
    - Replace: unwrap_or_default() with proper error handling
    - Location: firecracker.rs line 917, others
    - State: Deployable, observability improvement

### Phase E: Testing (Priority 5)
14. **Add integration test framework**:
    - File: tests/integration_test.rs
    - Gate: By FIRECRACKER_TESTS env var
    - State: Deployable, testing infrastructure

15. **Add unit tests for validation**:
    - File: src/validation.rs (inline tests)
    - Coverage: All validation functions
    - State: Deployable, test coverage

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| Path traversal blocked | Unit | `src/validation.rs` - test_validate_snapshot_name_rejects_path_traversal |
| Input size enforced | Unit | `src/validation.rs` - test_validate_code_input_size_limit |
| Session validation works | Unit | `src/mcp_tools.rs` - test_session_validation_fails_for_invalid_session |
| Snapshot counter atomic | Unit | `src/executor/firecracker.rs` - test_concurrent_snapshot_creation |
| Timeout triggers | Integration | `tests/query_loop_test.rs` - test_query_loop_timeout |
| Memory limit enforced | Unit | `src/logger.rs` - test_memory_backend_event_limit |
| Parser limits enforced | Unit | `src/parser.rs` - test_parser_size_limit, test_parser_recursion_limit |
| CI build succeeds | CI | GitHub Actions workflow |
| Mock executor works | Unit | `src/executor/mock.rs` - test_mock_executor_basic |
| Error context preserved | Unit | Various - verify `#[source]` propagation |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| External dependency unavailable | ExecutionEnvironment trait + MockExecutor | Mock may not match real behavior |
| Security vulnerabilities | Comprehensive validation layer | Zero-day vulnerabilities in dependencies |
| Race conditions | Atomic operations, documented lock ordering | Complex concurrent scenarios untested |
| Memory exhaustion | Enforced limits with eviction | Limits may be too high/low for production |
| Timeout handling | Configurable timeouts | May interrupt legitimate long-running operations |
| Lock contention | Lock ordering documentation | May still occur under high load |
| Feature flag complexity | Clear documentation, sensible defaults | User confusion about which features to enable |

## 8. Open Questions / Decisions for Human Review

1. **Firecracker-rust Timeline**: Should we proceed with abstraction layer immediately, or wait for firecracker-rust PRs to merge?

2. **Default Feature Set**: Should "mcp" be in default features, or opt-in?

3. **Validation Strictness**: Should KG validation be mandatory (blocking) or optional (warning) for code execution?

4. **Resource Limits**: Are proposed limits appropriate?
   - MAX_CODE_SIZE: 1MB
   - MAX_MEMORY_EVENTS: 10,000
   - MAX_INPUT_SIZE: 10KB
   - MAX_RECURSION_DEPTH: 100
   - Query timeout: 5 minutes
   - max_snapshots_per_session: 50

5. **Lock Ordering**: Preferred order: SessionManager → VmManager → SnapshotManager?

6. **Error Handling**: Should all errors use thiserror with `#[source]`, or are there cases for simple error types?

7. **CI Testing**: Should we add a GitHub Actions job that runs VM tests on self-hosted runner (bigbox)?
