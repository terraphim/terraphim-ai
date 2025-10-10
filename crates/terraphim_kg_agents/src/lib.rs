//! # Terraphim Knowledge Graph Agents
//!
//! Specialized GenAgent implementations that leverage knowledge graph capabilities
//! for intelligent task planning, execution, and coordination.
//!
//! This crate provides concrete implementations of the GenAgent trait that integrate
//! deeply with Terraphim's knowledge graph infrastructure to provide:
//!
//! - **Planning Agents**: Intelligent task decomposition and execution planning
//! - **Worker Agents**: Domain-specialized task execution with knowledge graph context
//! - **Coordination Agents**: Multi-agent workflow coordination and supervision
//!
//! ## Core Features
//!
//! - **Knowledge Graph Integration**: Deep integration with automata and role graphs
//! - **Domain Specialization**: Agents specialized for specific knowledge domains
//! - **Task Compatibility**: Intelligent task-agent matching using connectivity analysis
//! - **Context-Aware Execution**: Task execution guided by knowledge graph context
//! - **Coordination Capabilities**: Multi-agent workflow orchestration

// Re-export core types
pub use terraphim_agent_registry::{AgentMetadata, AgentPid, SupervisorId};
// Define GenAgent types locally since we removed gen_agent dependency
pub type GenAgentResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum GenAgentError {
    #[error("Execution error: {0}")]
    ExecutionError(String),
}
pub use terraphim_types::*;

// TODO: Re-enable when GenAgent trait framework is implemented
// These modules depend on terraphim_gen_agent and terraphim_automata::Automata
// which don't exist yet in the current codebase
// pub mod coordination;
pub mod error;
// pub mod planning;
// pub mod worker;

// pub use coordination::*;
pub use error::*;
// pub use planning::*;
// pub use worker::*;

/// Result type for knowledge graph agent operations
pub type KgAgentResult<T> = Result<T, KgAgentError>;

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
