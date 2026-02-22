//! Error types for the LLM proxy
//!
//! Comprehensive error handling following the error handling architecture document.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use std::time::Duration;
use thiserror::Error;
use tracing::{error, warn};

/// Root error type for the LLM proxy
#[derive(Error, Debug, Clone)]
pub enum ProxyError {
    // Authentication Errors (400-403 status codes)
    #[error("Missing API key")]
    MissingApiKey,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("API key expired")]
    ApiKeyExpired,

    #[error("Too many failed authentication attempts. Retry after {retry_after:?}")]
    TooManyFailedAttempts { retry_after: Duration },

    // Authorization Errors (403 status codes)
    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),

    #[error("Session hijacking attempt detected")]
    SessionHijackingAttempt,

    // Rate Limiting Errors (429 status codes)
    #[error("Rate limit exceeded: {limit_type}. Retry after {retry_after:?}")]
    RateLimitExceeded {
        limit_type: String,
        retry_after: Duration,
    },

    #[error("Too many concurrent requests. Maximum: {max}")]
    TooManyConcurrentRequests { max: usize },

    // Validation Errors (400 status codes)
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Invalid model name: {0}")]
    InvalidModel(String),

    #[error("Invalid content: {0}")]
    InvalidContent(String),

    #[error("Invalid max_tokens value: {0}")]
    InvalidMaxTokens(u64),

    #[error("Request too large: {size} bytes (max: {max} bytes)")]
    RequestTooLarge { size: usize, max: usize },

    #[error("Too many messages: {count} (max: {max})")]
    TooManyMessages { count: usize, max: usize },

    // Token Counting Errors (500 status codes)
    #[error("Token count overflow")]
    TokenCountOverflow,

    #[error("Token count too large: {0}")]
    TokenCountTooLarge(usize),

    #[error("Token counting failed: {0}")]
    TokenCountingError(String),

    // Routing Errors (500 status codes)
    #[error("No suitable provider found for request")]
    NoProviderFound,

    #[error("Routing policy violation: {0}")]
    RoutingPolicyViolation(String),

    #[error("Invalid routing decision: {0}")]
    InvalidRoutingDecision(String),

    // Security Errors (403 status codes)
    #[error("SSRF attempt detected: {0}")]
    SsrfAttempt(String),

    #[error("DNS rebinding attack detected")]
    DnsRebindingAttack,

    #[error("Invalid provider URL: {0}")]
    InvalidProviderUrl(String),

    // Provider Errors (502, 503, 504 status codes)
    #[error("Provider error: {provider} - {message}")]
    ProviderError { provider: String, message: String },

    #[error("Provider timeout: {provider} - {elapsed:?}")]
    ProviderTimeout { provider: String, elapsed: Duration },

    #[error("Provider unavailable: {provider}")]
    ProviderUnavailable { provider: String },

    #[error("Invalid provider response: {0}")]
    InvalidProviderResponse(String),

    #[error("Response too large: {size} bytes (max: {max} bytes)")]
    ResponseTooLarge { size: usize, max: usize },

    // Configuration Errors (500 status codes on startup)
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid configuration file: {0}")]
    InvalidConfig(String),

    #[error("Configuration file permissions too permissive")]
    InsecureConfigPermissions,

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Performance test error: {0}")]
    TestError(String),

    // Session Errors (400, 401 status codes)
    #[error("Invalid session ID")]
    InvalidSession,

    #[error("Session expired")]
    SessionExpired,

    // Transformer Errors (500 status codes)
    #[error("Transformer error: {transformer} - {message}")]
    TransformerError {
        transformer: String,
        message: String,
    },

    #[error("Transformer chain failed: {0}")]
    TransformerChainError(String),

    // I/O Errors (500 status codes)
    #[error("I/O error: {0}")]
    Io(String),

    #[error("DNS resolution failed: {0}")]
    DnsResolutionFailed(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    // Serialization Errors (400, 500 status codes)
    #[error("JSON serialization error: {0}")]
    JsonSerialization(String),

    #[error("TOML parsing error: {0}")]
    TomlParsing(String),

    // Internal Errors (500 status codes)
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl ProxyError {
    /// Convert error to HTTP status code
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 400 Bad Request
            Self::InvalidRequest(_)
            | Self::InvalidModel(_)
            | Self::InvalidContent(_)
            | Self::InvalidMaxTokens(_)
            | Self::TooManyMessages { .. }
            | Self::InvalidSession
            | Self::JsonSerialization(_)
            | Self::InvalidProviderUrl(_) => StatusCode::BAD_REQUEST,

            // 401 Unauthorized
            Self::MissingApiKey
            | Self::InvalidApiKey
            | Self::ApiKeyExpired
            | Self::SessionExpired => StatusCode::UNAUTHORIZED,

            // 403 Forbidden
            Self::InsufficientPermissions(_)
            | Self::SessionHijackingAttempt
            | Self::SsrfAttempt(_)
            | Self::DnsRebindingAttack => StatusCode::FORBIDDEN,

            // 404 Not Found
            Self::NotFound(_) => StatusCode::NOT_FOUND,

            // 413 Payload Too Large
            Self::RequestTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,

            // 429 Too Many Requests
            Self::RateLimitExceeded { .. }
            | Self::TooManyFailedAttempts { .. }
            | Self::TooManyConcurrentRequests { .. } => StatusCode::TOO_MANY_REQUESTS,

            // 500 Internal Server Error
            Self::TokenCountOverflow
            | Self::TokenCountTooLarge(_)
            | Self::TokenCountingError(_)
            | Self::NoProviderFound
            | Self::RoutingPolicyViolation(_)
            | Self::InvalidRoutingDecision(_)
            | Self::ConfigError(_)
            | Self::InvalidConfig(_)
            | Self::InsecureConfigPermissions
            | Self::ConfigurationError(_)
            | Self::TestError(_)
            | Self::TransformerError { .. }
            | Self::TransformerChainError(_)
            | Self::Io(_)
            | Self::TomlParsing(_)
            | Self::Internal(_)
            | Self::NotImplemented(_) => StatusCode::INTERNAL_SERVER_ERROR,

            // 502 Bad Gateway
            Self::ProviderError { .. }
            | Self::InvalidProviderResponse(_)
            | Self::ResponseTooLarge { .. }
            | Self::DnsResolutionFailed(_)
            | Self::NetworkError(_) => StatusCode::BAD_GATEWAY,

            // 503 Service Unavailable
            Self::ProviderUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,

            // 504 Gateway Timeout
            Self::ProviderTimeout { .. } => StatusCode::GATEWAY_TIMEOUT,
        }
    }

    /// Convert error to client-safe message (no sensitive info)
    pub fn client_message(&self) -> String {
        match self {
            // Return original message for user-facing errors
            Self::MissingApiKey
            | Self::InvalidApiKey
            | Self::ApiKeyExpired
            | Self::InvalidRequest(_)
            | Self::InvalidModel(_)
            | Self::InvalidContent(_)
            | Self::InvalidMaxTokens(_)
            | Self::RequestTooLarge { .. }
            | Self::TooManyMessages { .. }
            | Self::RateLimitExceeded { .. }
            | Self::TooManyConcurrentRequests { .. }
            | Self::InvalidSession
            | Self::SessionExpired
            | Self::NotFound(_) => self.to_string(),

            // Generic messages for security-sensitive errors
            Self::SessionHijackingAttempt | Self::SsrfAttempt(_) | Self::DnsRebindingAttack => {
                "Forbidden".to_string()
            }

            Self::TooManyFailedAttempts { retry_after } => {
                format!(
                    "Too many failed attempts. Retry after {} seconds",
                    retry_after.as_secs()
                )
            }

            // Generic messages for provider errors
            Self::ProviderError { provider, .. } => {
                format!("Provider '{}' returned an error", provider)
            }
            Self::ProviderTimeout { provider, .. } => {
                format!("Provider '{}' timed out", provider)
            }
            Self::ProviderUnavailable { provider } => {
                format!("Provider '{}' is unavailable", provider)
            }

            // Generic messages for internal errors (hide details)
            _ => "Internal server error".to_string(),
        }
    }

    /// Whether this error should be logged as a warning vs error
    pub fn log_level(&self) -> tracing::Level {
        use tracing::Level;

        match self {
            // User errors (log as warnings)
            Self::MissingApiKey
            | Self::InvalidApiKey
            | Self::InvalidRequest(_)
            | Self::InvalidModel(_)
            | Self::InvalidContent(_)
            | Self::InvalidMaxTokens(_)
            | Self::RequestTooLarge { .. }
            | Self::TooManyMessages { .. }
            | Self::RateLimitExceeded { .. }
            | Self::TooManyConcurrentRequests { .. }
            | Self::InvalidSession
            | Self::SessionExpired
            | Self::ProviderError { .. }
            | Self::ProviderTimeout { .. }
            | Self::ProviderUnavailable { .. }
            | Self::InvalidProviderResponse(_)
            | Self::ResponseTooLarge { .. } => Level::WARN,

            // Security incidents (log as errors)
            Self::TooManyFailedAttempts { .. }
            | Self::SessionHijackingAttempt
            | Self::SsrfAttempt(_)
            | Self::DnsRebindingAttack => Level::ERROR,

            // System errors (log as errors)
            _ => Level::ERROR,
        }
    }
}

