use anyhow::Result;
use std::fs;
use std::io::Write;
use std::time::Duration;
use tempfile::tempdir;
use terraphim_config::{Config, ConfigState, Haystack, Role, ServiceType};
use terraphim_service::TerraphimService;
use terraphim_types::{NormalizedTermValue, RelevanceFunction, RoleName, SearchQuery};

/// Test that ACTUALLY calls Ollama and generates real AI summaries
/// This test waits for real LLM responses and shows the actual generated content
#[tokio::test]
async fn test_real_ollama_summarization_integration() -> Result<()> {
    println!("🔥 REAL OLLAMA INTEGRATION TEST");
    println!("===============================");

    // Skip if OLLAMA_TEST is not set (for CI environments)
    if std::env::var("OLLAMA_TEST").is_err() {
        println!("⚠️  Skipping Ollama test - set OLLAMA_TEST=1 to run");
        return Ok(());
    }

    // Initialize persistence
    terraphim_persistence::DeviceStorage::init_memory_only().await?;

    // Create a substantial document that will trigger summarization
    let temp_dir = tempdir()?;
    let doc_path = temp_dir.path().join("rust_advanced_guide.md");

    let substantial_content = r#"# Advanced Rust Programming Techniques

Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety. This comprehensive guide explores advanced Rust concepts and techniques for building high-performance applications.

## Memory Management and Ownership

The ownership system in Rust is fundamental to its memory safety guarantees. Every value has a single owner, and when the owner goes out of scope, the value is automatically dropped. This eliminates the need for manual memory management or garbage collection.

Key ownership rules:
- Each value has exactly one owner
- When the owner goes out of scope, the value is dropped
- You can transfer ownership by moving values
- References allow borrowing without taking ownership

## Advanced Type System Features

Rust's type system includes powerful features like generics, traits, and lifetime parameters. These enable zero-cost abstractions while maintaining compile-time safety guarantees.

Generics allow writing code that works with multiple types while maintaining type safety. Traits define shared behavior that types can implement, similar to interfaces in other languages but more powerful.

## Concurrency and Parallelism

Rust's approach to concurrency is based on the ownership model, which prevents data races at compile time. The standard library provides several concurrency primitives:

- Threads for parallel execution
- Channels for message passing between threads
- Mutexes and atomic types for shared state
- Async/await for asynchronous programming

## Error Handling Patterns

Rust uses the Result type for error handling, which forces explicit handling of error cases. This approach makes errors visible in function signatures and prevents many runtime panics.

Common error handling patterns include:
- Using the ? operator for error propagation
- Pattern matching on Result values
- Creating custom error types with the thiserror crate
- Handling multiple error types with trait objects

## Performance Optimization Techniques

Rust provides several features for writing high-performance code:
- Zero-cost abstractions that compile to efficient machine code
- Inline assembly for performance-critical sections
- SIMD instructions for parallel data processing
- Memory layout control with repr attributes

This document provides a comprehensive overview of advanced Rust programming techniques and should be substantial enough to trigger automatic summarization in the Terraphim AI system.
"#;

    let mut file = fs::File::create(&doc_path)?;
    file.write_all(substantial_content.as_bytes())?;

    println!(
        "📄 Created substantial document: {} chars",
        substantial_content.len()
    );

    // Configure role with REAL Ollama settings
    let role_name = RoleName::new("Ollama Test Role");
    let mut role = Role {
        shortname: Some("ollama_test".into()),
        name: role_name.clone(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "test".into(),
        kg: None,
        haystacks: vec![Haystack {
            location: temp_dir.path().to_string_lossy().to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
        }],
        extra: ahash::AHashMap::new(),
        ..Default::default()
    };

    // Configure for REAL Ollama integration
    role.extra
        .insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra.insert(
        "ollama_base_url".into(),
        serde_json::json!("http://127.0.0.1:11434"),
    );
    role.extra
        .insert("ollama_model".into(), serde_json::json!("gemma2:2b"));
    role.extra
        .insert("llm_auto_summarize".into(), serde_json::json!(true));

    let mut config = Config::default();
    config.roles.insert(role_name.clone(), role);
    config.default_role = role_name.clone();
    config.selected_role = role_name.clone();

    println!("⚙️  Configured role with REAL Ollama settings:");
    println!("   Provider: ollama");
    println!("   Model: gemma2:2b");
    println!("   Base URL: http://127.0.0.1:11434");
    println!("   Auto-summarize: true");

    // Initialize service
    let config_state = ConfigState::new(&mut config).await?;
    let mut service = TerraphimService::new(config_state);

    println!("🔍 Executing search to trigger REAL AI summarization...");
    let start_time = std::time::Instant::now();

    let search_query = SearchQuery {
        search_term: NormalizedTermValue::new("Rust programming".into()),
        limit: Some(5),
        role: Some(role_name),
        ..Default::default()
    };

    let search_results = service.search(&search_query).await?;
    let search_duration = start_time.elapsed();

    println!("📊 Initial Search Results (took {:?}):", search_duration);
    println!("   Documents found: {}", search_results.documents.len());
    println!(
        "   Summarization tasks queued: {}",
        search_results.summarization_task_ids.len()
    );

    if !search_results.documents.is_empty() {
        let doc = &search_results.documents[0];
        println!("   📄 Document: {}", doc.title);
        if let Some(desc) = &doc.description {
            println!("   📝 Description: {}", desc);
        }
        println!("   📏 Content length: {} chars", doc.body.len());
    }

    if !search_results.summarization_task_ids.is_empty() {
        println!("   ✅ Summarization tasks queued:");
        for task_id in &search_results.summarization_task_ids {
            println!("     📋 {}", task_id);
        }

        // Wait for REAL LLM processing (this should take several seconds)
        println!("⏳ Waiting for REAL Ollama processing (this will take several seconds)...");

        // Wait longer for real AI processing
        for i in 1..=10 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            println!("   ⏱️  Waiting... {} seconds", i * 2);

            // Try to get updated results
            let merged_start = std::time::Instant::now();
            let merged_results = search_results.clone().merge_completed_summaries(None).await;
            let merge_duration = merged_start.elapsed();

            let docs_with_summaries = merged_results
                .documents
                .iter()
                .filter(|doc| doc.summarization.is_some())
                .count();

            if docs_with_summaries > 0 {
                println!("🎉 FOUND REAL AI SUMMARIES after {} seconds!", i * 2);
                println!("   Merge operation took: {:?}", merge_duration);

                for doc in &merged_results.documents {
                    if let Some(summary) = &doc.summarization {
                        println!("   🤖 REAL AI SUMMARY: '{}'", summary);
                        println!("   📏 Summary length: {} chars", summary.len());

                        // Verify this looks like real AI content
                        if summary.len() > 50 && summary.contains(" ") {
                            println!("   ✅ CONFIRMED: This looks like real AI-generated content!");
                        }
                    }
                }
                break;
            } else {
                println!("   ℹ️  No summaries yet, continuing to wait...");
            }
        }
    } else {
        println!("   ⚠️  No summarization tasks queued - check configuration");
    }

    println!("===============================");
    println!("🎯 REAL OLLAMA TEST COMPLETE");

    Ok(())
}

/// Test that shows the exact Ollama configuration needed for real AI
#[tokio::test]
async fn test_ollama_configuration_validation() -> Result<()> {
    println!("🔧 OLLAMA CONFIGURATION VALIDATION");
    println!("==================================");

    // Test direct Ollama connectivity
    let client = reqwest::Client::new();

    match client.get("http://127.0.0.1:11434/api/tags").send().await {
        Ok(response) if response.status().is_success() => {
            println!("✅ Ollama server is running and accessible");

            let models: serde_json::Value = response.json().await?;
            if let Some(model_list) = models.get("models").and_then(|m| m.as_array()) {
                println!("✅ Available models:");
                for model in model_list {
                    if let Some(name) = model.get("name").and_then(|n| n.as_str()) {
                        println!("   📦 {}", name);
                        if name == "gemma2:2b" {
                            println!("     ✅ Target model found!");
                        }
                    }
                }
            }
        }
        Ok(response) => {
            println!(
                "❌ Ollama server responded with error: {}",
                response.status()
            );
        }
        Err(e) => {
            println!("❌ Cannot connect to Ollama server: {}", e);
            println!("   Make sure Ollama is running: ollama serve");
        }
    }

    println!("==================================");
    Ok(())
}
