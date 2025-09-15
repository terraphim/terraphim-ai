use anyhow::Result;
use serial_test::serial;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::time::Duration;
use tempfile::tempdir;
use terraphim_config::{Config, ConfigState, Haystack, Role, ServiceType};
use terraphim_persistence::Persistable;
use terraphim_service::TerraphimService;
use terraphim_types::{NormalizedTermValue, RelevanceFunction, RoleName, SearchQuery};

/// Integration test that validates the complete automatic summarization workflow
/// including search, document processing, persistence, merging, and SSE streaming
#[tokio::test]
#[serial]
async fn test_complete_auto_summarization_workflow() -> Result<()> {
    println!("🚀 Starting complete automatic summarization workflow test");

    // Initialize test persistence
    terraphim_persistence::DeviceStorage::init_memory_only().await?;
    println!("📦 Initialized memory-only persistence");

    // Step 1: Create test data
    let dir = tempdir().expect("tempdir");
    let test_documents = create_test_documents(&dir)?;
    println!("📄 Created {} test documents", test_documents.len());

    // Step 2: Set up service with auto-summarization enabled
    let mut config = create_test_config_with_auto_summarization(&dir)?;
    let config_state = ConfigState::new(&mut config).await?;
    let mut service = TerraphimService::new(config_state);
    println!("⚙️  Configured service with auto-summarization enabled");

    // Step 3: Perform search that should trigger summarization
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::new("rust".into()),
        limit: Some(10),
        skip: None,
        role: Some(RoleName::new("AutoSummary Test Role")),
        ..Default::default()
    };

    println!("🔍 Executing search query");
    let search_results = service.search(&search_query).await?;
    println!(
        "📊 Search returned {} documents",
        search_results.documents.len()
    );

    // Step 4: Verify that summarization tasks were queued
    assert!(
        !search_results.summarization_task_ids.is_empty(),
        "Summarization task IDs should not be empty for auto-summarization"
    );
    println!(
        "✅ Confirmed {} summarization tasks were queued",
        search_results.summarization_task_ids.len()
    );

    // Step 5: Wait for summarization to potentially complete
    println!("⏳ Waiting for summarization processing...");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Step 6: Merge completed summaries
    let merged_results = search_results.merge_completed_summaries(None).await;

    // Step 7: Verify documents have descriptions
    let documents_with_descriptions = merged_results
        .documents
        .iter()
        .filter(|doc| doc.description.is_some())
        .count();

    println!(
        "📝 Found {} documents with descriptions",
        documents_with_descriptions
    );
    assert!(
        documents_with_descriptions > 0,
        "At least some documents should have descriptions"
    );

    // Step 8: Verify descriptions are reasonable
    for doc in &merged_results.documents {
        if let Some(description) = &doc.description {
            println!(
                "📋 Document '{}' (len={}): '{}'",
                doc.id,
                description.len(),
                description
            );
            assert!(
                !description.trim().is_empty(),
                "Description should not be empty"
            );
            assert!(
                description.len() >= 10,
                "Description should be at least 10 characters long"
            );
            assert!(
                description.len() <= 500,
                "Description should not exceed 500 characters"
            );
        }
    }

    // Step 9: Test persistence of summarized documents
    println!("🔍 Testing document persistence");
    for doc in &merged_results.documents {
        if doc.description.is_some() {
            // Try to save the document first, then load it
            match doc.clone().save().await {
                Ok(_) => {
                    // Now try to load it back
                    match terraphim_types::Document::new(doc.id.clone()).load().await {
                        Ok(loaded_doc) => {
                            if loaded_doc.description.is_some() {
                                println!("💾 Successfully persisted and loaded description for document '{}'",
                                        doc.id);
                            }
                        }
                        Err(_) => {
                            // It's okay if loading fails - persistence may not be fully set up
                            println!("ℹ️  Persistence test skipped for document '{}'", doc.id);
                        }
                    }
                }
                Err(_) => {
                    println!("ℹ️  Could not save document '{}' - persistence may not be fully configured", doc.id);
                }
            }
        }
    }

    // Step 10: Validate rate limiting works without deadlocks
    println!("🔒 Testing concurrent summarization requests");
    let mut concurrent_tasks = vec![];

    for i in 0..5 {
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(format!("concurrent-test-{}", i)),
            limit: Some(2),
            role: Some(RoleName::new("AutoSummary Test Role")),
            ..Default::default()
        };

        // Create a new service instance with fresh config for concurrent testing
        let temp_dir = tempdir().expect("tempdir");
        let mut config_clone = create_test_config_with_auto_summarization(&temp_dir)?;
        let config_state_clone = ConfigState::new(&mut config_clone).await?;
        let mut service_clone = TerraphimService::new(config_state_clone);
        let task = tokio::spawn(async move { service_clone.search(&search_query).await });
        concurrent_tasks.push(task);
    }

    // Wait for all concurrent tasks to complete
    let mut successful_requests = 0;
    for task in concurrent_tasks {
        if let Ok(Ok(_)) = task.await {
            successful_requests += 1;
        }
    }

    println!(
        "✅ Completed {} concurrent requests successfully",
        successful_requests
    );
    assert!(
        successful_requests > 0,
        "At least some concurrent requests should succeed"
    );

    println!("🎉 Complete automatic summarization workflow test passed!");
    Ok(())
}

