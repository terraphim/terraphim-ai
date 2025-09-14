//! Error types for the supervision system

use crate::{AgentPid, SupervisorId};
use thiserror::Error;

/// Errors that can occur in the supervision system
#[derive(Error, Debug)]
pub enum SupervisionError {
    #[error("Agent {0} not found")]
    AgentNotFound(AgentPid),

    #[error("Supervisor {0} not found")]
    SupervisorNotFound(SupervisorId),

    #[error("Agent {0} failed to start: {1}")]
    AgentStartFailed(AgentPid, String),

    #[error("Agent {0} failed during execution: {1}")]
    AgentExecutionFailed(AgentPid, String),

    #[error("Supervisor {0} exceeded maximum restart attempts")]
    MaxRestartsExceeded(SupervisorId),

    #[error("Agent {0} restart failed: {1}")]
    RestartFailed(AgentPid, String),

    #[error("Supervision tree shutdown failed: {0}")]
    ShutdownFailed(String),

    #[error("Agent specification invalid: {0}")]
    InvalidAgentSpec(String),

    #[error("Supervisor configuration invalid: {0}")]
    InvalidSupervisorConfig(String),

    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Timeout waiting for agent response")]
    Timeout,

    #[error("Agent communication failed: {0}")]
    CommunicationFailed(String),

    #[error("System error: {0}")]
    System(String),
}

impl SupervisionError {
    /// Check if this error is recoverable through restart
    pub fn is_recoverable(&self) -> bool {
        match self {
            SupervisionError::AgentExecutionFailed(_, _) => true,
            SupervisionError::AgentStartFailed(_, _) => true,
            SupervisionError::CommunicationFailed(_) => true,
            SupervisionError::Timeout => true,
            SupervisionError::MaxRestartsExceeded(_) => false,
            SupervisionError::InvalidAgentSpec(_) => false,
            SupervisionError::InvalidSupervisorConfig(_) => false,
            SupervisionError::ShutdownFailed(_) => false,
            SupervisionError::System(_) => false,
            SupervisionError::AgentNotFound(_) => false,
            SupervisionError::SupervisorNotFound(_) => false,
            SupervisionError::RestartFailed(_, _) => false,
            SupervisionError::Persistence(_) => true,
            SupervisionError::Serialization(_) => false,
        }
    }

    /// Get error category for monitoring and alerting
    pub fn category(&self) -> ErrorCategory {
        match self {
            SupervisionError::AgentNotFound(_) => ErrorCategory::NotFound,
            SupervisionError::SupervisorNotFound(_) => ErrorCategory::NotFound,
            SupervisionError::AgentStartFailed(_, _) => ErrorCategory::Startup,
            SupervisionError::AgentExecutionFailed(_, _) => ErrorCategory::Runtime,
            SupervisionError::MaxRestartsExceeded(_) => ErrorCategory::Policy,
            SupervisionError::RestartFailed(_, _) => ErrorCategory::Recovery,
            SupervisionError::ShutdownFailed(_) => ErrorCategory::Shutdown,
            SupervisionError::InvalidAgentSpec(_) => ErrorCategory::Configuration,
            SupervisionError::InvalidSupervisorConfig(_) => ErrorCategory::Configuration,
            SupervisionError::Persistence(_) => ErrorCategory::Storage,
            SupervisionError::Serialization(_) => ErrorCategory::Serialization,
            SupervisionError::Timeout => ErrorCategory::Communication,
            SupervisionError::CommunicationFailed(_) => ErrorCategory::Communication,
            SupervisionError::System(_) => ErrorCategory::System,
        }
    }
}

/// Error categories for monitoring and alerting
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    NotFound,
    Startup,
    Runtime,
    Policy,
    Recovery,
    Shutdown,
    Configuration,
    Storage,
    Serialization,
    Communication,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentPid, SupervisorId};

    #[test]
    fn test_error_recoverability() {
        let recoverable_error =
            SupervisionError::AgentExecutionFailed(AgentPid::new(), "test error".to_string());
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error = SupervisionError::InvalidAgentSpec("invalid spec".to_string());
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_error_categorization() {
        let runtime_error =
            SupervisionError::AgentExecutionFailed(AgentPid::new(), "runtime failure".to_string());
        assert_eq!(runtime_error.category(), ErrorCategory::Runtime);

        let config_error = SupervisionError::InvalidSupervisorConfig("bad config".to_string());
        assert_eq!(config_error.category(), ErrorCategory::Configuration);
    }
}
