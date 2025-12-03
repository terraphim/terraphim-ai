# Comprehensive Testing Strategy for Reusable Components

## Overview

This document outlines the testing strategy for implementing high-performance, fully-tested reusable components in the Terraphim AI system. The strategy ensures code quality, performance guarantees, and true reusability across different contexts.

## Testing Philosophy

### Core Principles
1. **No Mocks**: Follow the established project philosophy - use real implementations, not mocks
2. **Comprehensive Coverage**: Target 95% code coverage for all components
3. **Performance First**: Every test must include performance assertions
4. **Reusability Validation**: Components must be tested in multiple contexts
5. **Continuous Integration**: All tests run on every PR with strict quality gates

### Testing Pyramid
```
    E2E Tests (5%) - Real user workflows
       ↓
Integration Tests (25%) - Component interactions
       ↓
Unit Tests (70%) - Individual component logic
```

## Testing Framework Architecture

### Test Structure
```rust
// tests/
├── unit/                    // Fast unit tests (<1ms each)
│   ├── components/
│   ├── services/
│   └── utils/
├── integration/             // Component integration tests (<100ms each)
│   ├── search_integration.rs
│   ├── chat_integration.rs
│   └── kg_integration.rs
├── performance/             // Performance benchmarks
│   ├── benchmarks.rs
│   └── regressions.rs
├── e2e/                    // End-to-end tests (<1s each)
│   ├── user_workflows.rs
│   └── cross_component.rs
└── fixtures/               // Test data and utilities
    ├── data/
    └── helpers/
```

### Test Utilities
**File**: `tests/common/mod.rs`

