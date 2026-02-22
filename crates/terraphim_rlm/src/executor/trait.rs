//! ExecutionEnvironment trait definition.
//!
//! This trait defines the interface that all execution backends must implement.

use async_trait::async_trait;

use super::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
use crate::types::SessionId;

/// The core trait for execution backends.
///
/// All execution backends (Firecracker, Docker, E2B) implement this trait
/// to provide a unified interface for:
/// - Code execution (Python, bash)
/// - Command validation (knowledge graph)
/// - Snapshot management (state versioning)
///
/// # Example
///
/// ```rust,ignore
/// use terraphim_rlm::executor::{ExecutionEnvironment, ExecutionContext};
///
/// async fn run_code<E: ExecutionEnvironment>(
///     executor: &E,
///     code: &str,
/// ) -> Result<String, E::Error> {
///     let ctx = ExecutionContext::default();
///     let result = executor.execute_code(code, &ctx).await?;
///     Ok(result.stdout)
/// }
/// ```
#[async_trait]
pub trait ExecutionEnvironment: Send + Sync {
    /// Error type for this executor.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute Python code in the environment.
    ///
    /// # Arguments
    ///
    /// * `code` - Python code to execute
    /// * `ctx` - Execution context with session info, timeouts, etc.
    ///
    /// # Returns
    ///
    /// `ExecutionResult` containing stdout, stderr, and exit status.
    async fn execute_code(
        &self,
        code: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error>;

    /// Execute a bash command in the environment.
    ///
    /// # Arguments
    ///
    /// * `cmd` - Bash command to execute
    /// * `ctx` - Execution context with session info, timeouts, etc.
    ///
    /// # Returns
    ///
    /// `ExecutionResult` containing stdout, stderr, and exit status.
    async fn execute_command(
        &self,
        cmd: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error>;

    /// Validate input against knowledge graph.
    ///
    /// # Arguments
    ///
    /// * `input` - Text to validate (command or code)
    ///
    /// # Returns
    ///
    /// `ValidationResult` with matched terms and any unknown terms.
    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error>;

    /// Create a named snapshot of the current environment state for a session.
    ///
    /// Snapshots capture:
    /// - Python interpreter state (variables, imports)
    /// - Filesystem state (OverlayFS upper layer)
    /// - Environment variables
    /// - VM state (for Firecracker)
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session to create snapshot for
    /// * `name` - User-provided name for the snapshot
    ///
    /// # Returns
    ///
    /// `SnapshotId` that can be used to restore this state.
    async fn create_snapshot(
        &self,
        session_id: &SessionId,
        name: &str,
    ) -> Result<SnapshotId, Self::Error>;

    /// Restore environment to a previous snapshot.
    ///
    /// Restores complete state including:
    /// - Python interpreter state
    /// - Filesystem state
    /// - Environment variables
    /// - VM state (for Firecracker)
    ///
    /// Note: External state (APIs, databases) is not restored.
    /// Per spec: "Ignore external state drift on restore".
    ///
    /// # Arguments
    ///
    /// * `id` - Snapshot to restore
    async fn restore_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error>;

    /// List available snapshots for a session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session to list snapshots for
    async fn list_snapshots(&self, session_id: &SessionId) -> Result<Vec<SnapshotId>, Self::Error>;

    /// Delete a snapshot.
    async fn delete_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error>;

    /// Delete all snapshots for a session.
    ///
    /// Called when a session is destroyed.
    async fn delete_session_snapshots(&self, session_id: &SessionId) -> Result<(), Self::Error>;

    /// Get the capabilities supported by this executor.
    ///
    /// Different backends may have different capabilities:
    /// - Firecracker: Full isolation, snapshots, network audit
    /// - Docker: Container isolation, may lack snapshots
    /// - E2B: Cloud execution, may have network restrictions
    fn capabilities(&self) -> &[Capability];

    /// Check if a specific capability is supported.
    fn has_capability(&self, capability: Capability) -> bool {
        self.capabilities().contains(&capability)
    }

    /// Get the backend type for this executor.
    fn backend_type(&self) -> crate::config::BackendType;

    /// Check if the executor is healthy and ready to accept work.
    async fn health_check(&self) -> Result<bool, Self::Error>;

    /// Cleanup resources associated with this executor.
    ///
    /// Called when shutting down or when a session ends.
    async fn cleanup(&self) -> Result<(), Self::Error>;
}
