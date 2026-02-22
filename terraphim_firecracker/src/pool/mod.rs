use anyhow::Result;
use dashmap::DashMap;
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::performance::{Sub2SecondOptimizer, PREWARMED_ALLOCATION_TARGET};
use crate::vm::{Vm, VmInstance, VmManager, VmState};

pub mod allocation;
pub mod maintenance;
pub mod prewarming;

pub use crate::performance::PrewarmingManager;
pub use allocation::VmAllocator;
pub use maintenance::VmMaintenanceManager;

/// VM Pool Manager for sub-2 second VM allocation
///
/// This manager maintains a pool of prewarmed VMs ready for instant allocation,
/// enabling sub-500ms VM startup times for terraphim-ai coding assistant.
#[allow(dead_code)]
pub struct VmPoolManager {
    /// Pool of prewarmed VMs by type
    prewarmed_pools: DashMap<String, Arc<RwLock<Vec<PrewarmedVm>>>>,
    /// VM allocator for managing pool allocation
    allocator: Arc<VmAllocator>,
    /// Prewarming manager for maintaining pool levels
    prewarming_manager: Arc<PrewarmingManager>,
    /// Maintenance manager for pool health
    maintenance_manager: Arc<VmMaintenanceManager>,
    /// Performance optimizer
    optimizer: Arc<Sub2SecondOptimizer>,
    /// Pool configuration
    config: PoolConfig,
}

/// A prewarmed VM ready for instant allocation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PrewarmedVm {
    pub vm: Vm,
    pub prewarmed_at: Instant,
    pub last_health_check: Instant,
    pub allocation_count: u32,
    pub snapshot_id: Option<String>,
    pub ready_state: PrewarmedState,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum PrewarmedState {
    /// VM is created and ready to start
    Ready,
    /// VM is running and ready for immediate use
    Running,
    /// VM has snapshot ready for instant restore
    Snapshotted,
    /// VM is being allocated
    Allocating,
    /// VM needs maintenance
    NeedsMaintenance,
}

/// Pool configuration for managing VM pools
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PoolConfig {
    /// Minimum pool size per VM type
    pub min_pool_size: usize,
    /// Maximum pool size per VM type
    pub max_pool_size: usize,
    /// Target pool size (ideal number of prewarmed VMs)
    pub target_pool_size: usize,
    /// Maximum age of prewarmed VMs before refresh
    pub max_prewarmed_age: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Prewarming interval
    pub prewarming_interval: Duration,
    /// Allocation timeout
    pub allocation_timeout: Duration,
    /// Enable snapshot-based instant boot
    pub enable_snapshots: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_pool_size: 2,
            max_pool_size: 10,
            target_pool_size: 5,
            max_prewarmed_age: Duration::from_secs(300), // 5 minutes
            health_check_interval: Duration::from_secs(30),
            prewarming_interval: Duration::from_secs(60),
            allocation_timeout: Duration::from_millis(500),
            enable_snapshots: true,
        }
    }
}

#[allow(dead_code)]
impl VmPoolManager {
    pub fn new(
        _vm_manager: Arc<dyn VmManager>,
        optimizer: Arc<Sub2SecondOptimizer>,
        config: PoolConfig,
    ) -> Self {
        let performance_monitor = Arc::new(crate::performance::PerformanceMonitor::new());
        let strategy = crate::pool::allocation::AllocationStrategy::FirstAvailable;
        let allocator = Arc::new(VmAllocator::new(performance_monitor.clone(), strategy));
        let prewarming_manager = Arc::new(PrewarmingManager::new(
            5,
            std::time::Duration::from_secs(300),
        ));
        let maintenance_manager = Arc::new(VmMaintenanceManager::new(performance_monitor));

        Self {
            prewarmed_pools: DashMap::new(),
            allocator,
            prewarming_manager,
            maintenance_manager,
            optimizer,
            config,
        }
    }

    /// Initialize VM pools for specified VM types
    pub async fn initialize_pools(&self, vm_types: Vec<String>) -> Result<()> {
        info!("Initializing VM pools for types: {:?}", vm_types);

        for vm_type in vm_types {
            self.prewarmed_pools
                .insert(vm_type.clone(), Arc::new(RwLock::new(Vec::new())));
            info!("Created pool for VM type: {}", vm_type);
        }

        // Start background tasks
        self.start_background_tasks().await?;

        info!("VM pools initialized successfully");
        Ok(())
    }

