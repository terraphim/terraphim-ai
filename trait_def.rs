// ExecutionEnvironment trait definition using async_trait for dyn compatibility
use async_trait::async_trait;

#[async_trait]
pub trait ExecutionEnvironment: Send + Sync {
    /// Error type returned by this environment.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute Python code.
    async fn execute_code(&self, code: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error>;

    /// Execute a bash command.
    async fn execute_command(&self, cmd: &str, ctx: &ExecutionContext) -> Result<ExecutionResult, Self::Error>;

    /// Create a snapshot.
    async fn create_snapshot(&self, session_id: &crate::types::SessionId, name: &str) -> Result<SnapshotId, Self::Error>;

    /// Restore a snapshot.
    async fn restore_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error>;

    /// List snapshots for a session.
    async fn list_snapshots(&self, session_id: &crate::types::SessionId) -> Result<Vec<SnapshotId>, Self::Error>;

    /// Get the capabilities supported by this environment.
    fn capabilities(&self) -> &[Capability];

    /// Check if a specific capability is supported.
    fn has_capability(&self, capability: Capability) -> bool;

    /// Perform a health check.
    async fn health_check(&self) -> Result<bool, Self::Error>;

    /// Clean up resources.
    async fn cleanup(&self) -> Result<(), Self::Error>;
}