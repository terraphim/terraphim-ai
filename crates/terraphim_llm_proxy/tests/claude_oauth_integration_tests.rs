//! Integration tests for Claude OAuth API flow.
//!
//! Tests that the server correctly handles Claude OAuth configuration
//! for both Bearer token and API key creation modes.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt as HttpBodyExt;
use terraphim_llm_proxy::{
    config::{
        ManagementSettings, OAuthProviderSettings, OAuthSettings, Provider, ProxyConfig,
        ProxySettings, RouterSettings, SecuritySettings,
    },
    routing::RoutingStrategy,
    server::create_server,
    webhooks::WebhookSettings,
};
use tower::util::ServiceExt;

/// Create a test config with Claude OAuth in bearer mode.
fn create_claude_bearer_config() -> ProxyConfig {
    ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 0,
            api_key: "test-api-key".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "anthropic,claude-3-5-sonnet".to_string(),
            background: None,
            think: None,
            plan_implementation: None,
            long_context: None,
            long_context_threshold: 60000,
            web_search: None,
            image: None,
            model_mappings: vec![],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![Provider {
            name: "anthropic".to_string(),
            api_base_url: "https://api.anthropic.com/v1".to_string(),
            api_key: "sk-ant-test-fallback-key".to_string(),
            models: vec!["claude-3-5-sonnet".to_string()],
            transformers: vec![],
        }],
        security: SecuritySettings::default(),
        oauth: OAuthSettings {
            storage_backend: "file".to_string(),
            redis_url: None,
            storage_path: None,
            claude: OAuthProviderSettings {
                enabled: true,
                callback_port: 54545,
                client_id: Some("test-client-id".to_string()),
                client_secret: None,
                auth_mode: Some("bearer".to_string()),
                scopes: Some(vec![
                    "user:inference".to_string(),
                    "user:profile".to_string(),
                ]),
                anthropic_beta: Some("oauth-2025-04-20".to_string()),
            },
            gemini: OAuthProviderSettings::default(),
            openai: OAuthProviderSettings::default(),
            copilot: OAuthProviderSettings::default(),
        },
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    }
}

/// Create a test config with Claude OAuth in api_key mode.
fn create_claude_api_key_config() -> ProxyConfig {
    ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 0,
            api_key: "test-api-key".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "anthropic,claude-3-5-sonnet".to_string(),
            background: None,
            think: None,
            plan_implementation: None,
            long_context: None,
            long_context_threshold: 60000,
            web_search: None,
            image: None,
            model_mappings: vec![],
            model_exclusions: vec![],
            strategy: RoutingStrategy::default(),
        },
        providers: vec![Provider {
            name: "anthropic".to_string(),
            api_base_url: "https://api.anthropic.com/v1".to_string(),
            api_key: "sk-ant-test-fallback-key".to_string(),
            models: vec!["claude-3-5-sonnet".to_string()],
            transformers: vec![],
        }],
        security: SecuritySettings::default(),
        oauth: OAuthSettings {
            storage_backend: "file".to_string(),
            redis_url: None,
            storage_path: None,
            claude: OAuthProviderSettings {
                enabled: true,
                callback_port: 54545,
                client_id: Some("test-client-id".to_string()),
                client_secret: None,
                auth_mode: Some("api_key".to_string()),
                scopes: Some(vec![
                    "org:create_api_key".to_string(),
                    "user:profile".to_string(),
                    "user:inference".to_string(),
                ]),
                anthropic_beta: None,
            },
            gemini: OAuthProviderSettings::default(),
            openai: OAuthProviderSettings::default(),
            copilot: OAuthProviderSettings::default(),
        },
        management: ManagementSettings::default(),
        webhooks: WebhookSettings::default(),
    }
}

#[tokio::test]
async fn test_server_starts_with_bearer_config() {
    let config = create_claude_bearer_config();
    let app = create_server(config)
        .await
        .expect("Server should start with bearer config");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_server_starts_with_api_key_config() {
    let config = create_claude_api_key_config();
    let app = create_server(config)
        .await
        .expect("Server should start with api_key config");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_claude_oauth_login_available_in_bearer_mode() {
    let config = create_claude_bearer_config();
    let app = create_server(config).await.unwrap();

    // POST to login should return auth URL
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/claude/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify auth URL contains the correct scopes
    let auth_url = json["auth_url"].as_str().unwrap();
    assert!(auth_url.contains("user%3Ainference"));
    assert!(auth_url.contains("user%3Aprofile"));
}

#[tokio::test]
async fn test_claude_oauth_login_available_in_api_key_mode() {
    let config = create_claude_api_key_config();
    let app = create_server(config).await.unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/claude/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify auth URL contains api_key scopes
    let auth_url = json["auth_url"].as_str().unwrap();
    assert!(auth_url.contains("org%3Acreate_api_key"));
}

#[tokio::test]
async fn test_claude_oauth_providers_lists_claude() {
    let config = create_claude_bearer_config();
    let app = create_server(config).await.unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/oauth/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let providers = json.as_array().expect("response should be a JSON array");
    let claude = providers
        .iter()
        .find(|p| p["id"].as_str() == Some("claude"))
        .expect("claude should be in providers list");
    assert_eq!(claude["name"].as_str(), Some("Claude (Anthropic)"));
}
