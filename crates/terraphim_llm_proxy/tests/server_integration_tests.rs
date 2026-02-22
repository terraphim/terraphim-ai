//! Server integration tests for Issue #56
//!
//! Tests verify:
//! - Server starts with all features enabled
//! - OAuth routes mounted when providers enabled
//! - Management API routes accessible when enabled
//! - Model aliasing works in request flow

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt as HttpBodyExt;
use terraphim_llm_proxy::{
    config::{
        ManagementSettings, OAuthProviderSettings, OAuthSettings, Provider, ProxyConfig,
        ProxySettings, RouterSettings, SecuritySettings,
    },
    routing::{ModelExclusion, ModelMapping, RoutingStrategy},
    server::create_server,
    webhooks::WebhookSettings,
};
use tower::util::ServiceExt;

/// Create a test configuration with all features enabled
fn create_full_feature_config() -> ProxyConfig {
    ProxyConfig {
        proxy: ProxySettings {
            host: "127.0.0.1".to_string(),
            port: 3456,
            api_key: "test_api_key_server_integration".to_string(),
            timeout_ms: 60000,
        },
        router: RouterSettings {
            default: "test,test-model".to_string(),
            background: Some("test,background-model".to_string()),
            think: Some("test,reasoning-model".to_string()),
            plan_implementation: Some("test,plan-model".to_string()),
            long_context: Some("test,long-context-model".to_string()),
            long_context_threshold: 60000,
            web_search: Some("test,search-model".to_string()),
            image: Some("test,vision-model".to_string()),
            model_mappings: vec![
                // Add model alias mappings for testing
                ModelMapping {
                    from: "gpt-4".to_string(),
                    to: "test,claude-3-opus".to_string(),
                    bidirectional: true,
                },
                ModelMapping {
                    from: "gpt-3.5-turbo".to_string(),
                    to: "test,claude-3-haiku".to_string(),
                    bidirectional: false,
                },
            ],
            model_exclusions: vec![ModelExclusion {
                provider: "test".to_string(),
                patterns: vec!["deprecated-*".to_string()],
            }],
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
                "claude-3-opus".to_string(),
                "claude-3-haiku".to_string(),
            ],
            transformers: vec!["anthropic".to_string()],
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
                client_secret: Some("test-client-secret".to_string()),
                ..Default::default()
            },
            gemini: OAuthProviderSettings::default(),
            openai: OAuthProviderSettings::default(),
            copilot: OAuthProviderSettings::default(),
        },
        management: ManagementSettings {
            enabled: true,
            secret_key: Some("test-management-secret".to_string()),
            allow_remote: false,
            ..Default::default()
        },
        webhooks: WebhookSettings::default(),
    }
}

/// Create a config with OAuth disabled
fn create_oauth_disabled_config() -> ProxyConfig {
    let mut config = create_full_feature_config();
    config.oauth.claude.enabled = false;
    config.oauth.gemini.enabled = false;
    config.oauth.copilot.enabled = false;
    config
}

/// Create a config with management disabled
fn create_management_disabled_config() -> ProxyConfig {
    let mut config = create_full_feature_config();
    config.management.enabled = false;
    config
}

/// Create config that includes MiniMax route-chain fallback.
fn create_minimax_route_chain_config() -> ProxyConfig {
    let mut config = create_full_feature_config();
    config.router.default = "openai-codex,gpt-5.2-codex|minimax,MiniMax-M2.5".to_string();
    config.router.think = Some("minimax,MiniMax-M2.5".to_string());

    config.providers = vec![
        Provider {
            name: "openai-codex".to_string(),
            api_base_url: "https://api.openai.com/v1".to_string(),
            api_key: "test-codex-key".to_string(),
            models: vec!["gpt-5.2-codex".to_string()],
            transformers: vec!["openai-codex".to_string()],
        },
        Provider {
            name: "minimax".to_string(),
            api_base_url: "https://api.minimax.io/anthropic".to_string(),
            api_key: "test-minimax-key".to_string(),
            models: vec!["MiniMax-M2.5".to_string()],
            transformers: vec!["anthropic".to_string()],
        },
    ];

    config
}

// ============================================================================
// Server Initialization Tests
// ============================================================================

