# Specification: Terraphim RLM Integration

**Version**: 1.2
**Date**: 2026-01-06
**Status**: Approved via Specification Interview (Updated with Architecture Review Findings)

---

## Executive Summary

This specification defines the integration of Recursive Language Model (RLM) patterns into terraphim-ai, creating a new `terraphim_rlm` crate that enables LLMs to execute code in isolated Firecracker VMs with knowledge graph validation and recursive self-invocation capabilities.

**Key Architecture Decision**: Bypass rig-core entirely; use terraphim_service directly for LLM operations with a new TerraphimRlm interface.

---

## 1. Core Components

### 1.1 TerraphimRlm Struct

```rust
pub struct TerraphimRlm {
    llm_client: Arc<dyn GenericLlmClient>,
    executor: Arc<FirecrackerExecutor>,
    kg_validator: Arc<KnowledgeGraphValidator>,
    supervisor: Arc<AgentSupervisor>,
    config: RlmConfig,
}
```

### 1.2 FirecrackerExecutor

Implements the async `ExecutionEnvironment` trait:

```rust
#[async_trait]
pub trait ExecutionEnvironment: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn execute_code(&self, code: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error>;
    async fn execute_command(&self, cmd: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error>;
    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error>;
    async fn create_snapshot(&self, name: &str) -> Result<SnapshotId, Self::Error>;
    async fn restore_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error>;

    fn capabilities(&self) -> &[Capability];
}
```

### 1.3 Command Types

```rust
pub enum Command {
    Run(BashCommand),      // Execute bash command in VM
    Code(PythonCode),      // Execute Python code in VM
    Final(String),         // Return final result
    QueryLlm(LlmQuery),    // Recursive LLM invocation
    Snapshot(String),      // Create named snapshot
    Rollback(SnapshotId),  // Restore to snapshot
}
```

---

## 2. Specification Interview Findings

**Interview Date**: 2026-01-06
**Dimensions Covered**: Concurrency, Failure Modes, Edge Cases, Security, Scale, Integration, Operations
**Convergence Status**: Complete

### 2.1 Concurrency & Race Conditions

| Decision | Rationale |
|----------|-----------|
| **Kill immediately on parent cancel** | When parent RLM session times out or is cancelled, all child recursive executions are terminated immediately. Cascading cancellation via tokio CancellationToken. |
| **Spawn overflow VMs on pool exhaustion** | When pre-warmed pool is exhausted, cold-boot additional VMs beyond pool limit. Clean up overflow VMs after use with lower priority. |

**Implementation Notes**:
- Use `tokio_util::sync::CancellationToken` propagated through execution tree
- Overflow VMs tracked separately with `overflow_vm_cleanup_queue`
- Pool manager signals `pool_exhausted` metric for monitoring

### 2.2 Failure Modes & Recovery

| Decision | Rationale |
|----------|-----------|
| **Checkpoint resume after crash** | Supervised RLM agents resume from last successful command in REPL loop after crash/restart. Uses Firecracker VM snapshots for state preservation. |
| **Logs only on timeout** | When execution times out, preserve stdout/stderr/exit status, destroy VM state immediately. No full snapshots for timed-out executions. |

**Implementation Notes**:
- Checkpoint stored as Firecracker VM snapshot before each command execution
- `CommandHistory` tracks successful commands for replay
- Timeout handler: extract logs → destroy VM → return structured error

### 2.3 Edge Cases & Boundaries

| Decision | Rationale |
|----------|-----------|
| **Stream large output to file** | When stdout/stderr exceeds threshold (default 64KB), stream to temp file in VM, return file path to LLM. Prevents memory exhaustion. |
| **Snapshot points for state versioning** | Users can create named snapshots. Rolling back restores full VM state including Python interpreter, context variables, and filesystem. |

**Implementation Notes**:
- Output streaming threshold configurable via `RlmConfig::max_inline_output_bytes`
- Snapshot names must be unique within session; duplicate names error
- Maximum snapshots per session: configurable, default 10

### 2.4 User Mental Model & Experience

| Decision | Rationale |
|----------|-----------|
| **Polling status for MCP progress** | MCP tool returns task ID immediately. Client polls `/tasks/{id}/status` endpoint for progress updates. Final result via `/tasks/{id}/result`. |
| **Escalate to user on KG validation failure** | After N retries (default 3), if LLM cannot rephrase using known concepts, pause and ask human user to approve or reject unknown terms. |

