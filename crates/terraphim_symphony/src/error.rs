//! Error types for the Symphony orchestration service.

/// All errors that can occur in the Symphony service.
#[derive(Debug, thiserror::Error)]
pub enum SymphonyError {
    // --- Workflow / Configuration ---
    #[error("missing workflow file: {path}")]
    MissingWorkflowFile { path: String },

    #[error("workflow parse error: {reason}")]
    WorkflowParseError { reason: String },

    #[error("workflow front matter is not a map")]
    WorkflowFrontMatterNotAMap,

    #[error("template parse error: {reason}")]
    TemplateParseError { reason: String },

    #[error("template render error: {reason}")]
    TemplateRenderError { reason: String },

    #[error("validation failed: {checks:?}")]
    ValidationFailed { checks: Vec<String> },

    // --- Issue Tracker ---
    #[error("unsupported tracker kind: {kind}")]
    UnsupportedTrackerKind { kind: String },

    #[error("tracker error ({kind}): {message}")]
    Tracker { kind: String, message: String },

    #[error("authentication missing for {service}")]
    AuthenticationMissing { service: String },

    // --- Workspace ---
    #[error("workspace error for {identifier}: {reason}")]
    Workspace { identifier: String, reason: String },

    #[error("workspace path outside root: {path}")]
    WorkspacePathOutsideRoot { path: String },

    // --- Agent ---
    #[error("agent error: {reason}")]
    Agent { reason: String },

    #[error("agent protocol error: {method} - {message}")]
    AgentProtocol { method: String, message: String },

    #[error("agent timeout after {duration_secs}s")]
    AgentTimeout { duration_secs: u64 },

    #[error("agent stalled: no activity for {duration_secs}s")]
    AgentStalled { duration_secs: u64 },

    #[error("agent requested user input (unsupported)")]
    AgentUserInputRequired,

    // --- Hooks ---
    #[error("hook failed ({hook}): {reason}")]
    HookFailed { hook: String, reason: String },

    #[error("hook timed out ({hook}) after {timeout_ms}ms")]
    HookTimeout { hook: String, timeout_ms: u64 },

    // --- Standard wrapping ---
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    SerdeYaml(#[from] serde_yaml::Error),
}

impl SymphonyError {
    /// Whether this error is recoverable (the operation can be retried).
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            SymphonyError::Tracker { .. }
                | SymphonyError::AgentTimeout { .. }
                | SymphonyError::AgentStalled { .. }
                | SymphonyError::Agent { .. }
                | SymphonyError::HookFailed { .. }
                | SymphonyError::HookTimeout { .. }
                | SymphonyError::Reqwest(_)
                | SymphonyError::Io(_)
        )
    }

    /// Whether this error should trigger a retry with backoff.
    pub fn should_retry(&self) -> bool {
        matches!(
            self,
            SymphonyError::Tracker { .. }
                | SymphonyError::AgentTimeout { .. }
                | SymphonyError::AgentStalled { .. }
                | SymphonyError::Agent { .. }
                | SymphonyError::Reqwest(_)
        )
    }
}

/// Convenience type alias.
pub type Result<T> = std::result::Result<T, SymphonyError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_errors_are_recoverable() {
        let err = SymphonyError::Tracker {
            kind: "linear".into(),
            message: "timeout".into(),
        };
        assert!(err.is_recoverable());
        assert!(err.should_retry());
    }

    #[test]
    fn validation_errors_are_not_recoverable() {
        let err = SymphonyError::ValidationFailed {
            checks: vec!["missing tracker.kind".into()],
        };
        assert!(!err.is_recoverable());
        assert!(!err.should_retry());
    }

    #[test]
    fn missing_workflow_is_not_recoverable() {
        let err = SymphonyError::MissingWorkflowFile {
            path: "./WORKFLOW.md".into(),
        };
        assert!(!err.is_recoverable());
    }

    #[test]
    fn hook_errors_are_recoverable() {
        let err = SymphonyError::HookFailed {
            hook: "before_run".into(),
            reason: "exit code 1".into(),
        };
        assert!(err.is_recoverable());
        // Hook failures are recoverable but should not trigger backoff retry
        assert!(!err.should_retry());
    }
}
