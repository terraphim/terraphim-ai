//! Error types for the Terraphim platform runtime.

use thiserror::Error;

/// Errors returned by policy planners.
#[derive(Debug, Error)]
pub enum PolicyError {
    /// The command is not on the allowlist.
    #[error("command disallowed by policy: {0}")]
    Disallowed(String),
    /// The workspace snapshot failed validation.
    #[error("workspace validation failed: {0}")]
    WorkspaceValidation(String),
    /// An internal planner error occurred.
    #[error("internal planner error: {0}")]
    Internal(String),
}

/// Errors returned by workspace validators.
#[derive(Debug, Error)]
pub enum ValidatorError {
    /// The validation pass reported actionable failures.
    #[error("validation failed: {0}")]
    Validation(String),
    /// An internal validator error occurred.
    #[error("internal validator error: {0}")]
    Internal(String),
}

/// Errors returned by executor backends.
#[derive(Debug, Error)]
pub enum ExecutionError {
    /// The command exited unsuccessfully or could not be started.
    #[error("execution failed: {0}")]
    Execution(String),
    /// The selected backend is not available.
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),
    /// The execution did not complete in time.
    #[error("execution timed out")]
    Timeout,
}
