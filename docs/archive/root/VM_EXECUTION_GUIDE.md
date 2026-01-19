# VM Execution System Guide

## Overview

The Terraphim Multi-Agent System integrates secure code execution capabilities using Firecracker MicroVMs. This guide covers the complete architecture for executing code from LLM agents in isolated VM environments with comprehensive safety, history tracking, and session management.

## Architecture Components

### 1. Core Models (`vm_execution/models.rs`)

#### VmExecutionConfig
Configuration for VM-based code execution:
```rust
pub struct VmExecutionConfig {
    pub enabled: bool,
    pub api_base_url: String,        // fcctl-web URL (e.g., "http://localhost:8080")
    pub vm_pool_size: usize,          // Pre-warmed VMs per agent
    pub default_vm_type: String,      // "ubuntu", "rust-vm", etc.
    pub execution_timeout_ms: u64,    // Max execution time
    pub allowed_languages: Vec<String>, // ["python", "javascript", "rust", "bash"]
    pub auto_provision: bool,         // Auto-create VMs when needed
    pub code_validation: bool,        // Enable security checks
    pub max_code_length: usize,       // Code size limit
    pub history: HistoryConfig,       // History tracking configuration
}
```

#### HistoryConfig
VM session history and snapshot configuration:
```rust
pub struct HistoryConfig {
    pub enabled: bool,
    pub snapshot_on_execution: bool,      // Snapshot after each command
    pub snapshot_on_failure: bool,        // Snapshot on errors
    pub auto_rollback_on_failure: bool,   // Auto-revert on failures
    pub max_history_entries: usize,       // History size limit
    pub persist_history: bool,            // Save to disk
    pub integration_mode: String,         // "http" or "direct"
}
```

#### Language Support
Built-in language configurations with security restrictions:

**Python**:
- Extension: `.py`
- Execute: `python3`
- Restrictions: `subprocess`, `os.system`, `eval`, `exec`, `__import__`
- Timeout multiplier: 1.0x

**JavaScript/Node.js**:
- Extension: `.js`
- Execute: `node`
- Restrictions: `child_process`, `eval`, `Function(`, `require('fs')`
- Timeout multiplier: 1.0x

**Bash**:
- Extension: `.sh`
- Execute: `bash`
- Restrictions: `rm -rf`, `dd`, `mkfs`, `:(){ :|:& };:`, `chmod 777`
- Timeout multiplier: 1.0x

**Rust**:
- Extension: `.rs`
- Execute: `rustc` (compile then run)
- Restrictions: `unsafe`, `std::process`, `std::fs::remove`
- Timeout multiplier: 3.0x (accounts for compilation time)

### 2. Code Extraction (`vm_execution/code_extractor.rs`)

#### CodeBlockExtractor
Extracts executable code blocks from LLM responses:

```rust
let extractor = CodeBlockExtractor::new();

// Extract all code blocks with confidence scores
let blocks = extractor.extract_code_blocks(llm_response);

for block in blocks {
    println!("Language: {}", block.language);
    println!("Confidence: {:.2}", block.execution_confidence);
    println!("Code:\n{}", block.code);

    // Validate before execution
    if let Err(e) = extractor.validate_code(&block) {
        eprintln!("Security violation: {}", e);
    }
}
```

#### Pattern Detection
Identifies code blocks in markdown format:
```
```python
def factorial(n):
    return 1 if n <= 1 else n * factorial(n-1)
print(factorial(5))
```
```

### 3. VM Execution Client (`vm_execution/client.rs`)

#### VmExecutionClient
HTTP client for fcctl-web API integration:

```rust
let config = VmExecutionConfig {
    enabled: true,
    api_base_url: "http://localhost:8080".to_string(),
    vm_pool_size: 2,
    default_vm_type: "ubuntu".to_string(),
    execution_timeout_ms: 30000,
    allowed_languages: vec!["python".into(), "rust".into()],
    auto_provision: true,
    code_validation: true,
    max_code_length: 10000,
    history: HistoryConfig::default(),
};

let client = VmExecutionClient::new(&config);

// Execute Python code
let response = client.execute_python(
    "agent-001",
    "print('Hello from VM!')",
    None
).await?;

println!("Exit Code: {}", response.exit_code);
println!("Output: {}", response.stdout);
```

### 4. DirectSessionAdapter (`vm_execution/session_adapter.rs`)

Low-overhead session management using HTTP API (avoids fcctl-repl dependency conflicts):

