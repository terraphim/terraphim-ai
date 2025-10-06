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
pub mod agents;
pub mod context;
pub mod error;
pub mod genai_llm_client;
pub mod history;
pub mod llm_types;
pub mod vm_execution;
// pub mod llm_client;      // Disabled - uses rig-core
// pub mod simple_llm_client; // Disabled - uses rig-core
pub mod pool;
pub mod pool_manager;
pub mod registry;
pub mod tracking;
pub mod workflows;

pub use agent::*;
pub use agents::*;
pub use context::*;
pub use error::*;
pub use genai_llm_client::*;
pub use history::*;
pub use llm_types::*;
// pub use llm_client::*;      // Disabled - uses rig-core
// pub use simple_llm_client::*; // Disabled - uses rig-core
pub use pool::*;
pub use pool_manager::*;
pub use registry::*;
pub use tracking::*;
pub use workflows::*;

/// Result type for multi-agent operations
pub type MultiAgentResult<T> = Result<T, MultiAgentError>;

/// Agent identifier type
pub type AgentId = uuid::Uuid;

// Test utilities using real Ollama with gemma3:270m model
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    use super::*;
    use std::sync::Arc;
    use terraphim_config::Role;
    use terraphim_persistence::DeviceStorage;

    pub fn create_test_role() -> Role {
        let mut role = Role::new("TestAgent");
        role.shortname = Some("test".to_string());
        role.relevance_function = terraphim_types::RelevanceFunction::BM25;
        // Use rust-genai with Ollama for local testing with gemma3:270m
        role.extra
            .insert("llm_provider".to_string(), serde_json::json!("ollama"));
        role.extra
            .insert("llm_model".to_string(), serde_json::json!("gemma3:270m"));
        role.extra.insert(
            "ollama_base_url".to_string(),
            serde_json::json!("http://127.0.0.1:11434"),
        );
        role
    }

    pub async fn create_test_agent_simple() -> Result<TerraphimAgent, MultiAgentError> {
        use terraphim_persistence::memory::create_memory_only_device_settings;

        let _settings = create_memory_only_device_settings()
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        // Initialize memory storage
        DeviceStorage::init_memory_only()
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        let storage_ref = DeviceStorage::instance()
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        // Use the same unsafe pattern from the examples
        use std::ptr;
        let storage_copy = unsafe { ptr::read(storage_ref) };
        let persistence = Arc::new(storage_copy);

        let role = create_test_role();
        TerraphimAgent::new(role, persistence, None).await
    }

    // For now, alias the simpler version for tests
    pub async fn create_test_agent() -> Result<TerraphimAgent, MultiAgentError> {
        create_test_agent_simple().await
    }

    /// Create memory storage for testing
    pub async fn create_memory_storage() -> Result<Arc<DeviceStorage>, MultiAgentError> {
        use terraphim_persistence::memory::create_memory_only_device_settings;

        let _settings = create_memory_only_device_settings()
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        // Initialize memory storage
        DeviceStorage::init_memory_only()
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        let storage_ref = DeviceStorage::instance()
            .await
            .map_err(|e| MultiAgentError::PersistenceError(e.to_string()))?;

        // Use the same unsafe pattern from the examples
        use std::ptr;
        let storage_copy = unsafe { ptr::read(storage_ref) };
        Ok(Arc::new(storage_copy))
    }

    /// Create test rolegraph for testing
    pub async fn create_test_rolegraph(
    ) -> Result<Arc<terraphim_rolegraph::RoleGraph>, MultiAgentError> {
        // Create a simple test rolegraph with empty thesaurus
        use terraphim_types::Thesaurus;
        let empty_thesaurus = Thesaurus::new("test_thesaurus".to_string());
        let rolegraph = terraphim_rolegraph::RoleGraph::new("TestRole".into(), empty_thesaurus)
            .await
            .map_err(|e| {
                MultiAgentError::KnowledgeGraphError(format!(
                    "Failed to create test rolegraph: {}",
                    e
                ))
            })?;
        Ok(Arc::new(rolegraph))
    }

    /// Create test automata for testing
    pub fn create_test_automata(
    ) -> Result<Arc<terraphim_automata::AutocompleteIndex>, MultiAgentError> {
        // Create a simple test automata index with empty thesaurus
        use terraphim_types::Thesaurus;
        let empty_thesaurus = Thesaurus::new("test_thesaurus".to_string());
        let automata = terraphim_automata::build_autocomplete_index(empty_thesaurus, None)
            .map_err(|e| {
                MultiAgentError::KnowledgeGraphError(format!(
                    "Failed to create test automata: {}",
                    e
                ))
            })?;
        Ok(Arc::new(automata))
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
