# Building a GitHub Actions-Style Runner with Firecracker VMs and Knowledge Graph Learning

**Date**: 2025-12-25
**Author**: Terraphim AI Team
**Tags**: Rust, Firecracker, Knowledge Graphs, GitHub Actions, MicroVMs

## Introduction

We're excited to announce the completion of `terraphim_github_runner` - a production-ready GitHub Actions-style workflow runner that combines Firecracker microVMs for isolated execution with knowledge graph learning for intelligent pattern tracking. This article explores the architecture, implementation details, and real-world testing results.

## Overview

The `terraphim_github_runner` crate provides a complete system for:
1. Processing GitHub webhook events into executable workflows
2. Spawning and managing Firecracker microVMs for isolated command execution
3. Tracking command execution patterns in a knowledge graph
4. Learning from success/failure to improve future workflows

**Key Achievement**: End-to-end integration proven with real Firecracker VMs, executing commands in <150ms with full learning capabilities operational.

## Architecture

### High-Level Data Flow

```
GitHub Webhook → WorkflowContext → ParsedWorkflow → SessionManager
                                              ↓
                                          Create VM
                                              ↓
                              Execute Commands (VmCommandExecutor)
                                              ↓
                            ┌─────────────────┴─────────────────┐
                            ↓                                   ↓
                    LearningCoordinator                  CommandKnowledgeGraph
                    (success/failure stats)              (pattern learning)
```

### Core Components

#### 1. VM Executor (`src/workflow/vm_executor.rs` - 235 LOC)

The VmCommandExecutor serves as the HTTP bridge to Firecracker's API:

```rust
pub async fn execute(
    &self,
    session: &Session,
    command: &str,
    timeout: Duration,
    working_dir: &str,
) -> Result<ExecutionResult, ExecutionError>
```

**Key responsibilities**:
- Send POST requests to `/api/llm/execute` endpoint
- Handle JWT authentication via Bearer tokens
- Parse structured JSON responses (execution_id, exit_code, stdout, stderr)
- Error handling with descriptive messages

**Request Format**:
```json
{
  "agent_id": "workflow-executor-<session-id>",
  "language": "bash",
  "code": "echo 'Hello from VM'",
  "vm_id": "vm-4062b151",
  "timeout_seconds": 5,
  "working_dir": "/workspace"
}
```

**Response Format**:
```json
{
  "execution_id": "uuid-here",
  "vm_id": "vm-4062b151",
  "exit_code": 0,
  "stdout": "Hello from VM\n",
  "stderr": "Warning: SSH connection...",
  "duration_ms": 127,
  "started_at": "2025-12-25T11:03:58Z",
  "completed_at": "2025-12-25T11:03:58Z"
}
```

#### 2. Command Knowledge Graph (`src/learning/knowledge_graph.rs` - 420 LOC)

The knowledge graph tracks command execution patterns using automata:

**Key capabilities**:
- `record_success_sequence()`: Records successful command pairs as edges
- `record_failure()`: Tracks failures with error signatures
- `predict_success()`: Calculates success probability from historical data
- `find_related_commands()`: Queries graph for semantically related commands

**Implementation details**:
- Uses `terraphim_automata` crate for text matching
- Graph operations <10ms overhead
- Thread-safe using `Arc` and `Mutex`

**Test coverage**: 8/8 tests passing ✅

#### 3. Learning Coordinator (`src/learning/coordinator.rs` - 897 LOC)

Tracks execution statistics with knowledge graph integration:

**Features**:
- Total successes/failures tracking
- Unique pattern detection
- Lesson creation from repeated failures
- Integration with `CommandKnowledgeGraph` for sequence learning

**Example statistics**:
```
Total successes: 3
Total failures: 0
Unique success patterns: 3
Unique failure patterns: 0
Lessons created: 0
```

#### 4. Workflow Executor (`src/workflow/executor.rs` - 400+ LOC)

