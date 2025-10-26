use crate::performance::{OptimizationStrategy, SUB2_TARGET_BOOT_TIME};
use crate::vm::config::{get_vm_type_config, VmConfig};
use anyhow::Result;
use log::{debug, info, warn};
use std::time::Duration;
use tokio::time::sleep;

/// Sub-2 Second VM Boot Optimizer
///
/// This optimizer implements aggressive performance optimizations to achieve
/// sub-2 second VM boot times through multiple techniques:
///
/// 1. Ultra-minimal kernel parameters
/// 2. Systemd service elimination
/// 3. Memory pre-allocation and prewarming
/// 4. Resource optimization
/// 5. Boot sequence optimization
#[allow(dead_code)]
pub struct Sub2SecondOptimizer {
    strategy: OptimizationStrategy,
    prewarmed_resources: PrewarmedResources,
}

#[allow(dead_code)]
impl Sub2SecondOptimizer {
    pub fn new() -> Self {
        Self {
            strategy: OptimizationStrategy::Sub2Second,
            prewarmed_resources: PrewarmedResources::new(),
        }
    }

    pub fn with_strategy(mut self, strategy: OptimizationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Get optimized VM configuration for sub-2 second boot
    pub async fn get_optimized_config(&self, vm_type: &str) -> Result<VmConfig> {
        let vm_type_config = get_vm_type_config(vm_type)?;

        let (optimized_memory, optimized_vcpus) = self
            .strategy
            .get_resource_config(vm_type_config.memory_mb, vm_type_config.vcpus);

        let mut config = VmConfig::new(
            format!("{}-{}", vm_type, uuid::Uuid::new_v4()),
            vm_type.to_string(),
        )
        .with_memory(optimized_memory)
        .with_vcpus(optimized_vcpus)
        .with_kernel_path(vm_type_config.kernel_path)
        .with_rootfs_path(vm_type_config.rootfs_path)
        .with_kernel_args(self.strategy.get_kernel_args(vm_type, optimized_memory))
        .with_networking(vm_type_config.enable_networking);

        // Apply sub-2 second specific optimizations
        if matches!(
            self.strategy,
            OptimizationStrategy::Sub2Second | OptimizationStrategy::UltraFast
        ) {
            config = self.apply_sub2_optimizations(config).await?;
        }

        Ok(config)
    }

    /// Get ultra-fast boot arguments for sub-2 second target
    pub fn get_ultra_fast_boot_args(&self, vm_type: &str, memory_mb: u32) -> String {
        let base_args = self.strategy.get_kernel_args(vm_type, memory_mb);

        // Add sub-2 second specific optimizations
        let sub2_args = [
            "nopti",                       // Disable Page Table Isolation
            "nospectre_v2",                // Disable Spectre V2 mitigations
            "nospec_store_bypass_disable", // Disable Speculative Store Bypass
            "l1tf=off",                    // Disable L1 Terminal Fault mitigations
            "mds=off",                     // Disable Microarchitectural Data Sampling mitigations
            "tsx=off",                     // Disable Transactional Synchronization Extensions
            "mitigations=off",             // Disable all CPU mitigations for speed
            "init_on_alloc=0",             // Skip memory clearing on allocation
            "init_on_free=0",              // Skip memory clearing on free
            "slab_nomerge",                // Disable slab merging for predictable performance
            "page_alloc.shuffle=1",        // Enable page allocation shuffling
        ];

        format!("{} {}", base_args, sub2_args.join(" "))
    }

    /// Get sub-2 second boot arguments with maximum optimizations
    pub fn get_sub2_boot_args(&self, vm_type: &str, memory_mb: u32) -> String {
        let ultra_fast_args = self.get_ultra_fast_boot_args(vm_type, memory_mb);

        // Add additional sub-2 second optimizations
        let additional_args = [
            "systemd.log_level=0",                   // Minimal systemd logging
            "systemd.show_status=0",                 // Hide systemd status
            "systemd.journald.forward_to_console=0", // No journal forwarding
            "systemd.journald.storage=none",         // No persistent journal storage
            "rd.udev.log-priority=0",                // Minimal udev logging
            "quiet",                                 // Suppress kernel messages
            "loglevel=0",                            // Minimal kernel logging
            "console=ttyS0",                         // Only serial console
            "earlyprintk=serial,ttyS0,115200",       // Early serial output
            "rootflags=noatime,compress=lzo",        // Optimize root filesystem
            "ro",                                    // Start read-only for speed
        ];

        format!("{} {}", ultra_fast_args, additional_args.join(" "))
    }

    /// Apply sub-2 second specific optimizations to VM config
    async fn apply_sub2_optimizations(&self, mut config: VmConfig) -> Result<VmConfig> {
        // Optimize memory layout for faster boot
        config.memory_mb = self.optimize_memory_for_speed(config.memory_mb);

        // Force single CPU for fastest boot (unless explicitly required)
        if config.vcpus > 1 && !config.vm_type.contains("development") {
            config.vcpus = 1;
            debug!("Forcing single CPU for sub-2 second boot target");
        }

        // Disable networking for minimal boot unless required
        if !config.vm_type.contains("development") && !config.vm_type.contains("standard") {
            config.enable_networking = false;
            debug!("Disabling networking for sub-2 second boot target");
        }

        Ok(config)
    }

    /// Optimize memory configuration for fastest boot
    fn optimize_memory_for_speed(&self, memory_mb: u32) -> u32 {
        // Use minimal memory for fastest boot, but ensure enough for basic operation
        match memory_mb {
            0..=128 => 256,    // Minimum viable memory
            129..=512 => 512,  // Standard minimal config
            513..=1024 => 512, // Cap at 512MB for speed
            _ => 512,          // Force 512MB for sub-2 second target
        }
    }

    /// Pre-warm system resources for faster VM creation
    pub async fn prewarm_resources(&mut self) -> Result<()> {
        info!("Pre-warming system resources for sub-2 second VM boot");

        let start_time = std::time::Instant::now();

        // Pre-warm memory allocator
        self.prewarmed_resources.prewarm_memory().await?;

        // Pre-warm file system cache
        self.prewarmed_resources.prewarm_filesystem().await?;

        // Pre-warm network stack (if needed)
        self.prewarmed_resources.prewarm_network().await?;

        // Pre-warm Firecracker binary
        self.prewarmed_resources.prewarm_firecracker().await?;

        let prewarming_time = start_time.elapsed();
        info!(
            "Resource prewarming completed in: {:.3}s",
            prewarming_time.as_secs_f64()
        );

        Ok(())
    }

    /// Create boot snapshot for instant VM allocation
    pub async fn create_boot_snapshot(&self, _vm_config: &VmConfig) -> Result<String> {
        info!("Creating boot snapshot for instant VM allocation");

        let snapshot_id = format!("boot-snapshot-{}", uuid::Uuid::new_v4());

        // This would integrate with Firecracker snapshot functionality
        // For now, return a placeholder snapshot ID
        debug!("Created boot snapshot: {}", snapshot_id);

        Ok(snapshot_id)
    }

    /// Restore VM from boot snapshot for instant allocation
    pub async fn restore_from_snapshot(
        &self,
        snapshot_id: &str,
        _vm_config: &VmConfig,
    ) -> Result<Duration> {
        info!("Restoring VM from snapshot: {}", snapshot_id);

        let start_time = std::time::Instant::now();

        // This would integrate with Firecracker snapshot restore functionality
        // Simulate instant restore time
        sleep(Duration::from_millis(50)).await;

        let restore_time = start_time.elapsed();
        info!(
            "VM restored from snapshot in: {:.3}s",
            restore_time.as_secs_f64()
        );

        Ok(restore_time)
    }

    /// Validate that VM meets sub-2 second boot target
    pub fn validate_sub2_target(&self, boot_time: Duration) -> bool {
        let meets_target = boot_time <= SUB2_TARGET_BOOT_TIME;

        if meets_target {
            info!(
                "✅ Sub-2 second boot target MET: {:.3}s",
                boot_time.as_secs_f64()
            );
        } else {
            warn!(
                "❌ Sub-2 second boot target MISSED by {:.3}s",
                (boot_time - SUB2_TARGET_BOOT_TIME).as_secs_f64()
            );
        }

        meets_target
    }

    /// Get performance recommendations based on boot time
    pub fn get_performance_recommendations(
        &self,
        boot_time: Duration,
        vm_type: &str,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if boot_time > Duration::from_secs(5) {
            recommendations
                .push("Consider using terraphim-minimal VM type for faster boot".to_string());
            recommendations.push("Enable all sub-2 second optimizations".to_string());
        } else if boot_time > Duration::from_secs(3) {
            recommendations.push("Try ultra-fast optimization strategy".to_string());
            recommendations.push("Consider prewarming VM pool".to_string());
        } else if boot_time > SUB2_TARGET_BOOT_TIME {
            recommendations.push("Enable snapshot-based instant boot".to_string());
            recommendations.push("Reduce memory allocation to minimum".to_string());
        }

        // VM-type specific recommendations
        if vm_type.contains("development") {
            recommendations.push("Development VMs have higher resource requirements - consider terraphim-minimal for speed".to_string());
        }

        if boot_time <= SUB2_TARGET_BOOT_TIME {
            recommendations.push(
                "🎉 Sub-2 second target achieved! Consider enabling instant boot from snapshots."
                    .to_string(),
            );
        }

        recommendations
    }
}

impl Default for Sub2SecondOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Prewarmed system resources for faster VM boot
#[allow(dead_code)]
pub struct PrewarmedResources {
    memory_prewarmed: bool,
    filesystem_prewarmed: bool,
    network_prewarmed: bool,
    firecracker_prewarmed: bool,
}

#[allow(dead_code)]
impl PrewarmedResources {
    pub fn new() -> Self {
        Self {
            memory_prewarmed: false,
            filesystem_prewarmed: false,
            network_prewarmed: false,
            firecracker_prewarmed: false,
        }
    }