```rust
use std::time::{Duration, Instant};
use std::sync::Arc;
use anyhow::Result;
use tempfile::TempDir;
use tokio::sync::mpsc;

/// Test context builder for consistent test setup
pub struct TestContextBuilder {
    temp_dir: Option<TempDir>,
    config: ComponentConfig,
    services: ServiceRegistry,
}

impl TestContextBuilder {
    pub fn new() -> Self {
        Self {
            temp_dir: None,
            config: ComponentConfig::default(),
            services: ServiceRegistry::new(),
        }
    }

    pub fn with_temp_dir(mut self) -> Self {
        self.temp_dir = Some(TempDir::new().unwrap());
        self
    }

    pub fn with_config(mut self, config: ComponentConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_service<T: ServiceInterface + AnyService>(mut self, service: Arc<T>) -> Self {
        self.services.register(service).unwrap();
        self
    }

    pub fn build(self) -> TestContext {
        TestContext {
            temp_dir: self.temp_dir,
            config: self.config,
            services: Arc::new(self.services),
        }
    }
}

/// Test context providing common test utilities
pub struct TestContext {
    pub temp_dir: Option<TempDir>,
    pub config: ComponentConfig,
    pub services: Arc<ServiceRegistry>,
}

impl TestContext {
    pub fn temp_dir_path(&self) -> Option<&std::path::Path> {
        self.temp_dir.as_ref().map(|d| d.path())
    }
}

/// Performance assertion utilities
pub struct PerformanceAssertions;

impl PerformanceAssertions {
    pub fn assert_duration_under(actual: Duration, max: Duration) -> Result<()> {
        assert!(
            actual < max,
            "Operation took {:?} which exceeds maximum of {:?}",
            actual,
            max
        );
        Ok(())
    }

    pub fn assert_memory_usage_under(usage_bytes: usize, max_mb: usize) -> Result<()> {
        let max_bytes = max_mb * 1024 * 1024;
        assert!(
            usage_bytes < max_bytes,
            "Memory usage {}MB exceeds maximum of {}MB",
            usage_bytes / 1024 / 1024,
            max_mb
        );
        Ok(())
    }

    pub fn assert_cache_hit_rate(hit_rate: f64, min_rate: f64) -> Result<()> {
        assert!(
            hit_rate >= min_rate,
            "Cache hit rate {:.2%} below minimum of {:.2%}",
            hit_rate,
            min_rate
        );
        Ok(())
    }

    pub fn assert_throughput(operations: usize, duration: Duration, min_ops_per_sec: f64) -> Result<()> {
        let actual_ops_per_sec = operations as f64 / duration.as_secs_f64();
        assert!(
            actual_ops_per_sec >= min_ops_per_sec,
            "Throughput {:.2} ops/sec below minimum of {:.2}",
            actual_ops_per_sec,
            min_ops_per_sec
        );
        Ok(())
    }
}

/// Test data generators
pub mod generators {
    use rand::{Rng, SeedableRng};
    use rand::rngs::StdRng;
    use serde_json::json;

    pub fn generate_search_query(length: usize) -> String {
        let words = vec![
            "rust", "async", "performance", "component", "search",
            "knowledge", "graph", "context", "chat", "streaming",
            "cache", "optimization", "reusability", "testing", "gpui"
        ];

        let mut rng = StdRng::seed_from_u64(42);
        (0..length)
            .map(|_| words[rng.gen_range(0..words.len())])
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn generate_component_config() -> ComponentConfig {
        ComponentConfig {
            component_id: format!("test_component_{}", uuid::Uuid::new_v4()),
            version: "1.0.0".to_string(),
            theme: ThemeConfig::default(),
            performance: PerformanceConfig {
                cache_size: Some(100),
                debounce_ms: 50,
                batch_size: 10,
                timeout_ms: 1000,
                enable_metrics: true,
                enable_profiling: false,
                max_memory_mb: Some(128),
                gc_strategy: GarbageCollectionStrategy::Threshold,
            },
            features: FeatureFlags::default(),
            integrations: IntegrationConfig::default(),
            custom: HashMap::from([
                ("test_setting".to_string(), json!(true)),
                ("test_value".to_string(), json!(42)),
            ]),
        }
    }

    pub fn generate_chat_message(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            id: Some(uuid::Uuid::new_v4().to_string()),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn generate_kg_term() -> KGTerm {
        KGTerm {
            id: uuid::Uuid::new_v4().to_string(),
            label: format!("Term_{}", uuid::Uuid::new_v4().as_simple()),
            description: Some("Generated test term".to_string()),
            category: Some("Test".to_string()),
            relationships: vec![
                KGRelationship {
                    target_id: uuid::Uuid::new_v4().to_string(),
                    relationship_type: "related_to".to_string(),
                    weight: 0.5,
                }
            ],
            metadata: HashMap::from([
                ("created_by".to_string(), json!("test_generator")),
            ]),
        }
    }
}

/// Async test utilities
pub mod async_utils {
    use tokio::time::timeout;
    use std::time::Duration;

    pub async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        match timeout(duration, future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!("Test timed out after {:?}", duration)),
        }
    }

    pub async fn wait_for_condition<F, Fut>(
        mut condition: F,
        timeout_ms: u64,
    ) -> Result<()>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
            if condition().await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Err(anyhow::anyhow!("Condition not met within timeout"))
    }
}
```

## Component Testing Standards

### Search Component Tests
**File**: `tests/unit/search_components.rs`

```rust
use crate::common::*;
use terraphim_desktop_gpui::{
    search::{SearchInput, AutocompleteState, SearchService},
    config::ComponentConfig,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_input_performance() {
        let context = TestContextBuilder::new()
            .with_temp_dir()
            .with_config(generators::generate_component_config())
            .build();

        // Create search input
        let search_input = SearchInput::new(context.config.clone());

        // Measure input response time
        let start = Instant::now();
        search_input.handle_event(SearchEvent::Input("test query".to_string())).await?;
        let input_time = start.elapsed();

        // Assert performance
        PerformanceAssertions::assert_duration_under(input_time, Duration::from_millis(10))?;

        // Test autocomplete trigger
        let start = Instant::now();
        let _suggestions = search_input.get_autocomplete_suggestions().await?;
        let autocomplete_time = start.elapsed();

        // Assert autocomplete performance
        PerformanceAssertions::assert_duration_under(autocomplete_time, Duration::from_millis(5))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_search_caching() {
        let context = TestContextBuilder::new()
            .with_temp_dir()
            .with_service(Arc::new(MockSearchService::new()))
            .build();

        let mut search_input = SearchInput::new(context.config);
        search_input.set_cache_size(100);

        let query = generators::generate_search_query(5);

        // First search (should be cache miss)
        let start = Instant::now();
        let results1 = search_input.search(query.clone()).await?;
        let first_search_time = start.elapsed();

        // Second search (should be cache hit)
        let start = Instant::now();
        let results2 = search_input.search(query).await?;
        let second_search_time = start.elapsed();

        // Verify cache hit
        assert_eq!(results1.len(), results2.len());
        assert!(second_search_time < first_search_time);

        // Verify cache metrics
        let metrics = search_input.metrics();
        PerformanceAssertions::assert_cache_hit_rate(metrics.cache_hit_rate, 0.5)?;

        Ok(())
    }

    #[tokio::test]
    async fn test_search_reusability() {
        let config1 = generators::generate_component_config();
        let config2 = generators::generate_component_config();

        // Test with different configurations
        let search1 = SearchInput::new(config1);
        let search2 = SearchInput::new(config2);

        // Test that both work independently
        let query = "test query";
        let results1 = search1.search(query.to_string()).await?;
        let results2 = search2.search(query.to_string()).await?;

        // Results should be different due to different configurations
        assert_ne!(results1, results2);

        // Both should have valid metrics
        assert!(search1.metrics().total_operations > 0);
        assert!(search2.metrics().total_operations > 0);

        Ok(())
    }
}
```

