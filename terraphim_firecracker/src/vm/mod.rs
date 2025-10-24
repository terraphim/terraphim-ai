use anyhow::Result;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::performance::PerformanceMetrics;

pub mod config;
pub mod firecracker;
pub mod network;

pub use config::VmConfig;
pub use firecracker::FirecrackerClient;

/// Type alias for VM instances wrapped in Arc<RwLock<>>
pub type VmInstance = Arc<RwLock<Vm>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vm {
    pub id: String,
    pub vm_type: String,
    pub state: VmState,
    pub config: VmConfig,
    pub ip_address: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub boot_time: Option<Duration>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub metrics: PerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VmState {
    Initializing,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Prewarmed,
    Ready,
    Prewarming,
    Allocating,
    NeedsMaintenance,
    Snapshotted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct VmMetrics {
    pub vm_id: String,
    pub boot_time: Duration,
    pub memory_usage_mb: u32,
    pub cpu_usage_percent: f64,
    pub network_io_bytes: u64,
    pub disk_io_bytes: u64,
    pub uptime: Duration,
}

impl Vm {
    pub fn new(vm_type: String, config: VmConfig) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            vm_type,
            state: VmState::Initializing,
            config,
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }
    }

    #[allow(dead_code)]
    pub fn prewarmed(vm_type: String, config: VmConfig) -> Self {
        let mut vm = Self::new(vm_type, config);
        vm.state = VmState::Prewarmed;
        vm
    }

    #[allow(dead_code)]
    pub fn mark_running(&mut self, boot_time: Duration, ip_address: Option<String>) {
        self.state = VmState::Running;
        self.boot_time = Some(boot_time);
        self.ip_address = ip_address;
        self.last_used = Some(chrono::Utc::now());
    }

    #[allow(dead_code)]
    pub fn mark_failed(&mut self) {
        self.state = VmState::Failed;
    }

    #[allow(dead_code)]
    pub fn is_ready_for_allocation(&self) -> bool {
        matches!(self.state, VmState::Prewarmed | VmState::Running)
    }

    pub fn uptime(&self) -> Duration {
        let now = chrono::Utc::now();
        let created = self.created_at;
        (now - created).to_std().unwrap_or_default()
    }

    pub async fn collect_metrics(&self) -> Result<VmMetrics> {
        // This would integrate with Firecracker API to get real metrics
        // For now, return placeholder metrics
        Ok(VmMetrics {
            vm_id: self.id.clone(),
            boot_time: self.boot_time.unwrap_or_default(),
            memory_usage_mb: self.config.memory_mb,
            cpu_usage_percent: 0.0,
            network_io_bytes: 0,
            disk_io_bytes: 0,
            uptime: self.uptime(),
        })
    }
}

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait VmManager: Send + Sync {
    async fn create_vm(&self, config: &VmConfig) -> Result<Vm>;
    async fn start_vm(&self, vm_id: &str) -> Result<Duration>;
    async fn stop_vm(&self, vm_id: &str) -> Result<()>;
    async fn delete_vm(&self, vm_id: &str) -> Result<()>;
    async fn get_vm(&self, vm_id: &str) -> Result<Option<Vm>>;
    async fn list_vms(&self) -> Result<Vec<Vm>>;
    async fn get_vm_metrics(&self, vm_id: &str) -> Result<VmMetrics>;
}

#[allow(dead_code)]
pub struct Sub2SecondVmManager {
    firecracker_client: FirecrackerClient,
    vm_storage: std::sync::Arc<dyn VmStorage>,
    performance_optimizer: crate::performance::Sub2SecondOptimizer,
}

impl Sub2SecondVmManager {
    pub async fn new(
        firecracker_socket_dir: PathBuf,
        storage: std::sync::Arc<dyn VmStorage>,
    ) -> Result<Self> {
        Ok(Self {
            firecracker_client: FirecrackerClient::new(firecracker_socket_dir),
            vm_storage: storage,
            performance_optimizer: crate::performance::Sub2SecondOptimizer::new(),
        })
    }

