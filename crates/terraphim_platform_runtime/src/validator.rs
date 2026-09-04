//! Validator-handle vocabulary for the Terraphim platform runtime.

use crate::{FailureEvent, ValidatorError, Workspace};

/// Abstraction over a validation pass that scans a workspace for failures.
#[async_trait::async_trait]
pub trait ValidatorHandle: Send + Sync {
    /// Validate `workspace` and return any failure events found.
    async fn validate(&self, workspace: &Workspace) -> Result<Vec<FailureEvent>, ValidatorError>;
}
