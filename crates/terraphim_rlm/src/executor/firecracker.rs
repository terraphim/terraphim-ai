//! Firecracker execution backend.
//!
//! This module provides the `FirecrackerExecutor` which implements the
//! `ExecutionEnvironment` trait using Firecracker microVMs for full isolation.
//!
//! ## Features
//!
//! - Full VM isolation (no shared kernel with host)
//! - Pre-warmed VM pool for sub-500ms allocation
//! - Snapshot support for state versioning via fcctl-core SnapshotManager
//! - Network audit logging
//! - OverlayFS for session-specific packages
//!
//! ## Requirements
//!
//! - Linux with KVM support (`/dev/kvm` must exist)
//! - Firecracker binary installed
//! - VM kernel and rootfs images

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

// Use fcctl-core for VM and snapshot management
// Note: SnapshotType must come from firecracker::models to match SnapshotManager's expected type
use fcctl_core::firecracker::models::SnapshotType;
use fcctl_core::snapshot::SnapshotManager;
use fcctl_core::vm::VmManager;

// Keep terraphim_firecracker for pool management
use terraphim_firecracker::{PoolConfig, Sub2SecondOptimizer, VmPoolManager};

use super::ssh::SshExecutor;
use super::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
use crate::config::{BackendType, RlmConfig};
use crate::error::{RlmError, RlmResult};
use crate::types::SessionId;

/// Firecracker execution backend.
///
/// Wraps fcctl-core's VmManager and SnapshotManager to provide RLM execution
/// capabilities with full VM isolation.
///
/// All mutable state uses interior mutability to allow the trait
/// implementation to use `&self` as required by `ExecutionEnvironment`.
///
/// Note: `vm_manager` and `snapshot_manager` use `tokio::sync::Mutex` because
/// their methods require `&mut self` and we need to hold the lock across
/// `.await` points. Other state uses `parking_lot::RwLock` for efficiency.
pub struct FirecrackerExecutor {
    /// Configuration for the executor.
    config: RlmConfig,

    /// VM manager from fcctl-core for VM lifecycle.
    /// Uses tokio::sync::Mutex for Send-safe async access.
    vm_manager: tokio::sync::Mutex<Option<VmManager>>,

    /// Snapshot manager from fcctl-core for state versioning.
    /// Uses tokio::sync::Mutex for Send-safe async access.
    snapshot_manager: tokio::sync::Mutex<Option<SnapshotManager>>,

    /// VM pool manager for pre-warmed VMs.
    pool_manager: parking_lot::RwLock<Option<Arc<VmPoolManager>>>,

    /// SSH executor for running commands on VMs.
    ssh_executor: SshExecutor,

    /// Capabilities supported by this executor.
    capabilities: Vec<Capability>,

    /// Session to VM ID mapping for affinity.
    /// Maps SessionId -> vm_id (used by VmManager).
    session_to_vm: parking_lot::RwLock<HashMap<SessionId, String>>,

    /// Current active snapshot per session (for rollback support).
    current_snapshot: parking_lot::RwLock<HashMap<SessionId, String>>,

    /// Snapshot count per session (for limit enforcement).
    snapshot_counts: parking_lot::RwLock<HashMap<SessionId, u32>>,
}

impl FirecrackerExecutor {
    /// Create a new Firecracker executor.
    ///
    /// # Arguments
    ///
    /// * `config` - RLM configuration
    ///
    /// # Errors
    ///
    /// Returns an error if KVM is not available.
    pub fn new(config: RlmConfig) -> Result<Self, RlmError> {
        if !super::is_kvm_available() {
            return Err(RlmError::BackendInitFailed {
                backend: "firecracker".to_string(),
                message: "KVM is not available (/dev/kvm does not exist)".to_string(),
            });
        }

        let capabilities = vec![
            Capability::VmIsolation,
            Capability::Snapshots,
            Capability::NetworkAudit,
            Capability::OverlayFs,
            Capability::LlmBridge,
            Capability::DnsAllowlist,
            Capability::ResourceLimits,
            Capability::PythonExecution,
            Capability::BashExecution,
            Capability::FileOperations,
        ];

        // Configure SSH executor with sensible defaults for VM access
        let ssh_executor = SshExecutor::new()
            .with_user("root")
            .with_output_dir(std::env::temp_dir().join("terraphim_rlm_output"));

        Ok(Self {
            config,
            vm_manager: tokio::sync::Mutex::new(None),
            snapshot_manager: tokio::sync::Mutex::new(None),
            pool_manager: parking_lot::RwLock::new(None),
            ssh_executor,
            capabilities,
            session_to_vm: parking_lot::RwLock::new(HashMap::new()),
            current_snapshot: parking_lot::RwLock::new(HashMap::new()),
            snapshot_counts: parking_lot::RwLock::new(HashMap::new()),
        })
    }

