//! Tantivy-based full-text index for session search
//!
//! Provides persistent, high-performance search over imported sessions
//! with custom tokenizers for code and natural language.

#[cfg(feature = "tantivy-index")]
mod schema;

#[cfg(feature = "tantivy-index")]
mod writer;

#[cfg(feature = "tantivy-index")]
mod reader;

#[cfg(feature = "tantivy-index")]
pub use reader::{SessionIndex, SessionSearchResult};

#[cfg(feature = "tantivy-index")]
pub use schema::{SESSION_SCHEMA, build_schema};

#[cfg(feature = "tantivy-index")]
pub use writer::SessionIndexWriter;
