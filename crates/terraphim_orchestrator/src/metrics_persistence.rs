//! Metrics persistence for agent cost and performance tracking.
//!
//! This module provides persistence capabilities for agent execution metrics,
//! enabling long-term storage and analysis of agent performance data.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cost_tracker::AgentMetrics;

/// Persistable collection of agent metrics for storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAgentMetrics {
    /// Version for schema migration support.
    pub version: u32,
    /// Timestamp when metrics were last updated.
    pub updated_at: String,
    /// Per-agent metrics collection.
    pub agents: HashMap<String, AgentMetrics>,
    /// Fleet-wide aggregated metrics.
    pub fleet: AgentMetrics,
}

impl PersistedAgentMetrics {
    /// Create a new persisted metrics collection.
    pub fn new(agents: HashMap<String, AgentMetrics>, fleet: AgentMetrics) -> Self {
        Self {
            version: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
            agents,
            fleet,
        }
    }
}

/// Configuration for metrics persistence.
#[derive(Debug, Clone)]
pub struct MetricsPersistenceConfig {
    /// Storage key prefix for metrics.
    pub key_prefix: String,
    /// Whether to compress metrics data.
    pub compress: bool,
}

impl Default for MetricsPersistenceConfig {
    fn default() -> Self {
        Self {
            key_prefix: "adf/metrics".to_string(),
            compress: true,
        }
    }
}

/// Metrics persistence trait for storing and loading agent metrics.
#[async_trait]
pub trait MetricsPersistence: Send + Sync {
    /// Save agent metrics to storage.
    async fn save_metrics(
        &self,
        agent_name: &str,
        metrics: &AgentMetrics,
    ) -> Result<(), MetricsPersistenceError>;

    /// Load agent metrics from storage.
    async fn load_metrics(
        &self,
        agent_name: &str,
    ) -> Result<Option<AgentMetrics>, MetricsPersistenceError>;

    /// Save fleet-wide metrics.
    async fn save_fleet_metrics(
        &self,
        metrics: &AgentMetrics,
    ) -> Result<(), MetricsPersistenceError>;

    /// Load fleet-wide metrics.
    async fn load_fleet_metrics(&self) -> Result<Option<AgentMetrics>, MetricsPersistenceError>;

    /// List all stored agent metrics.
    async fn list_agents(&self) -> Result<Vec<String>, MetricsPersistenceError>;

    /// Delete metrics for an agent.
    async fn delete_metrics(&self, agent_name: &str) -> Result<(), MetricsPersistenceError>;
}

/// Errors that can occur during metrics persistence operations.
#[derive(Debug, thiserror::Error)]
pub enum MetricsPersistenceError {
    #[error("storage error: {0}")]
    Storage(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("agent not found: {0}")]
    NotFound(String),
}

/// In-memory metrics persistence for testing and development.
pub struct InMemoryMetricsPersistence {
    data: std::sync::RwLock<HashMap<String, AgentMetrics>>,
    fleet: std::sync::RwLock<Option<AgentMetrics>>,
}

impl InMemoryMetricsPersistence {
    /// Create a new in-memory metrics store.
    pub fn new() -> Self {
        Self {
            data: std::sync::RwLock::new(HashMap::new()),
            fleet: std::sync::RwLock::new(None),
        }
    }
}

impl Default for InMemoryMetricsPersistence {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MetricsPersistence for InMemoryMetricsPersistence {
    async fn save_metrics(
        &self,
        agent_name: &str,
        metrics: &AgentMetrics,
    ) -> Result<(), MetricsPersistenceError> {
        let mut data = self
            .data
            .write()
            .map_err(|e| MetricsPersistenceError::Storage(format!("Lock poisoned: {}", e)))?;
        data.insert(agent_name.to_string(), metrics.clone());
        Ok(())
    }

    async fn load_metrics(
        &self,
        agent_name: &str,
    ) -> Result<Option<AgentMetrics>, MetricsPersistenceError> {
        let data = self
            .data
            .read()
            .map_err(|e| MetricsPersistenceError::Storage(format!("Lock poisoned: {}", e)))?;
        Ok(data.get(agent_name).cloned())
    }

    async fn save_fleet_metrics(
        &self,
        metrics: &AgentMetrics,
    ) -> Result<(), MetricsPersistenceError> {
        let mut fleet = self
            .fleet
            .write()
            .map_err(|e| MetricsPersistenceError::Storage(format!("Lock poisoned: {}", e)))?;
        *fleet = Some(metrics.clone());
        Ok(())
    }

    async fn load_fleet_metrics(&self) -> Result<Option<AgentMetrics>, MetricsPersistenceError> {
        let fleet = self
            .fleet
            .read()
            .map_err(|e| MetricsPersistenceError::Storage(format!("Lock poisoned: {}", e)))?;
        Ok(fleet.clone())
    }

    async fn list_agents(&self) -> Result<Vec<String>, MetricsPersistenceError> {
        let data = self
            .data
            .read()
            .map_err(|e| MetricsPersistenceError::Storage(format!("Lock poisoned: {}", e)))?;
        Ok(data.keys().cloned().collect())
    }