**Implementation Notes**:
- Task status endpoint returns: `{ status: "running"|"complete"|"failed", progress_pct: 0-100, current_step: "..." }`
- KG escalation returns structured choice: `{ unknown_terms: [...], suggested_action: "approve"|"reject", context: "..." }`

### 2.5 Scale & Performance

| Decision | Rationale |
|----------|-----------|
| **Configurable token and time budget for recursion** | Dual budget system: total LLM tokens consumed + total wall-clock time. Both configurable, either exceeded terminates recursive tree. |
| **All terraphim providers supported** | Full `GenericLlmClient` integration from day one. Supports Ollama, OpenRouter, and any future providers. |

**Implementation Notes**:
- Default budgets: 100K tokens, 5 minutes wall-clock
- Budget tracked per session, passed to recursive calls as remaining budget
- Provider selection via `RlmConfig::llm_provider` with runtime switching

### 2.6 Security & Privacy

| Decision | Rationale |
|----------|-----------|
| **Full outbound network + audit** | VMs have unrestricted outbound network access. All connections logged with timestamp, destination, bytes transferred. Audit log retained per session. |
| **Opt-in verbose tracing** | Minimal default tracing (command types, durations). User can enable full content tracing per session via flag. |

**Implementation Notes**:
- Network audit via iptables logging in VM + structured extraction
- Verbose tracing flag: `RlmConfig::enable_verbose_tracing: bool`
- Trace data subject to session TTL, auto-deleted after expiry

### 2.7 Integration Effects

| Decision | Rationale |
|----------|-----------|
| **Session affinity for MCP** | Multiple MCP tool calls within same conversation share VM and accumulated state. Session expires on conversation end or timeout. |
| **Specialized MCP tools** | Separate tools: `rlm_code`, `rlm_bash`, `rlm_query`, `rlm_context`, `rlm_snapshot`, `rlm_status`. Clear separation of concerns. |

**MCP Tool Definitions**:
```
rlm_code     - Execute Python code in session VM
rlm_bash     - Execute bash command in session VM
rlm_query    - Send query through full RLM pipeline
rlm_context  - Read/write session context variables
rlm_snapshot - Create/list/restore named snapshots
rlm_status   - Get execution status for async tasks
```

### 2.8 Migration & Compatibility

| Decision | Rationale |
|----------|-----------|
| **Bypass rig-core entirely** | Use terraphim_service directly for all LLM operations. Avoids Ollama compatibility issues with rig-core's OpenAI client. |
| **New TerraphimRlm interface** | Design fresh API optimized for terraphim patterns rather than maintaining rig-rlm compatibility. Clean break. |

**Implementation Notes**:
- No dependency on `rig-core` crate
- RLM prompt/preamble adapted from rig-rlm but customized for terraphim
- Command parsing uses `terraphim_automata` patterns

### 2.9 Operational Concerns

| Decision | Rationale |
|----------|-----------|
| **Virtual layers for pip packages** | OverlayFS for session-specific packages. pip installs persisted for session duration, discarded after. Base image unchanged. |
| **VM snapshot for checkpoints** | Firecracker snapshot captures full state including Python interpreter and filesystem. Enables exact-state resume after crash. |

**Implementation Notes**:
- OverlayFS lower layer: read-only base image
- OverlayFS upper layer: session tmpfs (cleared on session end)
- Snapshot includes overlay state

### 2.10 Architecture Review Findings (2026-01-06)

The following decisions were made based on architecture review with firecracker-rust crate analysis:

#### 2.10.1 Pool & Lifecycle Management

| Decision | Rationale |
|----------|-----------|
| **Parallel overflow spawn (limit 3)** | When pre-warmed pool exhausted, spawn up to 3 overflow VMs concurrently. Prevents thundering herd while maintaining responsiveness. |
| **Fail fast on crash** | When VM crashes or becomes unresponsive, immediately return error to client. Let client decide retry strategy. No automatic retry at executor level. |
| **Queue depth trigger for scaling** | Pool autoscaling triggered when pending request queue exceeds threshold. Scale down after configurable idle timeout. |
| **Hybrid state authority** | SessionManager owns session state (context, history, affinity). VmManager owns VM health and lifecycle. Clear separation prevents state corruption. |

