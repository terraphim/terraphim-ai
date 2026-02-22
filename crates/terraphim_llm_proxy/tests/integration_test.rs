//! Integration tests for complete request pipeline
//!
//! These tests validate the entire request flow from HTTP to LLM response.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt as HttpBodyExt;
use serde_json::json;
use terraphim_llm_proxy::{
    config::{
        ManagementSettings, OAuthSettings, Provider, ProxyConfig, ProxySettings, RouterSettings,
        SecuritySettings,
    },
    routing::RoutingStrategy,
    server::create_server,
    webhooks::WebhookSettings,
};
use tower::util::ServiceExt;

/// Create a test configuration for integration testing
fn create_integration_config() -> ProxyConfig {
    ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 3456,
            api_key: "test_api_key_for_integration_testing_12345".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "test,test-model".to_string(),
            background: Some("test,background-model".to_string()),
            think: Some("test,reasoning-model".to_string()),
            plan_implementation: None,
            long_context: Some("test,long-context-model".to_string()),
            long_context_threshold: 60000,
            web_search: Some("test,search-model".to_string()),
            image: Some("test,vision-model".to_string()),
            model_mappings: vec![],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![Provider {
            name: "test".to_string(),
            api_base_url: "http://localhost:11434/v1/chat/completions".to_string(),
            api_key: "test".to_string(),
            models: vec![
                "test-model".to_string(),
                "background-model".to_string(),
                "reasoning-model".to_string(),
                "long-context-model".to_string(),
                "search-model".to_string(),
                "vision-model".to_string(),
            ],
            transformers: vec!["anthropic".to_string()],
        }],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    let config = create_integration_config();
    let app = create_server(config).await.unwrap();

    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_count_tokens_endpoint() {
    let config = create_integration_config();
    let app = create_server(config).await.unwrap();

    let chat_request = json!({
        "model": "test-model",
        "messages": [
            {"role": "user", "content": "Hello, world!"}
        ]
    });

    let request = Request::builder()
        .uri("/v1/messages/count_tokens")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-api-key", "test_api_key_for_integration_testing_12345")
        .body(Body::from(serde_json::to_string(&chat_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(result.get("input_tokens").is_some());
    let tokens = result["input_tokens"].as_u64().unwrap();
    assert!(tokens > 0, "Expected positive token count, got {}", tokens);
}

#[tokio::test]
async fn test_authentication_required() {
    let config = create_integration_config();
    let app = create_server(config).await.unwrap();

    let chat_request = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Test"}]
    });

    let request = Request::builder()
        .uri("/v1/messages/count_tokens")
        .method("POST")
        .header("content-type", "application/json")
        // No API key provided
        .body(Body::from(serde_json::to_string(&chat_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_invalid_api_key_rejected() {
    let config = create_integration_config();
    let app = create_server(config).await.unwrap();

    let chat_request = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Test"}]
    });

    let request = Request::builder()
        .uri("/v1/messages/count_tokens")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-api-key", "wrong_api_key")
        .body(Body::from(serde_json::to_string(&chat_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_request_analysis_and_token_counting() {
    let config = create_integration_config();
    let app = create_server(config).await.unwrap();

    // Create a request with system prompt and tools
    let chat_request = json!({
        "model": "test-model",
        "messages": [
            {"role": "user", "content": "What's the weather?"}
        ],
        "system": "You are a helpful assistant.",
        "tools": [{
            "name": "get_weather",
            "description": "Get weather information",
            "input_schema": {
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                }
            }
        }],
        "max_tokens": 1024
    });

    let request = Request::builder()
        .uri("/v1/messages/count_tokens")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-api-key", "test_api_key_for_integration_testing_12345")
        .body(Body::from(serde_json::to_string(&chat_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let tokens = result["input_tokens"].as_u64().unwrap();
    // Should count: messages + system prompt + tool definition
    assert!(
        tokens > 20,
        "Expected >20 tokens with system and tools, got {}",
        tokens
    );
}

#[tokio::test]
async fn test_bearer_token_authentication() {
    let config = create_integration_config();
    let app = create_server(config).await.unwrap();

    let chat_request = json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Test"}]
    });

    let request = Request::builder()
        .uri("/v1/messages/count_tokens")
        .method("POST")
        .header("content-type", "application/json")
        .header(
            "authorization",
            "Bearer test_api_key_for_integration_testing_12345",
        )
        .body(Body::from(serde_json::to_string(&chat_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
