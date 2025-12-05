//! Knowledge Graph module for terraphim-powered search
//!
//! This module provides advanced search capabilities using terraphim_automata
//! for concept-based matching and query evaluation.
//!
//! ## Architecture
//!
//! - `builder`: Constructs knowledge graphs from tool invocations
//! - `search`: Evaluates queries against the knowledge graph
//! - `query`: Query AST for complex concept searches
//!
//! ## Example
//!
//! ```rust,ignore
//! use claude_log_analyzer::kg::{KnowledgeGraphBuilder, KnowledgeGraphSearch};
//! use claude_log_analyzer::kg::QueryNode;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Build knowledge graph from tool invocations
//! let builder = KnowledgeGraphBuilder::from_tool_invocations(&tools);
//!
//! // Search using concept queries
//! let search = KnowledgeGraphSearch::new(builder);
//! let query = QueryNode::And(
//!     Box::new(QueryNode::Concept("BUN".to_string())),
//!     Box::new(QueryNode::Concept("install".to_string()))
//! );
//!
//! let results = search.search("bunx install packages", &query);
//! # Ok(())
//! # }
//! ```

pub mod builder;
pub mod query;
pub mod search;

pub use builder::KnowledgeGraphBuilder;
pub use query::{QueryNode, QueryParser};
pub use search::{KnowledgeGraphSearch, SearchResult};
