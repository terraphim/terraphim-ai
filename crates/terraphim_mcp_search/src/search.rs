//! Generic search helpers for MCP tools and skill-like entries.
//!
//! These functions provide a stateless, allocation-light search interface that
//! builds an ephemeral [`McpToolIndex`] internally. Use them when you have a
//! small/medium list to search once or a few times. For persistent indexes
//! with thousands of tools, construct [`McpToolIndex`] directly.
//!
//! # Performance
//!
//! Both functions share the underlying Aho-Corasick matching from
//! `terraphim_automata`. The benchmark NFR of **< 70 ms for 100 entries**
//! applies (see `McpToolIndex::test_discovery_latency_benchmark`).
//!
//! # Examples
//!
//! ```
//! use terraphim_mcp_search::{mcp_search_tools, mcp_search_skills, SkillEntry};
//! use terraphim_types::McpToolEntry;
//!
//! let tools = vec![
//!     McpToolEntry::new("search_files", "Search for files", "filesystem"),
//!     McpToolEntry::new("read_file",    "Read file contents", "filesystem"),
//! ];
//! let hits = mcp_search_tools("search", &tools);
//! assert_eq!(hits.len(), 1);
//!
//! let skills = vec![
//!     SkillEntry::new("code-review", "Automated code review"),
//!     SkillEntry::new("deploy",      "Deploy to staging"),
//! ];
//! let hits = mcp_search_skills("review", &skills);
//! assert_eq!(hits[0].name, "code-review");
//! ```

use serde::{Deserialize, Serialize};

use crate::mcp_tool_index::McpToolIndex;
use terraphim_types::McpToolEntry;

/// Search a list of MCP tool entries for those matching the query.
///
/// This is a thin convenience wrapper that builds an ephemeral
/// [`McpToolIndex`], runs the search, and returns owned (cloned) entries.
/// Equivalent to building an index by hand for small/medium lists.
///
/// # Arguments
///
/// * `query` — free-text search query (whitespace-split into keywords,
///   keywords of length < 2 are ignored, matching `McpToolIndex::search`).
/// * `tools` — slice of tool entries to search across.
///
/// # Returns
///
/// A `Vec<McpToolEntry>` (owned, not borrowed) preserving input order.
/// Empty if the query is empty or nothing matches.
///
/// # Performance
///
/// Same NFR as `McpToolIndex::search`: **< 70 ms for 100 tools**.
///
/// # Examples
///
/// ```
/// use terraphim_mcp_search::mcp_search_tools;
/// use terraphim_types::McpToolEntry;
///
/// let tools = vec![
///     McpToolEntry::new("search_files", "Search for files", "filesystem"),
///     McpToolEntry::new("read_file",    "Read file contents", "filesystem"),
/// ];
/// let hits = mcp_search_tools("search", &tools);
/// assert_eq!(hits.len(), 1);
/// assert_eq!(hits[0].name, "search_files");
/// ```
pub fn mcp_search_tools(query: &str, tools: &[McpToolEntry]) -> Vec<McpToolEntry> {
    // Build an ephemeral index in /tmp; the path is never read because we
    // never call `.save()`. Using a deterministic name keeps parallel test
    // runs from colliding if any test ever decides to persist.
    let mut index = McpToolIndex::new(std::path::PathBuf::from(
        "/tmp/terraphim-mcp-search-ephemeral.json",
    ));
    for tool in tools {
        index.add_tool(tool.clone());
    }
    index.search(query).into_iter().cloned().collect()
}

/// A search-indexable entry for a Terraphim skill.
///
/// `SkillEntry` is the skill-side analogue of `McpToolEntry`: it exposes
/// the same `name + description + tags` search surface so a single
/// [`McpToolIndex`] can search across both tools and skills.
///
/// This is intentionally **not** a copy of `terraphim_tinyclaw::skills::Skill`
/// (which is workflow-shaped with `inputs` and `steps`). `SkillEntry` is a
/// discovery projection: just what a search index needs to know to rank the
/// skill for a query.
///
/// # Constructing from TinyClaw skill JSON
///
/// ```no_run
/// use terraphim_mcp_search::SkillEntry;
/// use terraphim_types::McpToolEntry;
///
/// let skill_json = r#"{
///   "name": "code-review",
///   "version": "1.0.0",
///   "description": "Automated code review",
///   "author": "Terraphim",
///   "inputs": [],
///   "steps": []
/// }"#;
///
/// let entry: SkillEntry = serde_json::from_str(skill_json).unwrap();
/// assert_eq!(entry.name, "code-review");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillEntry {
    /// Skill name (unique identifier).
    pub name: String,
    /// Human-readable description (one or two sentences).
    pub description: String,
    /// Semantic version string.
    #[serde(default = "default_skill_version")]
    pub version: String,
    /// Optional author attribution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Tags for categorising and searching skills.
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_skill_version() -> String {
    "0.0.0".to_string()
}

