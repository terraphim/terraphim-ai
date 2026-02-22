//! Management API error types.
//!
//! Provides structured error handling for management API operations
//! with proper HTTP status code mapping.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// Errors that can occur during management API operations.
#[derive(Debug, Error)]
pub enum ManagementError {
    /// Authentication is required but not provided
    #[error("Authentication required")]
    Unauthorized,

    /// Provided credentials are invalid
    #[error("Invalid management secret")]
    InvalidSecret,

    /// Configuration validation failed
    #[error("Configuration validation failed: {0}")]
    ValidationError(String),

    /// Operation is not allowed
    #[error("Operation not allowed: {0}")]
    NotAllowed(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Conflict with existing resource
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimited,

    /// Invalid request format
    #[error("Invalid request: {0}")]
    BadRequest(String),
}

/// Result type for management operations.
pub type ManagementResult<T> = Result<T, ManagementError>;

/// Error response body for JSON responses.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl ManagementError {
    /// Get the error code string for this error type.
    pub fn code(&self) -> &'static str {
        match self {
            ManagementError::Unauthorized => "UNAUTHORIZED",
            ManagementError::InvalidSecret => "INVALID_SECRET",
            ManagementError::ValidationError(_) => "VALIDATION_ERROR",
            ManagementError::NotAllowed(_) => "NOT_ALLOWED",
            ManagementError::NotFound(_) => "NOT_FOUND",
            ManagementError::Conflict(_) => "CONFLICT",
            ManagementError::Internal(_) => "INTERNAL_ERROR",
            ManagementError::RateLimited => "RATE_LIMITED",
            ManagementError::BadRequest(_) => "BAD_REQUEST",
        }
    }

    /// Get the HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            ManagementError::Unauthorized => StatusCode::UNAUTHORIZED,
            ManagementError::InvalidSecret => StatusCode::FORBIDDEN,
            ManagementError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ManagementError::NotAllowed(_) => StatusCode::FORBIDDEN,
            ManagementError::NotFound(_) => StatusCode::NOT_FOUND,
            ManagementError::Conflict(_) => StatusCode::CONFLICT,
            ManagementError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ManagementError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ManagementError::BadRequest(_) => StatusCode::BAD_REQUEST,
        }
    }

    /// Get additional details if available.
    fn details(&self) -> Option<String> {
        match self {
            ManagementError::ValidationError(msg) => Some(msg.clone()),
            ManagementError::NotAllowed(msg) => Some(msg.clone()),
            ManagementError::NotFound(msg) => Some(msg.clone()),
            ManagementError::Conflict(msg) => Some(msg.clone()),
            ManagementError::Internal(msg) => Some(msg.clone()),
            ManagementError::BadRequest(msg) => Some(msg.clone()),
            _ => None,
        }
    }
}

impl IntoResponse for ManagementError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorResponse {
            error: self.to_string(),
            code: self.code(),
            details: self.details(),
        };

        (status, Json(body)).into_response()
    }
}

impl From<serde_json::Error> for ManagementError {
    fn from(err: serde_json::Error) -> Self {
        ManagementError::BadRequest(format!("Invalid JSON: {}", err))
    }
}

impl From<std::io::Error> for ManagementError {
    fn from(err: std::io::Error) -> Self {
        ManagementError::Internal(format!("IO error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(ManagementError::Unauthorized.code(), "UNAUTHORIZED");
        assert_eq!(ManagementError::InvalidSecret.code(), "INVALID_SECRET");
        assert_eq!(
            ManagementError::ValidationError("test".into()).code(),
            "VALIDATION_ERROR"
        );
        assert_eq!(
            ManagementError::NotAllowed("test".into()).code(),
            "NOT_ALLOWED"
        );
        assert_eq!(ManagementError::NotFound("test".into()).code(), "NOT_FOUND");
        assert_eq!(ManagementError::Conflict("test".into()).code(), "CONFLICT");
        assert_eq!(
            ManagementError::Internal("test".into()).code(),
            "INTERNAL_ERROR"
        );
        assert_eq!(ManagementError::RateLimited.code(), "RATE_LIMITED");
        assert_eq!(
            ManagementError::BadRequest("test".into()).code(),
            "BAD_REQUEST"
        );
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(
            ManagementError::Unauthorized.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ManagementError::InvalidSecret.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ManagementError::ValidationError("test".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ManagementError::NotAllowed("test".into()).status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ManagementError::NotFound("test".into()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ManagementError::Conflict("test".into()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            ManagementError::Internal("test".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ManagementError::RateLimited.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            ManagementError::BadRequest("test".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            ManagementError::Unauthorized.to_string(),
            "Authentication required"
        );
        assert_eq!(
            ManagementError::InvalidSecret.to_string(),
            "Invalid management secret"
        );
        assert_eq!(
            ManagementError::ValidationError("invalid field".into()).to_string(),
            "Configuration validation failed: invalid field"
        );
    }

    #[test]
    fn test_error_details() {
        assert!(ManagementError::Unauthorized.details().is_none());
        assert!(ManagementError::InvalidSecret.details().is_none());
        assert_eq!(
            ManagementError::ValidationError("details here".into()).details(),
            Some("details here".to_string())
        );
        assert_eq!(
            ManagementError::NotFound("resource".into()).details(),
            Some("resource".to_string())
        );
    }

    #[test]
    fn test_from_serde_error() {
        let json_err = serde_json::from_str::<String>("invalid").unwrap_err();
        let mgmt_err = ManagementError::from(json_err);
        match mgmt_err {
            ManagementError::BadRequest(msg) => assert!(msg.contains("Invalid JSON")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let mgmt_err = ManagementError::from(io_err);
        match mgmt_err {
            ManagementError::Internal(msg) => assert!(msg.contains("IO error")),
            _ => panic!("Expected Internal"),
        }
    }
}
