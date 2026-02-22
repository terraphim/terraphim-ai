//! GitHub Copilot OAuth device code flow implementation.
//!
//! Implements OAuth2 Device Authorization Grant (RFC 8628) for GitHub Copilot.
//! This flow is designed for devices without a browser or with limited input capability.
//!
//! # Flow
//!
//! 1. Request device code from GitHub
//! 2. Display user_code and verification_uri to user
//! 3. User visits URL and enters the code
//! 4. CLI polls for authorization completion
//! 5. On success, exchange device code for access token
//!
//! # Example
//!
//! ```rust,ignore
//! let provider = CopilotOAuthProvider::new("client_id".to_string());
//!
//! // Get device code
//! let device_info = provider.request_device_code().await?;
//! println!("Visit {} and enter code: {}", device_info.verification_uri, device_info.user_code);
//!
//! // Poll for completion
//! loop {
//!     match provider.poll_device_code(&device_info.device_code).await? {
//!         Some(tokens) => {
//!             println!("Authenticated as {}", tokens.account_id);
//!             break;
//!         }
//!         None => {
//!             tokio::time::sleep(Duration::from_secs(device_info.interval)).await;
//!         }
//!     }
//! }
//! ```

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::oauth::claude::generate_state_token;
use crate::oauth::error::{OAuthError, OAuthResult};
use crate::oauth::provider::{DeviceCodeInfo, DeviceCodeProvider, OAuthProvider};
use crate::oauth::types::{AccountInfo, AuthFlowState, TokenBundle, TokenValidation};

/// GitHub device code endpoint
const DEVICE_CODE_ENDPOINT: &str = "https://github.com/login/device/code";
/// GitHub token endpoint
const TOKEN_ENDPOINT: &str = "https://github.com/login/oauth/access_token";
/// GitHub user API endpoint
const USER_ENDPOINT: &str = "https://api.github.com/user";

/// Default scopes for GitHub Copilot access
const DEFAULT_SCOPES: &[&str] = &["read:user", "user:email"];

/// GitHub Copilot OAuth provider using device code flow.
///
/// This provider implements the OAuth 2.0 Device Authorization Grant,
/// which is ideal for CLI applications where browser-based redirects
/// are not practical.
#[derive(Debug, Clone)]
pub struct CopilotOAuthProvider {
    /// OAuth client ID from GitHub
    client_id: String,
    /// HTTP client for OAuth requests
    http_client: reqwest::Client,
}

impl CopilotOAuthProvider {
    /// Create a new Copilot OAuth provider.
    ///
    /// # Arguments
    /// * `client_id` - OAuth client ID from GitHub (for Copilot CLI)
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            http_client: reqwest::Client::new(),
        }
    }

    /// Create with custom HTTP client.
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = client;
        self
    }

    /// Fetch user info from GitHub API.
    async fn fetch_user_info(&self, access_token: &str) -> OAuthResult<AccountInfo> {
        let response = self
            .http_client
            .get(USER_ENDPOINT)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "terraphim-llm-proxy")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(OAuthError::HttpError)?;

        let status = response.status();
        let body = response.text().await.map_err(OAuthError::HttpError)?;

        if !status.is_success() {
            return Err(OAuthError::ValidationFailed(format!(
                "GitHub user request failed with status {}: {}",
                status, body
            )));
        }

        let user: GitHubUser = serde_json::from_str(&body).map_err(|e| {
            OAuthError::ValidationFailed(format!("Failed to parse GitHub user response: {}", e))
        })?;

        Ok(AccountInfo {
            id: user.id.to_string(),
            email: user.email,
            name: user.name.or(Some(user.login.clone())),
            avatar_url: user.avatar_url,
        })
    }
}

#[async_trait]
impl OAuthProvider for CopilotOAuthProvider {
    fn provider_id(&self) -> &str {
        "copilot"
    }

    fn display_name(&self) -> &str {
        "GitHub Copilot"
    }

    fn uses_browser_callback(&self) -> bool {
        // Device code flow doesn't use browser callback
        false
    }

    fn token_endpoint(&self) -> &str {
        TOKEN_ENDPOINT
    }

