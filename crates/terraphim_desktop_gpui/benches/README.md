# GPUI Desktop Performance Benchmarks

Comprehensive benchmark suite for measuring and validating GPUI Desktop application performance.

## Overview

This benchmark suite measures key performance metrics for the GPUI desktop implementation:

### Core Performance Metrics
- **Startup Time**: Target < 1.5s (vs Tauri 2.3s)
- **Memory Usage**: Target < 130MB (vs Tauri 175MB)
- **Response Time**: Target < 100ms (vs Tauri 150ms)
- **Rendering FPS**: Target > 50 FPS (vs Tauri 28 FPS)
- **Binary Size**: Target < 20MB (vs Tauri 52MB)

## Benchmark Categories

### 1. Core Performance (`core_performance.rs`)
- Startup time measurement
- Memory allocation patterns
- Response time for common operations
- Rendering performance
- Async operation throughput

### 2. Component Performance (`component_performance.rs`)
- Search operations (query execution, indexing)
- Chat operations (message handling, streaming)
- Virtual scrolling (range calculation, height calculation)
- Context management (CRUD operations, LRU caching)
- Term chip operations (parsing, extraction)

### 3. Stress Tests (`stress_tests.rs`)
- Large dataset handling (10K+ documents)
- Concurrent operations (multi-threaded load)
- Memory pressure (allocation stress)
- Long-running operations (sustained load)
- Resource contention (lock/mutex contention)

## Running Benchmarks

### Quick Benchmarks
```bash
# Run all benchmarks with default settings
cargo bench

# Run specific benchmark category
cargo bench --bench core_performance
cargo bench --bench component_performance
cargo bench --bench stress_tests

# Run with custom configuration
cargo bench -- --quick  # Fewer iterations
cargo bench -- --release  # Release mode (slower but more accurate)
```

### Main Benchmark Runner
```bash
# Run comprehensive benchmark suite with detailed reporting
cargo bench --bench main

# Save results to file
cargo bench --bench main -- --save-results

# Run with custom iterations
cargo bench --bench main -- --iterations 200
```

### Benchmark Options
```bash
# Filter benchmarks by name
cargo bench -- search_operations

# Output benchmark results
cargo bench -- --output-format json

# Plot results (requires gnuplot)
cargo bench -- --plot
```

## Benchmark Configuration

Edit `benches/main.rs` to customize:

```rust
let config = BenchmarkConfig {
    iterations: 100,           // Number of iterations per benchmark
    warmup_iterations: 10,     // Warmup iterations (ignored in measurements)
    measurement_time: Duration::from_secs(10),  // Minimum measurement time
    save_results: true,        // Save results to JSON
    results_file: "benchmark_results.json".to_string(),
    // ... other options
};
```

## Performance Targets

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Startup Time | < 1500ms | Application initialization |
| Memory Usage | < 130MB | Runtime memory footprint |
| Response Time | < 100ms | Query processing latency |
| Rendering FPS | > 50 FPS | Element rendering throughput |
| Binary Size | < 20MB | Release binary size |

## Interpreting Results

### Benchmark Output
```
🚀 GPUI Desktop Performance Benchmark Suite
============================================

📊 Configuration:
   Iterations: 100
   Warmup iterations: 10
   Measurement time: 10s

🔬 Running Core Performance Benchmarks...
   ⏱️  Measuring startup time...
   💾 Measuring memory usage...
   ⚡ Measuring response time...
   🎨 Measuring rendering performance...
   🔄 Measuring async operations...

📈 Performance Summary
======================

Total Benchmarks: 50
Passed: 50 ✅
Failed: 0 ❌

🎯 Key Performance Metrics:
   Startup Time:       1200.50ms (Target: < 1500ms)
   Memory Usage:       95.2MB
   Response Time:      45.3ms (Target: < 100ms)
   Rendering FPS:      65.4 (Target: > 50)
   Search Throughput:  1250.5 ops/sec
   Chat Throughput:    890.3 ops/sec

🎯 Performance Targets Status:
   Startup Time:       ✅ PASS
   Response Time:      ✅ PASS
   Rendering FPS:      ✅ PASS
```

