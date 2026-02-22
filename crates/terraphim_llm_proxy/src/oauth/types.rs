//! OAuth type definitions for token management and authentication flows.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Bundle of OAuth tokens with metadata.
///
/// Stores access tokens, refresh tokens, and associated metadata for
/// authenticated sessions with OAuth providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBundle {
    /// Access token for API calls
    pub access_token: String,

    /// Refresh token for obtaining new access tokens
    pub refresh_token: Option<String>,

    /// Token type (usually "Bearer")
    pub token_type: String,

    /// When the access token expires
    pub expires_at: Option<DateTime<Utc>>,

    /// OAuth scope granted
    pub scope: Option<String>,

    /// Provider identifier (e.g., "claude", "gemini", "copilot")
    pub provider: String,

    /// Account identifier (email or username)
    pub account_id: String,

    /// Provider-specific metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,

    /// When the token was first created
    pub created_at: DateTime<Utc>,

    /// Last successful refresh timestamp
    pub last_refresh: Option<DateTime<Utc>>,
}

impl TokenBundle {
    /// Create a new token bundle with minimal required fields.
    pub fn new(
        access_token: String,
        token_type: String,
        provider: String,
        account_id: String,
    ) -> Self {
        Self {
            access_token,
            refresh_token: None,
            token_type,
            expires_at: None,
            scope: None,
            provider,
            account_id,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            last_refresh: None,
        }
    }

    /// Check if token is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp <= Utc::now())
            .unwrap_or(false)
    }

    /// Check if token will expire within the given duration.
    ///
    /// Returns `true` if the token expires within `duration` from now,
    /// or if the token has no expiration set (considered never expiring).
    pub fn expires_within(&self, duration: Duration) -> bool {
        self.expires_at
            .map(|exp| exp <= Utc::now() + duration)
            .unwrap_or(false)
    }

    /// Check if token needs refresh (expires within 4 hours).
    pub fn needs_refresh(&self) -> bool {
        self.expires_within(Duration::hours(4))
    }

    /// Get remaining lifetime of the token in seconds.
    pub fn remaining_seconds(&self) -> Option<i64> {
        self.expires_at.map(|exp| {
            let remaining = exp - Utc::now();
            remaining.num_seconds().max(0)
        })
    }

    /// Update the token after a refresh operation.
    pub fn update_from_refresh(
        &mut self,
        new_access_token: String,
        new_expires_at: Option<DateTime<Utc>>,
    ) {
        self.access_token = new_access_token;
        self.expires_at = new_expires_at;
        self.last_refresh = Some(Utc::now());
    }
}

/// State for tracking in-progress OAuth flows.
///
/// Stores the state token and PKCE verifier for validating OAuth callbacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthFlowState {
    /// CSRF state token for validation
    pub state: String,

    /// PKCE code verifier (for PKCE flows)
    pub code_verifier: Option<String>,

    /// Provider being authenticated
    pub provider: String,

    /// When the flow was started
    pub started_at: DateTime<Utc>,

    /// Callback port for this flow
    pub callback_port: u16,

    /// Optional nonce for OpenID Connect
    pub nonce: Option<String>,
}

impl AuthFlowState {
    /// Create a new auth flow state.
    pub fn new(state: String, provider: String, callback_port: u16) -> Self {
        Self {
            state,
            code_verifier: None,
            provider,
            started_at: Utc::now(),
            callback_port,
            nonce: None,
        }
    }

    /// Create with PKCE code verifier.
    pub fn with_pkce(mut self, code_verifier: String) -> Self {
        self.code_verifier = Some(code_verifier);
        self
    }

    /// Create with OpenID Connect nonce.
    pub fn with_nonce(mut self, nonce: String) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Check if the flow has expired (default: 10 minutes).
    pub fn is_expired(&self) -> bool {
        self.started_at + Duration::minutes(10) < Utc::now()
    }

    /// Validate that the state token matches.
    pub fn validate_state(&self, state: &str) -> bool {
        self.state == state
    }
}

/// Result of token validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidation {
    /// Whether the token is valid
    pub valid: bool,

    /// Seconds until token expires
    pub expires_in_seconds: Option<i64>,

    /// Scopes granted to the token
    pub scopes: Vec<String>,

    /// Account information if available
    pub account_info: Option<AccountInfo>,

    /// Error message if validation failed
    pub error: Option<String>,
}

