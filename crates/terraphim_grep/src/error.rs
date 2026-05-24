use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum TerraphimGrepError {
    #[error("search failed: {0}")]
    SearchFailed(String),

    #[error("LLM not configured: {0}")]
    LlmNotConfigured(String),

    #[error("insufficient results: {0}")]
    InsufficientResults(String),

    #[error("KG curation failed: {0}")]
    KgCurationFailed(String),

    #[error("RLM execution failed: {0}")]
    RlmFailed(String),

    #[error("timeout after {0:?}")]
    Timeout(Duration),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

pub type Result<T> = std::result::Result<T, TerraphimGrepError>;
