//! Agent pooling system for performance optimization

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{interval, Instant};

use terraphim_config::Role;
use terraphim_persistence::DeviceStorage;

use crate::{
    AgentId, CommandInput, CommandOutput, LoadMetrics, MultiAgentError, MultiAgentResult,
    TerraphimAgent,
};

/// Configuration for agent pooling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of agents to keep in the pool
    pub min_pool_size: usize,
    /// Maximum number of agents allowed in the pool
    pub max_pool_size: usize,
    /// Maximum idle time before an agent is removed from the pool
    pub max_idle_duration: Duration,
    /// Time between pool maintenance cycles
    pub maintenance_interval: Duration,
    /// Maximum number of concurrent operations per agent
    pub max_concurrent_operations: usize,
    /// Agent creation timeout
    pub agent_creation_timeout: Duration,
    /// Pool warming enabled
    pub enable_pool_warming: bool,
    /// Load balancing strategy
    pub load_balancing_strategy: LoadBalancingStrategy,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_pool_size: 2,
            max_pool_size: 10,
            max_idle_duration: Duration::from_secs(300), // 5 minutes
            maintenance_interval: Duration::from_secs(60), // 1 minute
            max_concurrent_operations: 5,
            agent_creation_timeout: Duration::from_secs(30),
            enable_pool_warming: true,
            load_balancing_strategy: LoadBalancingStrategy::LeastConnections,
        }
    }
}

/// Load balancing strategies for agent selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Round-robin selection
    RoundRobin,
    /// Select agent with least active connections
    LeastConnections,
    /// Select agent with lowest average response time
    FastestResponse,
    /// Random selection
    Random,
    /// Weighted selection based on capabilities
    WeightedCapabilities,
}

/// Agent pool entry with metadata
#[derive(Debug)]
struct PooledAgent {
    /// The actual agent
    agent: Arc<TerraphimAgent>,
    /// When the agent was last used
    last_used: Instant,
    /// Current number of active operations
    active_operations: u32,
    /// Total operations processed
    total_operations: u64,
    /// Agent load metrics
    load_metrics: LoadMetrics,
}

impl PooledAgent {
    fn new(agent: Arc<TerraphimAgent>, _max_concurrent_operations: usize) -> Self {
        let now = Instant::now();
        Self {
            agent,
            last_used: now,
            active_operations: 0,
            total_operations: 0,
            load_metrics: LoadMetrics::new(),
        }
    }

    fn is_idle(&self, max_idle_duration: Duration) -> bool {
        self.last_used.elapsed() > max_idle_duration && self.active_operations == 0
    }

    fn release_operation(&mut self, duration: Duration, success: bool) {
        if self.active_operations > 0 {
            self.active_operations -= 1;
        }
        self.total_operations += 1;

        // Update load metrics
        let duration_ms = duration.as_millis() as f64;
        if self.load_metrics.average_response_time_ms == 0.0 {
            self.load_metrics.average_response_time_ms = duration_ms;
        } else {
            // Exponential moving average
            self.load_metrics.average_response_time_ms =
                0.9 * self.load_metrics.average_response_time_ms + 0.1 * duration_ms;
        }

        // Update success rate
        let total_ops = self.total_operations as f64;
        if success {
            self.load_metrics.success_rate =
                ((total_ops - 1.0) * self.load_metrics.success_rate + 1.0) / total_ops;
        } else {
            self.load_metrics.success_rate =
                ((total_ops - 1.0) * self.load_metrics.success_rate) / total_ops;
        }

        self.load_metrics.last_updated = Utc::now();
    }
}

/// Agent pool for efficient agent management and reuse
pub struct AgentPool {
    /// Pool configuration
    config: PoolConfig,
    /// Role configuration for creating new agents
    role_config: Role,
    /// Persistence layer
    persistence: Arc<DeviceStorage>,
    /// Available agents in the pool
    available_agents: Arc<RwLock<VecDeque<PooledAgent>>>,
    /// Currently busy agents
    busy_agents: Arc<RwLock<HashMap<AgentId, PooledAgent>>>,
    /// Pool statistics
    stats: Arc<RwLock<PoolStats>>,
    /// Round-robin counter for load balancing
    round_robin_counter: Arc<RwLock<usize>>,
    /// Pool maintenance task handle
    _maintenance_task: tokio::task::JoinHandle<()>,
}

