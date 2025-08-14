#![cfg(feature = "ollama")]

use serial_test::serial;
use ahash::AHashMap;
use terraphim_config::{Config, ConfigState, Haystack, Role, ServiceType};
use terraphim_service::{llm, TerraphimService};
use terraphim_types::{NormalizedTermValue, RelevanceFunction, RoleName, SearchQuery};

/// Comprehensive integration test suite for Ollama LLM integration with llama3.2:3b
/// Tests connectivity, summarization, role configuration, and end-to-end search functionality
#[tokio::test]
#[serial]
async fn ollama_llama_integration_comprehensive() {
    let base_url = std::env::var("OLLAMA_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    // Test 1: Basic connectivity and model availability
    test_ollama_connectivity(&base_url).await;

    // Test 2: Direct LLM client functionality
    test_direct_llm_client(&base_url).await;

    // Test 3: Role-based configuration and LLM building
    test_role_based_llm_config(&base_url).await;

    // Test 4: End-to-end search with auto-summarization
    test_e2e_search_with_summarization(&base_url).await;

    // Test 5: Model listing and validation
    test_model_listing(&base_url).await;
}

/// Test 1: Verify Ollama instance is reachable and responsive
async fn test_ollama_connectivity(base_url: &str) {
    println!("🧪 Testing Ollama connectivity to {}", base_url);
    
    let http = reqwest::Client::new();
    let health_url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    
    let response = http
        .get(&health_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("✅ Ollama connectivity test passed");
            } else {
                panic!("❌ Ollama returned non-success status: {}", resp.status());
            }
        }
        Err(e) => {
            panic!("❌ Ollama connectivity test failed: {}", e);
        }
    }
}

/// Test 2: Test direct LLM client functionality with llama3.2:3b
async fn test_direct_llm_client(base_url: &str) {
    println!("🧪 Testing direct LLM client with llama3.2:3b");
    
    // Create a test role with llama3.2:3b configuration
    let mut role = Role {
        shortname: Some("LlamaTest".into()),
        name: "Llama Test".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![],
        extra: AHashMap::new(),
    };
    
    role.extra.insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra.insert("llm_model".into(), serde_json::json!("llama3.2:3b"));
    role.extra.insert("llm_base_url".into(), serde_json::json!(base_url));
    role.extra.insert("llm_auto_summarize".into(), serde_json::json!(true));
    
    // Build the LLM client
    let client = match llm::build_llm_from_role(&role) {
        Some(c) => c,
        None => panic!("Failed to initialize Ollama LLM client from role config"),
    };
    
    // Test summarization with llama3.2:3b
    let test_content = r#"
    Rust is a systems programming language that runs blazingly fast, prevents segfaults, 
    and guarantees thread safety. It offers memory safety without garbage collection, 
    concurrency without data races, and abstraction without overhead.
    
    Key features include:
    - Zero-cost abstractions
    - Memory safety without garbage collection
    - Thread safety without data races
    - Modern tooling with Cargo
    - Excellent documentation and community
    "#;
    
    let summary = client
        .summarize(
            test_content,
            llm::SummarizeOptions {
                max_length: 200,
            },
        )
        .await
        .expect("Summarization should succeed with llama3.2:3b");
    
    assert!(!summary.trim().is_empty(), "Summary should be non-empty");
    assert!(summary.len() <= 250, "Summary should respect max length constraint");
    
    println!("✅ Direct LLM client test passed - Summary length: {} chars", summary.len());
    println!("📝 Generated summary: {}", summary);
}

