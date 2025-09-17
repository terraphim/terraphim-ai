//! Agent registry for discovery and management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{AgentId, AgentStatus, MultiAgentError, MultiAgentResult, TerraphimAgent};

/// Information about a registered agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub name: String,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
}

/// Agent registry for managing multiple agents
#[derive()]
pub struct AgentRegistry {
    /// All registered agents
    agents: Arc<RwLock<HashMap<AgentId, Arc<TerraphimAgent>>>>,
    /// Capability mapping
    capabilities: Arc<RwLock<HashMap<String, Vec<AgentId>>>>,
    /// Role name to agent mapping
    role_agents: Arc<RwLock<HashMap<String, AgentId>>>,
    /// Agent load metrics
    agent_load: Arc<RwLock<HashMap<AgentId, LoadMetrics>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            capabilities: Arc::new(RwLock::new(HashMap::new())),
            role_agents: Arc::new(RwLock::new(HashMap::new())),
            agent_load: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new agent
    pub async fn register_agent(&self, agent: Arc<TerraphimAgent>) -> MultiAgentResult<()> {
        let agent_id = agent.agent_id;
        let role_name = agent.role_config.name.clone();
        let capabilities = agent.get_capabilities();

        {
            let mut agents = self.agents.write().await;
            if agents.contains_key(&agent_id) {
                return Err(MultiAgentError::AgentAlreadyExists(agent_id));
            }
            agents.insert(agent_id, agent);
        }

        {
            let mut role_agents = self.role_agents.write().await;
            role_agents.insert(role_name.to_string(), agent_id);
        }

        {
            let mut cap_map = self.capabilities.write().await;
            for capability in capabilities {
                cap_map
                    .entry(capability)
                    .or_insert_with(Vec::new)
                    .push(agent_id);
            }
        }

        {
            let mut load_map = self.agent_load.write().await;
            load_map.insert(agent_id, LoadMetrics::new());
        }

        log::info!("Registered agent {} in registry", agent_id);
        Ok(())
    }

    /// Get agent by ID
    pub async fn get_agent(&self, agent_id: &AgentId) -> Option<Arc<TerraphimAgent>> {
        let agents = self.agents.read().await;
        agents.get(agent_id).cloned()
    }

    /// Find agents by capability
    pub async fn find_agents_by_capability(&self, capability: &str) -> Vec<AgentId> {
        let capabilities = self.capabilities.read().await;
        capabilities.get(capability).cloned().unwrap_or_default()
    }

    /// List all registered agents
    pub async fn list_agents(&self) -> Vec<AgentId> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }

    /// List all agents with their information
    pub async fn list_all_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        let mut result = Vec::new();
        
        for (id, agent) in agents.iter() {
            let status = agent.status.read().await.clone();
            result.push(AgentInfo {
                id: *id,
                name: agent.role_config.name.to_string(),
                capabilities: vec![], // TODO: Extract capabilities from agent
                status,
            });
        }
        
        result
    }

    /// Get all agents (for workflow orchestration)
    pub async fn get_all_agents(&self) -> Vec<Arc<TerraphimAgent>> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Load metrics for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadMetrics {
    pub active_commands: u32,
    pub queue_length: u32,
    pub average_response_time_ms: f64,
    pub success_rate: f64,
    pub last_updated: DateTime<Utc>,
}

impl LoadMetrics {
    pub fn new() -> Self {
        Self {
            active_commands: 0,
            queue_length: 0,
            average_response_time_ms: 0.0,
            success_rate: 1.0,
            last_updated: Utc::now(),
        }
    }
}

impl Default for LoadMetrics {
    fn default() -> Self {
        Self::new()
    }
}
