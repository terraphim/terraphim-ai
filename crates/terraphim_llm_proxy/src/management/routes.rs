//! Management API route definitions.
//!
//! Defines all management endpoints with authentication middleware.

use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

use super::auth::{management_auth_middleware, ManagementAuthState};
use super::handlers::{
    create_api_key, delete_api_key, get_config, get_health, get_log_level, get_logs, get_metrics,
    list_api_keys, put_config, reload_config, set_log_level, ManagementState,
};
use super::ConfigManager;

/// Create the management API router with all endpoints.
///
/// All routes are protected by management authentication middleware.
///
/// # Arguments
/// * `config_manager` - Shared configuration manager
/// * `auth_state` - Authentication state for the management secret
///
/// # Endpoints
///
/// ## Configuration
/// - `GET  /v0/management/config` - Get current config (secrets redacted)
/// - `PUT  /v0/management/config` - Update configuration
/// - `POST /v0/management/config/reload` - Reload config from disk
///
/// ## API Keys
/// - `GET    /v0/management/api-keys` - List API keys
/// - `POST   /v0/management/api-keys` - Create new API key
/// - `DELETE /v0/management/api-keys/:key_id` - Delete API key
///
/// ## Logging
/// - `GET /v0/management/logs/level` - Get current log level
/// - `PUT /v0/management/logs/level` - Set log level
/// - `GET /v0/management/logs` - Get recent log entries
///
/// ## Health & Metrics
/// - `GET /v0/management/health` - Detailed health status
/// - `GET /v0/management/metrics` - Usage metrics
pub fn management_routes(
    config_manager: Arc<ConfigManager>,
    auth_state: ManagementAuthState,
) -> Router {
    let management_state = ManagementState::new(config_manager);

    Router::new()
        // Configuration endpoints
        .route("/v0/management/config", get(get_config).put(put_config))
        .route("/v0/management/config/reload", post(reload_config))
        // API key endpoints
        .route(
            "/v0/management/api-keys",
            get(list_api_keys).post(create_api_key),
        )
        .route("/v0/management/api-keys/:key_id", delete(delete_api_key))
        // Logging endpoints
        .route(
            "/v0/management/logs/level",
            get(get_log_level).put(set_log_level),
        )
        .route("/v0/management/logs", get(get_logs))
        // Health and metrics endpoints
        .route("/v0/management/health", get(get_health))
        .route("/v0/management/metrics", get(get_metrics))
        // Apply authentication middleware to all routes
        .layer(middleware::from_fn_with_state(
            auth_state,
            management_auth_middleware,
        ))
        // Apply management state to handlers that need it
        .with_state(management_state)
}

/// Create management routes without authentication (for testing).
#[cfg(test)]
pub fn management_routes_no_auth(config_manager: Arc<ConfigManager>) -> Router {
    let management_state = ManagementState::new(config_manager);

    Router::new()
        .route("/v0/management/config", get(get_config).put(put_config))
        .route("/v0/management/config/reload", post(reload_config))
        .route(
            "/v0/management/api-keys",
            get(list_api_keys).post(create_api_key),
        )
        .route("/v0/management/api-keys/:key_id", delete(delete_api_key))
        .route(
            "/v0/management/logs/level",
            get(get_log_level).put(set_log_level),
        )
        .route("/v0/management/logs", get(get_logs))
        .route("/v0/management/health", get(get_health))
        .route("/v0/management/metrics", get(get_metrics))
        .with_state(management_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ManagementSettings, OAuthSettings, Provider, ProxyConfig, ProxySettings, RouterSettings,
        SecuritySettings,
    };
    use crate::routing::RoutingStrategy;
    use crate::webhooks::WebhookSettings;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::path::PathBuf;
    use tempfile::NamedTempFile;
    use tower::ServiceExt;

    fn create_test_config() -> ProxyConfig {
        ProxyConfig {
            proxy: ProxySettings {
                host: "127.0.0.1".to_string(),
                port: 3456,
                api_key: "test-key".to_string(),
                timeout_ms: 60000,
            },
            router: RouterSettings {
                default: "openai,gpt-4".to_string(),
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
                name: "openai".to_string(),
                api_base_url: "https://api.openai.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                models: vec!["gpt-4".to_string()],
                transformers: vec![],
            }],
            security: SecuritySettings::default(),
            oauth: OAuthSettings::default(),
            management: ManagementSettings::default(),
            webhooks: WebhookSettings::default(),
        }
    }

    async fn setup_test_app() -> (Router, PathBuf, NamedTempFile) {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        config.save_toml(&temp_path).unwrap();

        let config_manager = Arc::new(ConfigManager::with_config(config, temp_path.clone()));
        let app = management_routes_no_auth(config_manager);

        // Return temp_file to keep it alive
        (app, temp_path, temp_file)
    }

    #[tokio::test]
    async fn test_get_config_redacts_secrets() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .uri("/v0/management/config")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Secrets should be redacted
        assert!(body_str.contains("[REDACTED]"));
        // Actual secrets should not appear
        assert!(!body_str.contains("test-key"));
        assert!(!body_str.contains("sk-test"));
        // Non-secrets should be present
        assert!(body_str.contains("127.0.0.1"));
        assert!(body_str.contains("3456"));
    }

    #[tokio::test]
    async fn test_reload_config() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .method("POST")
            .uri("/v0/management/config/reload")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"status\":\"ok\""));
        assert!(body_str.contains("reloaded"));
    }

    #[tokio::test]
    async fn test_list_api_keys() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .uri("/v0/management/api-keys")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"keys\""));
    }

    #[tokio::test]
    async fn test_create_api_key() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .method("POST")
            .uri("/v0/management/api-keys")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"key": "new-test-key"}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"status\":\"created\""));
        assert!(body_str.contains("\"id\""));
    }

    #[tokio::test]
    async fn test_delete_api_key() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .method("DELETE")
            .uri("/v0/management/api-keys/test-key-id")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"status\":\"deleted\""));
    }

    #[tokio::test]
    async fn test_get_log_level() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .uri("/v0/management/logs/level")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"level\""));
        assert!(body_str.contains("\"options\""));
    }

    #[tokio::test]
    async fn test_set_log_level() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .method("PUT")
            .uri("/v0/management/logs/level")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"level": "debug"}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"status\":\"ok\""));
        assert!(body_str.contains("\"current\":\"debug\""));
    }

    #[tokio::test]
    async fn test_set_log_level_invalid() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .method("PUT")
            .uri("/v0/management/logs/level")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"level": "invalid"}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_logs() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .uri("/v0/management/logs?lines=50")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_health() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .uri("/v0/management/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"status\":\"healthy\""));
        assert!(body_str.contains("\"version\""));
        assert!(body_str.contains("\"providers\""));
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let (app, _temp_path, _temp_file) = setup_test_app().await;

        let request = Request::builder()
            .uri("/v0/management/metrics")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("\"requests_total\""));
        assert!(body_str.contains("\"average_latency_ms\""));
    }

    #[tokio::test]
    async fn test_management_requires_auth() {
        let config = create_test_config();
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();
        config.save_toml(&temp_path).unwrap();

        let config_manager = Arc::new(ConfigManager::with_config(config, temp_path));
        let auth_state = ManagementAuthState::new("secret-key");

        let app = management_routes(config_manager, auth_state);

        // Request without auth should be rejected
        let request = Request::builder()
            .uri("/v0/management/config")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Request with valid auth should succeed
        let request = Request::builder()
            .uri("/v0/management/config")
            .header("X-Management-Key", "secret-key")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
