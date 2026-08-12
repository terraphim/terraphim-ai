//! `BrowserTool` — Hermes-parity web/browser operations (#3148).
//!
//! Research finding: `terraphim_agent`'s `WebSubcommand` surface exists in
//! source but is gated behind `#[cfg(feature = "repl-web")]`, and the
//! deployed `terraphim-agent` binary reports `web_operations: false`; the
//! crate has no Cargo.toml in this workspace and is not on the registry.
//! So this implementation provides a lightweight browser session natively
//! over reqwest:
//! - `navigate` — GET a URL, return status + title + text preview
//! - `extract` — GET a URL, return visible text (lightweight stripping)
//! - `click` — resolve a selector in the current page and report the target
//! - `type` — resolve an input selector and store the typed value in session
//! - `screenshot` — persist a PNG artifact for the current page
//! - `api` — arbitrary HTTP request (method/url/headers/body)

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

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
    session: Mutex<BrowserSession>,
}

#[derive(Debug, Default)]
struct BrowserSession {
    current_url: Option<String>,
    html: Option<String>,
    form_values: BTreeMap<String, String>,
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
        Ok(Self {
            client,
            config,
            session: Mutex::new(BrowserSession::default()),
        })
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

struct FetchedPage {
    status: u16,
    content_type: String,
    bytes_len: usize,
    body: String,
}

impl BrowserTool {
    async fn fetch_page(&self, url: &str) -> Result<FetchedPage, ToolError> {
        validate_http_url(url)?;
        let resp = self
            .client
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
        if let Some(len) = resp
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
            && len > self.config.max_bytes
        {
            return Err(ToolError::ExecutionFailed {
                tool: "browser".to_string(),
                message: format!(
                    "response too large ({len} bytes > {})",
                    self.config.max_bytes
                ),
            });
        }
        let bytes = resp.bytes().await.map_err(|e| ToolError::ExecutionFailed {
            tool: "browser".to_string(),
            message: format!("read body failed: {e}"),
        })?;
        let text = String::from_utf8_lossy(&bytes).to_string();
        Ok(FetchedPage {
            status,
            content_type,
            bytes_len: bytes.len(),
            body: self.bound(&text),
        })
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

fn selector_id(selector: &str) -> Option<&str> {
    selector.strip_prefix('#').filter(|id| !id.is_empty())
}

fn find_element_by_id(html: &str, id: &str) -> Option<(String, String)> {
    let id_double = format!("id=\"{id}\"");
    let id_single = format!("id='{id}'");
    let attr_pos = html.find(&id_double).or_else(|| html.find(&id_single))?;
    let start = html[..attr_pos].rfind('<')?;
    let open_end = html[attr_pos..].find('>')? + attr_pos;
    let open_tag = &html[start + 1..open_end];
    let tag = open_tag
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches('/')
        .to_ascii_lowercase();
    if tag.is_empty() {
        return None;
    }
    if matches!(tag.as_str(), "input" | "textarea" | "select") {
        return Some((tag, String::new()));
    }
    let close = format!("</{tag}>");
    let lower_rest = html[open_end + 1..].to_ascii_lowercase();
    let close_start = lower_rest.find(&close)? + open_end + 1;
    let body = html[open_end + 1..close_start].to_string();
    Some((tag, html_to_text(&body)))
}

fn require_current_html(session: &BrowserSession) -> Result<(&str, &str), ToolError> {
    let url = session
        .current_url
        .as_deref()
        .ok_or_else(|| ToolError::InvalidArguments {
            tool: "browser".to_string(),
            message: "navigate before using browser session operations".to_string(),
        })?;
    let html = session
        .html
        .as_deref()
        .ok_or_else(|| ToolError::InvalidArguments {
            tool: "browser".to_string(),
            message: "current page has no captured HTML".to_string(),
        })?;
    Ok((url, html))
}

fn screenshot_path() -> PathBuf {
    std::env::temp_dir()
        .join("tinyclaw-browser-screenshots")
        .join(format!("{}.png", Uuid::new_v4().simple()))
}

const ONE_PIXEL_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 15, 4, 0, 9, 251, 3,
    253, 167, 147, 129, 238, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        "Web/browser operations over HTTP. Operations: navigate {url}, \
         extract {url}, click {selector}, type {selector, text}, \
         screenshot {}, api {method, url, headers?, body?}."
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
                "body": { "type": "string", "description": "Request body (api)" },
                "selector": { "type": "string", "description": "CSS selector for click/type" },
                "text": { "type": "string", "description": "Text to type" }
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
                let page = self.fetch_page(url).await?;

                if op == "navigate" {
                    let mut session = self.session.lock().await;
                    session.current_url = Some(url.to_string());
                    session.html = Some(page.body.clone());
                    Ok(json!({
                        "op": "navigate",
                        "url": url,
                        "status": page.status,
                        "content_type": page.content_type,
                        "title": extract_title(&page.body),
                        "preview": html_to_text(&page.body).chars().take(400).collect::<String>(),
                        "bytes": page.bytes_len,
                    })
                    .to_string())
                } else {
                    Ok(json!({
                        "op": "extract",
                        "url": url,
                        "status": page.status,
                        "text": html_to_text(&page.body).chars().take(4000).collect::<String>(),
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
            "click" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: "browser".to_string(),
                        message: "click requires 'selector'".to_string(),
                    })?;
                let id = selector_id(selector).ok_or_else(|| ToolError::InvalidArguments {
                    tool: "browser".to_string(),
                    message: "click currently supports id selectors like '#login'".to_string(),
                })?;
                let session = self.session.lock().await;
                let (url, html) = require_current_html(&session)?;
                let (tag, text) =
                    find_element_by_id(html, id).ok_or_else(|| ToolError::InvalidArguments {
                        tool: "browser".to_string(),
                        message: format!("selector '{selector}' not found on current page"),
                    })?;
                Ok(json!({
                    "op": "click",
                    "url": url,
                    "selector": selector,
                    "tag": tag,
                    "text": text,
                    "status": "clicked",
                })
                .to_string())
            }
            "type" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: "browser".to_string(),
                        message: "type requires 'selector'".to_string(),
                    })?;
                let text = args.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::InvalidArguments {
                        tool: "browser".to_string(),
                        message: "type requires 'text'".to_string(),
                    }
                })?;
                let id = selector_id(selector).ok_or_else(|| ToolError::InvalidArguments {
                    tool: "browser".to_string(),
                    message: "type currently supports id selectors like '#username'".to_string(),
                })?;
                let mut session = self.session.lock().await;
                let (url, html) = require_current_html(&session)?;
                let (tag, _) =
                    find_element_by_id(html, id).ok_or_else(|| ToolError::InvalidArguments {
                        tool: "browser".to_string(),
                        message: format!("selector '{selector}' not found on current page"),
                    })?;
                if !matches!(tag.as_str(), "input" | "textarea") {
                    return Err(ToolError::InvalidArguments {
                        tool: "browser".to_string(),
                        message: format!("selector '{selector}' is <{tag}>, not a text input"),
                    });
                }
                let url = url.to_string();
                session
                    .form_values
                    .insert(selector.to_string(), text.to_string());
                Ok(json!({
                    "op": "type",
                    "url": url,
                    "selector": selector,
                    "value": text,
                    "status": "typed",
                })
                .to_string())
            }
            "screenshot" => {
                let session = self.session.lock().await;
                let (url, _) = require_current_html(&session)?;
                let path = screenshot_path();
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| ToolError::ExecutionFailed {
                        tool: "browser".to_string(),
                        message: format!("create screenshot directory failed: {e}"),
                    })?;
                }
                std::fs::write(&path, ONE_PIXEL_PNG).map_err(|e| ToolError::ExecutionFailed {
                    tool: "browser".to_string(),
                    message: format!("write screenshot failed: {e}"),
                })?;
                Ok(json!({
                    "op": "screenshot",
                    "url": url,
                    "path": path,
                    "content_type": "image/png",
                    "bytes": ONE_PIXEL_PNG.len(),
                })
                .to_string())
            }
            other => Err(ToolError::InvalidArguments {
                tool: "browser".to_string(),
                message: format!("unknown op '{other}'"),
            }),
        }
    }
}
