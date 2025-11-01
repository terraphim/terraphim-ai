use crate::{McpProxyError, Result};

pub const TOOL_PREFIX_SEPARATOR: &str = "__";

pub fn parse_tool_name(prefixed_name: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = prefixed_name.splitn(2, TOOL_PREFIX_SEPARATOR).collect();

    if parts.len() != 2 {
        return Err(McpProxyError::InvalidToolName(format!(
            "Tool name '{}' does not contain server prefix. Expected format: 'ServerName__toolName'",
            prefixed_name
        )));
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

pub fn create_prefixed_name(server_name: &str, tool_name: &str) -> String {
    format!("{}{}{}", server_name, TOOL_PREFIX_SEPARATOR, tool_name)
}

pub fn is_prefixed(tool_name: &str) -> bool {
    tool_name.contains(TOOL_PREFIX_SEPARATOR)
}

#[derive(Debug, Clone, Default)]
pub struct ToolRouter {
    /// Map of prefixed tool names to server names
    tool_to_server: std::collections::HashMap<String, String>,
}

impl ToolRouter {
    /// Create a new tool router
    pub fn new() -> Self {
        Self {
            tool_to_server: std::collections::HashMap::new(),
        }
    }

    /// Register a tool with its server
    pub fn register_tool(&mut self, server_name: impl Into<String>, tool_name: impl Into<String>) {
        let server_name = server_name.into();
        let tool_name = tool_name.into();
        let prefixed = create_prefixed_name(&server_name, &tool_name);
        self.tool_to_server.insert(prefixed, server_name);
    }

    /// Get the server name for a prefixed tool
    pub fn get_server_for_tool(&self, prefixed_tool_name: &str) -> Result<&str> {
        self.tool_to_server
            .get(prefixed_tool_name)
            .map(|s| s.as_str())
            .ok_or_else(|| McpProxyError::ToolNotFound(prefixed_tool_name.to_string()))
    }

    /// Get the unprefixed tool name
    pub fn get_unprefixed_name(&self, prefixed_tool_name: &str) -> Result<String> {
        let (_, tool_name) = parse_tool_name(prefixed_tool_name)?;
        Ok(tool_name)
    }

    /// Clear all registered tools
    pub fn clear(&mut self) {
        self.tool_to_server.clear();
    }

    /// Get all registered prefixed tool names
    pub fn get_all_tools(&self) -> Vec<String> {
        self.tool_to_server.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_name() {
        let (server, tool) = parse_tool_name("filesystem__read_file").unwrap();
        assert_eq!(server, "filesystem");
        assert_eq!(tool, "read_file");
    }

    #[test]
    fn test_parse_tool_name_invalid() {
        assert!(parse_tool_name("read_file").is_err());
        assert!(parse_tool_name("").is_err());
    }

    #[test]
    fn test_create_prefixed_name() {
        let prefixed = create_prefixed_name("github", "list_repos");
        assert_eq!(prefixed, "github__list_repos");
    }

    #[test]
    fn test_is_prefixed() {
        assert!(is_prefixed("filesystem__read_file"));
        assert!(!is_prefixed("read_file"));
    }

    #[test]
    fn test_tool_router() {
        let mut router = ToolRouter::new();

        router.register_tool("filesystem", "read_file");
        router.register_tool("github", "list_repos");

        assert_eq!(
            router.get_server_for_tool("filesystem__read_file").unwrap(),
            "filesystem"
        );
        assert_eq!(
            router.get_server_for_tool("github__list_repos").unwrap(),
            "github"
        );

        assert_eq!(
            router.get_unprefixed_name("filesystem__read_file").unwrap(),
            "read_file"
        );

        let tools = router.get_all_tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.contains(&"filesystem__read_file".to_string()));
        assert!(tools.contains(&"github__list_repos".to_string()));
    }
}