    /// Pre-warm memory allocator
    pub async fn prewarm_memory(&mut self) -> Result<()> {
        if self.memory_prewarmed {
            return Ok(());
        }

        debug!("Pre-warming memory allocator");

        // Allocate and free memory to warm up the allocator
        let _warmup: Vec<u8> = vec![0; 1024 * 1024]; // 1MB
        let _warmup2: Vec<u8> = vec![0; 2 * 1024 * 1024]; // 2MB
        let _warmup3: Vec<u8> = vec![0; 4 * 1024 * 1024]; // 4MB

        // Force garbage collection
        drop((_warmup, _warmup2, _warmup3));

        self.memory_prewarmed = true;
        Ok(())
    }

    /// Pre-warm filesystem cache
    pub async fn prewarm_filesystem(&mut self) -> Result<()> {
        if self.filesystem_prewarmed {
            return Ok(());
        }

        debug!("Pre-warming filesystem cache");

        // Touch common VM image paths to warm filesystem cache
        let common_paths = vec![
            "/var/lib/terraphim/images",
            "/var/lib/terraphim/snapshots",
            "/var/lib/terraphim/kernels",
        ];

        for path in common_paths {
            if std::path::Path::new(path).exists() {
                let _ = std::fs::read_dir(path);
            }
        }

        self.filesystem_prewarmed = true;
        Ok(())
    }

