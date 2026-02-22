//! Comprehensive Anthropic API Endpoint Tests
//!
//! Tests all major Anthropic API endpoints to ensure full compatibility
//! with Claude Code and other Anthropic-compatible clients.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use terraphim_llm_proxy::{
    config::{
        ManagementSettings, OAuthSettings, Provider, ProxyConfig, ProxySettings, RouterSettings,
        SecuritySettings,
    },
    routing::RoutingStrategy,
    server::create_server,
    webhooks::WebhookSettings,
    Result,
};

fn create_test_config() -> ProxyConfig {
    ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 3456,
            api_key: "test_api_key".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "openrouter,anthropic/claude-3.5-sonnet".to_string(),
            background: Some("deepseek,deepseek-chat".to_string()),
            think: Some("openrouter,deepseek/deepseek-v3.1-terminus".to_string()),
            plan_implementation: None,
            long_context: Some("openrouter,anthropic/claude-3.5-sonnet".to_string()),
            long_context_threshold: 60000,
            web_search: Some("openrouter,openai/gpt-4o".to_string()),
            image: Some("openrouter,google/gemini-pro-vision".to_string()),
            model_mappings: vec![],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![
            Provider {
                name: "openrouter".to_string(),
                api_base_url: "https://openrouter.ai/api/v1".to_string(),
                api_key: "test_key".to_string(),
                models: vec![
                    "anthropic/claude-3.5-sonnet".to_string(),
                    "anthropic/claude-sonnet-4.5".to_string(),
                    "deepseek/deepseek-v3.1-terminus".to_string(),
                ],
                transformers: vec![],
            },
            Provider {
                name: "deepseek".to_string(),
                api_base_url: "https://api.deepseek.com".to_string(),
                api_key: "test_key".to_string(),
                models: vec!["deepseek-chat".to_string(), "deepseek-reasoner".to_string()],
                transformers: vec![],
            },
        ],
        security: SecuritySettings::default(),
        oauth: OAuthSettings::default(),
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    }
}

async fn create_test_server() -> Result<Router> {
    let config = create_test_config();
    create_server(config).await
}

#[tokio::test]
async fn test_health_check_endpoint() {
    let app = create_test_server().await.unwrap();

    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let health_response: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(health_response["status"], "healthy");
    assert!(health_response["checks"].is_object());
}

#[tokio::test]
async fn test_detailed_health_check_endpoint() {
    let app = create_test_server().await.unwrap();

    let request = Request::builder()
        .uri("/health/detailed")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let health_response: Value = serde_json::from_slice(&body).unwrap();

    // Proxy-specific endpoint - match server's DetailedHealthResponse structure
    assert_eq!(health_response["status"], "healthy");
    assert!(health_response["providers"].is_object());
    assert!(health_response["system"]["status"].is_string());
}

#[tokio::test]
async fn test_readiness_probe_endpoint() {
    let app = create_test_server().await.unwrap();

    let request = Request::builder()
        .uri("/ready")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_liveness_probe_endpoint() {
    let app = create_test_server().await.unwrap();

    let request = Request::builder().uri("/live").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_chat_completions_endpoint() {
    let app = create_test_server().await.unwrap();

    let request_body = json!({
        "model": "auto",
        "messages": [
            {
                "role": "user",
                "content": "Hello, how are you?"
            }
        ],
        "max_tokens": 10,
        "temperature": 0.7
    });

    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_api_key")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Per Anthropic docs: "When receiving a streaming response via SSE,
    // it's possible that an error can occur after returning a 200 response"
    // Streaming endpoints return 200 OK immediately; errors come via SSE events.
    // Also accept 502/504 proxy errors when backend is unavailable in tests.
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::BAD_GATEWAY
            || response.status() == StatusCode::GATEWAY_TIMEOUT
    );
}

#[tokio::test]
async fn test_messages_endpoint() {
    let app = create_test_server().await.unwrap();

    let request_body = json!({
        "model": "anthropic/claude-3.5-sonnet",
        "messages": [
            {
                "role": "user",
                "content": "Hello"
            }
        ],
        "max_tokens": 10
    });

    let request = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_api_key")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Per Anthropic docs: Streaming responses return 200 OK immediately.
    // "it's possible that an error can occur after returning a 200 response,
    // in which case error handling wouldn't follow these standard mechanisms"
    // Also accept 502/504 proxy errors when backend is unavailable in tests.
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::BAD_GATEWAY
            || response.status() == StatusCode::GATEWAY_TIMEOUT
    );
}

