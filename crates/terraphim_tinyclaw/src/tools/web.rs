use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use reqwest::Client;

/// Search provider trait for web search implementations.
#[async_trait]
pub trait SearchProvider: Send + Sync {
    /// Search the web and return formatted results.
    async fn search(&self, query: &str, num_results: usize) -> Result<String, ToolError>;
    /// Provider name.
    fn name(&self) -> &str;
}

/// Exa.ai search provider.
pub struct ExaProvider {
    client: Client,
    api_key: Option<String>,
}

impl ExaProvider {
    /// Create a new Exa provider.
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl SearchProvider for ExaProvider {
    fn name(&self) -> &str {
        "exa"
    }

    async fn search(&self, query: &str, num_results: usize) -> Result<String, ToolError> {
        log::info!("Searching via Exa: {}", query);

        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: "Exa API key not configured".to_string(),
            })?;

        let response = self
            .client
            .post("https://api.exa.ai/search")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "query": query,
                "numResults": num_results,
                "contents": {
                    "text": true
                }
            }))
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: format!("Exa API request failed: {}", e),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: format!("Exa API error ({}): {}", status, error_text),
            });
        }

        let data: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "web_search".to_string(),
                    message: format!("Failed to parse Exa response: {}", e),
                })?;

        // Format results
        let mut output = format!("Search results for '{}' via Exa:\n\n", query);

        if let Some(results) = data["results"].as_array() {
            for (i, result) in results.iter().take(num_results).enumerate() {
                let title = result["title"].as_str().unwrap_or("No title");
                let url = result["url"].as_str().unwrap_or("No URL");
                let text = result["text"].as_str().unwrap_or("");

                output.push_str(&format!(
                    "{}. {}\n{}
{}
\n",
                    i + 1,
                    title,
                    url,
                    text
                ));
            }
        }

        Ok(output)
    }
}

/// Kimi Search provider (Moonshot AI).
pub struct KimiSearchProvider {
    client: Client,
    api_key: Option<String>,
}

impl KimiSearchProvider {
    /// Create a new Kimi Search provider.
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl SearchProvider for KimiSearchProvider {
    fn name(&self) -> &str {
        "kimi_search"
    }

    async fn search(&self, query: &str, num_results: usize) -> Result<String, ToolError> {
        log::info!("Searching via Kimi: {}", query);

        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: "Kimi API key not configured".to_string(),
            })?;

        let response = self.client
            .post("https://api.moonshot.cn/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": "moonshot-v1-8k",
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a search assistant. Use web search to find information and provide concise, accurate results."
                    },
                    {
                        "role": "user",
                        "content": format!("Search for: {}. Provide {} results.", query, num_results)
                    }
                ],
                "tools": [
                    {
                        "type": "builtin_function",
                        "function": {
                            "name": "web_search"
                        }
                    }
                ]
            }))
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: format!("Kimi API request failed: {}", e),
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: format!("Kimi API error ({}): {}", status, error_text),
            });
        }

        let data: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "web_search".to_string(),
                    message: format!("Failed to parse Kimi response: {}", e),
                })?;

        // Extract search results from the response
        let mut output = format!("Search results for '{}' via Kimi:\n\n", query);

        if let Some(choices) = data["choices"].as_array() {
            if let Some(first) = choices.first() {
                if let Some(content) = first["message"]["content"].as_str() {
                    output.push_str(content);
                }
            }
        }

        Ok(output)
    }
}

/// Placeholder provider for when no API key is configured.
pub struct PlaceholderProvider;

#[async_trait]
impl SearchProvider for PlaceholderProvider {
    fn name(&self) -> &str {
        "placeholder"
    }

    async fn search(&self, query: &str, num_results: usize) -> Result<String, ToolError> {
        Ok(format!(
            "Search results for '{}' ({} results):\n\n\
             [Note: Web search integration requires API keys]\n\
             To enable web search:\n\
             1. Set up a search provider (exa, kimi_search)\n\
             2. Set the appropriate environment variable (EXA_API_KEY or KIMI_API_KEY)\n\
             3. Restart the application",
            query, num_results
        ))
    }
}

