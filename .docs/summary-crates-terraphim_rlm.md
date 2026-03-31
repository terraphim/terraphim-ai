# Summary: terraphim_rlm Crate

## File
`crates/terraphim_rlm/`

## Purpose
Production-ready Recursive Language Model (RLM) orchestration for Terraphim AI. Provides sandboxed code execution via Firecracker VMs with Model Context Protocol (MCP) integration.

## Key Components

### Core Modules
- **lib.rs**: Public API exports and crate documentation
- **rlm.rs**: Main TerraphimRlm struct with session management and code execution
- **config.rs**: RlmConfig with VM pool settings, budget limits, and backend preferences
- **session.rs**: SessionManager for VM affinity, context, snapshots, and extensions
- **budget.rs**: BudgetTracker for token and time budget enforcement
- **query_loop.rs**: QueryLoop for command parsing, execution, and result handling
- **parser.rs**: CommandParser for parsing natural language commands
- **validation.rs**: Input validation (code size, session IDs, snapshot names)
- **mcp_tools.rs**: RlmMcpService with 6 MCP tools for AI integration
- **logger.rs**: TrajectoryLogger for execution history

### Execution Environments
- **executor/mod.rs**: Backend selection logic (KVM/Docker/Mock detection)
- **executor/firecracker.rs**: FirecrackerExecutor with fcctl-core integration
- **executor/mock.rs**: MockExecutor for testing without VMs
- **executor/ssh.rs**: SshExecutor for remote execution
- **executor/trait.rs**: ExecutionEnvironment trait definition
- **executor/context.rs**: ExecutionContext and ExecutionResult types
- **executor/fcctl_adapter.rs**: Adapter for fcctl-core VmManager

## Public API

### Main Types
```rust
pub struct TerraphimRlm { ... }
pub struct RlmConfig { ... }
pub struct SessionId { ... }
pub struct BudgetTracker { ... }
pub struct RlmMcpService { ... }
```

### Key Methods
```rust
impl TerraphimRlm {
    pub async fn new(config: RlmConfig) -> Result<Self, RlmError>;
    pub async fn execute_code(&self, session: &SessionId, code: &str) -> Result<ExecutionResult, RlmError>;
    pub async fn execute_command(&self, session: &SessionId, cmd: &str) -> Result<ExecutionResult, RlmError>;
    pub async fn create_snapshot(&self, session: &SessionId, name: &str) -> Result<SnapshotId, RlmError>;
}
```

## Features

### Feature Flags
- `firecracker`: Firecracker VM execution (requires KVM)
- `docker-backend`: Docker container fallback
- `e2b-backend`: E2B cloud execution
- `mcp`: Model Context Protocol tools
- `llm`: LLM service integration (enabled by default)
- `kg-validation`: Knowledge graph validation
- `supervision`: Agent supervisor

### MCP Tools (when `mcp` feature enabled)
1. `rlm_code` - Execute Python code
2. `rlm_bash` - Execute bash commands
3. `rlm_query` - Query LLM recursively
4. `rlm_context` - Get/set context
5. `rlm_snapshot` - Create/restore snapshots
6. `rlm_status` - Get session status

## Dependencies

### Required
- tokio, serde, serde_json, async-trait, thiserror, anyhow, log
- ulid (for session IDs), jiff (for timestamps)
- parking_lot, dashmap (concurrent data structures)

### Optional
- fcctl-core (Firecracker control, git dependency)
- rmcp (MCP protocol implementation)
- bollard (Docker API)
- terraphim_service, terraphim_automata, etc.

## Testing

### Test Files
- `tests/integration_test.rs`: MCP tools E2E tests (4 tests)
- `tests/code_execution_test.rs`: Code execution tests (5 tests)
- Unit tests embedded in source files (132 tests total)

### Test Commands
```bash
# All tests
cargo test -p terraphim_rlm --features firecracker,mcp

# Integration tests only
cargo test -p terraphim_rlm --features firecracker,mcp --test integration_test

# Code execution tests only
cargo test -p terraphim_rlm --features firecracker,mcp --test code_execution_test

# Unit tests only
cargo test -p terraphim_rlm --lib --features firecracker,mcp
```

## Security Features

### Input Validation
- Path traversal prevention (rejects `..`, `/`, `\` in snapshot names)
- Code size limits (1MB max via MAX_CODE_SIZE constant)
- Session ID validation (ULID format required)
- Command injection prevention

### Resource Limits
- Token budget per session (default: 100K tokens)
- Time budget per session (default: 5 minutes)
- Max recursion depth (default: 10 levels)
- Max snapshots per session (default: 10)

### Execution Isolation
- Firecracker VMs provide full kernel-level isolation
- Docker containers provide process-level isolation (optional gVisor)
- Mock executor simulates isolation for testing

## Architecture Decisions

1. **Feature-gated fcctl-core**: Firecracker integration is optional to support CI environments without KVM
2. **MCP Protocol**: Uses industry-standard Model Context Protocol for AI tool integration
3. **BudgetTracker**: Thread-safe atomic operations for budget tracking
4. **SessionManager**: Arc<RwLock<...>> for concurrent session access
5. **MockExecutor**: Provides deterministic testing without VM infrastructure

## Recent Changes (PR #426)
- Merged: March 31, 2026
- Added: Complete RLM implementation with Firecracker support
- Added: 6 MCP tools for AI integration
- Added: Budget tracking and KG validation
- Added: 144 tests (132 unit + 9 integration + 3 doc tests)
- Feature-gated: fcctl-core and terraphim-firecracker dependencies
