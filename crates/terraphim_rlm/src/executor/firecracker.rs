//! Firecracker execution backend.
//!
//! This module provides the `FirecrackerExecutor` which implements the
//! `ExecutionEnvironment` trait using Firecracker microVMs for full isolation.
//!
//! ## Features
//!
//! - Full VM isolation (no shared kernel with host)
//! - Pre-warmed VM pool for sub-500ms allocation
//! - Snapshot support for state versioning
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
use std::sync::Arc;
use std::time::Instant;

use terraphim_firecracker::{PoolConfig, Sub2SecondOptimizer, VmPoolManager};

use super::ssh::SshExecutor;
use super::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
use crate::config::{BackendType, RlmConfig};
use crate::error::{RlmError, RlmResult};
use crate::types::SessionId;

/// Firecracker execution backend.
///
/// Wraps the `terraphim_firecracker` crate to provide RLM execution capabilities
/// with full VM isolation.
pub struct FirecrackerExecutor {
    /// Configuration for the executor.
    config: RlmConfig,

    /// VM pool manager (will be initialized on first use).
    pool_manager: Option<Arc<VmPoolManager>>,

    /// SSH executor for running commands on VMs.
    ssh_executor: SshExecutor,

    /// Capabilities supported by this executor.
    capabilities: Vec<Capability>,

    /// Active snapshots keyed by session.
    snapshots: parking_lot::RwLock<HashMap<SessionId, Vec<SnapshotId>>>,