**Implementation Notes**:
- Overflow VM counter: `overflow_active: AtomicU32` with max 3
- Pool scaling thresholds: `scale_up_queue_depth: 5`, `scale_down_idle_secs: 300`
- State ownership: Session → `terraphim_rlm::SessionManager`, VM → `fcctl-core::VmManager`

#### 2.10.2 Network Security

| Decision | Rationale |
|----------|-----------|
| **DNS allowlist + block suspicious** | Use DNS allowlist for known-safe domains. Block queries to non-approved domains. Log all blocked DNS attempts for forensic analysis. |
| **Session token validation for LLM bridge** | VM passes session token with each recursive LLM query. Bridge validates token against SessionManager before processing. |

**Implementation Notes**:
- DNS allowlist configurable via `RlmConfig::dns_allowlist: Vec<String>`
- Default allowlist: `["pypi.org", "github.com", "raw.githubusercontent.com"]`
- LLM bridge header: `X-Session-Token: <session_id>`
- Token validation: `session_manager.validate_session(token) -> Result<SessionInfo>`

#### 2.10.3 Session Extension & Snapshots

| Decision | Rationale |
|----------|-----------|
| **Explicit extension API + checkpoint** | Users can call `extend_session()` before timeout for fixed increment. Alternatively, create checkpoint and start new session from checkpoint for unlimited duration. |
| **Ignore external state drift** | When restoring snapshot, proceed regardless of time elapsed. Let user code handle any API/external state changes naturally. |

**Implementation Notes**:
- Extension increment: configurable, default +30 minutes
- Extension limit: configurable, default 3 extensions max
- Checkpoint-to-new-session: `create_checkpoint()` → `start_session_from_checkpoint(checkpoint_id)`

#### 2.10.4 Resource Limits

| Decision | Rationale |
|----------|-----------|
| **Dynamic disk expansion** | Start overlay at small size (100MB), expand dynamically as needed up to hard max (2GB). Avoids over-allocation while preventing surprise failures. |
| **Overlay always wins for packages** | Session-installed packages shadow base image packages unconditionally. Simplest mental model; user controls their environment. |

**Implementation Notes**:
- Initial overlay size: `RlmConfig::initial_overlay_mb: 100`
- Max overlay size: `RlmConfig::max_overlay_mb: 2048`
- Expansion step: double current size until max reached

#### 2.10.5 Operations & Monitoring

| Decision | Rationale |
|----------|-----------|
| **Alert escalation + metrics + playbook** | Auto-remediation attempts for common failures (restart stale VM, clear cache). Escalate after 3+ failures in 5 minutes. Dashboard with pool metrics. Documented runbook. |
| **Configurable KG strictness** | User sets validation strictness: `permissive` (warn only), `normal` (retry 3x then warn), `strict` (block unknown terms). |

**Implementation Notes**:
- Auto-remediation actions: `restart_vm`, `clear_overlay_cache`, `drain_and_refill_pool`
- Alert channels: configurable via `RlmConfig::alert_webhook_url`
- KG strictness levels enum: `KgStrictness::Permissive | Normal | Strict`

#### 2.10.6 Integration & Backend Selection

| Decision | Rationale |
|----------|-----------|
| **Wrap errors as MCP errors** | Convert all execution errors to MCP error format with error code and descriptive message. Clean interface for MCP clients. |
| **No cross-session resource sharing** | Each session is fully isolated. No shared volumes, no shared state. Simplifies security model and debugging. |
| **Preference + fallback backend selection** | User specifies preferred backend AND fallback order. System uses first available in order. Explicit control over execution environment. |
| **Configurable per-backend session model** | Firecracker uses session affinity (stateful VMs). E2B uses stateless model (new sandbox per call). Docker configurable either way. |

**Implementation Notes**:
- Backend preference config: `RlmConfig::backend_preference: Vec<BackendType>`
- Default order: `[Firecracker, E2B, Docker]`
- MCP error format: `{ code: i32, message: String, data: Option<Value> }`
- Per-backend session config: `BackendConfig::session_model: SessionModel::Affinity | Stateless`

---

## 3. Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         MCP Server (terraphim_mcp_server)               │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┬───────────┐  │
│  │ rlm_code    │ rlm_bash    │ rlm_query   │ rlm_context │rlm_status │  │
│  └──────┬──────┴──────┬──────┴──────┬──────┴──────┬──────┴─────┬─────┘  │
└─────────┼─────────────┼─────────────┼─────────────┼────────────┼────────┘
          │             │             │             │            │
          ▼             ▼             ▼             ▼            ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         TerraphimRlm (terraphim_rlm)                    │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────────────┐     │
