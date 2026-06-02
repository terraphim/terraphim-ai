use std::time::Duration;

/// Errors that can occur during a terraphim-grep search or synthesis operation.
#[derive(Debug, thiserror::Error)]
pub enum TerraphimGrepError {
    /// The underlying retrieval pipeline returned an error.
    #[error("search failed: {0}")]
    SearchFailed(String),

    /// An LLM client was required but not wired up.
    #[error("LLM not configured: {0}")]
    LlmNotConfigured(String),

    /// The search returned too few results to satisfy the query.
    #[error("insufficient results: {0}")]
    InsufficientResults(String),

    /// Concept extraction or KG persistence failed.
    #[error("KG curation failed: {0}")]
    KgCurationFailed(String),

    /// The RLM synthesis step failed (e.g. the LLM returned unparseable output).
    #[error("RLM execution failed: {0}")]
    RlmFailed(String),

    /// A search or synthesis step exceeded its time budget.
    #[error("timeout after {0:?}")]
    Timeout(Duration),

    /// The provided configuration is invalid.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Convenience alias for `Result<T, TerraphimGrepError>`.
pub type Result<T> = std::result::Result<T, TerraphimGrepError>;
