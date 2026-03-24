//! MCP Tool types for indexing and discovery.
//!
//! This module provides types for representing MCP (Model Context Protocol) tools
//! from configured servers, enabling searchable tool discovery via terraphim_automata.

use serde::{Deserialize, Serialize};

/// Represents an indexed MCP tool from configured servers.
///
/// This type is used to store and search available MCP tools, making them
/// discoverable via the terraphim search system.
///
/// # Examples
///
/// ```
/// use terraphim_types::McpToolEntry;
///
/// let tool = McpToolEntry {
///     name: "search_files".to_string(),
///     description: "Search for files matching a pattern".to_string(),
///     server_name: "filesystem".to_string(),
///     input_schema: None,
///     tags: vec!["filesystem".to_string(), "search".to_string()],
///     discovered_at: "2025-01-15T10:30:00Z".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpToolEntry {
    /// The name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// Name of the MCP server that provides this tool
    pub server_name: String,
    /// JSON schema for the tool's input parameters
    pub input_schema: Option<serde_json::Value>,
    /// Tags for categorizing and searching tools
    pub tags: Vec<String>,
    /// ISO 8601 timestamp when the tool was discovered/indexed
    pub discovered_at: String,
}

impl McpToolEntry {
    /// Create a new MCP tool entry
    ///
    /// # Arguments
    ///
    /// * `name` - The tool name
    /// * `description` - Tool description
    /// * `server_name` - Name of the MCP server
    ///
    /// # Examples
    ///
    /// ```
    /// use terraphim_types::McpToolEntry;
    ///
    /// let tool = McpToolEntry::new(
    ///     "search_files",
    ///     "Search for files",
    ///     "filesystem"
    /// );
    /// ```
    pub fn new(name: &str, description: &str, server_name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            server_name: server_name.to_string(),
            input_schema: None,
            tags: Vec::new(),
            discovered_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Add an input schema to the tool
    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Add tags to the tool
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Get a search string for this tool (name + description + tags)
    pub fn search_text(&self) -> String {
        let mut text = format!("{} {}", self.name, self.description);
        if !self.tags.is_empty() {
            text.push(' ');
            text.push_str(&self.tags.join(" "));
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_entry_roundtrip() {
        let tool = McpToolEntry {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            server_name: "test_server".to_string(),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                }
            })),
            tags: vec!["test".to_string(), "search".to_string()],
            discovered_at: "2025-01-15T10:30:00Z".to_string(),
        };

        let json = serde_json::to_string(&tool).expect("Failed to serialize");
        let deserialized: McpToolEntry =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(tool.name, deserialized.name);
        assert_eq!(tool.description, deserialized.description);
        assert_eq!(tool.server_name, deserialized.server_name);
        assert_eq!(tool.tags, deserialized.tags);
    }

    #[test]
    fn test_mcp_tool_entry_new() {
        let tool = McpToolEntry::new("my_tool", "Does something", "my_server");

        assert_eq!(tool.name, "my_tool");
        assert_eq!(tool.description, "Does something");
        assert_eq!(tool.server_name, "my_server");
        assert!(tool.input_schema.is_none());
        assert!(tool.tags.is_empty());
    }

    #[test]
    fn test_mcp_tool_entry_with_schema() {
        let schema = serde_json::json!({ "type": "object" });
        let tool =
            McpToolEntry::new("my_tool", "Does something", "my_server").with_schema(schema.clone());

        assert_eq!(tool.input_schema, Some(schema));
    }

    #[test]
    fn test_mcp_tool_entry_with_tags() {
        let tags = vec!["tag1".to_string(), "tag2".to_string()];
        let tool =
            McpToolEntry::new("my_tool", "Does something", "my_server").with_tags(tags.clone());

        assert_eq!(tool.tags, tags);
    }

    #[test]
    fn test_mcp_tool_entry_search_text() {
        let tool = McpToolEntry::new("search_files", "Search for files", "filesystem")
            .with_tags(vec!["filesystem".to_string(), "search".to_string()]);

        let search_text = tool.search_text();
        assert!(search_text.contains("search_files"));
        assert!(search_text.contains("Search for files"));
        assert!(search_text.contains("filesystem"));
        assert!(search_text.contains("search"));
    }

    #[test]
    fn test_mcp_tool_entry_search_text_without_tags() {
        let tool = McpToolEntry::new("search_files", "Search for files", "filesystem");

        let search_text = tool.search_text();
        assert!(search_text.contains("search_files"));
        assert!(search_text.contains("Search for files"));
    }
}