### JSON Results
Results are saved to `benchmark_results.json`:
```json
{
  "total_benchmarks": 50,
  "passed_benchmarks": 50,
  "performance_metrics": {
    "startup_time_ms": 1200.5,
    "memory_usage_mb": 95.2,
    "response_time_ms": 45.3,
    "rendering_fps": 65.4
  }
}
```

## Regression Detection

The benchmark suite includes regression detection:

- **Baseline Comparison**: Compare against saved baselines
- **Trend Analysis**: Track performance over time
- **Alert Thresholds**: Automatic failure on performance regressions
- **Statistical Significance**: Verify changes are real, not noise

### Setting Baselines
```bash
# Run benchmarks and save as baseline
cargo bench --bench main -- --save-baseline

# Compare against existing baseline
cargo bench --bench main -- --compare-baseline
```

## Continuous Integration

Add to `.github/workflows/ci.yml`:

```yaml
- name: Run Performance Benchmarks
  run: |
    cargo bench --bench main -- --quick --compare-baseline

- name: Upload Benchmark Results
  uses: actions/upload-artifact@v3
  with:
    name: benchmark-results
    path: benchmark_results.json
```

## Adding New Benchmarks

### 1. Create Benchmark Function
```rust
fn benchmark_my_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_category");

    group.bench_function("my_operation", |b| {
        b.iter(|| {
            // Your benchmark code here
            let result = my_operation();
            black_box(result);
        });
    });

    group.finish();
}
```

### 2. Register in Main
```rust
criterion_group!(
    benches,
    benchmark_my_feature,
    // ... other benchmarks
);
criterion_main!(benches);
```

### 3. Add to Main Runner
```rust
fn run_component_benchmarks(config: &BenchmarkConfig) -> Vec<BenchmarkResult> {
    // ...
    println!("   🔧 Measuring my feature...");
    results.extend(benchmark_my_feature(config));
    // ...
}
```

## Best Practices

### Writing Benchmarks
1. **Use `black_box()`**: Prevent compiler optimization
2. **Warmup Iterations**: Allow JIT/optimizer to warm up
3. **Statistical Rigor**: Use sufficient iterations (100+)
4. **Realistic Data**: Use production-like data sizes
5. **Clean Setup**: Clean up between benchmark runs

### Example Benchmark
```rust
fn benchmark_efficient_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    // Setup: Create test data
    let data = create_test_dataset(10000);

    group.bench_function("hashmap_lookup", |b| {
        b.iter(|| {
            let result = data.get(&black_box("search_term"));
            black_box(result)
        });
    });

    group.bench_function("linear_search", |b| {
        b.iter(|| {
            let result = data.iter()
                .find(|(k, _)| k == &black_box("search_term"));
            black_box(result)
        });
    });

    group.finish();
}
```

## Troubleshooting

### Benchmarks Running Too Slowly
```bash
# Use quick mode for development
cargo bench -- --quick

# Run only one benchmark
cargo bench -- my_specific_benchmark

# Reduce iterations
cargo bench -- --iterations 10
```

### Results Are Too Variable
1. Increase measurement time
2. Run on quieter system
3. Close background applications
4. Use more iterations

### Memory Benchmarks Show Inconsistent Results
1. Run with `--release` flag
2. Ensure system has enough RAM
3. Close other applications
4. Use larger allocation sizes

## Performance Optimization Guide

When benchmarks show poor performance:

1. **Profile First**: Use `perf` or Instruments
2. **Identify Bottlenecks**: Focus on slowest benchmarks
3. **Measure Impact**: Before/after optimization
4. **Document Changes**: Track optimization history

### Common Optimizations
- **Memory**: Use `Vec::with_capacity()`, avoid allocations
- **CPU**: Cache expensive calculations, use faster algorithms
- **Rendering**: Minimize DOM updates, use virtual scrolling
- **Async**: Reduce contention, use appropriate concurrency primitives

## References

- [Criterion Documentation](https://bheisler.github.io/criterion.rs/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [GPUI Framework](https://github.com/zed-industries/gpui)
- [Tokio Performance](https://tokio.rs/blog/2021-12-07-tokio-performance)
