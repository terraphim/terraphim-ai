//! Stress Testing Benchmarks
//!
//! Benchmarks for testing performance under heavy load:
//! - Large dataset handling
//! - Concurrent operations
//! - Memory pressure
//! - Long-running operations

use ahash::AHashMap;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lru::LruCache;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};
use terraphim_types::{ChatMessage, ContextItem, ConversationId, Document};
use tokio::sync::{Mutex, Semaphore};
use ulid::Ulid;

/// Benchmark large dataset operations
fn benchmark_large_datasets(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_datasets");

    // Large document processing
    for size in [10000, 50000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::new("large_document_processing", size),
            size,
            |b, &count| {
                b.iter(|| {
                    let mut documents = Vec::with_capacity(count);
                    for i in 0..count {
                        let doc = Document {
                            id: format!("doc_{}", i),
                            url: format!("https://example.com/doc_{}", i),
                            body: format!("Document {} content with machine learning, AI, programming, technology, data science, and neural networks information", i),
                            description: Some(format!("Description for document {}", i)),
                            tags: Some(vec![
                                "tech".to_string(),
                                "ai".to_string(),
                                "ml".to_string(),
                            ]),
                            rank: Some(i as f64 / count as f64),
                        };
                        documents.push(doc);
                    }

                    // Process documents
                    let mut tech_docs = 0;
                    let mut ai_docs = 0;
                    for doc in &documents {
                        if doc.body.contains("technology") {
                            tech_docs += 1;
                        }
                        if doc.body.contains("artificial intelligence") {
                            ai_docs += 1;
                        }
                    }

                    black_box((tech_docs, ai_docs));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("large_document_search", size),
            size,
            |b, &count| {
                let mut documents = Vec::with_capacity(count);
                for i in 0..count {
                    documents.push(Document {
                        id: format!("doc_{}", i),
                        url: format!("https://example.com/doc_{}", i),
                        body: format!("Document {} content with machine learning and AI", i),
                        description: Some(format!("Description {}", i)),
                        tags: Some(vec!["tech".to_string()]),
                        rank: Some(i as f64 / count as f64),
                    });
                }

                b.iter(|| {
                    let query = black_box("machine learning");
                    let results: Vec<_> = documents
                        .iter()
                        .filter(|doc| {
                            doc.body.contains(query)
                                || doc
                                    .description
                                    .as_ref()
                                    .map_or(false, |d| d.contains(query))
                        })
                        .collect();
                    black_box(results.len());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent operations
fn benchmark_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    // Concurrent document processing
    group.bench_function("concurrent_document_processing", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            let documents = (0..1000)
                .map(|i| Document {
                    id: format!("doc_{}", i),
                    url: format!("https://example.com/doc_{}", i),
                    body: format!("Document {} content", i),
                    description: Some(format!("Description {}", i)),
                    tags: Some(vec!["tech".to_string()]),
                    rank: Some(i as f64 / 1000.0),
                })
                .collect::<Vec<_>>();

            let semaphore = Arc::new(Semaphore::new(10));
            let mut handles = Vec::new();

            for doc in documents {
                let semaphore = Arc::clone(&semaphore);
                handles.push(tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    // Simulate processing
                    tokio::task::yield_now().await;

                    // Extract terms
                    let terms: Vec<&str> = doc.body.split_whitespace().take(5).collect();
                    terms.len()
                }));
            }

            let mut results = Vec::new();
            for handle in handles {
                results.push(handle.await.unwrap());
            }

            black_box(results.len());
        });
    });

    // Concurrent chat message handling
    group.bench_function("concurrent_chat_handling", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<ChatMessage>(100);
            let processed_count = Arc::new(AtomicUsize::new(0));

            // Spawn multiple producers
            let mut producers = Vec::new();
            for _ in 0..5 {
                let tx = tx.clone();
                producers.push(tokio::spawn(async move {
                    for i in 0..100 {
                        let msg = ChatMessage {
                            id: format!("msg_{}", i),
                            conversation_id: ConversationId::from(Ulid::new().to_string()),
                            role: "user".to_string(),
                            content: format!("Message {}", i),
                            timestamp: chrono::Utc::now(),
                            metadata: AHashMap::new(),
                        };
                        tx.send(msg).await.unwrap();
                    }
                }));
            }

            // Spawn consumer
            let processed_count = Arc::clone(&processed_count);
            let consumer = tokio::spawn(async move {
                let mut count = 0;
                while let Some(msg) = rx.recv().await {
                    // Simulate message processing
                    tokio::task::yield_now().await;
                    count += 1;
                }
                processed_count.store(count, Ordering::SeqCst);
                count
            });

            // Wait for all producers
            for producer in producers {
                producer.await.unwrap();
            }
            drop(tx); // Close channel

            let processed = consumer.await.unwrap();
            black_box(processed);
        });
    });

    // Concurrent search operations
    group.bench_function("concurrent_search_operations", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            let queries = vec![
                "machine learning",
                "artificial intelligence",
                "data science",
                "neural networks",
                "deep learning",
            ];

            let semaphore = Arc::new(Semaphore::new(5));
            let mut handles = Vec::new();

            for query in queries {
                let semaphore = Arc::clone(&semaphore);
                handles.push(tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    // Simulate search operation
                    tokio::time::sleep(Duration::from_millis(10)).await;

                    // Return fake results
                    (query.len(), 100)
                }));
            }

            let mut results = Vec::new();
            for handle in handles {
                results.push(handle.await.unwrap());
            }

            black_box(results.len());
        });
    });

    group.finish();
}