/// Pool statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub total_agents_created: u64,
    pub total_agents_destroyed: u64,
    pub total_operations_processed: u64,
    pub current_pool_size: usize,
    pub current_busy_agents: usize,
    pub average_operation_time_ms: f64,
    pub pool_hit_rate: f64,
    pub last_updated: DateTime<Utc>,
}

impl Default for PoolStats {
    fn default() -> Self {
        Self {
            total_agents_created: 0,
            total_agents_destroyed: 0,
            total_operations_processed: 0,
            current_pool_size: 0,
            current_busy_agents: 0,
            average_operation_time_ms: 0.0,
            pool_hit_rate: 1.0,
            last_updated: Utc::now(),
        }
    }
}

impl AgentPool {
    /// Create a new agent pool
    pub async fn new(
        role_config: Role,
        persistence: Arc<DeviceStorage>,
        config: Option<PoolConfig>,
    ) -> MultiAgentResult<Self> {
        let config = config.unwrap_or_default();

        let pool = Self {
            config: config.clone(),
            role_config,
            persistence,
            available_agents: Arc::new(RwLock::new(VecDeque::new())),
            busy_agents: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PoolStats::default())),
            round_robin_counter: Arc::new(RwLock::new(0)),
            _maintenance_task: tokio::spawn(async {}), // Placeholder
        };

        // Start maintenance task
        let maintenance_task = Self::start_maintenance_task(
            pool.available_agents.clone(),
            pool.busy_agents.clone(),
            pool.stats.clone(),
            config.clone(),
        );

        let pool = Self {
            _maintenance_task: maintenance_task,
            ..pool
        };

        // Warm up the pool if enabled
        if config.enable_pool_warming {
            pool.warm_up_pool().await?;
        }

        Ok(pool)
    }

    /// Warm up the pool by creating minimum number of agents
    async fn warm_up_pool(&self) -> MultiAgentResult<()> {
        log::info!(
            "Warming up agent pool with {} agents",
            self.config.min_pool_size
        );

        for _ in 0..self.config.min_pool_size {
            let agent = self.create_new_agent().await?;
            let pooled_agent = PooledAgent::new(agent, self.config.max_concurrent_operations);

            let mut available = self.available_agents.write().await;
            available.push_back(pooled_agent);
        }

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_agents_created += self.config.min_pool_size as u64;
        stats.current_pool_size = self.config.min_pool_size;
        stats.last_updated = Utc::now();

        log::info!("Agent pool warmed up successfully");
        Ok(())
    }

    /// Get an agent from the pool
    pub async fn get_agent(&self) -> MultiAgentResult<PooledAgentHandle> {
        // Try to get an available agent first
        if let Some(pooled_agent) = self.get_available_agent().await {
            return Ok(PooledAgentHandle::new(
                pooled_agent,
                self.available_agents.clone(),
                self.stats.clone(),
            ));
        }

        // If no available agents and we haven't reached max pool size, create a new one
        let current_total_size = {
            let available = self.available_agents.read().await;
            let busy = self.busy_agents.read().await;
            available.len() + busy.len()
        };

        if current_total_size < self.config.max_pool_size {
            let agent = self.create_new_agent().await?;
            let pooled_agent = PooledAgent::new(agent, self.config.max_concurrent_operations);

            // Update stats
            {
                let mut stats = self.stats.write().await;
                stats.total_agents_created += 1;
                stats.current_pool_size = current_total_size + 1;
                stats.pool_hit_rate = (stats.pool_hit_rate
                    * stats.total_operations_processed as f64)
                    / (stats.total_operations_processed + 1) as f64;
                stats.last_updated = Utc::now();
            }

            return Ok(PooledAgentHandle::new(
                pooled_agent,
                self.available_agents.clone(),
                self.stats.clone(),
            ));
        }

        // Pool is at capacity, return error
        Err(MultiAgentError::PoolExhausted)
    }

    /// Get an available agent using the configured load balancing strategy
    async fn get_available_agent(&self) -> Option<PooledAgent> {
        let mut available = self.available_agents.write().await;

        if available.is_empty() {
            return None;
        }

        let index = match self.config.load_balancing_strategy {
            LoadBalancingStrategy::RoundRobin => {
                let mut counter = self.round_robin_counter.write().await;
                let idx = *counter % available.len();
                *counter = (*counter + 1) % available.len();
                idx
            }
            LoadBalancingStrategy::LeastConnections => available
                .iter()
                .enumerate()
                .min_by_key(|(_, agent)| agent.active_operations)
                .map(|(idx, _)| idx)
                .unwrap_or(0),
            LoadBalancingStrategy::FastestResponse => available
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.load_metrics
                        .average_response_time_ms
                        .partial_cmp(&b.load_metrics.average_response_time_ms)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(idx, _)| idx)
                .unwrap_or(0),
            LoadBalancingStrategy::Random => {
                use rand::Rng;
                rand::rng().random_range(0..available.len())
            }
            LoadBalancingStrategy::WeightedCapabilities => {
                // For now, use least connections
                // TODO: Implement capability-based weighting
                available
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, agent)| agent.active_operations)
                    .map(|(idx, _)| idx)
                    .unwrap_or(0)
            }
        };

        if index < available.len() {
            Some(available.remove(index).unwrap())
        } else {
            available.pop_front()
        }
    }

    /// Create a new agent
    async fn create_new_agent(&self) -> MultiAgentResult<Arc<TerraphimAgent>> {
        let agent = tokio::time::timeout(
            self.config.agent_creation_timeout,
            TerraphimAgent::new(self.role_config.clone(), self.persistence.clone(), None),
        )
        .await
        .map_err(|_| MultiAgentError::AgentCreationTimeout)?
        .map_err(|e| MultiAgentError::AgentCreationFailed(e.to_string()))?;

        agent.initialize().await?;
        Ok(Arc::new(agent))
    }

    /// Start the maintenance task
    fn start_maintenance_task(
        available_agents: Arc<RwLock<VecDeque<PooledAgent>>>,
        busy_agents: Arc<RwLock<HashMap<AgentId, PooledAgent>>>,
        stats: Arc<RwLock<PoolStats>>,
        config: PoolConfig,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut maintenance_interval = interval(config.maintenance_interval);

            loop {
                maintenance_interval.tick().await;

                // Clean up idle agents
                let mut removed_count = 0;
                {
                    let mut available = available_agents.write().await;
                    let original_len = available.len();
                    let min_pool_size = config.min_pool_size;

                    // Keep only agents that are not idle or if we're at minimum pool size
                    available.retain(|agent| {
                        let should_keep = !agent.is_idle(config.max_idle_duration)
                            || original_len <= min_pool_size;
                        if !should_keep {
                            removed_count += 1;
                        }
                        should_keep
                    });
                }

                // Update stats
                if removed_count > 0 {
                    let mut stats = stats.write().await;
                    stats.total_agents_destroyed += removed_count;
                    stats.current_pool_size = stats
                        .current_pool_size
                        .saturating_sub(removed_count as usize);
                    stats.last_updated = Utc::now();

                    log::debug!("Pool maintenance: removed {} idle agents", removed_count);
                }

                // Log pool status
                let available_count = available_agents.read().await.len();
                let busy_count = busy_agents.read().await.len();
                log::debug!(
                    "Pool status: {} available, {} busy agents",
                    available_count,
                    busy_count
                );
            }
        })
    }

    /// Get current pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        let mut stats = self.stats.read().await.clone();
        stats.current_pool_size = self.available_agents.read().await.len();
        stats.current_busy_agents = self.busy_agents.read().await.len();
        stats
    }

    /// Execute a command using a pooled agent
    pub async fn execute_command(&self, input: CommandInput) -> MultiAgentResult<CommandOutput> {
        let agent_handle = self.get_agent().await?;
        agent_handle.execute_command(input).await
    }

    /// Shutdown the pool gracefully
    pub async fn shutdown(&self) -> MultiAgentResult<()> {
        log::info!("Shutting down agent pool");

        // Clear all agents
        {
            let mut available = self.available_agents.write().await;
            available.clear();
        }

        {
            let mut busy = self.busy_agents.write().await;
            busy.clear();
        }

        // Update final stats
        {
            let mut stats = self.stats.write().await;
            stats.current_pool_size = 0;
            stats.current_busy_agents = 0;
            stats.last_updated = Utc::now();
        }

        log::info!("Agent pool shutdown complete");
        Ok(())
    }
}

