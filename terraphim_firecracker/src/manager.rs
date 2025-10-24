use anyhow::Result;
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use crate::config::Config;
use crate::performance::{
    BenchmarkResults, BootMetrics, OptimizationStrategy, PerformanceMonitor, Sub2SecondOptimizer,
};
use crate::pool::VmPoolManager;
use crate::vm::{VmInstance, VmManager};

/// Main VM Manager for Terraphim AI with sub-2 second boot optimization
///
/// This manager coordinates all VM operations including:
/// - VM pool management with prewarming
/// - Performance optimization for sub-2 second boot times
/// - Benchmarking and performance monitoring
/// - Integration with terraphim-ai coding assistant
#[allow(dead_code)]
pub struct TerraphimVmManager {
    /// Core VM manager for Firecracker operations
    vm_manager: Arc<dyn VmManager>,
    /// Performance optimizer for sub-2 second boot
    optimizer: Arc<Sub2SecondOptimizer>,
    /// VM pool manager for instant allocation
    pool_manager: Arc<VmPoolManager>,
    /// Performance monitor for tracking metrics
    performance_monitor: Arc<tokio::sync::RwLock<PerformanceMonitor>>,
    /// Application configuration
    config: Config,
}

#[allow(dead_code)]
impl TerraphimVmManager {
    /// Create new Terraphim VM Manager
    pub async fn new(config_path: &str) -> Result<Self> {
        info!(
            "Initializing Terraphim VM Manager with config: {}",
            config_path
        );

        let config = Config::load_from_file(config_path).await?;

        // Initialize core components
        let vm_manager = Self::create_vm_manager(&config).await?;
        let optimizer = Arc::new(Sub2SecondOptimizer::new());
        let pool_manager =
            Self::create_pool_manager(vm_manager.clone(), optimizer.clone(), &config).await?;
        let performance_monitor = Arc::new(tokio::sync::RwLock::new(PerformanceMonitor::new()));

        let manager = Self {
            vm_manager,
            optimizer,
            pool_manager,
            performance_monitor,
            config,
        };

        // Initialize VM pools
        manager.initialize_vm_pools().await?;

        // Pre-warm system resources
        // TODO: Fix prewarm_resources mutability issue
        debug!("System resource prewarming skipped due to mutability constraints");

        info!("Terraphim VM Manager initialized successfully");
        Ok(manager)
    }

    /// Run the VM manager service
    pub async fn run(&self) -> Result<()> {
        info!("Starting Terraphim VM Manager service");

        // Start background tasks
        self.start_background_tasks().await?;

        info!("VM Manager service is running");

        // Keep the service running
        loop {
            sleep(Duration::from_secs(60)).await;

            // Periodic health check
            if let Err(e) = self.perform_health_check().await {
                error!("Health check failed: {}", e);
            }
        }
    }

    /// Test VM creation and boot performance
    pub async fn test_vm(&self, vm_type: &str) -> Result<TestResult> {
        info!("Testing VM creation for type: {}", vm_type);

        let start_time = Instant::now();

        // Get optimized configuration
        let config = self.optimizer.get_optimized_config(vm_type).await?;

        // Create and start VM
        let vm = self.vm_manager.create_vm(&config).await?;
        let boot_time = self.vm_manager.start_vm(&vm.id).await?;

        let total_time = start_time.elapsed();

        // Get VM metrics
        let metrics = self.vm_manager.get_vm_metrics(&vm.id).await?;

        // Record performance metrics
        let mut boot_metrics = BootMetrics::new(vm.id.clone(), vm_type.to_string(), boot_time);
        boot_metrics.memory_usage_mb = metrics.memory_usage_mb;
        boot_metrics.cpu_usage_percent = metrics.cpu_usage_percent;
        boot_metrics.optimization_level = "sub2-second".to_string();

        {
            let mut monitor = self.performance_monitor.write().await;
            monitor.record_metrics(boot_metrics.clone());
        }

        // Clean up
        let _ = self.vm_manager.stop_vm(&vm.id).await;
        let _ = self.vm_manager.delete_vm(&vm.id).await;

        let result = TestResult {
            vm_id: vm.id,
            vm_type: vm_type.to_string(),
            boot_time,
            total_time,
            success: true,
            metrics: boot_metrics,
        };

        info!("Test completed: {}", result.summary());
        Ok(result)
    }