```rust
let adapter = DirectSessionAdapter::new(
    PathBuf::from("/var/lib/terraphim/sessions"),
    "http://localhost:8080".to_string()
);

// Create or reuse session
let session_id = adapter.get_or_create_session(
    "vm-001",
    "agent-001",
    "ubuntu"
).await?;

// Execute command
let (output, exit_code) = adapter.execute_command_direct(
    &session_id,
    "echo 'Hello' > /tmp/state.txt && cat /tmp/state.txt"
).await?;

// Create snapshot
let snapshot_id = adapter.create_snapshot_direct(
    &session_id,
    "before-modification"
).await?;

// Rollback if needed
adapter.rollback_direct(&session_id, &snapshot_id).await?;

// Close when done
adapter.close_session(&session_id).await?;
```

### 5. Hook System (`vm_execution/hooks.rs`)

Pre/post processing hooks for tool and LLM interactions inspired by Claude Agent SDK:

#### Hook Trait
```rust
#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;

    async fn pre_tool(&self, context: &PreToolContext)
        -> Result<HookDecision, VmExecutionError>;

    async fn post_tool(&self, context: &PostToolContext)
        -> Result<HookDecision, VmExecutionError>;

    async fn pre_llm(&self, context: &PreLlmContext)
        -> Result<HookDecision, VmExecutionError>;

    async fn post_llm(&self, context: &PostLlmContext)
        -> Result<HookDecision, VmExecutionError>;
}
```

#### Built-in Hooks

**DangerousPatternHook** - Security validation:
```rust
let hook = DangerousPatternHook::new();
let manager = HookManager::new();
manager.add_hook(Arc::new(hook));

// Blocks dangerous patterns like "rm -rf /", "eval(...)", etc.
```

**SyntaxValidationHook** - Code validation:
```rust
let hook = SyntaxValidationHook::new();
// Validates language support, code length limits, basic syntax
```

**ExecutionLoggerHook** - Observability:
```rust
let hook = ExecutionLoggerHook::new();
// Logs all executions for debugging and audit trails
```

**DependencyInjectorHook** - Auto-import injection:
```rust
let hook = DependencyInjectorHook::new();
// Automatically adds required imports for common patterns
```

**OutputSanitizerHook** - Sensitive data filtering:
```rust
let hook = OutputSanitizerHook::new();
// Filters API keys, passwords, secrets from output
```

#### Hook Manager
Orchestrates multiple hooks with decision handling:

```rust
let mut manager = HookManager::new();
manager.add_hook(Arc::new(DangerousPatternHook::new()));
manager.add_hook(Arc::new(SyntaxValidationHook::new()));

let context = PreToolContext {
    code: "print('test')".to_string(),
    language: "python".to_string(),
    agent_id: "agent-001".to_string(),
    vm_id: "vm-001".to_string(),
    metadata: HashMap::new(),
};

match manager.run_pre_tool(&context).await? {
    HookDecision::Allow => { /* proceed */ },
    HookDecision::Block { reason } => { /* deny */ },
    HookDecision::Modify { transformed_code } => { /* use modified */ },
    HookDecision::AskUser { prompt } => { /* request approval */ },
}
```

### 6. FcctlBridge (`vm_execution/fcctl_bridge.rs`)

Integration layer between LLM agents and fcctl infrastructure:

```rust
let config = HistoryConfig {
    enabled: true,
    snapshot_on_execution: true,
    snapshot_on_failure: false,
    auto_rollback_on_failure: true,
    max_history_entries: 100,
    persist_history: true,
    integration_mode: "direct".to_string(),
};

let bridge = FcctlBridge::new(
    config,
    "http://localhost:8080".to_string()
);

// Track execution with automatic snapshots
bridge.track_execution(
    "agent-001",
    "vm-001",
    "echo 'test'",
    0,
    "test output"
).await?;

// Query history
let history = bridge.query_history("agent-001", "vm-001", None, None).await?;

// Auto-rollback on failure
bridge.auto_rollback_on_failure(
    "agent-001",
    "vm-001",
    &error_msg
).await?;
```

## Integration with TerraphimAgent

### Configuration

Add VM execution to agent role configuration:

```json
{
  "name": "Code Execution Agent",
  "relevance_function": "BM25",
  "extra": {
    "vm_execution": {
      "enabled": true,
      "api_base_url": "http://localhost:8080",
      "vm_pool_size": 2,
      "default_vm_type": "ubuntu",
      "execution_timeout_ms": 60000,
      "allowed_languages": ["python", "javascript", "bash", "rust"],
      "auto_provision": true,
      "code_validation": true,
      "max_code_length": 10000,
      "history": {
        "enabled": true,
        "snapshot_on_execution": true,
        "snapshot_on_failure": false,
        "auto_rollback_on_failure": false,
        "max_history_entries": 100,
        "persist_history": true,
        "integration_mode": "direct"
      }
    }
  }
}
```

