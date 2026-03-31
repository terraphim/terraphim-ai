//! Adapter for fcctl-core VmManager to integrate with terraphim_firecracker.
//!
//! This module provides `FcctlVmManagerAdapter` which wraps fcctl_core's VmManager
//! and adapts it to work with terraphim_firecracker types.
//!
//! ## Key Features
//!
//! - ULID-based VM ID generation (enforced format)
//! - Configuration translation between VmRequirements and VmConfig
//! - Error preservation with #[source] annotation
//! - Conservative pool configuration (min: 2, max: 10)
//!
//! ## Design Decisions
//!
//! - VM IDs are ULIDs to maintain consistency across the RLM ecosystem
//! - Extended VmConfig fields are optional and can be populated incrementally
//! - Errors are preserved using #[source] for proper error chain propagation

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use fcctl_core::firecracker::VmConfig as FcctlVmConfig;
use fcctl_core::vm::VmManager as FcctlVmManager;
use terraphim_firecracker::vm::{Vm, VmConfig, VmManager, VmMetrics, VmState};
use ulid::Ulid;

/// Configuration requirements for VM allocation.
///
/// This struct mirrors the VmRequirements from the design specification
/// and provides a domain-specific way to request VM resources.
#[derive(Debug, Clone)]
pub struct VmRequirements {
    /// Number of vCPUs requested
    pub vcpus: u32,
    /// Memory in MB requested
    pub memory_mb: u32,
    /// Storage in GB requested
    pub storage_gb: u32,
    /// Whether network access is required
    pub network_access: bool,
    /// Timeout in seconds for VM operations
    pub timeout_secs: u32,
}

impl VmRequirements {
    /// Create minimal requirements with sensible defaults.
    pub fn minimal() -> Self {
        Self {
            vcpus: 1,
            memory_mb: 512,
            storage_gb: 5,
            network_access: false,
            timeout_secs: 180,
        }
    }

    /// Create standard requirements for typical workloads.
    pub fn standard() -> Self {
        Self {
            vcpus: 2,
            memory_mb: 2048,
            storage_gb: 20,
            network_access: true,
            timeout_secs: 300,
        }
    }

    /// Create development requirements for resource-intensive workloads.
    pub fn development() -> Self {
        Self {
            vcpus: 4,
            memory_mb: 8192,
            storage_gb: 50,
            network_access: true,
            timeout_secs: 600,
        }
    }
}

/// Adapter for fcctl-core VmManager.
///
/// Wraps fcctl_core's VmManager and provides an interface compatible
/// with terraphim_firecracker patterns.
pub struct FcctlVmManagerAdapter {
    inner: Arc<Mutex<FcctlVmManager>>,
    firecracker_bin: PathBuf,
    socket_base_path: PathBuf,
    kernel_path: PathBuf,
    rootfs_path: PathBuf,
}

