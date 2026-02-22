//! Gemini (Google) OAuth2 PKCE provider implementation.
//!
//! Implements OAuth2 Authorization Code flow with PKCE for Google/Gemini API.
//!
//! # Flow
//!
//! 1. Generate PKCE code_verifier and code_challenge
//! 2. Redirect user to Google authorization URL
//! 3. User authorizes and is redirected back with authorization code
//! 4. Exchange code + code_verifier for access/refresh tokens
//! 5. Use access token for Gemini API calls, refresh when expired

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::oauth::claude::{generate_code_challenge, generate_code_verifier, generate_state_token};
use crate::oauth::error::{OAuthError, OAuthResult};
use crate::oauth::provider::OAuthProvider;
use crate::oauth::types::{AccountInfo, AuthFlowState, TokenBundle, TokenValidation};

/// Google OAuth2 endpoints
const AUTHORIZATION_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const USERINFO_ENDPOINT: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";

/// Default scopes for Gemini API access
const DEFAULT_SCOPES: &[&str] = &[
    "openid",
    "email",
    "profile",
    "https://www.googleapis.com/auth/generative-language",
];

/// Google OAuth client ID for CLI applications.
///
/// This is a public client ID for desktop/CLI applications.
/// In production, this should be configured via environment or config.
const DEFAULT_CLIENT_ID: &str = "your-google-client-id.apps.googleusercontent.com";

/// Gemini OAuth provider using Google OAuth2 with PKCE flow.
#[derive(Debug, Clone)]
pub struct GeminiOAuthProvider {
    /// OAuth client ID from Google Cloud Console
    client_id: String,
    /// HTTP client for OAuth requests
    http_client: reqwest::Client,
    /// Optional client secret (for confidential clients)
    client_secret: Option<String>,
}

impl GeminiOAuthProvider {
    /// Create a new Gemini OAuth provider with default client ID.
    ///
    /// Note: In production, configure the client ID via environment variable
    /// `GOOGLE_CLIENT_ID` or configuration file.
    pub fn new() -> Self {
        Self::with_client_id(
            std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string()),
        )
    }

    /// Create a new Gemini OAuth provider with a specific client ID.
    pub fn with_client_id(client_id: String) -> Self {
        Self {
            client_id,
            http_client: reqwest::Client::new(),
            client_secret: None,
        }
    }

    /// Create with client secret for confidential clients.
    pub fn with_secret(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            http_client: reqwest::Client::new(),
            client_secret: Some(client_secret),
        }
    }

    /// Create with custom HTTP client.
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = client;
        self
    }

    /// Build the callback URL for the given port.
    fn callback_url(port: u16) -> String {
        format!("http://127.0.0.1:{}/oauth/gemini/callback", port)
    }

    /// Revoke a token (logout).
    pub async fn revoke_token(&self, token: &str) -> OAuthResult<()> {
        let response = self
            .http_client
            .post(REVOKE_ENDPOINT)
            .form(&[("token", token)])
            .send()
            .await
            .map_err(OAuthError::HttpError)?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            warn!("Token revocation failed: {}", body);
            return Err(OAuthError::FlowFailed(format!(
                "Token revocation failed: {}",
                body
            )));
        }

        info!("Successfully revoked token");
        Ok(())
    }
}

impl Default for GeminiOAuthProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OAuthProvider for GeminiOAuthProvider {
    fn provider_id(&self) -> &str {
        "gemini"
    }

    fn display_name(&self) -> &str {
        "Gemini (Google)"
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
        DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect()
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

        // Google-specific parameters
        let auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
            AUTHORIZATION_ENDPOINT,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(&state),
            urlencoding::encode(&code_challenge),
        );

        info!(
            "Starting Gemini OAuth flow, redirect to: {}",
            AUTHORIZATION_ENDPOINT
        );
        debug!("Full auth URL: {}", auth_url);

