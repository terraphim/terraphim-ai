//! Error types for the goal alignment system

use crate::{AgentPid, GoalId};
use thiserror::Error;

/// Errors that can occur in the goal alignment system
#[derive(Error, Debug)]
pub enum GoalAlignmentError {
    #[error("Goal {0} not found")]
    GoalNotFound(GoalId),

    #[error("Goal {0} already exists")]
    GoalAlreadyExists(GoalId),

    #[error("Goal hierarchy validation failed: {0}")]
    HierarchyValidationFailed(String),

    #[error("Goal conflict detected between {0} and {1}: {2}")]
    GoalConflict(GoalId, GoalId, String),

    #[error("Goal propagation failed: {0}")]
    PropagationFailed(String),

    #[error("Knowledge graph operation failed: {0}")]
    KnowledgeGraphError(String),

    #[error("Role graph operation failed: {0}")]
    RoleGraphError(String),

    #[error("Goal alignment validation failed for {0}: {1}")]
    AlignmentValidationFailed(GoalId, String),

    #[error("Agent {0} not found for goal assignment")]
    AgentNotFound(AgentPid),

    #[error("Invalid goal specification for {0}: {1}")]
    InvalidGoalSpec(GoalId, String),

    #[error("Goal dependency cycle detected: {0}")]
    DependencyCycle(String),

    #[error("Goal constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("System error: {0}")]
    System(String),
}

impl GoalAlignmentError {
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            GoalAlignmentError::GoalNotFound(_) => true,
            GoalAlignmentError::GoalAlreadyExists(_) => false,
            GoalAlignmentError::HierarchyValidationFailed(_) => true,
            GoalAlignmentError::GoalConflict(_, _, _) => true,
            GoalAlignmentError::PropagationFailed(_) => true,
            GoalAlignmentError::KnowledgeGraphError(_) => true,
            GoalAlignmentError::RoleGraphError(_) => true,
            GoalAlignmentError::AlignmentValidationFailed(_, _) => true,
            GoalAlignmentError::AgentNotFound(_) => true,
            GoalAlignmentError::InvalidGoalSpec(_, _) => false,
            GoalAlignmentError::DependencyCycle(_) => false,
            GoalAlignmentError::ConstraintViolation(_) => true,
            GoalAlignmentError::Serialization(_) => false,
            GoalAlignmentError::System(_) => false,
        }
    }

    /// Get error category for monitoring
    pub fn category(&self) -> ErrorCategory {
        match self {
            GoalAlignmentError::GoalNotFound(_) => ErrorCategory::NotFound,
            GoalAlignmentError::GoalAlreadyExists(_) => ErrorCategory::Conflict,
            GoalAlignmentError::HierarchyValidationFailed(_) => ErrorCategory::Validation,
            GoalAlignmentError::GoalConflict(_, _, _) => ErrorCategory::Conflict,
            GoalAlignmentError::PropagationFailed(_) => ErrorCategory::Propagation,
            GoalAlignmentError::KnowledgeGraphError(_) => ErrorCategory::KnowledgeGraph,
            GoalAlignmentError::RoleGraphError(_) => ErrorCategory::RoleGraph,
            GoalAlignmentError::AlignmentValidationFailed(_, _) => ErrorCategory::Validation,
            GoalAlignmentError::AgentNotFound(_) => ErrorCategory::NotFound,
            GoalAlignmentError::InvalidGoalSpec(_, _) => ErrorCategory::Validation,
            GoalAlignmentError::DependencyCycle(_) => ErrorCategory::Validation,
            GoalAlignmentError::ConstraintViolation(_) => ErrorCategory::Constraint,
            GoalAlignmentError::Serialization(_) => ErrorCategory::Serialization,
            GoalAlignmentError::System(_) => ErrorCategory::System,
        }
    }
}

/// Error categories for monitoring and alerting
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    NotFound,
    Conflict,
    Validation,
    Propagation,
    KnowledgeGraph,
    RoleGraph,
    Constraint,
    Serialization,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        let recoverable_error = GoalAlignmentError::GoalNotFound("test_goal".to_string());
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error = GoalAlignmentError::InvalidGoalSpec(
            "test_goal".to_string(),
            "invalid spec".to_string(),
        );
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_error_categorization() {
        let not_found_error = GoalAlignmentError::GoalNotFound("test_goal".to_string());
        assert_eq!(not_found_error.category(), ErrorCategory::NotFound);

        let conflict_error = GoalAlignmentError::GoalConflict(
            "goal1".to_string(),
            "goal2".to_string(),
            "conflicting objectives".to_string(),
        );
        assert_eq!(conflict_error.category(), ErrorCategory::Conflict);
    }
}