impl SkillEntry {
    /// Construct a minimal skill entry from name and description.
    ///
    /// # Examples
    ///
    /// ```
    /// use terraphim_mcp_search::SkillEntry;
    /// let entry = SkillEntry::new("code-review", "Automated code review");
    /// assert_eq!(entry.name, "code-review");
    /// assert_eq!(entry.version, "0.0.0");
    /// ```
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            version: default_skill_version(),
            author: None,
            tags: Vec::new(),
        }
    }

    /// Attach tags (builder pattern).
    ///
    /// # Examples
    ///
    /// ```
    /// use terraphim_mcp_search::SkillEntry;
    /// let entry = SkillEntry::new("code-review", "Automated code review")
    ///     .with_tags(vec!["review".to_string(), "quality".to_string()]);
    /// assert_eq!(entry.tags.len(), 2);
    /// ```
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Build the search text fed into the Aho-Corasick index.
    ///
    /// Mirrors `McpToolEntry::search_text` exactly so both `McpToolIndex` and
    /// the convenience helpers see the same ranking surface.
    ///
    /// Exposed as `pub` for downstream callers that want to feed the
    /// `terraphim_automata::find_matches` engine directly (e.g. for ranking
    /// beyond the simple membership check that `mcp_search_skills` performs).
    pub fn search_text(&self) -> String {
        let mut text = format!("{} {}", self.name, self.description);
        if !self.tags.is_empty() {
            text.push(' ');
            text.push_str(&self.tags.join(" "));
        }
        text
    }
}

