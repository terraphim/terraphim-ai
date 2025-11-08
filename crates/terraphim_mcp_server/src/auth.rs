use anyhow::Result;
use rmcp::service::ServiceRole;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("API key expired")]
    ExpiredApiKey,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Invalid token format")]
    InvalidTokenFormat,
}

impl From<AuthError> for rmcp::model::ErrorData {
    fn from(err: AuthError) -> Self {
        rmcp::model::ErrorData::internal_error(err.to_string(), None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub created_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub rate_limit: u32,
    pub rate_limit_window: Duration,
}

impl ApiKey {
    pub fn new(key: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            permissions: vec!["*".to_string()], // All permissions by default
            created_at: SystemTime::now(),
            expires_at: None,
            rate_limit: 1000,
            rate_limit_window: Duration::from_secs(3600), // 1 hour
        }
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_expiration(mut self, expires_at: SystemTime) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn with_rate_limit(mut self, limit: u32, window: Duration) -> Self {
        self.rate_limit = limit;
        self.rate_limit_window = window;
        self
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| SystemTime::now() > exp)
            .unwrap_or(false)
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&"*".to_string())
            || self.permissions.contains(&permission.to_string())
    }
}

#[derive(Debug, Clone)]
struct RateLimitEntry {
    count: u32,
    window_start: SystemTime,
}

impl RateLimitEntry {
    fn new() -> Self {
        Self {
            count: 0,
            window_start: SystemTime::now(),
        }
    }

    fn is_expired(&self, window: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.window_start)
            .unwrap_or(Duration::MAX)
            > window
    }

    fn increment(&mut self) {
        self.count += 1;
    }

    fn reset(&mut self) {
        self.count = 0;
        self.window_start = SystemTime::now();
    }
}

