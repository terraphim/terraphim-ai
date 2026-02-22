//! OpenAI OAuth2 PKCE provider implementation for Codex CLI integration.
//!
//! Implements OAuth2 Authorization Code flow with PKCE for OpenAI API access,
//! specifically designed for GPT-5.2 reasoning/thinking tasks via Codex CLI.
//!
//! # Flow
//!
//! 1. Generate PKCE code_verifier and code_challenge
//! 2. Redirect user to OpenAI authorization URL
//! 3. User authorizes and is redirected back with authorization code
//! 4. Exchange code + code_verifier for access/refresh tokens
//! 5. Parse JWT to extract expiration and account information
//! 6. Use access token for API calls, refresh when expired
//!
//! # Token Storage
//!
//! Tokens are stored at: `~/.terraphim-llm-proxy/auth/openai/{account_id}.json`
//!
//! # JWT Claims
//!
//! The access_token is a JWT containing:
//! - `sub`: Account ID (e.g., `eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa`)
//! - `exp`: Expiration timestamp (Unix timestamp)
//! - Other standard JWT claims

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::oauth::claude::{generate_code_challenge, generate_code_verifier, generate_state_token};
use crate::oauth::error::{OAuthError, OAuthResult};
use crate::oauth::provider::OAuthProvider;
use crate::oauth::types::{AccountInfo, AuthFlowState, TokenBundle, TokenValidation};

/// OpenAI OAuth2 endpoints
const AUTHORIZATION_ENDPOINT: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";

/// Default scopes for OpenAI API access
const DEFAULT_SCOPES: &[&str] = &["openid", "email", "profile", "api"];

/// OpenAI OAuth provider using PKCE flow.
#[derive(Debug, Clone)]
pub struct OpenAiOAuthProvider {
    /// OAuth client ID from OpenAI developer console
    client_id: String,
    /// HTTP client for OAuth requests
    http_client: reqwest::Client,
    /// Optional client secret (for confidential clients)
    client_secret: Option<String>,
}

/// OpenAI token response
#[derive(Debug, Deserialize)]
struct OpenAiTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
    /// ID token (JWT) for OpenID Connect (present in response but not used directly)
    #[allow(dead_code)]
    id_token: Option<String>,
}

/// OpenAI OAuth error response
#[derive(Debug, Deserialize)]
struct OpenAiErrorResponse {
    error: String,
    error_description: Option<String>,
}

/// JWT Claims for parsing OpenAI access tokens
#[derive(Debug, Deserialize)]
struct JwtClaims {
    /// Subject (account ID)
    sub: String,
    /// Expiration time (Unix timestamp)
    exp: i64,
    /// Issued at (Unix timestamp)
    #[allow(dead_code)]
    iat: Option<i64>,
    /// Issuer
    #[allow(dead_code)]
    iss: Option<String>,
    /// Audience
    #[allow(dead_code)]
    aud: Option<String>,
    /// Email address
    email: Option<String>,
    /// Display name
    name: Option<String>,
}

impl OpenAiOAuthProvider {
    /// Create a new OpenAI OAuth provider.
    ///
    /// # Arguments
    /// * `client_id` - OAuth client ID from OpenAI developer console
    pub fn new(client_id: String) -> Self {
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
        format!("http://127.0.0.1:{}/oauth/openai/callback", port)
    }

    /// Parse a JWT token and extract claims.
    ///
    /// # Arguments
    /// * `jwt` - The JWT token string
    ///
    /// # Returns
    /// Parsed JWT claims or an error if parsing fails
    fn parse_jwt(jwt: &str) -> OAuthResult<JwtClaims> {
        // JWT structure: header.payload.signature
        let parts: Vec<&str> = jwt.split('.').collect();
        if parts.len() != 3 {
            return Err(OAuthError::ValidationFailed(
                "Invalid JWT format: expected 3 parts".to_string(),
            ));
        }

        // Decode the payload (base64url encoded)
        let payload = parts[1];
        let decoded = URL_SAFE_NO_PAD.decode(payload).map_err(|e| {
            OAuthError::ValidationFailed(format!("Failed to decode JWT payload: {}", e))
        })?;

        let claims: JwtClaims = serde_json::from_slice(&decoded).map_err(|e| {
            OAuthError::ValidationFailed(format!("Failed to parse JWT claims: {}", e))
        })?;

        Ok(claims)
    }

