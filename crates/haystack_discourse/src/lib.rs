//! Discourse forum haystack integration for Terraphim AI.
//!
//! Implements [`haystack_core::HaystackProvider`] for Discourse forums.
//! Exposes [`DiscourseClient`] for searching forum posts and topics via
//! the Discourse JSON API.
mod client;
mod models;

pub use client::DiscourseClient;
pub use models::Post;
