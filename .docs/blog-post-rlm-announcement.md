# Introducing Terraphim RLM: Production-Ready Recursive Language Models with Firecracker Isolation

**We're excited to announce the merge of PR #426, bringing production-ready Recursive Language Model (RLM) orchestration to Terraphim AI.**

After months of development and rigorous testing, we're proud to introduce a complete RLM implementation that combines the conceptual elegance of recursive LLM architectures with enterprise-grade security and isolation.

## What is a Recursive Language Model?

Recursive Language Models represent a paradigm shift in how AI agents interact with their environment. Instead of traditional tool-calling patterns, RLMs return commands that execute in a sandboxed REPL environment—and can recursively invoke sub-LLMs to solve complex problems.

Key advantages:
- **Severely constrained capabilities** via sandboxed execution (much safer!)
- **Stateful context** stored within the sandbox environment
- **Recursive composition** - each sub-LLM is itself an RLM
- **Natural reasoning** through iterative code execution

## Production-Ready Features

### 🔒 Multiple Isolation Backends

**Firecracker MicroVMs** (Primary)
- Full VM isolation with <500ms allocation time
- Pre-warmed VM pools for instant response
- Snapshot support for state versioning
- Requires: KVM, Firecracker v1.1.0+

**Docker Containers** (Fallback)
- gVisor/runsc support for enhanced isolation
- Automatic detection and fallback
- Perfect for development and CI

**Mock Executor** (Testing)
- Fast, deterministic execution for tests
- CI-friendly without VM requirements

### 🛠️ Six MCP Tools for AI Integration

Our Model Context Protocol (MCP) implementation provides 6 specialized tools:

1. **`rlm_code`** - Execute Python in isolated VM
2. **`rlm_bash`** - Execute bash commands in isolated VM
3. **`rlm_query`** - Query LLM recursively from VM context
4. **`rlm_context`** - Get/set session context and budget
5. **`rlm_snapshot`** - Create/restore VM snapshots
6. **`rlm_status`** - Get session status and history

### 💰 Dual Budget System

Prevent runaway execution with:
- **Token budget** - Maximum LLM tokens per session
- **Time budget** - Maximum wall-clock execution time
- **Recursion depth** - Maximum nested LLM calls
- **Iteration limits** - Maximum query loop iterations

### 🧠 Knowledge Graph Validation

Configurable command validation:
- **Strict mode** - Reject unknown terms
- **Normal mode** - Warn on unknown terms with suggestions
- **Permissive mode** - Log only, never block
- Automatic retry with context escalation

## Technical Architecture

```
TerraphimRlm (public API)
    ├── SessionManager (VM affinity, snapshots, extensions)
    ├── QueryLoop (command parsing, execution, result handling)
    ├── BudgetTracker (token counting, time tracking, depth limits)
    └── KnowledgeGraphValidator (term matching, retry, strictness)

ExecutionEnvironment trait
    ├── FirecrackerExecutor (primary, KVM-based VMs)
    ├── DockerExecutor (fallback, container isolation)
    └── MockExecutor (testing, CI-friendly)

MCP Integration
    └── RlmMcpService (6 tools for AI tool use)
```

## Comparison: Terraphim RLM vs rig-rlm

| Feature | rig-rlm (Reference) | Terraphim RLM |
|---------|-------------------|---------------|
| **Python Execution** | PyO3 (in-process) | Firecracker VM ✅ |
| **Bash Execution** | std::process | Firecracker VM ✅ |
| **Isolation** | None | VM-level ✅ |
| **Recursive LLM** | Basic function | MCP standard ✅ |
| **Snapshots** | ❌ Not implemented | Full lifecycle ✅ |
| **Budget Tracking** | ❌ Not implemented | Token + Time ✅ |
| **KG Validation** | ❌ Not implemented | Configurable ✅ |
| **Protocol** | Custom parsing | MCP standard ✅ |
| **Production Ready** | Demo/prototype | Enterprise-grade ✅ |

## Quick Start

### Installation

```bash
git clone https://github.com/terraphim/terraphim-ai
cd terraphim-ai
cargo build -p terraphim_rlm --features firecracker,mcp
```