│  │  Query Loop      │  │  KG Validator    │  │  Budget Tracker    │     │
│  │  - Parse command │  │  - terraphim_    │  │  - Token count     │     │
│  │  - Execute       │  │    automata      │  │  - Time elapsed    │     │
│  │  - Handle result │  │  - Escalation    │  │  - Recursion depth │     │
│  └────────┬─────────┘  └────────┬─────────┘  └──────────┬─────────┘     │
│           │                     │                       │               │
│           ▼                     ▼                       ▼               │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    Session Manager                              │    │
│  │  - Session affinity (conversation → VM mapping)                 │    │
│  │  - Snapshot management (create/restore/list)                    │    │
│  │  - Context state (variables, my_answer)                         │    │
│  └──────────────────────────────┬──────────────────────────────────┘    │
└─────────────────────────────────┼───────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    FirecrackerExecutor (terraphim_firecracker)          │
│  ┌────────────────┐  ┌────────────────┐  ┌─────────────────────────┐    │
│  │ VmPoolManager  │  │ Network Audit  │  │ OverlayFS Manager       │    │
│  │ - Pre-warmed   │  │ - iptables log │  │ - Base image (ro)       │    │
│  │ - Overflow     │  │ - Structured   │  │ - Session layer (rw)    │    │
│  │ - Allocation   │  │   extraction   │  │ - pip packages          │    │
│  └───────┬────────┘  └───────┬────────┘  └───────────┬─────────────┘    │
│          │                   │                       │                  │
│          ▼                   ▼                       ▼                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    Firecracker VM Instance                      │    │
│  │  - Python 3.11 + common packages (pre-baked)                    │    │
│  │  - query_llm() → HTTP to host terraphim_service                 │    │
│  │  - context variable + my_answer return value                    │    │
│  └─────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    AgentSupervisor (terraphim_agent_supervisor)         │
│  - Lifecycle: init → start → health_check → stop                        │
│  - Crash recovery: restore from VM snapshot checkpoint                  │
│  - Restart strategy: OneForOne with backoff                             │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    LLM Service (terraphim_service)                      │
│  - GenericLlmClient: Ollama, OpenRouter, future providers               │
│  - Token counting and cost tracking                                     │
│  - Context management with history                                      │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Configuration Schema

```rust
pub struct RlmConfig {
    // VM Pool
    pub pool_min_size: usize,           // Default: 2
    pub pool_max_size: usize,           // Default: 10
    pub pool_target_size: usize,        // Default: 4
    pub vm_boot_timeout_ms: u64,        // Default: 2000
    pub allocation_timeout_ms: u64,     // Default: 500
    pub max_overflow_vms: u32,          // Default: 3 (concurrent overflow limit)
    pub scale_up_queue_depth: u32,      // Default: 5
    pub scale_down_idle_secs: u64,      // Default: 300

    // Budgets
    pub max_recursion_depth: u32,       // Default: 5
    pub max_total_tokens: u64,          // Default: 100_000
    pub max_wall_clock_ms: u64,         // Default: 300_000 (5 min)

    // KG Validation
    pub kg_validation_enabled: bool,    // Default: true
    pub kg_retry_limit: u32,            // Default: 3
    pub kg_escalation_enabled: bool,    // Default: true
    pub kg_strictness: KgStrictness,    // Default: Normal

    // Output
    pub max_inline_output_bytes: usize, // Default: 65_536 (64KB)
    pub max_snapshots_per_session: u32, // Default: 10

    // Session
    pub session_timeout_ms: u64,        // Default: 3_600_000 (1 hour)
    pub session_extension_ms: u64,      // Default: 1_800_000 (30 min)
    pub max_extensions: u32,            // Default: 3
    pub checkpoint_enabled: bool,       // Default: true

    // OverlayFS / Disk
    pub initial_overlay_mb: u32,        // Default: 100
    pub max_overlay_mb: u32,            // Default: 2048

    // Network Security
    pub dns_allowlist: Vec<String>,     // Default: ["pypi.org", "github.com", ...]
    pub enable_network_audit: bool,     // Default: true

    // Tracing
    pub enable_verbose_tracing: bool,   // Default: false
    pub trace_retention_days: u32,      // Default: 7

    // Operations
    pub alert_webhook_url: Option<String>, // Default: None
    pub auto_remediation_enabled: bool, // Default: true

    // Backend Selection
    pub backend_preference: Vec<BackendType>, // Default: [Firecracker, E2B, Docker]

    // LLM
    pub llm_provider: String,           // Default: "ollama"
    pub llm_model: String,              // Default: "llama3.2:3b"
}

#[derive(Clone, Copy, Debug, Default)]
pub enum KgStrictness {
    Permissive,  // Warn only, never block
    #[default]
    Normal,      // Retry 3x, then warn
    Strict,      // Block unknown terms immediately
}

#[derive(Clone, Copy, Debug)]
pub enum BackendType {
    Firecracker,
    E2B,
    Docker,
}

#[derive(Clone, Copy, Debug)]
pub enum SessionModel {
    Affinity,   // Stateful - same VM for session
    Stateless,  // New sandbox per call
}
```

