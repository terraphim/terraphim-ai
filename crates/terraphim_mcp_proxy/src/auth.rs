use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::{McpProxyError, Result, Tool, ToolCallRequest};

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub api_key: String,
    pub rate_limit: u32,
    pub rate_limit_window: Duration,
    pub max_requests_per_user: u32,
    pub user_request_window: Duration,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            rate_limit: 100,
            rate_limit_window: Duration::from_secs(60),
            max_requests_per_user: 1000,
            user_request_window: Duration::from_secs(3600),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserAuth {
    pub user_id: String,
    pub api_key: String,
    pub permissions: Vec<String>,
    pub rate_limit: u32,
    pub rate_limit_window: Duration,
    pub created_at: SystemTime,
    pub expires_at: Option<SystemTime>,
}

impl UserAuth {
    pub fn new(user_id: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            api_key: api_key.into(),
            permissions: Vec::new(),
            rate_limit: 100,
            rate_limit_window: Duration::from_secs(60),
            created_at: SystemTime::now(),
            expires_at: None,
        }
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_rate_limit(mut self, limit: u32, window: Duration) -> Self {
        self.rate_limit = limit;
        self.rate_limit_window = window;
        self
    }

    pub fn with_expiration(mut self, expires_at: SystemTime) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| SystemTime::now() > exp)
            .unwrap_or(false)
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitEntry {
    pub count: u32,
    pub window_start: SystemTime,
}

impl RateLimitEntry {
    pub fn new() -> Self {
        Self {
            count: 0,
            window_start: SystemTime::now(),
        }
    }

    pub fn is_expired(&self, window: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.window_start)
            .unwrap_or(Duration::MAX) > window
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }

    pub fn reset(&mut self) {
        self.count = 0;
        self.window_start = SystemTime::now();
    }
}

pub struct AuthMiddleware {
    config: AuthConfig,
    user_auths: Arc<tokio::sync::RwLock<std::collections::HashMap<String, UserAuth>>>,
    rate_limits: Arc<tokio::sync::RwLock<std::collections::HashMap<String, RateLimitEntry>>>,
}

impl AuthMiddleware {
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config,
            user_auths: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            rate_limits: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn add_user_auth(&self, user_auth: UserAuth) {
        let mut auths = self.user_auths.write().await;
        auths.insert(user_auth.user_id.clone(), user_auth);
    }

    pub async fn remove_user_auth(&self, user_id: &str) {
        let mut auths = self.user_auths.write().await;
        auths.remove(user_id);
        
        // Also remove rate limit entry
        let mut rate_limits = self.rate_limits.write().await;
        rate_limits.remove(user_id);
    }

    pub async fn validate_api_key(&self, api_key: &str) -> Option<UserAuth> {
        let auths = self.user_auths.read().await;
        
        // Find user by API key
        for user_auth in auths.values() {
            if user_auth.api_key == api_key && !user_auth.is_expired() {
                return Some(user_auth.clone());
            }
        }
        
        None
    }

    pub async fn check_rate_limit(&self, user_id: &str) -> Result<bool> {
        let user_auth = {
            let auths = self.user_auths.read().await;
            auths.get(user_id).cloned()
        };

        if let Some(user_auth) = user_auth {
            let mut rate_limits = self.rate_limits.write().await;
            let entry = rate_limits.entry(user_id.to_string()).or_insert_with(RateLimitEntry::new);

            // Check if window has expired
            if entry.is_expired(user_auth.rate_limit_window) {
                entry.reset();
            }

            // Check if limit exceeded
            if entry.count >= user_auth.rate_limit {
                return Ok(false);
            }

            entry.increment();
            Ok(true)
        } else {
            // Use global rate limit for unknown users
            let mut rate_limits = self.rate_limits.write().await;
            let entry = rate_limits.entry(user_id.to_string()).or_insert_with(RateLimitEntry::new);

            if entry.is_expired(self.config.rate_limit_window) {
                entry.reset();
            }

            if entry.count >= self.config.rate_limit {
                return Ok(false);
            }

            entry.increment();
            Ok(true)
        }
    }

