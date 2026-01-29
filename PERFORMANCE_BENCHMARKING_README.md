# Terraphim AI Performance Benchmarking Framework

A comprehensive performance benchmarking suite for Terraphim AI release validation, providing automated performance testing, regression detection, and CI/CD integration.

## Overview

This framework provides complete performance validation for Terraphim AI, covering:

- **Server API Benchmarks**: HTTP request/response timing, throughput measurement
- **Search Engine Performance**: Query execution time, result ranking accuracy, indexing speed
- **Database Operations**: CRUD operation timing, transaction performance, query optimization
- **File System Operations**: Read/write performance, large file handling, concurrent access
- **Resource Utilization**: CPU, memory, disk I/O, and network monitoring
- **Scalability Testing**: Concurrent users, data scale handling, load balancing
- **Comparative Analysis**: Baseline establishment, regression detection, trend analysis

## Quick Start

### Prerequisites

```bash
# Required tools
sudo apt-get install curl jq bc wrk  # Linux
# or
brew install curl jq  # macOS (wrk may need separate installation)

# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Running Benchmarks

```bash
# Run all performance benchmarks
./scripts/run-performance-benchmarks.sh

# Run with custom iterations
./scripts/run-performance-benchmarks.sh --iterations=5000

# Run with baseline comparison
./scripts/run-performance-benchmarks.sh --baseline=benchmark-results/baseline.json

# Verbose output
./scripts/run-performance-benchmarks.sh --verbose
```

## Architecture

### Core Components

```
crates/terraphim_validation/src/performance/
├── benchmarking.rs          # Core benchmarking framework
├── ci_integration.rs        # CI/CD integration and gates
└── mod.rs                   # Module exports

scripts/
└── run-performance-benchmarks.sh  # Main benchmarking script

.github/workflows/
└── performance-benchmarking.yml   # GitHub Actions workflow

