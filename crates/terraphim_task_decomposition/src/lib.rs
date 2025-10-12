//! # Terraphim Task Decomposition System
//!
//! Knowledge graph-based task decomposition system for intelligent task analysis and execution planning.
//!
//! This crate provides sophisticated task analysis and decomposition capabilities that leverage
//! Terraphim's knowledge graph infrastructure to break down complex tasks into manageable subtasks,
//! generate execution plans, and assign tasks to appropriate agents based on their roles and capabilities.
//!
//! ## Core Features
//!
//! - **Task Analysis**: Deep analysis of task complexity using knowledge graph traversal
//! - **Knowledge Graph Integration**: Uses existing `extract_paragraphs_from_automata` and
//!   `is_all_terms_connected_by_path` for intelligent task decomposition
//! - **Execution Planning**: Generate step-by-step execution plans with dependencies
//! - **Role-aware Assignment**: Leverage `terraphim_rolegraph` for optimal task-to-role matching
//! - **Goal Integration**: Seamless integration with goal alignment system
//! - **Performance Optimization**: Efficient caching and incremental decomposition

// Re-export core types
// pub use terraphim_agent_registry::{AgentPid, AgentMetadata};
// pub use terraphim_goal_alignment::{Goal, GoalId};

// Temporary type definitions until dependencies are fixed
pub type AgentPid = String;
pub type AgentMetadata = std::collections::HashMap<String, String>;
pub type Goal = String;
pub type GoalId = String;
pub use terraphim_types::*;

// Shared mock automata type
#[derive(Debug, Clone, Default)]
pub struct MockAutomata;
pub type Automata = MockAutomata;

pub mod analysis;
pub mod decomposition;
pub mod error;
pub mod knowledge_graph;
pub mod planning;
pub mod system;
pub mod tasks;

pub use analysis::*;
pub use decomposition::{
    DecompositionConfig, DecompositionMetadata, DecompositionResult, DecompositionStrategy,
    KnowledgeGraphTaskDecomposer, TaskDecomposer,
};
pub use error::*;
pub use knowledge_graph::{
    KnowledgeGraphConfig, KnowledgeGraphIntegration, KnowledgeGraphQuery, QueryResult,
    QueryResultData, QueryType, TerraphimKnowledgeGraph,
};
pub use planning::*;
pub use system::*;
pub use tasks::*;

/// Result type for task decomposition operations
pub type TaskDecompositionResult<T> = Result<T, TaskDecompositionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_imports() {
        // Test that all modules compile and basic types are available
        let _agent_id: AgentPid = "test_agent".to_string();
        let _goal_id: GoalId = "test_goal".to_string();
    }
}
