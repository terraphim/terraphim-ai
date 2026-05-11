//! Haystack integration for Discourse forums.
//!
//! Implements [`haystack_core::HaystackProvider`] over the Discourse search
//! API, allowing forum topics and posts to be indexed as Terraphim documents.
mod client;
mod models;

pub use client::DiscourseClient;
pub use models::Post;