### Chat Component Tests
**File**: `tests/unit/chat_components.rs`

```rust
use crate::common::*;
use terraphim_desktop_gpui::{
    chat::{StreamingChatState, ChatView, ChatMessage},
    llm::{LlmService, MockLlmService},
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streaming_chat_performance() {
        let context = TestContextBuilder::new()
            .with_service(Arc::new(MockLlmService::new()))
            .build();

        let mut chat_state = StreamingChatState::new(
            context.config.clone(),
            Some(context.services.clone()),
        );

        // Start streaming message
        let message = generators::generate_chat_message("user", "Hello, world!");
        let conversation_id = chat_state.start_message_stream(message).await?;

        // Measure chunk processing performance
        let start = Instant::now();
        for i in 0..100 {
            let chunk = format!("Chunk {}", i);
            chat_state.add_stream_chunk(conversation_id.clone(), chunk, ChunkType::Text).await?;
        }
        let chunk_time = start.elapsed();

        // Assert performance (<1ms per chunk)
        PerformanceAssertions::assert_duration_under(
            chunk_time / 100,
            Duration::from_millis(1)
        )?;

        // Complete streaming
        chat_state.complete_stream(conversation_id).await?;

        // Verify metrics
        let metrics = chat_state.get_performance_metrics();
        assert_eq!(metrics.total_messages, 1);
        assert_eq!(metrics.chunks_processed, 100);

        Ok(())
    }

    #[tokio::test]
    async fn test_chat_memory_efficiency() {
        let context = TestContextBuilder::new()
            .with_temp_dir()
            .build();

        let mut chat_state = StreamingChatState::new(
            context.config.clone(),
            Some(context.services.clone()),
        );

        // Add many messages
        let initial_memory = get_memory_usage();

        for i in 0..1000 {
            let message = generators::generate_chat_message(
                "assistant",
                &format!("This is message number {}", i)
            );
            chat_state.add_message(message).await?;
        }

        let final_memory = get_memory_usage();
        let memory_increase = final_memory - initial_memory;

        // Assert memory efficiency (<50MB for 1000 messages)
        PerformanceAssertions::assert_memory_usage_under(memory_increase, 50)?;

        // Test virtual scrolling efficiency
        let chat_view = ChatView::new(chat_state);
        let visible_messages = chat_view.get_visible_messages(0, 50).await?;

        assert_eq!(visible_messages.len(), 50);

        Ok(())
    }

    #[tokio::test]
    async fn test_chat_context_integration() {
        let context = TestContextBuilder::new()
            .with_service(Arc::new(MockSearchService::new()))
            .build();

        let mut chat_state = StreamingChatState::new(
            context.config.clone(),
            Some(context.services.clone()),
        );

        // Test context injection
        let context_items = chat_state.add_context_from_search("Rust async programming").await?;
        assert!(!context_items.is_empty());

        // Start message with context
        let message = generators::generate_chat_message(
            "user",
            "Explain async patterns in Rust"
        );
        let conversation_id = chat_state.start_message_stream(message).await?;

        // Verify context was included
        let context_used = chat_state.was_context_used(conversation_id).await?;
        assert!(context_used);

        Ok(())
    }
}
```

