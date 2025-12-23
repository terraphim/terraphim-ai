//! Learning coordination for knowledge graph updates
//!
//! This module provides:
//! - Recording success and failure patterns (coordinator.rs)
//! - Command knowledge graph integration (knowledge_graph.rs)
//! - Command thesaurus for node ID generation (thesaurus.rs)
//! - Knowledge graph weight updates based on execution outcomes

pub mod coordinator;
pub mod knowledge_graph;
pub mod thesaurus;

pub use coordinator::{
    ApplicableLesson, FailureTracker, InMemoryLearningCoordinator, LearningCoordinator,
    LearningStats, OptimizationType, SuccessPattern, WorkflowOptimization,
};

pub use knowledge_graph::{CommandGraphStats, CommandKnowledgeGraph};
pub use thesaurus::{build_command_thesaurus, get_command_id, normalize_command};

#[cfg(feature = "github-runner")]
pub use coordinator::EvolutionLearningCoordinator;
