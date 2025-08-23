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

/// Common error patterns used across terraphim crates
#[derive(Error, Debug)]
pub enum CommonError {
    #[error("Network error: {message}")]
    Network { 
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    #[error("Configuration error: {message}")]
    Configuration { 
        message: String,
        field: Option<String>,
    },
    
    #[error("Validation error: {message}")]
    Validation { 
        message: String,
        field: Option<String>,
    },
    
    #[error("Authentication error: {message}")]
    Auth { 
        message: String,
    },
    
    #[error("Storage error: {message}")]
    Storage { 
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    #[error("Integration error with {service}: {message}")]
    Integration { 
        service: String,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    #[error("System error: {message}")]
    System { 
        message: String,
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
        matches!(self, 
            CommonError::Network { .. } | 
            CommonError::Integration { .. }
        )
    }
}

/// Helper functions for creating common error types
impl CommonError {
    pub fn network(message: impl Into<String>) -> Self {
        CommonError::Network {
            message: message.into(),
            source: None,
        }
    }
    
    pub fn network_with_source(
        message: impl Into<String>, 
        source: impl std::error::Error + Send + Sync + 'static
    ) -> Self {
        CommonError::Network {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
    
    pub fn config(message: impl Into<String>) -> Self {
        CommonError::Configuration {
            message: message.into(),
            field: None,
        }
    }
    
    pub fn config_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        CommonError::Configuration {
            message: message.into(),
            field: Some(field.into()),
        }
    }
    
    pub fn validation(message: impl Into<String>) -> Self {
        CommonError::Validation {
            message: message.into(),
            field: None,
        }
    }
    
    pub fn validation_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        CommonError::Validation {
            message: message.into(),
            field: Some(field.into()),
        }
    }
    
    pub fn auth(message: impl Into<String>) -> Self {
        CommonError::Auth {
            message: message.into(),
        }
    }
    
    pub fn storage(message: impl Into<String>) -> Self {
        CommonError::Storage {
            message: message.into(),
            source: None,
        }
    }
    
    pub fn storage_with_source(
        message: impl Into<String>, 
        source: impl std::error::Error + Send + Sync + 'static
    ) -> Self {
        CommonError::Storage {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
    
    pub fn integration(service: impl Into<String>, message: impl Into<String>) -> Self {
        CommonError::Integration {
            service: service.into(),
            message: message.into(),
            source: None,
        }
    }
    
    pub fn integration_with_source(
        service: impl Into<String>,
        message: impl Into<String>, 
        source: impl std::error::Error + Send + Sync + 'static
    ) -> Self {
        CommonError::Integration {
            service: service.into(),
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
    
    pub fn system(message: impl Into<String>) -> Self {
        CommonError::System {
            message: message.into(),
            source: None,
        }
    }
    
    pub fn system_with_source(
        message: impl Into<String>, 
        source: impl std::error::Error + Send + Sync + 'static
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
        CommonError::network_with_source(
            format!("{}: {}", context, err),
            err
        )
    }
    
    /// Convert any error into a storage error  
    pub fn as_storage_error<E: std::error::Error + Send + Sync + 'static>(
        err: E,
        context: &str,
    ) -> CommonError {
        CommonError::storage_with_source(
            format!("{}: {}", context, err),
            err
        )
    }
    
    /// Convert any error into an integration error
    pub fn as_integration_error<E: std::error::Error + Send + Sync + 'static>(
        err: E,
        service: &str,
        context: &str,
    ) -> CommonError {
        CommonError::integration_with_source(
            service,
            format!("{}: {}", context, err),
            err
        )
    }
    
    /// Convert any error into a system error
    pub fn as_system_error<E: std::error::Error + Send + Sync + 'static>(
        err: E,
        context: &str,
    ) -> CommonError {
        CommonError::system_with_source(
            format!("{}: {}", context, err),
            err
        )
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