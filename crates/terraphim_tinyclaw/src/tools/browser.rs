//! `BrowserTool` — Hermes-parity web/browser operations (#3148).
//!
//! Research finding: `terraphim_agent`'s `WebSubcommand` surface exists in
//! source but is gated behind `#[cfg(feature = "repl-web")]`, and the
//! deployed `terraphim-agent` binary reports `web_operations: false`; the
//! crate has no Cargo.toml in this workspace and is not on the registry.
//! So v1 implements browser operations natively over reqwest:
//! - `navigate` — GET a URL, return status + title + text preview
//! - `extract` — GET a URL, return visible text (lightweight stripping)
//! - `api` — arbitrary HTTP request (method/url/headers/body)
//!
//! Browser-native ops (click/type/screenshot) return
//! `ToolError::BackendUnavailable` — they need a real browser engine that
//! the deployed stack does not currently expose.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::time::Duration;

/// Configuration for the browser tool.
#[derive(Debug, Clone)]
pub struct BrowserToolConfig {
    /// HTTP timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum response bytes captured.
    pub max_bytes: usize,
    /// Optional proxy URL.
    pub proxy: Option<String>,
}

impl From<&crate::config::BrowserConfig> for BrowserToolConfig {
    fn from(cfg: &crate::config::BrowserConfig) -> Self {
        Self {
            timeout_secs: cfg.timeout_secs,
            max_bytes: cfg.max_bytes,
            proxy: cfg.proxy.clone(),
        }
    }
}

/// The browser tool.
pub struct BrowserTool {
    client: reqwest::Client,
    config: BrowserToolConfig,
}

impl BrowserTool {
    /// Create a browser tool from config.
    pub fn from_config(cfg: &crate::config::BrowserConfig) -> Result<Self, ToolError> {
        let config = BrowserToolConfig::from(cfg);
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent("terraphim-tinyclaw/1.0 (+https://terraphim.ai) browser-tool");
        if let Some(proxy) = &config.proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(|e| {
                ToolError::ExecutionFailed {
                    tool: "browser".to_string(),
                    message: format!("invalid proxy '{proxy}': {e}"),
                }
            })?);
        }
        let client = builder.build().map_err(|e| ToolError::ExecutionFailed {
            tool: "browser".to_string(),
            message: format!("failed to build HTTP client: {e}"),
        })?;
        Ok(Self { client, config })
    }

    /// Bound a body to max_bytes on a char boundary.
    fn bound(&self, s: &str) -> String {
        let max = self.config.max_bytes;
        if s.len() <= max {
            return s.to_string();
        }
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}… (truncated, {} bytes)", &s[..end], s.len())
    }
}

/// Validate that a URL uses http/https (matches `web_fetch` behaviour;
/// reqwest rejects other schemes anyway, but fail with a clear message).
fn validate_http_url(url: &str) -> Result<(), ToolError> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        Err(ToolError::InvalidArguments {
            tool: "browser".to_string(),
            message: format!("URL must start with http:// or https:// (got '{url}')"),
        })
    }
}

/// Extract a rough page title from HTML.
fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title>")? + 7;
    let end = lower[start..].find("</title>")? + start;
    let title = html[start..end].trim();
    if title.is_empty() {
        None
    } else {
        Some(title.to_string())
    }
}