    /// Extract account information from JWT claims.
    ///
    /// # Arguments
    /// * `access_token` - The access token (JWT)
    ///
    /// # Returns
    /// Account info extracted from the JWT
    fn extract_account_info(access_token: &str) -> OAuthResult<AccountInfo> {
        let claims = Self::parse_jwt(access_token)?;

        Ok(AccountInfo {
            id: claims.sub.clone(),
            email: claims.email,
            name: claims.name,
            avatar_url: None,
        })
    }

    /// Extract expiration time from JWT claims.
    ///
    /// # Arguments
    /// * `access_token` - The access token (JWT)
    ///
    /// # Returns
    /// Expiration timestamp or None if parsing fails
    fn extract_expiration(access_token: &str) -> Option<DateTime<Utc>> {
        match Self::parse_jwt(access_token) {
            Ok(claims) => {
                // Convert Unix timestamp to DateTime<Utc>
                Utc.timestamp_opt(claims.exp, 0).single()
            }
            Err(e) => {
                warn!(error = %e, "Failed to parse JWT for expiration");
                None
            }
        }
    }
}

#[async_trait]
impl OAuthProvider for OpenAiOAuthProvider {
    fn provider_id(&self) -> &str {
        "openai"
    }

    fn display_name(&self) -> &str {
        "OpenAI (Codex)"
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
            "Starting OpenAI OAuth flow, redirect to: {}",
            AUTHORIZATION_ENDPOINT
        );
        debug!("Full auth URL: {}", auth_url);

        // Create flow state
        let flow_state =
            AuthFlowState::new(state, "openai".to_string(), callback_port).with_pkce(code_verifier);

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

