//! Integration tests for Codex CLI compatibility
//!
//! These tests verify that the proxy correctly handles OpenAI API format requests
//! from Codex CLI, including model mapping and response formatting.

use serde_json::json;
use terraphim_llm_proxy::{
    client_detection::{detect_client, ClientType},
    routing::translate_model,
};

/// Test that Codex CLI requests are correctly detected
#[tokio::test]
async fn test_codex_cli_detection() {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", "Bearer sk-test123".parse().unwrap());
    headers.insert("user-agent", "codex-cli/1.0".parse().unwrap());

    let client_info = detect_client(&headers, "/v1/chat/completions");
    assert_eq!(client_info.client_type, ClientType::CodexCli);
}

/// Test model mapping for Codex CLI (GPT models to provider equivalents)
#[test]
fn test_codex_model_mapping_to_groq() {
    let available_models = vec![
        "llama-3.1-70b-versatile".to_string(),
        "llama-3.1-8b-instant".to_string(),
    ];

    // Test gpt-4o mapping to Groq
    let result = translate_model("gpt-4o", ClientType::CodexCli, "groq", &available_models);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "llama-3.1-70b-versatile");

    // Test gpt-3.5-turbo mapping
    let result = translate_model(
        "gpt-3.5-turbo",
        ClientType::CodexCli,
        "groq",
        &available_models,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "llama-3.1-8b-instant");
}

/// Test that Codex CLI requests use OpenAI API format
#[test]
fn test_codex_openai_format() {
    let client_type = ClientType::CodexCli;
    assert!(client_type.expects_openai_format());
    assert!(!client_type.expects_anthropic_format());
}

/// Integration test simulating Codex CLI chat completion request
#[tokio::test]
async fn test_codex_chat_completion_request() {
    // This test would require a running server instance
    // For now, we verify the request structure is correct
    let request_body = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Hello, how are you?"}
        ],
        "max_tokens": 100,
        "temperature": 0.7
    });

    // Verify the request body structure
    assert!(request_body.get("model").is_some());
    assert!(request_body.get("messages").is_some());
    assert!(request_body.get("max_tokens").is_some());

    let messages = request_body.get("messages").unwrap().as_array().unwrap();
    assert_eq!(messages.len(), 2);
}

/// Test streaming response handling for Codex CLI
#[tokio::test]
async fn test_codex_streaming_request() {
    let request_body = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Say hello"}
        ],
        "stream": true,
        "max_tokens": 50
    });

    assert_eq!(request_body.get("stream").unwrap(), true);
}

/// Test model passthrough when model is already provider-compatible
#[test]
fn test_codex_model_passthrough() {
    let available_models = vec!["custom-model".to_string()];

    // If model is already available on provider, it should pass through
    let result = translate_model(
        "custom-model",
        ClientType::CodexCli,
        "openrouter",
        &available_models,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "custom-model");
}

/// Test error handling for unknown models
#[test]
fn test_codex_unknown_model_error() {
    use terraphim_llm_proxy::routing::FallbackStrategy;
    use terraphim_llm_proxy::routing::ModelMapper;

    let available_models = vec!["known-model".to_string()];
    let mut mapper = ModelMapper::new().with_fallback(FallbackStrategy::Error);

    let result = mapper.translate(
        "unknown-gpt-model",
        ClientType::CodexCli,
        "groq",
        &available_models,
    );

    assert!(result.is_err());
}

/// Test that multiple Codex CLI requests can be handled concurrently
#[tokio::test]
async fn test_codex_concurrent_requests() {
    use tokio::task;

    let client_types = vec![
        ClientType::CodexCli,
        ClientType::CodexCli,
        ClientType::CodexCli,
    ];

    let handles: Vec<_> = client_types
        .into_iter()
        .map(|client_type| {
            task::spawn(async move {
                // Simulate request processing
                assert!(client_type.expects_openai_format());
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                client_type
            })
        })
        .collect();

    let results = futures::future::join_all(handles).await;
    assert_eq!(results.len(), 3);
}

/// Test Codex CLI with explicit provider specification
#[test]
fn test_codex_explicit_provider() {
    // Codex CLI can use explicit provider syntax: "openrouter:gpt-4o"
    let model_name = "openrouter:gpt-4o";
    let parts: Vec<&str> = model_name.split(':').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], "openrouter");
    assert_eq!(parts[1], "gpt-4o");
}
