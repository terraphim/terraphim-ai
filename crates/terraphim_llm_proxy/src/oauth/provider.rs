//! OAuth provider trait for implementing provider-specific authentication flows.

use async_trait::async_trait;

use crate::oauth::error::OAuthResult;
use crate::oauth::types::{AuthFlowState, TokenBundle, TokenValidation};

/// Core trait for OAuth provider implementations.
///
/// Implement this trait to add support for a new OAuth provider.
/// Each provider handles its own authentication flow (PKCE, device code, etc.).
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Get the provider identifier (e.g., "claude", "gemini", "copilot").
    ///
    /// This is used as the key for token storage and routing.
    fn provider_id(&self) -> &str;

    /// Get the human-readable provider name.
    fn display_name(&self) -> &str;

    /// Check if this provider uses browser-based OAuth (callback flow).
    ///
    /// Returns `true` for PKCE/authorization code flows,
    /// `false` for device code flows.
    fn uses_browser_callback(&self) -> bool {
        true
    }

    /// Initiate the OAuth flow.
    ///
    /// For browser-based flows, returns the authorization URL to open.
    /// For device code flows, returns the verification URL and user code.
    ///
    /// # Arguments
    /// * `callback_port` - Local port for OAuth callback (ignored for device flows)
    ///
    /// # Returns
    /// Tuple of (url_to_show_user, flow_state)
    async fn start_auth(&self, callback_port: u16) -> OAuthResult<(String, AuthFlowState)>;

    /// Exchange authorization code for tokens.
    ///
    /// Called after the user completes authorization and we receive the callback.
    ///
    /// # Arguments
    /// * `code` - Authorization code from the callback
    /// * `state` - Flow state containing PKCE verifier and CSRF token
    ///
    /// # Returns
    /// Token bundle with access token, refresh token, and metadata
    async fn exchange_code(&self, code: &str, state: &AuthFlowState) -> OAuthResult<TokenBundle>;

    /// Refresh an expired access token.
    ///
    /// # Arguments
    /// * `refresh_token` - The refresh token from a previous authentication
    ///
    /// # Returns
    /// New token bundle with refreshed access token
    async fn refresh_token(&self, refresh_token: &str) -> OAuthResult<TokenBundle>;

    /// Validate that a token is still valid.
    ///
    /// # Arguments
    /// * `access_token` - The access token to validate
    ///
    /// # Returns
    /// Validation result with expiry info and scopes
    async fn validate_token(&self, access_token: &str) -> OAuthResult<TokenValidation>;

    /// Get the default scopes for this provider.
    fn default_scopes(&self) -> Vec<String> {
        Vec::new()
    }

    /// Get the token endpoint URL for this provider.
    fn token_endpoint(&self) -> &str;

    /// Get the authorization endpoint URL for this provider.
    fn authorization_endpoint(&self) -> &str;

    /// Check if this provider supports token refresh.
    fn supports_refresh(&self) -> bool {
        true
    }
}

/// Device code information for providers that use device code flow.
#[derive(Debug, Clone)]
pub struct DeviceCodeInfo {
    /// The device code to use for polling
    pub device_code: String,

    /// The user code to display to the user
    pub user_code: String,

    /// The URL where the user should enter the code
    pub verification_uri: String,

    /// Optional pre-filled URL with user code
    pub verification_uri_complete: Option<String>,

    /// Seconds until the device code expires
    pub expires_in: u64,

    /// Minimum polling interval in seconds
    pub interval: u64,
}

/// Trait for providers that use device code flow (e.g., GitHub Copilot).
#[async_trait]
pub trait DeviceCodeProvider: OAuthProvider {
    /// Request a device code from the provider.
    async fn request_device_code(&self) -> OAuthResult<DeviceCodeInfo>;

    /// Poll for authorization completion.
    ///
    /// # Returns
    /// - `Ok(Some(bundle))` if authorization is complete
    /// - `Ok(None)` if still pending
    /// - `Err(_)` if authorization failed or expired
    async fn poll_device_code(&self, device_code: &str) -> OAuthResult<Option<TokenBundle>>;
}

/// Provider registry for managing multiple OAuth providers.
pub struct ProviderRegistry {
    providers: std::collections::HashMap<String, Box<dyn OAuthProvider>>,
}

