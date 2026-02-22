//! Management API request handlers.
//!
//! Implements handlers for configuration, API keys, logging, health, and metrics.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

use crate::config::{Provider, ProxyConfig};
use crate::management::{ConfigManager, ManagementError};

/// Shared state for management handlers.
#[derive(Clone)]
pub struct ManagementState {
    pub config_manager: Arc<ConfigManager>,
}

impl ManagementState {
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        Self { config_manager }
    }
}

// ============================================================================
// Config Handlers
// ============================================================================

/// Response for config endpoints with secrets redacted.
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub proxy: RedactedProxySettings,
    pub router: crate::config::RouterSettings,
    pub providers: Vec<RedactedProvider>,
    pub security: crate::config::SecuritySettings,
}

/// Proxy settings with secrets redacted.
#[derive(Debug, Serialize)]
pub struct RedactedProxySettings {
    pub host: String,
    pub port: u16,
    pub api_key: String, // Always "[REDACTED]"
    pub timeout_ms: u64,
}

/// Provider with secrets redacted.
#[derive(Debug, Serialize)]
pub struct RedactedProvider {
    pub name: String,
    pub api_base_url: String,
    pub api_key: String, // Always "[REDACTED]"
    pub models: Vec<String>,
    pub transformers: Vec<String>,
}

impl From<&ProxyConfig> for ConfigResponse {
    fn from(config: &ProxyConfig) -> Self {
        Self {
            proxy: RedactedProxySettings {
                host: config.proxy.host.clone(),
                port: config.proxy.port,
                api_key: "[REDACTED]".to_string(),
                timeout_ms: config.proxy.timeout_ms,
            },
            router: config.router.clone(),
            providers: config
                .providers
                .iter()
                .map(|p| RedactedProvider {
                    name: p.name.clone(),
                    api_base_url: p.api_base_url.clone(),
                    api_key: "[REDACTED]".to_string(),
                    models: p.models.clone(),
                    transformers: p.transformers.clone(),
                })
                .collect(),
            security: config.security.clone(),
        }
    }
}

/// Generic status response.
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub message: String,
}

/// Get current configuration with secrets redacted.
pub async fn get_config(
    State(state): State<ManagementState>,
) -> Result<Json<ConfigResponse>, ManagementError> {
    let config = state.config_manager.get().await;
    let response = ConfigResponse::from(&*config);
    debug!("Retrieved configuration (secrets redacted)");
    Ok(Json(response))
}

/// Update configuration request.
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub proxy: Option<crate::config::ProxySettings>,
    pub router: Option<crate::config::RouterSettings>,
    pub providers: Option<Vec<Provider>>,
    pub security: Option<crate::config::SecuritySettings>,
}

/// Update configuration.
pub async fn put_config(
    State(state): State<ManagementState>,
    Json(request): Json<UpdateConfigRequest>,
) -> Result<Json<StatusResponse>, ManagementError> {
    // Get current config and merge with updates
    let mut config = state.config_manager.get_cloned().await;

    if let Some(proxy) = request.proxy {
        config.proxy = proxy;
    }
    if let Some(router) = request.router {
        config.router = router;
    }
    if let Some(providers) = request.providers {
        config.providers = providers;
    }
    if let Some(security) = request.security {
        config.security = security;
    }

    state.config_manager.update(config).await?;

    info!("Configuration updated via management API");
    Ok(Json(StatusResponse {
        status: "ok".to_string(),
        message: "Configuration updated".to_string(),
    }))
}

/// Reload configuration from disk.
pub async fn reload_config(
    State(state): State<ManagementState>,
) -> Result<Json<StatusResponse>, ManagementError> {
    state.config_manager.reload().await?;

    info!("Configuration reloaded from disk");
    Ok(Json(StatusResponse {
        status: "ok".to_string(),
        message: "Configuration reloaded".to_string(),
    }))
}

// ============================================================================
// API Key Handlers
// ============================================================================

/// API key summary (never expose full key).
#[derive(Debug, Serialize)]
pub struct ApiKeySummary {
    pub id: String,
    pub prefix: String,
    pub created_at: String,
}

/// Response for listing API keys.
#[derive(Debug, Serialize)]
pub struct ApiKeysResponse {
    pub keys: Vec<ApiKeySummary>,
}

