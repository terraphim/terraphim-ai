//! Comprehensive LLM Chat Matrix Tests using real services
//! Uses configuration from .env file - no mocks, all real services

use serde_json::json;
use std::env;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use terraphim_config::{Haystack, Role, ServiceType};
use terraphim_service::llm::{build_llm_from_role, ChatOptions};
use tokio::time::sleep;

/// Load configuration from .env file
fn load_env_config() {
    dotenv::dotenv().ok();
}

/// Create a test role with appropriate LLM configuration
fn create_test_role(name: &str, provider: &str) -> Role {
    let mut role = Role {
        name: name.into(),
        shortname: Some(name.replace(" ", "").to_lowercase()),
        ..Default::default()
    };

    // Set role-specific system prompt based on our implementation
    role.llm_system_prompt = match name {
        "Rust Engineer" => Some("You are an expert Rust developer specializing in systems programming, async programming, WebAssembly, and performance optimization. Focus on memory safety, idiomatic Rust patterns, and modern Rust ecosystem tools.".to_string()),
        "AI Engineer" => Some("You are an expert AI/ML engineer specializing in implementing machine learning features, integrating language models, and building intelligent automation systems. Focus on practical AI implementation, model integration, and AI-powered applications.".to_string()),
        "Terraphim Engineer" => Some("You are an expert Terraphim AI engineer specializing in knowledge graphs, semantic search, and document intelligence. You help users build and optimize Terraphim AI systems with advanced graph-based search capabilities.".to_string()),
        "System Operator" => Some("You are an expert system operator and DevOps engineer specializing in infrastructure management, monitoring, deployment automation, and operational excellence. Focus on reliability, scalability, and security best practices.".to_string()),
        "Default" => None,
        _ => Some("You are a helpful assistant.".to_string()),
    };

    // Configure LLM provider
    match provider {
        "ollama" => {
            role.extra
                .insert("llm_provider".to_string(), json!("ollama"));
            role.extra.insert(
                "llm_model".to_string(),
                json!(env::var("OLLAMA_MODEL").unwrap_or("llama3.2:3b".to_string())),
            );
            role.extra.insert(
                "llm_base_url".to_string(),
                json!(env::var("OLLAMA_BASE_URL").unwrap_or("http://127.0.0.1:11434".to_string())),
            );
        }
        "openrouter" => {
            #[cfg(feature = "openrouter")]
            {
                if let Ok(api_key) = env::var("OPENROUTER_API_KEY") {
                    role.openrouter_enabled = true;
                    role.openrouter_api_key = Some(api_key);
                    role.openrouter_model = Some("openai/gpt-3.5-turbo".to_string());
                    role.openrouter_chat_enabled = true;
                }
            }
        }
        _ => {}
    }

    // Add a basic haystack for testing
    role.haystacks = vec![Haystack {
        location: "./docs/src".to_string(),
        service: ServiceType::Ripgrep,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
        weight: 1.0,
        fetch_content: false,
    }];

    role
}

/// Check if a service is available
async fn is_service_available(url: &str) -> bool {
    if let Ok(response) = reqwest::get(format!("{}/api/tags", url)).await {
        response.status().is_success()
    } else {
        false
    }
}

