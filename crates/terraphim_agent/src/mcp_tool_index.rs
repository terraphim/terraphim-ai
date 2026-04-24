//! MCP Tool Index for discovering and searching available MCP tools.
//!
//! This module provides an index of MCP (Model Context Protocol) tools from configured
//! servers, enabling fast searchable discovery via terraphim_automata's Aho-Corasick
//! pattern matching.
//!
//! # Examples
//!
//! ```
//! use terraphim_agent::mcp_tool_index::McpToolIndex;
//! use terraphim_types::McpToolEntry;
//! use std::path::PathBuf;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create or load an index
//! let index_path = PathBuf::from("/tmp/mcp-tools.json");
//! let mut index = McpToolIndex::new(index_path);
//!
//! // Add a tool
//! let tool = McpToolEntry::new(
//!     "search_files",
//!     "Search for files matching a pattern",
//!     "filesystem"
//! );
//! index.add_tool(tool);
//!
//! // Search for tools
//! let results = index.search("file");
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use terraphim_automata::find_matches;
use terraphim_types::{McpToolEntry, NormalizedTerm, NormalizedTermValue, Thesaurus};

/// Index of MCP tools for searchable discovery.
///
/// The index stores tools and provides fast search capabilities using
/// terraphim_automata's Aho-Corasick pattern matching against tool names
/// and descriptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolIndex {
    tools: Vec<McpToolEntry>,
    index_path: PathBuf,
}

impl McpToolIndex {
    /// Create a new empty tool index.
    ///
    /// # Arguments
    ///
    /// * `index_path` - Path where the index will be saved/loaded from
    ///
    /// # Examples
    ///
    /// ```
    /// use terraphim_agent::mcp_tool_index::McpToolIndex;
    /// use std::path::PathBuf;
    ///
    /// let index = McpToolIndex::new(PathBuf::from("~/.config/terraphim/mcp-tools.json"));
    /// ```
    pub fn new(index_path: PathBuf) -> Self {
        Self {
            tools: Vec::new(),
            index_path,
        }
    }

    /// Add a tool to the index.
    ///
    /// # Arguments
    ///
    /// * `tool` - The MCP tool entry to add
    ///
    /// # Examples
    ///
    /// ```
    /// use terraphim_agent::mcp_tool_index::McpToolIndex;
    /// use terraphim_types::McpToolEntry;
    /// use std::path::PathBuf;
    ///
    /// let mut index = McpToolIndex::new(PathBuf::from("/tmp/mcp-tools.json"));
    /// let tool = McpToolEntry::new("search_files", "Search for files", "filesystem");
    /// index.add_tool(tool);
    /// ```
    pub fn add_tool(&mut self, tool: McpToolEntry) {
        self.tools.push(tool);
    }

    /// Search for tools matching the query.
    ///
    /// Uses terraphim_automata to build a Thesaurus from tool names and descriptions,
    /// then performs pattern matching against the query.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    ///
    /// # Returns
    ///
    /// A vector of references to matching tool entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use terraphim_agent::mcp_tool_index::McpToolIndex;
    /// use terraphim_types::McpToolEntry;
    /// use std::path::PathBuf;
    ///
    /// let mut index = McpToolIndex::new(PathBuf::from("/tmp/mcp-tools.json"));
    /// index.add_tool(McpToolEntry::new("search_files", "Search for files", "filesystem"));
    /// index.add_tool(McpToolEntry::new("read_file", "Read file contents", "filesystem"));
    ///
    /// let results = index.search("search");
    /// assert_eq!(results.len(), 1);
    /// ```
    pub fn search(&self, query: &str) -> Vec<&McpToolEntry> {
        if self.tools.is_empty() || query.trim().is_empty() {
            return Vec::new();
        }

        // Split query into keywords and build a thesaurus from them
        // Each keyword becomes a pattern that we search for in tool descriptions
        let mut thesaurus = Thesaurus::new("query_terms".to_string());
        let keywords: Vec<&str> = query.split_whitespace().collect();

        for (idx, keyword) in keywords.iter().enumerate() {
            if keyword.len() >= 2 {
                let key = NormalizedTermValue::from(*keyword);
                let term = NormalizedTerm::new(idx as u64, key.clone());
                thesaurus.insert(key, term);
            }
        }

        if thesaurus.is_empty() {
            return Vec::new();
        }

        // Search each tool's text for query matches
        let mut results: Vec<&McpToolEntry> = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for (tool_idx, tool) in self.tools.iter().enumerate() {
            let search_text = tool.search_text();

            // Use terraphim_automata to find query keywords in the tool's search text
            match find_matches(&search_text, thesaurus.clone(), false) {
                Ok(matches) => {
                    if !matches.is_empty() && seen_ids.insert(tool_idx) {
                        results.push(&self.tools[tool_idx]);
                    }
                }
                Err(_) => continue,
            }
        }

        results
    }

