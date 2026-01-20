//! Quick Performance Test
//! Simple benchmark to verify performance characteristics

use std::time::{Duration, Instant};

fn main() {
    println!("🚀 GPUI Desktop Quick Performance Test");
    println!("=======================================\n");

    // Test 1: Basic Computation
    println!("⏱️  Test 1: Basic Computation (1M iterations)");
    let start = Instant::now();
    for i in 0..1_000_000 {
        let _ = i.wrapping_mul(2).wrapping_add(1);
    }
    let duration = start.elapsed();
    println!("   Duration: {:?}", duration);
    println!(
        "   Rate: {:.2} ops/sec",
        1_000_000.0 / duration.as_secs_f64()
    );
    println!(
        "   Status: {}",
        if duration.as_millis() < 100 {
            "✅ PASS"
        } else {
            "⚠️  SLOW"
        }
    );
    println!();

    // Test 2: String Operations
    println!("🔤 Test 2: String Operations (100K iterations)");
    let start = Instant::now();
    for _ in 0..100_000 {
        let s = "hello world terraphim ai".to_string();
        let _ = s.to_uppercase();
    }
    let duration = start.elapsed();
    println!("   Duration: {:?}", duration);
    println!("   Rate: {:.2} ops/sec", 100_000.0 / duration.as_secs_f64());
    println!(
        "   Status: {}",
        if duration.as_millis() < 500 {
            "✅ PASS"
        } else {
            "⚠️  SLOW"
        }
    );
    println!();

    // Test 3: Vector Operations
    println!("📦 Test 3: Vector Operations (10K iterations, 1K items each)");
    let start = Instant::now();
    for _ in 0..10_000 {
        let mut v = Vec::with_capacity(1000);
        for i in 0..1000 {
            v.push(i);
        }
        let _ = v.len();
    }
    let duration = start.elapsed();
    println!("   Duration: {:?}", duration);
    println!("   Rate: {:.2} ops/sec", 10_000.0 / duration.as_secs_f64());
    println!(
        "   Status: {}",
        if duration.as_millis() < 200 {
            "✅ PASS"
        } else {
            "⚠️  SLOW"
        }
    );
    println!();

    // Test 4: Hash Map Operations
    println!("🗂️  Test 4: Hash Map Operations (10K iterations, 100 items each)");
    let start = Instant::now();
    use std::collections::HashMap;
    for _ in 0..10_000 {
        let mut map = HashMap::with_capacity(100);
        for i in 0..100 {
            map.insert(format!("key_{}", i), format!("value_{}", i));
        }
        let _ = map.get("key_50");
    }
    let duration = start.elapsed();
    println!("   Duration: {:?}", duration);
    println!("   Rate: {:.2} ops/sec", 10_000.0 / duration.as_secs_f64());
    println!(
        "   Status: {}",
        if duration.as_millis() < 300 {
            "✅ PASS"
        } else {
            "⚠️  SLOW"
        }
    );
    println!();

    // Test 5: AHash Map Operations (faster hash)
    println!("⚡ Test 5: AHash Map Operations (10K iterations, 100 items each)");
    let start = Instant::now();
    for _ in 0..10_000 {
        let mut map = ahash::AHashMap::with_capacity(100);
        for i in 0..100 {
            map.insert(format!("key_{}", i), format!("value_{}", i));
        }
        let _ = map.get("key_50");
    }
    let duration = start.elapsed();
    println!("   Duration: {:?}", duration);
    println!("   Rate: {:.2} ops/sec", 10_000.0 / duration.as_secs_f64());
    println!(
        "   Status: {}",
        if duration.as_millis() < 200 {
            "✅ PASS"
        } else {
            "⚠️  SLOW"
        }
    );
    println!();

    // Test 6: Document Search Simulation
    println!("🔍 Test 6: Document Search (1K documents, filter operation)");
    let documents: Vec<terraphim_types::Document> = (0..1000)
        .map(|i| terraphim_types::Document {
            id: format!("doc_{}", i),
            url: format!("https://example.com/doc_{}", i),
            body: format!(
                "Document {} contains information about machine learning and AI",
                i
            ),
            description: Some(format!("Description for document {}", i)),
            tags: Some(vec!["tech".to_string(), "ai".to_string()]),
            rank: Some(i as f64 / 1000.0),
        })
        .collect();

    let start = Instant::now();
    let filtered: Vec<_> = documents
        .iter()
        .filter(|doc| doc.body.contains("machine learning"))
        .collect();
    let duration = start.elapsed();

    println!("   Duration: {:?}", duration);
    println!("   Found: {} matching documents", filtered.len());
    println!("   Rate: {:.2} searches/sec", 1.0 / duration.as_secs_f64());
    println!(
        "   Status: {}",
        if duration.as_millis() < 50 {
            "✅ PASS"
        } else {
            "⚠️  SLOW"
        }
    );
    println!();

    // Test 7: Chat Message Creation
    println!("💬 Test 7: Chat Message Creation (1K messages)");
    let start = Instant::now();
    for i in 0..1000 {
        let _message = terraphim_types::ChatMessage {
            id: format!("msg_{}", i),
            conversation_id: terraphim_types::ConversationId::from(ulid::Ulid::new().to_string()),
            role: if i % 2 == 0 {
                "user".to_string()
            } else {
                "assistant".to_string()
            },
            content: format!("This is message number {} in the conversation", i),
            timestamp: chrono::Utc::now(),
            metadata: ahash::AHashMap::new(),
        };
    }
    let duration = start.elapsed();
    println!("   Duration: {:?}", duration);
    println!(
        "   Rate: {:.2} messages/sec",
        1000.0 / duration.as_secs_f64()
    );
    println!(
        "   Status: {}",
        if duration.as_millis() < 100 {
            "✅ PASS"
        } else {
            "⚠️  SLOW"
        }
    );
    println!();

    // Test 8: Virtual Scrolling Calculation
    println!("📜 Test 8: Virtual Scrolling Calculation (10K iterations)");
    let start = Instant::now();
    for _ in 0..10_000 {
        let viewport_height = 800.0;
        let item_height = 60.0;
        let scroll_offset = 5000.0;
        let buffer_size = 5;
        let total_items = 10_000;

        let start_idx = (scroll_offset / item_height).floor() as usize;
        let visible_count = (viewport_height / item_height).ceil() as usize;
        let end_idx = (start_idx + visible_count + buffer_size).min(total_items);
        let _actual_start = start_idx.saturating_sub(buffer_size);

        let _ = (start_idx, end_idx);
    }
    let duration = start.elapsed();
    println!("   Duration: {:?}", duration);
    println!(
        "   Rate: {:.2} calculations/sec",
        10_000.0 / duration.as_secs_f64()
    );
    println!(
        "   Status: {}",
        if duration.as_millis() < 50 {
            "✅ PASS"
        } else {
            "⚠️  SLOW"
        }
    );
    println!();

    // Summary
    println!("📈 Performance Test Summary");
    println!("===========================");
    println!("✅ All tests completed successfully!");
    println!("");
    println!("Performance Targets:");
    println!("   - Basic Ops:      < 100ms   (1M iterations)");
    println!("   - String Ops:     < 500ms   (100K iterations)");
    println!("   - Vector Ops:     < 200ms   (10K iterations)");
    println!("   - Hash Map Ops:   < 300ms   (10K iterations)");
    println!("   - AHash Map Ops:  < 200ms   (10K iterations)");
    println!("   - Search:         < 50ms    (1K documents)");
    println!("   - Chat Messages:  < 100ms   (1K messages)");
    println!("   - Virtual Scroll: < 50ms    (10K iterations)");
    println!();
    println!("💡 Note: These are micro-benchmarks for basic operations.");
    println!("   For comprehensive benchmarks, use: cargo bench");
}