impl ProviderRegistry {
    /// Create a new empty provider registry.
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
        }
    }

    /// Register a provider.
    pub fn register<P: OAuthProvider + 'static>(&mut self, provider: P) {
        self.providers
            .insert(provider.provider_id().to_string(), Box::new(provider));
    }

    /// Get a provider by ID.
    pub fn get(&self, provider_id: &str) -> Option<&dyn OAuthProvider> {
        self.providers.get(provider_id).map(|p| p.as_ref())
    }

    /// List all registered provider IDs.
    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a provider is registered.
    pub fn has_provider(&self, provider_id: &str) -> bool {
        self.providers.contains_key(provider_id)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Mock provider for testing
    struct MockProvider {
        id: String,
        name: String,
    }

    impl MockProvider {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl OAuthProvider for MockProvider {
        fn provider_id(&self) -> &str {
            &self.id
        }

        fn display_name(&self) -> &str {
            &self.name
        }

        async fn start_auth(&self, callback_port: u16) -> OAuthResult<(String, AuthFlowState)> {
            let state =
                AuthFlowState::new("test_state".to_string(), self.id.clone(), callback_port);
            Ok((
                format!("https://auth.example.com?port={}", callback_port),
                state,
            ))
        }

        async fn exchange_code(
            &self,
            _code: &str,
            state: &AuthFlowState,
        ) -> OAuthResult<TokenBundle> {
            Ok(TokenBundle {
                access_token: "mock_access_token".to_string(),
                refresh_token: Some("mock_refresh_token".to_string()),
                token_type: "Bearer".to_string(),
                expires_at: None,
                scope: Some("read".to_string()),
                provider: state.provider.clone(),
                account_id: "test@example.com".to_string(),
                metadata: Default::default(),
                created_at: Utc::now(),
                last_refresh: None,
            })
        }

        async fn refresh_token(&self, _refresh_token: &str) -> OAuthResult<TokenBundle> {
            Ok(TokenBundle::new(
                "refreshed_token".to_string(),
                "Bearer".to_string(),
                self.id.clone(),
                "test@example.com".to_string(),
            ))
        }

        async fn validate_token(&self, _access_token: &str) -> OAuthResult<TokenValidation> {
            Ok(TokenValidation::valid(Some(3600), vec!["read".to_string()]))
        }

        fn token_endpoint(&self) -> &str {
            "https://auth.example.com/token"
        }

        fn authorization_endpoint(&self) -> &str {
            "https://auth.example.com/authorize"
        }
    }

    #[test]
    fn test_provider_registry_new() {
        let registry = ProviderRegistry::new();
        assert!(registry.list_providers().is_empty());
    }

    #[test]
    fn test_provider_registry_register() {
        let mut registry = ProviderRegistry::new();
        registry.register(MockProvider::new("test", "Test Provider"));

        assert!(registry.has_provider("test"));
        assert!(!registry.has_provider("other"));
    }

    #[test]
    fn test_provider_registry_get() {
        let mut registry = ProviderRegistry::new();
        registry.register(MockProvider::new("test", "Test Provider"));

        let provider = registry.get("test").unwrap();
        assert_eq!(provider.provider_id(), "test");
        assert_eq!(provider.display_name(), "Test Provider");

        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_provider_registry_list() {
        let mut registry = ProviderRegistry::new();
        registry.register(MockProvider::new("claude", "Claude"));
        registry.register(MockProvider::new("gemini", "Gemini"));

        let providers = registry.list_providers();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&"claude"));
        assert!(providers.contains(&"gemini"));
    }

    #[tokio::test]
    async fn test_mock_provider_start_auth() {
        let provider = MockProvider::new("test", "Test");
        let (url, state) = provider.start_auth(54545).await.unwrap();

        assert!(url.contains("54545"));
        assert_eq!(state.provider, "test");
        assert_eq!(state.callback_port, 54545);
    }

    #[tokio::test]
    async fn test_mock_provider_exchange_code() {
        let provider = MockProvider::new("test", "Test");
        let (_, state) = provider.start_auth(54545).await.unwrap();
        let bundle = provider.exchange_code("auth_code", &state).await.unwrap();

        assert_eq!(bundle.access_token, "mock_access_token");
        assert_eq!(bundle.provider, "test");
    }

    #[tokio::test]
    async fn test_mock_provider_validate_token() {
        let provider = MockProvider::new("test", "Test");
        let validation = provider.validate_token("token").await.unwrap();

        assert!(validation.valid);
        assert_eq!(validation.expires_in_seconds, Some(3600));
    }

    #[test]
    fn test_device_code_info() {
        let info = DeviceCodeInfo {
            device_code: "device123".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://github.com/login/device".to_string(),
            verification_uri_complete: Some(
                "https://github.com/login/device?user_code=ABCD-1234".to_string(),
            ),
            expires_in: 900,
            interval: 5,
        };

        assert_eq!(info.device_code, "device123");
        assert_eq!(info.user_code, "ABCD-1234");
        assert_eq!(info.interval, 5);
    }
}
