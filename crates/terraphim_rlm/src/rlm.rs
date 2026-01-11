//! TerraphimRlm - the main public API for RLM orchestration.
//!
//! This module provides the primary interface for executing LLM-generated code
//! in isolated environments with session management, budget tracking, and
//! knowledge graph validation.
//!
//! ## Example
//!
//! ```rust,ignore
//! use terraphim_rlm::{TerraphimRlm, RlmConfig};
//!
//! // Create RLM instance with default config
//! let rlm = TerraphimRlm::new(RlmConfig::default()).await?;
//!
//! // Create a session for code execution
//! let session = rlm.create_session().await?;
//!
//! // Execute Python code
//! let result = rlm.execute_code(&session.id, "print('Hello, RLM!')").await?;
//! println!("Output: {}", result.stdout);
//!
//! // Execute a full query with the RLM loop
//! let query_result = rlm.query(&session.id, "Calculate the first 10 fibonacci numbers").await?;
//! println!("Result: {:?}", query_result.result);
//!
//! // Create a snapshot for rollback
//! let snapshot = rlm.create_snapshot(&session.id, "checkpoint_1").await?;
//!
//! // Clean up
//! rlm.destroy_session(&session.id).await?;
//! ```

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::budget::BudgetTracker;
use crate::config::RlmConfig;
use crate::error::{RlmError, RlmResult};
use crate::executor::{
    ExecutionContext, ExecutionEnvironment, ExecutionResult, SnapshotId, select_executor,
};
use crate::llm_bridge::{LlmBridge, LlmBridgeConfig};
// CommandParser and TerminationReason are used internally by QueryLoop
use crate::query_loop::{QueryLoop, QueryLoopConfig, QueryLoopResult};
use crate::session::SessionManager;
use crate::types::{SessionId, SessionInfo};

/// The main RLM orchestrator.
///
/// `TerraphimRlm` is the primary public API for the RLM system. It manages:
/// - Session lifecycle (create, destroy, extend)
/// - Code and command execution in isolated VMs
/// - Query loop orchestration (LLM → parse → execute → feedback)
/// - Snapshot and rollback capabilities
/// - Budget tracking (tokens, time, recursion depth)
pub struct TerraphimRlm {
    /// Configuration for the RLM system.
    config: RlmConfig,
    /// Session manager for session state and VM affinity.
    session_manager: Arc<SessionManager>,
    /// LLM bridge for VM-to-host LLM calls.
    llm_bridge: Arc<LlmBridge>,
    /// The execution environment (Firecracker, Docker, or E2B).
    executor: Arc<dyn ExecutionEnvironment<Error = RlmError> + Send + Sync>,
    /// Cancellation senders for active queries, keyed by session ID.
    cancel_senders: dashmap::DashMap<SessionId, mpsc::Sender<()>>,
}

impl TerraphimRlm {
    /// Create a new TerraphimRlm instance.
    ///
    /// This initializes the execution backend, session manager, and LLM bridge.
    /// Backend selection follows the preference order in config, falling back
    /// to available alternatives.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the RLM system
    ///
    /// # Returns
    ///
    /// A new `TerraphimRlm` instance or an error if no backend is available.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = RlmConfig::default();
    /// let rlm = TerraphimRlm::new(config).await?;
    /// ```
    pub async fn new(config: RlmConfig) -> RlmResult<Self> {
        // Validate configuration
        config
            .validate()
            .map_err(|msg| RlmError::ConfigError { message: msg })?;

        // Select and initialize execution backend
        let executor = select_executor(&config).await?;

        // Create session manager
        let session_manager = Arc::new(SessionManager::new(config.clone()));

        // Create LLM bridge
        let llm_bridge_config = LlmBridgeConfig::default();
        let llm_bridge = Arc::new(LlmBridge::new(llm_bridge_config, session_manager.clone()));

        Ok(Self {
            config,
            session_manager,
            llm_bridge,
            executor: Arc::from(executor),
            cancel_senders: dashmap::DashMap::new(),
        })
    }

