//! # Terraphim Agent Evolution System
//!
//! A comprehensive agent memory, task, and learning evolution system that tracks
//! the complete development and learning journey of AI agents over time.
//!
//! ## Core Features
//!
//! - **Versioned Memory**: Time-based snapshots of agent memory states
//! - **Task List Evolution**: Complete lifecycle tracking of agent tasks
//! - **Lessons Learned**: Comprehensive learning and knowledge retention system
//! - **Goal Alignment**: Continuous tracking of agent alignment with objectives
//! - **Evolution Visualization**: Tools to view agent development over time
//!
//! ## Architecture
//!
//! The evolution system consists of three core tracking components that work together:
//!
//! - **Memory Evolution**: Tracks what the agent remembers and knows
//! - **Task List Evolution**: Tracks what the agent needs to do and has done
//! - **Lessons Evolution**: Tracks what the agent has learned and how it applies knowledge
//!
//! All components use terraphim_persistence for storage with time-based versioning.

pub mod error;
pub mod evolution;
pub mod integration;
pub mod lessons;
pub mod llm_adapter;
pub mod memory;
pub mod tasks;
pub mod viewer;
pub mod workflows;

pub use error::*;
pub use evolution::*;
pub use integration::*;
pub use lessons::*;
pub use llm_adapter::*;
pub use memory::*;
pub use tasks::*;
pub use viewer::*;

/// Result type for agent evolution operations
pub type EvolutionResult<T> = Result<T, EvolutionError>;

/// Agent identifier type
pub type AgentId = String;

/// Task identifier type
pub type TaskId = String;

/// Lesson identifier type
pub type LessonId = String;

/// Memory item identifier type
pub type MemoryId = String;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_types() {
        let _agent_id: AgentId = "test_agent".to_string();
        let _task_id: TaskId = "test_task".to_string();
        let _lesson_id: LessonId = "test_lesson".to_string();
        let _memory_id: MemoryId = "test_memory".to_string();
    }
}
