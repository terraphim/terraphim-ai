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
//! the deployed stack does not currently expose. The tool probes
//! `terraphim-agent` first and includes capability/protocol evidence in the
//! error so placeholder CLI output is never reported as success.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::Instant;

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

    async fn browser_native_unavailable(&self, op: &str, _args: &Value) -> ToolError {
        let evidence = match &self.config.agent_binary {
            Some(binary) => probe_agent_web_operations(binary, self.config.timeout_secs, op).await,
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

async fn run_agent(
    binary: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<std::process::Output, String> {
    let mut child = Command::new(binary)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("failed to execute {binary}: {e}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("failed to capture stdout for {binary}"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("failed to capture stderr for {binary}"))?;

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).await.map(|_| buf)
    });
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).await.map(|_| buf)
    });
    let mut stdout_task = stdout_task;
    let mut stderr_task = stderr_task;

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let status = match tokio::time::timeout_at(deadline, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => {
            stdout_task.abort();
            stderr_task.abort();
            return Err(format!("failed to wait for {binary}: {e}"));
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            stdout_task.abort();
            stderr_task.abort();
            return Err(format!(
                "probe timed out after {timeout_secs}s; killed terraphim-agent subprocess; stdout/stderr readers aborted"
            ));
        }
    };

    let stdout = tokio::select! {
        result = &mut stdout_task => {
            result
                .map_err(|e| format!("failed to join stdout reader for {binary}: {e}"))?
                .map_err(|e| format!("failed to read stdout from {binary}: {e}"))?
        }
        _ = tokio::time::sleep_until(deadline) => {
            stdout_task.abort();
            stderr_task.abort();
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            return Err(format!(
                "probe timed out after {timeout_secs}s; killed terraphim-agent subprocess if still running; stdout/stderr readers aborted"
            ));
        }
    };
    let stderr = tokio::select! {
        result = &mut stderr_task => {
            result
                .map_err(|e| format!("failed to join stderr reader for {binary}: {e}"))?
                .map_err(|e| format!("failed to read stderr from {binary}: {e}"))?
        }
        _ = tokio::time::sleep_until(deadline) => {
            stdout_task.abort();
            stderr_task.abort();
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            return Err(format!(
                "probe timed out after {timeout_secs}s; killed terraphim-agent subprocess if still running; stdout/stderr readers aborted"
            ));
        }
    };

    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
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

