//! Error types for agent evolution system

use thiserror::Error;

/// Errors that can occur in the agent evolution system
#[derive(Error, Debug)]
pub enum EvolutionError {
    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Lesson not found: {0}")]
    LessonNotFound(String),

    #[error("Memory item not found: {0}")]
    MemoryNotFound(String),

    #[error("Version not found for timestamp: {0}")]
    VersionNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Evolution snapshot error: {0}")]
    SnapshotError(String),

    #[error("Goal alignment calculation error: {0}")]
    AlignmentError(String),

    #[error("LLM operation error: {0}")]
    LlmError(String),

    #[error("Workflow execution error: {0}")]
    WorkflowError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
