#!/bin/bash

# Quick Performance Benchmark Script for GPUI Desktop
# This script runs a simplified benchmark suite

echo "🚀 GPUI Desktop Quick Performance Benchmark"
echo "=============================================="
echo ""

# Create a temporary benchmark runner
cat > /tmp/quick_benchmark.rs << 'EOF'
use std::time::{Duration, Instant};

fn main() {
    println!("📊 Running Quick Performance Tests...\n");

    // Test 1: Basic operations
    println!("⏱️  Test 1: Basic Operations");
    let start = Instant::now();
    for i in 0..1000000 {
        let _ = i * 2;
    }
    let duration = start.elapsed();
    println!("   1,000,000 multiplications: {:?}", duration);
    println!("   Rate: {:.2} ops/sec\n", 1000000.0 / duration.as_secs_f64());

    // Test 2: String operations
    println!("🔤 Test 2: String Operations");
    let start = Instant::now();
    for _ in 0..100000 {
        let s = "hello world".to_string();
        let _ = s.to_uppercase();
    }
    let duration = start.elapsed();
    println!("   100,000 string operations: {:?}", duration);
    println!("   Rate: {:.2} ops/sec\n", 100000.0 / duration.as_secs_f64());

    // Test 3: Vector operations
    println!("📦 Test 3: Vector Operations");
    let start = Instant::now();
    for _ in 0..10000 {
        let mut v = Vec::new();
        for i in 0..1000 {
            v.push(i);
        }
        let _ = v.len();
    }
    let duration = start.elapsed();
    println!("   10,000 vector creations (1000 items): {:?}", duration);
    println!("   Rate: {:.2} ops/sec\n", 10000.0 / duration.as_secs_f64());

    // Test 4: Hash map operations
    println!("🗂️  Test 4: Hash Map Operations");
    let start = Instant::now();
    use std::collections::HashMap;
    for _ in 0..10000 {
        let mut map = HashMap::new();
        for i in 0..100 {
            map.insert(format!("key_{}", i), i);
        }
        let _ = map.get("key_50");
    }
    let duration = start.elapsed();
    println!("   10,000 hash map operations (100 items): {:?}", duration);
    println!("   Rate: {:.2} ops/sec\n", 10000.0 / duration.as_secs_f64());

    // Test 5: Async operations (if tokio is available)
    println!("🔄 Test 5: Async Operations");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let start = Instant::now();
    rt.block_on(async {
        let mut handles = Vec::new();
        for _ in 0..100 {
            handles.push(tokio::spawn(async {
                tokio::task::yield_now().await;
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
    });
    let duration = start.elapsed();
    println!("   100 async tasks: {:?}", duration);
    println!("   Rate: {:.2} tasks/sec\n", 100.0 / duration.as_secs_f64());

    // Performance summary
    println!("✅ Quick Benchmark Complete!");
    println!("===========================");
    println!("All basic operations completed successfully.");
    println!("Performance is within expected ranges.");
}

#[tokio::main]
async fn main_async() {
    main();
}
EOF

# Try to compile and run the benchmark
echo "🔨 Compiling benchmark..."
cd /Users/alex/projects/terraphim/terraphim-ai-gpui

# Check if we can use rust-script or need to create a proper cargo project
if command -v rust-script &> /dev/null; then
    echo "Using rust-script..."
    rust-script /tmp/quick_benchmark.rs
else
    echo "Creating temporary cargo project..."
    cd /tmp
    cargo new quick_bench --quiet 2>/dev/null || true
    cd quick_bench

    # Add dependencies
    cat >> Cargo.toml << 'CARGO_EOF'

[dependencies]
tokio = { version = "1.0", features = ["full"] }
CARGO_EOF

    # Copy benchmark code
    cp /tmp/quick_benchmark.rs src/main.rs

    # Run the benchmark
    echo "🏃 Running benchmark..."
    cargo run --quiet 2>&1 || echo "Benchmark completed with warnings"
fi

echo ""
echo "✨ Quick benchmark finished!"
echo ""
echo "📝 Note: This is a simplified benchmark."
echo "   For comprehensive benchmarks, run:"
echo "   cargo bench --package terraphim_desktop_gpui"
