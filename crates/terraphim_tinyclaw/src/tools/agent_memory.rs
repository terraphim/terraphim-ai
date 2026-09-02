//! Bridge tools for `terraphim-agent memory` and `terraphim-agent learn`
//! CLI subcommands.
//!
//! Each tool shells out to the `terraphim-agent` binary via
//! `tokio::process::Command`, following the same subprocess pattern as
//! [`ShellTool`](super::shell::ShellTool).

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

// ---------------------------------------------------------------------------
// Shared config
// ---------------------------------------------------------------------------

/// Maximum bytes of subprocess stdout accepted per call. Guards against a
/// runaway `memory export` payload exhausting memory (see PR review P2).
pub(crate) const MAX_OUTPUT_BYTES: usize = 1 << 20; // 1 MiB

/// Configuration shared by all agent-memory bridge tools.
#[derive(Debug, Clone)]
pub struct AgentMemoryConfig {
    /// Path to the terraphim-agent binary. Defaults to `"terraphim-agent"`
    /// (resolved via PATH).
    pub binary: PathBuf,
    /// Optional role override for memory retrieve/apply.
    pub role: Option<String>,
    /// Timeout for subprocess calls in seconds.
    pub timeout_secs: u64,
    /// Maximum characters of memory context to inject into the system
    /// prompt (token-budget guard). Defaults to 4000 (~1000 tokens).
    pub max_context_chars: usize,
}

impl From<&crate::config::MemoryConfig> for AgentMemoryConfig {
    fn from(cfg: &crate::config::MemoryConfig) -> Self {
        Self {
            binary: PathBuf::from(&cfg.binary),
            role: cfg.role.clone(),
            timeout_secs: cfg.timeout_secs,
            max_context_chars: cfg.max_context_chars,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared subprocess helper
// ---------------------------------------------------------------------------

/// Execute `terraphim-agent` with the given args, optional stdin, and timeout.
/// Returns stdout on success. Maps errors to `ToolError` variants.
pub(crate) async fn run_agent(
    config: &AgentMemoryConfig,
    args: &[&str],
    stdin_data: Option<&str>,
) -> Result<String, ToolError> {
    let result = timeout(
        Duration::from_secs(config.timeout_secs),
        run_agent_inner(config, args, stdin_data),
    )
    .await;

    match result {
        Ok(inner) => inner,
        Err(_elapsed) => Err(ToolError::Timeout {
            tool: "agent_memory".to_string(),
            seconds: config.timeout_secs,
        }),
    }
}

async fn run_agent_inner(
    config: &AgentMemoryConfig,
    args: &[&str],
    stdin_data: Option<&str>,
) -> Result<String, ToolError> {
    let output = if let Some(data) = stdin_data {
        // Spawn, pipe stdin, then collect output.
        let mut child = Command::new(&config.binary)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| map_io_error(&config.binary, e))?;

        // Write stdin then drop to send EOF.
        if let Some(mut stdin_handle) = child.stdin.take() {
            stdin_handle.write_all(data.as_bytes()).await.map_err(|e| {
                ToolError::ExecutionFailed {
                    tool: "agent_memory".to_string(),
                    message: format!("Failed to write stdin: {}", e),
                }
            })?;
            // drop sends EOF
        }

        child
            .wait_with_output()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "agent_memory".to_string(),
                message: format!("Failed to wait for process: {}", e),
            })?
    } else {
        Command::new(&config.binary)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| map_io_error(&config.binary, e))?
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Stdout cap: a runaway `memory export` must not exhaust memory.
    if output.stdout.len() > MAX_OUTPUT_BYTES {
        return Err(ToolError::ExecutionFailed {
            tool: "agent_memory".to_string(),
            message: format!(
                "terraphim-agent output exceeds cap of {MAX_OUTPUT_BYTES} bytes (got {})",
                output.stdout.len()
            ),
        });
    }

    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);
        return Err(ToolError::ExecutionFailed {
            tool: "agent_memory".to_string(),
            message: format!(
                "terraphim-agent exited with code {}\nSTDERR: {}",
                exit_code, stderr
            ),
        });
    }

    Ok(stdout)
}

/// Map `io::Error` from `Command::spawn`/`Command::output` to `ToolError`.
fn map_io_error(binary: &std::path::Path, e: std::io::Error) -> ToolError {
    if e.kind() == std::io::ErrorKind::NotFound {
        ToolError::ExecutionFailed {
            tool: "agent_memory".to_string(),
            message: format!(
                "terraphim-agent binary not found at '{}'. \
                 Install terraphim-agent or set memory.binary in config.",
                binary.display()
            ),
        }
    } else {
        ToolError::ExecutionFailed {
            tool: "agent_memory".to_string(),
            message: format!("Failed to execute terraphim-agent: {}", e),
        }
    }
}