### Agent Usage

```rust
use terraphim_multi_agent::agent::{TerraphimAgent, CommandInput, CommandType};

let role = Role::from_file("code_execution_agent.json")?;
let agent = TerraphimAgent::new(role).await?;

let input = CommandInput {
    command: CommandType::Execute,
    text: r#"
Calculate fibonacci numbers:

```python
def fib(n):
    if n <= 1: return n
    return fib(n-1) + fib(n-2)

for i in range(10):
    print(f"fib({i}) = {fib(i)}")
```
    "#.to_string(),
    metadata: None,
};

let result = agent.process_command(input).await?;
println!("Execution result: {}", result.response);
```

## Testing

### Test Organization

#### Unit Tests
No external dependencies required:
```bash
./scripts/test-vm-features.sh unit
```

Tests:
- Hook system functionality
- Session adapter logic
- Code extraction and validation
- Configuration parsing
- Basic Rust execution tests

#### Integration Tests
Requires fcctl-web running at localhost:8080:
```bash
# Start fcctl-web
cd scratchpad/firecracker-rust && cargo run -p fcctl-web

# Run integration tests
./scripts/test-vm-features.sh integration
```

Tests:
- DirectSessionAdapter with real HTTP API
- FcctlBridge integration modes (direct vs HTTP)
- Hook integration with VM client
- Rust compilation and execution
- Session lifecycle and snapshots

#### End-to-End Tests
Requires full stack (fcctl-web + agent system):
```bash
./scripts/test-vm-features.sh e2e
```

Tests:
- Complete workflows from user input to VM execution
- Multi-language execution (Python, JavaScript, Bash, Rust)
- Security blocking dangerous code
- Multi-turn conversations with VM state persistence
- Error recovery with history
- Performance tests (rapid execution, concurrent sessions)

#### Language-Specific Tests
Rust compilation and execution suite:
```bash
./scripts/test-vm-features.sh rust
```

### Test Automation Script

```bash
# Unit tests only (fast, no server required)
./scripts/test-vm-features.sh unit

# Integration tests (requires fcctl-web)
./scripts/test-vm-features.sh integration

# E2E tests (requires full stack)
./scripts/test-vm-features.sh e2e

# Rust-specific suite
./scripts/test-vm-features.sh rust

# All tests
./scripts/test-vm-features.sh all

# Help
./scripts/test-vm-features.sh help
```

## Security Considerations

### Code Validation
- Automatic pattern detection for dangerous operations
- Language-specific security restrictions
- Code length limits to prevent resource exhaustion
- Syntax validation before execution

### Execution Isolation
- Each agent gets dedicated VM instances
- Network isolation between VMs
- Resource limits (CPU, memory, disk)
- Timeout enforcement for runaway code

### History and Rollback
- Snapshot before dangerous operations
- Automatic rollback on failures (optional)
- Command history for audit trails
- State recovery mechanisms

### Hook System Security
- Pre-execution validation hooks
- Output sanitization hooks
- User approval for sensitive operations
- Custom security policies per agent

## Performance Optimization

### VM Pool Management
- Pre-warmed VM instances for fast execution
- Pool size per agent for concurrent operations
- Auto-scaling based on demand
- Health checks and automatic recovery

### Session Reuse
- DirectSessionAdapter maintains persistent sessions
- Avoids VM creation overhead for sequential commands
- State preservation across command executions
- Efficient snapshot and rollback operations

### Language-Specific Optimizations
- Rust: 3x timeout multiplier for compilation
- Python/JavaScript: 1x standard timeouts
- Bash: Fast execution with minimal overhead
- Caching for compiled languages

## Advanced Features

### Custom Hook Implementation

```rust
use terraphim_multi_agent::vm_execution::hooks::*;

pub struct CustomSecurityHook {
    patterns: Vec<String>,
}

#[async_trait]
impl Hook for CustomSecurityHook {
    fn name(&self) -> &str {
        "CustomSecurityHook"
    }

    async fn pre_tool(&self, context: &PreToolContext)
        -> Result<HookDecision, VmExecutionError> {
        for pattern in &self.patterns {
            if context.code.contains(pattern) {
                return Ok(HookDecision::Block {
                    reason: format!("Code contains forbidden pattern: {}", pattern)
                });
            }
        }
        Ok(HookDecision::Allow)
    }

    // Implement other hook methods...
}
```

