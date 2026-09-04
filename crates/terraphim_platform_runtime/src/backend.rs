//! Executor-backend vocabulary for the Terraphim platform runtime.

use std::time::Duration;

use crate::{ExecutionError, Workspace};

/// Result of executing a command through a backend.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExecutionResult {
    /// Process exit code.
    pub exit_code: i32,
    /// Wall-clock duration of the execution.
    pub duration: Duration,
    /// Last lines of standard output.
    pub stdout_tail: String,
    /// Last lines of standard error.
    pub stderr_tail: String,
}

/// Abstraction over a command-execution backend.
#[async_trait::async_trait]
pub trait ExecutorBackend: Send + Sync {
    /// Execute `command` inside `workspace`.
    async fn execute(
        &self,
        command: &str,
        workspace: &Workspace,
    ) -> Result<ExecutionResult, ExecutionError>;
}