---

## 5. API Contracts

### 5.1 Public API

```rust
impl TerraphimRlm {
    /// Create new RLM instance with configuration
    pub async fn new(config: RlmConfig) -> Result<Self, RlmError>;

    /// Execute a full query through the RLM pipeline
    pub async fn query(&self, input: &str, session_id: Option<SessionId>) -> Result<RlmResponse, RlmError>;

    /// Execute code directly in session VM
    pub async fn execute_code(&self, code: &str, session_id: SessionId) -> Result<ExecutionResult, RlmError>;

    /// Execute bash command in session VM
    pub async fn execute_command(&self, cmd: &str, session_id: SessionId) -> Result<ExecutionResult, RlmError>;

    /// Get/set context variables
    pub async fn get_context(&self, session_id: SessionId, key: &str) -> Result<Option<String>, RlmError>;
    pub async fn set_context(&self, session_id: SessionId, key: &str, value: &str) -> Result<(), RlmError>;

    /// Snapshot management
    pub async fn create_snapshot(&self, session_id: SessionId, name: &str) -> Result<SnapshotId, RlmError>;
    pub async fn restore_snapshot(&self, session_id: SessionId, snapshot_id: &SnapshotId) -> Result<(), RlmError>;
    pub async fn list_snapshots(&self, session_id: SessionId) -> Result<Vec<SnapshotInfo>, RlmError>;

    /// Session management
    pub async fn create_session(&self) -> Result<SessionId, RlmError>;
    pub async fn destroy_session(&self, session_id: SessionId) -> Result<(), RlmError>;
    pub async fn get_session_status(&self, session_id: SessionId) -> Result<SessionStatus, RlmError>;
}
```

### 5.2 Error Types

```rust
pub enum RlmError {
    // Execution errors
    ExecutionTimeout { elapsed_ms: u64, limit_ms: u64 },
    ExecutionFailed { exit_code: i32, stderr: String },

    // Budget errors
    TokenBudgetExceeded { used: u64, limit: u64 },
    TimeBudgetExceeded { elapsed_ms: u64, limit_ms: u64 },
    RecursionLimitExceeded { depth: u32, limit: u32 },

    // KG validation errors
    KgValidationFailed { unknown_terms: Vec<String>, attempts: u32 },
    KgEscalationRequired { unknown_terms: Vec<String>, context: String },

    // VM errors
    VmAllocationFailed { reason: String },
    VmCommunicationError { details: String },

    // Session errors
    SessionNotFound { session_id: SessionId },
    SessionExpired { session_id: SessionId },
    SnapshotNotFound { snapshot_id: SnapshotId },
    SnapshotLimitExceeded { current: u32, limit: u32 },

    // LLM errors
    LlmProviderError { provider: String, message: String },

    // Internal errors
    InternalError { message: String },
}
```

---

## 6. Acceptance Criteria

### 6.1 Functional Requirements

| ID | Requirement | Acceptance Test |
|----|-------------|-----------------|
| F1 | Execute Python code in Firecracker VM | `execute_code("print('hello')")` returns `"hello\n"` |
| F2 | Execute bash commands in VM | `execute_command("ls /")` returns directory listing |
| F3 | Recursive LLM calls work | Code with `query_llm()` successfully invokes parent LLM |
| F4 | KG validation warns on unknown terms | Unknown term triggers warning log, execution proceeds |
| F5 | KG escalation pauses for user | After 3 retries, returns `KgEscalationRequired` error |
| F6 | Snapshots can be created and restored | Create → modify state → restore → state matches original |
| F7 | Session affinity persists state | Two sequential calls share `context` variable |
| F8 | MCP tools work via terraphim_mcp_server | All 6 tools callable via MCP protocol |

