//! Mock execution backend for CI and testing.
//!
//! This module provides `MockExecutor`, a mock implementation of the
//! `ExecutionEnvironment` trait that returns stub data without actual VM operations.
//! It's useful for:
//! - CI environments where KVM/Firecracker is unavailable
//! - Unit testing without infrastructure dependencies
//! - Development and debugging
//!
//! ## Usage
//!
//! ```rust,ignore
//! use terraphim_rlm::executor::MockExecutor;
//!
//! let executor = MockExecutor::new();
//! executor.initialize().await?;
//!
//! // Execute code (returns stub results)
//! let result = executor.execute_code("print('hello')", &ctx).await?;
//! ```

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
use crate::config::{BackendType, RlmConfig};
use crate::error::{RlmError, RlmResult};
use crate::types::SessionId;

/// Mock execution backend for CI and testing.
///
/// This executor provides stub implementations of all `ExecutionEnvironment`
/// methods without requiring actual VM infrastructure. All operations are
/// logged for test verification.
///
/// # Example
///
/// ```rust,ignore
/// let executor = MockExecutor::new();
/// executor.initialize().await?;
///
/// let result = executor.execute_code("print('test')", &ctx).await?;
/// assert_eq!(result.exit_code, 0);
/// ```
pub struct MockExecutor {
    /// Configuration for the executor
    config: RlmConfig,

    /// Capabilities supported by this executor
    capabilities: Vec<Capability>,

    /// Session to mock VM ID mapping
    session_to_vm: parking_lot::RwLock<HashMap<SessionId, String>>,

    /// Snapshot storage per session
    snapshots: parking_lot::RwLock<HashMap<SessionId, Vec<SnapshotId>>>,

    /// Operation log for test verification
    operation_log: parking_lot::Mutex<Vec<MockOperation>>,

    /// Counter for generating unique IDs
    id_counter: AtomicU64,

    /// Whether the executor is initialized
    initialized: parking_lot::RwLock<bool>,
}

/// Record of a mock operation for test verification.
#[derive(Debug, Clone)]
pub struct MockOperation {
    /// The operation type
    pub op_type: OperationType,
    /// The session ID (if applicable)
    pub session_id: Option<SessionId>,
    /// The input/parameters
    pub input: String,
    /// The result (success/failure)
    pub success: bool,
}

/// Types of mock operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Initialize,
    ExecuteCode,
    ExecuteCommand,
    CreateSnapshot,
    RestoreSnapshot,
    ListSnapshots,
    DeleteSnapshot,
    HealthCheck,
    Cleanup,
    Validate,
}

impl MockExecutor {
    /// Create a new mock executor.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let executor = MockExecutor::new();
    /// ```
    pub fn new() -> Self {
        let capabilities = vec![
            Capability::PythonExecution,
            Capability::BashExecution,
            Capability::Snapshots,
            Capability::FileOperations,
        ];

        Self {
            config: RlmConfig::default(),
            capabilities,
            session_to_vm: parking_lot::RwLock::new(HashMap::new()),
            snapshots: parking_lot::RwLock::new(HashMap::new()),
            operation_log: parking_lot::Mutex::new(Vec::new()),
            id_counter: AtomicU64::new(1),
            initialized: parking_lot::RwLock::new(false),
        }
    }

    /// Create a new mock executor with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - RLM configuration
    pub fn with_config(config: RlmConfig) -> Self {
        let capabilities = vec![
            Capability::PythonExecution,
            Capability::BashExecution,
            Capability::Snapshots,
            Capability::FileOperations,
        ];

        Self {
            config,
            capabilities,
            session_to_vm: parking_lot::RwLock::new(HashMap::new()),
            snapshots: parking_lot::RwLock::new(HashMap::new()),
            operation_log: parking_lot::Mutex::new(Vec::new()),
            id_counter: AtomicU64::new(1),
            initialized: parking_lot::RwLock::new(false),
        }
    }

    /// Initialize the mock executor.
    ///
    /// This is a no-op that just sets the initialized flag.
    pub async fn initialize(&self) -> RlmResult<()> {
        self.log_operation(OperationType::Initialize, None, "initializing", true);
        *self.initialized.write() = true;
        log::info!("MockExecutor initialized (CI/testing mode)");
        Ok(())
    }

    /// Get the operation log for test verification.
    ///
    /// Returns a copy of all operations performed by this executor.
    pub fn get_operation_log(&self) -> Vec<MockOperation> {
        self.operation_log.lock().clone()
    }

    /// Clear the operation log.
    pub fn clear_operation_log(&self) {
        self.operation_log.lock().clear();
    }

    /// Get the count of operations of a specific type.
    pub fn get_operation_count(&self, op_type: OperationType) -> usize {
        self.operation_log
            .lock()
            .iter()
            .filter(|op| op.op_type == op_type)
            .count()
    }

