//! Error types for the multi-agent system

use thiserror::Error;

/// Errors that can occur in the multi-agent system
#[derive(Debug, Error)]
pub enum MultiAgentError {
    /// Agent not found
    #[error("Agent with ID {0} not found")]
    AgentNotFound(crate::AgentId),

    /// Agent already exists
    #[error("Agent with ID {0} already exists")]
    AgentAlreadyExists(crate::AgentId),

    /// Agent not available
    #[error("Agent {0} is not available")]
    AgentNotAvailable(crate::AgentId),

    /// Invalid role configuration
    #[error("Invalid role configuration: {0}")]
    InvalidRoleConfig(String),

    /// LLM interaction error
    #[error("LLM error: {0}")]
    LlmError(String),

    /// Rig framework error
    #[error("Rig framework error: {0}")]
    RigError(String),

    /// Persistence error
    #[error("Persistence error: {0}")]
    PersistenceError(String),

    /// Knowledge graph error
    #[error("Knowledge graph error: {0}")]
    KnowledgeGraphError(String),

    /// Agent evolution error
    #[error("Agent evolution error: {0}")]
    EvolutionError(String),

    /// Context management error
    #[error("Context error: {0}")]
    ContextError(String),

    /// Task routing error
    #[error("Task routing error: {0}")]
    TaskRoutingError(String),

    /// Communication error between agents
    #[error("Agent communication error: {0}")]
    CommunicationError(String),

    /// Workflow execution error
    #[error("Workflow execution error: {0}")]
    WorkflowError(String),

    /// Token limit exceeded
    #[error("Token limit exceeded: {current}/{limit}")]
    TokenLimitExceeded { current: u64, limit: u64 },

    /// Budget limit exceeded
    #[error("Budget limit exceeded: ${current:.2}/${limit:.2}")]
    BudgetLimitExceeded { current: f64, limit: f64 },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {requests} requests in {window_seconds}s")]
    RateLimitExceeded { requests: u64, window_seconds: u64 },

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// System error
    #[error("System error: {0}")]
    SystemError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Task cancellation
    #[error("Task was cancelled")]
    TaskCancelled,

    /// Timeout error
    #[error("Operation timed out after {seconds}s")]
    Timeout { seconds: u64 },

    /// Session not found
    #[error("Session with ID {0} not found")]
    SessionNotFound(uuid::Uuid),

    /// Agent pool exhausted
    #[error("Agent pool is exhausted - no available agents")]
    PoolExhausted,

    /// Agent busy
    #[error("Agent {0} is currently busy")]
    AgentBusy(crate::AgentId),

    /// Agent creation timeout
    #[error("Agent creation timed out")]
    AgentCreationTimeout,

    /// Agent creation failed
    #[error("Agent creation failed: {0}")]
    AgentCreationFailed(String),

    /// Pool error
    #[error("Pool error: {0}")]
    PoolError(String),
}

impl From<anyhow::Error> for MultiAgentError {
    fn from(err: anyhow::Error) -> Self {
        MultiAgentError::SystemError(err.to_string())
    }
}