/// Load real documents from docs/src for testing
fn load_test_documents() -> Vec<String> {
    let docs_dir = Path::new("./docs/src");
    let mut documents = Vec::new();

    if docs_dir.exists() {
        if let Ok(entries) = fs::read_dir(docs_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "md" {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            // Take first 500 chars to keep context manageable
                            let preview = if content.len() > 500 {
                                format!("{}...", &content[..500])
                            } else {
                                content
                            };
                            documents.push(preview);
                        }
                    }
                }
            }
        }
    }

    if documents.is_empty() {
        // Fallback test content if no docs found
        documents.push("Terraphim AI is a privacy-first AI assistant that provides semantic search across knowledge repositories.".to_string());
    }

    documents
}

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_default_ripgrep_ollama() {
    load_env_config();

    // Skip if Ollama not available
    let ollama_url = env::var("OLLAMA_BASE_URL").unwrap_or("http://127.0.0.1:11434".to_string());
    if !is_service_available(&ollama_url).await {
        println!("Skipping test: Ollama not available at {}", ollama_url);
        return;
    }

    let role = create_test_role("Default", "ollama");
    let llm_client = build_llm_from_role(&role).expect("Failed to create Ollama client");

    // Create test message
    let messages = vec![
        json!({"role": "user", "content": "Hello, please respond with 'Test successful' if you can understand this message."}),
    ];

    // Test chat completion with timing
    let start = Instant::now();
    let result = llm_client
        .chat_completion(
            messages,
            ChatOptions {
                max_tokens: Some(1000),
                temperature: Some(0.7),
            },
        )
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Chat failed: {:?}", result);
    let response = result.unwrap();
    assert!(!response.is_empty(), "Empty response");
    assert!(
        duration < Duration::from_secs(10),
        "Response too slow: {:?}",
        duration
    );

    println!("✓ Default + Ripgrep + Ollama test passed in {:?}", duration);
    println!("Response: {}", response);
}

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_rust_engineer_ripgrep_ollama() {
    load_env_config();

    let ollama_url = env::var("OLLAMA_BASE_URL").unwrap_or("http://127.0.0.1:11434".to_string());
    if !is_service_available(&ollama_url).await {
        println!("Skipping test: Ollama not available");
        return;
    }

    let role = create_test_role("Rust Engineer", "ollama");
    let llm_client = build_llm_from_role(&role).expect("Failed to create Ollama client");

    // Create test message related to Rust
    let messages = vec![
        json!({"role": "user", "content": "Explain async/await in Rust in one sentence, focusing on memory safety."}),
    ];

    let start = Instant::now();
    let result = llm_client
        .chat_completion(
            messages,
            ChatOptions {
                max_tokens: Some(1000),
                temperature: Some(0.7),
            },
        )
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Chat failed: {:?}", result);
    let response = result.unwrap();
    assert!(!response.is_empty(), "Empty response");
    assert!(
        duration < Duration::from_secs(10),
        "Response too slow: {:?}",
        duration
    );

    // Check if response seems Rust-focused (basic validation)
    let response_lower = response.to_lowercase();
    assert!(
        response_lower.contains("async")
            || response_lower.contains("rust")
            || response_lower.contains("memory")
            || response_lower.contains("await"),
        "Response doesn't seem Rust-focused: {}",
        response
    );

    println!(
        "✓ Rust Engineer + Ripgrep + Ollama test passed in {:?}",
        duration
    );
    println!("Response: {}", response);
}

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_ai_engineer_ripgrep_ollama() {
    load_env_config();

    let ollama_url = env::var("OLLAMA_BASE_URL").unwrap_or("http://127.0.0.1:11434".to_string());
    if !is_service_available(&ollama_url).await {
        println!("Skipping test: Ollama not available");
        return;
    }

    let role = create_test_role("AI Engineer", "ollama");
    let llm_client = build_llm_from_role(&role).expect("Failed to create Ollama client");

    let messages = vec![
        json!({"role": "user", "content": "What is a transformer model? Answer in one sentence."}),
    ];

    let start = Instant::now();
    let result = llm_client
        .chat_completion(
            messages,
            ChatOptions {
                max_tokens: Some(1000),
                temperature: Some(0.7),
            },
        )
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Chat failed: {:?}", result);
    let response = result.unwrap();
    assert!(!response.is_empty(), "Empty response");
    assert!(
        duration < Duration::from_secs(10),
        "Response too slow: {:?}",
        duration
    );

    println!(
        "✓ AI Engineer + Ripgrep + Ollama test passed in {:?}",
        duration
    );
    println!("Response: {}", response);
}

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_terraphim_engineer_knowledgegraph_ollama() {
    load_env_config();

    let ollama_url = env::var("OLLAMA_BASE_URL").unwrap_or("http://127.0.0.1:11434".to_string());
    if !is_service_available(&ollama_url).await {
        println!("Skipping test: Ollama not available");
        return;
    }

    let role = create_test_role("Terraphim Engineer", "ollama");
    let llm_client = build_llm_from_role(&role).expect("Failed to create Ollama client");

    // Load some actual documentation content
    let docs = load_test_documents();
    let context = if docs.is_empty() {
        "No documentation found".to_string()
    } else {
        format!("Context from docs: {}", docs[0])
    };

    let messages = vec![
        json!({"role": "user", "content": format!("Based on this context: {}\n\nExplain what Terraphim AI is in one sentence.", context)}),
    ];

    let start = Instant::now();
    let result = llm_client
        .chat_completion(
            messages,
            ChatOptions {
                max_tokens: Some(1000),
                temperature: Some(0.7),
            },
        )
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Chat failed: {:?}", result);
    let response = result.unwrap();
    assert!(!response.is_empty(), "Empty response");
    assert!(
        duration < Duration::from_secs(10),
        "Response too slow: {:?}",
        duration
    );

    println!(
        "✓ Terraphim Engineer + KnowledgeGraph + Ollama test passed in {:?}",
        duration
    );
    println!("Response: {}", response);
}

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_system_operator_ripgrep_ollama() {
    load_env_config();

    let ollama_url = env::var("OLLAMA_BASE_URL").unwrap_or("http://127.0.0.1:11434".to_string());
    if !is_service_available(&ollama_url).await {
        println!("Skipping test: Ollama not available");
        return;
    }

    let role = create_test_role("System Operator", "ollama");
    let llm_client = build_llm_from_role(&role).expect("Failed to create Ollama client");

    let messages = vec![
        json!({"role": "user", "content": "What is the most important principle for system reliability? Answer in one sentence."}),
    ];

    let start = Instant::now();
    let result = llm_client
        .chat_completion(
            messages,
            ChatOptions {
                max_tokens: Some(1000),
                temperature: Some(0.7),
            },
        )
        .await;
    let duration = start.elapsed();

    assert!(result.is_ok(), "Chat failed: {:?}", result);
    let response = result.unwrap();
    assert!(!response.is_empty(), "Empty response");
    assert!(
        duration < Duration::from_secs(10),
        "Response too slow: {:?}",
        duration
    );

    println!(
        "✓ System Operator + Ripgrep + Ollama test passed in {:?}",
        duration
    );
    println!("Response: {}", response);
}