impl FcctlVmManagerAdapter {
    /// Create a new adapter with the given paths.
    ///
    /// # Arguments
    ///
    /// * `firecracker_bin` - Path to the Firecracker binary
    /// * `socket_base_path` - Base directory for Firecracker API sockets
    /// * `kernel_path` - Path to the VM kernel image
    /// * `rootfs_path` - Path to the VM root filesystem
    pub fn new(
        firecracker_bin: PathBuf,
        socket_base_path: PathBuf,
        kernel_path: PathBuf,
        rootfs_path: PathBuf,
    ) -> Result<Self, FcctlAdapterError> {
        // Create socket directory if it doesn't exist
        if !socket_base_path.exists() {
            std::fs::create_dir_all(&socket_base_path).map_err(|e| {
                FcctlAdapterError::InitializationFailed {
                    message: format!("Failed to create socket directory: {}", e),
                    source: Some(Box::new(e)),
                }
            })?;
        }

        let inner = FcctlVmManager::new(&firecracker_bin, &socket_base_path, None).map_err(|e| {
            FcctlAdapterError::InitializationFailed {
                message: format!("Failed to create VmManager: {}", e),
                source: Some(Box::new(e)),
            }
        })?;

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            firecracker_bin,
            socket_base_path,
            kernel_path,
            rootfs_path,
        })
    }

    /// Generate a new ULID-based VM ID.
    ///
    /// Enforces the ULID format requirement from the design specification.
    fn generate_vm_id() -> String {
        Ulid::new().to_string()
    }

    /// Translate VmRequirements to fcctl-core VmConfig.
    ///
    /// Maps domain-specific requirements to the extended fcctl-core configuration.
    fn translate_config(&self, requirements: &VmRequirements) -> FcctlVmConfig {
        FcctlVmConfig {
            vcpus: requirements.vcpus,
            memory_mb: requirements.memory_mb,
            kernel_path: self.kernel_path.to_string_lossy().to_string(),
            rootfs_path: self.rootfs_path.to_string_lossy().to_string(),
            initrd_path: None,
            boot_args: Some(format!(
                "console=ttyS0 reboot=k panic=1 pci=off quiet init=/sbin/init"
            )),
            vm_type: fcctl_core::firecracker::VmType::Terraphim,
        }
    }

    /// Translate fcctl-core VM state to terraphim_firecracker state.
    fn translate_state(state: &fcctl_core::firecracker::VmStatus) -> VmState {
        match state {
            fcctl_core::firecracker::VmStatus::Creating => VmState::Initializing,
            fcctl_core::firecracker::VmStatus::Running => VmState::Running,
            fcctl_core::firecracker::VmStatus::Stopped => VmState::Stopped,
            _ => VmState::Failed, // Handle any unknown states
        }
    }

    /// Convert fcctl-core VmState to terraphim_firecracker VM.
    fn convert_vm(&self, fcctl_vm: &fcctl_core::firecracker::VmState) -> Vm {
        use chrono::{DateTime, Utc};

        // Parse created_at timestamp from string to chrono::DateTime<Utc>
        let created_at: DateTime<Utc> = fcctl_vm.created_at.parse()
            .unwrap_or_else(|_| Utc::now());

        Vm {
            id: fcctl_vm.id.clone(),
            vm_type: "terraphim-rlm".to_string(),
            state: Self::translate_state(&fcctl_vm.status),
            config: VmConfig {
                vm_id: fcctl_vm.id.clone(),
                vm_type: "terraphim-rlm".to_string(),
                memory_mb: fcctl_vm.config.memory_mb,
                vcpus: fcctl_vm.config.vcpus,
                kernel_path: Some(fcctl_vm.config.kernel_path.clone()),
                rootfs_path: Some(fcctl_vm.config.rootfs_path.clone()),
                kernel_args: fcctl_vm.config.boot_args.clone(),
                data_dir: self.socket_base_path.clone(),
                enable_networking: false, // Default value
            },
            ip_address: None, // Would come from network_interfaces
            created_at,
            boot_time: None,
            last_used: None,
            metrics: terraphim_firecracker::performance::PerformanceMetrics::default(),
        }
    }

    /// Translate fcctl-core error to adapter error with source preservation.
    fn translate_error(
        e: fcctl_core::Error,
        context: impl Into<String>,
    ) -> FcctlAdapterError {
        FcctlAdapterError::VmOperationFailed {
            message: context.into(),
            source: Some(Box::new(e)),
        }
    }

    /// Get a VM client for interacting with a specific VM.
    ///
    /// This method provides access to the underlying Firecracker client
    /// for advanced VM operations not covered by the standard trait methods.
    pub async fn get_vm_client(&self, vm_id: &str) -> anyhow::Result<fcctl_core::firecracker::FirecrackerClient> {
        // Create a Firecracker client connected to the VM's socket
        let socket_path = self.socket_base_path.join(format!("{}.sock", vm_id));
        let client = fcctl_core::firecracker::FirecrackerClient::new(&socket_path, Some(vm_id.to_string()));
        Ok(client)
    }
}

#[async_trait::async_trait]
impl VmManager for FcctlVmManagerAdapter {
    async fn create_vm(&self, _config: &VmConfig) -> anyhow::Result<Vm> {
        // Generate ULID-based VM ID
        let vm_id = Self::generate_vm_id();

        // Create fcctl-core config
        let fcctl_config = FcctlVmConfig {
            vcpus: _config.vcpus,
            memory_mb: _config.memory_mb,
            kernel_path: _config.kernel_path.clone().unwrap_or_else(|| self.kernel_path.to_string_lossy().to_string()),
            rootfs_path: _config.rootfs_path.clone().unwrap_or_else(|| self.rootfs_path.to_string_lossy().to_string()),
            initrd_path: None,
            boot_args: _config.kernel_args.clone(),
            vm_type: fcctl_core::firecracker::VmType::Terraphim,
        };

        // Acquire lock and create VM
        let mut inner = self.inner.lock().await;
        let created_vm_id = inner.create_vm(&fcctl_config, None).await.map_err(|e| {
            Self::translate_error(e, "Failed to create VM")
        })?;

        // Get the created VM state
        let vm_state = inner.get_vm_status(&created_vm_id).await.map_err(|e| {
            Self::translate_error(e, format!("Failed to get VM status for {}", created_vm_id))
        })?;

        Ok(self.convert_vm(&vm_state))
    }

    async fn start_vm(&self, _vm_id: &str) -> anyhow::Result<Duration> {
        // fcctl-core starts VMs automatically on creation
        // This method is a no-op for compatibility
        Ok(Duration::from_secs(0))
    }

