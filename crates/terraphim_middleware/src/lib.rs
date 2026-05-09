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

pub mod command;
pub mod haystack;
pub mod indexer;
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

/// Errors that can occur during haystack indexing and search orchestration.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Serde deserialization error: {0}")]
    Json(#[from] json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Indexation error: {0}")]
    Indexation(String),

    #[error("Config error: {0}")]
    Config(#[from] TerraphimConfigError),

    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("Builder error: {0}")]
    Builder(#[from] BuilderError),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Validation error: {0}")]
    Validation(String),
}

/// Convenience alias for `Result<T, Error>` used throughout this crate.
pub type Result<T> = std::result::Result<T, Error>;
