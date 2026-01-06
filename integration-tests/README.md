# Terraphim AI Integration Testing Framework

A comprehensive integration testing framework for Terraphim AI release validation, implementing all required testing scenarios for production readiness.

## Overview

This framework provides automated integration testing across six critical dimensions:

1. **Multi-Component Integration**: Server + TUI, Desktop + Server, Multi-Server communication
2. **Data Flow Validation**: End-to-end workflows, data persistence, file operations, network communication
3. **Cross-Platform Integration**: Platform behaviors, container orchestration, system services
4. **Error Handling & Recovery**: Network failures, resource constraints, corruption scenarios, graceful degradation
5. **Performance & Scalability**: Concurrent load, data scaling, resource monitoring, regression detection
6. **Security Integration**: Authentication flows, authorization boundaries, data protection, audit trails

## Architecture

```
integration-tests/
├── run_integration_tests.sh      # Main test orchestrator
├── framework/
│   └── common.sh                 # Shared utilities and functions
├── scenarios/                    # Test scenario implementations
│   ├── multi_component_tests.sh
│   ├── data_flow_tests.sh
│   ├── cross_platform_tests.sh
│   ├── error_handling_tests.sh
│   ├── performance_tests.sh
│   └── security_tests.sh
├── matrix/                       # Test matrix configurations
├── performance/                  # Performance benchmarking
├── security/                     # Security test configurations
├── docker/                       # Container test setups
└── ci/                          # CI/CD integration
```

## Quick Start

### Prerequisites

- Linux/macOS/Windows with Bash
- Docker and Docker Compose (for container tests)
- curl, jq, bc (for test utilities)
- Rust toolchain (for building test servers)

### Running All Tests

```bash
cd integration-tests
./run_integration_tests.sh
```

### Running Specific Test Phases

```bash
# Run only multi-component tests
./run_integration_tests.sh --skip-data-flow --skip-cross-platform --skip-error-handling --skip-performance --skip-security

# Run only performance tests
./run_integration_tests.sh --skip-multi-component --skip-data-flow --skip-cross-platform --skip-error-handling --skip-security

# Skip specific test categories
./run_integration_tests.sh --skip-performance --skip-security
```

### Running Individual Test Scenarios

```bash
# Run specific test files directly
./scenarios/multi_component_tests.sh
./scenarios/performance_tests.sh
```

## Test Categories

### 1. Multi-Component Integration Testing

**Purpose**: Validate communication between different Terraphim components.

**Tests Include**:
- Server + TUI HTTP API communication
- Desktop + Server bidirectional communication
- Multi-server load balancing and failover
- External service integration (databases, APIs)

**Success Criteria**: All component interactions functional.

### 2. Data Flow Validation

**Purpose**: Ensure data flows correctly through the entire system.

**Tests Include**:
- End-to-end user journey validation
- Data persistence across sessions
- File system operations (import/export)
- Network communication patterns
- Streaming data handling

**Success Criteria**: Complete data workflows functional.

### 3. Cross-Platform Integration

**Purpose**: Validate behavior across different platforms and deployment scenarios.

**Tests Include**:
- Platform-specific file path handling
- Permission management
- Container orchestration (Docker)
- System service integration
- Background process management
- Hardware interaction

**Success Criteria**: Consistent behavior across platforms.

### 4. Error Handling and Recovery

**Purpose**: Test system resilience under adverse conditions.

**Tests Include**:
- Network failure recovery
- Resource constraint handling (memory, disk, CPU)
- Database/file corruption recovery
- File system issue recovery
- Network interruption handling
- Graceful degradation under load

**Success Criteria**: System recovers gracefully from all error conditions.

### 5. Performance and Scalability

**Purpose**: Validate performance characteristics and scaling capabilities.

**Tests Include**:
- Concurrent user load testing
- Large dataset handling
- System resource monitoring (CPU, memory, disk, network)
- Performance regression detection
- Response time distribution analysis

**Success Criteria**: Performance scales appropriately with load.

### 6. Security Integration Testing

**Purpose**: Validate security controls and data protection.