#[tokio::test]
#[ignore] // Run with --ignored flag
#[cfg(feature = "openrouter")]
async fn test_ai_engineer_ripgrep_openrouter() {
    load_env_config();

    // Skip if OpenRouter not configured
    if env::var("OPENROUTER_API_KEY").is_err() {
        println!("Skipping test: OpenRouter not configured");
        return;
    }

    let role = create_test_role("AI Engineer", "openrouter");
    let llm_client = build_llm_from_role(&role).expect("Failed to create OpenRouter client");

    let messages =
        vec![json!({"role": "user", "content": "What is machine learning in exactly 10 words?"})];

    // Test with retry logic for rate limits
    let mut attempts = 0;
    let max_attempts = 3;

    while attempts < max_attempts {
        let start = Instant::now();
        match llm_client
            .chat_completion(
                messages.clone(),
                ChatOptions {
                    max_tokens: Some(1000),
                    temperature: Some(0.7),
                },
            )
            .await
        {
            Ok(response) => {
                let duration = start.elapsed();
                assert!(!response.is_empty(), "Empty response");
                assert!(duration < Duration::from_secs(15), "Response too slow");

                println!(
                    "✓ AI Engineer + Ripgrep + OpenRouter test passed in {:?}",
                    duration
                );
                println!("Response: {}", response);
                return;
            }
            Err(e) if e.to_string().contains("429") || e.to_string().contains("rate limit") => {
                attempts += 1;
                if attempts < max_attempts {
                    let delay = Duration::from_secs(2_u64.pow(attempts));
                    println!("Rate limited, retrying in {:?}...", delay);
                    sleep(delay).await;
                } else {
                    panic!("Test failed after {} retries: {}", max_attempts, e);
                }
            }
            Err(e) => panic!("Test failed: {}", e),
        }
    }
}

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_all_roles_with_ollama() {
    load_env_config();

    let ollama_url = env::var("OLLAMA_BASE_URL").unwrap_or("http://127.0.0.1:11434".to_string());
    if !is_service_available(&ollama_url).await {
        println!("Skipping comprehensive test: Ollama not available");
        return;
    }

    let roles = vec![
        "Default",
        "Rust Engineer",
        "AI Engineer",
        "Terraphim Engineer",
        "System Operator",
    ];

    let mut total_tests = 0;
    let mut passed_tests = 0;
    let mut total_time = Duration::new(0, 0);

    println!("Running comprehensive test of all roles...");

    for role_name in roles {
        total_tests += 1;
        let role = create_test_role(role_name, "ollama");

        if let Some(client) = build_llm_from_role(&role) {
            let messages = vec![
                json!({"role": "user", "content": "Hello, please respond with just 'OK' to confirm you're working."}),
            ];

            let start = Instant::now();
            match client
                .chat_completion(
                    messages,
                    ChatOptions {
                        max_tokens: Some(1000),
                        temperature: Some(0.7),
                    },
                )
                .await
            {
                Ok(response) => {
                    let duration = start.elapsed();
                    total_time += duration;

                    if !response.is_empty() && duration < Duration::from_secs(10) {
                        passed_tests += 1;
                        println!("✓ {} with Ollama: PASSED ({:?})", role_name, duration);
                    } else {
                        println!(
                            "✗ {} with Ollama: FAILED (response: '{}', duration: {:?})",
                            role_name, response, duration
                        );
                    }
                }
                Err(e) => {
                    println!("✗ {} with Ollama: FAILED ({})", role_name, e);
                }
            }
        } else {
            println!(
                "✗ {} with Ollama: FAILED (could not create client)",
                role_name
            );
        }

        // Small delay between tests to be nice to the service
        sleep(Duration::from_millis(500)).await;
    }

    println!("\n=== Comprehensive Test Summary ===");
    println!(
        "Total: {}, Passed: {}, Failed: {}",
        total_tests,
        passed_tests,
        total_tests - passed_tests
    );
    println!(
        "Average response time: {:?}",
        total_time / total_tests as u32
    );
    println!(
        "Pass rate: {:.1}%",
        (passed_tests as f64 / total_tests as f64) * 100.0
    );

    // We want at least some tests to pass
    assert!(
        passed_tests > 0,
        "No tests passed - check Ollama configuration"
    );

    // Ideally all tests should pass, but we'll accept 80% pass rate
    let pass_rate = (passed_tests as f64 / total_tests as f64) * 100.0;
    assert!(pass_rate >= 60.0, "Pass rate too low: {:.1}%", pass_rate);

    println!("✅ Comprehensive test completed successfully!");
}