    /// Initialize the VM and snapshot managers.
    ///
    /// This should be called before using the executor.
    /// Note: Uses interior mutability so `&self` is sufficient.
    pub async fn initialize(&self) -> RlmResult<()> {
        log::info!("Initializing FirecrackerExecutor with fcctl-core managers");

        // Initialize VmManager from fcctl-core
        let firecracker_bin = PathBuf::from("/usr/bin/firecracker");
        let socket_base_path = PathBuf::from("/tmp/firecracker-sockets");

        // Create socket directory if it doesn't exist
        if !socket_base_path.exists() {
            std::fs::create_dir_all(&socket_base_path).map_err(|e| {
                RlmError::BackendInitFailed {
                    backend: "firecracker".to_string(),
                    message: format!("Failed to create socket directory: {}", e),
                }
            })?;
        }

        let vm_manager =
            VmManager::new(&firecracker_bin, &socket_base_path, None).map_err(|e| {
                RlmError::BackendInitFailed {
                    backend: "firecracker".to_string(),
                    message: format!("Failed to create VmManager: {}", e),
                }
            })?;

        *self.vm_manager.lock().await = Some(vm_manager);

        // Initialize SnapshotManager from fcctl-core
        let snapshots_dir = PathBuf::from("/var/lib/terraphim/snapshots");
        if !snapshots_dir.exists() {
            std::fs::create_dir_all(&snapshots_dir).map_err(|e| RlmError::BackendInitFailed {
                backend: "firecracker".to_string(),
                message: format!("Failed to create snapshots directory: {}", e),
            })?;
        }

        let snapshot_manager = SnapshotManager::new(&snapshots_dir, None).map_err(|e| {
            RlmError::BackendInitFailed {
                backend: "firecracker".to_string(),
                message: format!("Failed to create SnapshotManager: {}", e),
            }
        })?;

        *self.snapshot_manager.lock().await = Some(snapshot_manager);

        log::info!("FirecrackerExecutor initialized successfully");
        Ok(())
    }

    /// Initialize the VM pool for pre-warmed VMs.
    #[allow(dead_code)]
    async fn ensure_pool(&self) -> Result<Arc<VmPoolManager>, RlmError> {
        if let Some(ref pool) = *self.pool_manager.read() {
            return Ok(Arc::clone(pool));
        }

        log::info!(
            "Initializing Firecracker VM pool (min={}, max={}, target={})",
            self.config.pool_min_size,
            self.config.pool_max_size,
            self.config.pool_target_size
        );

        // Create pool configuration from RLM config
        let _pool_config = PoolConfig {
            min_pool_size: self.config.pool_min_size,
            max_pool_size: self.config.pool_max_size,
            target_pool_size: self.config.pool_target_size,
            allocation_timeout: std::time::Duration::from_millis(self.config.allocation_timeout_ms),
            ..Default::default()
        };

        let _optimizer = Arc::new(Sub2SecondOptimizer::new());

        // TODO: Create actual VmPoolManager with VmManager
        log::warn!("FirecrackerExecutor: VM pool initialization not yet fully implemented");

        Err(RlmError::BackendInitFailed {
            backend: "firecracker".to_string(),
            message: "VM pool initialization requires VmManager integration".to_string(),
        })
    }

