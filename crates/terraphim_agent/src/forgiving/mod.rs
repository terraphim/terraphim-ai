//! Forgiving CLI Parser
//!
//! Provides typo-tolerant command parsing for AI agents and human users.
//! Uses edit distance algorithms to auto-correct common typos and suggest
//! alternatives for unknown commands.

pub mod aliases;
pub mod parser;
pub mod suggestions;

pub use aliases::{AliasRegistry, DEFAULT_ALIASES};
pub use parser::{ForgivingParser, ParseResult};
pub use suggestions::CommandSuggestion;