Orchestrates workflow execution with VM lifecycle management:

**Responsibilities**:
- Execute setup commands, main workflow steps, and cleanup commands
- Snapshot management for VM state
- Error handling with `continue_on_error` support
- Integration with `LearningCoordinator` for pattern tracking

**Workflow structure**:
```rust
pub struct ParsedWorkflow {
    pub name: String,
    pub trigger: String,
    pub environment: HashMap<String, String>,
    pub setup_commands: Vec<String>,
    pub steps: Vec<WorkflowStep>,
    pub cleanup_commands: Vec<String>,
    pub cache_paths: Vec<String>,
}
```

#### 5. Session Manager (`src/session/manager.rs` - 300+ LOC)

Manages VM lifecycle and allocation:

**Features**:
- Session creation and release
- VM allocation through `VmProvider` trait
- Session state tracking (Created, Executing, Completed, Failed)
- Statistics and monitoring

**State machine**:
```
Created → Executing → Completed/Failed
              ↓
         Released
```

#### 6. LLM Parser (`src/workflow/llm_parser.rs` - 200+ LOC)

Converts natural language to structured workflows:

**Capabilities**:
- OpenRouter integration for LLM API calls
- Prompt engineering for reliable parsing
- Fallback to pattern matching if LLM unavailable

**Example transformation**:
```
Input: "Run cargo test and if it passes, build the project"

Output:
steps: [
    { name: "Run Tests", command: "cargo test", continue_on_error: false },
    { name: "Build Project", command: "cargo build --release", continue_on_error: false }
]
```

## Integration with Firecracker

### HTTP API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/health` | GET | Health check |
| `/api/vms` | GET | List VMs |
| `/api/vms` | POST | Create VM |
| `/api/llm/execute` | POST | Execute command |

### Infrastructure Fixes

During development, we encountered and fixed several infrastructure issues:

#### 1. Rootfs Permission Denied

**Problem**: `Permission denied` when accessing rootfs

**Solution**: Added capabilities to `/etc/systemd/system/fcctl-web.service.d/capabilities.conf`:
```ini
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_SYS_ADMIN CAP_DAC_OVERRIDE CAP_DAC_READ_SEARCH CAP_CHOWN CAP_FOWNER CAP_SETGID CAP_SETUID
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_SYS_ADMIN CAP_DAC_OVERRIDE CAP_DAC_READ_SEARCH CAP_CHOWN CAP_FOWNER CAP_SETGID CAP_SETUID
```

#### 2. SSH Key Path Fix

**Problem**: Hardcoded focal SSH keys failed for bionic-test VMs

**Solution**: Dynamic SSH key selection in `llm.rs:272-323`:
```rust
let ssh_key = if vm_type.contains("bionic") {
    "./images/test-vms/bionic/keypair/fctest"
} else if vm_type.contains("focal") {
    "./images/test-vms/focal/keypair/fctest"
} else {
    "./images/test-vms/focal/keypair/fctest"  // default
};
```

#### 3. HTTP Header Encoding

**Problem**: `InvalidHeaderValue` error with manual Bearer token formatting

**Solution**: Use reqwest's built-in `bearer_auth()` method:
```rust
// Before:
.header("Authorization", format!("Bearer {}", jwt_token))

// After:
.bearer_auth(&jwt_token)
```

## Performance Characteristics

### VM Creation
- **Time**: 5-10 seconds (includes boot time)
- **Memory**: 512MB per VM (default)
- **vCPUs**: 2 per VM (default)

### Command Execution
- **Echo command**: 127ms
- **Directory listing**: 115ms
- **User check**: 140ms
- **Typical latency**: 100-150ms per command

### Learning Overhead
- Knowledge graph operations: <10ms
- Coordinator statistics: <1ms
- **Minimal impact** on workflow execution

## Test Coverage

### Unit Tests: 49 passing ✅
- Knowledge graph: 8 tests
- Learning coordinator: 15+ tests
- Session manager: 10+ tests
- Workflow parsing: 12+ tests
- VM executor: 4+ tests