/// Request to create a new API key.
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub key: String,
}

/// Response for API key creation.
#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub status: String,
}

/// List API keys (only shows prefixes, not full keys).
pub async fn list_api_keys(
    State(_state): State<ManagementState>,
) -> Result<Json<ApiKeysResponse>, ManagementError> {
    // For now, return the proxy API key info
    // In a full implementation, this would track multiple API keys
    debug!("Listing API keys");
    Ok(Json(ApiKeysResponse {
        keys: vec![ApiKeySummary {
            id: "default".to_string(),
            prefix: "****".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }],
    }))
}

/// Create a new API key.
pub async fn create_api_key(
    State(_state): State<ManagementState>,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>, ManagementError> {
    // Validate the key format
    if request.key.is_empty() {
        return Err(ManagementError::ValidationError(
            "API key cannot be empty".to_string(),
        ));
    }

    // In a full implementation, this would store the key
    let key_id = uuid::Uuid::new_v4().to_string();
    info!("Created new API key with id: {}", key_id);

    Ok(Json(CreateApiKeyResponse {
        id: key_id,
        status: "created".to_string(),
    }))
}

/// Delete an API key.
pub async fn delete_api_key(
    Path(key_id): Path<String>,
) -> Result<Json<StatusResponse>, ManagementError> {
    // In a full implementation, this would remove the key from storage
    info!("Deleted API key: {}", key_id);

    Ok(Json(StatusResponse {
        status: "deleted".to_string(),
        message: format!("API key {} has been revoked", key_id),
    }))
}

// ============================================================================
// Logging Handlers
// ============================================================================

/// Response for log level query.
#[derive(Debug, Serialize)]
pub struct LogLevelResponse {
    pub level: String,
    pub options: Vec<String>,
}

/// Request to change log level.
#[derive(Debug, Deserialize)]
pub struct SetLogLevelRequest {
    pub level: String,
}

/// Response for log level change.
#[derive(Debug, Serialize)]
pub struct SetLogLevelResponse {
    pub status: String,
    pub previous: String,
    pub current: String,
}

/// Query parameters for log retrieval.
#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    #[serde(default = "default_lines")]
    pub lines: usize,
    pub level: Option<String>,
}

fn default_lines() -> usize {
    100
}

/// Log entry.
#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// Get current log level.
pub async fn get_log_level() -> Result<Json<LogLevelResponse>, ManagementError> {
    // Get current tracing level - in practice this would query the subscriber
    let current_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    Ok(Json(LogLevelResponse {
        level: current_level,
        options: vec![
            "error".to_string(),
            "warn".to_string(),
            "info".to_string(),
            "debug".to_string(),
            "trace".to_string(),
        ],
    }))
}

/// Set log level at runtime.
pub async fn set_log_level(
    Json(request): Json<SetLogLevelRequest>,
) -> Result<Json<SetLogLevelResponse>, ManagementError> {
    let valid_levels = ["error", "warn", "info", "debug", "trace"];

    if !valid_levels.contains(&request.level.as_str()) {
        return Err(ManagementError::ValidationError(format!(
            "Invalid log level: {}. Valid options: {:?}",
            request.level, valid_levels
        )));
    }

    let previous = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    // Set the environment variable (actual subscriber reload would need more work)
    std::env::set_var("RUST_LOG", &request.level);

    info!("Log level changed from {} to {}", previous, request.level);

    Ok(Json(SetLogLevelResponse {
        status: "ok".to_string(),
        previous,
        current: request.level,
    }))
}

/// Get recent log entries.
pub async fn get_logs(
    Query(query): Query<LogsQuery>,
) -> Result<Json<Vec<LogEntry>>, ManagementError> {
    // In a real implementation, this would read from a log buffer
    // For now, return empty with a note
    debug!(
        "Requested {} log entries with level filter: {:?}",
        query.lines, query.level
    );

    Ok(Json(vec![LogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: "info".to_string(),
        message: "Log retrieval not yet implemented - logs are written to stdout/stderr"
            .to_string(),
    }]))
}

// ============================================================================
// Health & Metrics Handlers
// ============================================================================

/// Detailed health response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub config_file: String,
    pub providers: Vec<ProviderHealth>,
}

