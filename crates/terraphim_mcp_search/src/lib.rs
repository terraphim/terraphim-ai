//! # terraphim_mcp_search
//!
//! MCP (Model Context Protocol) tool indexing and search.
//!
//! Provides [`McpToolIndex`] for fast discovery of MCP tools across configured
//! servers, using `terraphim_automata`'s Aho-Corasick pattern matching.
//!
//! ## What this crate gives you
//!
//! | Item | When to use |
//! |------|-------------|
//! | [`McpToolIndex`] | You want a persistent, mutable, large-corpus tool index with save/load. |
//! | [`mcp_search_tools`] | You have a slice of tools and want a one-shot search without managing state. |
//! | [`SkillEntry`] + [`mcp_search_skills`] | You want to search Terraphim skills (TinyClaw JSON, etc.) with the same engine. |
//!
//! ## Origin and migration plan
//!
//! This crate was extracted from `terraphim_agent::mcp_tool_index` (in the
//! sibling `terraphim-agents` polyrepo, version 1.20.x) on 2026-06-28. The
//! source is verbatim so that:
//!
//! - The `McpToolIndex` API is identical to the legacy one.
//! - Search semantics (per-keyword Aho-Corasick matching over
//!   `name + description + tags`) match the existing tests.
//! - Performance NFR is preserved: search completes in **< 70 ms for 100 tools**.
//!
//! Once `terraphim_agent` switches to depending on this crate (tracked in
//! Gitea issue [terraphim-agents#64](https://git.terraphim.cloud/terraphim/terraphim-agents/issues/64)),
//! `terraphim_agent::mcp_tool_index` will become a deprecated re-export shim
//! and eventually be removed.
//!
//! ## Example
//!
//! ```
//! use terraphim_mcp_search::{McpToolIndex, mcp_search_tools, mcp_search_skills, SkillEntry};
//! use terraphim_types::McpToolEntry;
//! use std::path::PathBuf;
//!
//! let mut index = McpToolIndex::new(PathBuf::from("/tmp/mcp-tools.json"));
//! index.add_tool(McpToolEntry::new("search_files", "Search files", "filesystem"));
//! index.add_tool(McpToolEntry::new("read_file",    "Read file contents", "filesystem"));
//!
//! let results = index.search("search");
//! assert_eq!(results.len(), 1);
//! assert_eq!(results[0].name, "search_files");
//!
//! // One-shot search helper
//! let hits = mcp_search_tools("search", &index.tools().iter().cloned().collect::<Vec<_>>());
//! assert_eq!(hits.len(), 1);
//!
//! // Skill search (separate corpus, same engine)
//! let skills = vec![
//!     SkillEntry::new("code-review", "Automated code review").with_tags(vec!["review".into()]),
//! ];
//! let skill_hits = mcp_search_skills("review", &skills);
//! assert_eq!(skill_hits.len(), 1);
//! ```

pub mod mcp_tool_index;
pub mod search;

pub use mcp_tool_index::McpToolIndex;
pub use search::{SkillEntry, mcp_search_skills, mcp_search_tools};
