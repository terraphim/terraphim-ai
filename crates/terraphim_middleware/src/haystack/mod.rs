#[cfg(feature = "ai-assistant")]
pub mod ai_assistant;
/// ClickUp task-management haystack indexer.
pub mod clickup;
#[cfg(feature = "grepapp")]
pub mod grep_app;
#[cfg(feature = "jmap")]
pub mod jmap;
/// MCP (Model Context Protocol) haystack indexer.
pub mod mcp;
/// Perplexity AI search haystack indexer.
pub mod perplexity;
/// QueryRs (Rust docs + Reddit) haystack indexer.
pub mod query_rs;
/// Quickwit cloud-native search engine haystack indexer.
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