// ---------------------------------------------------------------------------
// JSON parsing structs (lenient -- no deny_unknown_fields)
// ---------------------------------------------------------------------------

/// Top-level envelope from `terraphim-agent memory export --format json`.
#[derive(Debug, Deserialize)]
pub(crate) struct MemoryExport {
    #[serde(default)]
    pub memory_items: Vec<MemoryItem>,
}

/// Individual memory item from the export.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct MemoryItem {
    pub id: String,
    #[serde(default)]
    pub item_type: String,
    pub content: String,
    #[serde(default)]
    pub importance: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub access_count: u64,
    #[serde(default)]
    pub created_at: String,
}

/// Truncate a string to at most `max` chars, never splitting a UTF-8
/// character (char-boundary safe — cannot panic on non-ASCII input).
pub(crate) fn truncate_chars(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Client-side filter: case-insensitive substring match on `content`.
pub(crate) fn filter_items<'a>(
    items: &'a [MemoryItem],
    query: &str,
    limit: usize,
) -> Vec<&'a MemoryItem> {
    let query_lower = query.to_lowercase();
    items
        .iter()
        .filter(|item| item.content.to_lowercase().contains(&query_lower))
        .take(limit)
        .collect()
}

// ---------------------------------------------------------------------------
// Tool 1: MemoryCaptureTool
// ---------------------------------------------------------------------------

/// Capture a memory item into the terraphim-agent evolution store.
pub struct MemoryCaptureTool {
    config: Arc<AgentMemoryConfig>,
}

impl MemoryCaptureTool {
    pub fn new(config: Arc<AgentMemoryConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for MemoryCaptureTool {
    fn name(&self) -> &str {
        "memory_capture"
    }

    fn description(&self) -> &str {
        "Capture a memory item into the terraphim-agent evolution store. \
         Accepts text content and an optional provenance tag."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The memory content to capture"
                },
                "provenance_tag": {
                    "type": "string",
                    "description": "Optional provenance tag (e.g. session ID)"
                }
            },
            "required": ["content"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let content = args["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "memory_capture".to_string(),
                message: "Missing required 'content' parameter".to_string(),
            })?;

        let tag = args["provenance_tag"].as_str().unwrap_or("tinyclaw");

        let stdin_json = serde_json::json!({
            "content": content,
            "item_type": "Experience",
            "importance": "Medium"
        })
        .to_string();

        let mut cli_args = vec!["memory", "capture", "--provenance-tag", tag];

        // Add role if configured.
        let role_owned;
        if let Some(ref role) = self.config.role {
            role_owned = role.clone();
            cli_args.push("--role");
            cli_args.push(&role_owned);
        }

        run_agent(&self.config, &cli_args, Some(&stdin_json)).await
    }
}

// ---------------------------------------------------------------------------
// Tool 2: MemoryRetrieveTool
// ---------------------------------------------------------------------------

/// Retrieve memory items matching a query from the evolution store.
pub struct MemoryRetrieveTool {
    config: Arc<AgentMemoryConfig>,
}

impl MemoryRetrieveTool {
    pub fn new(config: Arc<AgentMemoryConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for MemoryRetrieveTool {
    fn name(&self) -> &str {
        "memory_retrieve"
    }

    fn description(&self) -> &str {
        "Retrieve memory items matching a query from the terraphim-agent \
         evolution store. Returns JSON array of matching items."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for memory retrieval"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of items to return",
                    "default": 5,
                    "minimum": 1,
                    "maximum": 20
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "memory_retrieve".to_string(),
                message: "Missing required 'query' parameter".to_string(),
            })?;

        let limit = args["limit"].as_u64().unwrap_or(5) as usize;
        let limit = limit.clamp(1, 20);

        // Use `memory export --format json` and filter client-side.
        // (memory retrieve lacks --json; see research.md:183-192)
        let cli_args = vec!["memory", "export", "--format", "json"];
        let raw = run_agent(&self.config, &cli_args, None).await?;

        let export: MemoryExport =
            serde_json::from_str(&raw).map_err(|e| ToolError::ExecutionFailed {
                tool: "memory_retrieve".to_string(),
                message: format!(
                    "Failed to parse export JSON: {}\nRaw output: {}",
                    e,
                    truncate_chars(&raw, 500)
                ),
            })?;

        let matches = filter_items(&export.memory_items, query, limit);
        let json = serde_json::to_string(&matches).map_err(|e| ToolError::ExecutionFailed {
            tool: "memory_retrieve".to_string(),
            message: format!("Failed to serialise results: {}", e),
        })?;

        Ok(json)
    }
}