impl TokenValidation {
    /// Create a valid token validation result.
    pub fn valid(expires_in_seconds: Option<i64>, scopes: Vec<String>) -> Self {
        Self {
            valid: true,
            expires_in_seconds,
            scopes,
            account_info: None,
            error: None,
        }
    }

    /// Create an invalid token validation result.
    pub fn invalid(error: String) -> Self {
        Self {
            valid: false,
            expires_in_seconds: None,
            scopes: Vec::new(),
            account_info: None,
            error: Some(error),
        }
    }
}

/// Account information from OAuth provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Provider-specific account ID
    pub id: String,

    /// Email address if available
    pub email: Option<String>,

    /// Display name if available
    pub name: Option<String>,

    /// Avatar/profile picture URL
    pub avatar_url: Option<String>,
}

impl AccountInfo {
    /// Create new account info with just an ID.
    pub fn new(id: String) -> Self {
        Self {
            id,
            email: None,
            name: None,
            avatar_url: None,
        }
    }

    /// Get a display identifier (email or ID).
    pub fn display_id(&self) -> &str {
        self.email.as_deref().unwrap_or(&self.id)
    }
}

/// Status of an OAuth flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FlowStatus {
    /// Flow is in progress, waiting for user action
    Pending,
    /// Flow completed successfully
    Completed,
    /// Flow failed with an error
    Error,
    /// Flow expired before completion
    Expired,
}

/// Response for OAuth flow status polling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStatusResponse {
    /// Current status of the flow
    pub status: FlowStatus,

    /// Error message if status is Error
    pub error: Option<String>,

    /// Account ID if status is Completed
    pub account_id: Option<String>,
}

impl FlowStatusResponse {
    pub fn pending() -> Self {
        Self {
            status: FlowStatus::Pending,
            error: None,
            account_id: None,
        }
    }

    pub fn completed(account_id: String) -> Self {
        Self {
            status: FlowStatus::Completed,
            error: None,
            account_id: Some(account_id),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            status: FlowStatus::Error,
            error: Some(message),
            account_id: None,
        }
    }

