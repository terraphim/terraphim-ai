//! Error types for the GenAgent framework

use crate::AgentPid;
use thiserror::Error;

/// Errors that can occur in the GenAgent framework
#[derive(Error, Debug)]
pub enum GenAgentError {
    #[error("Agent {0} initialization failed: {1}")]
    InitializationFailed(AgentPid, String),

    #[error("Agent {0} message handling failed: {1}")]
    MessageHandlingFailed(AgentPid, String),

    #[error("Agent {0} state transition failed: {1}")]
    StateTransitionFailed(AgentPid, String),

    #[error("Agent {0} termination failed: {1}")]
    TerminationFailed(AgentPid, String),

    #[error("Invalid message type for agent {0}: expected {1}, got {2}")]
    InvalidMessageType(AgentPid, String, String),

    #[error("Agent {0} timeout during operation: {1}")]
    OperationTimeout(AgentPid, String),

    #[error("Agent {0} is not running")]
    AgentNotRunning(AgentPid),

    #[error("Message {0} timeout for agent {1}")]
    MessageTimeout(crate::MessageId, AgentPid),

    #[error("Agent {0} state serialization failed: {1}")]
    StateSerialization(AgentPid, String),

    #[error("Agent {0} state deserialization failed: {1}")]
    StateDeserialization(AgentPid, String),

    #[error("Agent {0} runtime error: {1}")]
    Runtime(AgentPid, String),

    #[error("Supervisor error: {0}")]
    Supervisor(#[from] terraphim_agent_supervisor::SupervisionError),

    #[error("Messaging error: {0}")]
    Messaging(#[from] terraphim_agent_messaging::MessagingError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("System error: {0}")]
    System(String),
}

impl GenAgentError {
    /// Check if this error is recoverable through restart
    pub fn is_recoverable(&self) -> bool {
        match self {
            GenAgentError::MessageHandlingFailed(_, _) => true,
            GenAgentError::StateTransitionFailed(_, _) => true,
            GenAgentError::OperationTimeout(_, _) => true,
            GenAgentError::AgentNotRunning(_) => true,
            GenAgentError::MessageTimeout(_, _) => true,
            GenAgentError::Runtime(_, _) => true,
            GenAgentError::Supervisor(e) => e.is_recoverable(),
            GenAgentError::Messaging(e) => e.is_recoverable(),
            GenAgentError::InitializationFailed(_, _) => false,
            GenAgentError::TerminationFailed(_, _) => false,
            GenAgentError::InvalidMessageType(_, _, _) => false,
            GenAgentError::StateSerialization(_, _) => false,
            GenAgentError::StateDeserialization(_, _) => false,
            GenAgentError::Serialization(_) => false,
            GenAgentError::System(_) => false,
        }
    }

    /// Get error category for monitoring and alerting
    pub fn category(&self) -> ErrorCategory {
        match self {
            GenAgentError::InitializationFailed(_, _) => ErrorCategory::Initialization,
            GenAgentError::MessageHandlingFailed(_, _) => ErrorCategory::MessageHandling,
            GenAgentError::StateTransitionFailed(_, _) => ErrorCategory::StateManagement,
            GenAgentError::TerminationFailed(_, _) => ErrorCategory::Termination,
            GenAgentError::InvalidMessageType(_, _, _) => ErrorCategory::Validation,
            GenAgentError::AgentNotRunning(_) => ErrorCategory::Runtime,
            GenAgentError::MessageTimeout(_, _) => ErrorCategory::Timeout,
            GenAgentError::OperationTimeout(_, _) => ErrorCategory::Timeout,
            GenAgentError::StateSerialization(_, _) => ErrorCategory::Serialization,
            GenAgentError::StateDeserialization(_, _) => ErrorCategory::Serialization,
            GenAgentError::Runtime(_, _) => ErrorCategory::Runtime,
            GenAgentError::Supervisor(_) => ErrorCategory::Supervision,
            GenAgentError::Messaging(_) => ErrorCategory::Messaging,
            GenAgentError::Serialization(_) => ErrorCategory::Serialization,
            GenAgentError::System(_) => ErrorCategory::System,
        }
    }

    /// Get the agent ID associated with this error (if any)
    pub fn agent_id(&self) -> Option<&AgentPid> {
        match self {
            GenAgentError::InitializationFailed(pid, _) => Some(pid),
            GenAgentError::MessageHandlingFailed(pid, _) => Some(pid),
            GenAgentError::StateTransitionFailed(pid, _) => Some(pid),
            GenAgentError::TerminationFailed(pid, _) => Some(pid),
            GenAgentError::InvalidMessageType(pid, _, _) => Some(pid),
            GenAgentError::AgentNotRunning(pid) => Some(pid),
            GenAgentError::MessageTimeout(_, pid) => Some(pid),
            GenAgentError::OperationTimeout(pid, _) => Some(pid),
            GenAgentError::StateSerialization(pid, _) => Some(pid),
            GenAgentError::StateDeserialization(pid, _) => Some(pid),
            GenAgentError::Runtime(pid, _) => Some(pid),
            _ => None,
        }
    }
}

/// Error categories for monitoring and alerting
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    Initialization,
    MessageHandling,
    StateManagement,
    Termination,
    Validation,
    Timeout,
    Serialization,
    Runtime,
    Supervision,
    Messaging,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        let recoverable_error =
            GenAgentError::MessageHandlingFailed(AgentPid::new(), "test error".to_string());
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error = GenAgentError::InvalidMessageType(
            AgentPid::new(),
            "expected".to_string(),
            "unknown message type".to_string(),
        );
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_error_categorization() {
        let runtime_error = GenAgentError::Runtime(AgentPid::new(), "runtime failure".to_string());
        assert_eq!(runtime_error.category(), ErrorCategory::Runtime);

        let state_error = GenAgentError::StateTransitionFailed(
            AgentPid::new(),
            "invalid state transition".to_string(),
        );
        assert_eq!(state_error.category(), ErrorCategory::StateManagement);
    }

    #[test]
    fn test_agent_id_extraction() {
        let agent_id = AgentPid::new();
        let error =
            GenAgentError::MessageHandlingFailed(agent_id.clone(), "test error".to_string());

        assert_eq!(error.agent_id(), Some(&agent_id));

        let system_error = GenAgentError::System("system failure".to_string());
        assert_eq!(system_error.agent_id(), None);
    }
}