    /// Check if a specific operation was performed.
    pub fn was_operation_performed(&self, op_type: OperationType) -> bool {
        self.get_operation_count(op_type) > 0
    }

    /// Assign a mock VM ID to a session.
    pub fn assign_vm_to_session(&self, session_id: SessionId, vm_id: String) {
        self.session_to_vm.write().insert(session_id, vm_id);
    }

    /// Generate a unique mock VM ID.
    fn generate_vm_id(&self) -> String {
        let id = self.id_counter.fetch_add(1, Ordering::SeqCst);
        format!("mock-vm-{}", id)
    }

    /// Log an operation for test verification.
    fn log_operation(
        &self,
        op_type: OperationType,
        session_id: Option<SessionId>,
        input: &str,
        success: bool,
    ) {
        let op = MockOperation {
            op_type,
            session_id,
            input: input.to_string(),
            success,
        };
        self.operation_log.lock().push(op);
    }

    /// Ensure the executor is initialized.
    fn ensure_initialized(&self) -> RlmResult<()> {
        if !*self.initialized.read() {
            return Err(RlmError::BackendInitFailed {
                backend: "mock".to_string(),
                message: "Executor not initialized".to_string(),
            });
        }
        Ok(())
    }
}

impl Default for MockExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::ExecutionEnvironment for MockExecutor {
    type Error = RlmError;

    async fn execute_code(
        &self,
        code: &str,
        _ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        self.ensure_initialized()?;

        log::debug!("MockExecutor::execute_code ({} bytes)", code.len());

        // Generate stub output based on the code content
        let stdout = format!("[MOCK] Executed Python code ({} bytes)", code.len());
        let stderr = String::new();
        let exit_code = 0;

        self.log_operation(
            OperationType::ExecuteCode,
            None,
            &format!("{} bytes", code.len()),
            true,
        );

        Ok(ExecutionResult {
            stdout,
            stderr,
            exit_code,
            execution_time_ms: 1,
            output_truncated: false,
            output_file_path: None,
            timed_out: false,
            metadata: HashMap::new(),
        })
    }

    async fn execute_command(
        &self,
        cmd: &str,
        _ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        self.ensure_initialized()?;

        log::debug!("MockExecutor::execute_command: {}", cmd);

        // Generate stub output based on the command
        let stdout = format!("[MOCK] Executed: {}", cmd);
        let stderr = String::new();
        let exit_code = 0;

        self.log_operation(OperationType::ExecuteCommand, None, cmd, true);

        Ok(ExecutionResult {
            stdout,
            stderr,
            exit_code,
            execution_time_ms: 1,
            output_truncated: false,
            output_file_path: None,
            timed_out: false,
            metadata: HashMap::new(),
        })
    }

    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error> {
        self.ensure_initialized()?;

        log::debug!("MockExecutor::validate ({} bytes)", input.len());

        // Return valid result (no validation in mock mode)
        self.log_operation(
            OperationType::Validate,
            None,
            &format!("{} bytes", input.len()),
            true,
        );

        Ok(ValidationResult::valid(Vec::new()))
    }

    async fn create_snapshot(
        &self,
        session_id: &SessionId,
        name: &str,
    ) -> Result<SnapshotId, Self::Error> {
        self.ensure_initialized()?;

        log::info!(
            "MockExecutor: Creating snapshot '{}' for session {}",
            name,
            session_id
        );

        // Ensure session has a mock VM assigned
        {
            let mut session_to_vm = self.session_to_vm.write();
            if !session_to_vm.contains_key(session_id) {
                let vm_id = self.generate_vm_id();
                session_to_vm.insert(*session_id, vm_id);
            }
        }

        // Create the snapshot
        let snapshot_id = SnapshotId::new(name, *session_id);

        // Store it
        self.snapshots
            .write()
            .entry(*session_id)
            .or_default()
            .push(snapshot_id.clone());

        self.log_operation(OperationType::CreateSnapshot, Some(*session_id), name, true);

        log::info!(
            "MockExecutor: Created snapshot '{}' for session {}",
            name,
            session_id
        );

        Ok(snapshot_id)
    }

    async fn restore_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        self.ensure_initialized()?;

        log::info!(
            "MockExecutor: Restoring snapshot '{}' for session {}",
            id.name,
            id.session_id
        );

        // Verify snapshot exists
        let snapshots = self.snapshots.read();
        let session_snapshots = snapshots.get(&id.session_id);

        let exists = session_snapshots
            .map(|snaps| snaps.iter().any(|s| s.id == id.id))
            .unwrap_or(false);

        if !exists {
            return Err(RlmError::SnapshotNotFound {
                snapshot_id: id.id.to_string(),
            });
        }

        self.log_operation(
            OperationType::RestoreSnapshot,
            Some(id.session_id),
            &id.name,
            true,
        );

        log::info!(
            "MockExecutor: Restored snapshot '{}' for session {}",
            id.name,
            id.session_id
        );

