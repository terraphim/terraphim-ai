use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::fmt::Write as _;

const DEFAULT_BRAVE_BASE_URL: &str = "https://api.search.brave.com/res/v1/web/search";
const DEFAULT_SEARXNG_BASE_URL: &str = "https://searxng.site/search";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchProvider {
    Brave,
    Searxng,
}

impl SearchProvider {
    fn from_provider(provider: &str) -> Option<Self> {
        match provider.trim().to_ascii_lowercase().as_str() {
            "brave" => Some(Self::Brave),
            "searxng" | "searx" => Some(Self::Searxng),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Brave => "brave",
            Self::Searxng => "searxng",
        }
    }
}

#[derive(Debug, Clone)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
    source: Option<String>,
}

/// Web search tool using search providers.
pub struct WebSearchTool {
    client: Client,
    provider: String,
    base_url: Option<String>,
    api_key: Option<String>,
}

impl WebSearchTool {
    /// Create a new web search tool.
    pub fn new() -> Self {
        Self::with_config("brave", None, None)
    }

    /// Create with a specific provider.
    pub fn with_provider(provider: impl Into<String>) -> Self {
        Self::with_config(provider, None, None)
    }

    /// Create with provider and optional base URL / API key.
    pub fn with_config(
        provider: impl Into<String>,
        base_url: Option<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            provider: provider.into(),
            base_url,
            api_key,
        }
    }

    fn provider_kind(&self) -> Result<SearchProvider, ToolError> {
        SearchProvider::from_provider(&self.provider).ok_or_else(|| ToolError::InvalidArguments {
            tool: "web_search".to_string(),
            message: format!(
                "Unsupported search provider '{}'. Supported providers: brave, searxng",
                self.provider
            ),
        })
    }

    fn selected_base_url(&self, provider: SearchProvider) -> &str {
        self.base_url.as_deref().unwrap_or(match provider {
            SearchProvider::Brave => DEFAULT_BRAVE_BASE_URL,
            SearchProvider::Searxng => DEFAULT_SEARXNG_BASE_URL,
        })
    }

    fn normalized_api_key(&self) -> Option<&str> {
        self.api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
    }

    /// Perform a web search.
    async fn search(&self, query: &str, num_results: usize) -> Result<String, ToolError> {
        let provider = self.provider_kind()?;
        let bounded_results = num_results.max(1);
        log::info!(
            "Web search for: {} (provider: {})",
            query,
            provider.as_str()
        );

        let results = match provider {
            SearchProvider::Brave => self.search_brave(query, bounded_results).await?,
            SearchProvider::Searxng => self.search_searxng(query, bounded_results).await?,
        };

        Ok(format_results(provider, query, &results))
    }

    async fn search_brave(
        &self,
        query: &str,
        num_results: usize,
    ) -> Result<Vec<SearchResult>, ToolError> {
        let api_key = self
            .normalized_api_key()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "web_search".to_string(),
                message:
                    "Brave provider requires an API key. Configure api_key in the constructor."
                        .to_string(),
            })?;

        let endpoint = self.selected_base_url(SearchProvider::Brave);
        let count = num_results.to_string();
        let response = self
            .client
            .get(endpoint)
            .header("Accept", "application/json")
            .header("X-Subscription-Token", api_key)
            .query(&[("q", query), ("count", count.as_str())])
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: format!("Brave API request failed for '{}': {}", endpoint, e),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: format!(
                    "Brave API returned HTTP {}{}",
                    status,
                    format_error_body(&body)
                ),
            });
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: format!("Failed to parse Brave API response: {}", e),
            })?;

        Ok(parse_brave_results(&payload, num_results))
    }

    async fn search_searxng(
        &self,
        query: &str,
        num_results: usize,
    ) -> Result<Vec<SearchResult>, ToolError> {
        let endpoint = self.selected_base_url(SearchProvider::Searxng);
        let count = num_results.to_string();
        let mut request = self.client.get(endpoint).query(&[
            ("q", query),
            ("format", "json"),
            ("count", count.as_str()),
        ]);

        if let Some(api_key) = self.normalized_api_key() {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: format!("SearXNG request failed for '{}': {}", endpoint, e),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: format!(
                    "SearXNG returned HTTP {}{}",
                    status,
                    format_error_body(&body)
                ),
            });
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "web_search".to_string(),
                message: format!("Failed to parse SearXNG response: {}", e),
            })?;

        Ok(parse_searxng_results(&payload, num_results))
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
         Supports providers: brave and searxng. \
         Brave requires an API key."
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

