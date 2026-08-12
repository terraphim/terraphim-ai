//! `BrowserTool` — Hermes-parity web/browser operations (#3148).
//!
//! Research finding: `terraphim_agent`'s `WebSubcommand` surface exists in
//! source but is gated behind `#[cfg(feature = "repl-web")]`, and the
//! deployed `terraphim-agent` binary reports `web_operations: false`; the
//! crate has no Cargo.toml in this workspace and is not on the registry.
//! So this implementation provides HTTP-backed operations natively over
//! reqwest:
//! - `navigate` — GET a URL, return status + title + text preview
//! - `extract` — GET a URL, return visible text (lightweight stripping)
//! - `api` — arbitrary HTTP request (method/url/headers/body)
//!
//! Browser-native ops (click/type/screenshot) return
//! `ToolError::BackendUnavailable` — they need a real browser engine that
//! the deployed stack does not currently expose. The tool probes
//! `terraphim-agent` first and includes capability/protocol evidence in the
//! error so placeholder CLI output is never reported as success.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Mutex;

/// Configuration for the browser tool.
#[derive(Debug, Clone)]
pub struct BrowserToolConfig {
    /// HTTP timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum response bytes captured.
    pub max_bytes: usize,
    /// Optional proxy URL.
    pub proxy: Option<String>,
    /// Optional terraphim-agent binary used to probe browser-native backend availability.
    pub agent_binary: Option<String>,
}

impl From<&crate::config::BrowserConfig> for BrowserToolConfig {
    fn from(cfg: &crate::config::BrowserConfig) -> Self {
        Self {
            timeout_secs: cfg.timeout_secs,
            max_bytes: cfg.max_bytes,
            proxy: cfg.proxy.clone(),
            agent_binary: cfg.agent_binary.clone(),
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

    async fn browser_native_unavailable(&self, op: &str, args: &Value) -> ToolError {
        let evidence = match &self.config.agent_binary {
            Some(binary) => probe_agent_web_operations(binary, op, args).await,
            None => {
                "agent_binary is disabled; no terraphim-agent browser-native backend configured"
                    .to_string()
            }
        };
        ToolError::BackendUnavailable {
            tool: "browser".to_string(),
            message: format!(
                "'{op}' requires a verified terraphim-agent web_operations backend; {evidence}. \
                 TinyClaw will not simulate browser-native success. Use navigate/extract/api for \
                 HTTP-backed operations."
            ),
        }
    }
}

fn preview_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(240).collect()
}

async fn run_agent(binary: &str, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new(binary)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("failed to execute {binary}: {e}"))
}

fn capabilities_web_enabled(stdout: &[u8]) -> Result<(bool, bool), String> {
    let value: Value = serde_json::from_slice(stdout).map_err(|e| {
        format!(
            "capabilities output is not JSON: {e}; stdout='{}'",
            preview_output(stdout)
        )
    })?;
    let web_operations = value
        .pointer("/features/web_operations")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let command_advertised = value
        .get("commands")
        .and_then(Value::as_array)
        .map(|commands| commands.iter().any(|v| v.as_str() == Some("web")))
        .unwrap_or(false);
    Ok((web_operations, command_advertised))
}

async fn probe_agent_web_operations(binary: &str, op: &str, args: &Value) -> String {
    let caps = match run_agent(
        binary,
        &["--robot", "--format", "json", "robot", "capabilities"],
    )
    .await
    {
        Ok(output) => output,
        Err(e) => return e,
    };
    if !caps.status.success() {
        return format!(
            "capability probe failed with status {:?}; stderr='{}'",
            caps.status.code(),
            preview_output(&caps.stderr)
        );
    }
    let (web_operations, web_command_advertised) = match capabilities_web_enabled(&caps.stdout) {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !web_operations {
        return "capability probe reports web_operations=false".to_string();
    }

    let help = match run_agent(binary, &["--help"]).await {
        Ok(output) => output,
        Err(e) => return e,
    };
    let help_mentions_web = preview_output(&help.stdout)
        .split_whitespace()
        .any(|word| word == "web");
    if !web_command_advertised || !help.status.success() || !help_mentions_web {
        return format!(
            "capability probe reports web_operations=true, but no usable web subcommand is advertised (commands_has_web={web_command_advertised}, help_status={:?})",
            help.status.code()
        );
    }

    let url = args.get("url").and_then(Value::as_str).unwrap_or("");
    let candidate = match op {
        "screenshot" => vec!["web", "screenshot", url],
        "click" => {
            let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
            vec!["web", "click", url, selector]
        }
        "type" => {
            let selector = args.get("selector").and_then(Value::as_str).unwrap_or("");
            let text = args.get("text").and_then(Value::as_str).unwrap_or("");
            vec!["web", "type", url, selector, text]
        }
        _ => vec!["web", op, url],
    };
    match run_agent(binary, &candidate).await {
        Ok(output) => {
            let stdout = preview_output(&output.stdout);
            let stderr = preview_output(&output.stderr);
            if stdout.to_ascii_lowercase().contains("not yet implemented")
                || stdout.to_ascii_lowercase().contains("not implemented")
            {
                format!("agent web protocol is placeholder-only: stdout='{stdout}'")
            } else if !output.status.success() {
                format!(
                    "agent web protocol probe failed with status {:?}; stderr='{stderr}' stdout='{stdout}'",
                    output.status.code()
                )
            } else {
                format!(
                    "agent reports web_operations=true, but TinyClaw has no verified JSON/result protocol for '{op}'; stdout='{stdout}'"
                )
            }
        }
        Err(e) => e,
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

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        "HTTP web operations. Supported: navigate {url}, extract {url}, \
         api {method, url, headers?, body?}. Browser-engine ops \
         click/type/screenshot return BackendUnavailable."
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
            "click" | "type" | "screenshot" => {
                // Without an explicit `url`, the op targets the current
                // session page, so a navigate must have happened first.
                let mut probe_args = args.clone();
                if probe_args.get("url").and_then(Value::as_str).is_none() {
                    let session = self.session.lock().await;
                    let (url, _) = require_current_html(&session)?;
                    probe_args["url"] = json!(url);
                }
                Err(self.browser_native_unavailable(op, &probe_args).await)
            }
            other => Err(ToolError::InvalidArguments {
                tool: "browser".to_string(),
                message: format!("unknown op '{other}'"),
            }),
        }
    }
}
