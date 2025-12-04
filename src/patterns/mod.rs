//! Pattern matching infrastructure for identifying tools in Bash commands
//!
//! This module provides efficient pattern matching using Aho-Corasick automaton
//! to identify which tools (npm, cargo, git, wrangler, etc.) are being used in
//! Bash command invocations from Claude session logs.
//!
//! ## Architecture
//!
//! - `matcher`: Core pattern matching trait and Aho-Corasick implementation
//! - `loader`: TOML-based pattern configuration loading
//! - `knowledge_graph`: Advanced pattern learning with voting and confidence scoring (includes caching)
//!
//! ## Example
//!
//! ```rust
//! use claude_log_analyzer::patterns::{create_matcher, load_patterns};
//!
//! # fn main() -> anyhow::Result<()> {
//! // Load patterns from built-in TOML
//! let patterns = load_patterns()?;
//!
//! // Create matcher
//! let mut matcher = create_matcher();
//! matcher.initialize(&patterns)?;
//!
//! // Find matches
//! let matches = matcher.find_matches("npx wrangler deploy --env production");
//! for m in matches {
//!     println!("Found tool: {} at position {}", m.tool_name, m.start);
//! }
//! # Ok(())
//! # }
//! ```

pub mod knowledge_graph;
pub mod loader;
pub mod matcher;

// Re-export main types
pub use loader::load_all_patterns;
#[allow(unused_imports)] // Available for user configuration
pub use loader::{load_patterns, load_user_patterns};
#[allow(unused_imports)] // Used in doc examples
pub use loader::{ToolMetadata, ToolPattern};
#[allow(unused_imports)] // Used in doc examples
pub use matcher::{create_matcher, ToolMatch};
pub use matcher::{AhoCorasickMatcher, PatternMatcher};

// Re-export knowledge graph types for Phase 3 - pattern learning and caching
#[allow(unused_imports)] // Public API for pattern learning (Phase 3)
pub use knowledge_graph::{infer_category_from_contexts, LearnedPattern, PatternLearner};

// Re-export terraphim feature types
#[cfg(feature = "terraphim")]
#[allow(unused_imports)] // Public API for terraphim integration (future use)
pub use knowledge_graph::{KnowledgeGraph, RelationType, ToolRelationship};