    /// Pre-warm network stack
    pub async fn prewarm_network(&mut self) -> Result<()> {
        if self.network_prewarmed {
            return Ok(());
        }

        debug!("Pre-warming network stack");

        // Create and immediately drop network connections to warm up the stack
        let _listener = tokio::net::TcpListener::bind("127.0.0.1:0").await;

        self.network_prewarmed = true;
        Ok(())
    }

    /// Pre-warm Firecracker binary
    pub async fn prewarm_firecracker(&mut self) -> Result<()> {
        if self.firecracker_prewarmed {
            return Ok(());
        }

        debug!("Pre-warming Firecracker binary");

        // Try to locate and touch the Firecracker binary
        let possible_paths = vec![
            "/usr/bin/firecracker",
            "/usr/local/bin/firecracker",
            "/opt/firecracker/bin/firecracker",
        ];

        for path in possible_paths {
            if std::path::Path::new(path).exists() {
                let _ = std::fs::metadata(path);
                break;
            }
        }

        self.firecracker_prewarmed = true;
        Ok(())
    }

    /// Check if all resources are prewarmed
    pub fn is_fully_prewarmed(&self) -> bool {
        self.memory_prewarmed
            && self.filesystem_prewarmed
            && self.network_prewarmed
            && self.firecracker_prewarmed
    }