/// Error response JSON structure matching Anthropic API format
#[derive(Serialize)]
pub struct ErrorResponse {
    /// Top-level type field required by Anthropic API - always "error"
    #[serde(rename = "type")]
    pub response_type: &'static str,
    pub error: ErrorDetail,
}

#[derive(Serialize)]
pub struct ErrorDetail {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    pub code: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();

        // Extract retry_after for rate limit errors
        let retry_after = match &self {
            ProxyError::RateLimitExceeded { retry_after, .. } => Some(retry_after.as_secs()),
            ProxyError::TooManyFailedAttempts { retry_after } => Some(retry_after.as_secs()),
            _ => None,
        };

        let error_response = ErrorResponse {
            response_type: "error",
            error: ErrorDetail {
                error_type: error_type_string(&self),
                message: self.client_message(),
                code: status_code.as_u16(),
                retry_after,
            },
        };

        // Log error
        log_error(&self);

        // Build response
        let body = serde_json::to_string(&error_response).unwrap_or_else(|_| {
            r#"{"type":"error","error":{"type":"internal_error","message":"Internal server error","code":500}}"#
                .to_string()
        });

        let mut response = Response::new(body.into());
        *response.status_mut() = status_code;
        response
            .headers_mut()
            .insert("content-type", "application/json".parse().unwrap());

