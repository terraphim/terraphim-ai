//! Haystack indexing and search orchestration for Terraphim AI.
//!
//! This crate sits between the raw haystack integrations (Ripgrep, AtomicServer,
//! ClickUp, Discourse, JMAP …) and the service layer. It owns:
//!
//! - **Indexer abstraction** -- [`IndexMiddleware`](indexer::IndexMiddleware) trait
//!   and [`search_haystacks`] for parallel haystack queries
//! - **Thesaurus builders** -- converting source documents into automata-compatible JSON
//! - **Command execution** -- sandboxed invocation of external tools
//!
//! Each haystack service is registered via [`terraphim_config::ServiceType`] and
//! dispatched by [`indexer::search_haystacks`] at query time.

use serde_json as json;
use terraphim_automata::builder::BuilderError;
use terraphim_config::TerraphimConfigError;

/// Sandboxed external-tool execution (ripgrep, fff, shell commands).
pub mod command;
/// Haystack integrations: ClickUp, QueryRs, MCP, Atlassian, Discourse, JMAP, Quickwit.
pub mod haystack;
/// Indexer trait and parallel haystack dispatch.
pub mod indexer;
/// Thesaurus building from source documents and URLs.
pub mod thesaurus;

#[cfg(feature = "kg-integration")]
pub mod learning_indexer;

#[cfg(feature = "kg-integration")]
pub mod learning_query;

#[cfg(feature = "feedback-loop")]
pub mod feedback_loop;

pub use haystack::QueryRsHaystackIndexer;
pub use indexer::{search_haystacks, FffIndexer, RipgrepIndexer};

// #[cfg(test)]
// mod tests; // Removed - no tests module

/// Errors produced by the middleware layer during indexing and search orchestration.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// JSON deserialisation of haystack output failed.
    #[error("Serde deserialization error: {0}")]
    Json(#[from] json::Error),

    /// An I/O operation failed (file read, process pipe, …).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// The requested role does not exist in the current configuration.
    #[error("Role not found: {0}")]
    RoleNotFound(String),

    /// The indexation pipeline encountered an error.
    #[error("Indexation error: {0}")]
    Indexation(String),

    /// A configuration layer operation failed.
    #[error("Config error: {0}")]
    Config(#[from] TerraphimConfigError),

    /// A persistence layer operation failed.
    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    /// Building the automata from the thesaurus failed.
    #[error("Builder error: {0}")]
    Builder(#[from] BuilderError),

    /// An HTTP request to an external haystack service failed.
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    /// Input failed a domain-level validation check.
    #[error("Validation error: {0}")]
    Validation(String),

    /// The file-search subsystem returned an error.
    #[error("File search error: {0}")]
    FileSearch(String),
}

/// Convenience alias for `Result<T, Error>` used throughout this crate.
pub type Result<T> = std::result::Result<T, Error>;
