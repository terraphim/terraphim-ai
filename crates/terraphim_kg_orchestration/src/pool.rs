//! Agent pool management for orchestration

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{AgentMetadata, OrchestrationError, OrchestrationResult, SimpleAgent, Task};

/// Agent pool for managing available agents
pub struct AgentPool {
    /// Registered agents
    agents: Arc<RwLock<HashMap<String, Arc<dyn SimpleAgent>>>>,
}

impl AgentPool {
    /// Create a new agent pool
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an agent in the pool
    pub async fn register_agent(&self, agent: Arc<dyn SimpleAgent>) -> OrchestrationResult<()> {
        let agent_id = agent.agent_id().to_string();
        let mut agents = self.agents.write().await;

        if agents.contains_key(&agent_id) {
            return Err(OrchestrationError::AgentPoolError(format!(
                "Agent {} already registered",
                agent_id
            )));
        }

        agents.insert(agent_id, agent);
        Ok(())
    }

    /// Unregister an agent from the pool
    pub async fn unregister_agent(&self, agent_id: &str) -> OrchestrationResult<()> {
        let mut agents = self.agents.write().await;

        if agents.remove(agent_id).is_none() {
            return Err(OrchestrationError::AgentNotFound(agent_id.to_string()));
        }

        Ok(())
    }

    /// Get an agent by ID
    pub async fn get_agent(&self, agent_id: &str) -> OrchestrationResult<Arc<dyn SimpleAgent>> {
        let agents = self.agents.read().await;

        agents
            .get(agent_id)
            .cloned()
            .ok_or_else(|| OrchestrationError::AgentNotFound(agent_id.to_string()))
    }

    /// Find agents that can handle a specific task
    pub async fn find_suitable_agents(
        &self,
        task: &Task,
    ) -> OrchestrationResult<Vec<Arc<dyn SimpleAgent>>> {
        let agents = self.agents.read().await;
        let mut suitable_agents = Vec::new();

        for agent in agents.values() {
            if agent.can_handle_task(task) {
                suitable_agents.push(agent.clone());
            }
        }

        Ok(suitable_agents)
    }

    /// Get all registered agents
    pub async fn list_agents(&self) -> Vec<Arc<dyn SimpleAgent>> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    /// Get agent metadata for all registered agents
    pub async fn get_all_metadata(&self) -> Vec<AgentMetadata> {
        let agents = self.agents.read().await;
        agents.values().map(|agent| agent.metadata()).collect()
    }

    /// Get the number of registered agents
    pub async fn agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    /// Check if an agent is registered
    pub async fn has_agent(&self, agent_id: &str) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(agent_id)
    }
}

impl Default for AgentPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExampleAgent, TaskComplexity};

    #[tokio::test]
    async fn test_agent_pool_registration() {
        let pool = AgentPool::new();
        let agent = Arc::new(ExampleAgent::new(
            "test_agent".to_string(),
            vec!["capability1".to_string()],
        ));

        // Register agent
        assert!(pool.register_agent(agent.clone()).await.is_ok());
        assert_eq!(pool.agent_count().await, 1);
        assert!(pool.has_agent("test_agent").await);

        // Try to register same agent again (should fail)
        assert!(pool.register_agent(agent).await.is_err());
    }

    #[tokio::test]
    async fn test_agent_pool_unregistration() {
        let pool = AgentPool::new();
        let agent = Arc::new(ExampleAgent::new(
            "test_agent".to_string(),
            vec!["capability1".to_string()],
        ));

        // Register and then unregister
        pool.register_agent(agent).await.unwrap();
        assert_eq!(pool.agent_count().await, 1);

        pool.unregister_agent("test_agent").await.unwrap();
        assert_eq!(pool.agent_count().await, 0);
        assert!(!pool.has_agent("test_agent").await);

        // Try to unregister non-existent agent (should fail)
        assert!(pool.unregister_agent("non_existent").await.is_err());
    }

    #[tokio::test]
    async fn test_find_suitable_agents() {
        let pool = AgentPool::new();

        let agent1 = Arc::new(ExampleAgent::new(
            "agent1".to_string(),
            vec!["analysis".to_string()],
        ));

        let agent2 = Arc::new(ExampleAgent::new(
            "agent2".to_string(),
            vec!["processing".to_string()],
        ));

        pool.register_agent(agent1).await.unwrap();
        pool.register_agent(agent2).await.unwrap();

        let mut task = Task::new(
            "test_task".to_string(),
            "Test task".to_string(),
            TaskComplexity::Simple,
            1,
        );
        task.required_capabilities.push("analysis".to_string());

        let suitable_agents = pool.find_suitable_agents(&task).await.unwrap();
        assert_eq!(suitable_agents.len(), 1);
        assert_eq!(suitable_agents[0].agent_id(), "agent1");
    }

    #[tokio::test]
    async fn test_get_agent() {
        let pool = AgentPool::new();
        let agent = Arc::new(ExampleAgent::new(
            "test_agent".to_string(),
            vec!["capability1".to_string()],
        ));

        pool.register_agent(agent).await.unwrap();

        // Get existing agent
        let retrieved_agent = pool.get_agent("test_agent").await.unwrap();
        assert_eq!(retrieved_agent.agent_id(), "test_agent");

        // Try to get non-existent agent
        assert!(pool.get_agent("non_existent").await.is_err());
    }

    #[tokio::test]
    async fn test_list_agents() {
        let pool = AgentPool::new();

        let agent1 = Arc::new(ExampleAgent::new("agent1".to_string(), vec![]));
        let agent2 = Arc::new(ExampleAgent::new("agent2".to_string(), vec![]));

        pool.register_agent(agent1).await.unwrap();
        pool.register_agent(agent2).await.unwrap();

        let agents = pool.list_agents().await;
        assert_eq!(agents.len(), 2);

        let agent_ids: std::collections::HashSet<&str> =
            agents.iter().map(|a| a.agent_id()).collect();
        assert!(agent_ids.contains("agent1"));
        assert!(agent_ids.contains("agent2"));
    }
}
