//! Benchmarks for terraphim_tinyclaw
//!
//! According to the design document, Phase 1 doesn't require formal benchmarks
//! since LLM calls dominate the performance profile. These benchmarks are
//! provided for future optimization work.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Benchmark session operations
fn bench_session_operations(c: &mut Criterion) {
    // Session operations are file I/O bound
    // Design target: Session load < 10ms, Session save < 5ms
    c.bench_function("session_load", |b| {
        b.iter(|| {
            // Placeholder for future implementation
            black_box(())
        })
    });

    c.bench_function("session_save", |b| {
        b.iter(|| {
            // Placeholder for future implementation
            black_box(())
        })
    });
}

/// Benchmark message bus operations
fn bench_message_bus(c: &mut Criterion) {
    // Design target: Bus routing latency < 1ms
    c.bench_function("bus_send_receive", |b| {
        b.iter(|| {
            // Placeholder for future implementation
            black_box(())
        })
    });
}

/// Benchmark tool execution
fn bench_tool_execution(c: &mut Criterion) {
    // Design target: Filesystem tool < 50ms
    c.bench_function("tool_filesystem", |b| {
        b.iter(|| {
            // Placeholder for future implementation
            black_box(())
        })
    });
}

criterion_group!(
    benches,
    bench_session_operations,
    bench_message_bus,
    bench_tool_execution
);
criterion_main!(benches);