        Ok(())
    }

    async fn list_snapshots(&self, session_id: &SessionId) -> Result<Vec<SnapshotId>, Self::Error> {
        self.ensure_initialized()?;

        let snapshots = self.snapshots.read();
        let result = snapshots.get(session_id).cloned().unwrap_or_default();

        self.log_operation(
            OperationType::ListSnapshots,
            Some(*session_id),
            &format!("found {} snapshots", result.len()),
            true,
        );

        Ok(result)
    }

    async fn delete_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        self.ensure_initialized()?;

        log::info!(
            "MockExecutor: Deleting snapshot '{}' from session {}",
            id.name,
            id.session_id
        );

        let mut snapshots = self.snapshots.write();
        if let Some(session_snapshots) = snapshots.get_mut(&id.session_id) {
            session_snapshots.retain(|s| s.id != id.id);
        }

        self.log_operation(
            OperationType::DeleteSnapshot,
            Some(id.session_id),
            &id.name,
            true,
        );

        Ok(())
    }

    async fn delete_session_snapshots(&self, session_id: &SessionId) -> Result<(), Self::Error> {
        self.ensure_initialized()?;

        log::info!(
            "MockExecutor: Deleting all snapshots for session {}",
            session_id
        );

        self.snapshots.write().remove(session_id);

        self.log_operation(
            OperationType::DeleteSnapshot,
            Some(*session_id),
            "all session snapshots",
            true,
        );

        Ok(())
    }

    fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Docker // Use Docker as the mock backend type
    }

    async fn health_check(&self) -> Result<bool, Self::Error> {
        let initialized = *self.initialized.read();

        self.log_operation(
            OperationType::HealthCheck,
            None,
            &format!("initialized={}", initialized),
            initialized,
        );

        Ok(initialized)
    }

    async fn cleanup(&self) -> Result<(), Self::Error> {
        log::info!("MockExecutor: Cleaning up");

        self.session_to_vm.write().clear();
        self.snapshots.write().clear();
        *self.initialized.write() = false;

        self.log_operation(OperationType::Cleanup, None, "cleanup complete", true);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::ExecutionEnvironment;
    use super::*;

    #[tokio::test]
    async fn test_mock_executor_health_check() {
        let executor = MockExecutor::new();
        executor.initialize().await.unwrap();

        let health: Result<bool, RlmError> = ExecutionEnvironment::health_check(&executor).await;
        assert!(health.unwrap());
    }

    #[tokio::test]
    async fn test_mock_executor_cleanup() {
        let executor = MockExecutor::new();
        executor.initialize().await.unwrap();

        let result: Result<(), RlmError> = ExecutionEnvironment::cleanup(&executor).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_executor_initialize() {
        let executor = MockExecutor::new();

        // Should not be initialized yet
        assert!(!executor.health_check().await.unwrap());

        // Initialize
        executor.initialize().await.unwrap();

        // Should be initialized now
        assert!(executor.health_check().await.unwrap());

        // Check operation log
        assert!(executor.was_operation_performed(OperationType::Initialize));
        assert!(executor.was_operation_performed(OperationType::HealthCheck));
    }

    #[tokio::test]
    async fn test_mock_executor_execute_code() {
        let executor = MockExecutor::new();
        executor.initialize().await.unwrap();

        let ctx = ExecutionContext::default();
        let result = executor.execute_code("print('hello')", &ctx).await.unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("[MOCK]"));
        assert!(executor.was_operation_performed(OperationType::ExecuteCode));
    }

    #[tokio::test]
    async fn test_mock_executor_snapshots() {
        let executor = MockExecutor::new();
        executor.initialize().await.unwrap();

        let session_id = SessionId::new();

        // Initially no snapshots
        let snapshots: Vec<SnapshotId> = executor.list_snapshots(&session_id).await.unwrap();
        assert!(snapshots.is_empty());

        // Create a snapshot
        let snapshot: SnapshotId = executor
            .create_snapshot(&session_id, "test-snapshot")
            .await
            .unwrap();
        assert_eq!(snapshot.name, "test-snapshot");
        assert_eq!(snapshot.session_id, session_id);

        // Should have one snapshot now
        let snapshots = executor.list_snapshots(&session_id).await.unwrap();
        assert_eq!(snapshots.len(), 1);

        // Restore the snapshot
        executor.restore_snapshot(&snapshot).await.unwrap();

        // Delete the snapshot
        executor.delete_snapshot(&snapshot).await.unwrap();

        // Should be empty again
        let snapshots = executor.list_snapshots(&session_id).await.unwrap();
        assert!(snapshots.is_empty());
    }

    #[tokio::test]
    async fn test_mock_executor_uninitialized() {
        let executor = MockExecutor::new();
        // Don't initialize

        let ctx = ExecutionContext::default();
        let result: Result<ExecutionResult, RlmError> =
            executor.execute_code("print('test')", &ctx).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RlmError::BackendInitFailed { .. }
        ));
    }
}