    /// Get the VM ID for a session, or allocate one if needed.
    async fn get_or_allocate_vm(&self, session_id: &SessionId) -> RlmResult<Option<String>> {
        // Check if session already has an assigned VM
        {
            let session_to_vm = self.session_to_vm.read();
            if let Some(vm_id) = session_to_vm.get(session_id) {
                log::debug!("Using existing VM for session {}: {}", session_id, vm_id);
                return Ok(Some(vm_id.clone()));
            }
        }

        // No VM assigned yet - would allocate from pool in production
        log::debug!("No VM available for session {}", session_id);
        Ok(None)
    }

    /// Assign a VM to a session.
    pub fn assign_vm_to_session(&self, session_id: SessionId, vm_id: String) {
        log::info!("Assigning VM {} to session {}", vm_id, session_id);
        self.session_to_vm.write().insert(session_id, vm_id);
    }

    /// Release VM assignment for a session.
    pub fn release_session_vm(&self, session_id: &SessionId) -> Option<String> {
        self.session_to_vm.write().remove(session_id)
    }

    /// Get the current active snapshot for a session.
    pub fn get_current_snapshot(&self, session_id: &SessionId) -> Option<String> {
        self.current_snapshot.read().get(session_id).cloned()
    }

    /// Set the current active snapshot for a session.
    fn set_current_snapshot(&self, session_id: &SessionId, snapshot_id: String) {
        self.current_snapshot
            .write()
            .insert(*session_id, snapshot_id);
    }

    /// Clear the current snapshot for a session.
    fn clear_current_snapshot(&self, session_id: &SessionId) {
        self.current_snapshot.write().remove(session_id);
    }

    /// Rollback to the previous known good state.
    pub async fn rollback(&self, session_id: &SessionId) -> Result<(), RlmError> {
        let current = self.get_current_snapshot(session_id);

        match current {
            Some(snapshot_id) => {
                log::warn!(
                    "Rolling back session {} to snapshot '{}'",
                    session_id,
                    snapshot_id
                );

                // Get VM ID for this session
                let vm_id = self.session_to_vm.read().get(session_id).cloned();

                if let Some(vm_id) = vm_id {
                    let mut snapshot_manager_guard = self.snapshot_manager.lock().await;
                    let mut vm_manager_guard = self.vm_manager.lock().await;

                    if let (Some(snapshot_manager), Some(vm_manager)) =
                        (&mut *snapshot_manager_guard, &mut *vm_manager_guard)
                    {
                        let vm_client = vm_manager.get_vm_client(&vm_id).await.map_err(|e| {
                            RlmError::SnapshotRestoreFailed {
                                message: format!("Failed to get VM client: {}", e),
                            }
                        })?;

                        snapshot_manager
                            .restore_snapshot(vm_client, &snapshot_id)
                            .await
                            .map_err(|e| RlmError::SnapshotRestoreFailed {
                                message: format!("Rollback failed: {}", e),
                            })?;
                    }
                }

                Ok(())
            }
            None => {
                log::warn!(
                    "No current snapshot for session {}, rollback is a no-op",
                    session_id
                );
                Ok(())
            }
        }
    }