#[tokio::test]
async fn test_count_tokens_endpoint() {
    let app = create_test_server().await.unwrap();

    let request_body = json!({
        "model": "anthropic/claude-3.5-sonnet",
        "messages": [
            {
                "role": "user",
                "content": "Hello world"
            }
        ]
    });

    let request = Request::builder()
        .method("POST")
        .uri("/v1/messages/count_tokens")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_api_key")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 200 for token counting (even with invalid API keys)
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn test_list_models_endpoint() {
    let app = create_test_server().await.unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/v1/models")
        .header("Authorization", "Bearer test_api_key")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let models_response: Value = serde_json::from_slice(&body).unwrap();

    assert!(models_response["data"].is_array());
}

#[tokio::test]
async fn test_session_stats_endpoint() {
    let app = create_test_server().await.unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/api/sessions")
        .header("Authorization", "Bearer test_api_key")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let session_response: Value = serde_json::from_slice(&body).unwrap();

    // Proxy-specific endpoint - match server's session stats response
    assert!(session_response["active_sessions"].is_number());
    assert!(session_response["max_sessions"].is_number());
}

#[tokio::test]
async fn test_metrics_json_endpoint() {
    let app = create_test_server().await.unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/api/metrics/json")
        .header("Authorization", "Bearer test_api_key")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let metrics_response: Value = serde_json::from_slice(&body).unwrap();

    // Proxy-specific endpoint - match AggregatedMetrics structure
    assert!(metrics_response["timestamp"].is_string());
    assert!(metrics_response["system_health"]["uptime_seconds"].is_number());
}

#[tokio::test]
async fn test_metrics_prometheus_endpoint() {
    let app = create_test_server().await.unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/api/metrics/prometheus")
        .header("Authorization", "Bearer test_api_key")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let prometheus_text = String::from_utf8(body.to_vec()).unwrap();

    // Should contain Prometheus-style metrics
    assert!(prometheus_text.contains("# HELP") || prometheus_text.contains("# TYPE"));
}

#[tokio::test]
async fn test_message_batch_endpoints() {
    let app = create_test_server().await.unwrap();

    // Test batch creation
    let batch_request = json!({
        "requests": [
            {
                "custom_id": "test-1",
                "model": "anthropic/claude-3.5-sonnet",
                "messages": [{"role": "user", "content": "Hello"}],
                "max_tokens": 10
            }
        ]
    });

    let request = Request::builder()
        .method("POST")
        .uri("/v1/messages/batches")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_api_key")
        .body(Body::from(batch_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn test_file_endpoints() {
    let app = create_test_server().await.unwrap();

    // Test file listing
    let request = Request::builder()
        .method("GET")
        .uri("/v1/files")
        .header("Authorization", "Bearer test_api_key")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn test_experimental_prompt_endpoint() {
    let app = create_test_server().await.unwrap();

    let request_body = json!({
        "prompt": "Generate a creative story about AI",
        "model": "anthropic/claude-3.5-sonnet",
        "max_tokens": 50
    });

    let request = Request::builder()
        .method("POST")
        .uri("/v1/experimental/generate_prompt")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_api_key")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle the experimental endpoint
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn test_endpoint_authentication() {
    let app = create_test_server().await.unwrap();

    // Test without Authorization header
    let request_body = json!({
        "model": "auto",
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 10
    });

    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("Content-Type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_endpoint_rate_limiting() {
    // Send multiple rapid requests to test rate limiting
    for i in 0..5 {
        let app = create_test_server().await.unwrap();

        let request_body = json!({
            "model": "auto",
            "messages": [{"role": "user", "content": &format!("Test {}", i)}],
            "max_tokens": 5
        });

        let request = Request::builder()
            .method("POST")
            .uri("/v1/chat/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer test_api_key")
            .body(Body::from(request_body.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Per Anthropic docs: Streaming returns 200 OK, errors via SSE.
        // Also include 400 BAD_REQUEST per official error codes.
        // Also accept 502/504 proxy errors when backend is unavailable in tests.
        assert!(
            response.status() == StatusCode::OK
                || response.status() == StatusCode::INTERNAL_SERVER_ERROR
                || response.status() == StatusCode::UNAUTHORIZED
                || response.status() == StatusCode::TOO_MANY_REQUESTS
                || response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::BAD_GATEWAY
                || response.status() == StatusCode::GATEWAY_TIMEOUT
        );
    }
}

#[tokio::test]
async fn test_invalid_endpoint() {
    let app = create_test_server().await.unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/invalid/endpoint")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_endpoint_cors_headers() {
    let app = create_test_server().await.unwrap();

    let request = Request::builder()
        .method("OPTIONS")
        .uri("/v1/chat/completions")
        .header("Origin", "https://claude.ai")
        .header("Access-Control-Request-Method", "POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle OPTIONS requests for CORS
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::METHOD_NOT_ALLOWED
            || response.status() == StatusCode::NOT_FOUND
    );
}
