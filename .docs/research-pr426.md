# Research Document: PR #426 RLM Orchestration Completion

## 1. Problem Restatement and Scope

PR #426 implements the `terraphim_rlm` crate for Recursive Language Model (RLM) orchestration with isolated code execution in Firecracker VMs. The implementation is substantial (5,681 additions, 108 tests) but has critical security vulnerabilities, race conditions, and external dependency issues blocking merge.

**IN Scope:**
- Fix critical security vulnerabilities (path traversal, input validation)
- Fix race conditions in snapshot management
- Fix memory leaks and resource exhaustion issues
- Resolve external fcctl-core dependency for CI compatibility
- Add missing integration tests
- Add timeout handling to query loop
- Add input validation to parser

**OUT of Scope:**
- Firecracker-rust PRs #14-19 (assumed implemented)
- Full VM integration testing infrastructure
- Production deployment configuration

## 2. User & Business Outcomes

**User Outcomes:**
- Safe execution of Python/bash code in isolated VMs via MCP tools
- Session management with budget tracking (tokens, time, recursion depth)
- Snapshot/rollback capabilities for VM state management
- Trajectory logging for audit and debugging
- Knowledge graph validation for command safety

**Business Outcomes:**
- Secure AI agent execution environment
- Observable and auditable AI operations
- Integration with Terraphim knowledge graph
- Foundation for recursive LLM workflows

## 3. System Elements and Dependencies

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| terraphim_rlm crate | `crates/terraphim_rlm/` | Main RLM orchestration | fcctl-core (external), terraphim_types, terraphim_automata, terraphim_rolegraph |
| Command Parser | `src/parser.rs` | Parse LLM output commands | None |
| Query Loop | `src/query_loop.rs` | Orchestrate execution flow | tokio, async-trait |
| Firecracker Executor | `src/executor/firecracker.rs` | VM execution | fcctl-core::VmManager, SnapshotManager |
| Session Manager | `src/session.rs` | Session lifecycle | parking_lot::RwLock |
| Trajectory Logger | `src/logger.rs` | JSONL event logging | serde_json |
| KG Validator | `src/validator.rs` | Command validation | terraphim_automata, terraphim_rolegraph |
| MCP Tools | `src/mcp_tools.rs` | MCP protocol tools | rmcp 0.9.0 |

**External Dependencies:**
- `fcctl-core` from firecracker-rust (path dependency - not in CI)
- `rmcp` 0.9.0 for MCP protocol
- Firecracker VM with KVM (runtime only)

## 4. Constraints and Their Implications

**Security Constraints:**
- Path traversal prevention: Snapshot names must not contain `..` or path separators
- Input size limits: Code/command inputs must have MAX_CODE_SIZE (recommend 1MB)
- Session validation: All operations must verify session exists before proceeding
- Race condition prevention: Snapshot counter increment must be atomic

**Performance Constraints:**
- Memory leak prevention: MemoryBackend must have MAX_MEMORY_EVENTS limit
- Lock contention: Multiple simultaneous locks increase contention
- Timeout handling: Query loop needs overall timeout to prevent indefinite hangs

**CI/CD Constraints:**
- External dependencies break CI: fcctl-core not available in GitHub Actions
- KVM not available in CI: VM tests must be conditionally compiled/gated
- 429 errors: VM allocation rate limiting in GitHub runner

**Operational Constraints:**
- Error context preservation: Use `#[source]` attribute for error chaining
- Input length limits: Parser needs 10KB max and recursion depth limits
- Silent error handling: Replace `unwrap_or_default()` with proper error propagation

## 5. Risks, Unknowns, and Assumptions

**Critical Risks:**

1. **Security Vulnerabilities (HIGH)**
   - Path traversal in snapshot naming (firecracker.rs:726)
   - No size limits on MCP inputs (mcp_tools.rs:2625-2628)
   - Missing session validation (mcp_tools.rs:2630)
   - De-risk: Immediate fixes required before any merge

2. **Race Conditions (HIGH)**
   - Snapshot counter check-and-increment not atomic (firecracker.rs:692-693)
   - Can exceed max_snapshots_per_session
   - De-risk: Use write() lock for entire operation

3. **External Dependency Failure (HIGH)**
   - fcctl-core from firecracker-rust repository unavailable in CI
   - Blocks CI/CD pipeline
   - De-risk: Make optional with feature gate or mock implementation

4. **Memory Leaks (MEDIUM)**
   - Unbounded Vec growth in MemoryBackend (logger.rs:1638-1640)
   - De-risk: Add MAX_MEMORY_EVENTS limit

5. **Lock Contention (MEDIUM)**
   - Multiple simultaneous locks increase contention (firecracker.rs:481)
   - Deadlock risk with mixed tokio::Mutex and parking_lot::RwLock
   - De-risk: Document lock ordering, consider single RwLock

**Unknowns:**
- Firecracker-rust PRs #14-19 actual status
- Performance characteristics under load
- Integration behavior with actual Firecracker VMs

**Assumptions:**
- Firecracker-rust PRs #14-19 will be merged (trait, pre-warmed pool, OverlayFS, logging, LLM bridge, streaming)
- fcctl-core API matches expected interface
- MCP tools will be used with proper authentication

## 6. Context Complexity vs. Simplicity Opportunities

**Complexity Sources:**
1. External dependency coupling - fcctl-core unavailable in CI
2. Mixed concurrency primitives - tokio::Mutex + parking_lot::RwLock
3. Feature flags - mcp, kg-validation, llm-bridge, docker-backend, e2b-backend
4. Security surface area - arbitrary code execution in VMs
5. Integration testing gaps - no actual VM tests

**Simplification Strategies:**

1. **Dependency Abstraction**
   - Create trait-based abstraction for VmManager/SnapshotManager
   - Provide mock implementation for CI/testing
   - Gate fcctl-core behind "firecracker" feature flag

2. **Concurrency Unification**
   - Standardize on single lock type per component
   - Document clear lock ordering hierarchy
   - Use structured concurrency patterns

3. **Security Hardening**
   - Centralize all input validation
   - Add size limits and sanitization at API boundaries
   - Make KG validation mandatory, not optional

4. **Testing Strategy**
   - Unit tests with mocks (current - 108 tests)
   - Integration tests gated by environment variable
   - End-to-end tests with actual VMs (manual/periodic)

## 7. Questions for Human Reviewer

1. **Firecracker-rust Status**: What is the actual status of PRs #14-19 in firecracker-rust? Should we wait for merge or proceed with abstraction?

2. **CI Strategy**: Should we exclude terraphim_rlm from workspace permanently, or create a mock fcctl-core for CI?

3. **Security Boundaries**: Should KG validation be mandatory for all rlm_code/rlm_bash operations, or configurable per-deployment?

4. **Resource Limits**: What are appropriate limits for MAX_CODE_SIZE (1MB?), MAX_MEMORY_EVENTS (10,000?), query loop timeout (5 minutes?)?

5. **Error Handling**: Should we use thiserror with `#[source]` throughout, or are there cases where silent error handling is acceptable?

6. **Integration Testing**: Can we set up a VM-based integration test environment on bigbox, or should we rely on manual testing?

7. **Lock Ordering**: What is the preferred lock ordering when multiple locks are needed (SessionManager vs VmManager)?

8. **Feature Gates**: Which features should be in "full" feature set vs optional? Should mcp be default-enabled?

9. **Snapshot Limits**: What should max_snapshots_per_session be (10? 100?)? Should this be configurable?

10. **Parser Limits**: Should parser enforce 10KB limit and recursion depth, or is this the responsibility of callers?
