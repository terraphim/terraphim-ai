//! Main agent registry implementation
//!
//! Provides the core agent registry functionality with knowledge graph integration,
//! role-based specialization, and intelligent agent discovery.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use terraphim_rolegraph::RoleGraph;

use crate::{
    AgentCapability, AgentDiscoveryQuery, AgentDiscoveryResult, AgentMetadata, AgentPid, AgentRole,
    AutomataConfig, KnowledgeGraphIntegration, RegistryError, RegistryResult, SimilarityThresholds,
    SupervisorId,
};

/// Agent registry trait for different implementations
#[async_trait]
pub trait AgentRegistry: Send + Sync {
    /// Register a new agent
    async fn register_agent(&self, metadata: AgentMetadata) -> RegistryResult<()>;

    /// Unregister an agent
    async fn unregister_agent(&self, agent_id: &AgentPid) -> RegistryResult<()>;

    /// Update agent metadata
    async fn update_agent(&self, metadata: AgentMetadata) -> RegistryResult<()>;

    /// Get agent metadata by ID
    async fn get_agent(&self, agent_id: &AgentPid) -> RegistryResult<Option<AgentMetadata>>;

    /// List all registered agents
    async fn list_agents(&self) -> RegistryResult<Vec<AgentMetadata>>;

    /// Discover agents based on requirements
    async fn discover_agents(
        &self,
        query: AgentDiscoveryQuery,
    ) -> RegistryResult<AgentDiscoveryResult>;

    /// Find agents by role
    async fn find_agents_by_role(&self, role_id: &str) -> RegistryResult<Vec<AgentMetadata>>;

    /// Find agents by capability
    async fn find_agents_by_capability(
        &self,
        capability_id: &str,
    ) -> RegistryResult<Vec<AgentMetadata>>;

    /// Find agents by supervisor
    async fn find_agents_by_supervisor(
        &self,
        supervisor_id: &SupervisorId,
    ) -> RegistryResult<Vec<AgentMetadata>>;

    /// Get registry statistics
    async fn get_statistics(&self) -> RegistryResult<RegistryStatistics>;
}

