use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub mod optimizer;
pub mod prewarming;
pub mod profiler;

pub use optimizer::Sub2SecondOptimizer;
pub use prewarming::PrewarmingManager;

/// Performance target constants for sub-2 second VM boot
pub const SUB2_TARGET_BOOT_TIME: Duration = Duration::from_secs(2);
pub const ULTRA_FAST_TARGET_BOOT_TIME: Duration = Duration::from_millis(1500);
pub const PREWARMED_ALLOCATION_TARGET: Duration = Duration::from_millis(500);

/// Boot performance levels
#[derive(Debug, Clone, PartialEq)]
pub enum BootPerformanceLevel {
    Sub2Second, // <2000ms
    UltraFast,  // <1500ms
    Prewarmed,  // <500ms from pool
    Instant,    // <100ms from snapshot
}

impl BootPerformanceLevel {
    pub fn target_time(&self) -> Duration {
        match self {
            BootPerformanceLevel::Sub2Second => SUB2_TARGET_BOOT_TIME,
            BootPerformanceLevel::UltraFast => ULTRA_FAST_TARGET_BOOT_TIME,
            BootPerformanceLevel::Prewarmed => PREWARMED_ALLOCATION_TARGET,
            BootPerformanceLevel::Instant => Duration::from_millis(100),
        }
    }

    pub fn meets_target(&self, actual_time: Duration) -> bool {
        actual_time <= self.target_time()
    }

    pub fn get_level(boot_time: Duration) -> Self {
        if boot_time <= Duration::from_millis(100) {
            BootPerformanceLevel::Instant
        } else if boot_time <= PREWARMED_ALLOCATION_TARGET {
            BootPerformanceLevel::Prewarmed
        } else if boot_time <= ULTRA_FAST_TARGET_BOOT_TIME {
            BootPerformanceLevel::UltraFast
        } else {
            BootPerformanceLevel::Sub2Second // Default to sub-2 target
        }
    }
}

/// General performance metrics for VM operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PerformanceMetrics {
    pub vm_id: String,
    pub operation_type: String,
    pub duration: Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub success: bool,
    pub metadata: HashMap<String, String>,
    // VM lifecycle tracking fields
    pub boot_time: Option<Duration>,
    #[serde(skip)]
    pub last_allocated: Option<std::time::Instant>,
    #[serde(skip)]
    pub last_used: Option<std::time::Instant>,
    #[serde(skip)]
    pub last_prewarmed: Option<std::time::Instant>,
    pub usage_count: Option<u32>,
    // Maintenance tracking fields
    #[serde(skip)]
    pub maintenance_start_time: Option<std::time::Instant>,
    #[serde(skip)]
    pub last_maintenance: Option<std::time::Instant>,
    pub maintenance_duration: Option<Duration>,
    // Prewarming tracking fields
    #[serde(skip)]
    pub prewarming_start_time: Option<std::time::Instant>,
    pub prewarming_duration: Option<Duration>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            vm_id: String::new(),
            operation_type: String::new(),
            duration: Duration::ZERO,
            timestamp: chrono::Utc::now(),
            success: false,
            metadata: HashMap::new(),
            boot_time: None,
            last_allocated: None,
            last_used: None,
            last_prewarmed: None,
            usage_count: Some(0),
            maintenance_start_time: None,
            last_maintenance: None,
            maintenance_duration: None,
            prewarming_start_time: None,
            prewarming_duration: None,
        }
    }
}

