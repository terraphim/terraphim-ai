//! Error types for the orchestration engine

use crate::TaskId;
use thiserror::Error;

/// Errors that can occur in the orchestration engine
#[derive(Error, Debug)]
pub enum OrchestrationError {
    #[error("Agent {0} not found")]
    AgentNotFound(String),

    #[error("No suitable agent found for task {0}")]
    NoSuitableAgent(TaskId),

    #[error("Task {0} execution failed: {1}")]
    TaskExecutionFailed(TaskId, String),

    #[error("Workflow execution failed: {0}")]
    WorkflowExecutionFailed(String),

    #[error("Agent pool error: {0}")]
    AgentPoolError(String),

    #[error("Scheduling error: {0}")]
    SchedulingError(String),

    #[error("Coordination error: {0}")]
    CoordinationError(String),

    #[error("Task decomposition error: {0}")]
    TaskDecomposition(#[from] terraphim_task_decomposition::TaskDecompositionError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("System error: {0}")]
    System(String),

    #[error("Supervision error: {0}")]
    SupervisionError(String),

    #[error("System error: {0}")]
    SystemError(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
}

impl OrchestrationError {
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            OrchestrationError::AgentNotFound(_) => true,
            OrchestrationError::NoSuitableAgent(_) => true,
            OrchestrationError::TaskExecutionFailed(_, _) => true,
            OrchestrationError::WorkflowExecutionFailed(_) => true,
            OrchestrationError::AgentPoolError(_) => true,
            OrchestrationError::SchedulingError(_) => true,
            OrchestrationError::CoordinationError(_) => true,
            OrchestrationError::TaskDecomposition(e) => e.is_recoverable(),
            OrchestrationError::Serialization(_) => false,
            OrchestrationError::System(_) => false,
            OrchestrationError::SupervisionError(_) => true,
            OrchestrationError::SystemError(_) => false,
            OrchestrationError::ResourceExhausted(_) => true,
            OrchestrationError::WorkflowNotFound(_) => false,
        }
    }

    /// Get error category for monitoring
    pub fn category(&self) -> ErrorCategory {
        match self {
            OrchestrationError::AgentNotFound(_) => ErrorCategory::Agent,
            OrchestrationError::NoSuitableAgent(_) => ErrorCategory::Agent,
            OrchestrationError::TaskExecutionFailed(_, _) => ErrorCategory::Execution,
            OrchestrationError::WorkflowExecutionFailed(_) => ErrorCategory::Execution,
            OrchestrationError::AgentPoolError(_) => ErrorCategory::Agent,
            OrchestrationError::SchedulingError(_) => ErrorCategory::Scheduling,
            OrchestrationError::CoordinationError(_) => ErrorCategory::Coordination,
            OrchestrationError::TaskDecomposition(_) => ErrorCategory::TaskDecomposition,
            OrchestrationError::Serialization(_) => ErrorCategory::Serialization,
            OrchestrationError::System(_) => ErrorCategory::System,
            OrchestrationError::SupervisionError(_) => ErrorCategory::System,
            OrchestrationError::SystemError(_) => ErrorCategory::System,
            OrchestrationError::ResourceExhausted(_) => ErrorCategory::System,
            OrchestrationError::WorkflowNotFound(_) => ErrorCategory::System,
        }
    }
}

/// Error categories for monitoring and alerting
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    Agent,
    Execution,
    Scheduling,
    Coordination,
    TaskDecomposition,
    Serialization,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        let recoverable_error = OrchestrationError::AgentNotFound("test_agent".to_string());
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error = OrchestrationError::System("system failure".to_string());
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_error_categorization() {
        let agent_error = OrchestrationError::AgentNotFound("test_agent".to_string());
        assert_eq!(agent_error.category(), ErrorCategory::Agent);

        let execution_error = OrchestrationError::TaskExecutionFailed(
            "test_task".to_string(),
            "execution failed".to_string(),
        );
        assert_eq!(execution_error.category(), ErrorCategory::Execution);
    }
}
