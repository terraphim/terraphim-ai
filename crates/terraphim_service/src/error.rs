/// Centralized error handling module for terraphim system
///
/// This module provides common error handling patterns and utilities
/// to standardize error types across the terraphim codebase.
use thiserror::Error;

/// Base error trait for all terraphim errors
///
/// This trait provides common functionality for all error types
/// including error categories and user-friendly messages.
pub trait TerraphimError: std::error::Error + Send + Sync + 'static {
    /// Get the error category for logging and metrics
    fn category(&self) -> ErrorCategory;

    /// Get a user-friendly error message
    fn user_message(&self) -> String {
        self.to_string()
    }

    /// Check if the error is recoverable
    fn is_recoverable(&self) -> bool {
        false
    }
}

/// Error categories for classification and handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Network-related errors (timeouts, connection failures)
    Network,
    /// Configuration errors (invalid settings, missing values)
    Configuration,
    /// Authentication and authorization errors
    Auth,
    /// Data validation and parsing errors
    Validation,
    /// Storage and persistence errors
    Storage,
    /// External service integration errors
    Integration,
    /// Internal system errors
    System,
}

/// Common error patterns used across terraphim crates.
#[derive(Error, Debug)]
pub enum CommonError {
    /// A network or HTTP-level failure.
    #[error("Network error: {message}")]
    Network {
        /// Human-readable description of the failure.
        message: String,
        /// Underlying cause, if available.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// A configuration validation or loading failure.
    #[error("Configuration error: {message}")]
    Configuration {
        /// Human-readable description of the problem.
        message: String,
        /// Name of the specific configuration field at fault, if known.
        field: Option<String>,
    },

    /// An input validation failure.
    #[error("Validation error: {message}")]
    Validation {
        /// Human-readable description of what failed validation.
        message: String,
        /// Field name that failed validation, if applicable.
        field: Option<String>,
    },

    /// An authentication or authorisation failure.
    #[error("Authentication error: {message}")]
    Auth {
        /// Human-readable description of the auth failure.
        message: String,
    },

    /// A storage or persistence failure.
    #[error("Storage error: {message}")]
    Storage {
        /// Human-readable description of the failure.
        message: String,
        /// Underlying cause, if available.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An error from an external service integration.
    #[error("Integration error with {service}: {message}")]
    Integration {
        /// Name of the external service (e.g. `"Confluence"`, `"OpenRouter"`).
        service: String,
        /// Human-readable description of the failure.
        message: String,
        /// Underlying cause, if available.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// An unexpected internal system error.
    #[error("System error: {message}")]
    System {
        /// Human-readable description of the failure.
        message: String,
        /// Underlying cause, if available.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl TerraphimError for CommonError {
    fn category(&self) -> ErrorCategory {
        match self {
            CommonError::Network { .. } => ErrorCategory::Network,
            CommonError::Configuration { .. } => ErrorCategory::Configuration,
            CommonError::Validation { .. } => ErrorCategory::Validation,
            CommonError::Auth { .. } => ErrorCategory::Auth,
            CommonError::Storage { .. } => ErrorCategory::Storage,
            CommonError::Integration { .. } => ErrorCategory::Integration,
            CommonError::System { .. } => ErrorCategory::System,
        }
    }

    fn is_recoverable(&self) -> bool {
        matches!(
            self,
            CommonError::Network { .. } | CommonError::Integration { .. }
        )
    }
}

/// Convenience constructors for common error variants.
impl CommonError {
    /// Create a network error without an underlying source.
    pub fn network(message: impl Into<String>) -> Self {
        CommonError::Network {
            message: message.into(),
            source: None,
        }
    }

    /// Create a network error wrapping an existing error as the source.
    pub fn network_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        CommonError::Network {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a configuration error without a specific field.
    pub fn config(message: impl Into<String>) -> Self {
        CommonError::Configuration {
            message: message.into(),
            field: None,
        }
    }

    /// Create a configuration error pointing at a named field.
    pub fn config_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        CommonError::Configuration {
            message: message.into(),
            field: Some(field.into()),
        }
    }

    /// Create a validation error without a specific field.
    pub fn validation(message: impl Into<String>) -> Self {
        CommonError::Validation {
            message: message.into(),
            field: None,
        }
    }

    /// Create a validation error pointing at a named field.
    pub fn validation_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        CommonError::Validation {
            message: message.into(),
            field: Some(field.into()),
        }
    }

    /// Create an authentication error.
    pub fn auth(message: impl Into<String>) -> Self {
        CommonError::Auth {
            message: message.into(),
        }
    }

    /// Create a storage error without an underlying source.
    pub fn storage(message: impl Into<String>) -> Self {
        CommonError::Storage {
            message: message.into(),
            source: None,
        }
    }

    /// Create a storage error wrapping an existing error as the source.
    pub fn storage_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        CommonError::Storage {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an integration error without an underlying source.
    pub fn integration(service: impl Into<String>, message: impl Into<String>) -> Self {
        CommonError::Integration {
            service: service.into(),
            message: message.into(),
            source: None,
        }
    }

    /// Create an integration error wrapping an existing error as the source.
    pub fn integration_with_source(
        service: impl Into<String>,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        CommonError::Integration {
            service: service.into(),
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a system error without an underlying source.
    pub fn system(message: impl Into<String>) -> Self {
        CommonError::System {
            message: message.into(),
            source: None,
        }
    }

    /// Create a system error wrapping an existing error as the source.
    pub fn system_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        CommonError::System {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

/// Utility functions for error handling patterns
pub mod utils {
    use super::*;

    /// Convert any error into a network error
    pub fn as_network_error<E: std::error::Error + Send + Sync + 'static>(
        err: E,
        context: &str,
    ) -> CommonError {
        CommonError::network_with_source(format!("{}: {}", context, err), err)
    }

    /// Convert any error into a storage error
    pub fn as_storage_error<E: std::error::Error + Send + Sync + 'static>(
        err: E,
        context: &str,
    ) -> CommonError {
        CommonError::storage_with_source(format!("{}: {}", context, err), err)
    }

    /// Convert any error into an integration error
    pub fn as_integration_error<E: std::error::Error + Send + Sync + 'static>(
        err: E,
        service: &str,
        context: &str,
    ) -> CommonError {
        CommonError::integration_with_source(service, format!("{}: {}", context, err), err)
    }

    /// Convert any error into a system error
    pub fn as_system_error<E: std::error::Error + Send + Sync + 'static>(
        err: E,
        context: &str,
    ) -> CommonError {
        CommonError::system_with_source(format!("{}: {}", context, err), err)
    }
}

/// Result type alias using CommonError
pub type TerraphimResult<T> = Result<T, CommonError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        let network_err = CommonError::network("connection failed");
        assert_eq!(network_err.category(), ErrorCategory::Network);
        assert!(network_err.is_recoverable());

        let config_err = CommonError::config("invalid setting");
        assert_eq!(config_err.category(), ErrorCategory::Configuration);
        assert!(!config_err.is_recoverable());
    }

    #[test]
    fn test_error_construction() {
        let err = CommonError::config_field("missing required field", "api_key");
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("missing required field"));
    }

    #[test]
    fn test_error_utils() {
        use std::io::{Error as IoError, ErrorKind};

        let io_err = IoError::new(ErrorKind::NotFound, "file not found");
        let storage_err = utils::as_storage_error(io_err, "loading thesaurus");

        assert_eq!(storage_err.category(), ErrorCategory::Storage);
        assert!(storage_err.to_string().contains("loading thesaurus"));
    }
}
