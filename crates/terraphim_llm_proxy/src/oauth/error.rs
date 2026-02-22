//! OAuth error types for authentication and token management.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;

/// Errors that can occur during OAuth operations.
#[derive(Debug, Error)]
pub enum OAuthError {
    /// OAuth flow failed during authorization
    #[error("OAuth flow failed: {0}")]
    FlowFailed(String),

    /// Token refresh failed
    #[error("Token refresh failed: {0}")]
    RefreshFailed(String),

    /// Token validation failed
    #[error("Token validation failed: {0}")]
    ValidationFailed(String),

    /// Token storage error (file or Redis)
    #[error("Token storage error: {0}")]
    StorageError(String),

    /// Invalid or expired state token (CSRF)
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Provider is not configured
    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    /// Provider is not enabled
    #[error("Provider not enabled: {0}")]
    ProviderNotEnabled(String),

    /// Token not found for provider/account
    #[error("Token not found for {provider}/{account_id}")]
    TokenNotFound {
        provider: String,
        account_id: String,
    },

    /// Token is expired and cannot be refreshed
    #[error("Token expired and no refresh token available")]
    TokenExpiredNoRefresh,

    /// HTTP error during OAuth flow
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON parsing error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// IO error (file operations)
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// PKCE code verifier generation failed
    #[error("PKCE generation failed: {0}")]
    PkceError(String),

    /// Callback server error
    #[error("Callback server error: {0}")]
    CallbackServerError(String),

    /// Timeout waiting for OAuth callback
    #[error("OAuth flow timed out")]
    Timeout,

    /// Rate limited by provider
    #[error("Rate limited by {provider}, retry after {retry_after_seconds}s")]
    RateLimited {
        provider: String,
        retry_after_seconds: u64,
    },

    /// File lock acquisition failed (timeout or compromised)
    #[error("Lock error: {0}")]
    LockError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl OAuthError {
    /// Get the HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            OAuthError::FlowFailed(_) => StatusCode::BAD_REQUEST,
            OAuthError::RefreshFailed(_) => StatusCode::UNAUTHORIZED,
            OAuthError::ValidationFailed(_) => StatusCode::UNAUTHORIZED,
            OAuthError::StorageError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            OAuthError::InvalidState(_) => StatusCode::BAD_REQUEST,
            OAuthError::ProviderNotConfigured(_) => StatusCode::NOT_FOUND,
            OAuthError::ProviderNotEnabled(_) => StatusCode::NOT_FOUND,
            OAuthError::TokenNotFound { .. } => StatusCode::NOT_FOUND,
            OAuthError::TokenExpiredNoRefresh => StatusCode::UNAUTHORIZED,
            OAuthError::HttpError(_) => StatusCode::BAD_GATEWAY,
            OAuthError::JsonError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            OAuthError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            OAuthError::PkceError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            OAuthError::CallbackServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            OAuthError::Timeout => StatusCode::GATEWAY_TIMEOUT,
            OAuthError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            OAuthError::LockError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            OAuthError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            OAuthError::HttpError(_)
                | OAuthError::Timeout
                | OAuthError::RateLimited { .. }
                | OAuthError::LockError(_)
                | OAuthError::CallbackServerError(_)
        )
    }
}

/// Error response body for OAuth endpoints.
#[derive(Debug, Serialize)]
pub struct OAuthErrorResponse {
    pub error: String,
    pub error_description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let retry_after = if let OAuthError::RateLimited {
            retry_after_seconds,
            ..
        } = &self
        {
            Some(*retry_after_seconds)
        } else {
            None
        };

        let body = OAuthErrorResponse {
            error: self.error_code().to_string(),
            error_description: self.to_string(),
            retry_after,
        };

        let mut response = axum::Json(body).into_response();
        *response.status_mut() = status;

        if let Some(retry) = retry_after {
            response
                .headers_mut()
                .insert("Retry-After", retry.to_string().parse().unwrap());
        }

        response
    }
}

impl OAuthError {
    /// Get OAuth2 error code for the error.
    fn error_code(&self) -> &'static str {
        match self {
            OAuthError::FlowFailed(_) => "invalid_request",
            OAuthError::RefreshFailed(_) => "invalid_grant",
            OAuthError::ValidationFailed(_) => "invalid_token",
            OAuthError::StorageError(_) => "server_error",
            OAuthError::InvalidState(_) => "invalid_request",
            OAuthError::ProviderNotConfigured(_) => "invalid_request",
            OAuthError::ProviderNotEnabled(_) => "invalid_request",
            OAuthError::TokenNotFound { .. } => "invalid_token",
            OAuthError::TokenExpiredNoRefresh => "invalid_grant",
            OAuthError::HttpError(_) => "temporarily_unavailable",
            OAuthError::JsonError(_) => "server_error",
            OAuthError::IoError(_) => "server_error",
            OAuthError::PkceError(_) => "server_error",
            OAuthError::CallbackServerError(_) => "server_error",
            OAuthError::Timeout => "temporarily_unavailable",
            OAuthError::RateLimited { .. } => "temporarily_unavailable",
            OAuthError::LockError(_) => "server_error",
            OAuthError::Internal(_) => "server_error",
        }
    }
}

/// Result type for OAuth operations.
pub type OAuthResult<T> = std::result::Result<T, OAuthError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_error_status_codes() {
        assert_eq!(
            OAuthError::FlowFailed("test".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            OAuthError::RefreshFailed("test".into()).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            OAuthError::TokenNotFound {
                provider: "claude".into(),
                account_id: "user".into()
            }
            .status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            OAuthError::RateLimited {
                provider: "claude".into(),
                retry_after_seconds: 60
            }
            .status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_oauth_error_retryable() {
        assert!(OAuthError::Timeout.is_retryable());
        assert!(OAuthError::RateLimited {
            provider: "claude".into(),
            retry_after_seconds: 60
        }
        .is_retryable());
        assert!(OAuthError::LockError("timeout".into()).is_retryable());
        assert!(!OAuthError::FlowFailed("test".into()).is_retryable());
        assert!(!OAuthError::InvalidState("test".into()).is_retryable());
    }

    #[test]
    fn test_lock_error_status_code() {
        assert_eq!(
            OAuthError::LockError("timeout".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_oauth_error_display() {
        let err = OAuthError::TokenNotFound {
            provider: "claude".to_string(),
            account_id: "user@example.com".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Token not found for claude/user@example.com"
        );
    }

    #[test]
    fn test_oauth_error_response_serialization() {
        let response = OAuthErrorResponse {
            error: "invalid_token".to_string(),
            error_description: "Token expired".to_string(),
            retry_after: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("invalid_token"));
        assert!(json.contains("Token expired"));
        assert!(!json.contains("retry_after"));
    }

    #[test]
    fn test_oauth_error_response_with_retry() {
        let response = OAuthErrorResponse {
            error: "temporarily_unavailable".to_string(),
            error_description: "Rate limited".to_string(),
            retry_after: Some(60),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("retry_after"));
        assert!(json.contains("60"));
    }
}