/// Strip HTML tags/scripts/styles into approximate visible text.
fn html_to_text(html: &str) -> String {
    // Drop script/style blocks first.
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    loop {
        let lower = rest.to_ascii_lowercase();
        let skip_start = ["<script", "<style"].iter().find_map(|tag| {
            let idx = lower.find(tag)?;
            // Find closing tag.
            let close = lower[idx..].find("</")? + idx;
            Some((idx, close))
        });
        match skip_start {
            Some((idx, close)) => {
                out.push_str(&rest[..idx]);
                rest = &rest[close..];
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    // Strip tags entirely (regex removes `<...>` including tag names), then
    // collapse whitespace runs into single spaces.
    let tag_re = regex::Regex::new(r"<[^>]*>").expect("static tag regex");
    let no_tags = tag_re.replace_all(&out, " ");
    let mut text = String::with_capacity(no_tags.len());
    let mut prev_space = false;
    for ch in no_tags.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                text.push(' ');
                prev_space = true;
            }
        } else {
            text.push(ch);
            prev_space = false;
        }
    }
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        "".to_string()
    } else {
        trimmed
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        "Web/browser operations over HTTP. Operations: navigate {url}, \
         extract {url}, api {method, url, headers?, body?}. Browser-native \
         ops (click/type/screenshot) are unavailable in this build."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "op": {
                    "type": "string",
                    "enum": ["navigate", "extract", "api", "click", "type", "screenshot"],
                    "description": "Operation to perform"
                },
                "url": { "type": "string", "description": "Target URL" },
                "method": { "type": "string", "description": "HTTP method (api)" },
                "headers": { "type": "object", "description": "Extra headers (api)" },
                "body": { "type": "string", "description": "Request body (api)" }
            },
            "required": ["op"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let op =
            args.get("op")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: "browser".to_string(),
                    message: "missing required 'op' field".to_string(),
                })?;

        match op {
            "navigate" | "extract" => {
                let url = args.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::InvalidArguments {
                        tool: "browser".to_string(),
                        message: format!("{op} requires 'url'"),
                    }
                })?;
                validate_http_url(url)?;
                let resp =
                    self.client
                        .get(url)
                        .send()
                        .await
                        .map_err(|e| ToolError::ExecutionFailed {
                            tool: "browser".to_string(),
                            message: format!("GET {url} failed: {e}"),
                        })?;
                let status = resp.status().as_u16();
                let content_type = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                // Reject oversized responses up front via content-length.
                if let Some(len) = resp
                    .headers()
                    .get(reqwest::header::CONTENT_LENGTH)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<usize>().ok())
                    && len > self.config.max_bytes
                {
                    return Ok(json!({
                        "op": op,
                        "url": url,
                        "status": status,
                        "content_type": content_type,
                        "error": format!(
                            "response too large ({len} bytes > {})",
                            self.config.max_bytes
                        ),
                        "bytes": len,
                    })
                    .to_string());
                }
                let bytes = resp.bytes().await.map_err(|e| ToolError::ExecutionFailed {
                    tool: "browser".to_string(),
                    message: format!("read body failed: {e}"),
                })?;
                let text = String::from_utf8_lossy(&bytes).to_string();
                let body = self.bound(&text);

                if op == "navigate" {
                    Ok(json!({
                        "op": "navigate",
                        "url": url,
                        "status": status,
                        "content_type": content_type,
                        "title": extract_title(&body),
                        "preview": html_to_text(&body).chars().take(400).collect::<String>(),
                        "bytes": bytes.len(),
                    })
                    .to_string())
                } else {
                    Ok(json!({
                        "op": "extract",
                        "url": url,
                        "status": status,
                        "text": html_to_text(&body).chars().take(4000).collect::<String>(),
                    })
                    .to_string())
                }
            }
            "api" => {
                let url = args.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::InvalidArguments {
                        tool: "browser".to_string(),
                        message: "api requires 'url'".to_string(),
                    }
                })?;
                validate_http_url(url)?;
                let method = args
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("GET")
                    .to_uppercase();
                let mut req = self.client.request(
                    reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| {
                        ToolError::InvalidArguments {
                            tool: "browser".to_string(),
                            message: format!("invalid HTTP method '{method}'"),
                        }
                    })?,
                    url,
                );
                if let Some(headers) = args.get("headers").and_then(|v| v.as_object()) {
                    for (k, v) in headers {
                        if let Some(vs) = v.as_str() {
                            req = req.header(k, vs);
                        }
                    }
                }
                if let Some(body) = args.get("body").and_then(|v| v.as_str()) {
                    req = req.body(body.to_string());
                }
                let resp = req.send().await.map_err(|e| ToolError::ExecutionFailed {
                    tool: "browser".to_string(),
                    message: format!("{method} {url} failed: {e}"),
                })?;
                let status = resp.status().as_u16();
                if let Some(len) = resp
                    .headers()
                    .get(reqwest::header::CONTENT_LENGTH)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<usize>().ok())
                    && len > self.config.max_bytes
                {
                    return Ok(json!({
                        "op": "api",
                        "method": method,
                        "url": url,
                        "status": status,
                        "error": format!(
                            "response too large ({len} bytes > {})",
                            self.config.max_bytes
                        ),
                        "bytes": len,
                    })
                    .to_string());
                }
                let bytes = resp.bytes().await.map_err(|e| ToolError::ExecutionFailed {
                    tool: "browser".to_string(),
                    message: format!("read body failed: {e}"),
                })?;
                let text = String::from_utf8_lossy(&bytes).to_string();
                Ok(json!({
                    "op": "api",
                    "method": method,
                    "url": url,
                    "status": status,
                    "body": self.bound(&text),
                    "bytes": bytes.len(),
                })
                .to_string())
            }
            "click" | "type" | "screenshot" => Err(ToolError::BackendUnavailable {
                tool: "browser".to_string(),
                message: format!(
                    "'{op}' requires a browser engine; the deployed terraphim-agent \
                     build has web_operations disabled (feature 'repl-web'). \
                     Use navigate/extract/api instead."
                ),
            }),
            other => Err(ToolError::InvalidArguments {
                tool: "browser".to_string(),
                message: format!("unknown op '{other}'"),
            }),
        }
    }
}