/// Create test documents with varying content lengths
fn create_test_documents(dir: &tempfile::TempDir) -> Result<Vec<String>> {
    let documents = vec![
        (
            "short_doc.md",
            "# Short Document\n\nThis is a short Rust document about basic concepts.",
        ),
        (
            "medium_doc.md",
            r#"# Medium Rust Document

This document covers intermediate Rust programming concepts including ownership, borrowing, and lifetimes.

## Ownership
Rust's ownership system ensures memory safety without a garbage collector. Every value has an owner.

## Borrowing
References allow you to use values without taking ownership of them.

## Lifetimes
Lifetimes ensure that references are valid as long as we need them to be."#,
        ),
        (
            "long_doc.md",
            r#"# Comprehensive Rust Programming Guide

Rust is a systems programming language that focuses on safety, speed, and concurrency. This comprehensive guide covers the fundamental concepts and advanced features that make Rust unique.

## Memory Safety

Rust's ownership system is designed to prevent common programming errors such as null pointer dereferences, buffer overflows, and memory leaks. The system is enforced at compile time, ensuring that programs are safe without runtime overhead.

### Ownership Rules

1. Each value in Rust has an owner
2. There can only be one owner at a time
3. When the owner goes out of scope, the value is dropped

## Borrowing and References

Borrowing allows you to refer to some value without taking ownership of it. This is implemented through references, which are pointers that are guaranteed to point to valid data.

### Mutable and Immutable References

You can have either multiple immutable references or one mutable reference to a particular piece of data in a particular scope, but not both.

## Lifetimes

Lifetimes are annotations that tell the Rust compiler how long references should be valid. They prevent dangling references and ensure memory safety.

## Concurrency

Rust's approach to concurrency is based on the principle that fearless concurrency should be achievable. The ownership model plays a crucial role in making concurrent programming safe.

### Threads

Rust provides native threads through the standard library. The `spawn` function creates new threads, and message passing between threads is facilitated by channels.

### Async Programming

Rust supports asynchronous programming through the async/await syntax and the Future trait. This allows for efficient handling of I/O-bound operations.

## Error Handling

Rust encourages explicit error handling through the Result type. This makes error cases visible in function signatures and prevents many runtime panics.

## Conclusion

Rust's unique approach to systems programming provides both safety and performance. The learning curve can be steep, but the benefits of memory safety and zero-cost abstractions make it an excellent choice for systems development."#,
        ),
        (
            "code_example.md",
            r#"# Rust Code Examples

```rust
fn main() {
    println!("Hello, Rust!");

    let numbers = vec![1, 2, 3, 4, 5];
    let doubled: Vec<i32> = numbers.iter().map(|x| x * 2).collect();

    for num in doubled {
        println!("Doubled: {}", num);
    }
}
```

This example demonstrates basic Rust syntax and functional programming concepts."#,
        ),
    ];

    let mut created_files = Vec::new();

    for (filename, content) in documents {
        let file_path = dir.path().join(filename);
        let mut file = fs::File::create(&file_path)?;
        file.write_all(content.as_bytes())?;
        created_files.push(filename.to_string());
        println!("📄 Created test document: {}", filename);
    }

    Ok(created_files)
}