    /// Benchmark VM boot performance
    pub async fn benchmark_boot_performance(&self, trials: u32) -> Result<Vec<BenchmarkResults>> {
        info!(
            "Starting VM boot performance benchmark with {} trials",
            trials
        );

        let vm_types = vec![
            "terraphim-minimal",
            "terraphim-standard",
            "terraphim-development",
        ];

        let mut all_results = Vec::new();

        for vm_type in vm_types {
            info!("Benchmarking VM type: {}", vm_type);

            let mut trial_metrics = Vec::new();

            for trial in 1..=trials {
                debug!("Running trial {} of {} for {}", trial, trials, vm_type);

                match self.test_vm(vm_type).await {
                    Ok(result) => {
                        trial_metrics.push(result.metrics);
                    }
                    Err(e) => {
                        warn!("Trial {} failed for {}: {}", trial, vm_type, e);
                    }
                }

                // Brief pause between trials
                sleep(Duration::from_millis(1000)).await;
            }

            let benchmark_result = BenchmarkResults::new(
                vm_type.to_string(),
                OptimizationStrategy::Sub2Second,
                trial_metrics,
            );

            info!(
                "Benchmark results for {}:\n{}",
                vm_type,
                benchmark_result.summary()
            );
            all_results.push(benchmark_result);
        }

        info!("VM boot performance benchmark completed");
        Ok(all_results)
    }

    /// Initialize VM pools with prewarmed VMs
    pub async fn initialize_pool(&self, pool_size: usize) -> Result<()> {
        info!("Initializing VM pool with {} prewarmed VMs", pool_size);

        let vm_types = vec![
            "terraphim-minimal".to_string(),
            "terraphim-standard".to_string(),
        ];

        // Update pool configuration
        // Note: This would require making pool_config mutable
        // For now, use the default configuration

        // Initialize pools
        self.pool_manager.initialize_pools(vm_types).await?;

        info!("VM pool initialization completed");
        Ok(())
    }

    /// Allocate a VM for terraphim-ai coding assistant
    pub async fn allocate_vm_for_ai(&self, vm_type: &str) -> Result<VmAllocationResult> {
        info!(
            "Allocating VM for terraphim-ai coding assistant: {}",
            vm_type
        );

        let start_time = Instant::now();

        // Try to allocate from pool first
        let (vm, allocation_time) = self.pool_manager.allocate_vm(vm_type).await?;

        let total_time = start_time.elapsed();

        // Record allocation metrics
        let vm_id = vm.read().await.id.clone();
        let allocation_metrics = VmAllocationMetrics {
            vm_id,
            vm_type: vm_type.to_string(),
            allocation_time,
            total_time,
            from_pool: allocation_time <= Duration::from_millis(500),
            boot_time: vm.read().await.boot_time.unwrap_or_default(),
        };

        info!("VM allocated for AI: {}", allocation_metrics.summary());

        Ok(VmAllocationResult {
            vm,
            metrics: allocation_metrics,
        })
    }

    /// Get performance report
    pub async fn get_performance_report(&self) -> Result<String> {
        let monitor = self.performance_monitor.read().await;
        let report = monitor.generate_performance_report();

        // Add pool statistics
        let pool_stats = self.pool_manager.get_pool_stats().await;
        let pool_summary = format!("\n=== Pool Statistics ===\n{}", pool_stats.summary());

        Ok(format!("{}{}", report, pool_summary))
    }

    /// Initialize VM pools
    async fn initialize_vm_pools(&self) -> Result<()> {
        info!("Initializing VM pools");

        let vm_types = vec![
            "terraphim-minimal".to_string(),
            "terraphim-standard".to_string(),
            "terraphim-development".to_string(),
        ];

        self.pool_manager.initialize_pools(vm_types).await?;

        info!("VM pools initialized");
        Ok(())
    }

    /// Create core VM manager
    async fn create_vm_manager(config: &Config) -> Result<Arc<dyn VmManager>> {
        // This would create the actual Firecracker-based VM manager
        // For now, return a placeholder
        use crate::vm::Sub2SecondVmManager;

        let firecracker_socket_dir = std::path::PathBuf::from(&config.firecracker.socket_dir);
        let storage = Arc::new(crate::storage::InMemoryVmStorage::new());

        Ok(Arc::new(
            Sub2SecondVmManager::new(firecracker_socket_dir, storage).await?,
        ))
    }