    pub fn expired() -> Self {
        Self {
            status: FlowStatus::Expired,
            error: Some("OAuth flow expired".to_string()),
            account_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bundle_new() {
        let bundle = TokenBundle::new(
            "access_token".to_string(),
            "Bearer".to_string(),
            "claude".to_string(),
            "user@example.com".to_string(),
        );

        assert_eq!(bundle.access_token, "access_token");
        assert_eq!(bundle.token_type, "Bearer");
        assert_eq!(bundle.provider, "claude");
        assert_eq!(bundle.account_id, "user@example.com");
        assert!(bundle.refresh_token.is_none());
        assert!(bundle.expires_at.is_none());
    }

    #[test]
    fn test_token_bundle_is_expired() {
        let mut bundle = TokenBundle::new(
            "access_token".to_string(),
            "Bearer".to_string(),
            "claude".to_string(),
            "user@example.com".to_string(),
        );

        // No expiry set - not expired
        assert!(!bundle.is_expired());

        // Future expiry - not expired
        bundle.expires_at = Some(Utc::now() + Duration::hours(1));
        assert!(!bundle.is_expired());

        // Past expiry - expired
        bundle.expires_at = Some(Utc::now() - Duration::hours(1));
        assert!(bundle.is_expired());
    }

    #[test]
    fn test_token_bundle_expires_within() {
        let mut bundle = TokenBundle::new(
            "access_token".to_string(),
            "Bearer".to_string(),
            "claude".to_string(),
            "user@example.com".to_string(),
        );

        // No expiry - doesn't expire within any duration
        assert!(!bundle.expires_within(Duration::hours(1)));

        // Expires in 30 minutes - within 1 hour
        bundle.expires_at = Some(Utc::now() + Duration::minutes(30));
        assert!(bundle.expires_within(Duration::hours(1)));
        assert!(!bundle.expires_within(Duration::minutes(15)));
    }

    #[test]
    fn test_token_bundle_needs_refresh() {
        let mut bundle = TokenBundle::new(
            "access_token".to_string(),
            "Bearer".to_string(),
            "claude".to_string(),
            "user@example.com".to_string(),
        );

        // No expiry - doesn't need refresh
        assert!(!bundle.needs_refresh());

        // Expires in 2 hours - needs refresh (within 4 hour threshold)
        bundle.expires_at = Some(Utc::now() + Duration::hours(2));
        assert!(bundle.needs_refresh());

        // Expires in 5 hours - doesn't need refresh
        bundle.expires_at = Some(Utc::now() + Duration::hours(5));
        assert!(!bundle.needs_refresh());
    }

    #[test]
    fn test_token_bundle_remaining_seconds() {
        let mut bundle = TokenBundle::new(
            "access_token".to_string(),
            "Bearer".to_string(),
            "claude".to_string(),
            "user@example.com".to_string(),
        );

        // No expiry
        assert!(bundle.remaining_seconds().is_none());

        // Future expiry
        bundle.expires_at = Some(Utc::now() + Duration::seconds(3600));
        let remaining = bundle.remaining_seconds().unwrap();
        assert!(remaining > 3590 && remaining <= 3600);

        // Past expiry - returns 0
        bundle.expires_at = Some(Utc::now() - Duration::hours(1));
        assert_eq!(bundle.remaining_seconds().unwrap(), 0);
    }

    #[test]
    fn test_token_bundle_serialization() {
        let bundle = TokenBundle::new(
            "access_token".to_string(),
            "Bearer".to_string(),
            "claude".to_string(),
            "user@example.com".to_string(),
        );

        let json = serde_json::to_string(&bundle).unwrap();
        let deserialized: TokenBundle = serde_json::from_str(&json).unwrap();

        assert_eq!(bundle.access_token, deserialized.access_token);
        assert_eq!(bundle.provider, deserialized.provider);
        assert_eq!(bundle.account_id, deserialized.account_id);
    }

    #[test]
    fn test_auth_flow_state() {
        let state = AuthFlowState::new("csrf_token".to_string(), "claude".to_string(), 54545);

        assert_eq!(state.state, "csrf_token");
        assert_eq!(state.provider, "claude");
        assert_eq!(state.callback_port, 54545);
        assert!(state.code_verifier.is_none());
        assert!(!state.is_expired());
    }

    #[test]
    fn test_auth_flow_state_with_pkce() {
        let state = AuthFlowState::new("csrf_token".to_string(), "claude".to_string(), 54545)
            .with_pkce("verifier123".to_string());

        assert_eq!(state.code_verifier, Some("verifier123".to_string()));
    }

    #[test]
    fn test_auth_flow_state_validate_state() {
        let state = AuthFlowState::new("csrf_token".to_string(), "claude".to_string(), 54545);

        assert!(state.validate_state("csrf_token"));
        assert!(!state.validate_state("wrong_token"));
    }

    #[test]
    fn test_auth_flow_state_serialization() {
        let state = AuthFlowState::new("csrf_token".to_string(), "claude".to_string(), 54545)
            .with_pkce("verifier".to_string());

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: AuthFlowState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.state, deserialized.state);
        assert_eq!(state.provider, deserialized.provider);
        assert_eq!(state.code_verifier, deserialized.code_verifier);
    }

    #[test]
    fn test_token_validation_valid() {
        let validation = TokenValidation::valid(Some(3600), vec!["read".to_string()]);

        assert!(validation.valid);
        assert_eq!(validation.expires_in_seconds, Some(3600));
        assert_eq!(validation.scopes, vec!["read"]);
        assert!(validation.error.is_none());
    }

    #[test]
    fn test_token_validation_invalid() {
        let validation = TokenValidation::invalid("Token revoked".to_string());

        assert!(!validation.valid);
        assert!(validation.expires_in_seconds.is_none());
        assert!(validation.scopes.is_empty());
        assert_eq!(validation.error, Some("Token revoked".to_string()));
    }

    #[test]
    fn test_account_info() {
        let mut info = AccountInfo::new("user123".to_string());
        assert_eq!(info.display_id(), "user123");

        info.email = Some("user@example.com".to_string());
        assert_eq!(info.display_id(), "user@example.com");
    }

    #[test]
    fn test_flow_status_response() {
        let pending = FlowStatusResponse::pending();
        assert_eq!(pending.status, FlowStatus::Pending);

        let completed = FlowStatusResponse::completed("user@example.com".to_string());
        assert_eq!(completed.status, FlowStatus::Completed);
        assert_eq!(completed.account_id, Some("user@example.com".to_string()));

        let error = FlowStatusResponse::error("Failed".to_string());
        assert_eq!(error.status, FlowStatus::Error);
        assert_eq!(error.error, Some("Failed".to_string()));
    }
}