        info!("Exchanging authorization code for OpenAI tokens");

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
            return Err(parse_openai_error(&body, status.as_u16()));
        }

        let token_response: OpenAiTokenResponse = serde_json::from_str(&body).map_err(|e| {
            OAuthError::FlowFailed(format!("Failed to parse token response: {}", e))
        })?;

        // Extract account info from JWT
        let account_info = Self::extract_account_info(&token_response.access_token).ok();

        let account_id = account_info
            .as_ref()
            .map(|info| info.id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Try to extract expiration from JWT, fallback to expires_in
        let expires_at = Self::extract_expiration(&token_response.access_token).or_else(|| {
            token_response
                .expires_in
                .map(|secs| Utc::now() + Duration::seconds(secs as i64))
        });

        info!(
            "Successfully obtained OpenAI tokens for account: {}",
            account_id
        );

        Ok(TokenBundle {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
            expires_at,
            scope: token_response.scope,
            provider: "openai".to_string(),
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

        info!("Refreshing OpenAI access token");

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
            return Err(parse_openai_error(&body, status.as_u16()));
        }

        let token_response: OpenAiTokenResponse = serde_json::from_str(&body).map_err(|e| {
            OAuthError::RefreshFailed(format!("Failed to parse refresh response: {}", e))
        })?;

        // Extract account info from JWT
        let account_info = Self::extract_account_info(&token_response.access_token).ok();

        let account_id = account_info
            .as_ref()
            .map(|info| info.id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Try to extract expiration from JWT, fallback to expires_in
        let expires_at = Self::extract_expiration(&token_response.access_token).or_else(|| {
            token_response
                .expires_in
                .map(|secs| Utc::now() + Duration::seconds(secs as i64))
        });

        info!(
            "Successfully refreshed OpenAI token for account: {}",
            account_id
        );

        Ok(TokenBundle {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
            expires_at,
            scope: token_response.scope,
            provider: "openai".to_string(),
            account_id,
            metadata: Default::default(),
            created_at: Utc::now(),
            last_refresh: Some(Utc::now()),
        })
    }

    async fn validate_token(&self, access_token: &str) -> OAuthResult<TokenValidation> {
        match Self::extract_account_info(access_token) {
            Ok(info) => {
                debug!("OpenAI token validated successfully for: {}", info.id);

                // Try to extract expiration from JWT
                let expires_at = Self::extract_expiration(access_token);
                let expires_in_seconds = expires_at.map(|exp| {
                    let remaining = exp - Utc::now();
                    remaining.num_seconds().max(0)
                });

                Ok(TokenValidation {
                    valid: true,
                    expires_in_seconds,
                    scopes: self.default_scopes(),
                    account_info: Some(info),
                    error: None,
                })
            }
            Err(e) => {
                warn!("OpenAI token validation failed: {}", e);
                Ok(TokenValidation::invalid(e.to_string()))
            }
        }
    }
}

/// Parse an OpenAI OAuth error response.
fn parse_openai_error(body: &str, status_code: u16) -> OAuthError {
    if let Ok(error_response) = serde_json::from_str::<OpenAiErrorResponse>(body) {
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
            "unsupported_grant_type" => {
                OAuthError::FlowFailed(format!("Unsupported grant type: {}", description))
            }
            "invalid_scope" => OAuthError::FlowFailed(format!("Invalid scope: {}", description)),
            "temporarily_unavailable" => {
                if status_code == 429 {
                    OAuthError::RateLimited {
                        provider: "openai".to_string(),
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
    fn test_openai_provider_creation() {
        let provider = OpenAiOAuthProvider::new("test_client_id".to_string());

        assert_eq!(provider.provider_id(), "openai");
        assert_eq!(provider.display_name(), "OpenAI (Codex)");
        assert!(provider.uses_browser_callback());
    }

    #[test]
    fn test_openai_provider_with_secret() {
        let provider =
            OpenAiOAuthProvider::with_secret("test_client_id".to_string(), "secret".to_string());

        assert!(provider.client_secret.is_some());
        assert_eq!(provider.client_secret.unwrap(), "secret");
    }

    #[test]
    fn test_openai_provider_endpoints() {
        let provider = OpenAiOAuthProvider::new("test".to_string());

        assert_eq!(
            provider.authorization_endpoint(),
            "https://auth.openai.com/oauth/authorize"
        );
        assert_eq!(
            provider.token_endpoint(),
            "https://auth.openai.com/oauth/token"
        );
    }

    #[test]
    fn test_openai_provider_default_scopes() {
        let provider = OpenAiOAuthProvider::new("test".to_string());
        let scopes = provider.default_scopes();

        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"email".to_string()));
        assert!(scopes.contains(&"profile".to_string()));
        assert!(scopes.contains(&"api".to_string()));
    }

    #[test]
    fn test_callback_url() {
        let url = OpenAiOAuthProvider::callback_url(54545);
        assert_eq!(url, "http://127.0.0.1:54545/oauth/openai/callback");
    }

    #[test]
    fn test_parse_jwt_valid() {
        // Create a valid JWT with known claims
        // Header: {"alg":"RS256","typ":"JWT"}
        let header = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9";

        // Payload: {"sub":"eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa","exp":1707177600,"email":"test@example.com","name":"Test User"}
        let payload = "eyJzdWIiOiJlYjc4ZmQxZS1mYWQwLTQyZTAtYjliZC0wNjc0YzdlYTk0ZmEiLCJleHAiOjE3MDcxNzc2MDAsImVtYWlsIjoidGVzdEBleGFtcGxlLmNvbSIsIm5hbWUiOiJUZXN0IFVzZXIifQ";

        // Signature (not validated, just for structure)
        let signature = "dummy_signature";

        let jwt = format!("{}.{}.{}", header, payload, signature);

        let claims = OpenAiOAuthProvider::parse_jwt(&jwt).unwrap();

        assert_eq!(claims.sub, "eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa");
        assert_eq!(claims.exp, 1707177600);
        assert_eq!(claims.email, Some("test@example.com".to_string()));
        assert_eq!(claims.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_parse_jwt_invalid_format() {
        let result = OpenAiOAuthProvider::parse_jwt("invalid.jwt");
        assert!(result.is_err());

        let result = OpenAiOAuthProvider::parse_jwt("not_enough_parts");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_account_info() {
        // Header: {"alg":"RS256","typ":"JWT"}
        let header = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9";

        // Payload with account info
        let payload = "eyJzdWIiOiJlYjc4ZmQxZS1mYWQwLTQyZTAtYjliZC0wNjc0YzdlYTk0ZmEiLCJleHAiOjE3MDcxNzc2MDAsImVtYWlsIjoidGVzdEBleGFtcGxlLmNvbSIsIm5hbWUiOiJUZXN0IFVzZXIifQ";

        let jwt = format!("{}.{}.{}", header, payload, "sig");

        let info = OpenAiOAuthProvider::extract_account_info(&jwt).unwrap();

        assert_eq!(info.id, "eb78fd1e-fad0-42e0-b9bd-0674c7ea94fa");
        assert_eq!(info.email, Some("test@example.com".to_string()));
        assert_eq!(info.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_extract_expiration() {
        // Header
        let header = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9";

        // Payload with exp: 1707177600 (Feb 5, 2024 20:00:00 UTC)
        let payload = "eyJzdWIiOiJ0ZXN0IiwiZXhwIjoxNzA3MTc3NjAwfQ";

        let jwt = format!("{}.{}.{}", header, payload, "sig");

        let exp = OpenAiOAuthProvider::extract_expiration(&jwt);
        assert!(exp.is_some());

        let expected = Utc.timestamp_opt(1707177600, 0).single().unwrap();
        assert_eq!(exp.unwrap(), expected);
    }

    #[test]
    fn test_token_response_deserialization() {
        let json = r#"{
            "access_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0In0.sig",
            "refresh_token": "refresh123",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "openid email api",
            "id_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0In0.sig2"
        }"#;

        let response: OpenAiTokenResponse = serde_json::from_str(json).unwrap();

        assert!(response.access_token.contains("eyJhbGci"));
        assert_eq!(response.refresh_token, Some("refresh123".to_string()));
        assert_eq!(response.token_type, Some("Bearer".to_string()));
        assert_eq!(response.expires_in, Some(3600));
        assert_eq!(response.scope, Some("openid email api".to_string()));
    }

    #[test]
    fn test_parse_openai_error_invalid_grant() {
        let body = r#"{"error": "invalid_grant", "error_description": "Token has been revoked"}"#;
        let error = parse_openai_error(body, 400);

        match error {
            OAuthError::RefreshFailed(msg) => assert!(msg.contains("revoked")),
            _ => panic!("Expected RefreshFailed error"),
        }
    }

    #[test]
    fn test_parse_openai_error_invalid_client() {
        let body =
            r#"{"error": "invalid_client", "error_description": "Client authentication failed"}"#;
        let error = parse_openai_error(body, 401);

        match error {
            OAuthError::FlowFailed(msg) => assert!(msg.contains("Invalid client")),
            _ => panic!("Expected FlowFailed error"),
        }
    }

    #[test]
    fn test_parse_openai_error_rate_limited() {
        let body =
            r#"{"error": "temporarily_unavailable", "error_description": "Rate limit exceeded"}"#;
        let error = parse_openai_error(body, 429);

        match error {
            OAuthError::RateLimited { provider, .. } => {
                assert_eq!(provider, "openai");
            }
            _ => panic!("Expected RateLimited error"),
        }
    }

    #[tokio::test]
    async fn test_start_auth_generates_valid_url() {
        let provider = OpenAiOAuthProvider::new("test_client_id".to_string());
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
        assert_eq!(state.provider, "openai");
        assert_eq!(state.callback_port, 54545);
        assert!(state.code_verifier.is_some());
    }
}
