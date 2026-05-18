# Summary: terraphim_rlm/src/lib.rs

**Purpose:** Recursive Language Model (RLM) orchestration with isolated Firecracker VM execution, sub-500ms allocation, and knowledge graph validation.

**Architecture:**
```
TerraphimRlm (public API)
    ├── SessionManager (VM affinity, context, snapshots, extensions)
    ├── QueryLoop (command parsing, execution, result handling)
    ├── BudgetTracker (token counting, time tracking, depth limits)
    └── KnowledgeGraphValidator (term matching, retry, strictness)

ExecutionEnvironment trait
    ├── FirecrackerExecutor (primary, full isolation)
    ├── DockerExecutor (fallback, gVisor/runc)
    └── E2bExecutor (cloud option)
```

**Core Modules:**
- `config`: BackendType, KgStrictness, RlmConfig, SessionModel
- `executor`: Execution environment abstraction
- `budget`: BudgetTracker for token/time limits
- `session`: Session management with VM affinity
- `llm_bridge`: LLM invocation from within VMs
- `parser`: Command parsing
- `query_loop`: Query execution loop
- `rlm`: Main RLM orchestration
- `logger`: Trajectory logging

**Key Types:**
- `RlmConfig`: Configuration with timeout, model, strictness settings
- `RlmError`: Error types
- `ExecutionResult`: Command/code execution result
- `SnapshotId`: VM snapshot identifier
- `ValidationResult`: KG validation result
- `QueryRequest`, `QueryResponse`: LLM bridge types
- `TrajectoryEvent`: Logging event

**Features:**
- Firecracker microVM isolation with sub-500ms boot
- Dual budget system (tokens + time)
- VM snapshots and resume
- SSH executor for remote execution
- Knowledge graph command validation
- Trajectory logging for audit

**Cargo Features:**
- `kg-validation`: Enable KG validation
- `mcp`: MCP tools support