/// Knowledge graph-based agent registry implementation
pub struct KnowledgeGraphAgentRegistry {
    /// Registered agents storage
    agents: Arc<RwLock<HashMap<AgentPid, AgentMetadata>>>,
    /// Knowledge graph integration
    kg_integration: Arc<KnowledgeGraphIntegration>,
    /// Registry configuration
    config: RegistryConfig,
    /// Registry statistics
    statistics: Arc<RwLock<RegistryStatistics>>,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Maximum number of agents that can be registered
    pub max_agents: usize,
    /// Enable automatic cleanup of terminated agents
    pub auto_cleanup: bool,
    /// Cleanup interval in seconds
    pub cleanup_interval_secs: u64,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
    /// Cache TTL for discovery queries in seconds
    pub discovery_cache_ttl_secs: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_agents: 10000,
            auto_cleanup: true,
            cleanup_interval_secs: 300, // 5 minutes
            enable_monitoring: true,
            discovery_cache_ttl_secs: 3600, // 1 hour
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatistics {
    /// Total number of registered agents
    pub total_agents: usize,
    /// Agents by status
    pub agents_by_status: HashMap<String, usize>,
    /// Agents by role
    pub agents_by_role: HashMap<String, usize>,
    /// Total discovery queries processed
    pub total_discovery_queries: u64,
    /// Average discovery query time
    pub avg_discovery_time_ms: f64,
    /// Cache hit rate for discovery queries
    pub discovery_cache_hit_rate: f64,
    /// Registry uptime
    pub uptime_secs: u64,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for RegistryStatistics {
    fn default() -> Self {
        Self {
            total_agents: 0,
            agents_by_status: HashMap::new(),
            agents_by_role: HashMap::new(),
            total_discovery_queries: 0,
            avg_discovery_time_ms: 0.0,
            discovery_cache_hit_rate: 0.0,
            uptime_secs: 0,
            last_updated: chrono::Utc::now(),
        }
    }
}

impl KnowledgeGraphAgentRegistry {
    /// Create a new knowledge graph-based agent registry
    pub fn new(
        role_graph: Arc<RoleGraph>,
        config: RegistryConfig,
        automata_config: AutomataConfig,
        similarity_thresholds: SimilarityThresholds,
    ) -> Self {
        let kg_integration = Arc::new(KnowledgeGraphIntegration::new(
            role_graph,
            automata_config,
            similarity_thresholds,
        ));

        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            kg_integration,
            config,
            statistics: Arc::new(RwLock::new(RegistryStatistics::default())),
        }
    }

    /// Start background tasks for the registry
    pub async fn start_background_tasks(&self) -> RegistryResult<()> {
        if self.config.auto_cleanup {
            self.start_cleanup_task().await?;
        }

        if self.config.enable_monitoring {
            self.start_monitoring_task().await?;
        }

        Ok(())
    }

    /// Start automatic cleanup task
    async fn start_cleanup_task(&self) -> RegistryResult<()> {
        let agents = self.agents.clone();
        let statistics = self.statistics.clone();
        let cleanup_interval = self.config.cleanup_interval_secs;

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(cleanup_interval));

            loop {
                interval.tick().await;

                // Clean up terminated agents
                let mut agents_guard = agents.write().await;
                let initial_count = agents_guard.len();

                agents_guard
                    .retain(|_, agent| !matches!(agent.status, crate::AgentStatus::Terminated));

                let cleaned_count = initial_count - agents_guard.len();
                drop(agents_guard);

                if cleaned_count > 0 {
                    log::info!("Cleaned up {} terminated agents", cleaned_count);

                    // Update statistics
                    let mut stats = statistics.write().await;
                    stats.total_agents = stats.total_agents.saturating_sub(cleaned_count);
                    stats.last_updated = chrono::Utc::now();
                }
            }
        });

        Ok(())
    }

    /// Start monitoring task
    async fn start_monitoring_task(&self) -> RegistryResult<()> {
        let statistics = self.statistics.clone();
        let kg_integration = self.kg_integration.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            let start_time = std::time::Instant::now();

            loop {
                interval.tick().await;

                // Update uptime
                {
                    let mut stats = statistics.write().await;
                    stats.uptime_secs = start_time.elapsed().as_secs();
                    stats.last_updated = chrono::Utc::now();
                }

                // Clean up knowledge graph cache
                kg_integration.cleanup_cache().await;
            }
        });

        Ok(())
    }

    /// Update registry statistics
    async fn update_statistics(&self) -> RegistryResult<()> {
        let agents = self.agents.read().await;
        let mut stats = self.statistics.write().await;

        stats.total_agents = agents.len();

        // Count agents by status
        stats.agents_by_status.clear();
        for agent in agents.values() {
            let status_key = match &agent.status {
                crate::AgentStatus::Initializing => "initializing",
                crate::AgentStatus::Active => "active",
                crate::AgentStatus::Busy => "busy",
                crate::AgentStatus::Idle => "idle",
                crate::AgentStatus::Hibernating => "hibernating",
                crate::AgentStatus::Terminating => "terminating",
                crate::AgentStatus::Terminated => "terminated",
                crate::AgentStatus::Failed(_) => "failed",
            };
            *stats
                .agents_by_status
                .entry(status_key.to_string())
                .or_insert(0) += 1;
        }

        // Count agents by role
        stats.agents_by_role.clear();
        for agent in agents.values() {
            *stats
                .agents_by_role
                .entry(agent.primary_role.role_id.clone())
                .or_insert(0) += 1;
        }

        stats.last_updated = chrono::Utc::now();

        Ok(())
    }

    /// Validate agent metadata before registration
    fn validate_agent_metadata(&self, metadata: &AgentMetadata) -> RegistryResult<()> {
        // Validate metadata consistency
        metadata.validate()?;

        // Check if agent ID is unique (this should be checked by caller)
        // Additional validation can be added here

        Ok(())
    }
}

#[async_trait]
impl AgentRegistry for KnowledgeGraphAgentRegistry {
    async fn register_agent(&self, metadata: AgentMetadata) -> RegistryResult<()> {
        // Validate metadata
        self.validate_agent_metadata(&metadata)?;

        // Check capacity
        {
            let agents = self.agents.read().await;
            if agents.len() >= self.config.max_agents {
                return Err(RegistryError::System(format!(
                    "Registry capacity exceeded (max: {})",
                    self.config.max_agents
                )));
            }
        }

        // Register the agent
        let agent_id = metadata.agent_id.clone();
        {
            let mut agents = self.agents.write().await;

            // Check if agent already exists
            if agents.contains_key(&metadata.agent_id) {
                return Err(RegistryError::AgentAlreadyExists(metadata.agent_id.clone()));
            }

            agents.insert(agent_id.clone(), metadata);
        }

        // Update statistics
        self.update_statistics().await?;

        log::info!("Agent {} registered successfully", agent_id);
        Ok(())
    }