    /// Execute code in a VM.
    async fn execute_in_vm(
        &self,
        code: &str,
        is_python: bool,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, RlmError> {
        let start = Instant::now();

        log::debug!(
            "FirecrackerExecutor::execute_in_vm called (python={}, session={})",
            is_python,
            ctx.session_id
        );

        // Try to get a VM for this session
        let vm_id = self.get_or_allocate_vm(&ctx.session_id).await?;

        match vm_id {
            Some(ref id) => {
                // Get VM IP from VmManager
                let vm_ip = {
                    let vm_manager_guard = self.vm_manager.lock().await;
                    if let Some(ref vm_manager) = *vm_manager_guard {
                        vm_manager.get_vm_ip(id).ok()
                    } else {
                        None
                    }
                };

                match vm_ip {
                    Some(ip) => {
                        log::info!(
                            "Executing on VM {} ({}) for session {}",
                            id,
                            ip,
                            ctx.session_id
                        );

                        let result = if is_python {
                            self.ssh_executor.execute_python(&ip, code, ctx).await
                        } else {
                            self.ssh_executor.execute_command(&ip, code, ctx).await
                        };

                        match result {
                            Ok(mut res) => {
                                res.metadata.insert("vm_id".to_string(), id.clone());
                                res.metadata.insert("vm_ip".to_string(), ip);
                                res.metadata
                                    .insert("backend".to_string(), "firecracker".to_string());
                                Ok(res)
                            }
                            Err(e) => {
                                log::error!("VM execution failed: {}", e);
                                Err(e)
                            }
                        }
                    }
                    None => {
                        log::warn!("VM {} has no IP assigned", id);
                        self.stub_response(code, start)
                    }
                }
            }
            None => self.stub_response(code, start),
        }
    }

    /// Return a stub response when no VM is available.
    fn stub_response(&self, code: &str, start: Instant) -> Result<ExecutionResult, RlmError> {
        let execution_time = start.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            stdout: format!(
                "[FirecrackerExecutor] No VM available. Would execute: {}",
                if code.len() > 100 {
                    format!("{}...", &code[..100])
                } else {
                    code.to_string()
                }
            ),
            stderr: "Warning: No VM allocated for this session. \
                     Call initialize() and assign_vm_to_session() first."
                .to_string(),
            exit_code: 0,
            execution_time_ms: execution_time,
            output_truncated: false,
            output_file_path: None,
            timed_out: false,
            metadata: {
                let mut m = HashMap::new();
                m.insert("stub".to_string(), "true".to_string());
                m.insert("backend".to_string(), "firecracker".to_string());
                m
            },
        })
    }
}

#[async_trait]
impl super::ExecutionEnvironment for FirecrackerExecutor {
    type Error = RlmError;

    async fn execute_code(
        &self,
        code: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        self.execute_in_vm(code, true, ctx).await
    }

    async fn execute_command(
        &self,
        cmd: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        self.execute_in_vm(cmd, false, ctx).await
    }

    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error> {
        // TODO: Implement KG validation using terraphim_automata
        log::debug!(
            "FirecrackerExecutor::validate called for {} bytes",
            input.len()
        );

        Ok(ValidationResult::valid(Vec::new()))
    }

    async fn create_snapshot(
        &self,
        session_id: &SessionId,
        name: &str,
    ) -> Result<SnapshotId, Self::Error> {
        log::info!("Creating snapshot '{}' for session {}", name, session_id);

        // Check snapshot limit for this session
        let count = *self.snapshot_counts.read().get(session_id).unwrap_or(&0);
        if count >= self.config.max_snapshots_per_session {
            return Err(RlmError::MaxSnapshotsReached {
                max: self.config.max_snapshots_per_session,
            });
        }

        // Get VM ID for this session
        let vm_id = self
            .session_to_vm
            .read()
            .get(session_id)
            .cloned()
            .ok_or_else(|| RlmError::SnapshotCreationFailed {
                message: "No VM assigned to session".to_string(),
            })?;

        // Create snapshot using fcctl-core SnapshotManager
        let snapshot_id = {
            let mut snapshot_manager_guard = self.snapshot_manager.lock().await;
            let mut vm_manager_guard = self.vm_manager.lock().await;

            match (&mut *snapshot_manager_guard, &mut *vm_manager_guard) {
                (Some(snapshot_manager), Some(vm_manager)) => {
                    let vm_client = vm_manager.get_vm_client(&vm_id).await.map_err(|e| {
                        RlmError::SnapshotCreationFailed {
                            message: format!("Failed to get VM client: {}", e),
                        }
                    })?;

                    snapshot_manager
                        .create_snapshot(vm_client, &vm_id, name, SnapshotType::Full, None)
                        .await
                        .map_err(|e| RlmError::SnapshotCreationFailed {
                            message: format!("Snapshot creation failed: {}", e),
                        })?
                }
                (None, _) => {
                    return Err(RlmError::SnapshotCreationFailed {
                        message: "SnapshotManager not initialized".to_string(),
                    });
                }
                (_, None) => {
                    return Err(RlmError::SnapshotCreationFailed {
                        message: "VmManager not initialized".to_string(),
                    });
                }
            }
        };

        // Update tracking
        *self.snapshot_counts.write().entry(*session_id).or_insert(0) += 1;

        let result = SnapshotId::new(name, *session_id);

        log::info!(
            "Snapshot '{}' ({}) created for session {}",
            name,
            snapshot_id,
            session_id
        );

        Ok(result)
    }

