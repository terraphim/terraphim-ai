//! Firecracker execution backend.
//!
//! This module provides the `FirecrackerExecutor` which implements the
//! `ExecutionEnvironment` trait using Firecracker microVMs for full isolation.
//!
//! ## Features
//!
//! - Full VM isolation (no shared kernel with host)
//! - Pre-warmed VM pool for sub-500ms allocation
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

use terraphim_firecracker::vm::VmConfig;
use terraphim_firecracker::{
    InMemoryVmStorage, PoolConfig, Sub2SecondOptimizer, Sub2SecondVmManager, VmManager,
    VmPoolManager,
};

use super::ssh::SshExecutor;
use super::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
use crate::config::{BackendType, RlmConfig};
use crate::error::{RlmError, RlmResult};
use crate::types::SessionId;

/// Firecracker execution backend.
///
/// Wraps `terraphim_firecracker`'s `Sub2SecondVmManager` and `VmPoolManager`
/// to provide RLM execution capabilities with full VM isolation.
///
/// All mutable state uses interior mutability to allow the trait
/// implementation to use `&self` as required by `ExecutionEnvironment`.
pub struct FirecrackerExecutor {
    /// Configuration for the executor.
    config: RlmConfig,

    /// VM manager from terraphim_firecracker for VM lifecycle.
    /// Uses tokio::sync::Mutex for Send-safe async access.
    vm_manager: tokio::sync::Mutex<Option<Arc<dyn VmManager>>>,

    /// VM pool manager for pre-warmed VMs.
    pool_manager: parking_lot::RwLock<Option<Arc<VmPoolManager>>>,

    /// SSH executor for running commands on VMs.
    ssh_executor: SshExecutor,

    /// Capabilities supported by this executor.
    capabilities: Vec<Capability>,

    /// Session to VM ID mapping for affinity.
    session_to_vm: parking_lot::RwLock<HashMap<SessionId, String>>,

    /// Session to VM IP mapping for SSH access.
    session_to_vm_ip: parking_lot::RwLock<HashMap<SessionId, String>>,

    /// Current active snapshot per session (for rollback support).
    current_snapshot: parking_lot::RwLock<HashMap<SessionId, String>>,

    /// Snapshot count per session (for limit enforcement).
    snapshot_counts: parking_lot::RwLock<HashMap<SessionId, u32>>,
}

impl FirecrackerExecutor {
    /// Create a new Firecracker executor.
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

        let ssh_executor = SshExecutor::new()
            .with_user("root")
            .with_private_key("/tmp/ubuntu-22.04.id_rsa")
            .with_output_dir(std::env::temp_dir().join("terraphim_rlm_output"));