    async fn unregister_agent(&self, agent_id: &AgentPid) -> RegistryResult<()> {
        let removed = {
            let mut agents = self.agents.write().await;
            agents.remove(agent_id)
        };

        if removed.is_some() {
            self.update_statistics().await?;
            log::info!("Agent {} unregistered successfully", agent_id);
            Ok(())
        } else {
            Err(RegistryError::AgentNotFound(agent_id.clone()))
        }
    }

    async fn update_agent(&self, metadata: AgentMetadata) -> RegistryResult<()> {
        // Validate metadata
        self.validate_agent_metadata(&metadata)?;

        {
            let mut agents = self.agents.write().await;

            if !agents.contains_key(&metadata.agent_id) {
                return Err(RegistryError::AgentNotFound(metadata.agent_id.clone()));
            }

            agents.insert(metadata.agent_id.clone(), metadata);
        }

        // Update statistics
        self.update_statistics().await?;

        Ok(())
    }

    async fn get_agent(&self, agent_id: &AgentPid) -> RegistryResult<Option<AgentMetadata>> {
        let agents = self.agents.read().await;
        Ok(agents.get(agent_id).cloned())
    }

    async fn list_agents(&self) -> RegistryResult<Vec<AgentMetadata>> {
        let agents = self.agents.read().await;
        Ok(agents.values().cloned().collect())
    }

    async fn discover_agents(
        &self,
        query: AgentDiscoveryQuery,
    ) -> RegistryResult<AgentDiscoveryResult> {
        let start_time = std::time::Instant::now();

        // Get all available agents
        let available_agents = self.list_agents().await?;

        // Use knowledge graph integration for discovery
        let result = self
            .kg_integration
            .discover_agents(query, &available_agents)
            .await?;

        // Update statistics
        {
            let mut stats = self.statistics.write().await;
            stats.total_discovery_queries += 1;

            let query_time_ms = start_time.elapsed().as_millis() as f64;
            if stats.total_discovery_queries == 1 {
                stats.avg_discovery_time_ms = query_time_ms;
            } else {
                let total_time =
                    stats.avg_discovery_time_ms * (stats.total_discovery_queries - 1) as f64;
                stats.avg_discovery_time_ms =
                    (total_time + query_time_ms) / stats.total_discovery_queries as f64;
            }

            stats.last_updated = chrono::Utc::now();
        }

        Ok(result)
    }

    async fn find_agents_by_role(&self, role_id: &str) -> RegistryResult<Vec<AgentMetadata>> {
        let agents = self.agents.read().await;
        let matching_agents: Vec<AgentMetadata> = agents
            .values()
            .filter(|agent| agent.has_role(role_id))
            .cloned()
            .collect();

        Ok(matching_agents)
    }

    async fn find_agents_by_capability(
        &self,
        capability_id: &str,
    ) -> RegistryResult<Vec<AgentMetadata>> {
        let agents = self.agents.read().await;
        let matching_agents: Vec<AgentMetadata> = agents
            .values()
            .filter(|agent| agent.has_capability(capability_id))
            .cloned()
            .collect();

        Ok(matching_agents)
    }

    async fn find_agents_by_supervisor(
        &self,
        supervisor_id: &SupervisorId,
    ) -> RegistryResult<Vec<AgentMetadata>> {
        let agents = self.agents.read().await;
        let matching_agents: Vec<AgentMetadata> = agents
            .values()
            .filter(|agent| agent.supervisor_id == *supervisor_id)
            .cloned()
            .collect();

        Ok(matching_agents)
    }

    async fn get_statistics(&self) -> RegistryResult<RegistryStatistics> {
        // Update statistics before returning
        self.update_statistics().await?;

        let stats = self.statistics.read().await;
        Ok(stats.clone())
    }
}

/// Registry builder for easy configuration
pub struct RegistryBuilder {
    role_graph: Option<Arc<RoleGraph>>,
    config: RegistryConfig,
    automata_config: AutomataConfig,
    similarity_thresholds: SimilarityThresholds,
}

impl RegistryBuilder {
    pub fn new() -> Self {
        Self {
            role_graph: None,
            config: RegistryConfig::default(),
            automata_config: AutomataConfig::default(),
            similarity_thresholds: SimilarityThresholds::default(),
        }
    }

    pub fn with_role_graph(mut self, role_graph: Arc<RoleGraph>) -> Self {
        self.role_graph = Some(role_graph);
        self
    }

