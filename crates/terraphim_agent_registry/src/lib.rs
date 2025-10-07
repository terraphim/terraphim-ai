//! # Terraphim Agent Registry
//!
//! Knowledge graph-based agent registry for intelligent agent discovery and capability matching.
//!
//! This crate provides a sophisticated agent registry that leverages Terraphim's knowledge graph
//! infrastructure to enable intelligent agent discovery, capability matching, and role-based
//! specialization. It integrates with the existing automata and role graph systems to provide
//! context-aware agent management.
//!
//! ## Core Features
//!
//! - **Knowledge Graph Integration**: Uses existing `extract_paragraphs_from_automata` and
//!   `is_all_terms_connected_by_path` for intelligent agent discovery
//! - **Role-Based Specialization**: Leverages `terraphim_rolegraph` for agent role management
//! - **Capability Matching**: Semantic matching of agent capabilities to task requirements
//! - **Agent Metadata**: Rich metadata storage with knowledge graph context
//! - **Dynamic Discovery**: Real-time agent discovery based on evolving requirements
//! - **Performance Optimization**: Efficient indexing and caching for fast lookups

// Re-export core types
pub use terraphim_agent_supervisor::{AgentPid, SupervisorId};
pub use terraphim_agent_supervisor::{AgentSpec, RestartStrategy};

// Define GenAgentResult locally since we removed gen_agent dependency
pub type GenAgentResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub use terraphim_types::*;

pub mod capabilities;
pub mod discovery;
pub mod error;
pub mod knowledge_graph;
pub mod matching;
pub mod metadata;
pub mod registry;

pub use capabilities::*;
pub use discovery::*;
pub use error::*;
pub use knowledge_graph::*;
pub use matching::*;
pub use metadata::*;
pub use registry::*;

/// Result type for agent registry operations
pub type RegistryResult<T> = Result<T, RegistryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_imports() {
        // Test that all modules compile and basic types are available
        let _agent_id = AgentPid::new();
        let _supervisor_id = SupervisorId::new();
    }
}