/// Search a list of skill entries for those matching the query.
///
/// Same semantics as [`mcp_search_tools`] but operates on [`SkillEntry`].
///
/// # Arguments
///
/// * `query` — free-text search query (whitespace-split into keywords,
///   keywords of length < 2 are ignored, matching `McpToolIndex::search`).
/// * `skills` — slice of skill entries to search across.
///
/// # Returns
///
/// A `Vec<SkillEntry>` (owned, not borrowed) preserving input order.
/// Empty if the query is empty or nothing matches.
///
/// # Examples
///
/// ```
/// use terraphim_mcp_search::{mcp_search_skills, SkillEntry};
///
/// let skills = vec![
///     SkillEntry::new("code-review", "Automated code review"),
///     SkillEntry::new("deploy",      "Deploy to staging"),
/// ];
/// let hits = mcp_search_skills("review", &skills);
/// assert_eq!(hits.len(), 1);
/// assert_eq!(hits[0].name, "code-review");
/// ```
pub fn mcp_search_skills(query: &str, skills: &[SkillEntry]) -> Vec<SkillEntry> {
    // Convert SkillEntry list to a list of "fake" McpToolEntry values so we
    // can reuse the existing `McpToolIndex` engine. The server_name field
    // is set to "skill" so downstream callers can distinguish hits.
    let proxy: Vec<McpToolEntry> = skills
        .iter()
        .map(|s| McpToolEntry::new(&s.name, &s.description, "skill").with_tags(s.tags.clone()))
        .collect();

    // Build the index once over all skills (O(N) construction, not O(N²)).
    let mut index = McpToolIndex::new(std::path::PathBuf::from(
        "/tmp/terraphim-mcp-search-ephemeral.json",
    ));
    for tool in &proxy {
        index.add_tool(tool.clone());
    }
    let hits = index.search(query);

    // Map the hits (Vec<&McpToolEntry>) back to input positions, then to
    // owned SkillEntry. Preserve input order.
    let hit_names: std::collections::HashSet<&str> = hits.iter().map(|t| t.name.as_str()).collect();
    skills
        .iter()
        .filter(|s| hit_names.contains(s.name.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_entry_new() {
        let entry = SkillEntry::new("test", "Test skill");
        assert_eq!(entry.name, "test");
        assert_eq!(entry.description, "Test skill");
        assert_eq!(entry.version, "0.0.0");
        assert!(entry.author.is_none());
        assert!(entry.tags.is_empty());
    }

    #[test]
    fn test_skill_entry_with_tags() {
        let entry =
            SkillEntry::new("test", "Test skill").with_tags(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(entry.tags, vec!["a", "b"]);
    }

    #[test]
    fn test_skill_entry_search_text() {
        let entry = SkillEntry::new("code-review", "Automated review")
            .with_tags(vec!["review".to_string(), "quality".to_string()]);
        let text = entry.search_text();
        assert!(text.contains("code-review"));
        assert!(text.contains("Automated review"));
        assert!(text.contains("review"));
        assert!(text.contains("quality"));
    }

    #[test]
    fn test_skill_entry_serde_roundtrip() {
        let json = r#"{
            "name": "code-review",
            "version": "1.2.3",
            "description": "Automated code review",
            "author": "Terraphim",
            "inputs": [],
            "steps": []
        }"#;
        let entry: SkillEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.name, "code-review");
        assert_eq!(entry.version, "1.2.3");
        assert_eq!(entry.description, "Automated code review");
        assert_eq!(entry.author.as_deref(), Some("Terraphim"));

        let serialised = serde_json::to_string(&entry).unwrap();
        let roundtrip: SkillEntry = serde_json::from_str(&serialised).unwrap();
        assert_eq!(entry, roundtrip);
    }

    #[test]
    fn test_skill_entry_serde_minimal() {
        // No version, no author, no tags - all default.
        let json = r#"{"name": "x", "description": "y"}"#;
        let entry: SkillEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.name, "x");
        assert_eq!(entry.version, "0.0.0");
        assert!(entry.author.is_none());
        assert!(entry.tags.is_empty());
    }

    #[test]
    fn test_skill_entry_serde_skips_none() {
        let entry = SkillEntry::new("x", "y");
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            !json.contains("author"),
            "author should be skipped when None"
        );
        // tags serializes as [] when empty (Vec<String> has no
        // skip_serializing_if); the property under test is "no None fields",
        // not "minimal JSON". The minimal-JSON contract is enforced by the
        // TinyClaw-side adapter, not here.
        assert!(json.contains("\"tags\":[]"));
    }

    #[test]
    fn test_mcp_search_tools_empty_query() {
        let tools = vec![McpToolEntry::new("a", "b", "c")];
        assert!(mcp_search_tools("", &tools).is_empty());
        assert!(mcp_search_tools("   ", &tools).is_empty());
    }

    #[test]
    fn test_mcp_search_tools_empty_list() {
        let tools: Vec<McpToolEntry> = vec![];
        assert!(mcp_search_tools("anything", &tools).is_empty());
    }

    #[test]
    fn test_mcp_search_tools_basic_match() {
        let tools = vec![
            McpToolEntry::new("search_files", "Search for files", "filesystem"),
            McpToolEntry::new("read_file", "Read file contents", "filesystem"),
        ];
        let hits = mcp_search_tools("search", &tools);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "search_files");
    }

    #[test]
    fn test_mcp_search_tools_returns_owned() {
        let tools = vec![McpToolEntry::new("a", "alpha", "server")];
        let hits = mcp_search_tools("alpha", &tools);
        // hits is Vec<McpToolEntry> (owned), not Vec<&McpToolEntry>
        let _: McpToolEntry = hits.into_iter().next().unwrap();
    }

    #[test]
    fn test_mcp_search_skills_basic_match() {
        let skills = vec![
            SkillEntry::new("code-review", "Automated code review"),
            SkillEntry::new("deploy", "Deploy to staging"),
        ];
        let hits = mcp_search_skills("review", &skills);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "code-review");
    }

    #[test]
    fn test_mcp_search_skills_tag_match() {
        let skills = vec![
            SkillEntry::new("code-review", "review files").with_tags(vec!["quality".to_string()]),
            SkillEntry::new("deploy", "deploy things"),
        ];
        let hits = mcp_search_skills("quality", &skills);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "code-review");
    }

    #[test]
    fn test_mcp_search_skills_preserves_order() {
        let skills = vec![
            SkillEntry::new("a", "alpha beta"),
            SkillEntry::new("b", "beta gamma"),
            SkillEntry::new("c", "gamma alpha"),
        ];
        let hits = mcp_search_skills("beta", &skills);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].name, "a");
        assert_eq!(hits[1].name, "b");
    }

    #[test]
    fn test_mcp_search_skills_empty_inputs() {
        let skills: Vec<SkillEntry> = vec![];
        assert!(mcp_search_skills("anything", &skills).is_empty());
    }

    #[test]
    fn test_mcp_search_skills_empty_query() {
        let skills = vec![SkillEntry::new("a", "b")];
        assert!(mcp_search_skills("", &skills).is_empty());
    }

    #[test]
    fn test_mcp_search_tools_skills_share_engine() {
        // Tools and skills should both match via the same query style.
        let tools = vec![McpToolEntry::new("search_files", "Search for files", "fs")];
        let skills = vec![SkillEntry::new("analyze", "Analyze data")];

        let tool_hits = mcp_search_tools("search", &tools);
        let skill_hits = mcp_search_skills("analyze", &skills);

        assert_eq!(tool_hits.len(), 1);
        assert_eq!(skill_hits.len(), 1);
    }
}