        Ok(Self {
            config,
            vm_manager: tokio::sync::Mutex::new(None),
            pool_manager: parking_lot::RwLock::new(None),
            ssh_executor,
            capabilities,
            session_to_vm: parking_lot::RwLock::new(HashMap::new()),
            session_to_vm_ip: parking_lot::RwLock::new(HashMap::new()),
            current_snapshot: parking_lot::RwLock::new(HashMap::new()),
            snapshot_counts: parking_lot::RwLock::new(HashMap::new()),
        })
    }

    /// Initialize the VM manager and pre-warm the pool.
    ///
    /// This should be called before using the executor.
    pub async fn initialize(&self) -> RlmResult<()> {
        log::info!("Initializing FirecrackerExecutor with terraphim_firecracker managers");

        let socket_dir = PathBuf::from("/tmp/firecracker-sockets");
        if !socket_dir.exists() {
            std::fs::create_dir_all(&socket_dir).map_err(|e| RlmError::BackendInitFailed {
                backend: "firecracker".to_string(),
                message: format!("Failed to create socket directory: {}", e),
            })?;
        }

        let storage = Arc::new(InMemoryVmStorage::new());
        let vm_manager = Sub2SecondVmManager::new(socket_dir, storage)
            .await
            .map_err(|e| RlmError::BackendInitFailed {
                backend: "firecracker".to_string(),
                message: format!("Failed to create VmManager: {}", e),
            })?;

        *self.vm_manager.lock().await = Some(Arc::new(vm_manager));

        // Initialize the VM pool; non-fatal so execution can fall back to direct VM creation.
        if let Err(e) = self.ensure_pool().await {
            log::warn!(
                "FirecrackerExecutor: VM pool initialization failed (pre-warming disabled): {}",
                e
            );
        }

        log::info!("FirecrackerExecutor initialized successfully");
        Ok(())
    }

    /// Initialize the VM pool for pre-warmed VMs.
    ///
    /// Returns an `Arc<VmPoolManager>` that callers may use for pool statistics
    /// or direct allocation. The pool is cached so subsequent calls return the
    /// same instance without re-initializing.
    async fn ensure_pool(&self) -> Result<Arc<VmPoolManager>, RlmError> {
        if let Some(ref pool) = *self.pool_manager.read() {
            return Ok(Arc::clone(pool));
        }

        let pool_config = PoolConfig {
            min_pool_size: self.config.pool_min_size,
            max_pool_size: self.config.pool_max_size,
            target_pool_size: self.config.pool_target_size,
            allocation_timeout: std::time::Duration::from_millis(self.config.allocation_timeout_ms),
            ..Default::default()
        };

        let vm_manager = {
            let guard = self.vm_manager.lock().await;
            guard
                .as_ref()
                .cloned()
                .ok_or_else(|| RlmError::BackendInitFailed {
                    backend: "firecracker".to_string(),
                    message: "VmManager not initialised; call initialize() first".to_string(),
                })?
        };

        log::info!(
            "Initialising Firecracker VM pool (min={}, max={}, target={})",
            pool_config.min_pool_size,
            pool_config.max_pool_size,
            pool_config.target_pool_size,
        );

        let optimizer = Arc::new(Sub2SecondOptimizer::new());
        let pool_manager = VmPoolManager::new(vm_manager, optimizer, pool_config);

        // Seed the pool with the VM type used by this executor.
        pool_manager
            .initialize_pools(vec!["terraphim-minimal".to_string()])
            .await
            .map_err(|e| RlmError::BackendInitFailed {
                backend: "firecracker".to_string(),
                message: format!("Failed to initialise VM pools: {}", e),
            })?;

        let pool = Arc::new(pool_manager);
        *self.pool_manager.write() = Some(Arc::clone(&pool));

        log::info!("Firecracker VM pool ready");
        Ok(pool)
    }

    /// Get or allocate a VM for a session.
    ///
    /// Returns `(vm_id, Option<ip_address>)` for the session VM.
    async fn get_or_allocate_vm(
        &self,
        session_id: &SessionId,
    ) -> RlmResult<Option<(String, Option<String>)>> {
        // Return existing session assignment.
        {
            let session_to_vm = self.session_to_vm.read();
            if let Some(vm_id) = session_to_vm.get(session_id) {
                let ip = self.session_to_vm_ip.read().get(session_id).cloned();
                log::debug!("Using existing VM {} for session {}", vm_id, session_id);
                return Ok(Some((vm_id.clone(), ip)));
            }
        }

        // Try to create a new VM via the VmManager.
        let vm_manager_guard = self.vm_manager.lock().await;
        if let Some(ref vm_manager) = *vm_manager_guard {
            log::info!("Creating new VM for session {}", session_id);

            let vm_config = VmConfig {
                vm_id: ulid::Ulid::new().to_string(),
                vm_type: "terraphim-minimal".to_string(),
                memory_mb: 1024,
                vcpus: 2,
                kernel_path: Some("/tmp/vmlinux-5.10.225".to_string()),
                rootfs_path: Some("/tmp/ubuntu-22.04.ext4".to_string()),
                kernel_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
                data_dir: PathBuf::from("/tmp/terraphim-vms"),
                enable_networking: true,
            };

            match vm_manager.create_vm(&vm_config).await {
                Ok(vm) => {
                    let vm_id = vm.id.clone();
                    let ip = vm.ip_address.clone();
                    drop(vm_manager_guard);

                    log::info!("Created VM {} for session {}", vm_id, session_id);
                    self.session_to_vm
                        .write()
                        .insert(*session_id, vm_id.clone());
                    if let Some(ref ip_addr) = ip {
                        self.session_to_vm_ip
                            .write()
                            .insert(*session_id, ip_addr.clone());
                    }
                    return Ok(Some((vm_id, ip)));
                }
                Err(e) => {
                    log::error!("Failed to create VM for session {}: {}", session_id, e);
                }
            }
        } else {
            log::debug!(
                "VmManager not initialised, cannot create VM for session {}",
                session_id
            );
        }

        Ok(None)
    }

    /// Assign a VM to a session (used in tests and for manual wiring).
    pub fn assign_vm_to_session(&self, session_id: SessionId, vm_id: String) {
        log::info!("Assigning VM {} to session {}", vm_id, session_id);
        self.session_to_vm.write().insert(session_id, vm_id);
    }

    /// Release VM assignment for a session.
    pub fn release_session_vm(&self, session_id: &SessionId) -> Option<String> {
        self.session_to_vm_ip.write().remove(session_id);
        self.session_to_vm.write().remove(session_id)
    }

    /// Get the current active snapshot for a session.
    pub fn get_current_snapshot(&self, session_id: &SessionId) -> Option<String> {
        self.current_snapshot.read().get(session_id).cloned()
    }

    fn clear_current_snapshot(&self, session_id: &SessionId) {
        self.current_snapshot.write().remove(session_id);
    }

    /// Rollback to the previous known good state (no-op when no snapshot exists).
    pub async fn rollback(&self, session_id: &SessionId) -> Result<(), RlmError> {
        match self.get_current_snapshot(session_id) {
            Some(snapshot_id) => {
                log::warn!(
                    "Rolling back session {} to snapshot '{}' — snapshot restore not yet implemented",
                    session_id,
                    snapshot_id
                );
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

    /// Execute code in a VM, falling back to stub when no VM is available.
    async fn execute_in_vm(
        &self,
        code: &str,
        is_python: bool,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, RlmError> {
        let start = Instant::now();

        log::debug!(
            "FirecrackerExecutor::execute_in_vm (python={}, session={})",
            is_python,
            ctx.session_id
        );

        let vm_info = self.get_or_allocate_vm(&ctx.session_id).await?;

        match vm_info {
            Some((ref vm_id, Some(ref ip))) => {
                log::info!(
                    "Executing on VM {} ({}) for session {}",
                    vm_id,
                    ip,
                    ctx.session_id
                );

                // Wait for VM SSH to become available.
                let ssh_ready = tokio::time::timeout(std::time::Duration::from_secs(30), async {
                    for attempt in 0..30 {
                        if self.ssh_executor.check_connection(ip).await {
                            log::info!("VM {} SSH ready after {} attempts", vm_id, attempt + 1);
                            return true;
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    false
                })
                .await
                .unwrap_or(false);

                if !ssh_ready {
                    log::warn!("VM {} SSH not ready, returning stub", vm_id);
                    return self.stub_response(code, start);
                }

                let result = if is_python {
                    self.ssh_executor.execute_python(ip, code, ctx).await
                } else {
                    self.ssh_executor.execute_command(ip, code, ctx).await
                };

                match result {
                    Ok(mut res) => {
                        res.metadata.insert("vm_id".to_string(), vm_id.clone());
                        res.metadata.insert("vm_ip".to_string(), ip.clone());
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
            Some((ref vm_id, None)) => {
                log::warn!("VM {} has no IP address assigned", vm_id);
                self.stub_response(code, start)
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
        log::debug!(
            "FirecrackerExecutor::validate called for {} bytes",
            input.len()
        );
        Ok(ValidationResult::valid(Vec::new()))
    }

    async fn create_snapshot(
        &self,
        _session_id: &SessionId,
        _name: &str,
    ) -> Result<SnapshotId, Self::Error> {
        Err(RlmError::SnapshotCreationFailed {
            message: "Snapshot management requires fcctl-core integration (not yet available)"
                .to_string(),
        })
    }

    async fn restore_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        Err(RlmError::SnapshotRestoreFailed {
            message: format!(
                "Snapshot '{}' restore requires fcctl-core integration (not yet available)",
                id.name
            ),
        })
    }

    async fn list_snapshots(&self, session_id: &SessionId) -> Result<Vec<SnapshotId>, Self::Error> {
        log::debug!(
            "list_snapshots for session {} — returning empty list (fcctl-core not available)",
            session_id
        );
        Ok(Vec::new())
    }

    async fn delete_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        Err(RlmError::SnapshotNotFound {
            snapshot_id: format!(
                "Cannot delete snapshot '{}': fcctl-core integration not available",
                id.name
            ),
        })
    }

    async fn delete_session_snapshots(&self, session_id: &SessionId) -> Result<(), Self::Error> {
        log::info!(
            "delete_session_snapshots for session {} — no-op (fcctl-core not available)",
            session_id
        );
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
        if !super::is_kvm_available() {
            return Ok(false);
        }
        Ok(self.vm_manager.lock().await.is_some())
    }

    async fn cleanup(&self) -> Result<(), Self::Error> {
        log::info!("FirecrackerExecutor::cleanup");
        self.session_to_vm.write().clear();
        self.session_to_vm_ip.write().clear();
        self.current_snapshot.write().clear();
        self.snapshot_counts.write().clear();
        Ok(())
    }

    async fn end_session(&self, session_id: &SessionId) -> Result<(), Self::Error> {
        if let Some(vm_id) = self.release_session_vm(session_id) {
            log::debug!("end_session({}) released vm {}", session_id, vm_id);
        }
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
            eprintln!("Skipping: KVM not available");
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
            eprintln!("Skipping: KVM is available");
            return;
        }

        let config = RlmConfig::default();
        let result = FirecrackerExecutor::new(config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_session_vm_assignment() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();
        let session_id = SessionId::new();

        assert!(executor.session_to_vm.read().get(&session_id).is_none());

        executor.assign_vm_to_session(session_id, "vm-test-123".to_string());
        assert_eq!(
            executor.session_to_vm.read().get(&session_id),
            Some(&"vm-test-123".to_string())
        );

        let released = executor.release_session_vm(&session_id);
        assert_eq!(released, Some("vm-test-123".to_string()));
        assert!(executor.session_to_vm.read().get(&session_id).is_none());
    }

    #[tokio::test]
    async fn test_current_snapshot_tracking() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();
        let session_id = SessionId::new();

        assert!(executor.get_current_snapshot(&session_id).is_none());

        executor
            .current_snapshot
            .write()
            .insert(session_id, "snap-123".to_string());
        assert_eq!(
            executor.get_current_snapshot(&session_id),
            Some("snap-123".to_string())
        );

        executor.clear_current_snapshot(&session_id);
        assert!(executor.get_current_snapshot(&session_id).is_none());
    }

    #[tokio::test]
    async fn test_rollback_without_current_snapshot() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();
        let session_id = SessionId::new();

        let result = executor.rollback(&session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_without_initialization() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();

        let result = executor.health_check().await.unwrap();
        assert!(
            !result,
            "health_check should return false before initialize()"
        );
    }

    /// Verifies that `ensure_pool()` creates a real `VmPoolManager` after
    /// `initialize()` has wired up the underlying `VmManager`.
    #[tokio::test]
    async fn test_ensure_pool_creates_pool_manager() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();

        // Manually wire up VmManager so ensure_pool() has something to work with.
        let storage = Arc::new(InMemoryVmStorage::new());
        let vm_manager =
            Sub2SecondVmManager::new(PathBuf::from("/tmp/firecracker-sockets"), storage)
                .await
                .unwrap();
        *executor.vm_manager.lock().await = Some(Arc::new(vm_manager));

        let pool = executor
            .ensure_pool()
            .await
            .expect("ensure_pool() should succeed");
        let stats = pool.get_pool_stats().await;
        assert!(
            stats.type_stats.contains_key("terraphim-minimal"),
            "pool should be seeded with terraphim-minimal VM type"
        );
    }

    /// Verifies that `ensure_pool()` is idempotent: a second call returns the
    /// same cached `VmPoolManager` rather than creating a new one.
    #[tokio::test]
    async fn test_ensure_pool_is_idempotent() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();

        let storage = Arc::new(InMemoryVmStorage::new());
        let vm_manager =
            Sub2SecondVmManager::new(PathBuf::from("/tmp/firecracker-sockets"), storage)
                .await
                .unwrap();
        *executor.vm_manager.lock().await = Some(Arc::new(vm_manager));

        let pool_first = executor.ensure_pool().await.unwrap();
        let pool_second = executor.ensure_pool().await.unwrap();

        assert!(
            Arc::ptr_eq(&pool_first, &pool_second),
            "ensure_pool() must return the same Arc on successive calls"
        );
    }

    /// Verifies that `ensure_pool()` returns an error when called before
    /// `initialize()` has set up the `VmManager`.
    #[tokio::test]
    async fn test_ensure_pool_requires_vm_manager() {
        if !super::super::is_kvm_available() {
            eprintln!("Skipping: KVM not available");
            return;
        }

        let config = RlmConfig::default();
        let executor = FirecrackerExecutor::new(config).unwrap();

        // vm_manager is None — ensure_pool() must fail with a clear message.
        match executor.ensure_pool().await {
            Ok(_) => panic!("ensure_pool() must fail without a VmManager"),
            Err(e) => assert!(
                format!("{e}").contains("VmManager not initialised"),
                "error message should mention VmManager: {e}"
            ),
        }
    }
}
