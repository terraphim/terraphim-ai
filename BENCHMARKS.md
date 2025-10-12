# TerraphimAgent Performance Benchmarks

This document describes the comprehensive performance benchmarking suite for TerraphimAgent operations.

## Overview

The benchmark suite provides comprehensive performance testing for:

- **Core Agent Operations**: Agent creation, initialization, command processing
- **WebSocket Communication**: Protocol performance, message throughput, connection handling
- **Multi-Agent Workflows**: Concurrent execution, batch processing, coordination
- **Knowledge Graph Operations**: Query performance, path finding, automata operations
- **Memory Management**: Context enrichment, state persistence, resource utilization
- **LLM Integration**: Request processing, token tracking, cost management

## Benchmark Structure

### 1. Rust Core Benchmarks (`crates/terraphim_multi_agent/benches/`)

**File**: `agent_operations.rs`

Uses Criterion.rs for statistical benchmarking with HTML reports.

#### Key Benchmarks:

- **Agent Lifecycle**
  - Agent creation time
  - Agent initialization
  - State save/load operations

- **Command Processing**
  - Generate commands
  - Answer commands
  - Analyze commands
  - Create commands
  - Review commands

- **Registry Operations**
  - Agent registration
  - Capability-based discovery
  - Load balancing

- **Memory Operations**
  - Context enrichment
  - State persistence
  - Knowledge graph queries

- **Batch & Concurrent Operations**
  - Batch command processing (1, 5, 10, 20, 50 commands)
  - Concurrent execution (1, 2, 4, 8 threads)

- **Knowledge Graph Operations**
  - RoleGraph queries
  - Node matching
  - Path connectivity checks

- **Automata Operations**
  - Autocomplete functionality
  - Pattern matching
  - Text processing

- **LLM Operations**
  - Simple generation
  - Request processing

- **Tracking Operations**
  - Token usage tracking
  - Cost calculation
  - Budget monitoring

### 2. JavaScript WebSocket Benchmarks (`desktop/tests/benchmarks/`)

**File**: `agent-performance.benchmark.js`

Uses Vitest for JavaScript performance testing.

#### Key Benchmarks:

- **WebSocket Connection Performance**
  - Connection establishment (10 concurrent connections)
  - Message processing throughput (50 messages/connection)

- **Workflow Performance**
  - Workflow start latency (5 concurrent workflows)
  - Concurrent workflow execution

- **Command Processing Performance**
  - Different command types (generate, analyze, answer, create, review)
  - End-to-end processing time

- **Throughput Performance**
  - 10-second load test
  - Operations per second measurement
  - Latency under load

- **Memory and Resource Performance**
  - Memory operation efficiency (20 operations)
  - Batch operations (configurable batch size)

- **Error Handling Performance**
  - Malformed message handling
  - Connection resilience
  - Error recovery time

#### Performance Thresholds:

```javascript
const THRESHOLDS = {
  webSocketConnection: { avg: 500, p95: 1000 },    // ms
  messageProcessing: { avg: 100, p95: 200 },       // ms
  workflowStart: { avg: 2000, p95: 5000 },         // ms
  commandProcessing: { avg: 3000, p95: 10000 },    // ms
  memoryOperations: { avg: 50, p95: 100 },         // ms
  contextEnrichment: { avg: 500, p95: 1000 },      // ms
  batchOperations: { avg: 5000, p95: 15000 },      // ms
};
```

## Running Benchmarks

### Quick Start

```bash
# Run all benchmarks with comprehensive reporting
./scripts/run-benchmarks.sh

# Run only Rust benchmarks
./scripts/run-benchmarks.sh --rust-only

# Run only JavaScript benchmarks
./scripts/run-benchmarks.sh --js-only
```

### Individual Benchmark Execution

#### Rust Benchmarks

```bash
# Navigate to multi-agent crate
cd crates/terraphim_multi_agent

# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench agent_creation

# Generate HTML reports
cargo bench -- --output-format html
```

#### JavaScript Benchmarks

```bash
# Navigate to desktop directory
cd desktop

# Run benchmarks
yarn benchmark

# Watch mode for development
yarn benchmark:watch

# UI mode with visualization
yarn benchmark:ui

# Run with specific configuration
npx vitest --config vitest.benchmark.config.ts
```

## Benchmark Reports

### Generated Files

- **Performance Report**: `benchmark-results/[timestamp]/performance_report.md`
- **Rust Results**: `benchmark-results/[timestamp]/rust_benchmarks.txt`
- **JavaScript Results**: `benchmark-results/[timestamp]/js_benchmarks.json`
- **Criterion HTML**: `benchmark-results/[timestamp]/rust_criterion_reports/`

### Report Structure

Each benchmark run generates:

1. **Executive Summary**: Key performance metrics overview
2. **Detailed Results**: Per-operation timing and statistics
3. **Threshold Analysis**: Pass/fail status against performance targets
4. **Raw Data**: Complete benchmark output for further analysis
5. **Recommendations**: Performance optimization suggestions

### Reading Results

#### Rust Criterion Output

```
Agent Creation            time:   [45.2 ms 47.1 ms 49.3 ms]
                          change: [-2.1% +0.8% +3.9%] (p = 0.18 > 0.05)
```

- **Time Range**: Lower bound, estimate, upper bound
- **Change**: Performance change from previous run
- **P-value**: Statistical significance

#### JavaScript Vitest Output

```json
{
  "name": "WebSocket Connection",
  "count": 10,
  "avg": 324.5,
  "min": 298.1,
  "max": 367.2,
  "p95": 359.8,
  "p99": 365.1
}
```

- **Count**: Number of samples
- **Avg**: Average execution time (ms)
- **Min/Max**: Fastest/slowest execution
- **P95/P99**: 95th/99th percentile times

## Performance Optimization

### Identifying Bottlenecks

1. **High Latency Operations**: Look for operations exceeding thresholds
2. **Memory Pressure**: Monitor memory operations for excessive allocation
3. **Concurrency Issues**: Compare single-threaded vs multi-threaded performance
4. **Network Bottlenecks**: Analyze WebSocket throughput patterns

### Common Optimizations

#### Rust Side

- **Agent Pooling**: Reuse initialized agents
- **Connection Pooling**: Efficient database/LLM connections
- **Async Optimization**: Reduce unnecessary context switches
- **Memory Management**: Optimize allocation patterns

#### JavaScript Side

- **Message Batching**: Group related operations
- **Connection Management**: Reuse WebSocket connections
- **Error Recovery**: Fast error handling without reconnection
- **Resource Cleanup**: Proper cleanup to prevent memory leaks

## Continuous Integration

### Automated Benchmarking

The benchmark suite can be integrated into CI/CD pipelines:

```yaml
name: Performance Benchmarks
on: [pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
      - name: Setup Node.js
        uses: actions/setup-node@v3
      - name: Run Benchmarks
        run: ./scripts/run-benchmarks.sh
      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: benchmark-results/
```

### Performance Regression Detection

- **Threshold Monitoring**: Automated alerts when thresholds are exceeded
- **Trend Analysis**: Track performance changes over time
- **Comparative Analysis**: Compare performance across versions

## Configuration

### Rust Benchmark Configuration

Located in `crates/terraphim_multi_agent/Cargo.toml`:

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "agent_operations"
harness = false
```

### JavaScript Benchmark Configuration

Located in `desktop/vitest.benchmark.config.ts`:

```typescript
export default defineConfig({
  test: {
    include: ['tests/benchmarks/**/*.benchmark.{js,ts}'],
    timeout: 120000, // 2 minutes per test
    reporters: ['verbose', 'json'],
    pool: 'forks',
    poolOptions: {
      forks: { singleFork: true }
    }
  }
});
```

## Troubleshooting

### Common Issues

1. **Server Not Starting**: Ensure no other processes are using benchmark ports
2. **Timeout Errors**: Increase timeout values for slower systems
3. **Memory Issues**: Reduce batch sizes or concurrent operations
4. **WebSocket Failures**: Check firewall settings and port availability

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug ./scripts/run-benchmarks.sh

# Verbose JavaScript output
yarn benchmark -- --reporter=verbose

# Single test execution
yarn benchmark -- --run tests/benchmarks/specific-test.benchmark.js
```

## Contributing

### Adding New Benchmarks

#### Rust Benchmarks

1. Add benchmark function to `agent_operations.rs`
2. Include in `criterion_group!` macro
3. Document expected performance characteristics

#### JavaScript Benchmarks

1. Add test to `agent-performance.benchmark.js`
2. Include performance thresholds
3. Add proper error handling and cleanup

### Performance Testing Guidelines

1. **Consistent Environment**: Run benchmarks on consistent hardware
2. **Warm-up Runs**: Include warm-up iterations for JIT optimization
3. **Statistical Significance**: Ensure sufficient sample sizes
4. **Isolation**: Avoid interference from other processes
5. **Documentation**: Document expected performance ranges

## Performance Targets

### Production Thresholds

- **Agent Creation**: < 100ms average
- **Command Processing**: < 5s p95
- **WebSocket Latency**: < 200ms p95
- **Memory Operations**: < 100ms p95
- **Throughput**: > 100 operations/second

### Development Thresholds

- **Agent Creation**: < 200ms average
- **Command Processing**: < 10s p95
- **WebSocket Latency**: < 500ms p95
- **Memory Operations**: < 200ms p95
- **Throughput**: > 50 operations/second

These benchmarks ensure TerraphimAgent maintains high performance across all operation categories while providing detailed insights for optimization efforts.