benchmark-config.json        # Performance gate configuration
```

### Benchmark Categories

#### 1. Core Performance Benchmarks

**Server API Benchmarks**
- Health check endpoint performance
- Search API response times
- Configuration API operations
- Chat completion endpoints
- Custom endpoint benchmarking

**Search Engine Performance**
- Query execution latency
- Result ranking accuracy
- Fuzzy search performance
- Large result set handling
- Indexing operation speed

**Database Operations**
- CRUD operation timing
- Transaction performance
- Query optimization validation
- Bulk operation efficiency

**File System Operations**
- Read/write performance
- Large file handling
- Concurrent file access
- Directory operations

#### 2. Resource Utilization Monitoring

**CPU Monitoring**
- Idle CPU usage tracking
- Load condition CPU usage
- Thread utilization patterns
- Core contention detection

**Memory Monitoring**
- RSS memory consumption
- Virtual memory usage
- Memory leak detection
- Garbage collection efficiency

**Disk I/O Monitoring**
- Read/write throughput
- Seek time performance
- File system latency
- Concurrent I/O patterns

**Network Monitoring**
- Bandwidth utilization
- Connection handling efficiency
- Protocol overhead
- Data transfer rates

#### 3. Scalability Testing

**Concurrent User Simulation**
- Multiple simultaneous users
- Session management scaling
- Resource contention analysis
- Connection pool efficiency

**Data Scale Handling**
- Large dataset processing
- Search index scaling
- Document collection growth
- Memory usage scaling

**Load Balancing Validation**
- Request distribution analysis
- Failover scenario testing
- Capacity planning metrics
- Resource scaling limits

#### 4. Comparative Analysis

**Baseline Establishment**
- Historical performance data
- Version comparison framework
- Statistical baseline calculation
- Trend analysis setup

**Regression Detection**
- Performance degradation alerts
- Automated threshold checking
- Statistical significance testing
- Anomaly detection

**Optimization Validation**
- Performance improvement verification
- Tuning effectiveness measurement
- Comparative algorithm analysis
- Bottleneck identification

### 5. Automated Benchmarking Pipeline

**CI/CD Integration**
- GitHub Actions workflow automation
- Performance gate enforcement
- Build failure on regression
- Automated baseline updates

**Performance Gates**
- Configurable threshold checking
- Blocking vs warning severity levels
- Metric-based gate definitions
- SLO compliance validation

**Report Generation**
- HTML dashboard reports
- JSON structured data
- Markdown summaries
- PDF documentation export

**Historical Tracking**
- Performance trend analysis
- Version comparison charts
- Improvement tracking
- Degradation alerts

## Configuration

### Performance Gates Configuration

Create a `benchmark-config.json` file:

```json
{
  "gates": [
    {
      "name": "API Response Time",
      "metric": "search_api.avg_time_ms",
      "operator": "LessThan",
      "threshold": 1000.0,
      "severity": "Blocking"
    },
    {
      "name": "Search Success Rate",
      "metric": "search_api.success_rate",
      "operator": "GreaterThanOrEqual",
      "threshold": 99.0,
      "severity": "Blocking"
    }
  ],
  "fail_on_regression": true,
  "regression_threshold_percent": 5.0,
  "update_baseline_on_success": true,
  "reporting": {
    "json": true,
    "html": true,
    "markdown": true,
    "upload_external": false
  }
}
```

### SLO Configuration

Service Level Objectives are defined in the benchmarking code:

```rust
pub struct PerformanceSLO {
    pub max_startup_time_ms: u64,          // 5000ms
    pub max_api_response_time_ms: u64,     // 500ms
    pub max_search_time_ms: u64,           // 1000ms
    pub max_indexing_time_per_doc_ms: u64, // 50ms
    pub max_memory_mb: u64,                // 1024MB
    pub max_cpu_idle_percent: f32,         // 5%
    pub max_cpu_load_percent: f32,         // 80%
    pub min_rps: f64,                      // 10 req/sec
    pub max_concurrent_users: u32,         // 100 users
    pub max_data_scale: u64,               // 1M documents
}
```

## Usage Examples

### Command Line Interface

```bash
# Run benchmarks with custom configuration
./scripts/run-performance-benchmarks.sh \
  --iterations=5000 \
  --baseline=benchmark-results/baseline.json \
  --verbose

# CI/CD integration
export TERRAPHIM_BENCH_ITERATIONS=2000
export TERRAPHIM_SERVER_URL=http://localhost:3000
./scripts/run-performance-benchmarks.sh
```

### Programmatic Usage

```rust
use terraphim_validation::performance::benchmarking::{PerformanceBenchmarker, BenchmarkConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create benchmark configuration
    let config = BenchmarkConfig::default();

    // Create benchmarker
    let mut benchmarker = PerformanceBenchmarker::new(config);

    // Load baseline for comparison
    if let Ok(baseline) = std::fs::read_to_string("baseline.json") {
        let baseline_report: BenchmarkReport = serde_json::from_str(&baseline)?;
        benchmarker.load_baseline(baseline_report);
    }

    // Run all benchmarks
    let report = benchmarker.run_all_benchmarks().await?;

    // Export results
    let json = benchmarker.export_json(&report)?;
    std::fs::write("results.json", json)?;

    let html = benchmarker.export_html(&report)?;
    std::fs::write("report.html", html)?;

    println!("SLO Compliance: {:.1}%", report.slo_compliance.overall_compliance);

    Ok(())
}
```

### CI/CD Integration

The framework integrates with GitHub Actions for automated performance validation:

```yaml
# .github/workflows/performance-benchmarking.yml
name: Performance Benchmarking

on:
  pull_request:
    paths:
      - 'crates/terraphim_*/src/**'
      - 'terraphim_server/src/**'

