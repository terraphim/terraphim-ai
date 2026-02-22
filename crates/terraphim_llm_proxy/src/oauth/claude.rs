//! Claude (Anthropic) OAuth2 PKCE provider implementation.
//!
//! Implements OAuth2 Authorization Code flow with PKCE for Claude/Anthropic API.
//!
//! # Flow
//!
//! 1. Generate PKCE code_verifier and code_challenge
//! 2. Redirect user to Anthropic authorization URL
//! 3. User authorizes and is redirected back with authorization code
//! 4. Exchange code + code_verifier for access/refresh tokens
//! 5. Use access token for API calls, refresh when expired

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::oauth::error::{OAuthError, OAuthResult};
use crate::oauth::provider::OAuthProvider;
use crate::oauth::types::{AccountInfo, AuthFlowState, TokenBundle, TokenValidation};

/// Claude OAuth2 endpoints
const AUTHORIZATION_ENDPOINT: &str = "https://console.anthropic.com/oauth/authorize";
const TOKEN_ENDPOINT: &str = "https://api.anthropic.com/oauth/token";
const USERINFO_ENDPOINT: &str = "https://api.anthropic.com/oauth/userinfo";

/// Default scopes for Claude API access (user:inference for API calls, user:profile for user info)
const DEFAULT_SCOPES: &[&str] = &["user:inference", "user:profile"];

/// Claude OAuth provider using PKCE flow.
#[derive(Debug, Clone)]
pub struct ClaudeOAuthProvider {
    /// OAuth client ID from Anthropic Console
    client_id: String,
    /// HTTP client for OAuth requests
    http_client: reqwest::Client,
    /// Optional client secret (for confidential clients)
    client_secret: Option<String>,
    /// Custom scopes (overrides DEFAULT_SCOPES if set)
    custom_scopes: Option<Vec<String>>,
    /// Whether to create an API key after token exchange ("api_key" mode)
    api_key_mode: bool,
}

impl ClaudeOAuthProvider {
    /// Create a new Claude OAuth provider.
    ///
    /// # Arguments
    /// * `client_id` - OAuth client ID from Anthropic Console
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            http_client: reqwest::Client::new(),
            client_secret: None,
            custom_scopes: None,
            api_key_mode: false,
        }
    }

    /// Create with client secret for confidential clients.
    pub fn with_secret(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            http_client: reqwest::Client::new(),
            client_secret: Some(client_secret),
            custom_scopes: None,
            api_key_mode: false,
        }
    }

    /// Set custom scopes (overrides defaults).
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.custom_scopes = Some(scopes);
        self
    }

    /// Enable API key creation mode.
    /// After token exchange, creates a permanent Anthropic API key.
    pub fn with_api_key_mode(mut self) -> Self {
        self.api_key_mode = true;
        self
    }

    /// Create with custom HTTP client.
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = client;
        self
    }

    /// Build the callback URL for the given port.
    fn callback_url(port: u16) -> String {
        format!("http://127.0.0.1:{}/oauth/claude/callback", port)
    }
}

#[async_trait]
impl OAuthProvider for ClaudeOAuthProvider {
    fn provider_id(&self) -> &str {
        "claude"
    }

    fn display_name(&self) -> &str {
        "Claude (Anthropic)"
    }

    fn uses_browser_callback(&self) -> bool {
        true
    }

    fn token_endpoint(&self) -> &str {
        TOKEN_ENDPOINT
    }

    fn authorization_endpoint(&self) -> &str {
        AUTHORIZATION_ENDPOINT
    }

    fn default_scopes(&self) -> Vec<String> {
        if let Some(ref scopes) = self.custom_scopes {
            scopes.clone()
        } else {
            DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect()
        }
    }

    async fn start_auth(&self, callback_port: u16) -> OAuthResult<(String, AuthFlowState)> {
        // Generate PKCE code verifier and challenge
        let code_verifier = generate_code_verifier();
        let code_challenge = generate_code_challenge(&code_verifier);

        // Generate state token for CSRF protection
        let state = generate_state_token();

        // Build authorization URL
        let redirect_uri = Self::callback_url(callback_port);
        let scopes = self.default_scopes().join(" ");

        let auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
            AUTHORIZATION_ENDPOINT,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(&state),
            urlencoding::encode(&code_challenge),
        );

