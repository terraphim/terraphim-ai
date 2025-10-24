use crate::error::Result;
use crate::vm::{Vm, VmInstance, VmState, VmStorage};
use async_trait::async_trait;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Trait for VM storage backends
#[async_trait]
#[allow(dead_code)]
pub trait VmStorageBackend: Send + Sync {
    /// Store a VM instance
    async fn store_vm(&self, vm: VmInstance) -> Result<()>;

    /// Retrieve a VM instance by ID
    async fn get_vm(&self, vm_id: &str) -> Result<Option<VmInstance>>;

    /// Remove a VM instance
    async fn remove_vm(&self, vm_id: &str) -> Result<bool>;

    /// List all VM IDs
    async fn list_vms(&self) -> Result<Vec<String>>;

    /// Get VMs by state
    async fn get_vms_by_state(&self, state: VmState) -> Result<Vec<VmInstance>>;

    /// Get storage statistics
    async fn get_stats(&self) -> Result<VmStorageStats>;

    /// Cleanup old or unused VMs
    async fn cleanup(&self, max_age: Duration) -> Result<usize>;
}

/// Storage statistics
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct VmStorageStats {
    pub total_vms: usize,
    pub initializing_vms: usize,
    pub starting_vms: usize,
    pub ready_vms: usize,
    pub running_vms: usize,
    pub stopping_vms: usize,
    pub stopped_vms: usize,
    pub failed_vms: usize,
    pub prewarmed_vms: usize,
    pub prewarming_vms: usize,
    pub snapshotted_vms: usize,
    pub allocating_vms: usize,
    pub needs_maintenance_vms: usize,
    pub oldest_vm_age: Option<Duration>,
    pub newest_vm_age: Option<Duration>,
}

/// In-memory VM storage implementation
#[allow(dead_code)]
pub struct InMemoryVmStorage {
    vms: Arc<RwLock<HashMap<String, VmInstance>>>,
    created_at: Instant,
}

#[allow(dead_code)]
impl InMemoryVmStorage {
    /// Create a new in-memory VM storage
    pub fn new() -> Self {
        Self {
            vms: Arc::new(RwLock::new(HashMap::new())),
            created_at: Instant::now(),
        }
    }

    /// Get the number of stored VMs
    pub async fn len(&self) -> usize {
        self.vms.read().await.len()
    }

    /// Check if storage is empty
    pub async fn is_empty(&self) -> bool {
        self.vms.read().await.is_empty()
    }

    /// Clear all VMs from storage
    pub async fn clear(&self) -> Result<()> {
        info!("Clearing all VMs from in-memory storage");
        self.vms.write().await.clear();
        Ok(())
    }

    /// Get VMs older than specified duration
    pub async fn get_vms_older_than(&self, max_age: Duration) -> Result<Vec<String>> {
        let vms_guard = self.vms.read().await;
        let mut old_vm_ids = Vec::new();
        let cutoff_time = chrono::Utc::now()
            - chrono::Duration::from_std(max_age).unwrap_or(chrono::Duration::zero());

        for (vm_id, vm) in vms_guard.iter() {
            let vm_guard = vm.read().await;
            if vm_guard.created_at < cutoff_time {
                old_vm_ids.push(vm_id.clone());
            }
        }

        Ok(old_vm_ids)
    }

    /// Get VMs by usage count
    pub async fn get_vms_by_usage(&self, min_usage: u32) -> Result<Vec<String>> {
        let vms_guard = self.vms.read().await;
        let mut vm_ids = Vec::new();

        for (vm_id, vm) in vms_guard.iter() {
            let vm_guard = vm.read().await;
            if vm_guard.metrics.usage_count.unwrap_or(0) >= min_usage {
                vm_ids.push(vm_id.clone());
            }
        }

        Ok(vm_ids)
    }

    /// Update VM state
    pub async fn update_vm_state(&self, vm_id: &str, new_state: VmState) -> Result<bool> {
        let vms_guard = self.vms.read().await;

        if let Some(vm) = vms_guard.get(vm_id) {
            let mut vm_guard = vm.write().await;
            vm_guard.state = new_state.clone();
            debug!("Updated VM {} state to {:?}", vm_id, new_state);
            Ok(true)
        } else {
            warn!("VM {} not found for state update", vm_id);
            Ok(false)
        }
    }