    /// Create pool manager
    async fn create_pool_manager(
        vm_manager: Arc<dyn VmManager>,
        optimizer: Arc<Sub2SecondOptimizer>,
        config: &Config,
    ) -> Result<Arc<VmPoolManager>> {
        let pool_config = crate::pool::PoolConfig {
            min_pool_size: config.pool.min_size,
            max_pool_size: config.pool.max_size,
            target_pool_size: config.pool.target_size,
            max_prewarmed_age: Duration::from_secs(config.pool.max_age_seconds),
            health_check_interval: Duration::from_secs(config.pool.health_check_interval_seconds),
            prewarming_interval: Duration::from_secs(config.pool.prewarming_interval_seconds),
            allocation_timeout: Duration::from_millis(config.pool.allocation_timeout_ms),
            enable_snapshots: config.pool.enable_snapshots,
        };

        Ok(Arc::new(VmPoolManager::new(
            vm_manager,
            optimizer,
            pool_config,
        )))
    }

    /// Start background tasks
    async fn start_background_tasks(&self) -> Result<()> {
        info!("Starting background tasks");

        // Performance monitoring task
        let performance_monitor = self.performance_monitor.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes

            loop {
                interval.tick().await;

                // Generate performance report
                let monitor = performance_monitor.read().await;
                let report = monitor.generate_performance_report();
                debug!("Performance report:\n{}", report);
            }
        });

        info!("Background tasks started");
        Ok(())
    }

    /// Perform health check
    async fn perform_health_check(&self) -> Result<()> {
        debug!("Performing health check");

        // Check pool health
        let pool_stats = self.pool_manager.get_pool_stats().await;
        info!("Pool health: {}", pool_stats.summary());

        // Check system resources
        // This would include memory, CPU, disk checks

        Ok(())
    }
}

/// Test result for VM creation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestResult {
    pub vm_id: String,
    pub vm_type: String,
    pub boot_time: Duration,
    pub total_time: Duration,
    pub success: bool,
    pub metrics: BootMetrics,
}

#[allow(dead_code)]
impl TestResult {
    pub fn summary(&self) -> String {
        format!(
            "VM {} ({}): Boot {:.3}s, Total {:.3}s - {}",
            self.vm_id,
            self.vm_type,
            self.boot_time.as_secs_f64(),
            self.total_time.as_secs_f64(),
            if self.success {
                "✅ SUCCESS"
            } else {
                "❌ FAILED"
            }
        )
    }
}

/// VM allocation result
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VmAllocationResult {
    pub vm: VmInstance,
    pub metrics: VmAllocationMetrics,
}

/// VM allocation metrics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VmAllocationMetrics {
    pub vm_id: String,
    pub vm_type: String,
    pub allocation_time: Duration,
    pub total_time: Duration,
    pub from_pool: bool,
    pub boot_time: Duration,
}

#[allow(dead_code)]
impl VmAllocationMetrics {
    pub fn summary(&self) -> String {
        format!(
            "VM {} ({}): Allocation {:.3}s, Total {:.3}s, Boot {:.3}s - {}",
            self.vm_id,
            self.vm_type,
            self.allocation_time.as_secs_f64(),
            self.total_time.as_secs_f64(),
            self.boot_time.as_secs_f64(),
            if self.from_pool {
                "🏊 FROM POOL"
            } else {
                "🆕 CREATED"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_allocation_metrics() {
        let metrics = VmAllocationMetrics {
            vm_id: "test-vm".to_string(),
            vm_type: "terraphim-minimal".to_string(),
            allocation_time: Duration::from_millis(300),
            total_time: Duration::from_millis(800),
            from_pool: true,
            boot_time: Duration::from_millis(1500),
        };

        assert!(metrics.from_pool);
        assert_eq!(metrics.vm_type, "terraphim-minimal");

        let summary = metrics.summary();
        assert!(summary.contains("FROM POOL"));
        assert!(summary.contains("0.300s"));
    }

    #[test]
    fn test_test_result() {
        let metrics = BootMetrics::new(
            "test-vm".to_string(),
            "terraphim-minimal".to_string(),
            Duration::from_millis(1800),
        );
        let result = TestResult {
            vm_id: "test-vm".to_string(),
            vm_type: "terraphim-minimal".to_string(),
            boot_time: Duration::from_millis(1800),
            total_time: Duration::from_millis(2000),
            success: true,
            metrics,
        };

        assert!(result.success);
        assert_eq!(result.boot_time, Duration::from_millis(1800));

        let summary = result.summary();
        assert!(summary.contains("SUCCESS"));
        assert!(summary.contains("1.800s"));
    }
}
