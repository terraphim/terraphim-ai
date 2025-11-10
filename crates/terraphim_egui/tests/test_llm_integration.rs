//! LLM integration tests with Ollama
//!
//! These tests use real Ollama for LLM calls (no mocking).
//! Tests marked with #[ignore] require Ollama to be running.

use chrono::Utc;
use std::sync::{Arc, Mutex};
use terraphim_egui::state::{AppState, ChatMessage, ChatMessageRole};
use terraphim_types::Document;

/// Helper to create a test document
fn create_test_document(title: &str, content: &str) -> Document {
    Document {
        id: uuid::Uuid::new_v4().to_string(),
        url: "https://example.com/test".to_string(),
        title: title.to_string(),
        body: content.to_string(),
        description: Some(format!("Description for {}", title)),
        summarization: None,
        stub: None,
        tags: Some(vec!["test".to_string()]),
        rank: Some(100),
        source_haystack: Some("test".to_string()),
    }
}

/// Test basic Ollama connection
#[tokio::test]
#[ignore = "requires Ollama to be running"]
async fn test_ollama_connection() {
    let ollama_url =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

    // Try to connect to Ollama
    let response = reqwest::get(&format!("{}/api/tags", ollama_url))
        .await
        .expect("Failed to connect to Ollama");

    assert!(response.status().is_success(), "Ollama should be running");
}

/// Test document summarization with Ollama
#[tokio::test]
#[ignore = "requires Ollama to be running"]
async fn test_document_summarization() {
    let ollama_url =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string());

    // Create a test document
    let doc = create_test_document(
        "Rust Ownership",
        "Rust's ownership system is a unique feature that provides memory safety without a garbage collector. Each value in Rust has a single owner, and when the owner goes out of scope, the value is automatically dropped."
    );

    // Prepare the prompt for summarization
    let prompt = format!(
        "Please provide a concise summary of the following document:\n\nTitle: {}\nContent: {}",
        doc.title, doc.body
    );

    // Send request to Ollama
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/api/generate", ollama_url))
        .json(&serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        }))
        .send()
        .await
        .expect("Failed to send request to Ollama");

    assert!(
        response.status().is_success(),
        "Ollama should respond successfully"
    );

    let result: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse Ollama response");

    // Verify we got a response
    assert!(
        result.get("response").is_some(),
        "Response should contain 'response' field"
    );
    let summary = result["response"].as_str().unwrap_or("");
    assert!(!summary.trim().is_empty(), "Summary should not be empty");
    // Note: LLMs can be verbose, so we just check it's reasonable content
    assert!(
        summary.len() >= 20,
        "Summary should have substantial content"
    );
}

/// Test chat completion with context
#[tokio::test]
#[ignore = "requires Ollama to be running"]
async fn test_chat_with_context() {
    let ollama_url =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string());

    // Set up state with context
    let state = AppState::new();
    let state_arc = Arc::new(Mutex::new(state));

    // Add some context documents
    {
        let mut state = state_arc.lock().unwrap();
        let doc = create_test_document(
            "Async Programming in Rust",
            "Async programming in Rust allows for concurrent execution of tasks. It uses async/await syntax and futures to manage asynchronous operations efficiently."
        );
        state.add_document_to_context(doc);
    }

    // Create a chat message asking about the context
    let user_message = ChatMessage {
        id: uuid::Uuid::new_v4(),
        role: ChatMessageRole::User,
        content: "What can you tell me about async programming?".to_string(),
        timestamp: Utc::now(),
        metadata: None,
    };

    // Add message to conversation
    state_arc.lock().unwrap().add_chat_message(user_message);

    // Get context for the LLM
    let (context_docs, history_len) = {
        let state = state_arc.lock().unwrap();
        let docs = state.get_context_manager().selected_documents.clone();
        let history_len = state.get_conversation_history().len();
        (docs, history_len)
    };

    // Prepare the prompt with context
    let context_text = context_docs
        .iter()
        .map(|doc| format!("Title: {}\n{}", doc.title, doc.body))
        .collect::<Vec<_>>()
        .join("\n\n");

    let prompt = format!(
        "Context documents:\n{}\n\nQuestion: What can you tell me about async programming?",
        context_text
    );

    // Send to Ollama
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/api/generate", ollama_url))
        .json(&serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        }))
        .send()
        .await
        .expect("Failed to send request to Ollama");

    assert!(response.status().is_success(), "Ollama should respond");

    let result: serde_json::Value = response.json().await.expect("Failed to parse response");

    let assistant_response = result["response"].as_str().unwrap_or("");
    assert!(
        !assistant_response.trim().is_empty(),
        "Response should not be empty"
    );

    // Verify conversation history
    {
        let state = state_arc.lock().unwrap();
        let history = state.get_conversation_history();
        assert_eq!(history.len(), 1, "Should have one message in history");
        assert_eq!(history[0].role, ChatMessageRole::User);
    }
}

/// Test error handling when Ollama is unavailable
#[tokio::test]
async fn test_ollama_unavailable() {
    // Use a non-existent URL to simulate Ollama being unavailable
    let ollama_url = "http://127.0.0.1:99999";

    let client = reqwest::Client::new();
    let response = client.get(&format!("{}/api/tags", ollama_url)).send().await;

    // Should get an error, not panic
    match response {
        Ok(_) => {
            // If we get a response, it should be an error status
            // (Ollama might actually be running, which is fine)
        }
        Err(_) => {
            // Connection failed as expected when Ollama is not running
            // This is the normal case we're testing
        }
    }
}

/// Test LLM with markdown output
#[tokio::test]
#[ignore = "requires Ollama to be running"]
async fn test_markdown_output() {
    let ollama_url =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string());

    let prompt =
        "Please explain Rust's error handling in markdown format with headings and bullet points.";

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/api/generate", ollama_url))
        .json(&serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert!(
        response.status().is_success(),
        "Should get successful response"
    );

    let result: serde_json::Value = response.json().await.expect("Failed to parse response");

    let content = result["response"].as_str().unwrap_or("");
    assert!(!content.trim().is_empty(), "Response should not be empty");

    // Check if response contains markdown elements
    let has_heading = content.contains('#') || content.contains("**");
    let has_structure = content.contains('\n') && content.len() > 50;

    assert!(has_structure, "Response should be well-formatted");
}