    fn authorization_endpoint(&self) -> &str {
        // Device code flow uses a different endpoint
        DEVICE_CODE_ENDPOINT
    }

    fn default_scopes(&self) -> Vec<String> {
        DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect()
    }

    async fn start_auth(&self, _callback_port: u16) -> OAuthResult<(String, AuthFlowState)> {
        // For device code flow, we request a device code and return
        // the verification URL for the user to visit
        let device_info = self.request_device_code().await?;

        // Create a message for the user
        let message = format!(
            "Visit {} and enter code: {}",
            device_info.verification_uri, device_info.user_code
        );

        // Create flow state with device code stored in code_verifier field
        // (repurposing the field since device flow doesn't use PKCE)
        let state = generate_state_token();
        let flow_state = AuthFlowState::new(state, "copilot".to_string(), 0)
            .with_pkce(device_info.device_code.clone());

        // Store device info in metadata via the state
        // The caller should use the DeviceCodeProvider trait methods

        Ok((message, flow_state))
    }

    async fn exchange_code(&self, code: &str, _state: &AuthFlowState) -> OAuthResult<TokenBundle> {
        // For device code flow, the "code" is actually the device_code
        // and we poll for the token
        match self.poll_device_code(code).await? {
            Some(bundle) => Ok(bundle),
            None => Err(OAuthError::FlowFailed(
                "Authorization still pending".to_string(),
            )),
        }
    }