    /// Get VM performance metrics
    pub async fn get_vm_metrics(
        &self,
        vm_id: &str,
    ) -> Result<Option<crate::performance::PerformanceMetrics>> {
        let vms_guard = self.vms.read().await;

        if let Some(vm) = vms_guard.get(vm_id) {
            let vm_guard = vm.read().await;
            Ok(Some(vm_guard.metrics.clone()))
        } else {
            Ok(None)
        }
    }

    /// Find best performing VM
    pub async fn find_best_performing_vm(&self) -> Result<Option<VmInstance>> {
        let vms_guard = self.vms.read().await;
        let mut best_vm = None;
        let mut best_score = f64::MIN;

        for vm in vms_guard.values() {
            let vm_guard = vm.read().await;

            // Only consider ready VMs
            if vm_guard.state != VmState::Ready {
                continue;
            }

            // Calculate performance score
            let mut score = 0.0;

            // Boot time score (lower is better)
            if let Some(boot_time) = vm_guard.metrics.boot_time {
                score += 1000.0 / boot_time.as_millis() as f64;
            }

            // Usage bonus
            score += vm_guard.metrics.usage_count.unwrap_or(0) as f64 * 10.0;

            // Age penalty (older VMs get lower scores)
            let age = chrono::Utc::now() - vm_guard.created_at;
            score -= age.num_seconds() as f64 * 0.1;

            if score > best_score {
                best_score = score;
                best_vm = Some(vm.clone());
            }
        }

        Ok(best_vm)
    }

    /// Batch store VMs
    pub async fn batch_store(&self, vms: Vec<VmInstance>) -> Result<usize> {
        let mut vms_guard = self.vms.write().await;
        let mut stored_count = 0;

        for vm in vms {
            let vm_id = vm.read().await.id.clone();
            vms_guard.insert(vm_id, vm);
            stored_count += 1;
        }

        info!("Batch stored {} VMs", stored_count);
        Ok(stored_count)
    }

    /// Get storage age
    pub fn storage_age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

impl Default for InMemoryVmStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VmStorageBackend for InMemoryVmStorage {
    async fn store_vm(&self, vm: VmInstance) -> Result<()> {
        let vm_id = vm.read().await.id.clone();
        debug!("Storing VM {} in memory", vm_id);

        let mut vms_guard = self.vms.write().await;
        vms_guard.insert(vm_id, vm);

        Ok(())
    }

    async fn get_vm(&self, vm_id: &str) -> Result<Option<VmInstance>> {
        let vms_guard = self.vms.read().await;
        Ok(vms_guard.get(vm_id).cloned())
    }

    async fn remove_vm(&self, vm_id: &str) -> Result<bool> {
        debug!("Removing VM {} from memory", vm_id);

        let mut vms_guard = self.vms.write().await;
        Ok(vms_guard.remove(vm_id).is_some())
    }

    async fn list_vms(&self) -> Result<Vec<String>> {
        let vms_guard = self.vms.read().await;
        Ok(vms_guard.keys().cloned().collect())
    }

    async fn get_vms_by_state(&self, state: VmState) -> Result<Vec<VmInstance>> {
        let vms_guard = self.vms.read().await;
        let mut matching_vms = Vec::new();

        for vm in vms_guard.values() {
            let vm_guard = vm.read().await;
            if vm_guard.state == state {
                matching_vms.push(vm.clone());
            }
        }

        Ok(matching_vms)
    }

    async fn get_stats(&self) -> Result<VmStorageStats> {
        let vms_guard = self.vms.read().await;
        let mut stats = VmStorageStats::default();
        let mut oldest_time = None;
        let mut newest_time = None;

        stats.total_vms = vms_guard.len();

        for vm in vms_guard.values() {
            let vm_guard = vm.read().await;

            match vm_guard.state {
                VmState::Initializing => stats.initializing_vms += 1,
                VmState::Starting => stats.starting_vms += 1,
                VmState::Ready => stats.ready_vms += 1,
                VmState::Running => stats.running_vms += 1,
                VmState::Stopping => stats.stopping_vms += 1,
                VmState::Stopped => stats.stopped_vms += 1,
                VmState::Failed => stats.failed_vms += 1,
                VmState::Prewarmed => stats.prewarmed_vms += 1,
                VmState::Prewarming => stats.prewarming_vms += 1,
                VmState::Snapshotted => stats.snapshotted_vms += 1,
                VmState::Allocating => stats.allocating_vms += 1,
                VmState::NeedsMaintenance => stats.needs_maintenance_vms += 1,
            }

            let age = chrono::Utc::now() - vm_guard.created_at;
            let age_duration = std::time::Duration::from_millis(age.num_milliseconds() as u64);

            if oldest_time.is_none() || age_duration > oldest_time.unwrap() {
                oldest_time = Some(age_duration);
            }

            if newest_time.is_none() || age_duration < newest_time.unwrap() {
                newest_time = Some(age_duration);
            }
        }

        stats.oldest_vm_age = oldest_time;
        stats.newest_vm_age = newest_time;

        Ok(stats)
    }