### WebSocket Real-Time Updates

The fcctl-web API provides WebSocket support for streaming execution output:

```rust
// Connect to WebSocket for real-time updates
ws://localhost:8080/ws

// Send execution command
{
    "command_type": "execute_code",
    "session_id": "session-123",
    "workflow_id": "workflow-456",
    "data": {
        "language": "python",
        "code": "for i in range(10): print(i)"
    }
}

// Receive streaming output
{
    "response_type": "execution_output",
    "data": {
        "stdout": "0\n1\n2\n..."
    }
}
```

### Multi-Language Workflows

Execute multiple languages in sequence within same session:

```rust
let session_id = adapter.get_or_create_session("vm-1", "agent-1", "ubuntu").await?;

// Python data processing
let (py_output, _) = adapter.execute_command_direct(
    &session_id,
    "python3 -c 'import json; print(json.dumps({\"result\": 42}))'"
).await?;

// Bash file manipulation
let (bash_output, _) = adapter.execute_command_direct(
    &session_id,
    "echo '$py_output' > data.json && cat data.json"
).await?;

// Rust compilation and execution
let (rust_output, _) = adapter.execute_command_direct(
    &session_id,
    "rustc main.rs && ./main"
).await?;
```

## Troubleshooting

### Common Issues

**"SessionNotFound" errors**:
- Ensure fcctl-web is running on correct port
- Check session hasn't timed out
- Verify session_id is correct

**Compilation failures for Rust**:
- Increase execution timeout (3x standard)
- Check Rust toolchain installed in VM
- Verify code syntax before execution

**WebSocket disconnections**:
- Use correct protocol (`ws://` not `http://`)
- Implement reconnection logic
- Check firewall/proxy settings

**Security hook blocking legitimate code**:
- Review hook patterns
- Add exceptions for known-safe patterns
- Use custom hooks for specific requirements

### Debug Logging

Enable debug output:
```bash
RUST_LOG=debug cargo test --test vm_execution_e2e_tests -- --nocapture
```

### Health Checks

Verify fcctl-web availability:
```bash
curl http://localhost:8080/health
```

## Production Deployment

### Infrastructure Requirements
- fcctl-web service running and accessible
- Persistent storage for VM sessions and snapshots
- Network isolation for security
- Resource monitoring and alerts

### Configuration Best Practices
- Enable history and snapshots for critical operations
- Set appropriate timeout values per language
- Configure auto-rollback for production safety
- Use direct integration mode for lower overhead
- Enable all built-in security hooks
- Set reasonable pool sizes based on workload

### Monitoring
- Track execution success/failure rates
- Monitor VM resource usage
- Alert on timeout violations
- Audit history entries for compliance
- Track hook block/allow decisions

## Examples

### Complete Example: Fibonacci with Error Handling

```rust
use terraphim_multi_agent::{
    agent::{TerraphimAgent, CommandInput, CommandType},
    vm_execution::*,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load agent with VM execution enabled
    let role = Role::from_file("examples/vm_execution_agent_config.json")?;
    let agent = TerraphimAgent::new(role).await?;

    // Execute fibonacci calculation
    let input = CommandInput {
        command: CommandType::Execute,
        text: r#"
Calculate fibonacci(20) efficiently:

```python
def fib(n, memo={}):
    if n in memo: return memo[n]
    if n <= 1: return n
    memo[n] = fib(n-1, memo) + fib(n-2, memo)
    return memo[n]

result = fib(20)
print(f"Fibonacci(20) = {result}")
```
        "#.to_string(),
        metadata: None,
    };

    match agent.process_command(input).await {
        Ok(result) if result.success => {
            println!("✓ Execution successful!");
            println!("Output: {}", result.response);
        }
        Ok(result) => {
            eprintln!("✗ Execution failed: {}", result.response);
        }
        Err(e) => {
            eprintln!("✗ Error: {}", e);
        }
    }

    Ok(())
}
```

## Further Reading

- [fcctl-web API Documentation](../scratchpad/firecracker-rust/README.md)
- [Firecracker MicroVM Documentation](https://firecracker-microvm.github.io/)
- [Claude Agent SDK Python](https://github.com/anthropics/claude-agent-sdk-python)
- [Test Coverage Report](./VM_EXECUTION_TEST_PLAN.md)