    pub fn with_config(mut self, config: RegistryConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_automata_config(mut self, automata_config: AutomataConfig) -> Self {
        self.automata_config = automata_config;
        self
    }

    pub fn with_similarity_thresholds(
        mut self,
        similarity_thresholds: SimilarityThresholds,
    ) -> Self {
        self.similarity_thresholds = similarity_thresholds;
        self
    }

    pub fn build(self) -> RegistryResult<KnowledgeGraphAgentRegistry> {
        let role_graph = self
            .role_graph
            .ok_or_else(|| RegistryError::System("Role graph is required".to_string()))?;

        Ok(KnowledgeGraphAgentRegistry::new(
            role_graph,
            self.config,
            self.automata_config,
            self.similarity_thresholds,
        ))
    }
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentCapability, AgentRole, CapabilityMetrics};

    #[tokio::test]
    async fn test_registry_creation() {
        let role_graph = Arc::new(RoleGraph::new());
        let config = RegistryConfig::default();
        let automata_config = AutomataConfig::default();
        let similarity_thresholds = SimilarityThresholds::default();

        let registry = KnowledgeGraphAgentRegistry::new(
            role_graph,
            config,
            automata_config,
            similarity_thresholds,
        );

        let stats = registry.get_statistics().await.unwrap();
        assert_eq!(stats.total_agents, 0);
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let role_graph = Arc::new(RoleGraph::new());
        let registry = RegistryBuilder::new()
            .with_role_graph(role_graph)
            .build()
            .unwrap();

        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let role = AgentRole::new(
            "test_role".to_string(),
            "Test Role".to_string(),
            "A test role".to_string(),
        );

        let metadata = AgentMetadata::new(agent_id.clone(), supervisor_id, role);

        // Register agent
        registry.register_agent(metadata.clone()).await.unwrap();

        // Verify registration
        let retrieved = registry.get_agent(&agent_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().agent_id, agent_id);

        // Check statistics
        let stats = registry.get_statistics().await.unwrap();
        assert_eq!(stats.total_agents, 1);
    }

    #[tokio::test]
    async fn test_agent_discovery() {
        let role_graph = Arc::new(RoleGraph::new());
        let registry = RegistryBuilder::new()
            .with_role_graph(role_graph)
            .build()
            .unwrap();

        // Register a test agent
        let agent_id = AgentPid::new();
        let supervisor_id = SupervisorId::new();
        let mut role = AgentRole::new(
            "planner".to_string(),
            "Planning Agent".to_string(),
            "Responsible for task planning".to_string(),
        );
        role.knowledge_domains
            .push("project_management".to_string());

        let mut metadata = AgentMetadata::new(agent_id, supervisor_id, role);

        let capability = AgentCapability {
            capability_id: "task_planning".to_string(),
            name: "Task Planning".to_string(),
            description: "Plan and organize tasks".to_string(),
            category: "planning".to_string(),
            required_domains: vec!["project_management".to_string()],
            input_types: vec!["requirements".to_string()],
            output_types: vec!["plan".to_string()],
            performance_metrics: CapabilityMetrics::default(),
            dependencies: Vec::new(),
        };

        metadata.add_capability(capability).unwrap();
        registry.register_agent(metadata).await.unwrap();

        // Create discovery query
        let query = AgentDiscoveryQuery {
            required_roles: vec!["planner".to_string()],
            required_capabilities: vec!["task_planning".to_string()],
            required_domains: vec!["project_management".to_string()],
            task_description: Some("Plan a software project".to_string()),
            min_success_rate: None,
            max_resource_usage: None,
            preferred_tags: Vec::new(),
        };

        // Discover agents
        let result = registry.discover_agents(query).await.unwrap();

        assert!(!result.matches.is_empty());
        assert!(result.matches[0].match_score > 0.0);
    }

    #[tokio::test]
    async fn test_registry_builder() {
        let role_graph = Arc::new(RoleGraph::new());
        let config = RegistryConfig {
            max_agents: 100,
            auto_cleanup: false,
            cleanup_interval_secs: 60,
            enable_monitoring: false,
            discovery_cache_ttl_secs: 1800,
        };

        let registry = RegistryBuilder::new()
            .with_role_graph(role_graph)
            .with_config(config.clone())
            .build()
            .unwrap();

        assert_eq!(registry.config.max_agents, 100);
        assert!(!registry.config.auto_cleanup);
    }
}