    /// Create a new TerraphimRlm with a custom executor.
    ///
    /// This is useful for testing or when you need to inject a specific
    /// execution backend.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the RLM system
    /// * `executor` - The execution environment to use
    pub fn with_executor<E>(config: RlmConfig, executor: E) -> RlmResult<Self>
    where
        E: ExecutionEnvironment<Error = RlmError> + Send + Sync + 'static,
    {
        config
            .validate()
            .map_err(|msg| RlmError::ConfigError { message: msg })?;

        let session_manager = Arc::new(SessionManager::new(config.clone()));
        let llm_bridge_config = LlmBridgeConfig::default();
        let llm_bridge = Arc::new(LlmBridge::new(llm_bridge_config, session_manager.clone()));

        // Cast to dyn ExecutionEnvironment for type erasure
        let executor: Arc<dyn ExecutionEnvironment<Error = RlmError> + Send + Sync> =
            Arc::new(executor);

        Ok(Self {
            config,
            session_manager,
            llm_bridge,
            executor,
            cancel_senders: dashmap::DashMap::new(),
        })
    }

    // ========================================================================
    // Session Management
    // ========================================================================

    /// Create a new session.
    ///
    /// A session represents an isolated execution context with its own VM,
    /// budget tracking, and state. Sessions have a default duration and can
    /// be extended up to a maximum number of times.
    ///
    /// # Returns
    ///
    /// Information about the newly created session.
    pub async fn create_session(&self) -> RlmResult<SessionInfo> {
        let session = self.session_manager.create_session()?;
        log::info!("Created session: {}", session.id);
        Ok(session)
    }

    /// Destroy a session.
    ///
    /// This releases all resources associated with the session, including
    /// the VM, snapshots, and budget tracker.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to destroy
    pub async fn destroy_session(&self, session_id: &SessionId) -> RlmResult<()> {
        // Cancel any active query for this session
        if let Some((_, sender)) = self.cancel_senders.remove(session_id) {
            let _ = sender.send(()).await;
        }

        // Destroy the session
        self.session_manager.destroy_session(session_id)?;
        log::info!("Destroyed session: {}", session_id);
        Ok(())
    }

    /// Get information about a session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to query
    ///
    /// # Returns
    ///
    /// Current session information including state, budget, and VM assignment.
    pub fn get_session(&self, session_id: &SessionId) -> RlmResult<SessionInfo> {
        self.session_manager.get_session(session_id)
    }

    /// Extend a session's duration.
    ///
    /// Adds time to the session's expiration. Sessions can only be extended
    /// up to `max_extensions` times (default: 3).
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to extend
    ///
    /// # Returns
    ///
    /// Updated session information with new expiration time.
    pub fn extend_session(&self, session_id: &SessionId) -> RlmResult<SessionInfo> {
        self.session_manager.extend_session(session_id)
    }

    /// Set a context variable in the session.
    ///
    /// Context variables persist for the lifetime of the session and can
    /// be accessed by LLM-generated code via `FINAL_VAR(variable_name)`.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to modify
    /// * `key` - Variable name
    /// * `value` - Variable value
    pub fn set_context_variable(
        &self,
        session_id: &SessionId,
        key: &str,
        value: &str,
    ) -> RlmResult<()> {
        self.session_manager
            .set_context_variable(session_id, key.to_string(), value.to_string())
    }

    /// Get a context variable from the session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to query
    /// * `key` - Variable name
    ///
    /// # Returns
    ///
    /// The variable value if it exists.
    pub fn get_context_variable(
        &self,
        session_id: &SessionId,
        key: &str,
    ) -> RlmResult<Option<String>> {
        self.session_manager.get_context_variable(session_id, key)
    }

    // ========================================================================
    // Code Execution
    // ========================================================================

    /// Execute Python code in the session's VM.
    ///
    /// This is a direct execution without the query loop. The code runs
    /// in the session's isolated VM and returns the output.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to execute in
    /// * `code` - Python code to execute
    ///
    /// # Returns
    ///
    /// Execution result with stdout, stderr, and exit code.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = rlm.execute_code(&session.id, r#"
    ///     import math
    ///     print(f"Pi = {math.pi}")
    /// "#).await?;
    /// assert!(result.stdout.contains("Pi = 3.14"));
    /// ```
    pub async fn execute_code(
        &self,
        session_id: &SessionId,
        code: &str,
    ) -> RlmResult<ExecutionResult> {
        // Validate session
        self.session_manager.validate_session(session_id)?;

        // Build execution context
        let ctx = ExecutionContext {
            session_id: *session_id,
            timeout_ms: self.config.time_budget_ms,
            ..Default::default()
        };

        // Execute code
        self.executor
            .execute_code(code, &ctx)
            .await
            .map_err(|e| RlmError::ExecutionFailed {
                message: e.to_string(),
                exit_code: None,
                stdout: None,
                stderr: None,
            })
    }

