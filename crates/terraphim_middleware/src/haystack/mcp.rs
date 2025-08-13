use terraphim_config::Haystack;
use terraphim_types::{Document, Index};
use crate::{indexer::IndexMiddleware, Result};

/// MCP client haystack indexer
///
/// Expects haystack.location to be an MCP server URL or identifier and
/// extra_parameters to include any required connection info.
#[derive(Default)]
pub struct McpHaystackIndexer;

#[async_trait::async_trait]
impl IndexMiddleware for McpHaystackIndexer {
    fn index(
        &self,
        needle: &str,
        haystack: &Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send {
        // Placeholder SSE client: verify MCP server is reachable and return empty index.
        // Configuration:
        // - base_url: from haystack.location or extra_parameters["base_url"] (default http://127.0.0.1:3001)
        let base = if !haystack.location.is_empty() {
            haystack.location.clone()
        } else {
            haystack
                .get_extra_parameters()
                .get("base_url")
                .cloned()
                .unwrap_or_else(|| "http://127.0.0.1:3001".to_string())
        };
        let base = base.trim_end_matches('/').to_string();
        let _needle = needle.to_string();
        async move {
            let client = reqwest::Client::new();

            // Transport selection
            let transport = haystack
                .get_extra_parameters()
                .get("transport")
                .map(|s| s.as_str())
                .unwrap_or("sse");

            // Try SSE reachability (server-everything)
            if transport == "sse" {
                let sse_url = format!("{}/sse", base);
                match client.get(&sse_url).send().await {
                    Ok(resp) => {
                        if !resp.status().is_success() {
                            log::warn!("MCP SSE returned status {} at {}", resp.status(), sse_url);
                        } else {
                            log::info!("MCP SSE endpoint reachable at {}", sse_url);
                        }
                    }
                    Err(e) => log::warn!("SSE connect failed at {}: {}", sse_url, e),
                }
            } else if transport == "stdio" {
                // Placeholder: stdio transport requires spawning a process; not executed here
                log::info!("Using MCP stdio transport (not executed in this context)");
            } else if transport == "oauth" {
                // Placeholder: add OAuth header on requests
                log::info!("Using MCP oauth transport placeholder");
            }

            // Invoke via rust-sdk client if enabled; otherwise fallback to HTTP best-effort
            #[cfg(feature = "mcp-rust-sdk")]
            {
                match transport {
                    "stdio" => match query_mcp_stdio(&_needle).await {
                        Ok(index) => return Ok(index),
                        Err(e) => log::warn!("MCP stdio query failed: {}", e),
                    },
                    "oauth" => {
                        let token = haystack
                            .get_extra_parameters()
                            .get("oauth_token")
                            .cloned();
                        match query_mcp_sse(&base, &_needle, token.as_deref()).await {
                            Ok(index) => return Ok(index),
                            Err(e) => log::warn!("MCP oauth SSE query failed: {}", e),
                        }
                    }
                    _ => match query_mcp_sse(&base, &_needle, None).await {
                        Ok(index) => return Ok(index),
                        Err(e) => log::warn!("MCP SSE query failed: {}", e),
                    },
                }
            }

            // Fallback
            if transport == "oauth" {
                let bearer = haystack
                    .get_extra_parameters()
                    .get("oauth_token")
                    .map(|s| s.to_string());
                Ok(http_fallback_list_or_search(&client, &base, &_needle, bearer.as_deref()).await)
            } else {
                Ok(http_fallback_list_or_search(&client, &base, &_needle, None).await)
            }
        }
    }
}

/// Convert a generic JSON item into a `Document` best-effort.
fn item_to_document(item: &serde_json::Value) -> Option<Document> {
    let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let title = item
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(id);
    let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let body = item
        .get("content")
        .or_else(|| item.get("body"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if title.is_empty() {
        return None;
    }
    let mut doc = Document::default();
    doc.id = if !id.is_empty() {
        id.to_string()
    } else if !url.is_empty() {
        url.to_string()
    } else {
        title.to_string()
    };
    doc.title = title.to_string();
    doc.url = url.to_string();
    doc.body = body.to_string();
    doc.description = Some(body.chars().take(180).collect());
    Some(doc)
}

async fn http_fallback_list_or_search(
    client: &reqwest::Client,
    base: &str,
    needle: &str,
    bearer: Option<&str>,
) -> Index {
    let mut index = Index::new();
    let try_endpoints = vec![format!("{}/search", base), format!("{}/list", base)];
    for url in try_endpoints {
        let mut req = client.post(&url).json(&serde_json::json!({ "query": needle }));
        if let Some(token) = bearer {
            req = req.bearer_auth(token);
        }
        match req.send().await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(items) = json.as_array() {
                        for item in items {
                            if let Some(doc) = item_to_document(item) {
                                index.insert(doc.id.clone(), doc);
                            }
                        }
                        break;
                    }
                    // Some servers return an object with `items`
                    if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
                        for item in items {
                            if let Some(doc) = item_to_document(item) {
                                index.insert(doc.id.clone(), doc);
                            }
                        }
                        break;
                    }
                }
            }
            Ok(resp) => {
                log::debug!("MCP tool call {} returned {}", url, resp.status());
            }
            Err(e) => log::debug!("MCP tool call {} failed: {}", url, e),
        }
    }
    index
}