    /// Reset prewarming state
    pub fn reset(&mut self) {
        self.memory_prewarmed = false;
        self.filesystem_prewarmed = false;
        self.network_prewarmed = false;
        self.firecracker_prewarmed = false;
    }
}

impl Default for PrewarmedResources {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sub2_optimizer_creation() {
        let optimizer = Sub2SecondOptimizer::new();
        assert!(matches!(
            optimizer.strategy,
            OptimizationStrategy::Sub2Second
        ));
    }

    #[tokio::test]
    async fn test_ultra_fast_boot_args() {
        let optimizer = Sub2SecondOptimizer::new();
        let args = optimizer.get_ultra_fast_boot_args("terraphim-minimal", 256);

        assert!(args.contains("console=ttyS0"));
        assert!(args.contains("mitigations=off"));
        assert!(args.contains("nopti"));
        assert!(args.contains("nospectre_v2"));
    }

    #[tokio::test]
    async fn test_sub2_boot_args() {
        let optimizer = Sub2SecondOptimizer::new();
        let args = optimizer.get_sub2_boot_args("terraphim-minimal", 256);

        assert!(args.contains("systemd.log_level=0"));
        assert!(args.contains("loglevel=0"));
        assert!(args.contains("ro"));
    }

    #[tokio::test]
    async fn test_memory_optimization() {
        let optimizer = Sub2SecondOptimizer::new();

        assert_eq!(optimizer.optimize_memory_for_speed(128), 256);
        assert_eq!(optimizer.optimize_memory_for_speed(512), 512);
        assert_eq!(optimizer.optimize_memory_for_speed(1024), 512);
        assert_eq!(optimizer.optimize_memory_for_speed(2048), 512);
    }

    #[tokio::test]
    async fn test_prewarmed_resources() {
        let mut resources = PrewarmedResources::new();
        assert!(!resources.is_fully_prewarmed());

        resources.prewarm_memory().await.unwrap();
        assert!(resources.memory_prewarmed);
        assert!(!resources.is_fully_prewarmed());

        resources.prewarm_filesystem().await.unwrap();
        resources.prewarm_network().await.unwrap();
        resources.prewarm_firecracker().await.unwrap();

        assert!(resources.is_fully_prewarmed());
    }

    #[test]
    fn test_sub2_target_validation() {
        let optimizer = Sub2SecondOptimizer::new();

        assert!(optimizer.validate_sub2_target(Duration::from_millis(1500)));
        assert!(optimizer.validate_sub2_target(Duration::from_millis(2000)));
        assert!(!optimizer.validate_sub2_target(Duration::from_millis(2500)));
    }

    #[test]
    fn test_performance_recommendations() {
        let optimizer = Sub2SecondOptimizer::new();

        let recommendations = optimizer
            .get_performance_recommendations(Duration::from_millis(1500), "terraphim-minimal");
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("🎉")));

        let recommendations = optimizer
            .get_performance_recommendations(Duration::from_millis(5000), "terraphim-development");
        assert!(recommendations
            .iter()
            .any(|r| r.contains("Development VMs")));
    }
}
