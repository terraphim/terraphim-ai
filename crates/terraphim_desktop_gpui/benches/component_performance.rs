//! Component Performance Benchmarks
//!
//! Benchmarks for measuring component-level performance:
//! - Search operations
//! - Chat operations
//! - Virtual scrolling
//! - Context management

use ahash::AHashMap;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use lru::LruCache;
use std::sync::Arc;
use std::time::{Duration, Instant};
use terraphim_types::{ChatMessage, ContextItem, ContextType, ConversationId, Document};
use tokio::sync::Mutex;
use ulid::Ulid;

/// Benchmark search operations
fn benchmark_search_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_operations");

    // Document indexing
    for doc_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("document_indexing", doc_count),
            doc_count,
            |b, &count| {
                b.iter(|| {
                    let mut index = AHashMap::new();
                    for i in 0..count {
                        let doc = Document {
                            id: format!("doc_{}", i),
                            url: format!("https://example.com/doc_{}", i),
                            body: format!(
                                "Document {} content about machine learning, AI, and programming",
                                i
                            ),
                            description: Some(format!("Description {}", i)),
                            tags: Some(vec!["tech".to_string(), "ai".to_string()]),
                            rank: Some(i as f64 / count as f64),
                        };

                        // Index by terms
                        let terms: Vec<&str> = doc.body.split_whitespace().take(10).collect();
                        for term in terms {
                            index
                                .entry(term.to_lowercase())
                                .or_insert_with(Vec::new)
                                .push(doc.id.clone());
                        }

                        black_box(doc.id);
                    }
                    black_box(index.len());
                });
            },
        );
    }

    // Search query execution
    group.bench_function("search_query_execution", |b| {
        let documents = (0..1000)
            .map(|i| Document {
                id: format!("doc_{}", i),
                url: format!("https://example.com/doc_{}", i),
                body: format!(
                    "Document {} content about machine learning, AI, programming, and technology",
                    i
                ),
                description: Some(format!("Description {}", i)),
                tags: Some(vec!["tech".to_string(), "ai".to_string()]),
                rank: Some(i as f64 / 1000.0),
            })
            .collect::<Vec<_>>();

        let mut index = AHashMap::new();
        for doc in &documents {
            let terms: Vec<&str> = doc.body.split_whitespace().take(10).collect();
            for term in terms {
                index
                    .entry(term.to_lowercase())
                    .or_insert_with(Vec::new)
                    .push(doc.id.clone());
            }
        }

        b.iter(|| {
            let query = black_box("machine learning");
            let terms: Vec<&str> = query.split_whitespace().collect();

            let mut results = AHashMap::new();
            for term in terms {
                if let Some(doc_ids) = index.get(&term.to_lowercase()) {
                    for doc_id in doc_ids {
                        *results.entry(doc_id).or_insert(0) += 1;
                    }
                }
            }

            // Sort by relevance
            let mut sorted_results: Vec<_> = results.into_iter().collect();
            sorted_results.sort_by(|a, b| b.1.cmp(&a.1));

            black_box(sorted_results.len());
        });
    });

    // Autocomplete generation
    group.bench_function("autocomplete_generation", |b| {
        let terms = vec![
            "machine learning",
            "machine learning algorithms",
            "machine learning models",
            "artificial intelligence",
            "artificial neural networks",
            "deep learning",
            "neural networks",
            "computer vision",
            "natural language processing",
            "data science",
            "data analysis",
            "python programming",
        ];

        b.iter(|| {
            let query = black_box("machine");
            let suggestions: Vec<_> = terms
                .iter()
                .filter(|term| term.starts_with(&query))
                .map(|term| term.to_string())
                .collect();
            black_box(suggestions.len());
        });
    });

    group.finish();
}