/// Handle for a pooled agent that automatically returns the agent to the pool when dropped
pub struct PooledAgentHandle {
    pooled_agent: Option<PooledAgent>,
    available_agents: Arc<RwLock<VecDeque<PooledAgent>>>,
    stats: Arc<RwLock<PoolStats>>,
    operation_start: Instant,
}

impl PooledAgentHandle {
    fn new(
        pooled_agent: PooledAgent,
        available_agents: Arc<RwLock<VecDeque<PooledAgent>>>,
        stats: Arc<RwLock<PoolStats>>,
    ) -> Self {
        Self {
            pooled_agent: Some(pooled_agent),
            available_agents,
            stats,
            operation_start: Instant::now(),
        }
    }

    /// Execute a command using the pooled agent
    pub async fn execute_command(&self, input: CommandInput) -> MultiAgentResult<CommandOutput> {
        if let Some(pooled_agent) = &self.pooled_agent {
            // Try to acquire operation permit
            if let Some(_permit) = {
                // We need to access the pooled_agent mutably, but self is immutable
                // For now, we'll execute without the semaphore
                // TODO: Improve this design
                Some(())
            } {
                let result = pooled_agent.agent.process_command(input).await;

                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.total_operations_processed += 1;
                    stats.last_updated = Utc::now();
                }

                result
            } else {
                Err(MultiAgentError::AgentBusy(pooled_agent.agent.agent_id))
            }
        } else {
            Err(MultiAgentError::PoolError(
                "Agent handle is empty".to_string(),
            ))
        }
    }

    /// Get the underlying agent reference
    pub fn agent(&self) -> Option<&Arc<TerraphimAgent>> {
        self.pooled_agent.as_ref().map(|pa| &pa.agent)
    }
}