    /// Execute a bash command in the session's VM.
    ///
    /// This is a direct execution without the query loop. The command runs
    /// in the session's isolated VM and returns the output.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to execute in
    /// * `command` - Bash command to execute
    ///
    /// # Returns
    ///
    /// Execution result with stdout, stderr, and exit code.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = rlm.execute_command(&session.id, "ls -la /").await?;
    /// println!("Files: {}", result.stdout);
    /// ```
    pub async fn execute_command(
        &self,
        session_id: &SessionId,
        command: &str,
    ) -> RlmResult<ExecutionResult> {
        // Validate session
        self.session_manager.validate_session(session_id)?;

        // Build execution context
        let ctx = ExecutionContext {
            session_id: *session_id,
            timeout_ms: self.config.time_budget_ms,
            ..Default::default()
        };

        // Execute command
        self.executor
            .execute_command(command, &ctx)
            .await
            .map_err(|e| RlmError::ExecutionFailed {
                message: e.to_string(),
                exit_code: None,
                stdout: None,
                stderr: None,
            })
    }

    // ========================================================================
    // Query Loop
    // ========================================================================

    /// Execute a full RLM query.
    ///
    /// This runs the complete query loop:
    /// 1. Send prompt to LLM
    /// 2. Parse command from LLM response
    /// 3. Execute command in VM
    /// 4. Feed result back to LLM
    /// 5. Repeat until FINAL or budget exhaustion
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to execute in
    /// * `prompt` - Initial prompt/query for the LLM
    ///
    /// # Returns
    ///
    /// Query result including the final answer, termination reason, and history.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = rlm.query(&session.id, "Write a Python function to calculate factorial").await?;
    /// match result.termination_reason {
    ///     TerminationReason::FinalReached => {
    ///         println!("Result: {}", result.result.unwrap());
    ///     }
    ///     TerminationReason::TokenBudgetExhausted => {
    ///         println!("Ran out of tokens!");
    ///     }
    ///     _ => {}
    /// }
    /// ```
    pub async fn query(&self, session_id: &SessionId, prompt: &str) -> RlmResult<QueryLoopResult> {
        // Validate session
        self.session_manager.validate_session(session_id)?;

        // Create budget tracker for this query
        let budget = Arc::new(BudgetTracker::new(&self.config));

        // Create cancellation channel
        let (cancel_tx, cancel_rx) = mpsc::channel(1);
        self.cancel_senders.insert(*session_id, cancel_tx);

        // Build query loop config
        let loop_config = QueryLoopConfig {
            max_iterations: self.config.max_iterations,
            allow_recursion: true,
            max_recursion_depth: self.config.max_recursion_depth,
            strict_parsing: false,
            command_timeout_ms: self.config.time_budget_ms / 10, // Per-command timeout
        };

        // Create and execute query loop
        let mut query_loop = QueryLoop::new(
            *session_id,
            self.session_manager.clone(),
            budget,
            self.llm_bridge.clone(),
            self.executor.clone(),
            loop_config,
        )
        .with_cancel_channel(cancel_rx);

        let result = query_loop.execute(prompt).await;

        // Clean up cancellation sender
        self.cancel_senders.remove(session_id);

        result
    }

    /// Cancel an active query for a session.
    ///
    /// This sends a cancellation signal to the query loop, which will
    /// terminate gracefully at the next checkpoint.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session with the active query to cancel
    pub async fn cancel_query(&self, session_id: &SessionId) -> RlmResult<()> {
        if let Some((_, sender)) = self.cancel_senders.remove(session_id) {
            sender.send(()).await.map_err(|_| RlmError::Internal {
                message: "Failed to send cancellation signal".to_string(),
            })?;
            log::info!("Cancelled query for session: {}", session_id);
        }
        Ok(())
    }

    // ========================================================================
    // Snapshots
    // ========================================================================