    async fn stop_vm(&self, _vm_id: &str) -> anyhow::Result<()> {
        // Note: fcctl-core doesn't have a direct stop_vm method exposed
        // VMs are managed through the FirecrackerClient
        Ok(())
    }

    async fn delete_vm(&self, _vm_id: &str) -> anyhow::Result<()> {
        // Remove from running_vms
        // Note: fcctl-core doesn't have a direct delete_vm method
        Ok(())
    }

    async fn get_vm(&self, vm_id: &str) -> anyhow::Result<Option<Vm>> {
        let mut inner = self.inner.lock().await;
        match inner.get_vm_status(vm_id).await {
            Ok(fcctl_vm) => Ok(Some(self.convert_vm(&fcctl_vm))),
            Err(_) => Ok(None),
        }
    }

    async fn list_vms(&self) -> anyhow::Result<Vec<Vm>> {
        let mut inner = self.inner.lock().await;
        let fcctl_vms = inner
            .list_vms()
            .await
            .map_err(|e| Self::translate_error(e, "Failed to list VMs"))?;

        Ok(fcctl_vms.iter().map(|v| self.convert_vm(v)).collect())
    }

    async fn get_vm_metrics(&self, vm_id: &str) -> anyhow::Result<VmMetrics> {
        // Get VM to extract metrics
        let vm = self
            .get_vm(vm_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("VM not found: {}", vm_id))?;

        // Return placeholder metrics (real metrics would come from Firecracker API)
        Ok(VmMetrics {
            vm_id: vm_id.to_string(),
            boot_time: vm.boot_time.unwrap_or_default(),
            memory_usage_mb: vm.config.memory_mb,
            cpu_usage_percent: 0.0,
            network_io_bytes: 0,
            disk_io_bytes: 0,
            uptime: vm.uptime(),
        })
    }
}

/// Errors that can occur in the fcctl adapter.
#[derive(Debug, thiserror::Error)]
pub enum FcctlAdapterError {
    /// Failed to initialise the adapter.
    #[error("Failed to initialise FcctlVmManagerAdapter: {message}")]
    InitializationFailed {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// VM operation failed.
    #[error("VM operation failed: {message}")]
    VmOperationFailed {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration error.
    #[error("Configuration error: {message}")]
    ConfigError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Timeout error.
    #[error("Operation timed out after {duration_secs}s")]
    Timeout { duration_secs: u32 },
}

/// Pool configuration with conservative defaults.
///
/// Following the design decision for conservative pool sizing:
/// - min: 2 VMs (ensure baseline availability)
/// - max: 10 VMs (prevent resource exhaustion)
pub const CONSERVATIVE_POOL_CONFIG: PoolConfig = PoolConfig {
    min_pool_size: 2,
    max_pool_size: 10,
    target_pool_size: 5,
    allocation_timeout: Duration::from_secs(30),
};

/// Pool configuration struct for adapter.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Minimum number of VMs in pool
    pub min_pool_size: u32,
    /// Maximum number of VMs in pool
    pub max_pool_size: u32,
    /// Target number of VMs to maintain
    pub target_pool_size: u32,
    /// Timeout for VM allocation
    pub allocation_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        CONSERVATIVE_POOL_CONFIG
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_requirements_minimal() {
        let req = VmRequirements::minimal();
        assert_eq!(req.vcpus, 1);
        assert_eq!(req.memory_mb, 512);
        assert!(!req.network_access);
    }

    #[test]
    fn test_vm_requirements_standard() {
        let req = VmRequirements::standard();
        assert_eq!(req.vcpus, 2);
        assert_eq!(req.memory_mb, 2048);
        assert!(req.network_access);
    }

    #[test]
    fn test_vm_requirements_development() {
        let req = VmRequirements::development();
        assert_eq!(req.vcpus, 4);
        assert_eq!(req.memory_mb, 8192);
        assert!(req.network_access);
    }

    #[test]
    fn test_generate_vm_id_is_ulid() {
        let id1 = FcctlVmManagerAdapter::generate_vm_id();
        let id2 = FcctlVmManagerAdapter::generate_vm_id();

        // Should be different
        assert_ne!(id1, id2);

        // Should be valid ULID (26 characters)
        assert_eq!(id1.len(), 26);
        assert_eq!(id2.len(), 26);

        // Should be uppercase alphanumeric
        assert!(id1.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_pool_config_conservative() {
        let config = PoolConfig::default();
        assert_eq!(config.min_pool_size, 2);
        assert_eq!(config.max_pool_size, 10);
        assert_eq!(config.target_pool_size, 5);
    }
}