#[tokio::test]
async fn test_server_starts_with_all_features_enabled() {
    let config = create_full_feature_config();
    let result = create_server(config).await;
    assert!(
        result.is_ok(),
        "Server should start with all features enabled"
    );
}

#[tokio::test]
async fn test_server_starts_with_oauth_disabled() {
    let config = create_oauth_disabled_config();
    let result = create_server(config).await;
    assert!(result.is_ok(), "Server should start with OAuth disabled");
}

#[tokio::test]
async fn test_server_starts_with_management_disabled() {
    let config = create_management_disabled_config();
    let result = create_server(config).await;
    assert!(
        result.is_ok(),
        "Server should start with Management API disabled"
    );
}

// ============================================================================
// OAuth Route Tests
// ============================================================================

#[tokio::test]
async fn test_oauth_status_endpoint_exists_when_enabled() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    // Try to poll OAuth status for a non-existent flow
    let request = Request::builder()
        .uri("/oauth/claude/status?state=nonexistent")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return JSON response (even for unknown flows)
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "OAuth status endpoint should be mounted"
    );

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Unknown flow should return error status
    assert!(
        result.get("status").is_some(),
        "Response should have status field"
    );
}

#[tokio::test]
async fn test_oauth_callback_endpoint_exists_when_enabled() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    // Try OAuth callback without proper parameters
    let request = Request::builder()
        .uri("/oauth/claude/callback")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return error page (HTML) because no code/state provided
    // But the endpoint exists and responds
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::BAD_REQUEST,
        "OAuth callback endpoint should be mounted"
    );
}

#[tokio::test]
async fn test_oauth_routes_not_mounted_when_disabled() {
    let config = create_oauth_disabled_config();
    let app = create_server(config).await.unwrap();

    let request = Request::builder()
        .uri("/oauth/claude/status?state=test")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 404 (route not found) or 401 (proxy auth catches it first)
    // when OAuth is disabled - the key is it should NOT return 200 OK
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::UNAUTHORIZED,
        "OAuth routes should not return OK when disabled, got: {:?}",
        response.status()
    );
}

// ============================================================================
// Management API Route Tests
// ============================================================================

#[tokio::test]
async fn test_management_health_endpoint_exists_when_enabled() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    // Management health requires authentication (all management routes are protected)
    let request = Request::builder()
        .uri("/v0/management/health")
        .method("GET")
        .header("x-management-key", "test-management-secret")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Management health endpoint should be accessible with auth"
    );
}

#[tokio::test]
async fn test_management_config_requires_auth() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    let request = Request::builder()
        .uri("/v0/management/config")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Config endpoint requires authentication
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Management config should require authentication"
    );
}

#[tokio::test]
async fn test_management_config_accessible_with_auth() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    let request = Request::builder()
        .uri("/v0/management/config")
        .method("GET")
        .header("x-management-key", "test-management-secret")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be accessible with correct auth
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Management config should be accessible with valid auth"
    );
}

#[tokio::test]
async fn test_management_routes_not_mounted_when_disabled() {
    let config = create_management_disabled_config();
    let app = create_server(config).await.unwrap();

    let request = Request::builder()
        .uri("/v0/management/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 404 when Management API is disabled
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Management routes should not be mounted when disabled"
    );
}

// ============================================================================
// Model Aliasing Tests
// ============================================================================

#[tokio::test]
async fn test_model_aliasing_config_present() {
    let config = create_full_feature_config();

    // Verify the model mappings are configured correctly
    assert_eq!(config.router.model_mappings.len(), 2);
    assert_eq!(config.router.model_mappings[0].from, "gpt-4");
    assert_eq!(config.router.model_mappings[0].to, "test,claude-3-opus");
    assert_eq!(config.router.model_mappings[1].from, "gpt-3.5-turbo");
    assert_eq!(config.router.model_mappings[1].to, "test,claude-3-haiku");
}

#[tokio::test]
async fn test_model_exclusions_config_present() {
    let config = create_full_feature_config();

    // Verify model exclusions are configured
    assert_eq!(config.router.model_exclusions.len(), 1);
    assert_eq!(config.router.model_exclusions[0].provider, "test");
    assert!(config.router.model_exclusions[0]
        .patterns
        .contains(&"deprecated-*".to_string()));
}

// ============================================================================
// Feature Flag Tests
// ============================================================================

