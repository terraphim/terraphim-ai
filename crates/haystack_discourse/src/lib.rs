//! Haystack integration for Discourse forums.
//!
//! Implements [`haystack_core::HaystackProvider`] over the Discourse search
//! API, allowing forum topics and posts to be indexed as Terraphim documents.
mod client;
mod models;

/// HTTP client for querying the Discourse search API and converting topics to [`terraphim_types::Document`]s.
pub use client::DiscourseClient;
/// A Discourse post as returned by the search API.
pub use models::Post;
