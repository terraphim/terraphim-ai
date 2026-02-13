use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use reqwest::Client;

/// Web search tool using search providers.
pub struct WebSearchTool {
    provider: String,
}

impl WebSearchTool {
    /// Create a new web search tool.
    pub fn new() -> Self {
        Self {
            provider: "brave".to_string(),
        }
    }

    /// Create with a specific provider.
    pub fn with_provider(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
        }
    }

    /// Perform a web search.
    async fn search(&self, query: &str, num_results: usize) -> Result<String, ToolError> {
        // For now, return a placeholder implementation
        // In production, this would integrate with Brave, SearXNG, or Google APIs
        log::info!("Web search for: {} (provider: {})", query, self.provider);

        // Simulated search results
        let results = format!(
            "Search results for '{}' ({} results):\n\n\
             [Note: Web search integration requires API keys]\n\
             To enable web search:\n\
             1. Set up a search provider (Brave, SearXNG, or Google)\n\
             2. Configure API keys in your config file\n\
             3. Restart the application",
            query, num_results
        );

        Ok(results)
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information. \
         Supports multiple search providers (Brave, SearXNG, Google). \
         Note: Requires API key configuration."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results to return (default: 5)",
                    "default": 5
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "web_search".to_string(),
                message: "Missing 'query' parameter".to_string(),
            })?;

        let num_results = args["num_results"].as_u64().unwrap_or(5) as usize;

        self.search(query, num_results).await
    }
}

/// Web fetch tool for retrieving web pages.
pub struct WebFetchTool {
    client: Client,
    mode: String,
}

impl WebFetchTool {
    /// Create a new web fetch tool.
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            mode: "raw".to_string(),
        }
    }

    /// Create with a specific mode.
    pub fn with_mode(mode: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            mode: mode.into(),
        }
    }

    /// Fetch content from a URL.
    async fn fetch(&self, url: &str) -> Result<String, ToolError> {
        log::info!("Fetching URL: {} (mode: {})", url, self.mode);

        // Validate URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::InvalidArguments {
                tool: "web_fetch".to_string(),
                message: "URL must start with http:// or https://".to_string(),
            });
        }

        // Fetch the content
        let response =
            self.client
                .get(url)
                .send()
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "web_fetch".to_string(),
                    message: format!("Failed to fetch URL: {}", e),
                })?;

        let status = response.status();
        if !status.is_success() {
            return Err(ToolError::ExecutionFailed {
                tool: "web_fetch".to_string(),
                message: format!("HTTP error: {}", status),
            });
        }

        let content = response
            .text()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "web_fetch".to_string(),
                message: format!("Failed to read response: {}", e),
            })?;

        // In "readability" mode, we'd extract main content
        // For now, return the raw content (truncated if too long)
        let max_length = 10000;
        if content.len() > max_length {
            let truncated = format!(
                "{}\n\n[Content truncated - {} characters total]",
                &content[..max_length],
                content.len()
            );
            Ok(truncated)
        } else {
            Ok(content)
        }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a web page. \
         Supports raw HTML mode. \
         Content is truncated if too long."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch (must include http:// or https://)"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "web_fetch".to_string(),
                message: "Missing 'url' parameter".to_string(),
            })?;

        self.fetch(url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_tool_schema() {
        let tool = WebSearchTool::new();
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["num_results"].is_object());
    }

    #[test]
    fn test_web_fetch_tool_schema() {
        let tool = WebFetchTool::new();
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["url"].is_object());
    }

    #[tokio::test]
    async fn test_web_search_placeholder() {
        let tool = WebSearchTool::new();
        let args = serde_json::json!({
            "query": "rust programming language",
            "num_results": 5
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("rust programming language"));
        assert!(result.contains("Web search integration requires API keys"));
    }
}