    /// Allocate a VM from the prewarmed pool
    pub async fn allocate_vm(&self, vm_type: &str) -> Result<(VmInstance, Duration)> {
        let start_time = Instant::now();

        info!("Allocating VM from pool for type: {}", vm_type);

        // Try to allocate from prewarmed pool
        if let Some(pool) = self.prewarmed_pools.get(vm_type) {
            let mut pool_guard = pool.write().await;

            // Find the best candidate for allocation
            if let Some(index) = self.find_best_allocation_candidate(&pool_guard) {
                let mut prewarmed_vm = pool_guard.swap_remove(index);
                prewarmed_vm.ready_state = PrewarmedState::Allocating;

                let allocation_start = Instant::now();

                // Allocate the VM
                let vm = self.allocator.allocate_prewarmed_vm(&prewarmed_vm).await?;

                let allocation_time = allocation_start.elapsed();
                let total_time = start_time.elapsed();

                info!(
                    "VM allocated from pool in: {:.3}s (allocation: {:.3}s)",
                    total_time.as_secs_f64(),
                    allocation_time.as_secs_f64()
                );

                // Check if we met the sub-500ms allocation target
                if total_time > PREWARMED_ALLOCATION_TARGET {
                    warn!(
                        "Pool allocation exceeded target: {:.3}s > {:.3}s",
                        total_time.as_secs_f64(),
                        PREWARMED_ALLOCATION_TARGET.as_secs_f64()
                    );
                } else {
                    info!(
                        "✅ Pool allocation target met: {:.3}s",
                        total_time.as_secs_f64()
                    );
                }

                return Ok((vm, total_time));
            }
        }

        // No prewarmed VM available, create new one
        warn!(
            "No prewarmed VM available for type: {}, creating new VM",
            vm_type
        );
        self.create_and_allocate_vm(vm_type).await
    }

    /// Return a VM to the pool (if applicable)
    pub async fn return_vm_to_pool(&self, vm: Vm) -> Result<()> {
        info!("Returning VM {} to pool", vm.id);

        // Only return VMs that are in a good state
        if vm.state != VmState::Running {
            warn!("VM {} not in running state, not returning to pool", vm.id);
            return Ok(());
        }

        // Check if VM type has a pool
        if let Some(pool) = self.prewarmed_pools.get(&vm.vm_type) {
            let prewarmed_vm = PrewarmedVm {
                vm,
                prewarmed_at: Instant::now(),
                last_health_check: Instant::now(),
                allocation_count: 0,
                snapshot_id: None,
                ready_state: PrewarmedState::Running,
            };

            let mut pool_guard = pool.write().await;

            // Check pool size limit
            if pool_guard.len() < self.config.max_pool_size {
                pool_guard.push(prewarmed_vm);
                info!("VM returned to pool, pool size: {}", pool_guard.len());
            } else {
                info!("Pool at capacity, not returning VM");
            }
        }

        Ok(())
    }

    /// Get pool statistics
    pub async fn get_pool_stats(&self) -> PoolStats {
        let mut stats = PoolStats::new();

        for entry in self.prewarmed_pools.iter() {
            let vm_type = entry.key().clone();
            let pool_guard = entry.value().read().await;

            let pool_stats = PoolTypeStats {
                vm_type: vm_type.clone(),
                total_vms: pool_guard.len(),
                ready_vms: pool_guard
                    .iter()
                    .filter(|vm| vm.ready_state == PrewarmedState::Ready)
                    .count(),
                running_vms: pool_guard
                    .iter()
                    .filter(|vm| vm.ready_state == PrewarmedState::Running)
                    .count(),
                snapshotted_vms: pool_guard
                    .iter()
                    .filter(|vm| vm.ready_state == PrewarmedState::Snapshotted)
                    .count(),
                average_age: if pool_guard.is_empty() {
                    Duration::ZERO
                } else {
                    let total_age: Duration =
                        pool_guard.iter().map(|vm| vm.prewarmed_at.elapsed()).sum();
                    total_age / pool_guard.len() as u32
                },
            };

            stats.type_stats.insert(vm_type, pool_stats);
        }

        stats
    }