#[cfg(feature = "mcp-rust-sdk")]
async fn query_mcp_sse(base: &str, needle: &str, _bearer: Option<&str>) -> Result<Index> {
    use mcp_client::{ClientInfo, McpClient, McpClientTrait, McpService, SseTransport, Transport};
    use serde_json::json;
    use std::collections::HashMap;

    let sse_url = format!("{}/sse", base);
    let env: HashMap<String, String> = HashMap::new();
    let transport = SseTransport::new(sse_url, env);
    let handle = transport.start().await.map_err(|e| crate::Error::Indexation(e.to_string()))?;
    let mut client = McpClient::new(McpService::new(handle));
    let _ = client
        .initialize(
            ClientInfo {
                name: "terraphim".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            Default::default(),
        )
        .await
        .map_err(|e| crate::Error::Indexation(e.to_string()))?;

    let tools = client
        .list_tools(None)
        .await
        .map_err(|e| crate::Error::Indexation(e.to_string()))?;
    let tool_name = tools
        .tools
        .iter()
        .map(|t| t.name.as_str())
        .find(|&n| n == "search" || n == "list")
        .unwrap_or("list");

    let args = if tool_name == "search" {
        json!({ "query": needle })
    } else {
        json!({})
    };

    let call = client
        .call_tool(tool_name, args)
        .await
        .map_err(|e| crate::Error::Indexation(e.to_string()))?;

    let mut index = Index::new();
    for content in call.content {
        // Prefer explicit text blocks
        if let Some(text) = content.as_text() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
                if let Some(items) = value.as_array() {
                    for item in items {
                        if let Some(doc) = item_to_document(item) {
                            index.insert(doc.id.clone(), doc);
                        }
                    }
                }
            }
            continue;
        }
        // Handle embedded resources that may contain text
        if let mcp_spec::content::Content::Resource(res) = &content {
            let text = res.get_text();
            if !text.is_empty() {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(items) = value.as_array() {
                        for item in items {
                            if let Some(doc) = item_to_document(item) {
                                index.insert(doc.id.clone(), doc);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(index)
}

#[cfg(feature = "mcp-rust-sdk")]
async fn query_mcp_stdio(needle: &str) -> Result<Index> {
    use mcp_client::{ClientInfo, McpClient, McpClientTrait, McpService, StdioTransport, Transport};
    use serde_json::json;
    use std::collections::HashMap;
    use tokio::process::Command;

    // Launch server-everything in stdio mode
    let mut child = Command::new("npx")
        .arg("-y")
        .arg("@modelcontextprotocol/server-everything")
        .arg("stdio")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| crate::Error::Indexation(e.to_string()))?;

    let transport = StdioTransport::new(
        // executable
        "npx",
        // args
        vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-everything".to_string(),
            "stdio".to_string(),
        ],
        // env
        HashMap::new(),
    );
    let handle = transport.start().await.map_err(|e| crate::Error::Indexation(e.to_string()))?;
    let mut client = McpClient::new(McpService::new(handle));
    let _ = client
        .initialize(
            ClientInfo {
                name: "terraphim".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            Default::default(),
        )
        .await
        .map_err(|e| crate::Error::Indexation(e.to_string()))?;

    let tools = client
        .list_tools(None)
        .await
        .map_err(|e| crate::Error::Indexation(e.to_string()))?;
    let tool_name = tools
        .tools
        .iter()
        .map(|t| t.name.as_str())
        .find(|&n| n == "search" || n == "list")
        .unwrap_or("list");

    let args = if tool_name == "search" { json!({ "query": needle }) } else { json!({}) };
    let call = client
        .call_tool(tool_name, args)
        .await
        .map_err(|e| crate::Error::Indexation(e.to_string()))?;

    let mut index = Index::new();
    for content in call.content {
        if let Some(text) = content.as_text() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
                if let Some(items) = value.as_array() {
                    for item in items {
                        if let Some(doc) = item_to_document(item) {
                            index.insert(doc.id.clone(), doc);
                        }
                    }
                }
            }
            continue;
        }
        if let mcp_spec::content::Content::Resource(res) = &content {
            let text = res.get_text();
            if !text.is_empty() {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(items) = value.as_array() {
                        for item in items {
                            if let Some(doc) = item_to_document(item) {
                                index.insert(doc.id.clone(), doc);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(index)
}