#[derive(Debug)]
pub struct AuthManager {
    api_keys: Arc<RwLock<std::collections::HashMap<String, ApiKey>>>,
    rate_limits: Arc<RwLock<std::collections::HashMap<String, RateLimitEntry>>>,
    global_rate_limit: u32,
    global_rate_limit_window: Duration,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            api_keys: Arc::new(RwLock::new(std::collections::HashMap::new())),
            rate_limits: Arc::new(RwLock::new(std::collections::HashMap::new())),
            global_rate_limit: 10000,
            global_rate_limit_window: Duration::from_secs(3600), // 1 hour
        }
    }

    pub async fn add_api_key(&self, api_key: ApiKey) {
        let mut keys = self.api_keys.write().await;
        info!("Added API key: {} ({})", api_key.name, api_key.key);
        keys.insert(api_key.key.clone(), api_key);
    }

    pub async fn remove_api_key(&self, key: &str) -> bool {
        let mut keys = self.api_keys.write().await;
        let mut rate_limits = self.rate_limits.write().await;

        rate_limits.remove(key);
        keys.remove(key).is_some()
    }

    pub async fn validate_api_key(&self, key: &str) -> Result<ApiKey, AuthError> {
        let keys = self.api_keys.read().await;

        if let Some(api_key) = keys.get(key) {
            if api_key.is_expired() {
                return Err(AuthError::ExpiredApiKey);
            }
            Ok(api_key.clone())
        } else {
            Err(AuthError::InvalidApiKey)
        }
    }

    pub async fn check_rate_limit(&self, key: &str) -> Result<bool, AuthError> {
        let api_key = self.validate_api_key(key).await?;

        let mut rate_limits = self.rate_limits.write().await;
        let entry = rate_limits
            .entry(key.to_string())
            .or_insert_with(RateLimitEntry::new);

        // Check if window has expired
        if entry.is_expired(api_key.rate_limit_window) {
            entry.reset();
        }

        // Check if limit exceeded
        if entry.count >= api_key.rate_limit {
            return Ok(false);
        }

        entry.increment();
        Ok(true)
    }

    pub async fn check_global_rate_limit(&self, client_id: &str) -> Result<bool, AuthError> {
        let mut rate_limits = self.rate_limits.write().await;
        let entry = rate_limits
            .entry(format!("global:{}", client_id))
            .or_insert_with(RateLimitEntry::new);

        // Check if window has expired
        if entry.is_expired(self.global_rate_limit_window) {
            entry.reset();
        }

        // Check if limit exceeded
        if entry.count >= self.global_rate_limit {
            return Ok(false);
        }

        entry.increment();
        Ok(true)
    }

    pub async fn check_permission(&self, key: &str, permission: &str) -> Result<bool, AuthError> {
        let api_key = self.validate_api_key(key).await?;
        Ok(api_key.has_permission(permission))
    }

    pub async fn extract_api_key_from_context<R: ServiceRole>(
        &self,
        context: &rmcp::service::RequestContext<R>,
    ) -> Option<String> {
        // Try to extract API key from request context
        // This could be from headers, query params, or other metadata

        // For now, check if there's an "Authorization" header in meta
        if let Some(auth_header) = context.meta.get("authorization") {
            if let Some(auth_str) = auth_header.as_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    return Some(token.to_string());
                }
            }
        }

        // Check for "X-API-Key" header in meta
        if let Some(api_key_header) = context.meta.get("x-api-key") {
            if let Some(api_key_str) = api_key_header.as_str() {
                return Some(api_key_str.to_string());
            }
        }

        None
    }

    pub async fn authenticate_request<R: ServiceRole>(
        &self,
        context: &rmcp::service::RequestContext<R>,
    ) -> Result<ApiKey, AuthError> {
        let api_key = self
            .extract_api_key_from_context(context)
            .await
            .ok_or(AuthError::AuthenticationRequired)?;

        // Check rate limit
        if !self.check_rate_limit(&api_key).await? {
            return Err(AuthError::RateLimitExceeded);
        }

        // Validate API key
        self.validate_api_key(&api_key).await
    }

    pub async fn list_api_keys(&self) -> Vec<ApiKey> {
        let keys = self.api_keys.read().await;
        keys.values().cloned().collect()
    }

    pub async fn generate_api_key(&self, name: impl Into<String>) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Generate a simple key based on timestamp and random bytes
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let random_bytes: Vec<u8> = (0..16)
            .map(|i| ((timestamp.wrapping_add(i as u128)) % 256) as u8)
            .collect();

        let key = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            random_bytes,
        );

        let api_key = ApiKey::new(&key, name);
        self.add_api_key(api_key).await;
        key
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub require_api_key: bool,
    pub default_rate_limit: u32,
    pub default_rate_limit_window_secs: u64,
    pub global_rate_limit: u32,
    pub global_rate_limit_window_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            require_api_key: false,
            default_rate_limit: 1000,
            default_rate_limit_window_secs: 3600,
            global_rate_limit: 10000,
            global_rate_limit_window_secs: 3600,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_key_creation() {
        let api_key = ApiKey::new("test_key", "Test Key")
            .with_permissions(vec!["tool:list".to_string(), "tool:call".to_string()])
            .with_rate_limit(100, Duration::from_secs(60));

        assert_eq!(api_key.key, "test_key");
        assert_eq!(api_key.name, "Test Key");
        assert!(api_key.has_permission("tool:list"));
        assert!(api_key.has_permission("tool:call"));
        assert!(!api_key.has_permission("tool:admin"));
        assert_eq!(api_key.rate_limit, 100);
    }

    #[tokio::test]
    async fn test_auth_manager() {
        let auth_manager = AuthManager::new();

        let api_key =
            ApiKey::new("test_key", "Test Key").with_permissions(vec!["tool:list".to_string()]);

        auth_manager.add_api_key(api_key).await;

        // Valid key should pass
        let result = auth_manager.validate_api_key("test_key").await;
        assert!(result.is_ok());

        // Invalid key should fail
        let result = auth_manager.validate_api_key("invalid_key").await;
        assert!(matches!(result, Err(AuthError::InvalidApiKey)));

        // Permission check
        let has_perm = auth_manager.check_permission("test_key", "tool:list").await;
        assert!(has_perm.is_ok());
        assert!(has_perm.unwrap());

        let has_perm = auth_manager
            .check_permission("test_key", "tool:admin")
            .await;
        assert!(has_perm.is_ok());
        assert!(!has_perm.unwrap());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let auth_manager = AuthManager::new();

        let api_key =
            ApiKey::new("test_key", "Test Key").with_rate_limit(2, Duration::from_secs(60));

        auth_manager.add_api_key(api_key).await;

        // First two requests should pass
        assert!(auth_manager.check_rate_limit("test_key").await.unwrap());
        assert!(auth_manager.check_rate_limit("test_key").await.unwrap());

        // Third request should be rate limited
        assert!(!auth_manager.check_rate_limit("test_key").await.unwrap());
    }
}
