//! # Terraphim Multi-Agent System
//!
//! A production-ready multi-agent system built on Terraphim's role-based architecture
//! with Rig framework integration for professional LLM management.
//!
//! ## Core Concepts
//!
//! - **Role-as-Agent**: Each Terraphim Role configuration becomes an autonomous agent
//! - **Rig Integration**: Professional LLM management with built-in token/cost tracking
//! - **Knowledge Graph Intelligence**: Agents use rolegraph/automata for capabilities
//! - **Individual Evolution**: Each agent has own memory/tasks/lessons tracking
//! - **Multi-Agent Coordination**: Discovery, communication, and collaboration
//!
//! ## Architecture
//!
//! ```text
//! TerraphimAgent {
//!   Role Config + Rig Agent + Knowledge Graph + Individual Evolution
//! }
//!
//! AgentRegistry {
//!   Discovery + Capability Mapping + Load Balancing + Task Routing
//! }
//!
//! Multi-Agent Workflows {
//!   Role Chaining + Role Routing + Role Parallelization + Lead-Specialist + Review-Optimize
//! }
//! ```

pub mod agent;
pub mod context;
pub mod error;
pub mod history;
pub mod llm_client;
pub mod registry;
pub mod tracking;
pub mod workflows;

pub use agent::*;
pub use context::*;
pub use error::*;
pub use history::*;
pub use llm_client::*;
pub use registry::*;
pub use tracking::*;
pub use workflows::*;

/// Result type for multi-agent operations
pub type MultiAgentResult<T> = Result<T, MultiAgentError>;

/// Agent identifier type
pub type AgentId = uuid::Uuid;

// Test utilities using real Ollama with gemma3:270m model
pub mod test_utils {
    use super::*;
    use ahash::AHashMap;
    use std::sync::Arc;
    use terraphim_config::Role;
    use terraphim_persistence::DeviceStorage;

    pub fn create_test_role() -> Role {
        Role {
            shortname: Some("test".to_string()),
            name: "TestAgent".into(),
            relevance_function: terraphim_types::RelevanceFunction::BM25,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            extra: {
                let mut extra = AHashMap::new();
                extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
                extra.insert(
                    "ollama_base_url".to_string(),
                    serde_json::json!("http://127.0.0.1:11434"),
                );
                extra.insert("ollama_model".to_string(), serde_json::json!("gemma3:270m"));
                extra
            },
        }
    }

    pub async fn create_test_agent_with_mock_storage() -> Result<TerraphimAgent, MultiAgentError> {
        let role = create_test_role();

        // Create a mock storage for testing - this will be a simplified version
        // that doesn't require the complex DeviceStorage singleton
        use terraphim_persistence::memory::create_memory_only_device_settings;
        let _settings = create_memory_only_device_settings()
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        // Initialize real memory storage for testing
        DeviceStorage::init_memory_only()
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        // Use the singleton instance
        let storage_ref = DeviceStorage::instance()
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        // Clone the storage to avoid lifetime issues
        use std::ptr;
        let storage_copy = unsafe { ptr::read(storage_ref) };
        let persistence = Arc::new(storage_copy);

        TerraphimAgent::new(role, persistence, None).await
    }

    // For now, alias the simpler version for tests
    pub async fn create_test_agent() -> Result<TerraphimAgent, MultiAgentError> {
        create_test_agent_with_mock_storage().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_imports() {
        // Test that all modules compile and basic types are available
        let _agent_id = AgentId::new_v4();
    }
}
