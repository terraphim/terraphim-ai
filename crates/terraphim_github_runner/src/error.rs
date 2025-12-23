//! Error types for GitHub runner operations

use thiserror::Error;

/// Result type for GitHub runner operations
pub type Result<T> = std::result::Result<T, GitHubRunnerError>;

/// Errors that can occur during GitHub runner operations
#[derive(Error, Debug)]
pub enum GitHubRunnerError {
    /// Webhook signature verification failed
    #[error("Webhook signature verification failed: {0}")]
    SignatureVerification(String),

    /// Failed to parse webhook payload
    #[error("Failed to parse webhook payload: {0}")]
    PayloadParsing(String),

    /// Failed to parse workflow YAML
    #[error("Failed to parse workflow: {0}")]
    WorkflowParsing(String),

    /// LLM failed to understand workflow
    #[error("LLM workflow understanding failed: {0}")]
    LlmUnderstanding(String),

    /// VM allocation failed
    #[error("VM allocation failed: {0}")]
    VmAllocation(String),

    /// VM session not found
    #[error("VM session not found: {session_id}")]
    SessionNotFound { session_id: String },

    /// Command execution failed
    #[error("Command execution failed: {command} - {reason}")]
    ExecutionFailed { command: String, reason: String },

    /// Snapshot creation failed
    #[error("Snapshot creation failed: {0}")]
    SnapshotFailed(String),

    /// Rollback failed
    #[error("Rollback to snapshot {snapshot_id} failed: {reason}")]
    RollbackFailed { snapshot_id: String, reason: String },

    /// Knowledge graph update failed
    #[error("Knowledge graph update failed: {0}")]
    KnowledgeGraphUpdate(String),

    /// GitHub API error
    #[error("GitHub API error: {0}")]
    GitHubApi(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Timeout error
    #[error("Operation timed out after {duration_ms}ms: {operation}")]
    Timeout { operation: String, duration_ms: u64 },
}

impl GitHubRunnerError {
    /// Check if this error is recoverable (can be retried)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            GitHubRunnerError::VmAllocation(_)
                | GitHubRunnerError::Timeout { .. }
                | GitHubRunnerError::GitHubApi(_)
                | GitHubRunnerError::LlmUnderstanding(_)
        )
    }

    /// Check if this error should trigger a rollback
    pub fn should_rollback(&self) -> bool {
        matches!(
            self,
            GitHubRunnerError::ExecutionFailed { .. } | GitHubRunnerError::Timeout { .. }
        )
    }

    /// Check if this error should be recorded as a lesson
    pub fn should_record_lesson(&self) -> bool {
        matches!(
            self,
            GitHubRunnerError::ExecutionFailed { .. }
                | GitHubRunnerError::WorkflowParsing(_)
                | GitHubRunnerError::LlmUnderstanding(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        let recoverable = GitHubRunnerError::VmAllocation("pool exhausted".to_string());
        assert!(recoverable.is_recoverable());

        let not_recoverable = GitHubRunnerError::Configuration("invalid config".to_string());
        assert!(!not_recoverable.is_recoverable());
    }

    #[test]
    fn test_error_should_rollback() {
        let should = GitHubRunnerError::ExecutionFailed {
            command: "cargo build".to_string(),
            reason: "compilation error".to_string(),
        };
        assert!(should.should_rollback());

        let should_not = GitHubRunnerError::SignatureVerification("bad sig".to_string());
        assert!(!should_not.should_rollback());
    }

    #[test]
    fn test_error_should_record_lesson() {
        let should = GitHubRunnerError::ExecutionFailed {
            command: "npm install".to_string(),
            reason: "missing dependency".to_string(),
        };
        assert!(should.should_record_lesson());

        let should_not = GitHubRunnerError::VmAllocation("no VMs".to_string());
        assert!(!should_not.should_record_lesson());
    }
}