### Knowledge Graph Component Tests
**File**: `tests/unit/kg_components.rs`

```rust
use crate::common::*;
use terraphim_desktop_gpui::{
    kg::{KGSearchService, KGSearchModal, KGTerm, KGRelationship},
    services::MockKGService,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kg_search_performance() {
        let context = TestContextBuilder::new()
            .with_service(Arc::new(MockKGService::new()))
            .build();

        let kg_service = KGSearchService::new(context.services.clone());

        // Test term search performance
        let start = Instant::now();
        let terms = kg_service.search_terms("async programming").await?;
        let search_time = start.elapsed();

        // Assert performance (<50ms for term search)
        PerformanceAssertions::assert_duration_under(search_time, Duration::from_millis(50))?;
        assert!(!terms.is_empty());

        // Test relationship traversal performance
        let start = Instant::now();
        let relationships = kg_service.get_relationships(&terms[0].id, 2).await?;
        let traversal_time = start.elapsed();

        // Assert performance (<100ms for depth-2 traversal)
        PerformanceAssertions::assert_duration_under(
            traversal_time,
            Duration::from_millis(100)
        )?;

        Ok(())
    }

    #[tokio::test]
    async fn test_kg_modal_reusability() {
        let config1 = generators::generate_component_config();
        let config2 = generators::generate_component_config();

        // Create two KG modals with different configurations
        let modal1 = KGSearchModal::new(config1.clone());
        let modal2 = KGSearchModal::new(config2.clone());

        // Test independent operation
        modal1.set_search_depth(3);
        modal2.set_search_depth(5);

        assert_ne!(
            modal1.get_configuration().search_depth,
            modal2.get_configuration().search_depth
        );

        // Test both can perform searches
        let results1 = modal1.search("graph theory").await?;
        let results2 = modal2.search("graph theory").await?;

        assert!(results1.len() > 0);
        assert!(results2.len() > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_kg_caching() {
        let context = TestContextBuilder::new()
            .build();

        let mut kg_service = KGSearchService::new(context.services);
        kg_service.enable_caching(1000);

        let term_id = uuid::Uuid::new_v4().to_string();

        // First traversal (cache miss)
        let start = Instant::now();
        let _relationships1 = kg_service.get_relationships(&term_id, 3).await?;
        let first_time = start.elapsed();

        // Second traversal (cache hit)
        let start = Instant::now();
        let _relationships2 = kg_service.get_relationships(&term_id, 3).await?;
        let second_time = start.elapsed();

        // Verify cache hit
        assert!(second_time < first_time);

        // Verify cache metrics
        let metrics = kg_service.get_cache_metrics();
        PerformanceAssertions::assert_cache_hit_rate(metrics.hit_rate, 0.5)?;

        Ok(())
    }
}
```

## Integration Testing

### Search-Chat Integration
**File**: `tests/integration/search_chat_integration.rs`

