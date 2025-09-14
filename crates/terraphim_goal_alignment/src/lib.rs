//! # Terraphim Goal Alignment System
//!
//! Knowledge graph-based goal alignment system for multi-level goal management and conflict resolution.
//!
//! This crate provides intelligent goal management that leverages Terraphim's knowledge graph
//! infrastructure to ensure goal hierarchy consistency, detect conflicts, and propagate goals
//! through role hierarchies. It integrates with the agent registry and role graph systems
//! to provide context-aware goal alignment.
//!
//! ## Core Features
//!
//! - **Multi-level Goal Management**: Global, high-level, and local goal alignment
//! - **Knowledge Graph Integration**: Uses existing `extract_paragraphs_from_automata` and
//!   `is_all_terms_connected_by_path` for intelligent goal analysis
//! - **Conflict Detection**: Semantic conflict detection using knowledge graph analysis
//! - **Goal Propagation**: Intelligent goal distribution through role hierarchies
//! - **Dynamic Alignment**: Real-time goal alignment as system state changes
//! - **Performance Optimization**: Efficient caching and incremental updates

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export core types
pub use terraphim_agent_registry::{AgentMetadata, AgentPid, AgentRole};
pub use terraphim_gen_agent::{GenAgentResult, SupervisorId};
pub use terraphim_types::*;

pub mod alignment;
pub mod conflicts;
pub mod error;
pub mod goals;
pub mod knowledge_graph;
pub mod propagation;

pub use alignment::*;
pub use conflicts::*;
pub use error::*;
pub use goals::*;
pub use knowledge_graph::*;
pub use propagation::*;

/// Result type for goal alignment operations
pub type GoalAlignmentResult<T> = Result<T, GoalAlignmentError>;

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