/// Web search tool using search providers.
pub struct WebSearchTool {
    provider: Box<dyn SearchProvider>,
}

impl WebSearchTool {
    /// Create a new web search tool with default provider.
    pub fn new() -> Self {
        Self::from_env()
    }

    /// Create with a specific provider.
    pub fn with_provider(provider: Box<dyn SearchProvider>) -> Self {
        Self { provider }
    }

    /// Create from environment variables.
    pub fn from_env() -> Self {
        Self::from_env_inner()
    }

    /// Create from configuration.
    ///
    /// If config specifies a search provider, uses it.
    /// Otherwise falls back to environment variables (same as `new()`).
    ///
    /// # Arguments
    /// * `config` - Optional web tools configuration
    ///
    /// # Supported Providers
    /// - "exa" - Exa search API
    /// - "kimi_search" - Kimi search API
    pub fn from_config(config: Option<&crate::config::WebToolsConfig>) -> Self {
        match config {
            Some(cfg) => match cfg.search_provider.as_deref() {
                Some("exa") => {
                    let api_key = std::env::var("EXA_API_KEY").ok().filter(|k| !k.is_empty());
                    Self::with_provider(Box::new(ExaProvider::new(api_key)))
                }
                Some("kimi_search") => {
                    let api_key = std::env::var("KIMI_API_KEY").ok().filter(|k| !k.is_empty());
                    Self::with_provider(Box::new(KimiSearchProvider::new(api_key)))
                }
                Some(_) | None => Self::from_env_inner(),
            },
            None => Self::from_env_inner(),
        }
    }

    /// Internal helper to create from environment variables.
    fn from_env_inner() -> Self {
        // Check for Exa API key
        if let Ok(api_key) = std::env::var("EXA_API_KEY") {
            if !api_key.is_empty() {
                return Self::with_provider(Box::new(ExaProvider::new(Some(api_key))));
            }
        }

        // Check for Kimi API key
        if let Ok(api_key) = std::env::var("KIMI_API_KEY") {
            if !api_key.is_empty() {
                return Self::with_provider(Box::new(KimiSearchProvider::new(Some(api_key))));
            }
        }

        // Fall back to placeholder
        Self::with_provider(Box::new(PlaceholderProvider))
    }

    /// Perform a web search.
    async fn search(&self, query: &str, num_results: usize) -> Result<String, ToolError> {
        self.provider.search(query, num_results).await
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
         Supports multiple search providers (exa, kimi_search). \
         Requires EXA_API_KEY or KIMI_API_KEY environment variable."
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

    /// Create from configuration.
    ///
    /// If config specifies a fetch mode, uses it.
    /// Otherwise defaults to "raw".
    ///
    /// # Arguments
    /// * `config` - Optional web tools configuration
    ///
    /// # Supported Modes
    /// - "raw" - Fetch raw HTML
    /// - "readability" - Extract readable content
    pub fn from_config(config: Option<&crate::config::WebToolsConfig>) -> Self {
        let mode = config
            .and_then(|c| c.fetch_mode.clone())
            .unwrap_or_else(|| "raw".to_string());

        Self {
            client: Client::new(),
            mode,
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
        let tool = WebSearchTool::with_provider(Box::new(PlaceholderProvider));
        let args = serde_json::json!({
            "query": "rust programming language",
            "num_results": 5
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("rust programming language"));
        assert!(result.contains("Web search integration requires API keys"));
    }

    #[test]
    fn test_exa_provider_name() {
        let provider = ExaProvider::new(None);
        assert_eq!(provider.name(), "exa");
    }

    #[test]
    fn test_kimi_provider_name() {
        let provider = KimiSearchProvider::new(None);
        assert_eq!(provider.name(), "kimi_search");
    }

    #[test]
    fn test_placeholder_provider_name() {
        let provider = PlaceholderProvider;
        assert_eq!(provider.name(), "placeholder");
    }
}
