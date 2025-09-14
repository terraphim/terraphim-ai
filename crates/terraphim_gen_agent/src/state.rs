//! State management for GenAgent framework
//!
//! Provides immutable state transitions with persistence and recovery capabilities.

use std::any::Any;
use std::fmt::Debug;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AgentPid, GenAgentError, GenAgentResult};

/// Trait for agent state that can be persisted and recovered
#[async_trait]
pub trait AgentState: Send + Sync + Debug + Clone {
    /// Serialize the state for persistence
    async fn serialize(&self) -> GenAgentResult<Vec<u8>>;

    /// Deserialize the state from persistence
    async fn deserialize(data: &[u8]) -> GenAgentResult<Self>
    where
        Self: Sized;

    /// Get a unique identifier for this state type
    fn state_type(&self) -> &'static str;

    /// Validate the state for consistency
    fn validate(&self) -> GenAgentResult<()> {
        Ok(())
    }

    /// Get the state version for migration support
    fn version(&self) -> u32 {
        1
    }
}

/// State transition result
#[derive(Debug, Clone)]
pub enum StateTransition<S: AgentState> {
    /// Continue with new state
    Continue(S),
    /// Stop the agent with reason
    Stop(String),
    /// Hibernate the agent (pause message processing)
    Hibernate(S),
}

impl<S: AgentState> StateTransition<S> {
    /// Check if this transition continues agent execution
    pub fn is_continue(&self) -> bool {
        matches!(self, StateTransition::Continue(_))
    }

    /// Check if this transition stops agent execution
    pub fn is_stop(&self) -> bool {
        matches!(self, StateTransition::Stop(_))
    }

    /// Check if this transition hibernates the agent
    pub fn is_hibernate(&self) -> bool {
        matches!(self, StateTransition::Hibernate(_))
    }

    /// Extract the new state (if available)
    pub fn state(self) -> Option<S> {
        match self {
            StateTransition::Continue(state) => Some(state),
            StateTransition::Hibernate(state) => Some(state),
            StateTransition::Stop(_) => None,
        }
    }

    /// Get the stop reason (if this is a stop transition)
    pub fn stop_reason(&self) -> Option<&str> {
        match self {
            StateTransition::Stop(reason) => Some(reason),
            _ => None,
        }
    }
}

/// State manager for handling agent state persistence and recovery
#[async_trait]
pub trait StateManager: Send + Sync {
    /// Save agent state
    async fn save_state<S: AgentState>(&self, agent_id: &AgentPid, state: &S)
        -> GenAgentResult<()>;

    /// Load agent state
    async fn load_state<S: AgentState>(&self, agent_id: &AgentPid) -> GenAgentResult<Option<S>>;

    /// Delete agent state
    async fn delete_state(&self, agent_id: &AgentPid) -> GenAgentResult<()>;

    /// Check if state exists for agent
    async fn has_state(&self, agent_id: &AgentPid) -> GenAgentResult<bool>;

    /// List all agents with saved state
    async fn list_agents(&self) -> GenAgentResult<Vec<AgentPid>>;
}

/// In-memory state manager for testing and development
pub struct InMemoryStateManager {
    states: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<AgentPid, Vec<u8>>>>,
}