async fn probe_agent_web_operations(binary: &str, timeout_secs: u64, op: &str) -> String {
    let caps = match run_agent(
        binary,
        &["--robot", "--format", "json", "robot", "capabilities"],
        timeout_secs,
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

    let help = match run_agent(binary, &["--help"], timeout_secs).await {
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

    format!(
        "agent reports web_operations=true and advertises web, but TinyClaw has no verified non-mutating JSON/result protocol for '{op}'"
    )
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
            "click" | "type" | "screenshot" => {
                Err(self.browser_native_unavailable(op, &args).await)
            }
            other => Err(ToolError::InvalidArguments {
                tool: "browser".to_string(),
                message: format!("unknown op '{other}'"),
            }),
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::config::BrowserConfig;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn write_shim(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("terraphim-agent-shim");
        fs::write(&path, body).expect("write shim");
        let mut perms = fs::metadata(&path).expect("shim metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod shim");
        path
    }

    fn read_log(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_default()
    }

    fn browser_with_agent(agent_binary: String, timeout_secs: u64) -> BrowserTool {
        let cfg = BrowserConfig {
            enabled: true,
            timeout_secs,
            max_bytes: 4096,
            proxy: None,
            agent_binary: Some(agent_binary),
        };
        BrowserTool::from_config(&cfg).expect("browser tool")
    }

    #[tokio::test]
    async fn browser_native_probe_does_not_execute_requested_mutating_command() {
        let temp = TempDir::new().expect("tempdir");
        let log = temp.path().join("invocations.log");
        let shim = write_shim(
            temp.path(),
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$*" >> '{}'
if [ "$*" = "--robot --format json robot capabilities" ]; then
  printf '%s\n' '{{"features":{{"web_operations":true}},"commands":["web"]}}'
  exit 0
fi
if [ "$*" = "--help" ]; then
  printf '%s\n' 'Usage: terraphim-agent web'
  exit 0
fi
printf '%s\n' 'mutating web command executed' >&2
exit 23
"#,
                log.display()
            ),
        );
        let tool = browser_with_agent(shim.display().to_string(), 2);

        let err = tool
            .execute(json!({
                "op": "click",
                "url": "https://example.com",
                "selector": "#submit"
            }))
            .await
            .expect_err("click should fail closed");

        assert!(matches!(err, ToolError::BackendUnavailable { .. }));
        let invocations = read_log(&log);
        assert!(invocations.contains("--robot --format json robot capabilities"));
        assert!(invocations.contains("--help"));
        assert!(
            !invocations.contains("web click"),
            "probe executed requested mutating command: {invocations}"
        );
    }

    #[tokio::test]
    async fn browser_native_probe_timeout_is_bounded_and_reaps_hanging_child() {
        let temp = TempDir::new().expect("tempdir");
        let pid_file = temp.path().join("shim.pid");
        let shim = write_shim(
            temp.path(),
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$$" > '{}'
exec sleep 30
"#,
                pid_file.display()
            ),
        );
        let tool = browser_with_agent(shim.display().to_string(), 1);

        let result = tokio::time::timeout(
            Duration::from_secs(3),
            tool.execute(json!({
                "op": "screenshot",
                "url": "https://example.com"
            })),
        )
        .await
        .expect("probe should be bounded by BrowserConfig timeout_secs");

        assert!(matches!(result, Err(ToolError::BackendUnavailable { .. })));
        let pid: u32 = fs::read_to_string(&pid_file)
            .expect("pid file")
            .trim()
            .parse()
            .expect("pid");
        let status = std::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .status()
            .expect("kill -0");
        assert!(
            !status.success(),
            "hanging shim process {pid} is still alive"
        );
    }

    #[tokio::test]
    async fn browser_native_probe_timeout_is_bounded_when_descendant_inherits_pipes() {
        let temp = TempDir::new().expect("tempdir");
        let direct_pid_file = temp.path().join("direct.pid");
        let descendant_pid_file = temp.path().join("descendant.pid");
        let shim = write_shim(
            temp.path(),
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$$" > '{}'
sleep 30 &
printf '%s\n' "$!" > '{}'
exec sleep 30
"#,
                direct_pid_file.display(),
                descendant_pid_file.display()
            ),
        );
        let tool = browser_with_agent(shim.display().to_string(), 1);

        let started = std::time::Instant::now();
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            tool.execute(json!({
                "op": "screenshot",
                "url": "https://example.com"
            })),
        )
        .await;

        if let Ok(pid_text) = fs::read_to_string(&descendant_pid_file)
            && let Ok(pid) = pid_text.trim().parse::<u32>()
        {
            let _ = std::process::Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .status();
        }

        let result = result.expect("probe should not wait indefinitely for inherited pipes");
        assert!(matches!(result, Err(ToolError::BackendUnavailable { .. })));
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "probe exceeded independent timeout guard"
        );

        let direct_pid: u32 = fs::read_to_string(&direct_pid_file)
            .expect("direct pid file")
            .trim()
            .parse()
            .expect("direct pid");
        let status = std::process::Command::new("kill")
            .arg("-0")
            .arg(direct_pid.to_string())
            .status()
            .expect("kill -0 direct child");
        assert!(
            !status.success(),
            "direct shim process {direct_pid} is still alive"
        );
    }

    #[tokio::test]
    async fn browser_native_probe_timeout_after_child_exit_with_inherited_pipes() {
        let temp = TempDir::new().expect("tempdir");
        let descendant_pid_file = temp.path().join("descendant.pid");
        let shim = write_shim(
            temp.path(),
            &format!(
                r#"#!/bin/sh
sleep 30 &
printf '%s\n' "$!" > '{}'
exit 0
"#,
                descendant_pid_file.display()
            ),
        );
        let tool = browser_with_agent(shim.display().to_string(), 1);

        let started = std::time::Instant::now();
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            tool.execute(json!({
                "op": "screenshot",
                "url": "https://example.com"
            })),
        )
        .await;

        if let Ok(pid_text) = fs::read_to_string(&descendant_pid_file)
            && let Ok(pid) = pid_text.trim().parse::<u32>()
        {
            let _ = std::process::Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .status();
        }

        let result = result.expect("probe should not wait indefinitely after direct child exits 0");
        assert!(matches!(result, Err(ToolError::BackendUnavailable { .. })));
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "probe exceeded independent timeout guard"
        );
    }
}
