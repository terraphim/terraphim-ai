//! Error types for knowledge graph agents

use thiserror::Error;

use terraphim_gen_agent::GenAgentError;
use terraphim_task_decomposition::TaskDecompositionError;

/// Errors that can occur in knowledge graph agent operations
#[derive(Error, Debug, Clone)]
pub enum KgAgentError {
    /// Task decomposition failed
    #[error("Task decomposition failed: {0}")]
    DecompositionFailed(String),

    /// Knowledge graph query failed
    #[error("Knowledge graph query failed: {0}")]
    KnowledgeGraphError(String),

    /// Agent coordination failed
    #[error("Agent coordination failed: {0}")]
    CoordinationFailed(String),

    /// Task execution failed
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    /// Domain specialization error
    #[error("Domain specialization error: {0}")]
    DomainError(String),

    /// Task compatibility check failed
    #[error("Task compatibility check failed: {0}")]
    CompatibilityError(String),

    /// Planning error
    #[error("Planning error: {0}")]
    PlanningError(String),

    /// Worker agent error
    #[error("Worker agent error: {0}")]
    WorkerError(String),

    /// Coordination agent error
    #[error("Coordination agent error: {0}")]
    CoordinationAgentError(String),

    /// Generic GenAgent error
    #[error("GenAgent error: {0}")]
    GenAgentError(#[from] GenAgentError),

    /// Task decomposition error
    #[error("Task decomposition error: {0}")]
    TaskDecompositionError(#[from] TaskDecompositionError),

    /// System error
    #[error("System error: {0}")]
    SystemError(String),
}

impl KgAgentError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            KgAgentError::DecompositionFailed(_) => true,
            KgAgentError::KnowledgeGraphError(_) => true,
            KgAgentError::CoordinationFailed(_) => true,
            KgAgentError::ExecutionFailed(_) => false, // Execution failures are usually not recoverable
            KgAgentError::DomainError(_) => false,
            KgAgentError::CompatibilityError(_) => false,
            KgAgentError::PlanningError(_) => true,
            KgAgentError::WorkerError(_) => true,
            KgAgentError::CoordinationAgentError(_) => true,
            KgAgentError::GenAgentError(e) => e.is_recoverable(),
            KgAgentError::TaskDecompositionError(_) => true,
            KgAgentError::SystemError(_) => false,
        }
    }

    /// Get error category for logging and monitoring
    pub fn category(&self) -> &'static str {
        match self {
            KgAgentError::DecompositionFailed(_) => "decomposition",
            KgAgentError::KnowledgeGraphError(_) => "knowledge_graph",
            KgAgentError::CoordinationFailed(_) => "coordination",
            KgAgentError::ExecutionFailed(_) => "execution",
            KgAgentError::DomainError(_) => "domain",
            KgAgentError::CompatibilityError(_) => "compatibility",
            KgAgentError::PlanningError(_) => "planning",
            KgAgentError::WorkerError(_) => "worker",
            KgAgentError::CoordinationAgentError(_) => "coordination_agent",
            KgAgentError::GenAgentError(_) => "gen_agent",
            KgAgentError::TaskDecompositionError(_) => "task_decomposition",
            KgAgentError::SystemError(_) => "system",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        assert!(KgAgentError::DecompositionFailed("test".to_string()).is_recoverable());
        assert!(!KgAgentError::ExecutionFailed("test".to_string()).is_recoverable());
        assert!(!KgAgentError::SystemError("test".to_string()).is_recoverable());
    }

    #[test]
    fn test_error_categorization() {
        assert_eq!(
            KgAgentError::PlanningError("test".to_string()).category(),
            "planning"
        );
        assert_eq!(
            KgAgentError::WorkerError("test".to_string()).category(),
            "worker"
        );
        assert_eq!(
            KgAgentError::CoordinationFailed("test".to_string()).category(),
            "coordination"
        );
    }
}
