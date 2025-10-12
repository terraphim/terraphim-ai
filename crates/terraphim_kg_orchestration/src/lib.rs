//! # Terraphim Knowledge Graph Orchestration Engine
//!
//! A knowledge graph-based agent orchestration system that coordinates multi-agent workflows
//! using intelligent task decomposition, agent matching, and execution planning.
//!
//! ## Core Features
//!
//! - **Simple Agent Model**: Trait-based agents with clear capabilities
//! - **Task Decomposition Integration**: Uses terraphim_task_decomposition for intelligent task breakdown
//! - **Knowledge Graph Coordination**: Leverages knowledge graphs for agent-task matching
//! - **Execution Management**: Handles dependencies, parallel execution, and result aggregation
//! - **Fault Tolerance**: Basic error handling and recovery mechanisms
//!
//! ## Architecture
//!
//! The orchestration engine consists of several key components:
//!
//! - **Agent Pool**: Registry of available agents with their capabilities
//! - **Task Scheduler**: Decomposes complex tasks and schedules execution
//! - **Execution Coordinator**: Manages task execution, dependencies, and parallelism
//! - **Result Aggregator**: Combines results from multiple agents into coherent outputs

pub mod agent;
pub mod coordinator;
pub mod error;
pub mod pool;
pub mod scheduler;
pub mod supervision;

pub use agent::*;
pub use coordinator::*;
pub use error::*;
pub use pool::*;
pub use scheduler::*;
pub use supervision::*;

// Re-export key types from task decomposition
pub use terraphim_task_decomposition::{
    ExecutionPlan, Task, TaskComplexity, TaskDecompositionSystem, TaskDecompositionWorkflow,
    TaskId, TaskStatus, TerraphimTaskDecompositionSystem,
};

/// Result type for orchestration operations
pub type OrchestrationResult<T> = Result<T, OrchestrationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_imports() {
        // Test that all modules compile and basic types are available
        let _task_id: TaskId = "test_task".to_string();
        let _status = TaskStatus::Pending;
    }
}
