# Findings: terraphim_rlm Validation

## Examples Found

### Documentation Examples (in rustdoc)

#### From lib.rs:
1. **Basic usage example** (lines 29-45):
   - Create RlmConfig
   - Create TerraphimRlm instance
   - Create session
   - Execute Python code
   - Execute bash command

#### From rlm.rs:
2. **Main RLM example** (lines 9-31):
   - Create TerraphimRlm with default config
   - Create session
   - Execute Python code
   - Execute full query with RLM loop
   - Create snapshot
   - Destroy session

3. **new() constructor example** (lines 86-90):
   - Simple creation example

4. **execute_code() example** (lines 276-282):
   - Execute Python code with math module
   - Assert on output

5. **execute_command() example** (lines 326-329):
   - Execute bash command
   - Print output

6. **query() example** (lines 381-392):
   - Execute full RLM query
   - Match on termination reason
   - Handle different termination cases

### Test Examples (in rlm.rs tests module)

7. **test_rlm_with_mock_executor** (line 873):
   - Test creation with mock executor

8. **test_session_lifecycle** (line 881):
   - Create session
   - Get session
   - Set/get context variable
   - Destroy session

9. **test_execute_code** (line 905):
   - Execute code with mock
   - Assert on result

10. **test_execute_command** (line 920):
    - Execute command with mock
    - Assert on result

11. **test_snapshots** (line 932):
    - Create snapshot
    - List snapshots

12. **test_session_extension** (line 951):
    - Extend session
    - Check expiry and extension count

13. **test_version** (line 963):
    - Check version string

### MCP Tools Examples (from mcp_tools.rs)

14. **rlm_code tool** - Execute Python code in isolated VM
15. **rlm_bash tool** - Execute bash commands in isolated VM
16. **rlm_query tool** - Query LLM recursively
17. **rlm_context tool** - Get/set context variables
18. **rlm_snapshot tool** - Create/restore snapshots
19. **rlm_status tool** - Get session status

## System Elements

### Core Modules
- `lib.rs` - Public API exports
- `rlm.rs` - Main TerraphimRlm orchestrator
- `session.rs` - Session management
- `budget.rs` - Token and time budget tracking
- `config.rs` - Configuration (RlmConfig)
- `types.rs` - Shared types (SessionId, ExecutionResult, etc.)
- `error.rs` - Error types

### Execution Environment
- `executor/mod.rs` - Module definition
- `executor/trait.rs` - ExecutionEnvironment trait
- `executor/firecracker.rs` - Firecracker VM executor
- `executor/ssh.rs` - SSH executor
- `executor/context.rs` - Execution context

### Query Processing
- `parser.rs` - Command parsing
- `query_loop.rs` - Main query loop orchestration
- `llm_bridge.rs` - LLM bridge for VM-to-host calls

### Additional Features
- `logger.rs` - Trajectory logging
- `validator.rs` - Knowledge graph validation (feature-gated)
- `mcp_tools.rs` - MCP tool implementations (feature-gated)

## Features
- `full` - All features enabled (default)
- `llm` - LLM service integration
- `kg-validation` - Knowledge graph validation
- `supervision` - Agent supervision
- `llm-bridge` - HTTP bridge for LLM calls
- `docker-backend` - Docker execution backend
- `e2b-backend` - E2B cloud backend
- `dns-security` - DNS security features
- `mcp` - MCP tool support