#[allow(dead_code)]
impl PerformanceMetrics {
    #[allow(dead_code)]
    pub fn new(vm_id: String, operation_type: String, duration: Duration, success: bool) -> Self {
        Self {
            vm_id,
            operation_type,
            duration,
            timestamp: chrono::Utc::now(),
            success,
            metadata: HashMap::new(),
            boot_time: None,
            last_allocated: None,
            last_used: None,
            last_prewarmed: None,
            usage_count: Some(0),
            maintenance_start_time: None,
            last_maintenance: None,
            maintenance_duration: None,
            prewarming_start_time: None,
            prewarming_duration: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Boot performance metrics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BootMetrics {
    pub vm_id: String,
    pub vm_type: String,
    pub total_boot_time: Duration,
    pub performance_level: BootPerformanceLevel,
    pub phases: Vec<BootPhase>,
    pub memory_usage_mb: u32,
    pub cpu_usage_percent: f64,
    pub optimization_level: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BootPhase {
    pub name: String,
    pub duration: Duration,
    pub start_time: Instant,
    pub end_time: Instant,
}

impl BootMetrics {
    pub fn new(vm_id: String, vm_type: String, total_time: Duration) -> Self {
        let performance_level = BootPerformanceLevel::get_level(total_time);

        Self {
            vm_id,
            vm_type,
            total_boot_time: total_time,
            performance_level,
            phases: Vec::new(),
            memory_usage_mb: 0,
            cpu_usage_percent: 0.0,
            optimization_level: "unknown".to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    #[allow(dead_code)]
    pub fn add_phase(&mut self, name: String, duration: Duration) {
        let end_time = Instant::now();
        let start_time = end_time - duration;

        self.phases.push(BootPhase {
            name,
            duration,
            start_time,
            end_time,
        });
    }

    pub fn meets_sub2_target(&self) -> bool {
        self.performance_level.meets_target(self.total_boot_time)
    }

    #[allow(dead_code)]
    pub fn performance_summary(&self) -> String {
        format!(
            "VM {} ({}): {:.3}s - {:?} - {}",
            self.vm_id,
            self.vm_type,
            self.total_boot_time.as_secs_f64(),
            self.performance_level,
            if self.meets_sub2_target() {
                "✅ TARGET MET"
            } else {
                "❌ TARGET MISSED"
            }
        )
    }
}

/// Performance optimization strategies
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum OptimizationStrategy {
    /// Standard Firecracker boot with basic optimizations
    Standard,
    /// Fast boot with kernel parameter tuning
    Fast,
    /// Ultra-fast boot with aggressive optimizations
    UltraFast,
    /// Sub-2 second boot with all optimizations + prewarming
    Sub2Second,
    /// Instant boot from snapshot
    Instant,
}

impl OptimizationStrategy {
    pub fn get_kernel_args(&self, _vm_type: &str, _memory_mb: u32) -> String {
        match self {
            OptimizationStrategy::Standard => {
                "console=ttyS0 reboot=k panic=1 pci=off random.trust_cpu=on ip=172.26.0.10::172.26.0.1:255.255.255.0::eth0:off".to_string()
            }
            OptimizationStrategy::Fast => {
                "console=ttyS0 quiet reboot=k panic=1 pci=off random.trust_cpu=on systemd.unit=multi-user.target ip=172.26.0.10::172.26.0.1:255.255.255.0::eth0:off".to_string()
            }
            OptimizationStrategy::UltraFast => {
                "console=ttyS0 quiet loglevel=1 reboot=k panic=1 pci=off random.trust_cpu=on systemd.unit=multi-user.target systemd.mask=systemd-logind.service init_on_alloc=0 rcu_nocbs=0-1 isolcpus=1 ip=172.26.0.10::172.26.0.1:255.255.255.0::eth0:off".to_string()
            }
            OptimizationStrategy::Sub2Second => {
                "console=ttyS0 quiet loglevel=0 reboot=k panic=1 pci=off random.trust_cpu=on systemd.unit=multi-user.target systemd.mask=systemd-logind.service systemd.mask=udev.service systemd.mask=networkd.service init_on_alloc=0 rcu_nocbs=0-1 isolcpus=1 nopti nospectre_v2 nospec_store_bypass_disable l1tf=off mds=off tsx=off mitigations=off ip=172.26.0.10::172.26.0.1:255.255.255.0::eth0:off".to_string()
            }
            OptimizationStrategy::Instant => {
                // For snapshot restore - minimal kernel args
                "console=ttyS0 quiet loglevel=0 reboot=k panic=1 pci=off".to_string()
            }
        }
    }

    pub fn get_resource_config(&self, memory_mb: u32, vcpus: u32) -> (u32, u32) {
        match self {
            OptimizationStrategy::Standard => (memory_mb, vcpus),
            OptimizationStrategy::Fast => (memory_mb.max(512), vcpus.max(1)),
            OptimizationStrategy::UltraFast => {
                let optimized_memory = if memory_mb < 256 {
                    256
                } else {
                    memory_mb.min(1024)
                };
                let optimized_vcpus = vcpus.clamp(1, 2);
                (optimized_memory, optimized_vcpus)
            }
            OptimizationStrategy::Sub2Second => {
                // Minimal resources for fastest boot
                let optimized_memory = if memory_mb < 256 {
                    256
                } else {
                    memory_mb.min(512)
                };
                let optimized_vcpus = 1; // Single CPU for fastest boot
                (optimized_memory, optimized_vcpus)
            }
            OptimizationStrategy::Instant => {
                // Snapshot restore - use configured resources
                (memory_mb, vcpus)
            }
        }
    }
}

/// Performance benchmarking results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub vm_type: String,
    pub optimization_strategy: OptimizationStrategy,
    pub trials: Vec<BootMetrics>,
    pub average_boot_time: Duration,
    pub min_boot_time: Duration,
    pub max_boot_time: Duration,
    pub success_rate: f64,
    pub sub2_target_met_rate: f64,
}

impl BenchmarkResults {
    pub fn new(vm_type: String, strategy: OptimizationStrategy, trials: Vec<BootMetrics>) -> Self {
        let successful_trials: Vec<_> = trials
            .iter()
            .filter(|t| t.total_boot_time > Duration::ZERO)
            .collect();
        let success_rate = successful_trials.len() as f64 / trials.len() as f64;

        let sub2_target_met = successful_trials
            .iter()
            .filter(|t| t.meets_sub2_target())
            .count();
        let sub2_target_met_rate = sub2_target_met as f64 / successful_trials.len().max(1) as f64;

        let boot_times: Vec<_> = successful_trials
            .iter()
            .map(|t| t.total_boot_time)
            .collect();

        let (average_boot_time, min_boot_time, max_boot_time) = if boot_times.is_empty() {
            (Duration::ZERO, Duration::ZERO, Duration::ZERO)
        } else {
            let sum: Duration = boot_times.iter().sum();
            let average = sum / boot_times.len() as u32;
            let min = *boot_times.iter().min().unwrap();
            let max = *boot_times.iter().max().unwrap();
            (average, min, max)
        };

        Self {
            vm_type,
            optimization_strategy: strategy,
            trials,
            average_boot_time,
            min_boot_time,
            max_boot_time,
            success_rate,
            sub2_target_met_rate,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "Benchmark Results - {} ({:?}):\n  Trials: {}\n  Success Rate: {:.1}%\n  Sub-2s Target Rate: {:.1}%\n  Avg Boot Time: {:.3}s\n  Min Boot Time: {:.3}s\n  Max Boot Time: {:.3}s",
            self.vm_type,
            self.optimization_strategy,
            self.trials.len(),
            self.success_rate * 100.0,
            self.sub2_target_met_rate * 100.0,
            self.average_boot_time.as_secs_f64(),
            self.min_boot_time.as_secs_f64(),
            self.max_boot_time.as_secs_f64()
        )
    }
}

/// Performance monitoring and alerting
pub struct PerformanceMonitor {
    alert_thresholds: HashMap<String, Duration>,
    metrics_history: Vec<BootMetrics>,
    max_history_size: usize,
}

#[allow(dead_code)]
impl PerformanceMonitor {
    pub fn new() -> Self {
        let mut alert_thresholds = HashMap::new();
        alert_thresholds.insert("terraphim-minimal".to_string(), SUB2_TARGET_BOOT_TIME);
        alert_thresholds.insert(
            "terraphim-standard".to_string(),
            Duration::from_millis(3000),
        );
        alert_thresholds.insert(
            "terraphim-development".to_string(),
            Duration::from_millis(5000),
        );

        Self {
            alert_thresholds,
            metrics_history: Vec::new(),
            max_history_size: 1000,
        }
    }

    pub fn record_metrics(&mut self, metrics: BootMetrics) {
        // Check for performance alerts
        if let Some(threshold) = self.alert_thresholds.get(&metrics.vm_type) {
            if metrics.total_boot_time > *threshold {
                warn!(
                    "Performance Alert: {} boot time {:.3}s exceeds threshold {:.3}s",
                    metrics.vm_id,
                    metrics.total_boot_time.as_secs_f64(),
                    threshold.as_secs_f64()
                );
            }
        }

        // Add to history
        self.metrics_history.push(metrics);

        // Trim history if needed
        if self.metrics_history.len() > self.max_history_size {
            self.metrics_history.remove(0);
        }
    }

    #[allow(dead_code)]
    pub fn get_recent_metrics(&self, vm_type: &str, count: usize) -> Vec<&BootMetrics> {
        self.metrics_history
            .iter()
            .rev()
            .filter(|m| m.vm_type == vm_type)
            .take(count)
            .collect()
    }

    #[allow(dead_code)]
    pub fn get_performance_trend(&self, vm_type: &str, window_size: usize) -> Option<f64> {
        let recent_metrics = self.get_recent_metrics(vm_type, window_size);
        if recent_metrics.len() < 2 {
            return None;
        }

        let first_time = recent_metrics.last()?.total_boot_time.as_secs_f64();
        let last_time = recent_metrics.first()?.total_boot_time.as_secs_f64();

        Some((last_time - first_time) / first_time * 100.0)
    }

    pub fn generate_performance_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Performance Monitor Report ===\n\n");

        let vm_types: std::collections::HashSet<_> =
            self.metrics_history.iter().map(|m| &m.vm_type).collect();

        for vm_type in vm_types {
            let metrics: Vec<_> = self
                .metrics_history
                .iter()
                .filter(|m| m.vm_type == *vm_type)
                .collect();

            if metrics.is_empty() {
                continue;
            }

            let avg_time: Duration =
                metrics.iter().map(|m| m.total_boot_time).sum::<Duration>() / metrics.len() as u32;
            let min_time = metrics.iter().map(|m| m.total_boot_time).min().unwrap();
            let max_time = metrics.iter().map(|m| m.total_boot_time).max().unwrap();
            let sub2_rate = metrics.iter().filter(|m| m.meets_sub2_target()).count() as f64
                / metrics.len() as f64;

            report.push_str(&format!(
                "VM Type: {}\n  Total Boots: {}\n  Average: {:.3}s\n  Min: {:.3}s\n  Max: {:.3}s\n  Sub-2s Rate: {:.1}%\n\n",
                vm_type,
                metrics.len(),
                avg_time.as_secs_f64(),
                min_time.as_secs_f64(),
                max_time.as_secs_f64(),
                sub2_rate * 100.0
            ));
        }

        report
    }

    // Additional methods needed by other modules
    #[allow(dead_code)]
    pub fn record_prewarming_time(&mut self, vm_id: &str, duration: Duration) {
        debug!(
            "Recording prewarming time for VM {}: {:.3}s",
            vm_id,
            duration.as_secs_f64()
        );
        // This could be stored in a separate metrics history if needed
    }

    #[allow(dead_code)]
    pub fn record_allocation_time(&mut self, vm_id: &str, duration: Duration) {
        debug!(
            "Recording allocation time for VM {}: {:.3}s",
            vm_id,
            duration.as_secs_f64()
        );
        // This could be stored in a separate metrics history if needed
    }

    #[allow(dead_code)]
    pub fn record_release_time(&mut self, vm_id: &str, duration: Duration) {
        debug!(
            "Recording release time for VM {}: {:.3}s",
            vm_id,
            duration.as_secs_f64()
        );
        // This could be stored in a separate metrics history if needed
    }

    #[allow(dead_code)]
    pub fn record_maintenance_time(&mut self, vm_id: &str, duration: Duration) {
        debug!(
            "Recording maintenance time for VM {}: {:.3}s",
            vm_id,
            duration.as_secs_f64()
        );
        // This could be stored in a separate metrics history if needed
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_performance_levels() {
        assert_eq!(
            BootPerformanceLevel::get_level(Duration::from_millis(50)),
            BootPerformanceLevel::Instant
        );
        assert_eq!(
            BootPerformanceLevel::get_level(Duration::from_millis(300)),
            BootPerformanceLevel::Prewarmed
        );
        assert_eq!(
            BootPerformanceLevel::get_level(Duration::from_millis(1200)),
            BootPerformanceLevel::UltraFast
        );
        assert_eq!(
            BootPerformanceLevel::get_level(Duration::from_millis(1800)),
            BootPerformanceLevel::Sub2Second
        );
    }

    #[test]
    fn test_optimization_strategies() {
        let strategy = OptimizationStrategy::Sub2Second;
        let args = strategy.get_kernel_args("terraphim-minimal", 256);

        assert!(args.contains("console=ttyS0"));
        assert!(args.contains("quiet"));
        assert!(args.contains("mitigations=off"));
        assert!(args.contains("systemd.unit=multi-user.target"));
    }

    #[test]
    fn test_boot_metrics() {
        let mut metrics = BootMetrics::new(
            "test-vm".to_string(),
            "terraphim-minimal".to_string(),
            Duration::from_millis(1800),
        );
        metrics.add_phase("init".to_string(), Duration::from_millis(100));
        metrics.add_phase("boot".to_string(), Duration::from_millis(1700));

        assert_eq!(metrics.performance_level, BootPerformanceLevel::Sub2Second);
        assert!(metrics.meets_sub2_target());
        assert_eq!(metrics.phases.len(), 2);
    }

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();

        let metrics = BootMetrics::new(
            "test-vm".to_string(),
            "terraphim-minimal".to_string(),
            Duration::from_millis(1800),
        );
        monitor.record_metrics(metrics);

        assert_eq!(monitor.metrics_history.len(), 1);

        let recent = monitor.get_recent_metrics("terraphim-minimal", 5);
        assert_eq!(recent.len(), 1);
    }
}
