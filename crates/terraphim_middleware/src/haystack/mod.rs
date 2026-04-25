//! Haystack indexer implementations for each external service type.

#[cfg(feature = "ai-assistant")]
/// AI-assistant haystack indexer.
pub mod ai_assistant;
/// ClickUp task-management haystack indexer.
pub mod clickup;
#[cfg(feature = "grepapp")]
/// grep.app code search haystack indexer.
pub mod grep_app;
#[cfg(feature = "jmap")]
/// JMAP email haystack indexer.
pub mod jmap;
/// MCP tool haystack indexer.
pub mod mcp;
/// Perplexity AI search haystack indexer.
pub mod perplexity;
/// QueryRs (Rust docs + Reddit) haystack indexer.
pub mod query_rs;
/// Quickwit cloud-native search haystack indexer.
pub mod quickwit;
#[cfg(feature = "ai-assistant")]
pub use ai_assistant::AiAssistantHaystackIndexer;
pub use clickup::ClickUpHaystackIndexer;
#[cfg(feature = "grepapp")]
pub use grep_app::GrepAppHaystackIndexer;
#[cfg(feature = "jmap")]
pub use jmap::JmapHaystackIndexer;
pub use mcp::McpHaystackIndexer;
pub use perplexity::PerplexityHaystackIndexer;
pub use query_rs::QueryRsHaystackIndexer;
pub use quickwit::QuickwitHaystackIndexer;