        // Create flow state
        let flow_state =
            AuthFlowState::new(state, "gemini".to_string(), callback_port).with_pkce(code_verifier);

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
            return Err(parse_google_error(&body, status.as_u16()));
        }

        let token_response: GoogleTokenResponse = serde_json::from_str(&body).map_err(|e| {
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

        Ok(TokenBundle {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
            expires_at,
            scope: token_response.scope,
            provider: "gemini".to_string(),
            account_id,
            metadata: Default::default(),
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

        info!("Refreshing Gemini access token");

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
            return Err(parse_google_error(&body, status.as_u16()));
        }

        let token_response: GoogleTokenResponse = serde_json::from_str(&body).map_err(|e| {
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

        // Note: Google may not return a new refresh token on refresh
        // We keep the original refresh token in that case
        Ok(TokenBundle {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
            expires_at,
            scope: token_response.scope,
            provider: "gemini".to_string(),
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
                    expires_in_seconds: None,
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

impl GeminiOAuthProvider {
    /// Fetch user info from Google's userinfo endpoint.
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

        let userinfo: GoogleUserInfo = serde_json::from_str(&body).map_err(|e| {
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

/// Google OAuth2 token response.
#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
    /// ID token (JWT) for OpenID Connect
    #[allow(dead_code)]
    id_token: Option<String>,
}

/// Google OpenID Connect userinfo response.
#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    /// Subject identifier (Google user ID)
    sub: String,
    /// Email address
    email: Option<String>,
    /// Whether email is verified
    #[allow(dead_code)]
    email_verified: Option<bool>,
    /// Display name
    name: Option<String>,
    /// Given name
    #[allow(dead_code)]
    given_name: Option<String>,
    /// Family name
    #[allow(dead_code)]
    family_name: Option<String>,
    /// Profile picture URL
    picture: Option<String>,
    /// Locale
    #[allow(dead_code)]
    locale: Option<String>,
}

/// Google OAuth2 error response.
#[derive(Debug, Deserialize)]
struct GoogleErrorResponse {
    error: String,
    error_description: Option<String>,
}

/// Parse a Google OAuth error response.
fn parse_google_error(body: &str, status_code: u16) -> OAuthError {
    if let Ok(error_response) = serde_json::from_str::<GoogleErrorResponse>(body) {
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
            "invalid_scope" => OAuthError::FlowFailed(format!("Invalid scope: {}", description)),
            "server_error" => OAuthError::FlowFailed(format!("Server error: {}", description)),
            "temporarily_unavailable" => {
                if status_code == 429 {
                    OAuthError::RateLimited {
                        provider: "gemini".to_string(),
                        retry_after_seconds: 60,
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
    fn test_gemini_provider_creation() {
        let provider = GeminiOAuthProvider::with_client_id("test_client_id".to_string());

        assert_eq!(provider.provider_id(), "gemini");
        assert_eq!(provider.display_name(), "Gemini (Google)");
        assert!(provider.uses_browser_callback());
    }

    #[test]
    fn test_gemini_provider_with_secret() {
        let provider =
            GeminiOAuthProvider::with_secret("test_client_id".to_string(), "secret".to_string());

        assert!(provider.client_secret.is_some());
        assert_eq!(provider.client_secret.unwrap(), "secret");
    }

    #[test]
    fn test_gemini_provider_endpoints() {
        let provider = GeminiOAuthProvider::with_client_id("test".to_string());

        assert_eq!(
            provider.authorization_endpoint(),
            "https://accounts.google.com/o/oauth2/v2/auth"
        );
        assert_eq!(
            provider.token_endpoint(),
            "https://oauth2.googleapis.com/token"
        );
    }

    #[test]
    fn test_gemini_provider_default_scopes() {
        let provider = GeminiOAuthProvider::with_client_id("test".to_string());
        let scopes = provider.default_scopes();

        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"email".to_string()));
        assert!(scopes.contains(&"profile".to_string()));
        assert!(scopes.contains(&"https://www.googleapis.com/auth/generative-language".to_string()));
    }

    #[test]
    fn test_callback_url() {
        let url = GeminiOAuthProvider::callback_url(54545);
        assert_eq!(url, "http://127.0.0.1:54545/oauth/gemini/callback");
    }

    #[tokio::test]
    async fn test_start_auth_generates_valid_url() {
        let provider = GeminiOAuthProvider::with_client_id("test_client_id".to_string());
        let (auth_url, state) = provider.start_auth(54545).await.unwrap();

        // Check URL contains required parameters
        assert!(auth_url.starts_with(AUTHORIZATION_ENDPOINT));
        assert!(auth_url.contains("response_type=code"));
        assert!(auth_url.contains("client_id=test_client_id"));
        assert!(auth_url.contains("redirect_uri="));
        assert!(auth_url.contains("code_challenge="));
        assert!(auth_url.contains("code_challenge_method=S256"));
        assert!(auth_url.contains("state="));
        // Google-specific parameters
        assert!(auth_url.contains("access_type=offline"));
        assert!(auth_url.contains("prompt=consent"));

        // Check flow state
        assert_eq!(state.provider, "gemini");
        assert_eq!(state.callback_port, 54545);
        assert!(state.code_verifier.is_some());

        // Verify code_challenge matches the verifier
        let verifier = state.code_verifier.as_ref().unwrap();
        let expected_challenge = generate_code_challenge(verifier);
        assert!(auth_url.contains(&expected_challenge));
    }

    #[tokio::test]
    async fn test_start_auth_different_ports() {
        let provider = GeminiOAuthProvider::with_client_id("test".to_string());

        let (url1, _) = provider.start_auth(54545).await.unwrap();
        let (url2, _) = provider.start_auth(54546).await.unwrap();

        assert!(url1.contains("54545"));
        assert!(url2.contains("54546"));
    }

    #[test]
    fn test_parse_google_error_invalid_grant() {
        let body = r#"{"error": "invalid_grant", "error_description": "Token has been expired or revoked"}"#;
        let error = parse_google_error(body, 400);

        match error {
            OAuthError::RefreshFailed(msg) => assert!(msg.contains("expired")),
            _ => panic!("Expected RefreshFailed error"),
        }
    }

    #[test]
    fn test_parse_google_error_invalid_client() {
        let body =
            r#"{"error": "invalid_client", "error_description": "The OAuth client was not found"}"#;
        let error = parse_google_error(body, 401);

        match error {
            OAuthError::FlowFailed(msg) => assert!(msg.contains("Invalid client")),
            _ => panic!("Expected FlowFailed error"),
        }
    }

    #[test]
    fn test_parse_google_error_invalid_scope() {
        let body = r#"{"error": "invalid_scope", "error_description": "Unknown scope requested"}"#;
        let error = parse_google_error(body, 400);

        match error {
            OAuthError::FlowFailed(msg) => assert!(msg.contains("Invalid scope")),
            _ => panic!("Expected FlowFailed error"),
        }
    }

    #[test]
    fn test_parse_google_error_rate_limited() {
        let body =
            r#"{"error": "temporarily_unavailable", "error_description": "Too many requests"}"#;
        let error = parse_google_error(body, 429);

        match error {
            OAuthError::RateLimited {
                provider,
                retry_after_seconds,
            } => {
                assert_eq!(provider, "gemini");
                assert_eq!(retry_after_seconds, 60);
            }
            _ => panic!("Expected RateLimited error"),
        }
    }

    #[test]
    fn test_parse_google_error_malformed_json() {
        let body = "Not JSON";
        let error = parse_google_error(body, 500);

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
            "access_token": "ya29.access123",
            "refresh_token": "1//refresh456",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "openid email profile",
            "id_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."
        }"#;

        let response: GoogleTokenResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.access_token, "ya29.access123");
        assert_eq!(response.refresh_token, Some("1//refresh456".to_string()));
        assert_eq!(response.token_type, Some("Bearer".to_string()));
        assert_eq!(response.expires_in, Some(3600));
        assert_eq!(response.scope, Some("openid email profile".to_string()));
    }

    #[test]
    fn test_token_response_minimal() {
        let json = r#"{"access_token": "ya29.access123"}"#;

        let response: GoogleTokenResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.access_token, "ya29.access123");
        assert!(response.refresh_token.is_none());
        assert!(response.token_type.is_none());
        assert!(response.expires_in.is_none());
    }

    #[test]
    fn test_userinfo_response_deserialization() {
        let json = r#"{
            "sub": "123456789012345678901",
            "email": "user@gmail.com",
            "email_verified": true,
            "name": "Test User",
            "given_name": "Test",
            "family_name": "User",
            "picture": "https://lh3.googleusercontent.com/avatar.png",
            "locale": "en"
        }"#;

        let response: GoogleUserInfo = serde_json::from_str(json).unwrap();

        assert_eq!(response.sub, "123456789012345678901");
        assert_eq!(response.email, Some("user@gmail.com".to_string()));
        assert_eq!(response.name, Some("Test User".to_string()));
        assert_eq!(
            response.picture,
            Some("https://lh3.googleusercontent.com/avatar.png".to_string())
        );
    }

    #[test]
    fn test_userinfo_response_minimal() {
        let json = r#"{"sub": "123456789"}"#;

        let response: GoogleUserInfo = serde_json::from_str(json).unwrap();

        assert_eq!(response.sub, "123456789");
        assert!(response.email.is_none());
        assert!(response.name.is_none());
    }

    #[test]
    fn test_gemini_provider_default() {
        // Just verify Default trait works
        let _provider = GeminiOAuthProvider::default();
    }
}
