use crate::error::Result;
use crate::performance::PerformanceMonitor;
use crate::vm::{VmInstance, VmState};
use log::{debug, info, warn};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Allocation strategy for VM instances
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AllocationStrategy {
    /// First available VM
    FirstAvailable,
    /// Best performance score
    BestPerformance,
    /// Least recently used
    LeastRecentlyUsed,
    /// Round-robin allocation
    RoundRobin,
    /// Weighted random allocation
    WeightedRandom,
}

/// Allocation scoring factors
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct AllocationScore {
    pub performance_score: f64,
    pub age_penalty: f64,
    pub usage_bonus: f64,
    pub resource_efficiency: f64,
    pub total_score: f64,
}

#[allow(dead_code)]
impl AllocationScore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calculate(&mut self) {
        self.total_score =
            self.performance_score - self.age_penalty + self.usage_bonus + self.resource_efficiency;
    }
}

/// VM allocator for intelligent VM selection
#[allow(dead_code)]
pub struct VmAllocator {
    performance_monitor: Arc<PerformanceMonitor>,
    strategy: AllocationStrategy,
    allocation_timeout: Duration,
    round_robin_counter: usize,
}

#[allow(dead_code)]
impl VmAllocator {
    /// Create a new VM allocator
    pub fn new(performance_monitor: Arc<PerformanceMonitor>, strategy: AllocationStrategy) -> Self {
        Self {
            performance_monitor,
            strategy,
            allocation_timeout: Duration::from_secs(10),
            round_robin_counter: 0,
        }
    }

