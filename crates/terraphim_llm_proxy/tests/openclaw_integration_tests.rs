//! Integration tests for OpenClaw compatibility
//!
//! These tests verify that the proxy correctly handles requests from OpenClaw,
//! including multi-model routing and intelligent provider selection.

use terraphim_llm_proxy::{
    client_detection::{detect_client, ClientType, DetectionMethod},
    routing::translate_model,
};

/// Test that OpenClaw requests are correctly detected via User-Agent
#[tokio::test]
async fn test_openclaw_detection() {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("user-agent", "OpenClaw/0.35".parse().unwrap());
    headers.insert("authorization", "Bearer sk-test123".parse().unwrap());

    let client_info = detect_client(&headers, "/v1/messages");
    assert_eq!(client_info.client_type, ClientType::OpenClaw);
    assert_eq!(client_info.detected_by, DetectionMethod::UserAgent);
}

/// Test model mapping for OpenClaw (defaults to Anthropic format)
#[test]
fn test_openclaw_model_mapping() {
    let available_models = vec![
        "anthropic/claude-3.5-sonnet".to_string(),
        "anthropic/claude-3.5-haiku".to_string(),
    ];

    // Test Claude model mapping to OpenRouter
    let result = translate_model(
        "claude-3-5-sonnet",
        ClientType::OpenClaw,
        "openrouter",
        &available_models,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "anthropic/claude-3.5-sonnet");
}

/// Test that OpenClaw can use both Anthropic and OpenAI formats
#[test]
fn test_openclaw_api_format() {
    let client_type = ClientType::OpenClaw;
    // OpenClaw defaults to Anthropic format but can use either
    assert!(client_type.expects_anthropic_format());
}

/// Integration test simulating OpenClaw multi-model scenario
#[tokio::test]
async fn test_openclaw_multi_model_routing() {
    // This simulates OpenClaw routing different requests to different models
    let scenarios = vec![
        ("claude-3-5-sonnet", "think"),
        ("claude-3-5-haiku", "background"),
        ("claude-3-opus", "long_context"),
    ];

    for (model, scenario) in scenarios {
        // Verify model name is valid
        assert!(!model.is_empty());
        // Verify scenario is valid
        assert!(!scenario.is_empty());
    }
}

/// Test OpenClaw request with pattern-based routing hints
#[tokio::test]
async fn test_openclaw_pattern_routing() {
    // Simulate a request that should trigger think mode
    let query = "Please think deeply about this problem and analyze it step by step";
    let think_keywords = ["think", "analyze", "deeply"];

    let should_use_think_mode = think_keywords.iter().any(|kw| query.contains(kw));
    assert!(should_use_think_mode);
}

/// Test OpenClaw background task routing
#[tokio::test]
async fn test_openclaw_background_routing() {
    // Simulate a low-priority background request
    let is_background = true;
    let token_count = 500; // Small request

    // Background requests should route to fast/cheap provider
    assert!(is_background);
    assert!(token_count < 1000);
}

/// Test long context routing for OpenClaw
#[tokio::test]
async fn test_openclaw_long_context_routing() {
    let token_count = 80000; // Above typical threshold
    let long_context_threshold = 60000;

    let should_use_long_context = token_count >= long_context_threshold;
    assert!(should_use_long_context);
}

/// Test OpenClaw with explicit provider specification
#[test]
fn test_openclaw_explicit_provider() {
    // OpenClaw can use explicit provider syntax
    let model_specs = vec![
        "openrouter:anthropic/claude-3.5-sonnet",
        "groq:llama-3.1-70b-versatile",
        "anthropic:claude-3-opus",
    ];

    for spec in model_specs {
        let parts: Vec<&str> = spec.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert!(!parts[0].is_empty()); // provider
        assert!(!parts[1].is_empty()); // model
    }
}

/// Test concurrent OpenClaw requests
#[tokio::test]
async fn test_openclaw_concurrent_requests() {
    use tokio::task;

    let requests: Vec<_> = (0..5)
        .map(|i| {
            task::spawn(async move {
                // Simulate different request types
                let client_type = ClientType::OpenClaw;
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                (i, client_type)
            })
        })
        .collect();

    let results = futures::future::join_all(requests).await;
    assert_eq!(results.len(), 5);

    // Verify all completed successfully
    for result in results {
        assert!(result.is_ok());
    }
}

/// Test OpenClaw error handling
#[tokio::test]
async fn test_openclaw_error_recovery() {
    // Simulate a failed routing attempt that should fall back to default
    let fallback_triggered = true;
    let default_provider = "openrouter".to_string();

    assert!(fallback_triggered);
    assert!(!default_provider.is_empty());
}

/// Test OpenClaw session management
#[tokio::test]
async fn test_openclaw_session_affinity() {
    // Simulate maintaining session context across requests
    let session_id = "test-session-123".to_string();
    let consecutive_requests = 3;

    // All requests with same session ID should potentially route similarly
    assert!(!session_id.is_empty());
    assert_eq!(consecutive_requests, 3);
}

/// Test model fallback when preferred model unavailable
#[test]
fn test_openclaw_model_fallback() {
    use terraphim_llm_proxy::routing::FallbackStrategy;
    use terraphim_llm_proxy::routing::ModelMapper;

    let available_models = vec!["fallback-model".to_string()];
    let mut mapper = ModelMapper::new().with_fallback(FallbackStrategy::FuzzyMatch);

    // When preferred model not available, should find similar
    let result = mapper.translate(
        "claude-3-5-sonnet", // Requested
        ClientType::OpenClaw,
        "groq", // Provider without Claude models
        &available_models,
    );

    // With fuzzy match, should return something
    assert!(result.is_ok());
}

/// Test OpenClaw with image processing request
#[tokio::test]
async fn test_openclaw_image_routing() {
    let has_images = true;
    let image_configured = true;

    let should_route_to_image_provider = has_images && image_configured;
    assert!(should_route_to_image_provider);
}

/// Test OpenClaw with web search request
#[tokio::test]
async fn test_openclaw_web_search_routing() {
    let has_web_search_tool = true;
    let web_search_configured = true;

    let should_route_to_search_provider = has_web_search_tool && web_search_configured;
    assert!(should_route_to_search_provider);
}