**Tests Include**:
- Authentication flow validation
- Authorization boundary testing
- Data protection (encryption, sanitization)
- Audit trail validation
- Access pattern monitoring

**Success Criteria**: Security controls prevent unauthorized access.

## Configuration

### Test Server Configuration

Tests automatically start temporary Terraphim servers on different ports. Configuration is generated dynamically for each test scenario.

### Environment Variables

- `TERRAPHIM_TEST_MODE=true`: Enables test mode
- `RUST_LOG=debug`: Enables debug logging
- `NODE_ENV=test`: Sets Node.js environment for frontend tests

### Custom Test Configuration

Create custom configuration files in the `matrix/` directory:

```json
{
  "test_name": "custom_load_test",
  "concurrency_levels": [10, 50, 100],
  "duration_seconds": 300,
  "thresholds": {
    "max_response_time_ms": 5000,
    "max_error_rate": 0.05
  }
}
```

## Results and Reporting

### Test Results Format

Results are stored in JSON format at `/tmp/terraphim_integration_results_*.json`:

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "framework_version": "1.0.0",
  "results": {
    "multi_component": [...],
    "data_flow": [...],
    "cross_platform": [...],
    "error_handling": [...],
    "performance": [...],
    "security": [...]
  },
  "summary": {
    "total": 25,
    "passed": 23,
    "failed": 1,
    "skipped": 1,
    "coverage": 92.0
  }
}
```

### Result Interpretation

- **Coverage >= 95%**: Release candidate ready
- **Coverage 80-94%**: Additional testing recommended
- **Coverage < 80%**: Critical issues require attention

## CI/CD Integration

### GitHub Actions

Add to your workflow:

```yaml
- name: Run Integration Tests
  run: |
    cd integration-tests
    ./run_integration_tests.sh

- name: Upload Test Results
  uses: actions/upload-artifact@v3
  with:
    name: integration-test-results
    path: /tmp/terraphim_integration_results_*.json
```

### Jenkins Pipeline

```groovy
stage('Integration Tests') {
    steps {
        sh '''
            cd integration-tests
            ./run_integration_tests.sh
        '''
    }
    post {
        always {
            archiveArtifacts artifacts: '/tmp/terraphim_integration_results_*.json', allowEmptyArchive: true
        }
    }
}
```

## Troubleshooting

### Common Issues

1. **Port conflicts**: Tests use ports 8080-8999. Ensure these are available.

2. **Docker not available**: Container tests will be skipped automatically.

3. **Permission denied**: Run tests with appropriate permissions or use Docker.

4. **Resource constraints**: Reduce concurrency in `common.sh` for resource-limited environments.

### Debug Mode

Enable verbose logging:

```bash
export RUST_LOG=debug
export DEBUG_INTEGRATION_TESTS=true
./run_integration_tests.sh
```

### Manual Test Execution

Run individual test functions:

```bash
source framework/common.sh
test_concurrent_user_load
```

## Extending the Framework

### Adding New Test Scenarios

1. Create a new test file in `scenarios/`
2. Implement test functions following the naming pattern `test_*`
3. Add the test to the main orchestrator in `run_integration_tests.sh`
4. Update this README

### Custom Test Utilities

Add utility functions to `framework/common.sh`:

```bash
# Example: Custom assertion function
assert_response_time() {
    local url="$1"
    local max_time="$2"

    local response_time=$(measure_execution_time "curl -s '$url' > /dev/null")
    if (( $(echo "$response_time > $max_time" | bc -l) )); then
        log_error "Response time $response_time > $max_time"
        return 1
    fi
    return 0
}
```

## Success Criteria Summary

- **95%+ integration coverage** for all component interactions
- **End-to-end workflow validation** with real data flows
- **Error handling validation** for all failure scenarios
- **Performance scaling** validated up to defined limits
- **Security compliance** verified across all integration points
- **Automated testing** runnable in CI/CD pipelines

## Contributing

1. Follow the existing code style and patterns
2. Add comprehensive logging to new tests
3. Include proper error handling and cleanup
4. Update documentation for new features
5. Test on multiple platforms when possible

## License

This testing framework is part of the Terraphim AI project and follows the same license terms.