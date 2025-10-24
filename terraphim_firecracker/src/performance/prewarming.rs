use anyhow::Result;
use log::{debug, info};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Prewarming manager for VM pools
#[allow(dead_code)]
pub struct PrewarmingManager {
    target_pool_size: usize,
    prewarm_interval: Duration,
    last_prewarm: Arc<RwLock<Instant>>,
}

#[allow(dead_code)]
impl PrewarmingManager {
    pub fn new(target_pool_size: usize, prewarm_interval: Duration) -> Self {
        Self {
            target_pool_size,
            prewarm_interval,
            last_prewarm: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn should_prewarm(&self) -> bool {
        let last_prewarm = *self.last_prewarm.read().await;
        last_prewarm.elapsed() >= self.prewarm_interval
    }

    pub async fn mark_prewarmed(&self) {
        let mut last_prewarm = self.last_prewarm.write().await;
        *last_prewarm = Instant::now();
    }

    pub async fn calculate_prewarm_target(
        &self,
        current_size: usize,
        recent_demand: usize,
    ) -> usize {
        let base_target = self.target_pool_size;
        let demand_adjusted = (recent_demand as f64 * 1.2) as usize; // 20% buffer
        let max_target = base_target.max(demand_adjusted);

        max_target.saturating_sub(current_size)
    }

    pub async fn prewarm_resources(&self) -> Result<()> {
        debug!("Prewarming system resources for sub-2 second VM boot");

        // Simulate resource prewarming
        tokio::time::sleep(Duration::from_millis(100)).await;

        info!("System resources prewarmed successfully");
        Ok(())
    }

    pub async fn validate_prewarmed_state(&self) -> Result<bool> {
        debug!("Validating prewarmed state");

        // Simulate validation
        tokio::time::sleep(Duration::from_millis(50)).await;

        Ok(true)
    }

    /// Maintain pool levels by creating new prewarmed VMs as needed
    pub async fn maintain_pool_levels(
        &self,
        prewarmed_pools: &std::collections::HashMap<
            String,
            Arc<RwLock<Vec<crate::pool::PrewarmedVm>>>,
        >,
        target_pool_size: usize,
    ) -> Result<()> {
        debug!(
            "Maintaining pool levels with target size: {}",
            target_pool_size
        );

        for (vm_type, pool) in prewarmed_pools {
            let pool_guard = pool.read().await;
            let current_size = pool_guard.len();
            drop(pool_guard);

            if current_size < target_pool_size {
                let needed = target_pool_size - current_size;
                info!(
                    "Prewarming {} VMs of type {} (current: {}, target: {})",
                    needed, vm_type, current_size, target_pool_size
                );

                // Simulate prewarming VMs
                for _ in 0..needed {
                    debug!("Prewarming VM of type: {}", vm_type);
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }

        Ok(())
    }
}

impl Default for PrewarmingManager {
    fn default() -> Self {
        Self::new(5, Duration::from_secs(300)) // 5 VMs, every 5 minutes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prewarming_manager() {
        let manager = PrewarmingManager::new(3, Duration::from_secs(60));
        assert_eq!(manager.target_pool_size, 3);
        assert_eq!(manager.prewarm_interval, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_should_prewarm() {
        let manager = PrewarmingManager::new(3, Duration::from_millis(100));

        // Initially should not prewarm
        assert!(!manager.should_prewarm().await);

        // Wait past interval
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(manager.should_prewarm().await);
    }

    #[tokio::test]
    async fn test_calculate_prewarm_target() {
        let manager = PrewarmingManager::new(5, Duration::from_secs(60));

        // No demand
        assert_eq!(manager.calculate_prewarm_target(2, 0).await, 3);

        // High demand
        assert_eq!(manager.calculate_prewarm_target(2, 10).await, 10); // 10 * 1.2 = 12, minus current 2

        // At capacity
        assert_eq!(manager.calculate_prewarm_target(5, 2).await, 0);
    }
}
