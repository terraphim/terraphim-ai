use std::time::Duration;

/// Errors produced by the terraphim_grep hybrid search and RLM pipeline.
#[derive(Debug, thiserror::Error)]
pub enum TerraphimGrepError {
    /// The underlying search operation failed.
    #[error("search failed: {0}")]
    SearchFailed(String),

    /// An LLM integration was required but not configured.
    #[error("LLM not configured: {0}")]
    LlmNotConfigured(String),

    /// The search returned too few results to be considered useful.
    #[error("insufficient results: {0}")]
    InsufficientResults(String),

    /// Knowledge graph curation via the RLM pipeline failed.
    #[error("KG curation failed: {0}")]
    KgCurationFailed(String),

    /// Execution of the RLM model failed.
    #[error("RLM execution failed: {0}")]
    RlmFailed(String),

    /// The operation exceeded the configured time limit.
    #[error("timeout after {0:?}")]
    Timeout(Duration),

    /// The provided configuration was invalid.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Convenience alias for `Result<T, TerraphimGrepError>`.
pub type Result<T> = std::result::Result<T, TerraphimGrepError>;