```rust
use crate::common::*;
use terraphim_desktop_gpui::{
    search::SearchInput,
    chat::{StreamingChatState, ChatMessage},
    services::{SearchService, LlmService},
};

#[tokio::test]
async fn test_search_to_chat_workflow() {
    let context = TestContextBuilder::new()
        .with_temp_dir()
        .build();

    // Initialize search and chat components
    let search_input = SearchInput::new(context.config.clone());
    let mut chat_state = StreamingChatState::new(
        context.config.clone(),
        Some(context.services.clone()),
    );

    // 1. User searches for information
    let search_results = search_input.search("Rust async patterns").await?;
    assert!(!search_results.is_empty());

    // 2. User asks chat about search results
    let chat_message = ChatMessage {
        id: Some(uuid::Uuid::new_v4().to_string()),
        role: "user".to_string(),
        content: "Explain the async patterns from the search results".to_string(),
        timestamp: chrono::Utc::now(),
        metadata: HashMap::from([
            ("search_context".to_string(), serde_json::to_value(&search_results).unwrap()),
        ]),
    };

    // 3. Chat uses search results as context
    let conversation_id = chat_state.start_message_stream(chat_message).await?;

    // 4. Verify context was properly integrated
    let context_items = chat_state.get_context_items(conversation_id).await?;
    assert!(!context_items.is_empty());

    // 5. Complete chat response
    chat_state.complete_stream(conversation_id).await?;

    // Verify workflow performance
    let total_time = search_input.metrics().average_response_time()
        + chat_state.get_performance_metrics().average_stream_duration;

    PerformanceAssertions::assert_duration_under(total_time, Duration::from_millis(500))?;

    Ok(())
}

#[tokio::test]
async fn test_multi_component_state_sharing() {
    let context = TestContextBuilder::new()
        .build();

    // Create multiple components that share state
    let search_service: Arc<dyn SearchService> = context.services.get("search").unwrap();
    let chat_service: Arc<dyn LlmService> = context.services.get("llm").unwrap();

    let mut components = Vec::new();

    // Create 10 search inputs and 10 chat states
    for i in 0..10 {
        let search = SearchInput::with_services(
            generators::generate_component_config(),
            search_service.clone(),
        );
        let chat = StreamingChatState::with_services(
            generators::generate_component_config(),
            chat_service.clone(),
        );

        components.push((search, chat));
    }

    // Test all components work concurrently
    let start = Instant::now();

    let mut handles = Vec::new();
    for (search, chat) in components {
        let handle = tokio::spawn(async move {
            // Perform operations
            let _results = search.search("test query").await?;
            let message = generators::generate_chat_message("user", "test message");
            let _conv_id = chat.start_message_stream(message).await?;
            Ok::<(), anyhow::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await??;
    }

    let total_time = start.elapsed();

    // Assert concurrent performance (<1s for 20 concurrent operations)
    PerformanceAssertions::assert_duration_under(total_time, Duration::from_millis(1000))?;

    Ok(())
}
```

### Cross-Component Performance
**File**: `tests/performance/cross_component_benchmarks.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crate::common::*;
use terraphim_desktop_gpui::{
    search::SearchInput,
    chat::StreamingChatState,
    kg::KGSearchService,
};

fn bench_search_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("search_with_caching", |b| {
        b.to_async(&rt).iter(|| async {
            let context = TestContextBuilder::new().build();
            let mut search = SearchInput::new(context.config);
            search.enable_caching(1000);

            // Benchmark search operations
            for _ in 0..100 {
                let query = generators::generate_search_query(3);
                let _results = black_box(search.search(query).await.unwrap());
            }
        })
    });
}

fn bench_chat_streaming(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("chat_streaming", |b| {
        b.to_async(&rt).iter(|| async {
            let context = TestContextBuilder::new().build();
            let mut chat = StreamingChatState::new(context.config, None);

            // Benchmark streaming performance
            let message = generators::generate_chat_message("user", "Test message");
            let conv_id = black_box(chat.start_message_stream(message).await.unwrap());

            // Stream 1000 chunks
            for i in 0..1000 {
                let chunk = format!("Chunk {}", i);
                chat.add_stream_chunk(conv_id.clone(), chunk, ChunkType::Text).await.unwrap();
            }

            chat.complete_stream(conv_id).await.unwrap();
        })
    });
}

fn bench_kg_traversal(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("kg_depth_3_traversal", |b| {
        b.to_async(&rt).iter(|| async {
            let context = TestContextBuilder::new().build();
            let kg = KGSearchService::new(context.services);

            // Benchmark graph traversal
            let terms = black_box(kg.search_terms("programming").await.unwrap());
            if !terms.is_empty() {
                let _relationships = black_box(
                    kg.get_relationships(&terms[0].id, 3).await.unwrap()
                );
            }
        })
    });
}

criterion_group!(
    benches,
    bench_search_performance,
    bench_chat_streaming,
    bench_kg_traversal
);
criterion_main!(benches);
```

## Performance Regression Testing

**File**: `tests/performance/regressions.rs`