    /// Start background maintenance tasks
    async fn start_background_tasks(&self) -> Result<()> {
        info!("Starting pool background tasks");

        // Start prewarming task
        let prewarming_manager = self.prewarming_manager.clone();
        let prewarming_interval = self.config.prewarming_interval;
        let prewarmed_pools = self.prewarmed_pools.clone();
        let target_pool_size = self.config.target_pool_size;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(prewarming_interval);

            loop {
                interval.tick().await;

                // Convert DashMap to HashMap for method call
                let pools_map: std::collections::HashMap<_, _> = prewarmed_pools
                    .iter()
                    .map(|entry| (entry.key().clone(), entry.value().clone()))
                    .collect();

                if let Err(e) = prewarming_manager
                    .maintain_pool_levels(&pools_map, target_pool_size)
                    .await
                {
                    error!("Prewarming maintenance error: {}", e);
                }
            }
        });

        // Start health check task
        let maintenance_manager = self.maintenance_manager.clone();
        let health_check_interval = self.config.health_check_interval;
        let prewarmed_pools = self.prewarmed_pools.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(health_check_interval);

            loop {
                interval.tick().await;

                // Convert DashMap to HashMap for method call
                let pools_map: std::collections::HashMap<_, _> = prewarmed_pools
                    .iter()
                    .map(|entry| (entry.key().clone(), entry.value().clone()))
                    .collect();

                if let Err(e) = maintenance_manager.perform_health_checks(&pools_map).await {
                    error!("Health check maintenance error: {}", e);
                }
            }
        });

        info!("Background tasks started");
        Ok(())
    }

    /// Find the best VM candidate for allocation from the pool
    fn find_best_allocation_candidate(&self, pool: &[PrewarmedVm]) -> Option<usize> {
        if pool.is_empty() {
            return None;
        }

        // Prioritize: Snapshotted > Running > Ready
        // Within each category, choose the least recently allocated
        let mut best_index = 0;
        let mut best_score = 0;

        for (index, vm) in pool.iter().enumerate() {
            let score = match vm.ready_state {
                PrewarmedState::Snapshotted => 1000 - vm.allocation_count,
                PrewarmedState::Running => 500 - vm.allocation_count,
                PrewarmedState::Ready => 100 - vm.allocation_count,
                _ => 0, // Don't allocate VMs that need maintenance
            };

            if score > best_score {
                best_score = score;
                best_index = index;
            }
        }

        if best_score > 0 {
            Some(best_index)
        } else {
            None
        }
    }

    /// Create and allocate a new VM when pool is empty
    async fn create_and_allocate_vm(&self, vm_type: &str) -> Result<(VmInstance, Duration)> {
        let start_time = Instant::now();

        info!("Creating new VM for type: {}", vm_type);

        // Get optimized configuration
        let config = self.optimizer.get_optimized_config(vm_type).await?;

        // Create and start VM
        let vm = self.allocator.create_and_start_vm(&config).await?;

        let total_time = start_time.elapsed();

        info!(
            "New VM created and started in: {:.3}s",
            total_time.as_secs_f64()
        );

        Ok((vm, total_time))
    }

    /// Shutdown the pool manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down VM pool manager");

        // Stop all VMs in pools
        for entry in self.prewarmed_pools.iter() {
            let vm_type = entry.key();
            let pool_guard = entry.value().read().await;

            info!(
                "Stopping {} VMs in pool for type: {}",
                pool_guard.len(),
                vm_type
            );

            for prewarmed_vm in pool_guard.iter() {
                // Note: VM stopping would be handled by the VM manager
                debug!("Stopping VM: {}", prewarmed_vm.vm.id);
            }
        }

        info!("VM pool manager shutdown complete");
        Ok(())
    }
}

/// Pool statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    pub type_stats: std::collections::HashMap<String, PoolTypeStats>,
    pub total_vms: usize,
    pub total_ready_vms: usize,
    pub total_running_vms: usize,
    pub total_snapshotted_vms: usize,
}