/// Benchmark chat operations
fn benchmark_chat_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("chat_operations");

    // Message sending
    for msg_count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("message_sending", msg_count),
            msg_count,
            |b, &count| {
                b.iter(|| {
                    let mut messages = Vec::new();
                    for i in 0..count {
                        let message = ChatMessage {
                            id: format!("msg_{}", i),
                            conversation_id: ConversationId::from(Ulid::new().to_string()),
                            role: if i % 2 == 0 {
                                "user".to_string()
                            } else {
                                "assistant".to_string()
                            },
                            content: format!("This is message number {} with some content", i),
                            timestamp: chrono::Utc::now(),
                            metadata: AHashMap::new(),
                        };
                        messages.push(message);
                    }
                    black_box(messages.len());
                });
            },
        );
    }

    // Context injection
    group.bench_function("context_injection", |b| {
        let context_items = (0..50)
            .map(|i| ContextItem {
                id: format!("ctx_{}", i),
                context_type: ContextType::Document,
                title: format!("Context Item {}", i),
                summary: Some(format!("Summary for context item {}", i)),
                content: format!("This is the content of context item {}. It contains information about various topics.", i),
                metadata: {
                    let mut meta = AHashMap::new();
                    meta.insert("source".to_string(), format!("source_{}", i % 5));
                    meta
                },
                created_at: chrono::Utc::now(),
                relevance_score: Some(0.8),
            })
            .collect::<Vec<_>>();

        b.iter(|| {
            let mut context_content = String::from("=== CONTEXT ===\n");
            for (idx, item) in context_items.iter().enumerate() {
                context_content.push_str(&format!(
                    "{}. {}\n{}\n\n",
                    idx + 1,
                    item.title,
                    item.content
                ));
            }
            context_content.push_str("=== END CONTEXT ===\n");

            black_box(context_content.len());
        });
    });

    // Streaming simulation
    group.bench_function("streaming_simulation", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut chunks = Vec::new();
                let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);

                // Spawn producer
                let producer = tokio::spawn(async move {
                    for i in 0..100 {
                        let chunk = format!("This is chunk {} of the streaming response", i);
                        tx.send(chunk).await.unwrap();
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                });

                // Consume chunks
                while let Some(chunk) = rx.recv().await {
                    chunks.push(chunk);
                }

                producer.await.unwrap();
                black_box(chunks.len());
            });
        });
    });

    group.finish();
}