#[tokio::test]
async fn test_oauth_feature_flags() {
    let config = create_full_feature_config();

    // Claude OAuth enabled
    assert!(config.oauth.claude.enabled);
    assert!(config.oauth.claude.client_id.is_some());

    // Others disabled by default
    assert!(!config.oauth.gemini.enabled);
    assert!(!config.oauth.copilot.enabled);
}

#[tokio::test]
async fn test_management_feature_flags() {
    let config = create_full_feature_config();

    assert!(config.management.enabled);
    assert!(config.management.secret_key.is_some());
    assert!(!config.management.allow_remote); // Remote access disabled for tests
}

#[tokio::test]
async fn test_minimax_route_chain_config_starts_server() {
    let config = create_minimax_route_chain_config();
    let app = create_server(config.clone()).await;
    assert!(
        app.is_ok(),
        "Server should start with MiniMax route chain config"
    );

    assert_eq!(
        config.router.default,
        "openai-codex,gpt-5.2-codex|minimax,MiniMax-M2.5"
    );
    assert_eq!(config.router.think.as_deref(), Some("minimax,MiniMax-M2.5"));
    assert!(config
        .providers
        .iter()
        .any(|p| p.name == "minimax" && p.models.contains(&"MiniMax-M2.5".to_string())));
}

// ============================================================================
// Health Check with Features Enabled
// ============================================================================

#[tokio::test]
async fn test_health_check_with_all_features() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Health check should pass with all features enabled"
    );

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(result["status"], "healthy");
}

// ============================================================================
// Authentication Tests for /v1/chat/completions (Issue #63)
// ============================================================================

#[tokio::test]
async fn test_streaming_chat_completions_requires_auth() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    // Send streaming request WITHOUT authentication headers
    let request_body = serde_json::json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Hello"}],
        "stream": true
    });

    let request = Request::builder()
        .uri("/v1/chat/completions")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 401 Unauthorized (MissingApiKey error)
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Streaming /v1/chat/completions without auth should return 401"
    );
}

#[tokio::test]
async fn test_non_streaming_chat_completions_requires_auth() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    // Send non-streaming request WITHOUT authentication headers
    let request_body = serde_json::json!({
        "model": "test-model",
        "messages": [{"role": "user", "content": "Hello"}],
        "stream": false
    });

    let request = Request::builder()
        .uri("/v1/chat/completions")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Non-streaming /v1/chat/completions without auth should return 401"
    );
}

// ============================================================================
// OAuth Login and Providers Endpoint Tests (Issue #94)
// ============================================================================

#[tokio::test]
async fn test_oauth_login_endpoint_returns_auth_url() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    let request = Request::builder()
        .uri("/oauth/claude/login")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "OAuth login endpoint should return 200"
    );

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(
        result.get("auth_url").is_some(),
        "Login response should contain auth_url"
    );
    assert!(
        result.get("state").is_some(),
        "Login response should contain state"
    );
    assert_eq!(
        result["provider"], "claude",
        "Login response should identify provider as claude"
    );
}

#[tokio::test]
async fn test_oauth_login_unknown_provider_returns_404() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    let request = Request::builder()
        .uri("/oauth/unknown/login")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Login with unknown provider should return 404"
    );

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(
        result.get("error").is_some(),
        "Error response should contain error field"
    );
}

#[tokio::test]
async fn test_oauth_providers_endpoint_lists_enabled_providers() {
    let config = create_full_feature_config();
    let app = create_server(config).await.unwrap();

    let request = Request::builder()
        .uri("/oauth/providers")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Providers endpoint should return 200"
    );

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let providers = result.as_array().expect("Response should be a JSON array");
    assert!(
        !providers.is_empty(),
        "Should have at least one enabled provider"
    );

    let has_claude = providers.iter().any(|p| p["id"] == "claude");
    assert!(has_claude, "Claude should be listed as an enabled provider");
}

#[tokio::test]
async fn test_oauth_providers_endpoint_empty_when_disabled() {
    let config = create_oauth_disabled_config();
    let app = create_server(config).await.unwrap();

    let request = Request::builder()
        .uri("/oauth/providers")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // When OAuth is disabled, routes are not mounted so we get 404
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::UNAUTHORIZED,
        "OAuth providers should not be accessible when disabled, got: {:?}",
        response.status()
    );
}