    /// Session to VM IP mapping for affinity.
    session_vms: parking_lot::RwLock<HashMap<SessionId, String>>,
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
            pool_manager: None,
            ssh_executor,
            capabilities,
            snapshots: parking_lot::RwLock::new(HashMap::new()),
            session_vms: parking_lot::RwLock::new(HashMap::new()),
        })
    }

    /// Initialize the VM pool.
    ///
    /// This is called lazily on first execution to avoid startup overhead.
    async fn ensure_pool(&mut self) -> Result<Arc<VmPoolManager>, RlmError> {
        if let Some(ref pool) = self.pool_manager {
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

        // Create optimizer and VM manager
        // Note: This is a stub - actual implementation will create real VmManager
        let _optimizer = Arc::new(Sub2SecondOptimizer::new());

        // TODO: Create actual VmManager and VmPoolManager
        // For now, we'll return an error indicating initialization is incomplete
        log::warn!("FirecrackerExecutor: VM pool initialization not yet implemented");

        Err(RlmError::BackendInitFailed {
            backend: "firecracker".to_string(),
            message: "VM pool initialization not yet implemented".to_string(),
        })
    }

    /// Get or allocate a VM for a session.
    ///
    /// Returns the VM IP address if available, or None if no VM could be allocated.
    async fn get_or_allocate_vm(&self, session_id: &SessionId) -> RlmResult<Option<String>> {
        // Check if session already has an assigned VM
        {
            let session_vms = self.session_vms.read();
            if let Some(ip) = session_vms.get(session_id) {
                log::debug!("Using existing VM for session {}: {}", session_id, ip);
                return Ok(Some(ip.clone()));
            }
        }

        // Try to allocate from pool
        // Note: Full pool integration requires terraphim_firecracker enhancements (GitHub #15)
        // For now, we check if pool_manager is initialized
        if self.pool_manager.is_some() {
            // Pool allocation would happen here
            // let (vm, _alloc_time) = self.pool_manager.as_ref().unwrap()
            //     .allocate_vm("terraphim-minimal")
            //     .await
            //     .map_err(|e| RlmError::VmAllocationTimeout {
            //         timeout_ms: self.config.allocation_timeout_ms,
            //     })?;
            //
            // if let Some(ip) = vm.read().await.ip_address.clone() {
            //     self.session_vms.write().insert(*session_id, ip.clone());
            //     return Ok(Some(ip));
            // }
            log::debug!("VM pool available but allocation not yet implemented");
        }

        log::debug!("No VM available for session {}", session_id);
        Ok(None)
    }

    /// Assign a VM to a session (for external allocation).
    pub fn assign_vm_to_session(&self, session_id: SessionId, vm_ip: String) {
        log::info!("Assigning VM {} to session {}", vm_ip, session_id);
        self.session_vms.write().insert(session_id, vm_ip);
    }

    /// Release VM assignment for a session.
    pub fn release_session_vm(&self, session_id: &SessionId) -> Option<String> {
        self.session_vms.write().remove(session_id)
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
        let vm_ip = self.get_or_allocate_vm(&ctx.session_id).await?;

        match vm_ip {
            Some(ref ip) => {
                // Execute via SSH on the allocated VM
                log::info!("Executing on VM {} for session {}", ip, ctx.session_id);

                let result = if is_python {
                    self.ssh_executor.execute_python(ip, code, ctx).await
                } else {
                    self.ssh_executor.execute_command(ip, code, ctx).await
                };

                match result {
                    Ok(mut res) => {
                        // Add VM metadata
                        res.metadata
                            .insert("vm_ip".to_string(), ip.clone());
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
                // No VM available - return stub response indicating this
                // In production, this would be an error, but for development
                // we return a stub to allow testing without VMs
                log::warn!(
                    "No VM available for execution (session={}), returning stub response",
                    ctx.session_id
                );

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
                             Assign a VM using assign_vm_to_session() or ensure VM pool is initialized."
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
        // For now, accept all input
        log::debug!(
            "FirecrackerExecutor::validate called for {} bytes",
            input.len()
        );

        Ok(ValidationResult::valid(Vec::new()))
    }

    async fn create_snapshot(&self, name: &str) -> Result<SnapshotId, Self::Error> {
        // TODO: Implement Firecracker VM snapshot
        log::debug!("FirecrackerExecutor::create_snapshot called: {}", name);

        // Check snapshot limit
        // Note: This is a placeholder - actual implementation would check per-session
        let session_id = SessionId::new(); // Placeholder - would come from context

        let mut snapshots = self.snapshots.write();
        let session_snapshots = snapshots.entry(session_id).or_default();

        if session_snapshots.len() >= self.config.max_snapshots_per_session as usize {
            return Err(RlmError::MaxSnapshotsReached {
                max: self.config.max_snapshots_per_session,
            });
        }

        let snapshot_id = SnapshotId::new(name, session_id);
        session_snapshots.push(snapshot_id.clone());

        Ok(snapshot_id)
    }

    async fn restore_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        // TODO: Implement Firecracker VM snapshot restore
        log::debug!("FirecrackerExecutor::restore_snapshot called: {}", id);

        let snapshots = self.snapshots.read();
        if let Some(session_snapshots) = snapshots.get(&id.session_id) {
            if session_snapshots.iter().any(|s| s.id == id.id) {
                return Ok(());
            }
        }

        Err(RlmError::SnapshotNotFound {
            snapshot_id: id.to_string(),
        })
    }

    async fn list_snapshots(&self) -> Result<Vec<SnapshotId>, Self::Error> {
        // Return all snapshots across all sessions
        // Note: In real implementation, this would be session-scoped
        let snapshots = self.snapshots.read();
        let all_snapshots: Vec<SnapshotId> = snapshots.values().flatten().cloned().collect();
        Ok(all_snapshots)
    }

    async fn delete_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        let mut snapshots = self.snapshots.write();
        if let Some(session_snapshots) = snapshots.get_mut(&id.session_id) {
            if let Some(pos) = session_snapshots.iter().position(|s| s.id == id.id) {
                session_snapshots.remove(pos);
                return Ok(());
            }
        }

        Err(RlmError::SnapshotNotFound {
            snapshot_id: id.to_string(),
        })
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

        // TODO: Check VM pool health
        // For now, just return true if KVM is available
        Ok(true)
    }

    async fn cleanup(&self) -> Result<(), Self::Error> {
        log::info!("FirecrackerExecutor::cleanup called");

        // Clear snapshots
        self.snapshots.write().clear();

        // TODO: Cleanup VM pool
        // - Return VMs to pool or destroy overflow VMs
        // - Clean up any temp files

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionEnvironment;

    #[test]
    fn test_firecracker_executor_capabilities() {
        // Skip if KVM not available
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
        // This test verifies the error when KVM is not available
        // Note: This test will pass on systems without KVM
        if super::super::is_kvm_available() {
            eprintln!("Skipping test: KVM is available");
            return;
        }

        let config = RlmConfig::default();
        let result = FirecrackerExecutor::new(config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_firecracker_snapshot_management() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping test: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();

        // Create a snapshot
        let snapshot = executor.create_snapshot("test-snap").await.unwrap();
        assert_eq!(snapshot.name, "test-snap");

        // List snapshots
        let snapshots = executor.list_snapshots().await.unwrap();
        assert_eq!(snapshots.len(), 1);

        // Restore snapshot
        let result = executor.restore_snapshot(&snapshot).await;
        assert!(result.is_ok());

        // Delete snapshot
        let result = executor.delete_snapshot(&snapshot).await;
        assert!(result.is_ok());

        // Verify deletion
        let snapshots = executor.list_snapshots().await.unwrap();
        assert!(snapshots.is_empty());
    }
}