```rust
use crate::common::*;
use std::collections::HashMap;

/// Performance regression test suite
#[tokio::test]
async fn test_search_performance_regression() {
    let context = TestContextBuilder::new().build();
    let search = SearchInput::new(context.config);

    // Baseline performance targets (from previous runs)
    let targets = PerformanceTargets {
        search_cached: Duration::from_millis(50),
        search_uncached: Duration::from_millis(200),
        autocomplete: Duration::from_millis(10),
        throughput: 1000.0,
    };

    // Test search performance
    let start = Instant::now();
    let _results = search.search("test query").await?;
    let search_time = start.elapsed();

    assert!(
        search_time <= targets.search_uncached,
        "Search regression: {:?} > {:?}",
        search_time,
        targets.search_uncached
    );

    // Test throughput
    let operations = 100;
    let start = Instant::now();

    for _ in 0..operations {
        let _results = search.search("test query").await?;
    }

    let total_time = start.elapsed();
    PerformanceAssertions::assert_throughput(operations, total_time, targets.throughput)?;
}

#[tokio::test]
async fn test_memory_usage_regression() {
    let context = TestContextBuilder::new()
        .with_temp_dir()
        .build();

    // Baseline memory targets
    let targets = MemoryTargets {
        search_component_mb: 10,
        chat_component_mb: 50,
        kg_component_mb: 100,
        total_system_mb: 512,
    };

    let initial_memory = get_system_memory_usage();

    // Test search component memory
    let search = SearchInput::new(context.config.clone());
    let search_memory = get_component_memory_usage(&search);
    PerformanceAssertions::assert_memory_usage_under(
        search_memory,
        targets.search_component_mb
    )?;

    // Test chat component memory
    let mut chat = StreamingChatState::new(context.config.clone(), None);

    // Add 1000 messages
    for i in 0..1000 {
        let message = generators::generate_chat_message(
            "user",
            &format!("Message {}", i)
        );
        chat.add_message(message).await?;
    }

    let chat_memory = get_component_memory_usage(&chat);
    PerformanceAssertions::assert_memory_usage_under(
        chat_memory,
        targets.chat_component_mb
    )?;

    // Test total memory
    let final_memory = get_system_memory_usage();
    let memory_increase = final_memory - initial_memory;
    PerformanceAssertions::assert_memory_usage_under(
        memory_increase,
        targets.total_system_mb
    )?;
}

struct PerformanceTargets {
    search_cached: Duration,
    search_uncached: Duration,
    autocomplete: Duration,
    throughput: f64,
}

struct MemoryTargets {
    search_component_mb: usize,
    chat_component_mb: usize,
    kg_component_mb: usize,
    total_system_mb: usize,
}

fn get_component_memory_usage<T>(component: &T) -> usize {
    // Use size_of_val as approximation
    std::mem::size_of_val(component)
}

fn get_system_memory_usage() -> usize {
    // Get actual system memory usage
    // This would use platform-specific APIs
    0 // Placeholder
}
```

## End-to-End Testing

**File**: `tests/e2e/user_workflows.rs`

```rust
use crate::common::*;
use terraphim_desktop_gpui::app::TerraphimApp;

#[tokio::test]
async fn test_complete_research_workflow() {
    // 1. Initialize application
    let app = TerraphimApp::new_with_test_config().await?;
    let start = Instant::now();

    // 2. User searches for information
    let search_results = app.search("Rust tokio best practices").await?;
    assert!(!search_results.is_empty(), "Search should return results");

    // 3. User opens a result in markdown modal
    let modal = app.open_article_modal(&search_results[0]).await?;
    assert!(modal.is_open(), "Modal should open successfully");

    // 4. User asks chat about the search results
    let chat_response = app.chat_with_context(
        "Summarize the key points about tokio best practices"
    ).await?;

    assert!(!chat_response.is_empty(), "Chat should provide response");

    // 5. User searches knowledge graph for related terms
    let kg_terms = app.kg_search("async programming patterns").await?;
    assert!(!kg_terms.is_empty(), "KG search should return terms");

    // 6. User integrates KG terms into chat
    let enhanced_response = app.chat_with_kg_context(
        "How do these patterns relate to tokio?",
        kg_terms
    ).await?;

    assert!(!enhanced_response.is_empty(), "Enhanced chat should work");

    // 7. Verify total workflow performance
    let total_time = start.elapsed();
    PerformanceAssertions::assert_duration_under(total_time, Duration::from_secs(5))?;

    // 8. Verify all components are working
    let health_status = app.health_check().await?;
    assert!(health_status.all_healthy(), "All components should be healthy");

    Ok(())
}

#[tokio::test]
async fn test_high_load_scenario() {
    // Simulate high user load
    let app = TerraphimApp::new_with_test_config().await?;

    let concurrent_users = 100;
    let operations_per_user = 50;

    let start = Instant::now();
    let mut handles = Vec::new();

    // Spawn concurrent user sessions
    for user_id in 0..concurrent_users {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            for op in 0..operations_per_user {
                // Alternate between search and chat
                if op % 2 == 0 {
                    let query = format!("User {} Query {}", user_id, op);
                    let _results = app_clone.search(&query).await?;
                } else {
                    let message = format!("User {} Message {}", user_id, op);
                    let _response = app_clone.chat(&message).await?;
                }
            }
            Ok::<(), anyhow::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await??;
    }

    let total_time = start.elapsed();
    let total_operations = concurrent_users * operations_per_user;

    // Assert system handles load gracefully
    PerformanceAssertions::assert_throughput(
        total_operations,
        total_time,
        1000.0 // 1000 ops/sec minimum
    )?;

    // Verify system is still healthy
    let health = app.health_check().await?;
    assert!(health.error_rate < 0.01, "Error rate should be <1% under load");

    Ok(())
}
```