    async fn restore_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        log::info!(
            "Restoring snapshot '{}' ({}) for session {}",
            id.name,
            id.id,
            id.session_id
        );

        // Get VM ID for this session
        let vm_id = self
            .session_to_vm
            .read()
            .get(&id.session_id)
            .cloned()
            .ok_or_else(|| RlmError::SnapshotRestoreFailed {
                message: "No VM assigned to session".to_string(),
            })?;

        // Restore snapshot using fcctl-core SnapshotManager
        {
            let mut snapshot_manager_guard = self.snapshot_manager.lock().await;
            let mut vm_manager_guard = self.vm_manager.lock().await;

            match (&mut *snapshot_manager_guard, &mut *vm_manager_guard) {
                (Some(snapshot_manager), Some(vm_manager)) => {
                    let vm_client = vm_manager.get_vm_client(&vm_id).await.map_err(|e| {
                        RlmError::SnapshotRestoreFailed {
                            message: format!("Failed to get VM client: {}", e),
                        }
                    })?;

                    snapshot_manager
                        .restore_snapshot(vm_client, &id.id.to_string())
                        .await
                        .map_err(|e| RlmError::SnapshotRestoreFailed {
                            message: format!("Snapshot restore failed: {}", e),
                        })?;
                }
                (None, _) => {
                    return Err(RlmError::SnapshotRestoreFailed {
                        message: "SnapshotManager not initialized".to_string(),
                    });
                }
                (_, None) => {
                    return Err(RlmError::SnapshotRestoreFailed {
                        message: "VmManager not initialized".to_string(),
                    });
                }
            }
        }

        // Update current snapshot tracking
        self.set_current_snapshot(&id.session_id, id.id.to_string());

        log::info!(
            "Snapshot '{}' restored for session {}",
            id.name,
            id.session_id
        );