impl InMemoryStateManager {
    pub fn new() -> Self {
        Self {
            states: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for InMemoryStateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StateManager for InMemoryStateManager {
    async fn save_state<S: AgentState>(
        &self,
        agent_id: &AgentPid,
        state: &S,
    ) -> GenAgentResult<()> {
        let data = state.serialize().await?;
        let mut states = self.states.write().await;
        states.insert(agent_id.clone(), data);
        Ok(())
    }

    async fn load_state<S: AgentState>(&self, agent_id: &AgentPid) -> GenAgentResult<Option<S>> {
        let states = self.states.read().await;
        if let Some(data) = states.get(agent_id) {
            let state = S::deserialize(data).await?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    async fn delete_state(&self, agent_id: &AgentPid) -> GenAgentResult<()> {
        let mut states = self.states.write().await;
        states.remove(agent_id);
        Ok(())
    }

    async fn has_state(&self, agent_id: &AgentPid) -> GenAgentResult<bool> {
        let states = self.states.read().await;
        Ok(states.contains_key(agent_id))
    }

    async fn list_agents(&self) -> GenAgentResult<Vec<AgentPid>> {
        let states = self.states.read().await;
        Ok(states.keys().cloned().collect())
    }
}

/// State snapshot for debugging and monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub agent_id: AgentPid,
    pub state_type: String,
    pub version: u32,
    pub timestamp: DateTime<Utc>,
    pub data_size: usize,
}

impl StateSnapshot {
    pub fn new<S: AgentState>(agent_id: AgentPid, state: &S, data_size: usize) -> Self {
        Self {
            agent_id,
            state_type: state.state_type().to_string(),
            version: state.version(),
            timestamp: Utc::now(),
            data_size,
        }
    }
}

/// State manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateManagerStats {
    pub total_states: usize,
    pub memory_usage_bytes: usize,
    pub save_operations: u64,
    pub load_operations: u64,
    pub errors: u64,
}

/// Example agent state implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleState {
    pub counter: u64,
    pub name: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl ExampleState {
    pub fn new(name: String) -> Self {
        Self {
            counter: 0,
            name,
            active: true,
            created_at: Utc::now(),
        }
    }

    pub fn increment(&mut self) {
        self.counter += 1;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

#[async_trait]
impl AgentState for ExampleState {
    async fn serialize(&self) -> GenAgentResult<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| GenAgentError::Serialization(e))
    }

    async fn deserialize(data: &[u8]) -> GenAgentResult<Self> {
        serde_json::from_slice(data).map_err(|e| GenAgentError::Serialization(e))
    }

    fn state_type(&self) -> &'static str {
        "ExampleState"
    }

    fn validate(&self) -> GenAgentResult<()> {
        if self.name.is_empty() {
            return Err(GenAgentError::System("Name cannot be empty".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_example_state_serialization() {
        let state = ExampleState::new("test_agent".to_string());

        // Test serialization
        let data = state.serialize().await.unwrap();
        assert!(!data.is_empty());

        // Test deserialization
        let deserialized = ExampleState::deserialize(&data).await.unwrap();
        assert_eq!(deserialized.name, "test_agent");
        assert_eq!(deserialized.counter, 0);
        assert!(deserialized.active);
    }

    #[tokio::test]
    async fn test_state_validation() {
        let valid_state = ExampleState::new("valid_name".to_string());
        assert!(valid_state.validate().is_ok());

        let invalid_state = ExampleState::new("".to_string());
        assert!(invalid_state.validate().is_err());
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let state = ExampleState::new("test".to_string());

        let continue_transition = StateTransition::Continue(state.clone());
        assert!(continue_transition.is_continue());
        assert!(!continue_transition.is_stop());

        let stop_transition = StateTransition::Stop("test reason".to_string());
        assert!(stop_transition.is_stop());
        assert_eq!(stop_transition.stop_reason(), Some("test reason"));

        let hibernate_transition = StateTransition::Hibernate(state.clone());
        assert!(hibernate_transition.is_hibernate());
    }

    #[tokio::test]
    async fn test_in_memory_state_manager() {
        let manager = InMemoryStateManager::new();
        let agent_id = AgentPid::new();
        let state = ExampleState::new("test_agent".to_string());

        // Initially no state
        assert!(!manager.has_state(&agent_id).await.unwrap());
        assert!(manager
            .load_state::<ExampleState>(&agent_id)
            .await
            .unwrap()
            .is_none());

        // Save state
        manager.save_state(&agent_id, &state).await.unwrap();
        assert!(manager.has_state(&agent_id).await.unwrap());

        // Load state
        let loaded = manager
            .load_state::<ExampleState>(&agent_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.name, "test_agent");
        assert_eq!(loaded.counter, 0);

        // List agents
        let agents = manager.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], agent_id);

        // Delete state
        manager.delete_state(&agent_id).await.unwrap();
        assert!(!manager.has_state(&agent_id).await.unwrap());
    }

    #[test]
    fn test_state_snapshot() {
        let agent_id = AgentPid::new();
        let state = ExampleState::new("test".to_string());
        let snapshot = StateSnapshot::new(agent_id.clone(), &state, 100);

        assert_eq!(snapshot.agent_id, agent_id);
        assert_eq!(snapshot.state_type, "ExampleState");
        assert_eq!(snapshot.version, 1);
        assert_eq!(snapshot.data_size, 100);
    }
}