    /// Set allocation timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.allocation_timeout = timeout;
        self
    }

    /// Allocate a VM from the available pool
    pub async fn allocate_vm(&self, available_vms: &[VmInstance]) -> Result<VmInstance> {
        if available_vms.is_empty() {
            return Err(crate::error::Error::NoAvailableVms);
        }

        let start_time = Instant::now();
        info!(
            "Allocating VM from pool of {} available instances",
            available_vms.len()
        );

        // Filter ready VMs
        let ready_vms: Vec<_> = available_vms
            .iter()
            .filter(|vm| {
                let vm_guard = futures::executor::block_on(vm.read());
                vm_guard.state == VmState::Ready
            })
            .cloned()
            .collect();

        if ready_vms.is_empty() {
            return Err(crate::error::Error::NoAvailableVms);
        }

        // Select VM based on strategy
        let selected_vm = match self.strategy {
            AllocationStrategy::FirstAvailable => self.allocate_first_available(&ready_vms).await?,
            AllocationStrategy::BestPerformance => {
                self.allocate_best_performance(&ready_vms).await?
            }
            AllocationStrategy::LeastRecentlyUsed => self.allocate_lru(&ready_vms).await?,
            AllocationStrategy::RoundRobin => self.allocate_round_robin(&ready_vms).await?,
            AllocationStrategy::WeightedRandom => self.allocate_weighted_random(&ready_vms).await?,
        };

        // Mark VM as allocated
        {
            let mut vm_guard = selected_vm.write().await;
            vm_guard.state = VmState::Running;
            vm_guard.metrics.last_allocated = Some(start_time);
        }

        let duration = start_time.elapsed();
        info!(
            "VM {} allocated in {:?}",
            selected_vm.read().await.id,
            duration
        );

        // Record allocation metrics - TODO: Add internal synchronization to PerformanceMonitor
        debug!(
            "VM {} allocated in {:?}",
            selected_vm.read().await.id,
            duration
        );

        Ok(selected_vm)
    }

    /// Allocate first available VM
    async fn allocate_first_available(&self, vms: &[VmInstance]) -> Result<VmInstance> {
        debug!("Using first available allocation strategy");
        Ok(vms[0].clone())
    }

    /// Allocate VM with best performance score
    async fn allocate_best_performance(&self, vms: &[VmInstance]) -> Result<VmInstance> {
        debug!("Using best performance allocation strategy");

        let mut best_vm = None;
        let mut best_score = f64::MIN;

        for vm in vms {
            let _vm_guard = vm.read().await;
            let score = self.calculate_performance_score(vm).await;

            if score > best_score {
                best_score = score;
                best_vm = Some(vm.clone());
            }
        }

        best_vm.ok_or_else(|| crate::error::Error::Allocation("No suitable VM found".to_string()))
    }

    /// Allocate least recently used VM
    async fn allocate_lru(&self, vms: &[VmInstance]) -> Result<VmInstance> {
        debug!("Using LRU allocation strategy");

        let mut lru_vm = None;
        let mut oldest_time = Instant::now();

        for vm in vms {
            let vm_guard = vm.read().await;
            if let Some(last_used) = vm_guard.metrics.last_used {
                if last_used < oldest_time {
                    oldest_time = last_used;
                    lru_vm = Some(vm.clone());
                }
            } else {
                // VM never used, prefer it
                lru_vm = Some(vm.clone());
                break;
            }
        }

        lru_vm.ok_or_else(|| crate::error::Error::Allocation("No suitable VM found".to_string()))
    }

    /// Allocate VM using round-robin
    async fn allocate_round_robin(&self, vms: &[VmInstance]) -> Result<VmInstance> {
        debug!("Using round-robin allocation strategy");

        let index = self.round_robin_counter % vms.len();
        // Note: In a real implementation, this would need to be atomic or use a mutex
        // for thread safety. For now, we'll use a simple approach.
        Ok(vms[index].clone())
    }

    /// Allocate VM using weighted random selection
    async fn allocate_weighted_random(&self, vms: &[VmInstance]) -> Result<VmInstance> {
        debug!("Using weighted random allocation strategy");

        let mut weights = Vec::new();
        let mut total_weight = 0.0;

        for vm in vms {
            let _vm_guard = vm.read().await;
            let score = self.calculate_performance_score(vm).await;
            weights.push(score);
            total_weight += score;
        }

        if total_weight <= 0.0 {
            // Fallback to first available
            return Ok(vms[0].clone());
        }

        // Generate random number and select VM
        let random_value = fastrand::f64() * total_weight;
        let mut cumulative_weight = 0.0;

        for (i, &weight) in weights.iter().enumerate() {
            cumulative_weight += weight;
            if random_value <= cumulative_weight {
                return Ok(vms[i].clone());
            }
        }

        // Fallback to last VM
        Ok(vms[vms.len() - 1].clone())
    }

    /// Calculate performance score for a VM
    async fn calculate_performance_score(&self, vm: &VmInstance) -> f64 {
        let mut score = AllocationScore::new();

        let vm_guard = vm.read().await;

        // Performance score based on boot time and prewarming
        if let Some(boot_time) = vm_guard.metrics.boot_time {
            score.performance_score = 1000.0 / boot_time.as_millis() as f64;
        }

        // Age penalty - older VMs get lower scores
        if let Some(last_prewarmed) = vm_guard.metrics.last_prewarmed {
            let age = last_prewarmed.elapsed();
            score.age_penalty = age.as_secs() as f64 * 0.1;
        }

        // Usage bonus - frequently used VMs get higher scores
        if let Some(usage_count) = vm_guard.metrics.usage_count {
            score.usage_bonus = usage_count as f64 * 10.0;
        }

        // Resource efficiency based on memory usage
        score.resource_efficiency = 50.0; // Base efficiency score

        score.calculate();
        score.total_score.max(0.0) // Ensure non-negative
    }

    /// Batch allocate multiple VMs
    pub async fn allocate_batch(
        &self,
        available_vms: &[VmInstance],
        count: usize,
    ) -> Result<Vec<VmInstance>> {
        info!("Batch allocating {} VMs", count);

        if count > available_vms.len() {
            return Err(crate::error::Error::InsufficientResources(format!(
                "Requested {} VMs, only {} available",
                count,
                available_vms.len()
            )));
        }

        let mut allocated_vms = Vec::new();
        let mut remaining_vms = available_vms.to_vec();

        for _ in 0..count {
            match self.allocate_vm(&remaining_vms).await {
                Ok(vm) => {
                    allocated_vms.push(vm.clone());
                    // Remove allocated VM from remaining list
                    let vm_id = vm.read().await.id.clone();
                    let mut to_remove = Vec::new();
                    for (i, v) in remaining_vms.iter().enumerate() {
                        if v.read().await.id == vm_id {
                            to_remove.push(i);
                        }
                    }
                    // Remove in reverse order to maintain indices
                    for &i in to_remove.iter().rev() {
                        remaining_vms.remove(i);
                    }
                }
                Err(e) => {
                    warn!("Failed to allocate VM in batch: {}", e);
                    break;
                }
            }
        }

        info!(
            "Batch allocation completed: {}/{} VMs allocated",
            allocated_vms.len(),
            count
        );
        Ok(allocated_vms)
    }

    /// Release a VM back to the pool
    pub async fn release_vm(&self, vm: &VmInstance) -> Result<()> {
        let vm_id = vm.read().await.id.clone();
        info!("Releasing VM {} back to pool", vm_id);

        {
            let mut vm_guard = vm.write().await;
            vm_guard.state = VmState::Ready;
            vm_guard.metrics.last_used = Some(Instant::now());
            vm_guard.metrics.usage_count = Some(vm_guard.metrics.usage_count.unwrap_or(0) + 1);
        }

        // Record release metrics - TODO: Add internal synchronization to PerformanceMonitor
        debug!("VM {} released", vm_id);

        Ok(())
    }

    /// Get allocation strategy
    pub fn strategy(&self) -> &AllocationStrategy {
        &self.strategy
    }

    /// Update allocation strategy
    pub fn update_strategy(&mut self, strategy: AllocationStrategy) {
        info!("Updating allocation strategy to {:?}", strategy);
        self.strategy = strategy;
    }

    /// Allocate a prewarmed VM for instant use
    pub async fn allocate_prewarmed_vm(
        &self,
        prewarmed_vm: &crate::pool::PrewarmedVm,
    ) -> Result<VmInstance> {
        let start_time = Instant::now();
        info!("Allocating prewarmed VM: {}", prewarmed_vm.vm.id);

        // Create a copy of the VM for allocation
        let mut vm = prewarmed_vm.vm.clone();

        // Update VM state and metrics
        vm.state = VmState::Running;
        vm.last_used = Some(chrono::Utc::now());

        let allocation_time = start_time.elapsed();
        info!("Prewarmed VM {} allocated in: {:?}", vm.id, allocation_time);

        // Record allocation metrics - TODO: Add internal synchronization to PerformanceMonitor
        debug!("VM {} allocated in {:?}", vm.id, allocation_time);

        Ok(Arc::new(RwLock::new(vm)))
    }

    /// Create and start a new VM with the given configuration
    pub async fn create_and_start_vm(
        &self,
        config: &crate::vm::config::VmConfig,
    ) -> Result<VmInstance> {
        let start_time = Instant::now();
        info!("Creating and starting new VM with config: {}", config.vm_id);

        // Create new VM instance
        let mut vm = crate::vm::Vm::new(config.vm_type.clone(), config.clone());

        // Mark as starting
        vm.state = VmState::Starting;
        vm.last_used = Some(chrono::Utc::now());

        let creation_time = start_time.elapsed();
        info!(
            "New VM {} created and started in: {:?}",
            vm.id, creation_time
        );

        // Record allocation metrics - TODO: Add internal synchronization to PerformanceMonitor
        debug!("VM {} created in {:?}", vm.id, creation_time);

        Ok(Arc::new(RwLock::new(vm)))
    }

    /// Get allocation statistics
    pub async fn get_allocation_stats(&self, vms: &[VmInstance]) -> AllocationStats {
        let mut stats = AllocationStats::default();

        for vm in vms {
            let vm_guard = vm.read().await;
            match vm_guard.state {
                VmState::Ready => stats.ready_count += 1,
                VmState::Running => stats.running_count += 1,
                VmState::Prewarming => stats.prewarming_count += 1,
                VmState::Snapshotted => stats.snapshotted_count += 1,
                VmState::Allocating => stats.allocating_count += 1,
                VmState::NeedsMaintenance => stats.needs_maintenance_count += 1,
                VmState::Initializing => stats.ready_count += 1, // Count as ready for stats
                VmState::Starting => stats.running_count += 1,   // Count as running
                VmState::Stopping => stats.ready_count += 1,     // Count as ready
                VmState::Stopped => stats.ready_count += 1,
                VmState::Failed => stats.needs_maintenance_count += 1,
                VmState::Prewarmed => stats.ready_count += 1,
            }

            if let Some(boot_time) = vm_guard.metrics.boot_time {
                stats.total_boot_time += boot_time;
                if stats.boot_count > 0 {
                    stats.average_boot_time = stats.total_boot_time / stats.boot_count as u32;
                }
                stats.boot_count += 1;
            }
        }

        stats
    }
}