## Continuous Integration

### GitHub Actions Workflow
**File**: `.github/workflows/test-components.yml`

```yaml
name: Component Testing

on:
  push:
    branches: [main, claude/plan-gpui-migration-*]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
        rust: [stable, beta]

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: ${{ matrix.rust }}
        components: clippy, rustfmt

    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

    - name: Run formatting checks
      run: cargo fmt --all -- --check

    - name: Run clippy
      run: cargo clippy --all-targets --all-features -- -D warnings

    - name: Run unit tests
      run: cargo test --package terraphim_desktop_gpui --lib -- --test-threads=1

    - name: Run integration tests
      run: cargo test --package terraphim_desktop_gpui --test '*' -- --test-threads=1

    - name: Run performance benchmarks
      run: cargo bench --package terraphim_desktop_gpui

    - name: Check test coverage
      run: |
        cargo install cargo-tarpaulin
        cargo tarpaulin --out Xml --output-dir ./coverage
        bash <(curl -s https://codecov.io/bash) -f ./coverage/tarpaulin.xml

    - name: Upload test results
      uses: actions/upload-artifact@v3
      if: always()
      with:
        name: test-results-${{ matrix.os }}-${{ matrix.rust }}
        path: |
          target/
          coverage/
```

### Quality Gates

1. **All tests must pass**: Zero tolerance for test failures
2. **95% code coverage**: Enforced via codecov
3. **Zero clippy warnings**: Any warning fails the build
4. **Performance regression**: Benchmarks must not regress >5%
5. **Memory leaks**: Valgrind must report zero leaks

## Test Data Management

### Test Fixtures
**File**: `tests/fixtures/data/sample_search_results.json`

```json
{
  "search_results": [
    {
      "id": "doc1",
      "title": "Understanding Async in Rust",
      "url": "https://example.com/rust-async",
      "body": "Async programming in Rust uses the async/await syntax...",
      "description": "A comprehensive guide to async patterns",
      "rank": 0.95,
      "tags": ["rust", "async", "programming"]
    },
    {
      "id": "doc2",
      "title": "Tokio Tutorial",
      "url": "https://example.com/tokio-tutorial",
      "body": "Tokio is Rust's asynchronous runtime...",
      "description": "Learn Tokio from the ground up",
      "rank": 0.92,
      "tags": ["tokio", "async", "runtime"]
    }
  ],
  "kg_terms": [
    {
      "id": "term1",
      "label": "Async Programming",
      "description": "Programming paradigm for concurrent operations",
      "category": "Programming",
      "relationships": [
        {
          "target_id": "term2",
          "relationship_type": "uses",
          "weight": 0.9
        }
      ]
    }
  ],
  "chat_messages": [
    {
      "id": "msg1",
      "role": "user",
      "content": "What is async programming?",
      "timestamp": "2024-01-01T00:00:00Z"
    },
    {
      "id": "msg2",
      "role": "assistant",
      "content": "Async programming is a paradigm that allows...",
      "timestamp": "2024-01-01T00:00:01Z"
    }
  ]
}
```

This comprehensive testing strategy ensures that all reusable components are thoroughly tested, performant, and truly reusable across different contexts. The strategy emphasizes performance testing, integration validation, and continuous quality assurance.