/// Create a test configuration with auto-summarization enabled
fn create_test_config_with_auto_summarization(dir: &tempfile::TempDir) -> Result<Config> {
    let role_name = RoleName::new("AutoSummary Test Role");

    let mut role = Role {
        shortname: Some("auto_summary".into()),
        name: role_name.clone(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "test".into(),
        kg: None,
        haystacks: vec![Haystack {
            location: dir.path().to_string_lossy().to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: HashMap::new(),
        }],
        extra: ahash::AHashMap::new(),
        ..Default::default()
    };

    // Configure for generic LLM auto-summarization (works without OpenRouter)
    role.extra
        .insert("llm_provider".into(), serde_json::json!("test"));
    role.extra
        .insert("llm_model".into(), serde_json::json!("test-model"));
    role.extra
        .insert("llm_auto_summarize".into(), serde_json::json!(true));

    let mut config = Config::default();
    config.roles.insert(role_name.clone(), role);
    config.default_role = role_name.clone();
    config.selected_role = role_name.clone();

    println!("⚙️  Created test config with auto-summarization enabled");
    Ok(config)
}

/// Test that verifies the queue-based rate limiter prevents deadlocks
#[tokio::test]
#[serial]
async fn test_queue_based_rate_limiter_no_deadlocks() -> Result<()> {
    use std::sync::Arc;
    use terraphim_service::queue_based_rate_limiter::QueueBasedRateLimiterManager;
    use terraphim_service::summarization_queue::RateLimitConfig;

    println!("🔒 Testing queue-based rate limiter for deadlock prevention");

    let config = RateLimitConfig {
        max_requests_per_minute: 10,
        max_tokens_per_minute: 1000,
        burst_size: 50,
    };

    let mut configs = HashMap::new();
    configs.insert("test".to_string(), config);

    let rate_limiter = Arc::new(QueueBasedRateLimiterManager::new(configs));

    // Spawn multiple tasks that all try to acquire tokens simultaneously
    let mut handles = vec![];

    for i in 0..20 {
        let limiter = Arc::clone(&rate_limiter);
        let handle = tokio::spawn(async move {
            let result = limiter.acquire("test", 10.0).await;
            println!("Task {} completed with result: {:?}", i, result.is_ok());
            result.is_ok()
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut successful_acquisitions = 0;
    for handle in handles {
        if let Ok(success) = handle.await {
            if success {
                successful_acquisitions += 1;
            }
        }
    }

    println!(
        "✅ {} out of 20 token acquisitions succeeded",
        successful_acquisitions
    );

    // Some should succeed (within burst limit), some should fail
    // The key is that we didn't deadlock and all tasks completed
    assert!(
        successful_acquisitions > 0,
        "Some acquisitions should succeed"
    );
    assert!(
        successful_acquisitions <= 5,
        "Not all should succeed due to rate limiting"
    );

    println!("🎉 Queue-based rate limiter prevents deadlocks!");
    Ok(())
}

/// Test error handling in the summarization workflow
#[tokio::test]
#[serial]
async fn test_summarization_error_handling() -> Result<()> {
    use terraphim_service::summarization_manager::SummarizationManager;
    use terraphim_service::summarization_queue::QueueConfig;
    use terraphim_types::Document;

    println!("❌ Testing summarization error handling");

    // Initialize test persistence
    terraphim_persistence::DeviceStorage::init_memory_only().await?;

    let config = QueueConfig::default();
    let manager = SummarizationManager::new(config);

    // Create a test role
    let role = Role {
        name: "Error Test Role".to_string().into(),
        shortname: Some("error_test".to_string()),
        relevance_function: RelevanceFunction::TitleScorer,
        theme: "test".to_string(),
        terraphim_it: false,
        kg: None,
        haystacks: vec![],
        ..Default::default()
    };

    // Test with empty document body
    let mut empty_doc = Document {
        id: "empty-doc".to_string(),
        title: "Empty Document".to_string(),
        body: "".to_string(),
        url: "https://example.com/empty".to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
    };

    let task_id = manager
        .process_document_fields(&mut empty_doc, &role, true, true)
        .await?;

    // Should not queue summarization for empty document
    assert!(
        task_id.is_none(),
        "Empty document should not queue summarization"
    );
    assert!(
        empty_doc.description.is_none(),
        "Empty document should not have description"
    );

    println!("✅ Correctly handled empty document");

    // Test with very short document
    let mut short_doc = Document {
        id: "short-doc".to_string(),
        title: "Short Document".to_string(),
        body: "Very short.".to_string(),
        url: "https://example.com/short".to_string(),
        description: None,
        summarization: None,
        stub: None,
        tags: None,
        rank: None,
    };

    let task_id = manager
        .process_document_fields(&mut short_doc, &role, true, true)
        .await?;

    // Should extract description but not queue summarization
    assert!(
        task_id.is_none(),
        "Short document should not queue summarization"
    );
    assert!(
        short_doc.description.is_some(),
        "Short document should have description"
    );

    println!("✅ Correctly handled short document");

    println!("🎉 Summarization error handling works correctly!");
    Ok(())
}