### 6.2 Performance Requirements

| ID | Requirement | Target | Measurement |
|----|-------------|--------|-------------|
| P1 | VM allocation from pool | < 500ms | 95th percentile |
| P2 | Cold VM boot | < 2000ms | 95th percentile |
| P3 | Simple code execution | < 100ms | 95th percentile (excluding boot) |
| P4 | Snapshot creation | < 1000ms | 95th percentile |
| P5 | Snapshot restore | < 500ms | 95th percentile |

### 6.3 Security Requirements

| ID | Requirement | Verification |
|----|-------------|--------------|
| S1 | VM isolation prevents host access | Penetration test: no host filesystem access |
| S2 | Network audit captures all connections | Audit log contains all `curl` requests from VM |
| S3 | Execution timeout enforced | Infinite loop terminates at timeout |
| S4 | Resource limits prevent DOS | Fork bomb contained within VM |

---

## 7. Deferred Items

| Item | Reason | Future Phase |
|------|--------|--------------|
| vsock communication | HTTP simpler for MVP; vsock optimization later | Phase 2 |
| Custom VM images per user | Requires image build pipeline | Phase 3 |
| Multi-language support (beyond Python) | Python covers 90% of use cases | Phase 3 |
| Distributed VM pool | Single-node sufficient for initial scale | Phase 4 |

---

## 8. Interview Summary

The specification interview surfaced several critical design decisions that significantly shape the implementation:

### 8.1 Original Interview Findings (v1.0)

1. **Clean break from rig-core**: Rather than working around Ollama compatibility issues, we bypass rig-core entirely and use terraphim_service's GenericLlmClient. This simplifies the architecture and leverages existing battle-tested code.

2. **Dual budget system**: Token and time budgets provide defense-in-depth against runaway recursive calls. Both are configurable and either limit terminates the execution tree.

3. **Interactive KG validation**: The escalate-to-user pattern for unknown terms balances security (validation) with usability (not blocking legitimate but novel requests). This creates a natural feedback loop for improving the knowledge graph.

4. **VM snapshots for everything**: Firecracker's snapshotting capability is leveraged for both user-facing features (named snapshots) and internal reliability (checkpoint resume). This simplifies state management significantly.

5. **Session affinity with MCP**: Treating MCP conversations as sessions with persistent VMs enables stateful interactions that match user mental models of "working with an assistant" rather than "making isolated API calls".

### 8.2 Architecture Review Interview Findings (v1.2)

The architecture review of firecracker-rust (terraphim/firecracker-rust) and subsequent interview surfaced additional critical decisions:

6. **Fail-fast error handling**: The executor layer does not retry - it fails fast and returns errors to the client. This keeps the executor simple and gives clients control over retry policies.

7. **Hybrid state ownership**: Clear separation between SessionManager (owns session state, context, history) and VmManager (owns VM health, lifecycle). Prevents state corruption and simplifies debugging.

8. **DNS security with allowlist**: Rather than full outbound blocking, use DNS allowlist with logging of blocked attempts. Balances security (prevents exfiltration) with usability (common package sources work).

9. **Dynamic disk expansion**: OverlayFS starts small and grows on demand. Avoids over-allocation (cost) while preventing surprise failures (usability).

10. **Preference + fallback backend selection**: Users specify both preferred backend AND fallback order. Provides control while maintaining graceful degradation.

11. **Configurable KG strictness**: Three levels (permissive/normal/strict) let users choose their security/usability tradeoff based on context.

12. **No cross-session sharing**: Full session isolation simplifies security model and debugging. Shared resources handled via external stores (S3) if needed.

### 8.3 Key GitHub Issues Created

Architecture review identified 6 features needed in firecracker-rust:
- #14: ExecutionEnvironment trait abstraction
- #15: Pre-warmed VM pool with sub-500ms allocation
- #16: OverlayFS support for session packages
- #17: Network audit logging
- #18: VM-to-host LLM bridge HTTP endpoint
- #19: Output streaming to file for large outputs

The specification is now comprehensive enough to proceed to Phase 3 (disciplined-implementation) with minimal ambiguity.
