/// Error handling for the Terraphim Build Arguments crate
///
/// This module provides comprehensive error handling for build configuration
/// operations, validation, and file I/O.

use thiserror::Error;

/// Result type alias for consistent error handling
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for build argument operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Error {
    /// Configuration parsing or validation errors
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    /// Feature flag validation errors
    #[error("Feature validation error: {0}")]
    FeatureError(String),
    
    /// Build target validation errors
    #[error("Build target error: {0}")]
    TargetError(String),
    
    /// Environment configuration errors
    #[error("Environment error: {0}")]
    EnvironmentError(String),
    
    /// File I/O errors
    #[error("I/O error: {0}")]
    IoError(String),
    
    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Parsing errors for configuration files
    #[error("Parse error: {0}")]
    ParseError(String),
    
    /// Unsupported file format or configuration
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    /// Validation errors for build configurations
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    /// Command generation errors
    #[error("Command generation error: {0}")]
    CommandError(String),
    
    /// Workspace-related errors
    #[error("Workspace error: {0}")]
    WorkspaceError(String),
    
    /// Docker/container build errors
    #[error("Docker error: {0}")]
    DockerError(String),
    
    /// Cross-compilation errors
    #[error("Cross-compilation error: {0}")]
    CrossCompileError(String),
    
    /// Dependency resolution errors
    #[error("Dependency error: {0}")]
    DependencyError(String),
    
    /// Generic error for miscellaneous issues
    #[error("Build argument error: {0}")]
    General(String),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error.to_string())
    }
}

impl Error {
    /// Creates a configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::ConfigError(msg.into())
    }
    
    /// Creates a feature validation error
    pub fn feature<S: Into<String>>(msg: S) -> Self {
        Self::FeatureError(msg.into())
    }
    
    /// Creates a target validation error
    pub fn target<S: Into<String>>(msg: S) -> Self {
        Self::TargetError(msg.into())
    }
    
    /// Creates an environment error
    pub fn environment<S: Into<String>>(msg: S) -> Self {
        Self::EnvironmentError(msg.into())
    }
    
    /// Creates a validation error
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Self::ValidationError(msg.into())
    }
    
    /// Creates a command generation error
    pub fn command<S: Into<String>>(msg: S) -> Self {
        Self::CommandError(msg.into())
    }
    
    /// Creates a workspace error
    pub fn workspace<S: Into<String>>(msg: S) -> Self {
        Self::WorkspaceError(msg.into())
    }
    
    /// Creates a Docker error
    pub fn docker<S: Into<String>>(msg: S) -> Self {
        Self::DockerError(msg.into())
    }
    
    /// Creates a cross-compilation error
    pub fn cross_compile<S: Into<String>>(msg: S) -> Self {
        Self::CrossCompileError(msg.into())
    }
    
    /// Creates a dependency error
    pub fn dependency<S: Into<String>>(msg: S) -> Self {
        Self::DependencyError(msg.into())
    }
    
    /// Creates a general error
    pub fn general<S: Into<String>>(msg: S) -> Self {
        Self::General(msg.into())
    }
    
    /// Returns true if this error is related to validation
    pub fn is_validation_error(&self) -> bool {
        matches!(
            self,
            Self::ConfigError(_)
                | Self::FeatureError(_)
                | Self::TargetError(_)
                | Self::EnvironmentError(_)
                | Self::ValidationError(_)
        )
    }
    
    /// Returns true if this error is related to I/O operations
    pub fn is_io_error(&self) -> bool {
        matches!(
            self,
            Self::IoError(_) | Self::SerializationError(_) | Self::ParseError(_)
        )
    }
    
    /// Returns true if this error is related to build operations
    pub fn is_build_error(&self) -> bool {
        matches!(
            self,
            Self::CommandError(_)
                | Self::WorkspaceError(_)
                | Self::DockerError(_)
                | Self::CrossCompileError(_)
                | Self::DependencyError(_)
        )
    }
    
    /// Returns the error category for logging and debugging
    pub fn category(&self) -> &'static str {
        match self {
            Self::ConfigError(_) => "config",
            Self::FeatureError(_) => "feature",
            Self::TargetError(_) => "target",
            Self::EnvironmentError(_) => "environment",
            Self::IoError(_) => "io",
            Self::SerializationError(_) => "serialization",
            Self::ParseError(_) => "parse",
            Self::UnsupportedFormat(_) => "format",
            Self::ValidationError(_) => "validation",
            Self::CommandError(_) => "command",
            Self::WorkspaceError(_) => "workspace",
            Self::DockerError(_) => "docker",
            Self::CrossCompileError(_) => "cross-compile",
            Self::DependencyError(_) => "dependency",
            Self::General(_) => "general",
        }
    }
    
    /// Returns the inner message
    pub fn message(&self) -> &str {
        match self {
            Self::ConfigError(msg)
            | Self::FeatureError(msg)
            | Self::TargetError(msg)
            | Self::EnvironmentError(msg)
            | Self::IoError(msg)
            | Self::SerializationError(msg)
            | Self::ParseError(msg)
            | Self::UnsupportedFormat(msg)
            | Self::ValidationError(msg)
            | Self::CommandError(msg)
            | Self::WorkspaceError(msg)
            | Self::DockerError(msg)
            | Self::CrossCompileError(msg)
            | Self::DependencyError(msg)
            | Self::General(msg) => msg,
        }
    }
}

/// Error context extension for providing additional context to errors
pub trait ErrorContext<T> {
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
    
    fn context<S: Into<String>>(self, msg: S) -> Result<T>;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: std::fmt::Display,
{
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| Error::General(format!("{}: {}", f(), e)))
    }
    
    fn context<S: Into<String>>(self, msg: S) -> Result<T> {
        let context = msg.into();
        self.map_err(|e| Error::General(format!("{}: {}", context, e)))
    }
}

impl<T> ErrorContext<T> for Option<T> {
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.ok_or_else(|| Error::General(f()))
    }
    
    fn context<S: Into<String>>(self, msg: S) -> Result<T> {
        self.ok_or_else(|| Error::General(msg.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_creation() {
        let error = Error::config("Invalid configuration");
        assert_eq!(error.category(), "config");
        assert_eq!(error.message(), "Invalid configuration");
        assert!(error.is_validation_error());
        assert!(!error.is_io_error());
        assert!(!error.is_build_error());
    }
    
    #[test]
    fn test_error_categorization() {
        let io_error = Error::IoError("File not found".to_string());
        assert!(io_error.is_io_error());
        assert!(!io_error.is_validation_error());
        assert!(!io_error.is_build_error());
        
        let build_error = Error::CommandError("Build failed".to_string());
        assert!(build_error.is_build_error());
        assert!(!build_error.is_validation_error());
        assert!(!build_error.is_io_error());
    }
    
    #[test]
    fn test_error_context() {
        let result: std::result::Result<(), std::io::Error> = 
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));
        
        let error = result.context("Failed to read config file").unwrap_err();
        assert!(error.message().contains("Failed to read config file"));
        assert!(error.message().contains("file not found"));
    }
    
    #[test]
    fn test_option_context() {
        let option: Option<String> = None;
        let error = option.context("Missing required value").unwrap_err();
        assert_eq!(error.message(), "Missing required value");
    }
    
    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let converted: Error = io_error.into();
        
        assert!(converted.is_io_error());
        assert!(converted.message().contains("file not found"));
        assert_eq!(converted.category(), "io");
    }
}