        info!(
            "Starting Claude OAuth flow, redirect to: {}",
            AUTHORIZATION_ENDPOINT
        );
        debug!("Full auth URL: {}", auth_url);

        // Create flow state
        let flow_state =
            AuthFlowState::new(state, "claude".to_string(), callback_port).with_pkce(code_verifier);

        Ok((auth_url, flow_state))
    }

    async fn exchange_code(&self, code: &str, state: &AuthFlowState) -> OAuthResult<TokenBundle> {
        let code_verifier = state.code_verifier.as_ref().ok_or_else(|| {
            OAuthError::InvalidState("Missing PKCE code verifier in flow state".to_string())
        })?;

        let redirect_uri = Self::callback_url(state.callback_port);

        // Build token request
        let mut params = vec![
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", redirect_uri),
            ("client_id", self.client_id.clone()),
            ("code_verifier", code_verifier.clone()),
        ];

        // Add client secret if available
        if let Some(ref secret) = self.client_secret {
            params.push(("client_secret", secret.clone()));
        }

        info!("Exchanging authorization code for tokens");

        let response = self
            .http_client
            .post(TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await
            .map_err(OAuthError::HttpError)?;

        let status = response.status();
        let body = response.text().await.map_err(OAuthError::HttpError)?;

        if !status.is_success() {
            warn!("Token exchange failed with status {}: {}", status, body);
            return Err(parse_oauth_error(&body, status.as_u16()));
        }

        let token_response: TokenResponse = serde_json::from_str(&body).map_err(|e| {
            OAuthError::FlowFailed(format!("Failed to parse token response: {}", e))
        })?;

        // Fetch user info to get account ID
        let account_info = self
            .fetch_user_info(&token_response.access_token)
            .await
            .ok();

        let account_id = account_info
            .as_ref()
            .and_then(|info| info.email.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let expires_at = token_response
            .expires_in
            .map(|secs| Utc::now() + Duration::seconds(secs as i64));

        info!("Successfully obtained tokens for account: {}", account_id);

        // If api_key mode, create a permanent API key from the access token
        let mut metadata: std::collections::HashMap<String, serde_json::Value> = Default::default();
        if self.api_key_mode {
            match crate::oauth::claude_api_key::create_anthropic_api_key(
                &self.http_client,
                &token_response.access_token,
                &format!("terraphim-proxy-{}", &account_id),
            )
            .await
            {
                Ok(api_key) => {
                    info!("Created Anthropic API key for account: {}", account_id);
                    metadata.insert("api_key".to_string(), serde_json::Value::String(api_key));
                }
                Err(e) => {
                    warn!(
                        "Failed to create API key (falling back to Bearer token): {}",
                        e
                    );
                }
            }
        }

        Ok(TokenBundle {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
            expires_at,
            scope: token_response.scope,
            provider: "claude".to_string(),
            account_id,
            metadata,
            created_at: Utc::now(),
            last_refresh: None,
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> OAuthResult<TokenBundle> {
        let mut params = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token.to_string()),
            ("client_id", self.client_id.clone()),
        ];

        if let Some(ref secret) = self.client_secret {
            params.push(("client_secret", secret.clone()));
        }

        info!("Refreshing Claude access token");

        let response = self
            .http_client
            .post(TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await
            .map_err(OAuthError::HttpError)?;

        let status = response.status();
        let body = response.text().await.map_err(OAuthError::HttpError)?;

        if !status.is_success() {
            warn!("Token refresh failed with status {}: {}", status, body);
            return Err(parse_oauth_error(&body, status.as_u16()));
        }

        let token_response: TokenResponse = serde_json::from_str(&body).map_err(|e| {
            OAuthError::RefreshFailed(format!("Failed to parse refresh response: {}", e))
        })?;

        // Fetch user info to get account ID
        let account_info = self
            .fetch_user_info(&token_response.access_token)
            .await
            .ok();

        let account_id = account_info
            .as_ref()
            .and_then(|info| info.email.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let expires_at = token_response
            .expires_in
            .map(|secs| Utc::now() + Duration::seconds(secs as i64));

        info!("Successfully refreshed token for account: {}", account_id);

        Ok(TokenBundle {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
            expires_at,
            scope: token_response.scope,
            provider: "claude".to_string(),
            account_id,
            metadata: Default::default(),
            created_at: Utc::now(),
            last_refresh: Some(Utc::now()),
        })
    }

    async fn validate_token(&self, access_token: &str) -> OAuthResult<TokenValidation> {
        match self.fetch_user_info(access_token).await {
            Ok(info) => {
                debug!("Token validated successfully for: {:?}", info.email);
                Ok(TokenValidation {
                    valid: true,
                    expires_in_seconds: None, // We don't know without the token response
                    scopes: self.default_scopes(),
                    account_info: Some(info),
                    error: None,
                })
            }
            Err(e) => {
                warn!("Token validation failed: {}", e);
                Ok(TokenValidation::invalid(e.to_string()))
            }
        }
    }
}

impl ClaudeOAuthProvider {
    /// Fetch user info from the userinfo endpoint.
    async fn fetch_user_info(&self, access_token: &str) -> OAuthResult<AccountInfo> {
        let response = self
            .http_client
            .get(USERINFO_ENDPOINT)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(OAuthError::HttpError)?;

        let status = response.status();
        let body = response.text().await.map_err(OAuthError::HttpError)?;

        if !status.is_success() {
            return Err(OAuthError::ValidationFailed(format!(
                "Userinfo request failed with status {}: {}",
                status, body
            )));
        }

        let userinfo: UserInfoResponse = serde_json::from_str(&body).map_err(|e| {
            OAuthError::ValidationFailed(format!("Failed to parse userinfo response: {}", e))
        })?;

        Ok(AccountInfo {
            id: userinfo.sub,
            email: userinfo.email,
            name: userinfo.name,
            avatar_url: userinfo.picture,
        })
    }
}

/// OAuth2 token response from Anthropic.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
}

/// OpenID Connect userinfo response.
#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    /// Subject identifier
    sub: String,
    /// Email address
    email: Option<String>,
    /// Display name
    name: Option<String>,
    /// Profile picture URL
    picture: Option<String>,
}

/// OAuth2 error response.
#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: String,
    error_description: Option<String>,
}

/// Generate a PKCE code verifier (43-128 characters, URL-safe).
///
/// Uses cryptographically secure random bytes encoded as base64url.
pub fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Generate PKCE code challenge from verifier using S256 method.
///
/// code_challenge = BASE64URL(SHA256(code_verifier))
pub fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

/// Generate a random state token for CSRF protection.
pub fn generate_state_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..16).map(|_| rng.gen()).collect();
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Parse an OAuth error response.
fn parse_oauth_error(body: &str, status_code: u16) -> OAuthError {
    if let Ok(error_response) = serde_json::from_str::<OAuthErrorResponse>(body) {
        let description = error_response
            .error_description
            .unwrap_or_else(|| error_response.error.clone());

        match error_response.error.as_str() {
            "invalid_grant" => OAuthError::RefreshFailed(description),
            "invalid_client" => OAuthError::FlowFailed(format!("Invalid client: {}", description)),
            "invalid_request" => {
                OAuthError::FlowFailed(format!("Invalid request: {}", description))
            }
            "unauthorized_client" => {
                OAuthError::FlowFailed(format!("Unauthorized client: {}", description))
            }
            "access_denied" => OAuthError::FlowFailed(format!("Access denied: {}", description)),
            "temporarily_unavailable" => {
                if status_code == 429 {
                    OAuthError::RateLimited {
                        provider: "claude".to_string(),
                        retry_after_seconds: 60, // Default retry
                    }
                } else {
                    OAuthError::FlowFailed(format!("Temporarily unavailable: {}", description))
                }
            }
            _ => OAuthError::FlowFailed(format!("{}: {}", error_response.error, description)),
        }
    } else {
        OAuthError::FlowFailed(format!("HTTP {}: {}", status_code, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_verifier() {
        let verifier = generate_code_verifier();

        // Should be 43 characters (32 bytes base64url encoded)
        assert_eq!(verifier.len(), 43);

        // Should only contain URL-safe characters
        assert!(verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));

        // Each call should generate a different verifier
        let verifier2 = generate_code_verifier();
        assert_ne!(verifier, verifier2);
    }

    #[test]
    fn test_generate_code_challenge() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = generate_code_challenge(verifier);

        // SHA256 hash should produce 43 characters (32 bytes base64url encoded)
        assert_eq!(challenge.len(), 43);

        // Same verifier should produce same challenge
        let challenge2 = generate_code_challenge(verifier);
        assert_eq!(challenge, challenge2);
    }

    #[test]
    fn test_generate_code_challenge_known_value() {
        // Test vector from RFC 7636 Appendix B
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = generate_code_challenge(verifier);

        // Expected challenge for this verifier (S256 method)
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn test_generate_state_token() {
        let state = generate_state_token();

        // Should be 22 characters (16 bytes base64url encoded)
        assert_eq!(state.len(), 22);

        // Should only contain URL-safe characters
        assert!(state
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));

        // Each call should generate a different state
        let state2 = generate_state_token();
        assert_ne!(state, state2);
    }

    #[test]
    fn test_claude_provider_creation() {
        let provider = ClaudeOAuthProvider::new("test_client_id".to_string());

        assert_eq!(provider.provider_id(), "claude");
        assert_eq!(provider.display_name(), "Claude (Anthropic)");
        assert!(provider.uses_browser_callback());
    }

    #[test]
    fn test_claude_provider_with_secret() {
        let provider =
            ClaudeOAuthProvider::with_secret("test_client_id".to_string(), "secret".to_string());

        assert!(provider.client_secret.is_some());
        assert_eq!(provider.client_secret.unwrap(), "secret");
    }

    #[test]
    fn test_claude_provider_endpoints() {
        let provider = ClaudeOAuthProvider::new("test".to_string());

        assert_eq!(
            provider.authorization_endpoint(),
            "https://console.anthropic.com/oauth/authorize"
        );
        assert_eq!(
            provider.token_endpoint(),
            "https://api.anthropic.com/oauth/token"
        );
    }

    #[test]
    fn test_claude_provider_default_scopes() {
        let provider = ClaudeOAuthProvider::new("test".to_string());
        let scopes = provider.default_scopes();

        assert!(scopes.contains(&"user:inference".to_string()));
        assert!(scopes.contains(&"user:profile".to_string()));
    }

    #[test]
    fn test_claude_provider_custom_scopes() {
        let provider = ClaudeOAuthProvider::new("test".to_string()).with_scopes(vec![
            "org:create_api_key".to_string(),
            "user:profile".to_string(),
            "user:inference".to_string(),
        ]);
        let scopes = provider.default_scopes();

        assert_eq!(scopes.len(), 3);
        assert!(scopes.contains(&"org:create_api_key".to_string()));
        assert!(scopes.contains(&"user:profile".to_string()));
        assert!(scopes.contains(&"user:inference".to_string()));
    }

    #[test]
    fn test_callback_url() {
        let url = ClaudeOAuthProvider::callback_url(54545);
        assert_eq!(url, "http://127.0.0.1:54545/oauth/claude/callback");
    }

    #[tokio::test]
    async fn test_start_auth_generates_valid_url() {
        let provider = ClaudeOAuthProvider::new("test_client_id".to_string());
        let (auth_url, state) = provider.start_auth(54545).await.unwrap();

        // Check URL contains required parameters
        assert!(auth_url.starts_with(AUTHORIZATION_ENDPOINT));
        assert!(auth_url.contains("response_type=code"));
        assert!(auth_url.contains("client_id=test_client_id"));
        assert!(auth_url.contains("redirect_uri="));
        assert!(auth_url.contains("code_challenge="));
        assert!(auth_url.contains("code_challenge_method=S256"));
        assert!(auth_url.contains("state="));

        // Check flow state
        assert_eq!(state.provider, "claude");
        assert_eq!(state.callback_port, 54545);
        assert!(state.code_verifier.is_some());

        // Verify code_challenge matches the verifier
        let verifier = state.code_verifier.as_ref().unwrap();
        let expected_challenge = generate_code_challenge(verifier);
        assert!(auth_url.contains(&expected_challenge));
    }

    #[tokio::test]
    async fn test_start_auth_different_ports() {
        let provider = ClaudeOAuthProvider::new("test".to_string());

        let (url1, _) = provider.start_auth(54545).await.unwrap();
        let (url2, _) = provider.start_auth(54546).await.unwrap();

        assert!(url1.contains("54545"));
        assert!(url2.contains("54546"));
    }

    #[test]
    fn test_parse_oauth_error_invalid_grant() {
        let body = r#"{"error": "invalid_grant", "error_description": "Token expired"}"#;
        let error = parse_oauth_error(body, 400);

        match error {
            OAuthError::RefreshFailed(msg) => assert!(msg.contains("expired")),
            _ => panic!("Expected RefreshFailed error"),
        }
    }

    #[test]
    fn test_parse_oauth_error_invalid_client() {
        let body = r#"{"error": "invalid_client", "error_description": "Unknown client"}"#;
        let error = parse_oauth_error(body, 401);

        match error {
            OAuthError::FlowFailed(msg) => assert!(msg.contains("Invalid client")),
            _ => panic!("Expected FlowFailed error"),
        }
    }

    #[test]
    fn test_parse_oauth_error_rate_limited() {
        let body =
            r#"{"error": "temporarily_unavailable", "error_description": "Too many requests"}"#;
        let error = parse_oauth_error(body, 429);

        match error {
            OAuthError::RateLimited {
                provider,
                retry_after_seconds,
            } => {
                assert_eq!(provider, "claude");
                assert_eq!(retry_after_seconds, 60);
            }
            _ => panic!("Expected RateLimited error"),
        }
    }

    #[test]
    fn test_parse_oauth_error_malformed_json() {
        let body = "Not JSON";
        let error = parse_oauth_error(body, 500);

        match error {
            OAuthError::FlowFailed(msg) => {
                assert!(msg.contains("500"));
                assert!(msg.contains("Not JSON"));
            }
            _ => panic!("Expected FlowFailed error"),
        }
    }

    #[test]
    fn test_token_response_deserialization() {
        let json = r#"{
            "access_token": "access123",
            "refresh_token": "refresh456",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "openid email"
        }"#;

        let response: TokenResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.access_token, "access123");
        assert_eq!(response.refresh_token, Some("refresh456".to_string()));
        assert_eq!(response.token_type, Some("Bearer".to_string()));
        assert_eq!(response.expires_in, Some(3600));
        assert_eq!(response.scope, Some("openid email".to_string()));
    }

    #[test]
    fn test_token_response_minimal() {
        let json = r#"{"access_token": "access123"}"#;

        let response: TokenResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.access_token, "access123");
        assert!(response.refresh_token.is_none());
        assert!(response.token_type.is_none());
        assert!(response.expires_in.is_none());
    }

    #[test]
    fn test_userinfo_response_deserialization() {
        let json = r#"{
            "sub": "user123",
            "email": "user@example.com",
            "name": "Test User",
            "picture": "https://example.com/avatar.png"
        }"#;

        let response: UserInfoResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.sub, "user123");
        assert_eq!(response.email, Some("user@example.com".to_string()));
        assert_eq!(response.name, Some("Test User".to_string()));
        assert_eq!(
            response.picture,
            Some("https://example.com/avatar.png".to_string())
        );
    }
}