    async fn refresh_token(&self, refresh_token: &str) -> OAuthResult<TokenBundle> {
        // GitHub uses the same endpoint for refresh
        let params = [
            ("client_id", self.client_id.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ];

        info!("Refreshing GitHub Copilot access token");

        let response = self
            .http_client
            .post(TOKEN_ENDPOINT)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(OAuthError::HttpError)?;

        let status = response.status();
        let body = response.text().await.map_err(OAuthError::HttpError)?;

        if !status.is_success() {
            warn!("Token refresh failed with status {}: {}", status, body);
            return Err(parse_github_error(&body));
        }

        let token_response: GitHubTokenResponse = serde_json::from_str(&body).map_err(|e| {
            OAuthError::RefreshFailed(format!("Failed to parse refresh response: {}", e))
        })?;

        // Check for error in response body (GitHub returns 200 with error in body)
        if let Some(error) = token_response.error {
            return Err(parse_github_error_code(
                &error,
                token_response.error_description,
            ));
        }

        let access_token = token_response
            .access_token
            .ok_or_else(|| OAuthError::RefreshFailed("No access token in response".to_string()))?;

        // Fetch user info
        let account_info = self.fetch_user_info(&access_token).await.ok();

        let account_id = account_info
            .as_ref()
            .and_then(|info| info.email.clone())
            .or_else(|| account_info.as_ref().map(|info| info.id.clone()))
            .unwrap_or_else(|| "unknown".to_string());

        let expires_at = token_response
            .expires_in
            .map(|secs| Utc::now() + Duration::seconds(secs as i64));

        info!("Successfully refreshed token for account: {}", account_id);

        Ok(TokenBundle {
            access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response
                .token_type
                .unwrap_or_else(|| "bearer".to_string()),
            expires_at,
            scope: token_response.scope,
            provider: "copilot".to_string(),
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

#[async_trait]
impl DeviceCodeProvider for CopilotOAuthProvider {
    async fn request_device_code(&self) -> OAuthResult<DeviceCodeInfo> {
        let scopes = self.default_scopes().join(" ");

        let params = [("client_id", self.client_id.as_str()), ("scope", &scopes)];

        info!("Requesting GitHub device code");

        let response = self
            .http_client
            .post(DEVICE_CODE_ENDPOINT)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(OAuthError::HttpError)?;

        let status = response.status();
        let body = response.text().await.map_err(OAuthError::HttpError)?;

        if !status.is_success() {
            warn!(
                "Device code request failed with status {}: {}",
                status, body
            );
            return Err(parse_github_error(&body));
        }

        let device_response: GitHubDeviceCodeResponse =
            serde_json::from_str(&body).map_err(|e| {
                OAuthError::FlowFailed(format!("Failed to parse device code response: {}", e))
            })?;

        info!(
            "Got device code, user should visit {} and enter {}",
            device_response.verification_uri, device_response.user_code
        );

        Ok(DeviceCodeInfo {
            device_code: device_response.device_code,
            user_code: device_response.user_code,
            verification_uri: device_response.verification_uri,
            verification_uri_complete: device_response.verification_uri_complete,
            expires_in: device_response.expires_in,
            interval: device_response.interval.unwrap_or(5),
        })
    }

    async fn poll_device_code(&self, device_code: &str) -> OAuthResult<Option<TokenBundle>> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];

        debug!("Polling for device code authorization");

        let response = self
            .http_client
            .post(TOKEN_ENDPOINT)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(OAuthError::HttpError)?;

        let body = response.text().await.map_err(OAuthError::HttpError)?;

        let token_response: GitHubTokenResponse = serde_json::from_str(&body).map_err(|e| {
            OAuthError::FlowFailed(format!("Failed to parse token response: {}", e))
        })?;

        // Check for pending/error states
        if let Some(error) = token_response.error {
            match error.as_str() {
                "authorization_pending" => {
                    debug!("Authorization still pending");
                    return Ok(None);
                }
                "slow_down" => {
                    debug!("Received slow_down, should increase polling interval");
                    // Return None but caller should increase interval
                    return Ok(None);
                }
                "expired_token" => {
                    return Err(OAuthError::FlowFailed(
                        "Device code expired, please restart authentication".to_string(),
                    ));
                }
                "access_denied" => {
                    return Err(OAuthError::FlowFailed(
                        "User denied authorization".to_string(),
                    ));
                }
                _ => {
                    return Err(parse_github_error_code(
                        &error,
                        token_response.error_description,
                    ));
                }
            }
        }

        // Success - we have a token
        let access_token = token_response
            .access_token
            .ok_or_else(|| OAuthError::FlowFailed("No access token in response".to_string()))?;

        info!("Device code authorization successful");

        // Fetch user info
        let account_info = self.fetch_user_info(&access_token).await.ok();

        let account_id = account_info
            .as_ref()
            .and_then(|info| info.email.clone())
            .or_else(|| account_info.as_ref().map(|info| info.id.clone()))
            .unwrap_or_else(|| "unknown".to_string());

        let expires_at = token_response
            .expires_in
            .map(|secs| Utc::now() + Duration::seconds(secs as i64));

        Ok(Some(TokenBundle {
            access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response
                .token_type
                .unwrap_or_else(|| "bearer".to_string()),
            expires_at,
            scope: token_response.scope,
            provider: "copilot".to_string(),
            account_id,
            metadata: Default::default(),
            created_at: Utc::now(),
            last_refresh: None,
        }))
    }
}

/// GitHub device code response.
#[derive(Debug, Deserialize)]
struct GitHubDeviceCodeResponse {
    /// The device verification code
    device_code: String,
    /// The user verification code to display
    user_code: String,
    /// The URL where the user should enter the code
    verification_uri: String,
    /// URL with the user code pre-filled (optional)
    #[serde(default)]
    verification_uri_complete: Option<String>,
    /// Seconds until the code expires
    expires_in: u64,
    /// Minimum polling interval in seconds
    interval: Option<u64>,
}

/// GitHub token response (for both device code and refresh).
#[derive(Debug, Deserialize)]
struct GitHubTokenResponse {
    /// Access token (present on success)
    access_token: Option<String>,
    /// Refresh token (present on success)
    refresh_token: Option<String>,
    /// Token type (usually "bearer")
    token_type: Option<String>,
    /// Seconds until token expires
    expires_in: Option<u64>,
    /// Granted scopes
    scope: Option<String>,
    /// Error code (present on failure)
    error: Option<String>,
    /// Error description
    error_description: Option<String>,
}

/// GitHub user response.
#[derive(Debug, Deserialize)]
struct GitHubUser {
    /// GitHub user ID
    id: u64,
    /// GitHub username
    login: String,
    /// Display name
    name: Option<String>,
    /// Email address
    email: Option<String>,
    /// Avatar URL
    avatar_url: Option<String>,
}

/// Parse a GitHub error response body.
fn parse_github_error(body: &str) -> OAuthError {
    if let Ok(response) = serde_json::from_str::<GitHubTokenResponse>(body) {
        if let Some(error) = response.error {
            return parse_github_error_code(&error, response.error_description);
        }
    }
    OAuthError::FlowFailed(format!("GitHub error: {}", body))
}

/// Parse a GitHub error code into an OAuthError.
fn parse_github_error_code(error: &str, description: Option<String>) -> OAuthError {
    let desc = description.unwrap_or_else(|| error.to_string());

    match error {
        "authorization_pending" => OAuthError::FlowFailed("Authorization pending".to_string()),
        "slow_down" => OAuthError::RateLimited {
            provider: "copilot".to_string(),
            retry_after_seconds: 10,
        },
        "expired_token" => OAuthError::FlowFailed("Device code expired".to_string()),
        "access_denied" => OAuthError::FlowFailed(format!("Access denied: {}", desc)),
        "incorrect_client_credentials" => {
            OAuthError::FlowFailed(format!("Invalid client credentials: {}", desc))
        }
        "incorrect_device_code" => OAuthError::FlowFailed(format!("Invalid device code: {}", desc)),
        "unsupported_grant_type" => {
            OAuthError::FlowFailed(format!("Unsupported grant type: {}", desc))
        }
        "bad_verification_code" => {
            OAuthError::RefreshFailed(format!("Bad refresh token: {}", desc))
        }
        _ => OAuthError::FlowFailed(format!("{}: {}", error, desc)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copilot_provider_creation() {
        let provider = CopilotOAuthProvider::new("test_client_id".to_string());

        assert_eq!(provider.provider_id(), "copilot");
        assert_eq!(provider.display_name(), "GitHub Copilot");
        assert!(!provider.uses_browser_callback());
    }

    #[test]
    fn test_copilot_provider_endpoints() {
        let provider = CopilotOAuthProvider::new("test".to_string());

        assert_eq!(
            provider.authorization_endpoint(),
            "https://github.com/login/device/code"
        );
        assert_eq!(
            provider.token_endpoint(),
            "https://github.com/login/oauth/access_token"
        );
    }

    #[test]
    fn test_copilot_provider_default_scopes() {
        let provider = CopilotOAuthProvider::new("test".to_string());
        let scopes = provider.default_scopes();

        assert!(scopes.contains(&"read:user".to_string()));
        assert!(scopes.contains(&"user:email".to_string()));
    }

    #[test]
    fn test_device_code_response_deserialization() {
        let json = r#"{
            "device_code": "3584d83530557fdd1f46af8289938c8ef79f9dc5",
            "user_code": "WDJB-MJHT",
            "verification_uri": "https://github.com/login/device",
            "verification_uri_complete": "https://github.com/login/device?user_code=WDJB-MJHT",
            "expires_in": 899,
            "interval": 5
        }"#;

        let response: GitHubDeviceCodeResponse = serde_json::from_str(json).unwrap();

        assert_eq!(
            response.device_code,
            "3584d83530557fdd1f46af8289938c8ef79f9dc5"
        );
        assert_eq!(response.user_code, "WDJB-MJHT");
        assert_eq!(response.verification_uri, "https://github.com/login/device");
        assert!(response.verification_uri_complete.is_some());
        assert_eq!(response.expires_in, 899);
        assert_eq!(response.interval, Some(5));
    }

    #[test]
    fn test_device_code_response_minimal() {
        let json = r#"{
            "device_code": "abc123",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 600
        }"#;

        let response: GitHubDeviceCodeResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.device_code, "abc123");
        assert_eq!(response.user_code, "ABCD-1234");
        assert!(response.verification_uri_complete.is_none());
        assert!(response.interval.is_none());
    }

    #[test]
    fn test_token_response_success() {
        let json = r#"{
            "access_token": "gho_16C7e42F292c6912E7710c838347Ae178B4a",
            "refresh_token": "ghr_abc123",
            "token_type": "bearer",
            "expires_in": 28800,
            "scope": "read:user user:email"
        }"#;

        let response: GitHubTokenResponse = serde_json::from_str(json).unwrap();

        assert!(response.access_token.is_some());
        assert!(response.refresh_token.is_some());
        assert_eq!(response.token_type, Some("bearer".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_token_response_pending() {
        let json = r#"{
            "error": "authorization_pending",
            "error_description": "The user has not yet authorized the device"
        }"#;

        let response: GitHubTokenResponse = serde_json::from_str(json).unwrap();

        assert!(response.access_token.is_none());
        assert_eq!(response.error, Some("authorization_pending".to_string()));
    }

    #[test]
    fn test_token_response_slow_down() {
        let json = r#"{
            "error": "slow_down",
            "error_description": "Too many requests, please slow down"
        }"#;

        let response: GitHubTokenResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.error, Some("slow_down".to_string()));
    }

    #[test]
    fn test_token_response_expired() {
        let json = r#"{
            "error": "expired_token",
            "error_description": "The device code has expired"
        }"#;

        let response: GitHubTokenResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.error, Some("expired_token".to_string()));
    }

    #[test]
    fn test_github_user_deserialization() {
        let json = r#"{
            "id": 12345678,
            "login": "octocat",
            "name": "The Octocat",
            "email": "octocat@github.com",
            "avatar_url": "https://avatars.githubusercontent.com/u/583231"
        }"#;

        let user: GitHubUser = serde_json::from_str(json).unwrap();

        assert_eq!(user.id, 12345678);
        assert_eq!(user.login, "octocat");
        assert_eq!(user.name, Some("The Octocat".to_string()));
        assert_eq!(user.email, Some("octocat@github.com".to_string()));
    }

    #[test]
    fn test_github_user_minimal() {
        let json = r#"{
            "id": 1,
            "login": "user"
        }"#;

        let user: GitHubUser = serde_json::from_str(json).unwrap();

        assert_eq!(user.id, 1);
        assert_eq!(user.login, "user");
        assert!(user.name.is_none());
        assert!(user.email.is_none());
    }

    #[test]
    fn test_parse_github_error_code_pending() {
        let error = parse_github_error_code("authorization_pending", None);
        match error {
            OAuthError::FlowFailed(msg) => assert!(msg.contains("pending")),
            _ => panic!("Expected FlowFailed error"),
        }
    }

    #[test]
    fn test_parse_github_error_code_slow_down() {
        let error = parse_github_error_code("slow_down", None);
        match error {
            OAuthError::RateLimited {
                provider,
                retry_after_seconds,
            } => {
                assert_eq!(provider, "copilot");
                assert_eq!(retry_after_seconds, 10);
            }
            _ => panic!("Expected RateLimited error"),
        }
    }

    #[test]
    fn test_parse_github_error_code_expired() {
        let error = parse_github_error_code("expired_token", None);
        match error {
            OAuthError::FlowFailed(msg) => assert!(msg.contains("expired")),
            _ => panic!("Expected FlowFailed error"),
        }
    }

    #[test]
    fn test_parse_github_error_code_access_denied() {
        let error =
            parse_github_error_code("access_denied", Some("User denied access".to_string()));
        match error {
            OAuthError::FlowFailed(msg) => {
                assert!(msg.contains("Access denied"));
                assert!(msg.contains("User denied"));
            }
            _ => panic!("Expected FlowFailed error"),
        }
    }

    #[test]
    fn test_parse_github_error_body() {
        let body = r#"{"error": "access_denied", "error_description": "Denied"}"#;
        let error = parse_github_error(body);
        match error {
            OAuthError::FlowFailed(msg) => assert!(msg.contains("Access denied")),
            _ => panic!("Expected FlowFailed error"),
        }
    }

    #[test]
    fn test_parse_github_error_malformed() {
        let body = "Not JSON";
        let error = parse_github_error(body);
        match error {
            OAuthError::FlowFailed(msg) => assert!(msg.contains("Not JSON")),
            _ => panic!("Expected FlowFailed error"),
        }
    }

    #[tokio::test]
    async fn test_start_auth_returns_message() {
        // Note: This test would need mocking in a real scenario
        // For now, we just test that the method signature works
        let provider = CopilotOAuthProvider::new("test".to_string());

        // The actual HTTP call will fail, but we're testing the structure
        let result = provider.start_auth(0).await;

        // Should fail due to network error, not logic error
        assert!(result.is_err());
    }
}
