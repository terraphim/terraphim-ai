//! Pool manager for coordinating multiple agent pools

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use terraphim_config::Role;
use terraphim_persistence::DeviceStorage;

use crate::{
    AgentPool, CommandInput, CommandOutput, LoadBalancingStrategy, MultiAgentError,
    MultiAgentResult, PoolConfig, PoolStats, TerraphimAgent,
};

/// Configuration for the pool manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolManagerConfig {
    /// Default pool configuration for new pools
    pub default_pool_config: PoolConfig,
    /// Maximum number of pools to maintain
    pub max_pools: usize,
    /// Whether to create pools on demand
    pub create_pools_on_demand: bool,
    /// Pool cleanup interval
    pub cleanup_interval_seconds: u64,
    /// Maximum idle time for pools before cleanup
    pub pool_max_idle_duration_seconds: u64,
}

impl Default for PoolManagerConfig {
    fn default() -> Self {
        Self {
            default_pool_config: PoolConfig::default(),
            max_pools: 20,
            create_pools_on_demand: true,
            cleanup_interval_seconds: 300, // 5 minutes
            pool_max_idle_duration_seconds: 1800, // 30 minutes
        }
    }
}

/// Pool information for management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub role_name: String,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub stats: PoolStats,
    pub is_active: bool,
}

/// Centralized manager for multiple agent pools
pub struct PoolManager {
    /// Configuration
    config: PoolManagerConfig,
    /// Persistence layer
    persistence: Arc<DeviceStorage>,
    /// Active pools by role name
    pools: Arc<RwLock<HashMap<String, Arc<AgentPool>>>>,
    /// Pool metadata
    pool_info: Arc<RwLock<HashMap<String, PoolInfo>>>,
    /// Global statistics
    global_stats: Arc<RwLock<GlobalStats>>,
    /// Cleanup task handle
    _cleanup_task: tokio::task::JoinHandle<()>,
}

/// Global statistics across all pools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStats {
    pub total_pools: usize,
    pub total_agents: usize,
    pub total_operations: u64,
    pub average_operation_time_ms: f64,
    pub total_pool_hits: u64,
    pub total_pool_misses: u64,
    pub last_updated: DateTime<Utc>,
}

impl Default for GlobalStats {
    fn default() -> Self {
        Self {
            total_pools: 0,
            total_agents: 0,
            total_operations: 0,
            average_operation_time_ms: 0.0,
            total_pool_hits: 0,
            total_pool_misses: 0,
            last_updated: Utc::now(),
        }
    }
}

impl PoolManager {
    /// Create a new pool manager
    pub async fn new(
        persistence: Arc<DeviceStorage>,
        config: Option<PoolManagerConfig>,
    ) -> MultiAgentResult<Self> {
        let config = config.unwrap_or_default();
        
        let pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_info = Arc::new(RwLock::new(HashMap::new()));
        let global_stats = Arc::new(RwLock::new(GlobalStats::default()));

        // Start cleanup task
        let cleanup_task = Self::start_cleanup_task(
            pools.clone(),
            pool_info.clone(),
            global_stats.clone(),
            config.clone(),
        );

        Ok(Self {
            config,
            persistence,
            pools,
            pool_info,
            global_stats,
            _cleanup_task: cleanup_task,
        })
    }

    /// Get or create a pool for a specific role
    pub async fn get_pool(&self, role: &Role) -> MultiAgentResult<Arc<AgentPool>> {
        let role_name = role.name.to_string();
        
        // Check if pool already exists
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(&role_name) {
                // Update last used time
                {
                    let mut pool_info = self.pool_info.write().await;
                    if let Some(info) = pool_info.get_mut(&role_name) {
                        info.last_used = Utc::now();
                    }
                }
                
                // Record pool hit
                {
                    let mut stats = self.global_stats.write().await;
                    stats.total_pool_hits += 1;
                    stats.last_updated = Utc::now();
                }
                
                return Ok(pool.clone());
            }
        }

        // Check if we can create a new pool
        if !self.config.create_pools_on_demand {
            return Err(MultiAgentError::PoolError(
                "Pool creation on demand is disabled".to_string(),
            ));
        }

        let pools_count = self.pools.read().await.len();
        if pools_count >= self.config.max_pools {
            return Err(MultiAgentError::PoolError(format!(
                "Maximum number of pools ({}) reached",
                self.config.max_pools
            )));
        }

        // Create new pool
        log::info!("Creating new agent pool for role: {}", role_name);
        
        let pool = Arc::new(
            AgentPool::new(
                role.clone(),
                self.persistence.clone(),
                Some(self.config.default_pool_config.clone()),
            )
            .await?,
        );