/// Benchmark memory pressure
fn benchmark_memory_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pressure");

    // Large allocations
    group.bench_function("large_allocations", |b| {
        b.iter(|| {
            let mut allocations = Vec::new();
            for _ in 0..black_box(100) {
                // Allocate 1MB
                let data = vec![0u8; 1024 * 1024];
                allocations.push(data);
            }

            // Use allocations to prevent optimization
            let total_size: usize = allocations.iter().map(|v| v.len()).sum();
            black_box(total_size);
        });
    });

    // Memory fragmentation simulation
    group.bench_function("memory_fragmentation", |b| {
        b.iter(|| {
            let mut allocations = Vec::new();
            let mut freed_indices = Vec::new();

            // Allocate
            for i in 0..1000 {
                let size = (i % 100) * 1024 + 1024; // Variable sizes
                allocations.push(vec![0u8; size]);
            }

            // Free every third allocation
            for i in (0..allocations.len()).step_by(3) {
                freed_indices.push(i);
            }

            for &idx in &freed_indices {
                allocations[idx].clear();
            }

            // Allocate again (simulating fragmentation)
            for i in 0..freed_indices.len() {
                let size = (i % 50) * 2048 + 2048;
                allocations.push(vec![0u8; size]);
            }

            black_box(allocations.len());
        });
    });

    // Cache pressure
    group.bench_function("cache_pressure", |b| {
        b.iter(|| {
            let mut cache = LruCache::new(1000);
            let mut evictions = 0;

            // Fill cache
            for i in 0..2000 {
                cache.put(i, format!("value_{}", i));
                if cache.len() < i + 1 {
                    evictions += 1;
                }
            }

            black_box((cache.len(), evictions));
        });
    });

    group.finish();
}

/// Benchmark long-running operations
fn benchmark_long_running(c: &mut Criterion) {
    let mut group = c.benchmark_group("long_running");

    // Extended processing
    for duration_ms in [1000, 5000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("extended_processing", duration_ms),
            duration_ms,
            |b, &ms| {
                b.iter(|| {
                    let start = Instant::now();
                    let mut count = 0;

                    while start.elapsed() < Duration::from_millis(ms) {
                        // Simulate work
                        for i in 0..1000 {
                            count = count.wrapping_add(i);
                            count = count.wrapping_mul(2);
                        }
                    }

                    black_box(count);
                });
            },
        );
    }

    // Batch processing
    group.bench_function("batch_processing", |b| {
        b.iter(|| {
            let batch_size = black_box(1000);
            let mut processed = 0;

            for batch_start in (0..10000).step_by(batch_size) {
                let batch_end = (batch_start + batch_size).min(10000);
                let mut batch_results = Vec::new();

                for i in batch_start..batch_end {
                    // Simulate processing
                    let result = i * 2 + 1;
                    batch_results.push(result);
                }

                // Process batch
                let sum: i32 = batch_results.iter().sum();
                processed += sum;
            }

            black_box(processed);
        });
    });

    // Continuous operation simulation
    group.bench_function("continuous_operations", |b| {
        b.iter(|| {
            let mut state = 0;
            let operations = black_box(10000);

            for _ in 0..operations {
                // Simulate continuous operation
                state = state.wrapping_add(1);
                if state > 1000 {
                    state = 0;
                }
            }

            black_box(state);
        });
    });

    group.finish();
}

/// Benchmark system resource contention
fn benchmark_resource_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_contention");

    // Lock contention
    group.bench_function("lock_contention", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            let mutex = Arc::new(Mutex::new(0));
            let mut handles = Vec::new();

            for _ in 0..10 {
                let mutex = Arc::clone(&mutex);
                handles.push(tokio::spawn(async move {
                    for _ in 0..1000 {
                        let mut val = mutex.lock().await;
                        *val += 1;
                    }
                }));
            }

            for handle in handles {
                handle.await.unwrap();
            }

            let final_val = *mutex.lock().await;
            black_box(final_val);
        });
    });

    // Channel contention
    group.bench_function("channel_contention", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(&rt).iter(|| async {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<i32>(10);
            let total_sent = Arc::new(AtomicUsize::new(0));
            let total_received = Arc::new(AtomicUsize::new(0));

            // Multiple senders
            let mut senders = Vec::new();
            for _ in 0..5 {
                let tx = tx.clone();
                let total_sent = Arc::clone(&total_sent);
                senders.push(tokio::spawn(async move {
                    for i in 0..100 {
                        tx.send(i).await.unwrap();
                        total_sent.fetch_add(1, Ordering::SeqCst);
                    }
                }));
            }

            // Single receiver
            let receiver = tokio::spawn(async move {
                let mut sum = 0;
                while let Some(msg) = rx.recv().await {
                    sum += msg;
                    total_received.fetch_add(1, Ordering::SeqCst);
                }
                sum
            });

            // Wait for all senders
            for sender in senders {
                sender.await.unwrap();
            }
            drop(tx);

            let final_sum = receiver.await.unwrap();
            black_box((
                final_sum,
                total_sent.load(Ordering::SeqCst),
                total_received.load(Ordering::SeqCst),
            ));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_large_datasets,
    benchmark_concurrent_operations,
    benchmark_memory_pressure,
    benchmark_long_running,
    benchmark_resource_contention
);
criterion_main!(benches);