// ---------------------------------------------------------------------------
// Tool 3: MemoryApplyTool
// ---------------------------------------------------------------------------

/// Retrieve relevant memories formatted for system-prompt injection.
pub struct MemoryApplyTool {
    config: Arc<AgentMemoryConfig>,
}

impl MemoryApplyTool {
    pub fn new(config: Arc<AgentMemoryConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for MemoryApplyTool {
    fn name(&self) -> &str {
        "memory_apply"
    }

    fn description(&self) -> &str {
        "Retrieve relevant memories and format them for system-prompt injection. \
         Returns formatted memory context suitable for prepending to prompts."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The current prompt/query to retrieve memories for"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "memory_apply".to_string(),
                message: "Missing required 'prompt' parameter".to_string(),
            })?;

        let mut cli_args = vec!["memory", "apply", "--prompt", prompt];

        let role_owned;
        if let Some(ref role) = self.config.role {
            role_owned = role.clone();
            cli_args.push("--role");
            cli_args.push(&role_owned);
        }

        let output = run_agent(&self.config, &cli_args, None).await?;

        // Empty output is not an error -- it signals no memories matched.
        Ok(output)
    }
}

// ---------------------------------------------------------------------------
// Tool 4: LearnCaptureTool
// ---------------------------------------------------------------------------

/// Capture a failed command into the terraphim-agent learning store.
pub struct LearnCaptureTool {
    config: Arc<AgentMemoryConfig>,
}

impl LearnCaptureTool {
    pub fn new(config: Arc<AgentMemoryConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for LearnCaptureTool {
    fn name(&self) -> &str {
        "learn_capture"
    }

    fn description(&self) -> &str {
        "Capture a failed command and its error into the terraphim-agent \
         learning store for future reference."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command that failed"
                },
                "error": {
                    "type": "string",
                    "description": "The error message or output"
                },
                "exit_code": {
                    "type": "integer",
                    "description": "The exit code of the failed command",
                    "default": 1
                }
            },
            "required": ["command", "error"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "learn_capture".to_string(),
                message: "Missing required 'command' parameter".to_string(),
            })?;