### Basic Usage

```rust
use terraphim_rlm::{TerraphimRlm, RlmConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create RLM instance
    let config = RlmConfig::default();
    let rlm = TerraphimRlm::new(config).await?;

    // Create a session
    let session = rlm.create_session().await?;

    // Execute Python code
    let result = rlm.execute_code(&session, r#"
        data = [1, 2, 3, 4, 5]
        print(f"Sum: {sum(data)}")
    "#).await?;

    println!("Output: {}", result.stdout);

    // Execute bash command
    let result = rlm.execute_command(&session, "ls -la").await?;
    println!("Files: {}", result.stdout);

    // Create snapshot
    let snapshot = rlm.create_snapshot(&session, "checkpoint-1").await?;

    Ok(())
}
```

### MCP Tool Example

```rust
use terraphim_rlm::mcp_tools::RlmMcpService;

let mcp = RlmMcpService::new();
mcp.initialize(rlm).await;

// Execute code via MCP
let result = mcp.call_tool("rlm_code", Some(json!({
    "code": "print('Hello from MCP!')",
    "session_id": session_id.to_string()
}))).await?;
```

## Security Features

### Input Validation
- Path traversal prevention (rejects `..`, `/`, `\` in snapshot names)
- Code size limits (1MB max)
- Session ID format validation (ULID)
- Command injection prevention

### Resource Limits
- Memory limits per VM
- Timeout enforcement
- Parser recursion depth limits
- Max snapshots per session (configurable)

### Error Handling
- Full error context preservation with `#[source]`
- Proper error propagation via `?` operator
- No silent failures or unwrap defaults
- MCP-compatible error responses

## Testing

**144 tests passing:**
- 132 unit tests
- 9 integration tests (MCP + code execution)
- 3 documentation tests

```bash
# Run all tests
cargo test -p terraphim_rlm --features firecracker,mcp

# Run with Firecracker VMs (requires KVM)
cargo test -p terraphim_rlm --features firecracker -- --ignored

# Mock-only (CI-friendly)
cargo test -p terraphim_rlm
```

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `firecracker` | Firecracker VM execution | ❌ |
| `docker-backend` | Docker container fallback | ❌ |
| `e2b-backend` | E2B cloud execution | ❌ |
| `mcp` | Model Context Protocol tools | ❌ |
| `llm` | LLM service integration | ✅ |
| `kg-validation` | Knowledge graph validation | ❌ |
| `supervision` | Agent supervisor | ❌ |

## Roadmap

### Phase 1: Core ✅ (Complete)
- Firecracker integration
- MCP tools
- Session management
- Budget tracking

### Phase 2: Enhanced Security (Next)
- DNS security with allowlisting
- gVisor integration
- Seccomp profiles

### Phase 3: Operations
- Autoscaler for VM pools
- Prometheus metrics
- Distributed tracing

### Phase 4: Advanced Features
- Multi-region VM pools
- Persistent volumes
- Custom VM images

## Acknowledgments

This implementation was inspired by:
- [rig-rlm](https://github.com/joshua-mo-143/rig-rlm) by Joshua Mo - Reference implementation
- [Original RLM blog post](https://alexzhang13.github.io/blog/2025/rlm/) by Alex Zhang - Conceptual foundation
- Firecracker team at AWS - MicroVM technology

## Get Started Today

```bash
cargo add terraphim_rlm --features firecracker,mcp
```

Or clone the repo and explore the examples:

```bash
git clone https://github.com/terraphim/terraphim-ai
cd terraphim-ai/crates/terraphim_rlm
cargo test --features firecracker,mcp -- --nocapture
```

## Links

- **Repository**: https://github.com/terraphim/terraphim-ai
- **Documentation**: https://docs.terraphim.ai/rlm
- **Issues**: https://github.com/terraphim/terraphim-ai/issues
- **PR #426**: https://github.com/terraphim/terraphim-ai/pull/426

---

*Terraphim RLM: Where recursive AI meets production-grade isolation.*

**What's your use case for RLM? We'd love to hear from you!** Drop us an issue or join the discussion on GitHub.