    pub async fn extract_user_id_from_request(&self, request: &ToolCallRequest) -> Option<String> {
        // Try to extract user ID from request arguments
        if let Some(args) = &request.arguments {
            if let Some(obj) = args.as_object() {
                if let Some(user_id) = obj.get("user_id").and_then(|v| v.as_str()) {
                    return Some(user_id.to_string());
                }
                if let Some(user_id) = obj.get("userId").and_then(|v| v.as_str()) {
                    return Some(user_id.to_string());
                }
            }
        }
        
        None
    }
}

#[async_trait]
impl crate::McpMiddleware for AuthMiddleware {
    async fn before_tool_call(&self, request: &ToolCallRequest) -> crate::Result<Option<ToolCallRequest>> {
        // Extract user ID from request
        let user_id = self.extract_user_id_from_request(request).await
            .unwrap_or_else(|| "anonymous".to_string());

        // Check rate limit
        if !self.check_rate_limit(&user_id).await? {
            return Err(McpProxyError::Configuration(format!(
                "Rate limit exceeded for user: {}",
                user_id
            )));
        }

        // Check permissions if user is authenticated
        let user_auth = {
            let auths = self.user_auths.read().await;
            auths.get(&user_id).cloned()
        };

        if let Some(user_auth) = user_auth {
            // Check if user has permission to call this tool
            let tool_permission = format!("tool:{}", request.name);
            if !user_auth.has_permission(&tool_permission) && !user_auth.has_permission("tool:*") {
                return Err(McpProxyError::Configuration(format!(
                    "User {} does not have permission to call tool: {}",
                    user_id, request.name
                )));
            }
        }

        Ok(Some(request.clone()))
    }

    async fn filter_tools(&self, tools: Vec<Tool>) -> crate::Result<Vec<Tool>> {
        // Filter tools based on user permissions
        // For now, return all tools - can be extended to filter based on user permissions
        Ok(tools)
    }
}

pub struct ApiKeyAuthMiddleware {
    auth_middleware: Arc<AuthMiddleware>,
}

impl ApiKeyAuthMiddleware {
    pub fn new(auth_middleware: Arc<AuthMiddleware>) -> Self {
        Self { auth_middleware }
    }

    pub async fn authenticate_request(&self, api_key: &str) -> Result<UserAuth> {
        if let Some(user_auth) = self.auth_middleware.validate_api_key(api_key).await {
            Ok(user_auth)
        } else {
            Err(McpProxyError::Configuration("Invalid API key".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_user_auth_creation() {
        let user_auth = UserAuth::new("test_user", "test_api_key")
            .with_permissions(vec!["tool:filesystem__read_file".to_string()])
            .with_rate_limit(10, Duration::from_secs(60));

        assert_eq!(user_auth.user_id, "test_user");
        assert_eq!(user_auth.api_key, "test_api_key");
        assert!(user_auth.has_permission("tool:filesystem__read_file"));
        assert!(!user_auth.has_permission("tool:other_tool"));
    }

    #[tokio::test]
    async fn test_auth_middleware_rate_limiting() {
        let auth_middleware = AuthMiddleware::new(AuthConfig::default());
        
        let user_auth = UserAuth::new("test_user", "test_key")
            .with_rate_limit(2, Duration::from_secs(60));
        
        auth_middleware.add_user_auth(user_auth).await;

        // First two requests should pass
        assert!(auth_middleware.check_rate_limit("test_user").await.unwrap());
        assert!(auth_middleware.check_rate_limit("test_user").await.unwrap());
        
        // Third request should be rate limited
        assert!(!auth_middleware.check_rate_limit("test_user").await.unwrap());
    }

    #[tokio::test]
    async fn test_auth_middleware_permissions() {
        let auth_middleware = AuthMiddleware::new(AuthConfig::default());
        
        let user_auth = UserAuth::new("test_user", "test_key")
            .with_permissions(vec!["tool:allowed_tool".to_string()]);
        
        auth_middleware.add_user_auth(user_auth).await;

        let allowed_request = ToolCallRequest {
            name: "allowed_tool".to_string(),
            arguments: Some(json!({"user_id": "test_user"})),
        };

        let blocked_request = ToolCallRequest {
            name: "blocked_tool".to_string(),
            arguments: Some(json!({"user_id": "test_user"})),
        };

        // Allowed tool should pass
        let result = auth_middleware.before_tool_call(&allowed_request).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        // Blocked tool should fail
        let result = auth_middleware.before_tool_call(&blocked_request).await;
        assert!(result.is_err());
    }
}