    /// Save the index to disk.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an IO error on failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use terraphim_agent::mcp_tool_index::McpToolIndex;
    /// use terraphim_types::McpToolEntry;
    /// use std::path::PathBuf;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut index = McpToolIndex::new(PathBuf::from("/tmp/mcp-tools.json"));
    /// index.add_tool(McpToolEntry::new("search_files", "Search for files", "filesystem"));
    /// index.save()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(parent) = self.index_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&self.index_path, json)?;
        Ok(())
    }

    /// Load an index from disk.
    ///
    /// # Arguments
    ///
    /// * `index_path` - Path to the saved index file
    ///
    /// # Returns
    ///
    /// The loaded `McpToolIndex` on success, or an IO error on failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use terraphim_agent::mcp_tool_index::McpToolIndex;
    /// use std::path::PathBuf;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let index = McpToolIndex::load(PathBuf::from("/tmp/mcp-tools.json"))?;
    /// println!("Loaded {} tools", index.tool_count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load(index_path: PathBuf) -> Result<Self, std::io::Error> {
        let json = std::fs::read_to_string(&index_path)?;
        let index: Self = serde_json::from_str(&json)?;
        Ok(index)
    }

    /// Get the count of tools in the index.
    ///
    /// # Examples
    ///
    /// ```
    /// use terraphim_agent::mcp_tool_index::McpToolIndex;
    /// use terraphim_types::McpToolEntry;
    /// use std::path::PathBuf;
    ///
    /// let mut index = McpToolIndex::new(PathBuf::from("/tmp/mcp-tools.json"));
    /// assert_eq!(index.tool_count(), 0);
    ///
    /// index.add_tool(McpToolEntry::new("search_files", "Search for files", "filesystem"));
    /// assert_eq!(index.tool_count(), 1);
    /// ```
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Get all tools in the index.
    pub fn tools(&self) -> &[McpToolEntry] {
        &self.tools
    }

    /// Get the index path.
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn create_test_tool(name: &str, description: &str, server: &str) -> McpToolEntry {
        McpToolEntry::new(name, description, server)
    }

    #[test]
    fn test_tool_index_add_and_search() {
        let mut index = McpToolIndex::new(PathBuf::from("/tmp/test-mcp-tools.json"));

        let tool1 = create_test_tool(
            "search_files",
            "Search for files matching a pattern",
            "filesystem",
        );
        let tool2 = create_test_tool("read_file", "Read file contents", "filesystem");
        let tool3 = create_test_tool("grep_search", "Search text using grep", "search");

        index.add_tool(tool1);
        index.add_tool(tool2);
        index.add_tool(tool3);

        // Search for "file" should match tool1 and tool2
        let results = index.search("file");
        assert!(!results.is_empty());
        assert!(results.iter().any(|t| t.name == "search_files"));
        assert!(results.iter().any(|t| t.name == "read_file"));
    }

    #[test]
    fn test_tool_index_save_and_load() {
        let temp_dir = std::env::temp_dir();
        let index_path = temp_dir.join("test-mcp-index.json");

        // Create and save
        {
            let mut index = McpToolIndex::new(index_path.clone());
            let tool = create_test_tool("search_files", "Search for files", "filesystem")
                .with_tags(vec!["search".to_string(), "filesystem".to_string()]);
            index.add_tool(tool);
            index.save().expect("Failed to save index");
        }

        // Load and verify
        {
            let index = McpToolIndex::load(index_path.clone()).expect("Failed to load index");
            assert_eq!(index.tool_count(), 1);
            assert_eq!(index.tools[0].name, "search_files");
            assert_eq!(index.tools[0].tags, vec!["search", "filesystem"]);
        }

        // Cleanup
        let _ = std::fs::remove_file(&index_path);
    }

    #[test]
    fn test_tool_index_empty_search() {
        let index = McpToolIndex::new(PathBuf::from("/tmp/test-empty.json"));

        // Empty index should return empty results
        let results = index.search("anything");
        assert!(results.is_empty());
    }

    #[test]
    fn test_tool_index_count() {
        let mut index = McpToolIndex::new(PathBuf::from("/tmp/test-count.json"));
        assert_eq!(index.tool_count(), 0);

        index.add_tool(create_test_tool("tool1", "First tool", "server1"));
        assert_eq!(index.tool_count(), 1);

        index.add_tool(create_test_tool("tool2", "Second tool", "server1"));
        assert_eq!(index.tool_count(), 2);
    }

    #[test]
    fn test_search_partial_match() {
        let mut index = McpToolIndex::new(PathBuf::from("/tmp/test-partial.json"));

        index.add_tool(create_test_tool(
            "search_files",
            "Search for files",
            "filesystem",
        ));
        index.add_tool(create_test_tool(
            "search_code",
            "Search code repositories",
            "code",
        ));
        index.add_tool(create_test_tool(
            "read_file",
            "Read file contents",
            "filesystem",
        ));

        // Search for partial match
        let results = index.search("search");
        assert!(results.iter().any(|t| t.name == "search_files"));
        assert!(results.iter().any(|t| t.name == "search_code"));
        assert!(!results.iter().any(|t| t.name == "read_file"));
    }

    #[test]
    fn test_search_description_match() {
        let mut index = McpToolIndex::new(PathBuf::from("/tmp/test-desc.json"));

        index.add_tool(create_test_tool(
            "tool_a",
            "This tool reads data from files",
            "server",
        ));
        index.add_tool(create_test_tool(
            "tool_b",
            "This tool writes data to database",
            "server",
        ));

        // Search should match description
        let results = index.search("reads");
        assert!(results.iter().any(|t| t.name == "tool_a"));
        assert!(!results.iter().any(|t| t.name == "tool_b"));
    }

    #[test]
    fn test_discovery_latency_benchmark() {
        let mut index = McpToolIndex::new(PathBuf::from("/tmp/test-benchmark.json"));

        // Add 100 tools
        for i in 0..100 {
            let tool = create_test_tool(
                &format!("tool_{}", i),
                &format!("Tool number {} does something useful", i),
                &format!("server_{}", i % 10),
            );
            index.add_tool(tool);
        }

        // Measure search latency for partial name match
        let start = Instant::now();
        let results = index.search("tool_50");
        let elapsed = start.elapsed();

        assert!(!results.is_empty(), "Should find at least one tool");
        assert!(
            elapsed.as_millis() < 150,
            "Search should complete in under 150ms, took {:?}",
            elapsed
        );
    }

    #[test]
    fn test_search_with_tags() {
        let mut index = McpToolIndex::new(PathBuf::from("/tmp/test-tags.json"));

        let tool1 = create_test_tool("search_files", "Search for files", "filesystem")
            .with_tags(vec!["search".to_string(), "files".to_string()]);
        let tool2 = create_test_tool("grep_search", "Search with grep", "search")
            .with_tags(vec!["search".to_string(), "text".to_string()]);

        index.add_tool(tool1);
        index.add_tool(tool2);

        // Search by tag
        let results = index.search("text");
        assert!(results.iter().any(|t| t.name == "grep_search"));
    }

    #[test]
    fn test_empty_query_returns_empty() {
        let mut index = McpToolIndex::new(PathBuf::from("/tmp/test-empty-query.json"));
        index.add_tool(create_test_tool("tool1", "Description", "server"));

        let results = index.search("");
        assert!(results.is_empty());
    }

    #[test]
    fn test_new_creates_empty_index() {
        let index = McpToolIndex::new(PathBuf::from("/tmp/test-new.json"));
        assert_eq!(index.tool_count(), 0);
        assert!(index.tools().is_empty());
    }
}