    #[allow(dead_code)]
    async fn create_prewarmed_vm(&self, vm_type: &str) -> Result<Vm> {
        info!("Creating prewarmed VM of type: {}", vm_type);

        let start_time = Instant::now();

        // Get optimized configuration for sub-2 second boot
        let mut config = self
            .performance_optimizer
            .get_optimized_config(vm_type)
            .await?;
        config.vm_id = Uuid::new_v4().to_string();

        // Apply ultra-fast boot parameters
        config.kernel_args = Some(
            self.performance_optimizer
                .get_ultra_fast_boot_args(vm_type, config.memory_mb),
        );

        let vm = Vm::prewarmed(vm_type.to_string(), config);

        // Store VM in storage
        self.vm_storage.store_vm(&vm).await?;

        // Pre-warm system resources
        // TODO: Fix prewarm_resources mutability issue
        debug!("System resource prewarming skipped due to mutability constraints");

        let creation_time = start_time.elapsed();
        info!(
            "Prewarmed VM created in: {:.3}s",
            creation_time.as_secs_f64()
        );

        Ok(vm)
    }

    #[allow(dead_code)]
    async fn allocate_from_prewarmed(&self, vm_type: &str) -> Result<Vm> {
        info!("Allocating VM from prewarmed pool for type: {}", vm_type);

        let start_time = Instant::now();

        // Find available prewarmed VM
        let prewarmed_vms = self.vm_storage.list_prewarmed_vms(vm_type).await?;

        if let Some(mut vm) = prewarmed_vms.into_iter().next() {
            // Instant allocation from prewarmed state
            vm.state = VmState::Starting;
            vm.last_used = Some(chrono::Utc::now());

            // Start the VM (should be <500ms from prewarmed state)
            let boot_time = self.start_vm(&vm.id).await?;

            vm.mark_running(boot_time, None);
            self.vm_storage.update_vm(&vm).await?;

            let total_time = start_time.elapsed();
            info!(
                "VM allocated from prewarmed pool in: {:.3}s",
                total_time.as_secs_f64()
            );

            Ok(vm)
        } else {
            // No prewarmed VM available, create new one
            warn!("No prewarmed VM available, creating new VM");
            self.create_and_start_vm(vm_type).await
        }
    }

    #[allow(dead_code)]
    async fn create_and_start_vm(&self, vm_type: &str) -> Result<Vm> {
        info!("Creating and starting new VM of type: {}", vm_type);

        let start_time = Instant::now();

        // Get optimized configuration
        let mut config = self
            .performance_optimizer
            .get_optimized_config(vm_type)
            .await?;
        config.vm_id = Uuid::new_v4().to_string();

        // Apply sub-2 second optimizations
        config.kernel_args = Some(
            self.performance_optimizer
                .get_sub2_boot_args(vm_type, config.memory_mb),
        );

        let mut vm = Vm::new(vm_type.to_string(), config);
        vm.state = VmState::Starting;

        // Store VM
        self.vm_storage.store_vm(&vm).await?;

        // Create VM with Firecracker
        self.firecracker_client.create_vm(&vm.config).await?;

        // Start VM with performance optimization
        let boot_time = self.start_vm(&vm.id).await?;

        vm.mark_running(boot_time, None);
        self.vm_storage.update_vm(&vm).await?;

        let total_time = start_time.elapsed();
        info!(
            "VM created and started in: {:.3}s",
            total_time.as_secs_f64()
        );

        Ok(vm)
    }
}

#[async_trait::async_trait]
impl VmManager for Sub2SecondVmManager {
    async fn create_vm(&self, config: &VmConfig) -> Result<Vm> {
        let vm = Vm::new(config.vm_type.clone(), config.clone());

        // Create VM with Firecracker
        self.firecracker_client.create_vm(config).await?;

        // Store VM
        self.vm_storage.store_vm(&vm).await?;

        Ok(vm)
    }

