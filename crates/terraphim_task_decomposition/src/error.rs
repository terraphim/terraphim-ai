//! Error types for the task decomposition system

use crate::TaskId;
use thiserror::Error;

/// Errors that can occur in the task decomposition system
#[derive(Error, Debug)]
pub enum TaskDecompositionError {
    #[error("Task {0} not found")]
    TaskNotFound(TaskId),

    #[error("Task {0} already exists")]
    TaskAlreadyExists(TaskId),

    #[error("Task decomposition failed for {0}: {1}")]
    DecompositionFailed(TaskId, String),

    #[error("Task analysis failed for {0}: {1}")]
    AnalysisFailed(TaskId, String),

    #[error("Execution plan generation failed: {0}")]
    PlanGenerationFailed(String),

    #[error("Knowledge graph operation failed: {0}")]
    KnowledgeGraphError(String),

    #[error("Role graph operation failed: {0}")]
    RoleGraphError(String),

    #[error("Task dependency cycle detected: {0}")]
    DependencyCycle(String),

    #[error("Invalid task specification for {0}: {1}")]
    InvalidTaskSpec(TaskId, String),

    #[error("Task complexity analysis failed for {0}: {1}")]
    ComplexityAnalysisFailed(TaskId, String),

    #[error("Agent assignment failed for task {0}: {1}")]
    AgentAssignmentFailed(TaskId, String),

    #[error("Task execution planning failed: {0}")]
    ExecutionPlanningFailed(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("System error: {0}")]
    System(String),
}

impl TaskDecompositionError {
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            TaskDecompositionError::TaskNotFound(_) => true,
            TaskDecompositionError::TaskAlreadyExists(_) => false,
            TaskDecompositionError::DecompositionFailed(_, _) => true,
            TaskDecompositionError::AnalysisFailed(_, _) => true,
            TaskDecompositionError::PlanGenerationFailed(_) => true,
            TaskDecompositionError::KnowledgeGraphError(_) => true,
            TaskDecompositionError::RoleGraphError(_) => true,
            TaskDecompositionError::DependencyCycle(_) => false,
            TaskDecompositionError::InvalidTaskSpec(_, _) => false,
            TaskDecompositionError::ComplexityAnalysisFailed(_, _) => true,
            TaskDecompositionError::AgentAssignmentFailed(_, _) => true,
            TaskDecompositionError::ExecutionPlanningFailed(_) => true,
            TaskDecompositionError::Serialization(_) => false,
            TaskDecompositionError::System(_) => false,
        }
    }

    /// Get error category for monitoring
    pub fn category(&self) -> ErrorCategory {
        match self {
            TaskDecompositionError::TaskNotFound(_) => ErrorCategory::NotFound,
            TaskDecompositionError::TaskAlreadyExists(_) => ErrorCategory::Conflict,
            TaskDecompositionError::DecompositionFailed(_, _) => ErrorCategory::Decomposition,
            TaskDecompositionError::AnalysisFailed(_, _) => ErrorCategory::Analysis,
            TaskDecompositionError::PlanGenerationFailed(_) => ErrorCategory::Planning,
            TaskDecompositionError::KnowledgeGraphError(_) => ErrorCategory::KnowledgeGraph,
            TaskDecompositionError::RoleGraphError(_) => ErrorCategory::RoleGraph,
            TaskDecompositionError::DependencyCycle(_) => ErrorCategory::Validation,
            TaskDecompositionError::InvalidTaskSpec(_, _) => ErrorCategory::Validation,
            TaskDecompositionError::ComplexityAnalysisFailed(_, _) => ErrorCategory::Analysis,
            TaskDecompositionError::AgentAssignmentFailed(_, _) => ErrorCategory::Assignment,
            TaskDecompositionError::ExecutionPlanningFailed(_) => ErrorCategory::Planning,
            TaskDecompositionError::Serialization(_) => ErrorCategory::Serialization,
            TaskDecompositionError::System(_) => ErrorCategory::System,
        }
    }
}

/// Error categories for monitoring and alerting
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    NotFound,
    Conflict,
    Decomposition,
    Analysis,
    Planning,
    KnowledgeGraph,
    RoleGraph,
    Validation,
    Assignment,
    Serialization,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        let recoverable_error = TaskDecompositionError::TaskNotFound("test_task".to_string());
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error = TaskDecompositionError::InvalidTaskSpec(
            "test_task".to_string(),
            "invalid spec".to_string(),
        );
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_error_categorization() {
        let not_found_error = TaskDecompositionError::TaskNotFound("test_task".to_string());
        assert_eq!(not_found_error.category(), ErrorCategory::NotFound);

        let decomposition_error = TaskDecompositionError::DecompositionFailed(
            "test_task".to_string(),
            "decomposition failed".to_string(),
        );
        assert_eq!(decomposition_error.category(), ErrorCategory::Decomposition);
    }
}