jobs:
  performance-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run performance benchmarks
        run: ./scripts/run-performance-benchmarks.sh
      - name: Check performance gates
        run: |
          # Check SLO compliance
          COMPLIANCE=$(jq -r '.slo_compliance.overall_compliance' benchmark-results/*/benchmark_results.json)
          if (( $(echo "$COMPLIANCE < 95.0" | bc -l) )); then
            echo "Performance requirements not met: ${COMPLIANCE}%"
            exit 1
          fi
```

## Results Analysis

### Performance Reports

The framework generates multiple report formats:

**HTML Dashboard** (`benchmark_report.html`)
- Interactive charts and graphs
- Detailed performance metrics
- Trend analysis visualizations
- SLO compliance dashboards

**JSON Data** (`benchmark_results.json`)
- Structured performance data
- Complete benchmark results
- System information
- Statistical analysis

**Markdown Summary** (`benchmark_summary.md`)
- Executive summary
- Key performance indicators
- SLO compliance status
- Recommendations

### Key Metrics

#### Response Time Metrics
- Average response time
- 95th percentile response time
- Minimum/Maximum response times
- Standard deviation

#### Throughput Metrics
- Operations per second
- Requests per second
- Data transfer rates
- Concurrent operation capacity

#### Resource Utilization
- CPU usage percentage
- Memory consumption (RSS/Virtual)
- Disk I/O operations
- Network bandwidth usage

#### Success Rate Metrics
- Operation success percentage
- Error rate analysis
- Failure pattern identification
- Recovery time measurement

## Troubleshooting

### Common Issues

**Server Not Accessible**
```bash
# Check if server is running
curl http://localhost:3000/health

# Start server manually
cargo run --package terraphim_server
```

**Permission Errors**
```bash
# Make script executable
chmod +x scripts/run-performance-benchmarks.sh
```

**Missing Dependencies**
```bash
# Install required tools
sudo apt-get install curl jq bc wrk

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**High Variance in Results**
- Run benchmarks multiple times
- Increase iteration count
- Check system load
- Isolate benchmarking environment

### Performance Tuning

**Benchmark Configuration**
```json
{
  "iterations": 5000,
  "warmup_iterations": 500,
  "monitoring_interval_ms": 500
}
```

**System Optimization**
```bash
# Disable CPU frequency scaling
sudo cpupower frequency-set -g performance

# Disable swap (if sufficient RAM)
sudo swapoff -a

# Optimize kernel parameters
echo "net.core.somaxconn=65535" | sudo tee -a /etc/sysctl.conf
```

## Contributing

### Adding New Benchmarks

1. **Define Benchmark Operation**
```rust
async fn benchmark_custom_operation(&mut self) -> Result<()> {
    // Implementation
    let result = BenchmarkResult { /* ... */ };
    self.results.insert("custom_operation".to_string(), result);
    Ok(())
}
```

2. **Add to Main Benchmark Runner**
```rust
async fn run_all_benchmarks(&mut self) -> Result<BenchmarkReport> {
    // ... existing benchmarks ...
    self.benchmark_custom_operation().await?;
    // ... rest of method ...
}
```

3. **Update Performance Gates**
```json
{
  "gates": [
    {
      "name": "Custom Operation",
      "metric": "custom_operation.avg_time_ms",
      "operator": "LessThan",
      "threshold": 100.0,
      "severity": "Warning"
    }
  ]
}
```

### Adding New Metrics

1. **Extend ResourceUsage struct**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    // ... existing fields ...
    pub custom_metric: f64,
}
```

2. **Implement Metric Collection**
```rust
async fn capture_resource_usage(&self) -> Result<ResourceUsage> {
    // ... existing collection ...
    let custom_metric = self.collect_custom_metric().await?;
    Ok(ResourceUsage {
        // ... existing fields ...
        custom_metric,
    })
}
```

## Success Criteria

The performance benchmarking framework is considered successful when:

- ✅ **95%+ performance coverage** for all critical operations
- ✅ **SLA compliance validation** with configurable thresholds
- ✅ **Regression detection** with automated alerts
- ✅ **Scalability validation** up to defined limits
- ✅ **Automated reporting** with historical trend analysis
- ✅ **CI/CD integration** with performance gates

## License

This performance benchmarking framework is part of Terraphim AI and follows the same license terms.</content>
<parameter name="filePath">PERFORMANCE_BENCHMARKING_README.md