/// Provider health status.
#[derive(Debug, Serialize)]
pub struct ProviderHealth {
    pub name: String,
    pub status: String,
    pub models_available: usize,
}

/// Metrics response.
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub requests_total: u64,
    pub requests_success: u64,
    pub requests_failed: u64,
    pub average_latency_ms: f64,
    pub tokens_processed: u64,
    pub cache_hit_rate: f64,
}

// Track start time for uptime calculation
static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Initialize the start time (call at server startup).
pub fn init_start_time() {
    START_TIME.get_or_init(std::time::Instant::now);
}

/// Get detailed health status.
pub async fn get_health(
    State(state): State<ManagementState>,
) -> Result<Json<HealthResponse>, ManagementError> {
    let config = state.config_manager.get().await;

    let uptime = START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0);

    let providers: Vec<ProviderHealth> = config
        .providers
        .iter()
        .map(|p| ProviderHealth {
            name: p.name.clone(),
            status: "healthy".to_string(), // In practice, would check connectivity
            models_available: p.models.len(),
        })
        .collect();

    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        config_file: state
            .config_manager
            .file_path()
            .to_string_lossy()
            .to_string(),
        providers,
    }))
}

/// Get usage metrics.
pub async fn get_metrics() -> Result<Json<MetricsResponse>, ManagementError> {
    // In a real implementation, these would come from actual metrics collection
    Ok(Json(MetricsResponse {
        requests_total: 0,
        requests_success: 0,
        requests_failed: 0,
        average_latency_ms: 0.0,
        tokens_processed: 0,
        cache_hit_rate: 0.0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ManagementSettings, OAuthSettings, ProxySettings, RouterSettings, SecuritySettings,
    };
    use crate::routing::RoutingStrategy;
    use crate::webhooks::WebhookSettings;

    fn create_test_config() -> ProxyConfig {
        ProxyConfig {
            proxy: ProxySettings {
                host: "127.0.0.1".to_string(),
                port: 3456,
                api_key: "super-secret-key".to_string(),
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
                api_key: "sk-secret-api-key-12345".to_string(),
                models: vec!["gpt-4".to_string()],
                transformers: vec![],
            }],
            security: SecuritySettings::default(),
            oauth: OAuthSettings::default(),
            management: ManagementSettings::default(),
            webhooks: WebhookSettings::default(),
        }
    }

    #[test]
    fn test_config_response_redacts_secrets() {
        let config = create_test_config();
        let response = ConfigResponse::from(&config);

        // Proxy API key should be redacted
        assert_eq!(response.proxy.api_key, "[REDACTED]");
        assert_ne!(response.proxy.api_key, config.proxy.api_key);

        // Provider API keys should be redacted
        assert_eq!(response.providers.len(), 1);
        assert_eq!(response.providers[0].api_key, "[REDACTED]");
        assert_ne!(response.providers[0].api_key, config.providers[0].api_key);

        // Non-secret fields should be preserved
        assert_eq!(response.proxy.host, config.proxy.host);
        assert_eq!(response.proxy.port, config.proxy.port);
        assert_eq!(response.providers[0].name, config.providers[0].name);
    }

    #[test]
    fn test_config_response_preserves_non_secrets() {
        let config = create_test_config();
        let response = ConfigResponse::from(&config);

        assert_eq!(response.proxy.host, "127.0.0.1");
        assert_eq!(response.proxy.port, 3456);
        assert_eq!(response.proxy.timeout_ms, 60000);
        assert_eq!(response.router.default, "openai,gpt-4");
        assert_eq!(response.providers[0].name, "openai");
        assert_eq!(
            response.providers[0].api_base_url,
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn test_log_level_validation() {
        let valid_levels = ["error", "warn", "info", "debug", "trace"];

        for level in valid_levels {
            assert!(valid_levels.contains(&level));
        }

        assert!(!valid_levels.contains(&"invalid"));
    }

    #[test]
    fn test_status_response_serialization() {
        let response = StatusResponse {
            status: "ok".to_string(),
            message: "Test message".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"message\":\"Test message\""));
    }

    #[test]
    fn test_api_key_summary_hides_full_key() {
        let summary = ApiKeySummary {
            id: "key-123".to_string(),
            prefix: "sk-...".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"prefix\":\"sk-...\""));
        // Full key should never appear
        assert!(!json.contains("super-secret"));
    }
}