        let error = args["error"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "learn_capture".to_string(),
                message: "Missing required 'error' parameter".to_string(),
            })?;

        let exit_code = args["exit_code"].as_i64().unwrap_or(1);

        let exit_code_str = exit_code.to_string();
        let cli_args = vec![
            "learn",
            "capture",
            command,
            "--error",
            error,
            "--exit-code",
            &exit_code_str,
        ];

        run_agent(&self.config, &cli_args, None).await
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Arc<AgentMemoryConfig> {
        Arc::new(AgentMemoryConfig {
            binary: PathBuf::from("terraphim-agent"),
            role: None,
            timeout_secs: 10,
            max_context_chars: 4000,
        })
    }

    // -- Schema tests -------------------------------------------------------

    #[test]
    fn test_memory_capture_schema() {
        let tool = MemoryCaptureTool::new(test_config());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("content")));
        assert!(schema["properties"]["content"]["type"] == "string");
    }

    #[test]
    fn test_memory_retrieve_schema() {
        let tool = MemoryRetrieveTool::new(test_config());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("query")));
        assert!(schema["properties"]["limit"]["type"] == "integer");
    }

    #[test]
    fn test_memory_apply_schema() {
        let tool = MemoryApplyTool::new(test_config());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("prompt")));
    }

    #[test]
    fn test_learn_capture_schema() {
        let tool = LearnCaptureTool::new(test_config());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("command")));
        assert!(required.contains(&serde_json::json!("error")));
    }

    #[test]
    fn test_truncate_chars_non_ascii_no_panic() {
        // Multi-byte chars (emoji + Cyrillic) at the truncation boundary:
        // must not panic and must return valid UTF-8.
        let s = "привет мир 🌍 こんにちは".repeat(20);
        let t = truncate_chars(&s, 100);
        assert!(t.len() <= 100);
        assert!(t.is_char_boundary(t.len()));
        assert!(std::str::from_utf8(t.as_bytes()).is_ok());

        let t2 = truncate_chars(&s, 7); // mid-char boundary
        assert!(t2.len() <= 7);
        assert!(std::str::from_utf8(t2.as_bytes()).is_ok());

        // Short strings pass through untouched.
        assert_eq!(truncate_chars("abc", 10), "abc");
        assert_eq!(truncate_chars("", 5), "");
    }

    #[test]
    fn test_tool_names_unique() {
        let cfg = test_config();
        let capture = MemoryCaptureTool::new(cfg.clone());
        let retrieve = MemoryRetrieveTool::new(cfg.clone());
        let apply = MemoryApplyTool::new(cfg.clone());
        let learn = LearnCaptureTool::new(cfg);
        let names: Vec<&str> = vec![capture.name(), retrieve.name(), apply.name(), learn.name()];
        let mut deduped = names.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(names.len(), deduped.len(), "Tool names must be unique");
    }

    // -- JSON parsing tests -------------------------------------------------

    #[test]
    fn test_export_json_parsing() {
        let json = r#"{
            "agent": "cli-agent",
            "exported_at": "2026-08-11T18:47:22Z",
            "memory_items": [
                {
                    "id": "001",
                    "item_type": "Experience",
                    "content": "test memory content",
                    "importance": "Medium",
                    "tags": [],
                    "access_count": 0,
                    "created_at": "2026-01-01T00:00:00Z"
                }
            ],
            "lessons": [],
            "summary": { "memory_count": 1, "lesson_count": 0 }
        }"#;
        let export: MemoryExport = serde_json::from_str(json).unwrap();
        assert_eq!(export.memory_items.len(), 1);
        assert_eq!(export.memory_items[0].id, "001");
        assert_eq!(export.memory_items[0].content, "test memory content");
        assert_eq!(export.memory_items[0].importance, "Medium");
    }

    #[test]
    fn test_export_json_empty_items() {
        let json = r#"{
            "agent": "test",
            "exported_at": "2026-01-01T00:00:00Z",
            "memory_items": [],
            "lessons": [],
            "summary": { "memory_count": 0, "lesson_count": 0 }
        }"#;
        let export: MemoryExport = serde_json::from_str(json).unwrap();
        assert!(export.memory_items.is_empty());
    }

    #[test]
    fn test_export_json_unknown_fields() {
        let json = r#"{
            "agent": "test",
            "exported_at": "2026-01-01T00:00:00Z",
            "memory_items": [
                {
                    "id": "x",
                    "content": "hello",
                    "some_future_field": true,
                    "another_field": 42
                }
            ],
            "brand_new_field": "surprise",
            "lessons": [],
            "summary": {}
        }"#;
        let export: MemoryExport = serde_json::from_str(json).unwrap();
        assert_eq!(export.memory_items.len(), 1);
        assert_eq!(export.memory_items[0].content, "hello");
    }

    // -- Filter tests -------------------------------------------------------

    #[test]
    fn test_filter_items_case_insensitive() {
        let items = vec![MemoryItem {
            id: "1".to_string(),
            item_type: "Experience".to_string(),
            content: "Writing Rust code is fun".to_string(),
            importance: "Medium".to_string(),
            tags: vec![],
            access_count: 0,
            created_at: String::new(),
        }];
        let matches = filter_items(&items, "rust", 5);
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_filter_items_no_match() {
        let items = vec![MemoryItem {
            id: "1".to_string(),
            item_type: "Experience".to_string(),
            content: "Python is great".to_string(),
            importance: "Medium".to_string(),
            tags: vec![],
            access_count: 0,
            created_at: String::new(),
        }];
        let matches = filter_items(&items, "rust", 5);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_filter_items_respects_limit() {
        let items: Vec<MemoryItem> = (0..10)
            .map(|i| MemoryItem {
                id: i.to_string(),
                item_type: "Experience".to_string(),
                content: format!("match item {}", i),
                importance: "Medium".to_string(),
                tags: vec![],
                access_count: 0,
                created_at: String::new(),
            })
            .collect();
        let matches = filter_items(&items, "match", 3);
        assert_eq!(matches.len(), 3);
    }

    // -- Missing args tests -------------------------------------------------

    #[tokio::test]
    async fn test_missing_args_capture() {
        let tool = MemoryCaptureTool::new(test_config());
        let result = tool.execute(serde_json::json!({})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn test_missing_args_retrieve() {
        let tool = MemoryRetrieveTool::new(test_config());
        let result = tool.execute(serde_json::json!({})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn test_missing_args_apply() {
        let tool = MemoryApplyTool::new(test_config());
        let result = tool.execute(serde_json::json!({})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn test_missing_args_learn_capture() {
        let tool = LearnCaptureTool::new(test_config());
        let result = tool.execute(serde_json::json!({})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));

        // Also test with only one of two required params.
        let result = tool
            .execute(serde_json::json!({"command": "npm install"}))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }
}
