use std::time::Duration;

/// Describes error variants for the terraphim_grep crate.
#[derive(Debug, thiserror::Error)]
pub enum TerraphimGrepError {
    /// The underlying search pipeline failed with the given message.
    #[error("search failed: {0}")]
    SearchFailed(String),

    /// No LLM client was configured when synthesis was requested.
    #[error("LLM not configured: {0}")]
    LlmNotConfigured(String),

    /// The search returned fewer results than the required minimum.
    #[error("insufficient results: {0}")]
    InsufficientResults(String),

    /// Knowledge-graph curation via the RLM pipeline failed.
    #[error("KG curation failed: {0}")]
    KgCurationFailed(String),

    /// The RLM execution step failed with the given message.
    #[error("RLM execution failed: {0}")]
    RlmFailed(String),

    /// The operation exceeded the allowed duration.
    #[error("timeout after {0:?}")]
    Timeout(Duration),

    /// A configuration value was missing or invalid.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

/// The standard `Result` type alias for this crate, using [`TerraphimGrepError`].
pub type Result<T> = std::result::Result<T, TerraphimGrepError>;
