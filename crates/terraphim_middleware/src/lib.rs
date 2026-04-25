//! Middleware layer for Terraphim haystack indexing and search orchestration.
//!
//! Provides [`IndexMiddleware`] implementations for each haystack service type
//! (Ripgrep, QueryRs, MCP, ClickUp, Quickwit, etc.) and the [`search_haystacks`]
//! orchestrator that fans out queries across all configured haystacks.

use serde_json as json;
use terraphim_automata::builder::BuilderError;
use terraphim_config::TerraphimConfigError;

/// Shell command wrappers (ripgrep, etc.) used by haystack indexers.
pub mod command;
/// Haystack indexer implementations for each service type.
pub mod haystack;
/// Core indexer trait and multi-haystack search orchestration.
pub mod indexer;
/// Thesaurus building from haystack content.
pub mod thesaurus;

#[cfg(feature = "kg-integration")]
pub mod learning_indexer;

#[cfg(feature = "kg-integration")]
pub mod learning_query;

#[cfg(feature = "feedback-loop")]
pub mod feedback_loop;

pub use haystack::QueryRsHaystackIndexer;
pub use indexer::{search_haystacks, RipgrepIndexer};

// #[cfg(test)]
// mod tests; // Removed - no tests module

/// Errors returned by middleware operations.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// JSON deserialisation failure.
    #[error("Serde deserialization error: {0}")]
    Json(#[from] json::Error),

    /// I/O error from the filesystem or a child process.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// The requested role is not present in the configuration.
    #[error("Role not found: {0}")]
    RoleNotFound(String),

    /// An indexation operation failed.
    #[error("Indexation error: {0}")]
    Indexation(String),

    /// A configuration error propagated from `terraphim_config`.
    #[error("Config error: {0}")]
    Config(#[from] TerraphimConfigError),

    /// A persistence error propagated from `terraphim_persistence`.
    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    /// An automata builder error.
    #[error("Builder error: {0}")]
    Builder(#[from] BuilderError),

    /// An HTTP request to an external API failed.
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    /// Input validation error.
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Type alias for middleware operation results.
pub type Result<T> = std::result::Result<T, Error>;