impl PoolStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn summary(&self) -> String {
        format!(
            "Pool Stats - Total: {} (Ready: {}, Running: {}, Snapshotted: {})",
            self.total_vms,
            self.total_ready_vms,
            self.total_running_vms,
            self.total_snapshotted_vms
        )
    }
}

/// Statistics for a specific VM type
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PoolTypeStats {
    pub vm_type: String,
    pub total_vms: usize,
    pub ready_vms: usize,
    pub running_vms: usize,
    pub snapshotted_vms: usize,
    pub average_age: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{VmConfig, VmManager};
    use std::sync::Arc;

    // Mock VM manager for testing
    struct MockVmManager;

    #[async_trait::async_trait]
    impl VmManager for MockVmManager {
        async fn create_vm(&self, _config: &VmConfig) -> Result<Vm> {
            Ok(Vm::new(
                "test-type".to_string(),
                VmConfig {
                    vm_id: "test-vm".to_string(),
                    vm_type: "test-type".to_string(),
                    memory_mb: 512,
                    vcpus: 1,
                    kernel_path: None,
                    rootfs_path: None,
                    kernel_args: None,
                    data_dir: std::path::PathBuf::from("/tmp"),
                    enable_networking: false,
                },
            ))
        }

        async fn start_vm(&self, _vm_id: &str) -> Result<Duration> {
            Ok(Duration::from_millis(100))
        }

        async fn stop_vm(&self, _vm_id: &str) -> Result<()> {
            Ok(())
        }

        async fn delete_vm(&self, _vm_id: &str) -> Result<()> {
            Ok(())
        }

        async fn get_vm(&self, _vm_id: &str) -> Result<Option<Vm>> {
            Ok(None)
        }

        async fn list_vms(&self) -> Result<Vec<Vm>> {
            Ok(vec![])
        }

        async fn get_vm_metrics(&self, _vm_id: &str) -> Result<crate::vm::VmMetrics> {
            Err(anyhow::anyhow!("Not implemented"))
        }
    }

    #[tokio::test]
    async fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.min_pool_size, 2);
        assert_eq!(config.max_pool_size, 10);
        assert_eq!(config.target_pool_size, 5);
    }

    #[tokio::test]
    async fn test_prewarmed_vm_creation() {
        let vm = Vm::new(
            "test-type".to_string(),
            VmConfig {
                vm_id: "test-vm".to_string(),
                vm_type: "test-type".to_string(),
                memory_mb: 512,
                vcpus: 1,
                kernel_path: None,
                rootfs_path: None,
                kernel_args: None,
                data_dir: std::path::PathBuf::from("/tmp"),
                enable_networking: false,
            },
        );
        let prewarmed_vm = PrewarmedVm {
            vm: vm.clone(),
            prewarmed_at: Instant::now(),
            last_health_check: Instant::now(),
            allocation_count: 0,
            snapshot_id: None,
            ready_state: PrewarmedState::Ready,
        };

        assert_eq!(prewarmed_vm.vm.id, vm.id);
        assert_eq!(prewarmed_vm.ready_state, PrewarmedState::Ready);
        assert_eq!(prewarmed_vm.allocation_count, 0);
    }

    #[tokio::test]
    async fn test_pool_stats() {
        let mut stats = PoolStats::new();
        assert_eq!(stats.total_vms, 0);

        let type_stats = PoolTypeStats {
            vm_type: "test-type".to_string(),
            total_vms: 5,
            ready_vms: 2,
            running_vms: 2,
            snapshotted_vms: 1,
            average_age: Duration::from_secs(60),
        };

        stats.type_stats.insert("test-type".to_string(), type_stats);
        assert_eq!(stats.type_stats.len(), 1);
    }

    #[tokio::test]
    async fn test_vm_pool_manager_creation() {
        let vm_manager = Arc::new(MockVmManager);
        let optimizer = Arc::new(Sub2SecondOptimizer::new());
        let config = PoolConfig::default();

        let pool_manager = VmPoolManager::new(vm_manager, optimizer, config);

        // Test that pools are empty initially
        let stats = pool_manager.get_pool_stats().await;
        assert_eq!(stats.total_vms, 0);
    }
}