    async fn cleanup(&self, max_age: Duration) -> Result<usize> {
        info!("Cleaning up VMs older than {:?}", max_age);

        let old_vm_ids = self.get_vms_older_than(max_age).await?;
        let mut removed_count = 0;

        for vm_id in old_vm_ids {
            if self.remove_vm(&vm_id).await? {
                removed_count += 1;
                debug!("Removed old VM {}", vm_id);
            }
        }

        info!("Cleaned up {} old VMs", removed_count);
        Ok(removed_count)
    }
}

// Implement the VmStorage trait from vm module
#[async_trait::async_trait]
impl VmStorage for InMemoryVmStorage {
    async fn store_vm(&self, vm: &Vm) -> anyhow::Result<()> {
        let vm_instance = Arc::new(RwLock::new(vm.clone()));
        let mut vms = self.vms.write().await;
        vms.insert(vm.id.clone(), vm_instance);
        Ok(())
    }

    async fn get_vm(&self, vm_id: &str) -> anyhow::Result<Option<Vm>> {
        let vms = self.vms.read().await;
        if let Some(vm_instance) = vms.get(vm_id) {
            let vm_guard = vm_instance.read().await;
            Ok(Some(vm_guard.clone()))
        } else {
            Ok(None)
        }
    }

    async fn update_vm(&self, vm: &Vm) -> anyhow::Result<()> {
        VmStorage::store_vm(self, vm).await
    }

    async fn delete_vm(&self, vm_id: &str) -> anyhow::Result<()> {
        let mut vms = self.vms.write().await;
        vms.remove(vm_id);
        Ok(())
    }

    async fn list_vms(&self) -> anyhow::Result<Vec<Vm>> {
        let vms = self.vms.read().await;
        let mut result = Vec::new();
        for vm_instance in vms.values() {
            let vm_guard = vm_instance.read().await;
            result.push(vm_guard.clone());
        }
        Ok(result)
    }

