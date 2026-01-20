//! Core Performance Benchmarks
//!
//! Benchmarks for measuring fundamental performance metrics:
//! - Startup time
//! - Memory usage
//! - Response time
//! - Rendering performance

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use gpui::{Application, Context, Entity, Window};
use std::time::{Duration, Instant};
use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_desktop_gpui::{ConfigState, TerraphimApp};
use tokio::runtime::Runtime;

/// Benchmark startup time
fn benchmark_startup_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("startup_time");

    group.bench_function("gpu_app_initialization", |b| {
        b.iter(|| {
            // Measure application initialization time
            let start = Instant::now();

            // Create tokio runtime (required for terraphim_service)
            let runtime = Runtime::new().unwrap();

            // Initialize configuration
            let config_state = runtime.block_on(async {
                let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
                    .build()
                    .unwrap();

                if let Err(_) = config.load().await {
                    config = ConfigBuilder::new().build_default_desktop().unwrap();
                }

                terraphim_config::ConfigState::new(&mut config)
                    .await
                    .unwrap()
            });

            let _guard = runtime.enter();

            // Initialize GPUI application (lightweight version)
            let _app = Application::new();

            let startup_time = start.elapsed();
            black_box(startup_time);
        });
    });

    group.bench_function("config_loading", |b| {
        b.iter(|| {
            let runtime = Runtime::new().unwrap();
            let start = Instant::now();

            let _config_state = runtime.block_on(async {
                let mut config = ConfigBuilder::new_with_id(ConfigId::Desktop)
                    .build()
                    .unwrap();

                if let Err(_) = config.load().await {
                    config = ConfigBuilder::new().build_default_desktop().unwrap();
                }

                terraphim_config::ConfigState::new(&mut config)
                    .await
                    .unwrap()
            });

            black_box(start.elapsed());
        });
    });

    group.finish();
}

/// Benchmark memory allocation patterns
fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    // Test different allocation sizes
    for size in [64, 256, 1024, 4096, 16384].iter() {
        group.bench_with_input(
            BenchmarkId::new("vec_allocation", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut vec = Vec::with_capacity(size);
                    for i in 0..size {
                        vec.push(black_box(i));
                    }
                    black_box(vec.len());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("string_concatenation", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut s = String::new();
                    for _ in 0..size {
                        s.push_str(black_box("x"));
                    }
                    black_box(s.len());
                });
            },
        );
    }

    group.bench_function("hashmap_operations", |b| {
        b.iter(|| {
            let mut map = ahash::AHashMap::new();
            for i in 0..black_box(1000) {
                map.insert(format!("key_{}", i), format!("value_{}", i));
            }
            black_box(map.len());
        });
    });

    group.bench_function("cache_operations", |b| {
        b.iter(|| {
            let mut cache = lru::LruCache::new(100);
            for i in 0..black_box(1000) {
                cache.put(i, format!("value_{}", i));
            }
            black_box(cache.len());
        });
    });

    group.finish();
}