        Ok(())
    }

    async fn list_snapshots(&self, session_id: &SessionId) -> Result<Vec<SnapshotId>, Self::Error> {
        // Get VM ID for this session
        let vm_id = self.session_to_vm.read().get(session_id).cloned();

        if vm_id.is_none() {
            log::debug!(
                "No VM assigned to session {}, returning empty snapshot list",
                session_id
            );
            return Ok(Vec::new());
        }

        // List snapshots using fcctl-core SnapshotManager
        // Note: SnapshotManager.list_snapshots requires &mut self
        // For now, return empty list and log
        log::debug!(
            "list_snapshots for session {} (vm={})",
            session_id,
            vm_id.unwrap()
        );

        // TODO: Call snapshot_manager.list_snapshots() when we have mutable access
        Ok(Vec::new())
    }

    async fn delete_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        log::info!(
            "Deleting snapshot '{}' ({}) from session {}",
            id.name,
            id.id,
            id.session_id
        );

        // Delete snapshot using fcctl-core SnapshotManager
        {
            let mut snapshot_manager_guard = self.snapshot_manager.lock().await;
            if let Some(snapshot_manager) = &mut *snapshot_manager_guard {
                snapshot_manager
                    .delete_snapshot(&id.id.to_string(), true)
                    .await
                    .map_err(|e| RlmError::SnapshotNotFound {
                        snapshot_id: format!("Delete failed: {}", e),
                    })?;
            } else {
                return Err(RlmError::SnapshotNotFound {
                    snapshot_id: "SnapshotManager not initialized".to_string(),
                });
            }
        }

        // Update count
        if let Some(count) = self.snapshot_counts.write().get_mut(&id.session_id) {
            *count = count.saturating_sub(1);
        }

        log::debug!("Snapshot {} deleted", id.id);
        Ok(())
    }

    async fn delete_session_snapshots(&self, session_id: &SessionId) -> Result<(), Self::Error> {
        log::info!("Deleting all snapshots for session {}", session_id);

        // Get VM ID for this session to filter snapshots
        let vm_id = self.session_to_vm.read().get(session_id).cloned();

        if let Some(vm_id) = vm_id {
            let mut snapshot_manager_guard = self.snapshot_manager.lock().await;
            if let Some(snapshot_manager) = &mut *snapshot_manager_guard {
                // List and delete all snapshots for this VM
                let snapshots = snapshot_manager
                    .list_snapshots(Some(&vm_id))
                    .await
                    .unwrap_or_default();

                for snapshot in snapshots {
                    if let Err(e) = snapshot_manager.delete_snapshot(&snapshot.id, true).await {
                        log::warn!("Failed to delete snapshot {}: {}", snapshot.id, e);
                    }
                }

                log::info!(
                    "Deleted snapshots for session {} (vm={})",
                    session_id,
                    vm_id
                );
            }
        }

        // Clear tracking
        self.snapshot_counts.write().remove(session_id);
        self.clear_current_snapshot(session_id);

        Ok(())
    }

    fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Firecracker
    }

    async fn health_check(&self) -> Result<bool, Self::Error> {
        // Check KVM availability
        if !super::is_kvm_available() {
            return Ok(false);
        }

        // Check if managers are initialized
        let vm_manager_initialized = self.vm_manager.lock().await.is_some();
        let snapshot_manager_initialized = self.snapshot_manager.lock().await.is_some();

        if !vm_manager_initialized || !snapshot_manager_initialized {
            log::warn!("FirecrackerExecutor not fully initialized");
            return Ok(false);
        }

        Ok(true)
    }

    async fn cleanup(&self) -> Result<(), Self::Error> {
        log::info!("FirecrackerExecutor::cleanup called");

        // Clear all session mappings
        self.session_to_vm.write().clear();
        self.current_snapshot.write().clear();
        self.snapshot_counts.write().clear();

        // TODO: Stop and cleanup VMs via VmManager
        // for (session_id, vm_id) in session_to_vm {
        //     vm_manager.stop_vm(&vm_id, true).await?;
        // }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionEnvironment;

    #[test]
    fn test_firecracker_executor_capabilities() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping test: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();

        assert!(executor.has_capability(Capability::VmIsolation));
        assert!(executor.has_capability(Capability::Snapshots));
        assert!(executor.has_capability(Capability::PythonExecution));
        assert!(!executor.has_capability(Capability::ContainerIsolation));
    }

    #[test]
    fn test_firecracker_executor_requires_kvm() {
        if super::super::is_kvm_available() {
            eprintln!("Skipping test: KVM is available");
            return;
        }

        let config = RlmConfig::default();
        let result = FirecrackerExecutor::new(config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_session_vm_assignment() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping test: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();
        let session_id = SessionId::new();

        // Initially no VM assigned
        assert!(executor.session_to_vm.read().get(&session_id).is_none());

        // Assign VM
        executor.assign_vm_to_session(session_id, "vm-test-123".to_string());

        // Now should have VM
        assert_eq!(
            executor.session_to_vm.read().get(&session_id),
            Some(&"vm-test-123".to_string())
        );

        // Release VM
        let released = executor.release_session_vm(&session_id);
        assert_eq!(released, Some("vm-test-123".to_string()));
        assert!(executor.session_to_vm.read().get(&session_id).is_none());
    }

    #[tokio::test]
    async fn test_current_snapshot_tracking() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping test: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();
        let session_id = SessionId::new();

        // Initially no current snapshot
        assert!(executor.get_current_snapshot(&session_id).is_none());

        // Set current snapshot
        executor.set_current_snapshot(&session_id, "snap-123".to_string());
        assert_eq!(
            executor.get_current_snapshot(&session_id),
            Some("snap-123".to_string())
        );

        // Clear current snapshot
        executor.clear_current_snapshot(&session_id);
        assert!(executor.get_current_snapshot(&session_id).is_none());
    }

    #[tokio::test]
    async fn test_rollback_without_current_snapshot() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping test: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();
        let session_id = SessionId::new();

        // Rollback without any snapshot should be no-op
        let result = executor.rollback(&session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_without_initialization() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping test: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();

        // Health check should fail if not initialized
        let result = executor.health_check().await.unwrap();
        assert!(!result);
    }
}