    async fn delete_metrics(&self, agent_name: &str) -> Result<(), MetricsPersistenceError> {
        let mut data = self
            .data
            .write()
            .map_err(|e| MetricsPersistenceError::Storage(format!("Lock poisoned: {}", e)))?;
        data.remove(agent_name);
        Ok(())
    }
}

/// File-based metrics persistence using terraphim_persistence.
pub struct FileMetricsPersistence {
    config: MetricsPersistenceConfig,
}

impl FileMetricsPersistence {
    /// Create a new file-based metrics persistence.
    pub fn new(config: MetricsPersistenceConfig) -> Self {
        Self { config }
    }

    /// Build storage key for an agent.
    fn agent_key(&self, agent_name: &str) -> String {
        format!("{}/{}", self.config.key_prefix, agent_name)
    }

    /// Build storage key for fleet metrics.
    fn fleet_key(&self) -> String {
        format!("{}/fleet", self.config.key_prefix)
    }
}

#[async_trait]
impl MetricsPersistence for FileMetricsPersistence {
    async fn save_metrics(
        &self,
        agent_name: &str,
        _metrics: &AgentMetrics,
    ) -> Result<(), MetricsPersistenceError> {
        // Note: This would integrate with terraphim_persistence::Persistable
        // For now, this is a placeholder for the actual implementation
        tracing::debug!(
            "Saving metrics for agent {} (key: {})",
            agent_name,
            self.agent_key(agent_name)
        );
        Ok(())
    }

    async fn load_metrics(
        &self,
        agent_name: &str,
    ) -> Result<Option<AgentMetrics>, MetricsPersistenceError> {
        tracing::debug!(
            "Loading metrics for agent {} (key: {})",
            agent_name,
            self.agent_key(agent_name)
        );
        // Placeholder - would load from terraphim_persistence
        Ok(None)
    }

    async fn save_fleet_metrics(
        &self,
        _metrics: &AgentMetrics,
    ) -> Result<(), MetricsPersistenceError> {
        tracing::debug!("Saving fleet metrics (key: {})", self.fleet_key());
        Ok(())
    }

    async fn load_fleet_metrics(&self) -> Result<Option<AgentMetrics>, MetricsPersistenceError> {
        tracing::debug!("Loading fleet metrics (key: {})", self.fleet_key());
        Ok(None)
    }

    async fn list_agents(&self) -> Result<Vec<String>, MetricsPersistenceError> {
        // Placeholder - would list from terraphim_persistence
        Ok(vec![])
    }

    async fn delete_metrics(&self, agent_name: &str) -> Result<(), MetricsPersistenceError> {
        tracing::debug!(
            "Deleting metrics for agent {} (key: {})",
            agent_name,
            self.agent_key(agent_name)
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_save_and_load() {
        let persistence = InMemoryMetricsPersistence::new();

        let mut metrics = AgentMetrics::new("test-agent".to_string());
        metrics.total_executions = 10;
        metrics.total_tokens = 5000;

        persistence
            .save_metrics("test-agent", &metrics)
            .await
            .unwrap();

        let loaded = persistence.load_metrics("test-agent").await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.agent_name, "test-agent");
        assert_eq!(loaded.total_executions, 10);
        assert_eq!(loaded.total_tokens, 5000);
    }

    #[tokio::test]
    async fn test_in_memory_load_not_found() {
        let persistence = InMemoryMetricsPersistence::new();

        let loaded = persistence.load_metrics("non-existent").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_fleet_metrics() {
        let persistence = InMemoryMetricsPersistence::new();

        let mut fleet = AgentMetrics::new("fleet".to_string());
        fleet.total_executions = 100;
        fleet.total_cost_usd = 5.0;

        persistence.save_fleet_metrics(&fleet).await.unwrap();

        let loaded = persistence.load_fleet_metrics().await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.agent_name, "fleet");
        assert_eq!(loaded.total_executions, 100);
        assert!((loaded.total_cost_usd - 5.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_in_memory_list_agents() {
        let persistence = InMemoryMetricsPersistence::new();

        let metrics1 = AgentMetrics::new("agent-1".to_string());
        let metrics2 = AgentMetrics::new("agent-2".to_string());

        persistence
            .save_metrics("agent-1", &metrics1)
            .await
            .unwrap();
        persistence
            .save_metrics("agent-2", &metrics2)
            .await
            .unwrap();

        let agents = persistence.list_agents().await.unwrap();
        assert_eq!(agents.len(), 2);
        assert!(agents.contains(&"agent-1".to_string()));
        assert!(agents.contains(&"agent-2".to_string()));
    }

    #[tokio::test]
    async fn test_in_memory_delete() {
        let persistence = InMemoryMetricsPersistence::new();

        let metrics = AgentMetrics::new("test-agent".to_string());
        persistence
            .save_metrics("test-agent", &metrics)
            .await
            .unwrap();

        persistence.delete_metrics("test-agent").await.unwrap();

        let loaded = persistence.load_metrics("test-agent").await.unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_persisted_agent_metrics_new() {
        let mut agents = HashMap::new();
        agents.insert(
            "agent-1".to_string(),
            AgentMetrics::new("agent-1".to_string()),
        );

        let fleet = AgentMetrics::new("fleet".to_string());

        let persisted = PersistedAgentMetrics::new(agents, fleet);
        assert_eq!(persisted.version, 1);
        assert_eq!(persisted.agents.len(), 1);
        assert_eq!(persisted.fleet.agent_name, "fleet");
    }
}
