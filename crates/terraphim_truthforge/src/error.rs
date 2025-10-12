use thiserror::Error;

#[derive(Error, Debug)]
pub enum TruthForgeError {
    #[error("Narrative too long: {length} characters (max: {max_length})")]
    NarrativeTooLong { length: usize, max_length: usize },

    #[error("Invalid narrative context: {0}")]
    InvalidContext(String),

    #[error("Agent execution failed: {agent_name} - {reason}")]
    AgentExecutionFailed { agent_name: String, reason: String },

    #[error("Workflow execution failed: {phase} - {reason}")]
    WorkflowExecutionFailed { phase: String, reason: String },

    #[error("Omission detection failed: {0}")]
    OmissionDetectionFailed(String),

    #[error("Debate evaluation failed: {0}")]
    DebateEvaluationFailed(String),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Terraphim multi-agent error: {0}")]
    TerraphimError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, TruthForgeError>;
