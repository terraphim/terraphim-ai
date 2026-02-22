//! Live test for Ollama chat completion with context
//!
//! This test requires a running Ollama instance with llama3.2:3b model.
//! Run with: OLLAMA_BASE_URL=http://127.0.0.1:11434 cargo test ollama_chat_context_live_test -- --ignored

use ahash::AHashMap;
use serial_test::serial;
use std::env;
use terraphim_service::context::{ContextConfig, ContextManager};
use terraphim_service::llm::{ChatOptions, build_llm_from_role};
use terraphim_types::{ContextItem, ContextType, RoleName};

#[tokio::test]
#[serial]
#[ignore] // Only run with --ignored flag
#[cfg(feature = "ollama")]
async fn ollama_chat_context_live_test() {
    // Check if Ollama is available
    let ollama_url =
        env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

    println!("🔍 Testing Ollama chat with context at: {}", ollama_url);

    // Create Ollama role configuration
    let role = create_ollama_live_role(&ollama_url);

    // Build LLM client
    let llm_client = match build_llm_from_role(&role) {
        Some(client) => {
            println!("✅ Created Ollama LLM client: {}", client.name());
            client
        }
        None => {
            panic!("❌ Failed to create Ollama LLM client");
        }
    };

    // Test basic model availability first
    println!("🔍 Testing model availability...");
    match llm_client.list_models().await {
        Ok(models) => {
            println!("✅ Available models: {:?}", models);
            if !models
                .iter()
                .any(|m| m.contains("llama3.2") || m.contains("llama"))
            {
                println!(
                    "⚠️  Warning: llama3.2:3b not found in model list, but continuing test..."
                );
            }
        }
        Err(e) => {
            println!(
                "⚠️  Warning: Could not list models: {}, but continuing test...",
                e
            );
        }
    }

    // Create context manager and conversation
    let mut context_manager = ContextManager::new(ContextConfig::default());
    let conversation_id = context_manager
        .create_conversation("Rust Live Chat".to_string(), RoleName::new("RustDev"))
        .await
        .expect("Should create conversation");

    println!("✅ Created conversation: {}", conversation_id.as_str());

    // Add comprehensive context about Rust async programming
    let rust_context = ContextItem {
        id: "rust-async-guide".to_string(),
        context_type: ContextType::Document,
        title: "Rust Async Programming Guide".to_string(),
        summary: Some("Comprehensive guide to async programming patterns in Rust".to_string()),
        content: r#"
# Rust Async Programming with Tokio

## Key Concepts:
- **async/await**: Core syntax for asynchronous programming
- **Future trait**: Represents a computation that will complete in the future
- **Tokio runtime**: Provides the executor for async tasks
- **Tasks**: Lightweight threads managed by the runtime
- **Channels**: Communication between async tasks (mpsc, broadcast, oneshot)

## Best Practices:
1. Use `tokio::spawn` for concurrent tasks
2. Prefer bounded channels for backpressure
3. Use `tokio::select!` for managing multiple operations
4. Implement timeouts with `tokio::time::timeout`
5. Use `Arc<Mutex<T>>` carefully - prefer channels for communication

## Common Patterns:
- Producer-consumer with mpsc channels
- Fan-out with broadcast channels
- Request-response with oneshot channels
- Graceful shutdown with cancellation tokens
"#
        .trim()
        .to_string(),
        metadata: {
            let mut map = AHashMap::new();
            map.insert("source".to_string(), "rust-async-book".to_string());
            map.insert("version".to_string(), "2024".to_string());
            map
        },
        created_at: chrono::Utc::now(),
        relevance_score: Some(95.0),
    };

    context_manager
        .add_context(&conversation_id, rust_context)
        .expect("Should add Rust context");

    // Add second context about error handling
    let error_context = ContextItem {
        id: "rust-error-handling".to_string(),
        context_type: ContextType::Document,
        title: "Rust Error Handling Best Practices".to_string(),
        summary: Some("Error handling patterns in async Rust code".to_string()),
        content: r#"
# Error Handling in Async Rust

## Key Principles:
- Use `Result<T, E>` for recoverable errors
- Use `?` operator for error propagation
- Implement custom error types with `thiserror`
- Handle errors at appropriate boundaries
- Use `anyhow` for application-level error handling

## Async-Specific Patterns:
- Wrap async operations in try blocks
- Use `tokio::try_join!` for concurrent error propagation
- Implement proper error recovery in long-running tasks
- Log errors with context using `tracing`
"#
        .trim()
        .to_string(),
        metadata: {
            let mut map = AHashMap::new();
            map.insert("topic".to_string(), "error-handling".to_string());
            map
        },
        created_at: chrono::Utc::now(),
        relevance_score: Some(87.0),
    };

    context_manager
        .add_context(&conversation_id, error_context)
        .expect("Should add error handling context");

    println!("✅ Added 2 context items to conversation");

    // Get conversation and build messages with context
    let conversation = context_manager
        .get_conversation(&conversation_id)
        .expect("Should get conversation");

    // Verify context is properly loaded
    assert_eq!(conversation.global_context.len(), 2);
    println!(
        "✅ Verified context items in conversation: {}",
        conversation.global_context.len()
    );

    // Build messages with context
    let mut messages =
        terraphim_service::context::build_llm_messages_with_context(&conversation, true);

    // Verify context was injected and extract content for later use
    assert!(!messages.is_empty());
    let context_content = {
        let context_message = &messages[0];
        assert_eq!(context_message["role"], "system");
        let content = context_message["content"].as_str().unwrap();
        assert!(content.contains("Rust Async Programming Guide"));
        assert!(content.contains("tokio::spawn"));
        assert!(content.contains("Error Handling Best Practices"));
        content.to_string()
    };

    println!(
        "✅ Context properly injected into system message ({} chars)",
        context_content.len()
    );

    // Add user question about async programming
    let user_question = "Based on the context provided, can you explain how to implement a producer-consumer pattern in Rust using async/await and tokio channels? Please reference the specific best practices mentioned in the context.";

    messages.push(serde_json::json!({
        "role": "user",
        "content": user_question
    }));

    println!("🤖 Asking Ollama: {}", user_question);

    // Configure chat options
    let chat_opts = ChatOptions {
        max_tokens: Some(2048),
        temperature: Some(0.7),
    };

    // Perform chat completion with context
    println!("⏳ Sending chat completion request to Ollama...");
    let start_time = std::time::Instant::now();

    match llm_client
        .chat_completion(messages, chat_opts.clone())
        .await
    {
        Ok(response) => {
            let elapsed = start_time.elapsed();
            println!(
                "✅ Chat completion successful in {:.2}s",
                elapsed.as_secs_f64()
            );
            println!("📝 Response length: {} characters", response.len());

            // Verify the response is context-aware
            let response_lower = response.to_lowercase();

            // Check for context-specific terms
            let context_terms = [
                "context",
                "provided",
                "mentioned",
                "tokio",
                "mpsc",
                "producer",
                "consumer",
                "async",
                "await",
                "channel",
            ];

            let mut found_terms = Vec::new();
            for term in &context_terms {
                if response_lower.contains(term) {
                    found_terms.push(*term);
                }
            }

            println!("✅ Found context-aware terms: {:?}", found_terms);

            // Verify meaningful response
            assert!(response.len() > 100, "Response should be substantial");
            assert!(found_terms.len() >= 4, "Should reference context terms");

            // Print first few lines of response for manual verification
            let lines: Vec<&str> = response.lines().take(5).collect();
            println!("📄 First few lines of response:");
            for (i, line) in lines.iter().enumerate() {
                println!("  {}: {}", i + 1, line);
            }

            // Test follow-up question
            println!("\n🔄 Testing follow-up question...");
            let follow_up_messages = vec![
                serde_json::json!({"role": "system", "content": context_content}),
                serde_json::json!({"role": "user", "content": user_question}),
                serde_json::json!({"role": "assistant", "content": response}),
                serde_json::json!({"role": "user", "content": "What about error handling in this pattern? How should I handle errors in the producer and consumer tasks?"}),
            ];

            match llm_client
                .chat_completion(follow_up_messages, chat_opts)
                .await
            {
                Ok(follow_up_response) => {
                    println!(
                        "✅ Follow-up response successful ({} chars)",
                        follow_up_response.len()
                    );

                    // Check for error handling context
                    let error_terms = ["error", "result", "thiserror", "anyhow", "try_join"];
                    let mut found_error_terms = Vec::new();
                    let follow_up_lower = follow_up_response.to_lowercase();

                    for term in &error_terms {
                        if follow_up_lower.contains(term) {
                            found_error_terms.push(*term);
                        }
                    }

                    println!("✅ Found error handling terms: {:?}", found_error_terms);
                    assert!(
                        found_error_terms.len() >= 2,
                        "Should reference error handling concepts"
                    );
                }
                Err(e) => {
                    println!("⚠️  Follow-up failed: {}", e);
                    // Don't fail the test for follow-up errors
                }
            }
        }
        Err(e) => {
            panic!("❌ Chat completion failed: {}", e);
        }
    }

    println!("🎉 Live Ollama chat with context test completed successfully!");
}

fn create_ollama_live_role(base_url: &str) -> terraphim_config::Role {
    let mut role = terraphim_config::Role {
        shortname: Some("LiveOllama".into()),
        name: "Live Ollama Test".into(),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![],
        llm_enabled: true,
        llm_api_key: None,
        llm_model: Some("gemma3:270m".to_string()),
        llm_auto_summarize: false,
        llm_chat_enabled: true,
        llm_chat_system_prompt: Some("You are a helpful assistant.".to_string()),
        llm_chat_model: Some("gemma3:270m".to_string()),
        llm_context_window: None,
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
    };

    // Configure for Ollama
    role.extra
        .insert("llm_provider".to_string(), serde_json::json!("ollama"));
    role.extra
        .insert("llm_model".to_string(), serde_json::json!("llama3.2:3b"));
    role.extra
        .insert("llm_base_url".to_string(), serde_json::json!(base_url));
    role.extra.insert(
        "system_prompt".to_string(),
        serde_json::json!("You are an expert Rust programming assistant specializing in async programming and systems development."),
    );

    role
}