    async fn list_prewarmed_vms(&self, vm_type: &str) -> anyhow::Result<Vec<Vm>> {
        let vms = self.vms.read().await;
        let mut result = Vec::new();
        for vm_instance in vms.values() {
            let vm_guard = vm_instance.read().await;
            if vm_guard.vm_type == vm_type && vm_guard.state == VmState::Prewarmed {
                result.push(vm_guard.clone());
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::PerformanceMetrics;

    #[tokio::test]
    async fn test_in_memory_storage_creation() {
        let storage = InMemoryVmStorage::new();
        assert_eq!(storage.len().await, 0);
        assert!(storage.is_empty().await);
    }

    #[tokio::test]
    async fn test_store_and_get_vm() {
        let storage = InMemoryVmStorage::new();

        let vm = Arc::new(RwLock::new(Vm {
            id: "test-vm-1".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Ready,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        // Store VM
        VmStorageBackend::store_vm(&storage, vm.clone())
            .await
            .unwrap();
        assert_eq!(storage.len().await, 1);

        // Get VM
        let retrieved_vm = VmStorageBackend::get_vm(&storage, "test-vm-1")
            .await
            .unwrap();
        assert!(retrieved_vm.is_some());
        assert_eq!(retrieved_vm.unwrap().read().await.id, "test-vm-1");
    }

    #[tokio::test]
    async fn test_remove_vm() {
        let storage = InMemoryVmStorage::new();

        let vm = Arc::new(RwLock::new(Vm {
            id: "test-vm-2".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Ready,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        VmStorageBackend::store_vm(&storage, vm.clone())
            .await
            .unwrap();
        assert_eq!(storage.len().await, 1);

        // Remove VM
        let removed = VmStorageBackend::remove_vm(&storage, "test-vm-2")
            .await
            .unwrap();
        assert!(removed);
        assert_eq!(storage.len().await, 0);

        // Try to remove non-existent VM
        let removed = VmStorageBackend::remove_vm(&storage, "non-existent")
            .await
            .unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_list_vms() {
        let storage = InMemoryVmStorage::new();

        let vm1 = Arc::new(RwLock::new(Vm {
            id: "test-vm-1".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Ready,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        let vm2 = Arc::new(RwLock::new(Vm {
            id: "test-vm-2".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Running,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        VmStorageBackend::store_vm(&storage, vm1).await.unwrap();
        VmStorageBackend::store_vm(&storage, vm2).await.unwrap();

        let vm_ids = VmStorageBackend::list_vms(&storage).await.unwrap();
        assert_eq!(vm_ids.len(), 2);
        assert!(vm_ids.contains(&"test-vm-1".to_string()));
        assert!(vm_ids.contains(&"test-vm-2".to_string()));
    }

    #[tokio::test]
    async fn test_get_vms_by_state() {
        let storage = InMemoryVmStorage::new();

        let vm1 = Arc::new(RwLock::new(Vm {
            id: "test-vm-1".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Ready,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        let vm2 = Arc::new(RwLock::new(Vm {
            id: "test-vm-2".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Ready,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        let vm3 = Arc::new(RwLock::new(Vm {
            id: "test-vm-3".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Running,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        VmStorageBackend::store_vm(&storage, vm1).await.unwrap();
        VmStorageBackend::store_vm(&storage, vm2).await.unwrap();
        VmStorageBackend::store_vm(&storage, vm3).await.unwrap();

        let ready_vms = storage.get_vms_by_state(VmState::Ready).await.unwrap();
        assert_eq!(ready_vms.len(), 2);

        let running_vms = storage.get_vms_by_state(VmState::Running).await.unwrap();
        assert_eq!(running_vms.len(), 1);
    }

    #[tokio::test]
    async fn test_update_vm_state() {
        let storage = InMemoryVmStorage::new();

        let vm = Arc::new(RwLock::new(Vm {
            id: "test-vm-1".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Ready,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        VmStorageBackend::store_vm(&storage, vm.clone())
            .await
            .unwrap();

        // Update state
        let updated = storage
            .update_vm_state("test-vm-1", VmState::Running)
            .await
            .unwrap();
        assert!(updated);

        // Verify state change
        let retrieved_vm = VmStorageBackend::get_vm(&storage, "test-vm-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved_vm.read().await.state, VmState::Running);

        // Try to update non-existent VM
        let updated = storage
            .update_vm_state("non-existent", VmState::Running)
            .await
            .unwrap();
        assert!(!updated);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let storage = InMemoryVmStorage::new();

        let vm1 = Arc::new(RwLock::new(Vm {
            id: "test-vm-1".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Ready,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        let vm2 = Arc::new(RwLock::new(Vm {
            id: "test-vm-2".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Running,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        VmStorageBackend::store_vm(&storage, vm1).await.unwrap();
        VmStorageBackend::store_vm(&storage, vm2).await.unwrap();

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.total_vms, 2);
        assert_eq!(stats.ready_vms, 1);
        assert_eq!(stats.running_vms, 1);
        assert!(stats.oldest_vm_age.is_some());
        assert!(stats.newest_vm_age.is_some());
    }

    #[tokio::test]
    async fn test_clear() {
        let storage = InMemoryVmStorage::new();

        let vm = Arc::new(RwLock::new(Vm {
            id: "test-vm-1".to_string(),
            vm_type: "test-type".to_string(),
            state: VmState::Ready,
            config: Default::default(),
            ip_address: None,
            created_at: chrono::Utc::now(),
            boot_time: None,
            last_used: None,
            metrics: PerformanceMetrics::default(),
        }));

        VmStorageBackend::store_vm(&storage, vm).await.unwrap();
        assert_eq!(storage.len().await, 1);

        storage.clear().await.unwrap();
        assert_eq!(storage.len().await, 0);
        assert!(storage.is_empty().await);
    }
}