    async fn start_vm(&self, vm_id: &str) -> Result<Duration> {
        let start_time = Instant::now();

        // Start VM with Firecracker
        self.firecracker_client.start_vm(vm_id).await?;

        // Wait for VM to be ready with optimized timeout
        let mut retries = 0;
        let max_retries = 20; // 10 seconds with 500ms intervals

        while retries < max_retries {
            if let Ok(Some(vm)) = self.vm_storage.get_vm(vm_id).await {
                if vm.state == VmState::Running {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
            retries += 1;
        }

        let boot_time = start_time.elapsed();

        if boot_time > Duration::from_secs(2) {
            warn!(
                "VM boot time exceeded 2s target: {:.3}s",
                boot_time.as_secs_f64()
            );
        } else {
            info!("VM boot time: {:.3}s", boot_time.as_secs_f64());
        }

        Ok(boot_time)
    }

    async fn stop_vm(&self, vm_id: &str) -> Result<()> {
        self.firecracker_client.stop_vm(vm_id).await?;

        // Update VM state
        if let Ok(Some(mut vm)) = self.vm_storage.get_vm(vm_id).await {
            vm.state = VmState::Stopped;
            self.vm_storage.update_vm(&vm).await?;
        }

        Ok(())
    }

    async fn delete_vm(&self, vm_id: &str) -> Result<()> {
        self.firecracker_client.delete_vm(vm_id).await?;
        self.vm_storage.delete_vm(vm_id).await?;
        Ok(())
    }

    async fn get_vm(&self, vm_id: &str) -> Result<Option<Vm>> {
        self.vm_storage.get_vm(vm_id).await
    }

    async fn list_vms(&self) -> Result<Vec<Vm>> {
        self.vm_storage.list_vms().await
    }

    async fn get_vm_metrics(&self, vm_id: &str) -> Result<VmMetrics> {
        if let Some(vm) = self.get_vm(vm_id).await? {
            vm.collect_metrics().await
        } else {
            Err(anyhow::anyhow!("VM not found: {}", vm_id))
        }
    }
}

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait VmStorage: Send + Sync {
    async fn store_vm(&self, vm: &Vm) -> Result<()>;
    async fn get_vm(&self, vm_id: &str) -> Result<Option<Vm>>;
    async fn update_vm(&self, vm: &Vm) -> Result<()>;
    async fn delete_vm(&self, vm_id: &str) -> Result<()>;
    async fn list_vms(&self) -> Result<Vec<Vm>>;
    async fn list_prewarmed_vms(&self, vm_type: &str) -> Result<Vec<Vm>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::config::VmConfig;

    #[test]
    fn test_vm_creation() {
        let config = VmConfig {
            vm_id: "test-vm".to_string(),
            vm_type: "terraphim-minimal".to_string(),
            memory_mb: 512,
            vcpus: 1,
            kernel_path: Some("/path/to/kernel".to_string()),
            rootfs_path: Some("/path/to/rootfs".to_string()),
            kernel_args: None,
            data_dir: std::path::PathBuf::from("/tmp"),
            enable_networking: false,
        };

        let vm = Vm::new("test-type".to_string(), config);

        assert_eq!(vm.vm_type, "test-type");
        assert_eq!(vm.state, VmState::Initializing);
        assert!(vm.boot_time.is_none());
    }

    #[test]
    fn test_prewarmed_vm() {
        let config = VmConfig {
            vm_id: "prewarmed-vm".to_string(),
            vm_type: "terraphim-minimal".to_string(),
            memory_mb: 512,
            vcpus: 1,
            kernel_path: Some("/path/to/kernel".to_string()),
            rootfs_path: Some("/path/to/rootfs".to_string()),
            kernel_args: None,
            data_dir: std::path::PathBuf::from("/tmp"),
            enable_networking: false,
        };

        let vm = Vm::prewarmed("test-type".to_string(), config);

        assert_eq!(vm.state, VmState::Prewarmed);
        assert!(vm.is_ready_for_allocation());
    }

    #[test]
    fn test_vm_state_transitions() {
        let config = VmConfig {
            vm_id: "state-test".to_string(),
            vm_type: "terraphim-minimal".to_string(),
            memory_mb: 512,
            vcpus: 1,
            kernel_path: Some("/path/to/kernel".to_string()),
            rootfs_path: Some("/path/to/rootfs".to_string()),
            kernel_args: None,
            data_dir: std::path::PathBuf::from("/tmp"),
            enable_networking: false,
        };

        let mut vm = Vm::new("test-type".to_string(), config);

        // Test marking as running
        vm.mark_running(Duration::from_secs(1), Some("172.26.0.10".to_string()));
        assert_eq!(vm.state, VmState::Running);
        assert_eq!(vm.boot_time, Some(Duration::from_secs(1)));
        assert_eq!(vm.ip_address, Some("172.26.0.10".to_string()));

        // Test marking as failed
        vm.mark_failed();
        assert_eq!(vm.state, VmState::Failed);
    }
}