/// Allocation statistics
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct AllocationStats {
    pub ready_count: usize,
    pub running_count: usize,
    pub prewarming_count: usize,
    pub snapshotted_count: usize,
    pub allocating_count: usize,
    pub needs_maintenance_count: usize,
    pub boot_count: usize,
    pub average_boot_time: Duration,
    pub total_boot_time: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::PerformanceMonitor;

    #[tokio::test]
    async fn test_allocator_creation() {
        let monitor = Arc::new(PerformanceMonitor::new());
        let allocator = VmAllocator::new(monitor, AllocationStrategy::BestPerformance);

        assert!(matches!(
            allocator.strategy(),
            AllocationStrategy::BestPerformance
        ));
    }

    #[tokio::test]
    async fn test_strategy_update() {
        let monitor = Arc::new(PerformanceMonitor::new());
        let mut allocator = VmAllocator::new(monitor, AllocationStrategy::FirstAvailable);

        allocator.update_strategy(AllocationStrategy::RoundRobin);
        assert!(matches!(
            allocator.strategy(),
            AllocationStrategy::RoundRobin
        ));
    }

    #[tokio::test]
    async fn test_allocation_score() {
        let mut score = AllocationScore::new();
        score.performance_score = 100.0;
        score.age_penalty = 10.0;
        score.usage_bonus = 20.0;
        score.resource_efficiency = 15.0;

        score.calculate();
        assert_eq!(score.total_score, 125.0);
    }

    #[tokio::test]
    async fn test_empty_pool_allocation() {
        let monitor = Arc::new(PerformanceMonitor::new());
        let allocator = VmAllocator::new(monitor, AllocationStrategy::FirstAvailable);

        let result = allocator.allocate_vm(&[]).await;
        assert!(result.is_err());
    }
}