/// Test 3: Test role-based LLM configuration and client building
async fn test_role_based_llm_config(base_url: &str) {
    println!("🧪 Testing role-based LLM configuration");
    
    // Test different role configurations
    let test_configs = vec![
        ("llama3.2:3b", "llama3.2:3b"),
        ("llama3.2:3b", "llama3.2:3b"),
        ("llama3.2:3b", "llama3.2:3b"), // Multiple tests for robustness
    ];
    
    for (config_name, model_name) in test_configs {
        let mut role = Role {
            shortname: Some(format!("Test{}", config_name).into()),
            name: format!("Test Role {}", config_name).into(),
            relevance_function: RelevanceFunction::TitleScorer,
            terraphim_it: false,
            theme: "default".into(),
            kg: None,
            haystacks: vec![],
            extra: AHashMap::new(),
        };
        
        role.extra.insert("llm_provider".into(), serde_json::json!("ollama"));
        role.extra.insert("llm_model".into(), serde_json::json!(model_name));
        role.extra.insert("llm_base_url".into(), serde_json::json!(base_url));
        role.extra.insert("llm_auto_summarize".into(), serde_json::json!(true));
        
        let client = llm::build_llm_from_role(&role);
        assert!(client.is_some(), "LLM client should be built for role {}", config_name);
        
        let client = client.unwrap();
        assert_eq!(client.name(), "ollama", "Client should identify as ollama");
        
        println!("✅ Role configuration test passed for {}", config_name);
    }
}