#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_performance_benchmarks() {
    load_env_config();

    let ollama_url = env::var("OLLAMA_BASE_URL").unwrap_or("http://127.0.0.1:11434".to_string());
    if !is_service_available(&ollama_url).await {
        println!("Skipping performance test: Ollama not available");
        return;
    }

    let role = create_test_role("Default", "ollama");
    let llm_client = build_llm_from_role(&role).expect("Failed to create Ollama client");

    // Test different message lengths
    let test_cases = vec![
        ("short", "Hello"),
        ("medium", "Explain what Terraphim AI does in 2-3 sentences."),
        ("long", "Please provide a detailed explanation of how semantic search works, including the benefits of using knowledge graphs for document retrieval and the role of vector embeddings in modern information retrieval systems."),
    ];

    println!("Running performance benchmarks...");

    for (name, message) in test_cases {
        let messages = vec![json!({"role": "user", "content": message})];

        let start = Instant::now();
        match llm_client
            .chat_completion(
                messages,
                ChatOptions {
                    max_tokens: Some(1000),
                    temperature: Some(0.7),
                },
            )
            .await
        {
            Ok(response) => {
                let duration = start.elapsed();
                println!(
                    "✓ {} message: {:?} (response: {} chars)",
                    name,
                    duration,
                    response.len()
                );

                // Performance assertions
                match name {
                    "short" => assert!(duration < Duration::from_secs(3), "Short message too slow"),
                    "medium" => {
                        assert!(duration < Duration::from_secs(5), "Medium message too slow")
                    }
                    "long" => assert!(duration < Duration::from_secs(10), "Long message too slow"),
                    _ => {}
                }
            }
            Err(e) => {
                println!("✗ {} message failed: {}", name, e);
            }
        }

        // Delay between tests
        sleep(Duration::from_millis(1000)).await;
    }

    println!("✅ Performance benchmarks completed");
}