        // Add Retry-After header for rate limits
        if let Some(retry_secs) = retry_after {
            response
                .headers_mut()
                .insert("retry-after", retry_secs.to_string().parse().unwrap());
        }

        response
    }
}

fn error_type_string(error: &ProxyError) -> String {
    match error {
        ProxyError::MissingApiKey | ProxyError::InvalidApiKey => "invalid_api_key",
        ProxyError::RateLimitExceeded { .. } => "rate_limit_exceeded",
        ProxyError::InvalidRequest(_) => "invalid_request",
        ProxyError::ProviderError { .. } => "provider_error",
        ProxyError::ProviderTimeout { .. } => "provider_timeout",
        _ => "internal_error",
    }
    .to_string()
}

fn log_error(error: &ProxyError) {
    match error.log_level() {
        tracing::Level::ERROR => error!(?error, "Request failed"),
        tracing::Level::WARN => warn!(?error, "Request warning"),
        _ => {}
    }
}

// Implement From traits for common error types
impl From<std::io::Error> for ProxyError {
    fn from(err: std::io::Error) -> Self {
        ProxyError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for ProxyError {
    fn from(err: serde_json::Error) -> Self {
        ProxyError::JsonSerialization(err.to_string())
    }
}

impl From<toml::de::Error> for ProxyError {
    fn from(err: toml::de::Error) -> Self {
        ProxyError::TomlParsing(err.to_string())
    }
}

/// Result type for proxy operations
pub type Result<T> = std::result::Result<T, ProxyError>;