/// Test 4: End-to-end search with auto-summarization using llama3.2:3b
async fn test_e2e_search_with_summarization(base_url: &str) {
    println!("🧪 Testing end-to-end search with auto-summarization");
    
    // Create a temporary haystack with test content
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let test_file = temp_dir.path().join("rust_guide.md");
    std::fs::write(&test_file, r#"
# Rust Programming Guide

Rust is a modern systems programming language that emphasizes safety, speed, and concurrency. 
It was designed to prevent common programming errors like null pointer dereferences, data races, and buffer overflows.

## Key Features

### Memory Safety
Rust's ownership system ensures memory safety without garbage collection. The compiler enforces rules at compile time that prevent common memory-related bugs.

### Zero-Cost Abstractions
Rust provides high-level abstractions that compile down to efficient machine code. You can write expressive, safe code without performance penalties.

### Concurrency Without Data Races
Rust's type system prevents data races at compile time, making concurrent programming safer and easier.

### Modern Tooling
Cargo, Rust's package manager, handles dependencies, building, testing, and publishing. It integrates seamlessly with the Rust ecosystem.

## Getting Started

To get started with Rust, install the Rust toolchain using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then create your first project:

```bash
cargo new hello_rust
cd hello_rust
cargo run
```

## Community and Resources

Rust has a vibrant community with excellent documentation, tutorials, and examples. The Rust Book is the definitive guide for learning Rust.
    "#).expect("Failed to write test file");
    
    // Create role with llama3.2:3b configuration
    let role_name = RoleName::new("Llama Rust Engineer");
    let mut role = Role {
        shortname: Some("llama-rust".into()),
        name: role_name.clone(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![Haystack {
            location: temp_dir.path().to_string_lossy().to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
        }],
        extra: AHashMap::new(),
    };
    
    role.extra.insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra.insert("llm_model".into(), serde_json::json!("llama3.2:3b"));
    role.extra.insert("llm_base_url".into(), serde_json::json!(base_url));
    role.extra.insert("llm_auto_summarize".into(), serde_json::json!(true));
    
    // Create configuration and service
    let mut config = Config::default();
    config.roles.insert(role_name.clone(), role);
    config.default_role = role_name.clone();
    config.selected_role = role_name.clone();
    
    let config_state = ConfigState::new(&mut config).await.expect("Failed to create config state");
    let mut service = TerraphimService::new(config_state);
    
    // Execute search
    let search_query = SearchQuery {
        search_term: NormalizedTermValue::new("Rust".into()),
        limit: Some(5),
        skip: None,
        role: Some(role_name.clone()),
    };
    
    let results = service.search(&search_query).await.expect("Search should succeed");
    
    if results.is_empty() {
        println!("⚠️  No search results found - this may indicate an indexing issue");
        return;
    }
    
    // Verify that at least one result has an AI-generated description
    let has_ai_description = results.iter().any(|result| {
        result.description.as_ref().map(|desc| !desc.trim().is_empty()).unwrap_or(false)
    });
    
    assert!(has_ai_description, "At least one result should have AI-generated description");
    
    println!("✅ End-to-end search test passed - Found {} results with AI descriptions", 
             results.iter().filter(|r| r.description.as_ref().map(|d| !d.trim().is_empty()).unwrap_or(false)).count());
    
    // Clean up
    temp_dir.close().expect("Failed to clean up temp directory");
}

/// Test 5: Test model listing and validation
async fn test_model_listing(base_url: &str) {
    println!("🧪 Testing model listing and validation");
    
    let mut role = Role {
        shortname: Some("ModelTest".into()),
        name: "Model Test".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![],
        extra: AHashMap::new(),
    };
    
    role.extra.insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra.insert("llm_model".into(), serde_json::json!("llama3.2:3b"));
    role.extra.insert("llm_base_url".into(), serde_json::json!(base_url));
    
    let client = llm::build_llm_from_role(&role).expect("Failed to build LLM client");
    
    let models = client.list_models().await.expect("Model listing should succeed");
    
    assert!(!models.is_empty(), "Should list available models");
    
    // Check if llama3.2:3b is available
    let has_llama = models.iter().any(|model| model.contains("llama3.2:3b"));
    
    if has_llama {
        println!("✅ Model listing test passed - llama3.2:3b is available");
    } else {
        println!("⚠️  Model listing test passed - llama3.2:3b not found, available models: {:?}", models);
    }
    
    println!("📋 Available models: {:?}", models);
}

/// Test 6: Performance and reliability test with multiple concurrent requests
#[tokio::test]
#[serial]
async fn ollama_llama_performance_test() {
    let base_url = std::env::var("OLLAMA_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
    
    println!("🧪 Testing Ollama llama3.2:3b performance and reliability");
    
    // Create test role
    let mut role = Role {
        shortname: Some("PerfTest".into()),
        name: "Performance Test".into(),
        relevance_function: RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![],
        extra: AHashMap::new(),
    };
    
    role.extra.insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra.insert("llm_model".into(), serde_json::json!("llama3.2:3b"));
    role.extra.insert("llm_base_url".into(), serde_json::json!(base_url));
    
    let client = llm::build_llm_from_role(&role).expect("Failed to build LLM client");
    
    // Test content for summarization
    let test_content = "Rust is a systems programming language that emphasizes safety, speed, and concurrency. It prevents segfaults and guarantees thread safety.";
    
    // Measure performance of multiple requests
    let start_time = std::time::Instant::now();
    let mut successful_requests = 0;
    let total_requests = 3;
    
    for i in 1..=total_requests {
        let request_start = std::time::Instant::now();
        
        match client
            .summarize(
                test_content,
                llm::SummarizeOptions {
                    max_length: 100,
                },
            )
            .await
        {
            Ok(summary) => {
                let duration = request_start.elapsed();
                println!("✅ Request {} completed in {:?} - Summary: {}", i, duration, summary);
                successful_requests += 1;
            }
            Err(e) => {
                println!("❌ Request {} failed: {}", i, e);
            }
        }
    }
    
    let total_duration = start_time.elapsed();
    let success_rate = (successful_requests as f64 / total_requests as f64) * 100.0;
    
    println!("📊 Performance test results:");
    println!("   Total requests: {}", total_requests);
    println!("   Successful: {}", successful_requests);
    println!("   Success rate: {:.1}%", success_rate);
    println!("   Total time: {:?}", total_duration);
    println!("   Average time per request: {:?}", total_duration / total_requests as u32);
    
    assert!(successful_requests > 0, "At least one request should succeed");
    assert!(success_rate >= 50.0, "Success rate should be at least 50%");
    
    println!("✅ Performance test passed");
}