fn parse_brave_results(payload: &Value, num_results: usize) -> Vec<SearchResult> {
    payload
        .get("web")
        .and_then(|web| web.get("results"))
        .and_then(Value::as_array)
        .map(|results| {
            results
                .iter()
                .take(num_results)
                .map(|item| SearchResult {
                    title: json_string(item.get("title")),
                    url: json_string(item.get("url")),
                    snippet: first_non_empty(&[
                        json_string(item.get("description")),
                        item.get("extra_snippets")
                            .and_then(Value::as_array)
                            .and_then(|snippets| snippets.first())
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .unwrap_or_default()
                            .to_string(),
                    ]),
                    source: Some("brave".to_string()),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_searxng_results(payload: &Value, num_results: usize) -> Vec<SearchResult> {
    payload
        .get("results")
        .and_then(Value::as_array)
        .map(|results| {
            results
                .iter()
                .take(num_results)
                .map(|item| SearchResult {
                    title: json_string(item.get("title")),
                    url: json_string(item.get("url")),
                    snippet: first_non_empty(&[
                        json_string(item.get("content")),
                        json_string(item.get("snippet")),
                    ]),
                    source: item
                        .get("engine")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|engine| !engine.is_empty())
                        .map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn json_string(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn first_non_empty(candidates: &[String]) -> String {
    candidates
        .iter()
        .find(|candidate| !candidate.trim().is_empty())
        .cloned()
        .unwrap_or_default()
}

fn format_error_body(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let body_snippet: String = trimmed.chars().take(200).collect();
    if trimmed.chars().count() > 200 {
        format!(" - {}...", body_snippet)
    } else {
        format!(" - {}", body_snippet)
    }
}

fn format_results(provider: SearchProvider, query: &str, results: &[SearchResult]) -> String {
    let mut output = format!(
        "Web search results\nProvider: {}\nQuery: {}\nTotal results: {}",
        provider.as_str(),
        query,
        results.len()
    );

    if results.is_empty() {
        output.push_str("\n\nNo results found.");
        return output;
    }

    for (index, result) in results.iter().enumerate() {
        let title = if result.title.is_empty() {
            "(untitled)"
        } else {
            &result.title
        };
        let url = if result.url.is_empty() {
            "(missing URL)"
        } else {
            &result.url
        };

        let _ = writeln!(output, "\n\n{}. {}", index + 1, title);
        let _ = writeln!(output, "   URL: {}", url);
        if !result.snippet.is_empty() {
            let _ = writeln!(output, "   Snippet: {}", result.snippet);
        }
        if let Some(source) = &result.source {
            if !source.is_empty() {
                let _ = writeln!(output, "   Source: {}", source);
            }
        }
    }

    output
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

    #[test]
    fn test_web_search_provider_selection() {
        let brave = WebSearchTool::with_provider("BrAvE");
        assert_eq!(brave.provider_kind().unwrap(), SearchProvider::Brave);

        let searxng = WebSearchTool::with_provider("searxng");
        assert_eq!(searxng.provider_kind().unwrap(), SearchProvider::Searxng);

        let searx_alias = WebSearchTool::with_provider("SEARX");
        assert_eq!(
            searx_alias.provider_kind().unwrap(),
            SearchProvider::Searxng
        );
    }

    #[tokio::test]
    async fn test_web_search_unsupported_provider() {
        let tool = WebSearchTool::with_provider("google");
        let args = serde_json::json!({
            "query": "rust async",
            "num_results": 5
        });

        let result = tool.execute(args).await;
        match result {
            Err(ToolError::InvalidArguments { tool, message }) => {
                assert_eq!(tool, "web_search");
                assert!(message.contains("Unsupported search provider"));
                assert!(message.contains("brave"));
                assert!(message.contains("searxng"));
            }
            other => panic!("Expected InvalidArguments error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_web_search_brave_missing_api_key() {
        let tool = WebSearchTool::with_config("brave", None, None);
        let args = serde_json::json!({
            "query": "rust programming language",
            "num_results": 5
        });

        let result = tool.execute(args).await;
        match result {
            Err(ToolError::InvalidArguments { tool, message }) => {
                assert_eq!(tool, "web_search");
                assert!(message.contains("Brave provider requires an API key"));
            }
            other => panic!("Expected InvalidArguments error, got {:?}", other),
        }
    }
}