    /// Create a named snapshot of the session's VM state.
    ///
    /// Snapshots capture the full VM state including filesystem, memory,
    /// and running processes. They can be used to rollback to a known
    /// good state.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to snapshot
    /// * `name` - Name for the snapshot (for later reference)
    ///
    /// # Returns
    ///
    /// Information about the created snapshot.
    pub async fn create_snapshot(
        &self,
        session_id: &SessionId,
        name: &str,
    ) -> RlmResult<SnapshotId> {
        // Validate session
        self.session_manager.validate_session(session_id)?;

        // Check snapshot limit
        let session = self.session_manager.get_session(session_id)?;
        if session.snapshot_count >= self.config.max_snapshots_per_session {
            return Err(RlmError::MaxSnapshotsReached {
                max: self.config.max_snapshots_per_session,
            });
        }

        // Create snapshot
        let snapshot_id = self
            .executor
            .create_snapshot(session_id, name)
            .await
            .map_err(|e| RlmError::SnapshotCreationFailed {
                message: e.to_string(),
            })?;

        // Record snapshot creation in session manager
        self.session_manager
            .record_snapshot_created(session_id, snapshot_id.name.clone(), true)?;

        log::info!(
            "Created snapshot '{}' for session {}",
            snapshot_id.name,
            session_id
        );
        Ok(snapshot_id)
    }

    /// Restore a session's VM to a previous snapshot.
    ///
    /// This rolls back the VM to the exact state at the time of the snapshot.
    /// Note that external state (e.g., network requests made) cannot be undone.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to restore
    /// * `snapshot_name` - Name of the snapshot to restore
    pub async fn restore_snapshot(
        &self,
        session_id: &SessionId,
        snapshot_name: &str,
    ) -> RlmResult<()> {
        // Validate session
        self.session_manager.validate_session(session_id)?;

        // Find snapshot by name
        let snapshots = self
            .executor
            .list_snapshots(session_id)
            .await
            .map_err(|e| RlmError::SnapshotRestoreFailed {
                message: e.to_string(),
            })?;

        let snapshot = snapshots
            .iter()
            .find(|s| s.name == snapshot_name)
            .ok_or_else(|| RlmError::SnapshotNotFound {
                snapshot_id: snapshot_name.to_string(),
            })?;

        // Restore snapshot
        self.executor
            .restore_snapshot(snapshot)
            .await
            .map_err(|e| RlmError::SnapshotRestoreFailed {
                message: e.to_string(),
            })?;

        // Record snapshot restoration in session manager
        self.session_manager
            .record_snapshot_restored(session_id, snapshot_name.to_string())?;

        log::info!(
            "Restored session {} to snapshot '{}'",
            session_id,
            snapshot_name
        );
        Ok(())
    }

    /// List all snapshots for a session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session to query
    ///
    /// # Returns
    ///
    /// List of snapshot information.
    pub async fn list_snapshots(&self, session_id: &SessionId) -> RlmResult<Vec<SnapshotId>> {
        self.session_manager.validate_session(session_id)?;

        self.executor
            .list_snapshots(session_id)
            .await
            .map_err(|e| RlmError::Internal {
                message: format!("Failed to list snapshots: {}", e),
            })
    }

    // ========================================================================
    // Status and Metrics
    // ========================================================================

    /// Get statistics about all sessions.
    pub fn get_stats(&self) -> crate::session::SessionStats {
        self.session_manager.get_stats()
    }

    /// Get the current configuration.
    pub fn config(&self) -> &RlmConfig {
        &self.config
    }

    /// Check if the execution backend is healthy.
    pub async fn health_check(&self) -> RlmResult<bool> {
        self.executor
            .health_check()
            .await
            .map_err(|e| RlmError::Internal {
                message: format!("Health check failed: {}", e),
            })
    }

    /// Get the version of the RLM crate.
    pub fn version() -> &'static str {
        crate::VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BackendType;
    use crate::executor::{Capability, ValidationResult};
    use crate::types::SessionState;
    use async_trait::async_trait;

    /// Mock executor for testing
    struct MockExecutor {
        capabilities: Vec<Capability>,
    }

    impl MockExecutor {
        fn new() -> Self {
            Self {
                capabilities: vec![Capability::PythonExecution, Capability::BashExecution],
            }
        }
    }

    #[async_trait]
    impl ExecutionEnvironment for MockExecutor {
        type Error = RlmError;

