---
name: terraphim-rlm
description: |
  Recursive Language Model (RLM) orchestration for Terraphim AI. Provides isolated code execution
  in Firecracker VMs, session management, budget tracking, and knowledge graph validation.
  Use when: (1) Executing LLM-generated code in sandboxed environments, (2) Running untrusted
  code with resource limits, (3) Managing long-running code execution sessions with snapshots,
  (4) Building AI agents that write and execute code. Triggers: "execute code", "run python",
  "sandbox", "code isolation", "RLM", "firecracker", "session management", "budget tracking".
---

# Terraphim RLM

Recursive Language Model orchestration with sandboxed code execution.

## Core Concepts

- **TerraphimRlm**: Main entry point - manages sessions, execution, and budget tracking
- **ExecutionEnvironment**: Trait for different backends (Firecracker, Docker, Local)
- **Session**: Isolated execution context with VM affinity, snapshots, and budget
- **BudgetTracker**: Dual budget (tokens + time) prevents runaway execution

## Quick Start

```rust
use terraphim_rlm::{TerraphimRlm, RlmConfig};

let config = RlmConfig::default();
let rlm = TerraphimRlm::new(config).await?;

let session = rlm.create_session().await?;
let result = rlm.execute_code(&session.id, "print('Hello, RLM!')").await?;
let result = rlm.execute_command(&session.id, "ls -la").await?;
```

## Execution Backends

| Backend | Isolation | Use Case |
|---------|-----------|----------|
| Firecracker | Full VM | Production, untrusted code |
| Docker | Container (gVisor) | Containerized workloads |
| Local | None | Development, trusted code |

LocalExecutor selected when: no KVM, no Docker, or explicitly configured.

## Session Management

```rust
// Create session
let session = rlm.create_session().await?;

// Set context variables (accessible via FINAL_VAR in code)
rlm.set_context_variable(&session.id, "MY_VAR", "value")?;

// Get context
let value = rlm.get_context_variable(&session.id, "MY_VAR")?;

// Extend session duration
let extended = rlm.extend_session(&session.id)?;

// Destroy session
rlm.destroy_session(&session.id).await?;
```

## Code Execution

```rust
// Execute Python
let result = rlm.execute_code(&session.id, "print('hello')").await?;

// Execute bash command
let result = rlm.execute_command(&session.id, "echo $HOME").await?;

// Full RLM query loop (LLM → parse → execute → feedback)
let query_result = rlm.query(&session.id, "Calculate fibonacci").await?;
```

## Snapshots

```rust
// Create snapshot (rollback point)
let snapshot = rlm.create_snapshot(&session.id, "checkpoint_1").await?;

// Restore to snapshot
rlm.restore_snapshot(&session.id, "checkpoint_1").await?;

// List snapshots
let snapshots = rlm.list_snapshots(&session.id).await?;
```

## Configuration

```rust
let config = RlmConfig::minimal(); // For testing with mock executor

// Or with custom settings
let config = RlmConfig {
    token_budget: 100_000,
    time_budget_ms: 300_000,
    max_recursion_depth: 10,
    backend_preference: vec![BackendType::Local], // Force local
    ..Default::default()
};
```

## MCP Tools (with `mcp` feature)

When `terraphim_rlm` built with `mcp` feature, exposes MCP tools:
- `rlm_bash` - Execute bash commands
- `rlm_code` - Execute Python code
- `rlm_context` - Get/set session context
- `rlm_snapshot` - Snapshot/restore sessions
- `rlm_query` - Full RLM query loop

## Key Files

- `crates/terraphim_rlm/src/rlm.rs` - Main TerraphimRlm API
- `crates/terraphim_rlm/src/executor/local.rs` - LocalExecutor (no isolation)
- `crates/terraphim_rlm/src/executor/firecracker.rs` - FirecrackerExecutor (full VM)
- `crates/terraphim_rlm/src/session.rs` - SessionManager
- `crates/terraphim_rlm/src/budget.rs` - BudgetTracker