/// Benchmark response time for common operations
fn benchmark_response_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_time");

    // Search query processing
    group.bench_function("query_parsing", |b| {
        b.iter(|| {
            let query = black_box("machine learning AND (neural networks OR deep learning)");
            let mut terms = Vec::new();
            let mut current_term = String::new();
            let mut in_quotes = false;

            for ch in query.chars() {
                match ch {
                    '"' => in_quotes = !in_quotes,
                    ' ' if !in_quotes => {
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

    // Document filtering
    group.bench_function("document_filtering", |b| {
        let documents = (0..1000)
            .map(|i| terraphim_types::Document {
                id: format!("doc_{}", i),
                url: format!("https://example.com/doc_{}", i),
                body: format!(
                    "This is document number {} with some content about machine learning and AI",
                    i
                ),
                description: Some(format!("Description for document {}", i)),
                tags: Some(vec!["tech".to_string(), "ai".to_string()]),
                rank: Some(i as f64 / 1000.0),
            })
            .collect::<Vec<_>>();

        b.iter(|| {
            let filtered: Vec<_> = documents
                .iter()
                .filter(|doc| {
                    doc.body.contains("machine learning")
                        || doc
                            .description
                            .as_ref()
                            .map_or(false, |d| d.contains("machine learning"))
                })
                .collect();
            black_box(filtered.len());
        });
    });

    // Term chip operations
    group.bench_function("term_chip_parsing", |b| {
        b.iter(|| {
            let query = black_box("machine learning AND neural networks OR deep learning");
            let mut chips = Vec::new();
            let mut current_chip = String::new();
            let mut in_quotes = false;

            for ch in query.chars() {
                match ch {
                    '"' => in_quotes = !in_quotes,
                    ' ' if !in_quotes => {
                        if !current_chip.trim().is_empty() {
                            chips.push(current_chip.trim().to_string());
                            current_chip.clear();
                        }
                    }
                    _ => current_chip.push(ch),
                }
            }

            if !current_chip.trim().is_empty() {
                chips.push(current_chip.trim().to_string());
            }

            black_box(chips.len());
        });
    });

    group.finish();
}

/// Benchmark rendering operations
fn benchmark_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("rendering");

    // Component rendering simulation
    for element_count in [10, 100, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::new("element_rendering", element_count),
            element_count,
            |b, &count| {
                b.iter(|| {
                    // Simulate rendering elements
                    let mut elements = Vec::with_capacity(count);
                    for i in 0..count {
                        elements.push(format!(
                            "<div class=\"element\" data-id=\"{}\">Content {}</div>",
                            i, i
                        ));
                    }
                    black_box(elements.len());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("virtual_scroll_calculation", element_count),
            element_count,
            |b, &count| {
                b.iter(|| {
                    let viewport_height = black_box(800.0);
                    let item_height = black_box(60.0);
                    let scroll_offset = black_box(1000.0);

                    let start = (scroll_offset / item_height).floor() as usize;
                    let visible_count = (viewport_height / item_height).ceil() as usize;
                    let end = (start + visible_count).min(count);

                    black_box((start, end));
                });
            },
        );
    }

    // Markdown rendering
    group.bench_function("markdown_parsing", |b| {
        let markdown = r#"# Title

This is a paragraph with **bold** and *italic* text.

## Section 1

- Item 1
- Item 2
- Item 3

### Subsection

More content here with [a link](https://example.com) and code `inline`.

```rust
fn hello() {
    println!("Hello, world!");
}
```
"#;

        b.iter(|| {
            use pulldown_cmark::{Options, Parser, html};
            let mut options = Options::empty();
            options.insert(Options::ENABLE_TABLES);
            options.insert(Options::ENABLE_FOOTNOTES);
            options.insert(Options::ENABLE_STRIKETHROUGH);
            options.insert(Options::ENABLE_TASKLISTS);

            let parser = Parser::new_ext(markdown, options);
            let html = html::push_html(String::new(), parser);
            black_box(html.len());
        });
    });

    group.finish();
}

/// Benchmark async operations
fn benchmark_async_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_operations");

    // Concurrent tasks
    group.bench_function("task_spawning", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut handles = Vec::new();
                for i in 0..black_box(100) {
                    handles.push(tokio::spawn(async move {
                        tokio::task::yield_now().await;
                        i * 2
                    }));
                }

                let mut results = Vec::new();
                for handle in handles {
                    results.push(handle.await.unwrap());
                }
                black_box(results.len());
            });
        });
    });

    // Channel operations
    group.bench_function("channel_communication", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (tx, mut rx) = tokio::sync::mpsc::channel::<i32>(100);

                // Spawn sender task
                let sender = tokio::spawn(async move {
                    for i in 0..black_box(1000) {
                        tx.send(i).await.unwrap();
                    }
                });

                // Receive all messages
                let mut count = 0;
                while let Some(_msg) = rx.recv().await {
                    count += 1;
                }

                sender.await.unwrap();
                black_box(count);
            });
        });
    });

    // Shared state operations
    group.bench_function("mutex_operations", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let counter = Arc::new(tokio::sync::Mutex::new(0));
                let mut handles = Vec::new();

                for _ in 0..black_box(10) {
                    let counter = Arc::clone(&counter);
                    handles.push(tokio::spawn(async move {
                        for _ in 0..100 {
                            let mut c = counter.lock().await;
                            *c += 1;
                        }
                    }));
                }

                for handle in handles {
                    handle.await.unwrap();
                }

                let final_count = *counter.lock().await;
                black_box(final_count);
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_startup_time,
    benchmark_memory_usage,
    benchmark_response_time,
    benchmark_rendering,
    benchmark_async_operations
);
criterion_main!(benches);