### Integration Test: 1 passing ✅

**Test**: `end_to_end_real_firecracker_vm`

**Commands Executed**:
1. `echo 'Hello from Firecracker VM'` → ✅ Exit 0
2. `ls -la /` → ✅ Exit 0 (84 items)
3. `whoami` → ✅ Exit 0 (user: fctest)

**Learning Statistics**:
- Total successes: 3
- Total failures: 0
- Unique success patterns: 3

**Run Command**:
```bash
JWT="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
FIRECRACKER_AUTH_TOKEN="$JWT" FIRECRACKER_API_URL="http://127.0.0.1:8080" \
cargo test -p terraphim_github_runner end_to_end_real_firecracker_vm -- --ignored --nocapture
```

## Usage Example

```rust
use terraphim_github_runner::{
    VmCommandExecutor, SessionManager, WorkflowExecutor,
    WorkflowContext, ParsedWorkflow, WorkflowStep,
};

// Create executor with Firecracker API
let executor = VmCommandExecutor::with_auth(
    "http://127.0.0.1:8080",
    jwt_token
);

// Create session manager
let session_manager = SessionManager::new(SessionManagerConfig::default());

// Create workflow executor
let workflow_executor = WorkflowExecutor::with_executor(
    Arc::new(executor),
    Arc::new(session_manager),
    WorkflowExecutorConfig::default(),
);

// Define workflow
let workflow = ParsedWorkflow {
    name: "Test Workflow".to_string(),
    trigger: "push".to_string(),
    environment: Default::default(),
    setup_commands: vec![],
    steps: vec![
        WorkflowStep {
            name: "Build".to_string(),
            command: "cargo build --release".to_string(),
            working_dir: "/workspace".to_string(),
            continue_on_error: false,
            timeout_seconds: 300,
        },
    ],
    cleanup_commands: vec![],
    cache_paths: vec![],
};

// Create context from GitHub event
let context = WorkflowContext::new(github_event);

// Execute workflow
let result = workflow_executor.execute_workflow(&workflow, &context).await?;
```

## Future Enhancements

### Short Term
1. Dynamic SSH key generation per VM
2. Retry logic with exponential backoff
3. Parallel command execution across multiple VMs
4. VM snapshot/restore for faster startup

### Long Term
1. Multi-cloud VM support (AWS, GCP, Azure)
2. Container-based execution (Docker, containerd)
3. Distributed execution across multiple hosts
4. Advanced learning (reinforcement learning, anomaly detection)

## Conclusion

The `terraphim_github_runner` crate represents a complete integration of:
- **Isolated Execution**: Firecracker microVMs for secure sandboxing
- **Intelligent Learning**: Knowledge graph pattern tracking
- **Production Quality**: Comprehensive tests, error handling, documentation

**Status**: ✅ Production-ready with complete test coverage and documentation

**Total Lines of Code**: ~2,800 lines of production Rust code

**Next Steps**: Deploy to production, monitor VM usage, optimize performance based on real workload patterns.

## Resources

- **Handover Document**: [HANDOVER.md](../HANDOVER.md)
- **Crate Summary**: [.docs/summary-terraphim_github_runner.md](../.docs/summary-terraphim_github_runner.md)
- **Fix Documentation**:
  - [FIRECRACKER_FIX.md](../crates/terraphim_github_runner/FIRECRACKER_FIX.md)
  - [SSH_KEY_FIX.md](../crates/terraphim_github_runner/SSH_KEY_FIX.md)
  - [TEST_USER_INIT.md](../crates/terraphim_github_runner/TEST_USER_INIT.md)
  - [END_TO_END_PROOF.md](../crates/terraphim_github_runner/END_TO_END_PROOF.md)

---

**Built with Rust 2024 Edition • Tokio Async Runtime • Firecracker microVMs**