/// Benchmark virtual scrolling
fn benchmark_virtual_scrolling(c: &mut Criterion) {
    let mut group = c.benchmark_group("virtual_scrolling");

    for item_count in [1000, 10000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::new("visible_range_calculation", item_count),
            item_count,
            |b, &count| {
                b.iter(|| {
                    let viewport_height = black_box(800.0);
                    let item_height = black_box(60.0);
                    let scroll_offset = black_box(5000.0);
                    let buffer_size = black_box(5);

                    let start = (scroll_offset / item_height).floor() as usize;
                    let visible_count = (viewport_height / item_height).ceil() as usize;
                    let end = (start + visible_count + buffer_size).min(count);
                    let start = start.saturating_sub(buffer_size);

                    black_box((start, end));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("height_calculation", item_count),
            item_count,
            |b, &count| {
                b.iter(|| {
                    let mut heights = Vec::with_capacity(count);
                    for i in 0..count {
                        // Simulate variable height calculation
                        let base_height = 60.0;
                        let content_factor = (i % 100) as f32 / 100.0;
                        let height = base_height + (content_factor * 40.0);
                        heights.push(height);
                    }
                    black_box(heights.len());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("accumulated_height_calculation", item_count),
            item_count,
            |b, &count| {
                b.iter(|| {
                    let mut accumulated_heights = vec![0.0; count + 1];
                    for i in 0..count {
                        let height = 60.0 + ((i % 100) as f32 / 100.0) * 40.0;
                        accumulated_heights[i + 1] = accumulated_heights[i] + height;
                    }
                    black_box(accumulated_heights.len());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark context management
fn benchmark_context_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_management");

    // Context CRUD operations
    group.bench_function("context_item_creation", |b| {
        b.iter(|| {
            let item = ContextItem {
                id: Ulid::new().to_string(),
                context_type: ContextType::Document,
                title: "Test Context Item".to_string(),
                summary: Some("This is a test context item".to_string()),
                content: "This is the full content of the test context item with some details"
                    .to_string(),
                metadata: {
                    let mut meta = AHashMap::new();
                    meta.insert("source".to_string(), "test".to_string());
                    meta.insert("author".to_string(), "system".to_string());
                    meta
                },
                created_at: chrono::Utc::now(),
                relevance_score: Some(0.9),
            };
            black_box(item);
        });
    });

    // Conversation management
    group.bench_function("conversation_management", |b| {
        b.iter(|| {
            let mut conversations = AHashMap::new();
            for i in 0..black_box(100) {
                let conv_id = ConversationId::from(Ulid::new().to_string());
                let messages = (0..20)
                    .map(|j| ChatMessage {
                        id: format!("msg_{}_{}", i, j),
                        conversation_id: conv_id.clone(),
                        role: if j % 2 == 0 {
                            "user".to_string()
                        } else {
                            "assistant".to_string()
                        },
                        content: format!("Message {} in conversation {}", j, i),
                        timestamp: chrono::Utc::now(),
                        metadata: AHashMap::new(),
                    })
                    .collect::<Vec<_>>();

                conversations.insert(conv_id, messages);
            }
            black_box(conversations.len());
        });
    });

    // LRU cache operations
    group.bench_function("lru_cache_operations", |b| {
        b.iter(|| {
            let mut cache = LruCache::new(1000);
            for i in 0..black_box(2000) {
                cache.put(i, format!("value_{}", i));
            }
            black_box(cache.len());
        });
    });

    // Cache hit ratio simulation
    group.bench_function("cache_hit_ratio_simulation", |b| {
        b.iter(|| {
            let mut cache = LruCache::new(100);
            let mut hits = 0;
            let mut misses = 0;

            // Simulate access pattern
            for i in 0..black_box(1000) {
                // 80% chance of accessing recent items
                let key = if i % 5 != 0 { i % 200 } else { i };

                if cache.get(&key).is_some() {
                    hits += 1;
                } else {
                    cache.put(key, format!("value_{}", key));
                    misses += 1;
                }
            }

            black_box((hits, misses));
        });
    });

    group.finish();
}

/// Benchmark term chip operations
fn benchmark_term_chip_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("term_chip_operations");

    // Term extraction
    group.bench_function("term_extraction", |b| {
        let queries = vec![
            "machine learning AND neural networks",
            "artificial intelligence OR deep learning",
            "(computer vision AND image processing) NOT photography",
            "data science +python +pandas",
            "natural language processing AND (transformers OR BERT)",
        ];

        b.iter(|| {
            let query = black_box(queries[0]);
            let mut terms = Vec::new();
            let mut current_term = String::new();
            let mut in_quotes = false;
            let mut paren_depth = 0;

            for ch in query.chars() {
                match ch {
                    '"' => in_quotes = !in_quotes,
                    '(' => paren_depth += 1,
                    ')' => paren_depth = paren_depth.saturating_sub(1),
                    ' ' | '+' if !in_quotes && paren_depth == 0 => {
                        if !current_term.trim().is_empty() {
                            terms.push(current_term.trim().to_string());
                            current_term.clear();
                        }
                    }
                    _ => current_term.push(ch),
                }
            }

            if !current_term.trim().is_empty() {
                terms.push(current_term.trim().to_string());
            }

            black_box(terms.len());
        });
    });

    // Query parsing with operators
    group.bench_function("query_parsing_with_operators", |b| {
        let queries = vec![
            "A AND B OR C",
            "(A AND B) OR (C AND D)",
            "A NOT B",
            "A +B -C",
        ];

        b.iter(|| {
            let query = black_box(queries[0]);
            let mut tokens = Vec::new();
            let mut current = String::new();

            for ch in query.chars() {
                match ch {
                    ' ' => {
                        if !current.is_empty() {
                            tokens.push(current);
                            current = String::new();
                        }
                    }
                    _ => current.push(ch),
                }
            }

            if !current.is_empty() {
                tokens.push(current);
            }

            black_box(tokens.len());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_search_operations,
    benchmark_chat_operations,
    benchmark_virtual_scrolling,
    benchmark_context_management,
    benchmark_term_chip_operations
);
criterion_main!(benches);