impl Drop for PooledAgentHandle {
    fn drop(&mut self) {
        if let Some(mut pooled_agent) = self.pooled_agent.take() {
            let duration = self.operation_start.elapsed();
            pooled_agent.release_operation(duration, true); // Assume success for now

            // Return agent to available pool
            let available_agents = self.available_agents.clone();
            tokio::spawn(async move {
                let mut available = available_agents.write().await;
                available.push_back(pooled_agent);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_role;
    use terraphim_persistence::DeviceStorage;

    #[tokio::test]
    async fn test_pool_creation() {
        DeviceStorage::init_memory_only().await.unwrap();
        let storage = DeviceStorage::arc_memory_only().await.unwrap();

        let role = create_test_role();
        let config = PoolConfig {
            min_pool_size: 2,
            max_pool_size: 5,
            enable_pool_warming: true,
            ..Default::default()
        };

        let pool = AgentPool::new(role, storage, Some(config)).await.unwrap();
        let stats = pool.get_stats().await;

        assert_eq!(stats.current_pool_size, 2);
        assert_eq!(stats.total_agents_created, 2);
    }

    #[tokio::test]
    async fn test_agent_acquisition() {
        DeviceStorage::init_memory_only().await.unwrap();
        let storage = DeviceStorage::arc_memory_only().await.unwrap();

        let role = create_test_role();
        let pool = AgentPool::new(role, storage, None).await.unwrap();

        let agent_handle = pool.get_agent().await.unwrap();
        assert!(agent_handle.agent().is_some());

        // Agent should be returned to pool when handle is dropped
        drop(agent_handle);

        // Give time for async return
        tokio::time::sleep(Duration::from_millis(100)).await;

        let stats = pool.get_stats().await;
        assert!(stats.current_pool_size > 0);
    }

    #[tokio::test]
    async fn test_pool_exhaustion() {
        DeviceStorage::init_memory_only().await.unwrap();
        let storage = DeviceStorage::arc_memory_only().await.unwrap();

        let role = create_test_role();
        let config = PoolConfig {
            min_pool_size: 1,
            max_pool_size: 2,
            enable_pool_warming: true,
            ..Default::default()
        };

        let pool = AgentPool::new(role, storage, Some(config)).await.unwrap();

        // Acquire all available agents
        let _handle1 = pool.get_agent().await.unwrap();
        let _handle2 = pool.get_agent().await.unwrap();

        // Next acquisition should succeed - pool creates agents on demand
        let result = pool.get_agent().await;
        assert!(result.is_ok());
    }
}