        async fn execute_code(
            &self,
            code: &str,
            _ctx: &ExecutionContext,
        ) -> Result<ExecutionResult, Self::Error> {
            Ok(ExecutionResult::success(format!("Executed: {}", code)))
        }

        async fn execute_command(
            &self,
            command: &str,
            _ctx: &ExecutionContext,
        ) -> Result<ExecutionResult, Self::Error> {
            Ok(ExecutionResult::success(format!("Ran: {}", command)))
        }

        async fn validate(&self, _input: &str) -> Result<ValidationResult, Self::Error> {
            Ok(ValidationResult::valid(vec![]))
        }

        async fn create_snapshot(
            &self,
            session_id: &SessionId,
            name: &str,
        ) -> Result<SnapshotId, Self::Error> {
            Ok(SnapshotId::new(name, *session_id))
        }

        async fn restore_snapshot(&self, _snapshot: &SnapshotId) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn list_snapshots(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<SnapshotId>, Self::Error> {
            Ok(vec![])
        }

        async fn delete_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn delete_session_snapshots(
            &self,
            _session_id: &SessionId,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn capabilities(&self) -> &[Capability] {
            &self.capabilities
        }

        fn backend_type(&self) -> BackendType {
            BackendType::Docker // Mock uses Docker as backend type
        }

        async fn health_check(&self) -> Result<bool, Self::Error> {
            Ok(true)
        }

        async fn cleanup(&self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn test_rlm_with_mock_executor() {
        let config = RlmConfig::minimal();
        let _rlm = TerraphimRlm::with_executor(config, MockExecutor::new()).unwrap();
        // Just test creation - health_check is async so we test creation only
        assert_eq!(TerraphimRlm::version(), crate::VERSION);
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let config = RlmConfig::minimal();
        let rlm = TerraphimRlm::with_executor(config, MockExecutor::new()).unwrap();

        // Create session (starts in Initializing state)
        let session = rlm.create_session().await.unwrap();
        assert_eq!(session.state, SessionState::Initializing);

        // Get session
        let retrieved = rlm.get_session(&session.id).unwrap();
        assert_eq!(retrieved.id, session.id);

        // Set and get context variable
        rlm.set_context_variable(&session.id, "test_key", "test_value")
            .unwrap();
        let value = rlm.get_context_variable(&session.id, "test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Destroy session
        rlm.destroy_session(&session.id).await.unwrap();
        assert!(rlm.get_session(&session.id).is_err());
    }

    #[tokio::test]
    async fn test_execute_code() {
        let config = RlmConfig::minimal();
        let rlm = TerraphimRlm::with_executor(config, MockExecutor::new()).unwrap();

        let session = rlm.create_session().await.unwrap();
        let result = rlm
            .execute_code(&session.id, "print('hello')")
            .await
            .unwrap();

        assert!(result.stdout.contains("Executed"));
        assert_eq!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_execute_command() {
        let config = RlmConfig::minimal();
        let rlm = TerraphimRlm::with_executor(config, MockExecutor::new()).unwrap();

        let session = rlm.create_session().await.unwrap();
        let result = rlm.execute_command(&session.id, "ls -la").await.unwrap();

        assert!(result.stdout.contains("Ran"));
        assert_eq!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_snapshots() {
        let config = RlmConfig::minimal();
        let rlm = TerraphimRlm::with_executor(config, MockExecutor::new()).unwrap();

        let session = rlm.create_session().await.unwrap();

        // Create snapshot
        let snapshot = rlm
            .create_snapshot(&session.id, "test_snapshot")
            .await
            .unwrap();
        assert_eq!(snapshot.name, "test_snapshot");

        // List snapshots (mock returns empty)
        let snapshots = rlm.list_snapshots(&session.id).await.unwrap();
        assert!(snapshots.is_empty()); // Mock returns empty list
    }

    #[tokio::test]
    async fn test_session_extension() {
        let config = RlmConfig::minimal();
        let rlm = TerraphimRlm::with_executor(config, MockExecutor::new()).unwrap();

        let session = rlm.create_session().await.unwrap();
        let original_expiry = session.expires_at;

        let extended = rlm.extend_session(&session.id).unwrap();
        assert!(extended.expires_at > original_expiry);
        assert_eq!(extended.extension_count, 1);
    }

    #[test]
    fn test_version() {
        let version = TerraphimRlm::version();
        assert!(!version.is_empty());
    }
}