        // Register the new pool
        {
            let mut pools = self.pools.write().await;
            pools.insert(role_name.clone(), pool.clone());
        }

        {
            let mut pool_info = self.pool_info.write().await;
            let now = Utc::now();
            pool_info.insert(
                role_name.clone(),
                PoolInfo {
                    role_name: role_name.clone(),
                    created_at: now,
                    last_used: now,
                    stats: pool.get_stats().await,
                    is_active: true,
                },
            );
        }

        // Update global stats
        {
            let mut stats = self.global_stats.write().await;
            stats.total_pools += 1;
            stats.total_pool_misses += 1;
            stats.last_updated = Utc::now();
        }

        log::info!("Successfully created agent pool for role: {}", role_name);
        Ok(pool)
    }

    /// Execute a command using the appropriate pool
    pub async fn execute_command(
        &self,
        role: &Role,
        input: CommandInput,
    ) -> MultiAgentResult<CommandOutput> {
        let pool = self.get_pool(role).await?;
        let start_time = std::time::Instant::now();
        
        let result = pool.execute_command(input).await;
        
        // Update global statistics
        let duration = start_time.elapsed();
        {
            let mut stats = self.global_stats.write().await;
            stats.total_operations += 1;
            
            let duration_ms = duration.as_millis() as f64;
            if stats.average_operation_time_ms == 0.0 {
                stats.average_operation_time_ms = duration_ms;
            } else {
                // Exponential moving average
                stats.average_operation_time_ms = 
                    0.95 * stats.average_operation_time_ms + 0.05 * duration_ms;
            }
            
            stats.last_updated = Utc::now();
        }

        result
    }

    /// Get an agent directly from a pool
    pub async fn get_agent(&self, role: &Role) -> MultiAgentResult<Arc<TerraphimAgent>> {
        let pool = self.get_pool(role).await?;
        let handle = pool.get_agent().await?;
        
        if let Some(agent) = handle.agent() {
            Ok(agent.clone())
        } else {
            Err(MultiAgentError::PoolError("Agent handle is empty".to_string()))
        }
    }

    /// List all pools
    pub async fn list_pools(&self) -> Vec<PoolInfo> {
        let pool_info = self.pool_info.read().await;
        pool_info.values().cloned().collect()
    }

    /// Get pool statistics for a specific role
    pub async fn get_pool_stats(&self, role_name: &str) -> Option<PoolStats> {
        let pools = self.pools.read().await;
        if let Some(pool) = pools.get(role_name) {
            Some(pool.get_stats().await)
        } else {
            None
        }
    }

    /// Get global statistics
    pub async fn get_global_stats(&self) -> GlobalStats {
        let mut stats = self.global_stats.read().await.clone();
        
        // Update current totals
        let pools = self.pools.read().await;
        stats.total_pools = pools.len();
        
        let mut total_agents = 0;
        for pool in pools.values() {
            let pool_stats = pool.get_stats().await;
            total_agents += pool_stats.current_pool_size + pool_stats.current_busy_agents;
        }
        stats.total_agents = total_agents;
        
        stats
    }

    /// Shutdown a specific pool
    pub async fn shutdown_pool(&self, role_name: &str) -> MultiAgentResult<()> {
        let pool = {
            let mut pools = self.pools.write().await;
            pools.remove(role_name)
        };

        if let Some(pool) = pool {
            pool.shutdown().await?;
            
            // Update pool info
            {
                let mut pool_info = self.pool_info.write().await;
                if let Some(info) = pool_info.get_mut(role_name) {
                    info.is_active = false;
                }
            }

            // Update global stats
            {
                let mut stats = self.global_stats.write().await;
                stats.total_pools = stats.total_pools.saturating_sub(1);
                stats.last_updated = Utc::now();
            }

            log::info!("Shut down pool for role: {}", role_name);
        }

        Ok(())
    }

    /// Shutdown all pools
    pub async fn shutdown_all(&self) -> MultiAgentResult<()> {
        log::info!("Shutting down all agent pools");
        
        let pool_names: Vec<String> = {
            let pools = self.pools.read().await;
            pools.keys().cloned().collect()
        };

        for role_name in pool_names {
            if let Err(e) = self.shutdown_pool(&role_name).await {
                log::error!("Failed to shutdown pool {}: {}", role_name, e);
            }
        }

        log::info!("All agent pools shut down");
        Ok(())
    }

    /// Configure load balancing strategy for all pools
    pub async fn set_load_balancing_strategy(&self, strategy: LoadBalancingStrategy) {
        // Note: This would require extending the AgentPool to support runtime strategy changes
        // For now, this is a placeholder for future implementation
        log::info!("Load balancing strategy update requested: {:?}", strategy);
    }

    /// Start the cleanup task
    fn start_cleanup_task(
        pools: Arc<RwLock<HashMap<String, Arc<AgentPool>>>>,
        pool_info: Arc<RwLock<HashMap<String, PoolInfo>>>,
        global_stats: Arc<RwLock<GlobalStats>>,
        config: PoolManagerConfig,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(config.cleanup_interval_seconds)
            );
            
            loop {
                interval.tick().await;
                
                let max_idle_duration = std::time::Duration::from_secs(
                    config.pool_max_idle_duration_seconds
                );
                
                // Find idle pools to clean up
                let pools_to_cleanup = {
                    let pool_info_guard = pool_info.read().await;
                    let now = Utc::now();
                    
                    pool_info_guard
                        .iter()
                        .filter_map(|(name, info)| {
                            let idle_duration = now - info.last_used;
                            if idle_duration.to_std().unwrap_or_default() > max_idle_duration 
                                && info.is_active {
                                Some(name.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                };

                // Clean up idle pools
                for pool_name in pools_to_cleanup {
                    log::info!("Cleaning up idle pool: {}", pool_name);
                    
                    // Remove from active pools
                    {
                        let mut pools_guard = pools.write().await;
                        if let Some(pool) = pools_guard.remove(&pool_name) {
                            // Shutdown the pool
                            if let Err(e) = pool.shutdown().await {
                                log::error!("Failed to shutdown pool {}: {}", pool_name, e);
                            }
                        }
                    }

                    // Mark as inactive
                    {
                        let mut pool_info_guard = pool_info.write().await;
                        if let Some(info) = pool_info_guard.get_mut(&pool_name) {
                            info.is_active = false;
                        }
                    }

                    // Update global stats
                    {
                        let mut stats = global_stats.write().await;
                        stats.total_pools = stats.total_pools.saturating_sub(1);
                        stats.last_updated = Utc::now();
                    }
                }

                // Log pool manager status
                let active_pools = pools.read().await.len();
                if active_pools > 0 {
                    log::debug!("Pool manager status: {} active pools", active_pools);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_role;
    use terraphim_persistence::DeviceStorage;

    #[tokio::test]
    async fn test_pool_manager_creation() {
        DeviceStorage::init_memory_only().await.unwrap();
        let storage = {
            let storage_ref = DeviceStorage::instance().await.unwrap();
            use std::ptr;
            let storage_copy = unsafe { ptr::read(storage_ref) };
            Arc::new(storage_copy)
        };

        let manager = PoolManager::new(storage, None).await.unwrap();
        let stats = manager.get_global_stats().await;
        
        assert_eq!(stats.total_pools, 0);
        assert_eq!(stats.total_agents, 0);
    }

    #[tokio::test]
    async fn test_pool_creation_on_demand() {
        DeviceStorage::init_memory_only().await.unwrap();
        let storage = {
            let storage_ref = DeviceStorage::instance().await.unwrap();
            use std::ptr;
            let storage_copy = unsafe { ptr::read(storage_ref) };
            Arc::new(storage_copy)
        };

        let manager = PoolManager::new(storage, None).await.unwrap();
        let role = create_test_role();
        
        // First call should create the pool
        let pool1 = manager.get_pool(&role).await.unwrap();
        assert!(pool1.get_stats().await.current_pool_size > 0);
        
        // Second call should return the same pool
        let pool2 = manager.get_pool(&role).await.unwrap();
        assert!(Arc::ptr_eq(&pool1, &pool2));
        
        let stats = manager.get_global_stats().await;
        assert_eq!(stats.total_pools, 1);
        assert_eq!(stats.total_pool_hits, 1);
        assert_eq!(stats.total_pool_misses, 1);
    }

    #[tokio::test]
    async fn test_pool_shutdown() {
        DeviceStorage::init_memory_only().await.unwrap();
        let storage = {
            let storage_ref = DeviceStorage::instance().await.unwrap();
            use std::ptr;
            let storage_copy = unsafe { ptr::read(storage_ref) };
            Arc::new(storage_copy)
        };

        let manager = PoolManager::new(storage, None).await.unwrap();
        let role = create_test_role();
        
        let _pool = manager.get_pool(&role).await.unwrap();
        assert_eq!(manager.get_global_stats().await.total_pools, 1);
        
        manager.shutdown_pool(&role.name).await.unwrap();
        assert_eq!(manager.get_global_stats().await.total_pools, 0);
    }
}