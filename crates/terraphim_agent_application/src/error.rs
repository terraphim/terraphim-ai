//! Error types for the agent application

use thiserror::Error;

/// Errors that can occur in the agent application
#[derive(Error, Debug, Clone)]
pub enum ApplicationError {
    /// Application startup failed
    #[error("Application startup failed: {0}")]
    StartupFailed(String),

    /// Application shutdown failed
    #[error("Application shutdown failed: {0}")]
    ShutdownFailed(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Hot reload failed
    #[error("Hot reload failed: {0}")]
    HotReloadFailed(String),

    /// Supervision tree error
    #[error("Supervision tree error: {0}")]
    SupervisionError(String),

    /// Deployment error
    #[error("Deployment error: {0}")]
    DeploymentError(String),

    /// Health check failed
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    /// System diagnostics error
    #[error("System diagnostics error: {0}")]
    DiagnosticsError(String),

    /// Agent lifecycle error
    #[error("Agent lifecycle error: {0}")]
    AgentLifecycleError(String),

    /// Resource management error
    #[error("Resource management error: {0}")]
    ResourceError(String),

    /// System error
    #[error("System error: {0}")]
    SystemError(String),
}

impl ApplicationError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            ApplicationError::StartupFailed(_) => false,
            ApplicationError::ShutdownFailed(_) => false,
            ApplicationError::ConfigurationError(_) => true,
            ApplicationError::HotReloadFailed(_) => true,
            ApplicationError::SupervisionError(_) => true,
            ApplicationError::DeploymentError(_) => true,
            ApplicationError::HealthCheckFailed(_) => true,
            ApplicationError::DiagnosticsError(_) => true,
            ApplicationError::AgentLifecycleError(_) => true,
            ApplicationError::ResourceError(_) => true,
            ApplicationError::SystemError(_) => false,
        }
    }

    /// Get error category for monitoring
    pub fn category(&self) -> &'static str {
        match self {
            ApplicationError::StartupFailed(_) => "startup",
            ApplicationError::ShutdownFailed(_) => "shutdown",
            ApplicationError::ConfigurationError(_) => "configuration",
            ApplicationError::HotReloadFailed(_) => "hot_reload",
            ApplicationError::SupervisionError(_) => "supervision",
            ApplicationError::DeploymentError(_) => "deployment",
            ApplicationError::HealthCheckFailed(_) => "health_check",
            ApplicationError::DiagnosticsError(_) => "diagnostics",
            ApplicationError::AgentLifecycleError(_) => "agent_lifecycle",
            ApplicationError::ResourceError(_) => "resource",
            ApplicationError::SystemError(_) => "system",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        assert!(!ApplicationError::StartupFailed("test".to_string()).is_recoverable());
        assert!(ApplicationError::ConfigurationError("test".to_string()).is_recoverable());
        assert!(!ApplicationError::SystemError("test".to_string()).is_recoverable());
    }

    #[test]
    fn test_error_categorization() {
        assert_eq!(
            ApplicationError::StartupFailed("test".to_string()).category(),
            "startup"
        );
        assert_eq!(
            ApplicationError::HotReloadFailed("test".to_string()).category(),
            "hot_reload"
        );
    }
}
