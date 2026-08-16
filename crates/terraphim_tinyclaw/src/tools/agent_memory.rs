//! Bridge tools for `terraphim-agent memory` and `terraphim-agent learn`
//! CLI subcommands.
//!
//! Each tool shells out to the `terraphim-agent` binary via
//! `tokio::process::Command`, following the same subprocess pattern as
//! [`ShellTool`](super::shell::ShellTool).
//!
//! Retrieval is ranked and role-scoped (issue #3226): `memory_retrieve`
//! prefers `terraphim-agent memory retrieve --format json` (upstream
//! ranked retrieval) and falls back to `memory export --format json`
//! with a local BM25 ranker plus exact-phrase boost when the installed
//! binary lacks the flag.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use terraphim_types::score::OkapiBM25Scorer;
use terraphim_types::{Document, DocumentType};
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
    run_agent_in_dir(config, args, stdin_data, None).await
}

/// Execute `terraphim-agent` with an explicit working directory.
///
/// `terraphim-agent learn capture` resolves its project learnings store
/// from the process working directory (`<cwd>/.terraphim/learnings`), so
/// the agent loop pins `work_dir` to the configured agent workspace to
/// keep captured learnings project-local (#3225). `None` inherits the
/// current process directory (existing behaviour for all other calls).
pub(crate) async fn run_agent_in_dir(
    config: &AgentMemoryConfig,
    args: &[&str],
    stdin_data: Option<&str>,
    work_dir: Option<&std::path::Path>,
) -> Result<String, ToolError> {
    let result = timeout(
        Duration::from_secs(config.timeout_secs),
        run_agent_inner(config, args, stdin_data, work_dir),
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
    work_dir: Option<&std::path::Path>,
) -> Result<String, ToolError> {
    let output = if let Some(data) = stdin_data {
        // Spawn, pipe stdin, then collect output.
        let mut command = Command::new(&config.binary);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = work_dir {
            command.current_dir(dir);
        }
        let mut child = command
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
        let mut command = Command::new(&config.binary);
        command
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = work_dir {
            command.current_dir(dir);
        }
        command
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
// Invariant failure capture (#3225)
// ---------------------------------------------------------------------------

/// Glob patterns for commands that must never be auto-captured as
/// learnings. Test runners fail routinely (that is their job), so
/// capturing them would flood the learning store with noise. Mirrors
/// `LearningCaptureConfig::default().ignore_patterns` in
/// `terraphim-agent`; the CLI applies the same list server-side, this
/// client-side check avoids spawning a subprocess at all.
pub(crate) const CAPTURE_IGNORE_PATTERNS: &[&str] =
    &["cargo test*", "npm test*", "pytest*", "yarn test*"];

/// Minimal glob matcher supporting the `*` wildcard (any run of
/// characters, including empty). Sufficient for the ignore list above
/// and avoids a new dependency for four patterns.
pub(crate) fn glob_matches(pattern: &str, text: &str) -> bool {
    let segments: Vec<&str> = pattern.split('*').collect();
    if segments.len() == 1 {
        return pattern == text;
    }

    let mut rest = text;
    // The first segment anchors at the start of the text.
    let first = segments[0];
    if !rest.starts_with(first) {
        return false;
    }
    rest = &rest[first.len()..];

    // Middle segments must appear in order.
    for segment in &segments[1..segments.len() - 1] {
        match rest.find(segment) {
            Some(idx) => rest = &rest[idx + segment.len()..],
            None => return false,
        }
    }

    // The last segment anchors at the end (empty when the pattern ends
    // with '*', which matches any suffix).
    let last = segments[segments.len() - 1];
    last.is_empty() || rest.ends_with(last)
}

/// Check whether a command matches the capture ignore list.
pub(crate) fn should_ignore_command(command: &str) -> bool {
    let trimmed = command.trim_start();
    CAPTURE_IGNORE_PATTERNS
        .iter()
        .any(|pattern| glob_matches(pattern, trimmed))
}

/// Invoke `terraphim-agent learn capture` for a failed command.
///
/// Shared by [`LearnCaptureTool`] (model-elected capture) and the agent
/// loop's invariant capture path (#3225). Secret redaction is delegated
/// to `terraphim-agent`, which redacts before persisting. The error
/// output is truncated to [`MAX_OUTPUT_BYTES`] before being passed to
/// the subprocess. Respects the configured subprocess timeout via
/// [`run_agent`].
///
/// `work_dir` pins the learnings store location (see
/// [`run_agent_in_dir`]); `None` inherits the process directory.
pub(crate) async fn capture_failed_command(
    config: &AgentMemoryConfig,
    command: &str,
    error: &str,
    exit_code: i64,
    work_dir: Option<&std::path::Path>,
) -> Result<String, ToolError> {
    let error_truncated = truncate_chars(error, MAX_OUTPUT_BYTES);
    let exit_code_str = exit_code.to_string();
    let cli_args = vec![
        "learn",
        "capture",
        command,
        "--error",
        error_truncated,
        "--exit-code",
        &exit_code_str,
    ];
    run_agent_in_dir(config, &cli_args, None, work_dir).await
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

// ---------------------------------------------------------------------------
// Ranked hybrid retrieval (issue #3226)
// ---------------------------------------------------------------------------

/// Additive score boost when the full query phrase appears verbatim
/// (case-insensitive) in an item's content. Mirrors the concept boost in
/// `terraphim_sessions::search` (`KG_BOOST_MULTIPLIER`): an exact-concept
/// match must outrank items that merely share a term.
const EXACT_PHRASE_BOOST: f64 = 10.0;

/// Tag prefix marking an item as belonging to a role (`role:<name>`).
/// Items without any such marker are shared across roles.
const ROLE_TAG_PREFIX: &str = "role:";

/// Clamp the caller-supplied limit to the documented contract:
/// 1..=20, default 5.
pub(crate) fn clamp_limit(limit: Option<u64>) -> usize {
    limit.unwrap_or(5).clamp(1, 20) as usize
}

/// Normalise free text into lowercase alphanumeric tokens separated by
/// single spaces. `OkapiBM25Scorer` tokenises on whitespace only, so
/// punctuation must be stripped here for `rust,` to match `rust`.
pub(crate) fn normalise_tokens(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_space = true;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
            last_was_space = false;
        } else if !last_was_space {
            out.push(' ');
            last_was_space = true;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Role markers carried by an item: the `<name>` part of each
/// `role:<name>` tag (prefix matched case-insensitively).
fn item_role_markers(item: &MemoryItem) -> Vec<&str> {
    item.tags
        .iter()
        .filter_map(|tag| {
            if tag.len() > ROLE_TAG_PREFIX.len()
                && tag[..ROLE_TAG_PREFIX.len()].eq_ignore_ascii_case(ROLE_TAG_PREFIX)
            {
                Some(&tag[ROLE_TAG_PREFIX.len()..])
            } else {
                None
            }
        })
        .collect()
}

/// An item is visible to `role` when it carries no `role:` markers at all
/// (shared memory) or at least one marker matches case-insensitively.
pub(crate) fn in_role_scope(item: &MemoryItem, role: Option<&str>) -> bool {
    let Some(role) = role else {
        return true;
    };
    let markers = item_role_markers(item);
    markers.is_empty() || markers.iter().any(|m| m.eq_ignore_ascii_case(role))
}

/// Convert a memory item into a `terraphim_types::Document` for BM25
/// scoring. The body is the normalised content so the
/// whitespace-tokenising scorer sees clean terms.
fn memory_item_to_document(item: &MemoryItem) -> Document {
    Document {
        id: item.id.clone(),
        url: String::new(),
        title: item.id.clone(),
        body: normalise_tokens(&item.content),
        description: None,
        summarization: None,
        stub: None,
        tags: if item.tags.is_empty() {
            None
        } else {
            Some(item.tags.clone())
        },
        rank: None,
        source_haystack: None,
        doc_type: DocumentType::default(),
        synonyms: None,
        route: None,
        priority: None,
        quality_score: None,
    }
}

/// Rank `items` against `query` with BM25 over tokenised content plus an
/// exact-phrase boost, scoped to `role`. Returns at most `limit` items,
/// best first. Items with no term overlap and no phrase match score zero
/// and are dropped: an incidental substring no longer surfaces.
pub(crate) fn rank_items<'a>(
    items: &'a [MemoryItem],
    query: &str,
    role: Option<&str>,
    limit: usize,
) -> Vec<&'a MemoryItem> {
    if query.trim().is_empty() || items.is_empty() || limit == 0 {
        return Vec::new();
    }

    let scoped: Vec<&MemoryItem> = items
        .iter()
        .filter(|item| in_role_scope(item, role))
        .collect();

    let documents: Vec<Document> = scoped.iter().map(|i| memory_item_to_document(i)).collect();

    let mut scorer = OkapiBM25Scorer::new();
    scorer.initialize(&documents);

    let normalised_query = normalise_tokens(query);
    let phrase = query.trim().to_lowercase();

    let mut scored: Vec<(f64, usize)> = documents
        .iter()
        .enumerate()
        .map(|(idx, doc)| {
            let mut score = scorer.score(&normalised_query, doc);
            if !phrase.is_empty() && scoped[idx].content.to_lowercase().contains(&phrase) {
                score += EXACT_PHRASE_BOOST;
            }
            (score, idx)
        })
        .filter(|(score, _)| *score > 0.0)
        .collect();

    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            // Stable tie-break on original position.
            .then(a.1.cmp(&b.1))
    });

    scored
        .into_iter()
        .take(limit)
        .map(|(_, idx)| scoped[idx])
        .collect()
}

/// Lenient payload shape for `memory retrieve --format json` once the
/// upstream flag lands (companion change filed as a follow-up to #3226).
/// Accepts either a bare array of items or an envelope object carrying
/// `memory_items` (alias `items`).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum RetrievePayload {
    Items(Vec<MemoryItem>),
    Envelope(RetrieveEnvelope),
}

/// Envelope variant of [`RetrievePayload`].
#[derive(Debug, Deserialize)]
pub(crate) struct RetrieveEnvelope {
    #[serde(default, alias = "items")]
    pub memory_items: Vec<MemoryItem>,
}

/// Parse the stdout of `memory retrieve --format json`. Returns `None`
/// when the output is not recognisable JSON, signalling the caller to
/// fall back to the export path.
pub(crate) fn parse_retrieve_items(raw: &str) -> Option<Vec<MemoryItem>> {
    match serde_json::from_str::<RetrievePayload>(raw.trim()).ok()? {
        RetrievePayload::Items(items) => Some(items),
        RetrievePayload::Envelope(envelope) => Some(envelope.memory_items),
    }
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

    /// Attempt ranked retrieval via `memory retrieve --format json`.
    ///
    /// Returns `None` when the installed binary lacks the flag or the
    /// output is not parseable JSON, signalling the caller to fall back
    /// to the export path with local ranking.
    async fn retrieve_upstream(&self, query: &str, limit: usize) -> Option<Vec<MemoryItem>> {
        let limit_str = limit.to_string();
        let mut cli_args = vec![
            "memory",
            "retrieve",
            query,
            "--limit",
            &limit_str,
            "--format",
            "json",
        ];

        // Pass the configured role through to retrieval, not only
        // capture/apply (#3226).
        let role_owned;
        if let Some(ref role) = self.config.role {
            role_owned = role.clone();
            cli_args.push("--role");
            cli_args.push(&role_owned);
        }

        let raw = match run_agent(&self.config, &cli_args, None).await {
            Ok(raw) => raw,
            Err(e) => {
                tracing::debug!(
                    "memory retrieve --format json unavailable, falling back to export: {}",
                    e
                );
                return None;
            }
        };

        let mut items = match parse_retrieve_items(&raw) {
            Some(items) => items,
            None => {
                tracing::debug!(
                    "memory retrieve output not parseable as JSON, falling back to export"
                );
                return None;
            }
        };

        // Defence in depth: enforce role scope and the limit client-side
        // even when the binary claims to handle them.
        items.retain(|item| in_role_scope(item, self.config.role.as_deref()));
        items.truncate(limit);
        Some(items)
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

        let limit = clamp_limit(args["limit"].as_u64());

        // Primary path: ranked, role-scoped retrieval upstream. Requires
        // `--format json` on `memory retrieve` (companion change filed as
        // a follow-up to #3226).
        if let Some(items) = self.retrieve_upstream(query, limit).await {
            let json =
                serde_json::to_string(&items).map_err(|e| ToolError::ExecutionFailed {
                    tool: "memory_retrieve".to_string(),
                    message: format!("Failed to serialise results: {}", e),
                })?;
            return Ok(json);
        }

        // Fallback path: `memory export --format json` plus local BM25
        // ranking with role scoping. Ranked, never a raw substring scan.
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

        let matches = rank_items(
            &export.memory_items,
            query,
            self.config.role.as_deref(),
            limit,
        );
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

        capture_failed_command(&self.config, command, error, exit_code, None).await
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_matches() {
        assert!(glob_matches("cargo test*", "cargo test"));
        assert!(glob_matches("cargo test*", "cargo test --lib"));
        assert!(glob_matches("pytest*", "pytest tests/"));
        assert!(!glob_matches("cargo test*", "cargo build"));
        // Glob prefix semantics: matches, same as glob::Pattern in terraphim-agent.
        assert!(glob_matches("cargo test*", "cargo testing"));
        assert!(glob_matches("exact", "exact"));
        assert!(!glob_matches("exact", "exactly"));
        assert!(glob_matches("*test*", "my test run"));
        assert!(!glob_matches("*test*", "my run"));
        assert!(glob_matches("a*b*c", "aXbYc"));
        assert!(!glob_matches("a*b*c", "aXbY"));
    }

    #[test]
    fn test_should_ignore_command() {
        assert!(should_ignore_command("cargo test --lib"));
        assert!(should_ignore_command("  cargo test"));
        assert!(should_ignore_command("npm test"));
        assert!(should_ignore_command("pytest tests/"));
        assert!(should_ignore_command("yarn test --watch"));
        assert!(!should_ignore_command("ls /nonexistent"));
        assert!(!should_ignore_command("cargo build"));
        assert!(!should_ignore_command("git push"));
    }

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

    // -- Ranking tests (#3226) ----------------------------------------------

    fn fixture_item(id: &str, content: &str, tags: Vec<&str>) -> MemoryItem {
        MemoryItem {
            id: id.to_string(),
            item_type: "Experience".to_string(),
            content: content.to_string(),
            importance: "Medium".to_string(),
            tags: tags.into_iter().map(str::to_string).collect(),
            access_count: 0,
            created_at: String::new(),
        }
    }

    #[test]
    fn test_normalise_tokens_strips_punctuation_and_case() {
        assert_eq!(normalise_tokens("Rust, cargo!  NEXTEST."), "rust cargo nextest");
        assert_eq!(normalise_tokens(""), "");
        assert_eq!(normalise_tokens("---"), "");
    }

    #[test]
    fn test_rank_items_case_insensitive() {
        let items = vec![fixture_item("1", "Writing Rust code is fun", vec![])];
        let matches = rank_items(&items, "rust", None, 5);
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_rank_items_exact_concept_outranks_incidental_substring() {
        // "incidental" contains the query as a pure substring
        // ("esc**argo nextest**imonial") but shares no token with it; the
        // old substring filter would have surfaced it. Ranked retrieval
        // must put the exact-concept match first and drop the incidental
        // one entirely.
        let items = vec![
            fixture_item(
                "incidental",
                "The escargot nextestimonial dinner was lovely",
                vec![],
            ),
            fixture_item(
                "exact",
                "Use cargo nextest for faster Rust test runs in CI",
                vec![],
            ),
        ];
        let matches = rank_items(&items, "cargo nextest", None, 5);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].id, "exact");
        assert!(
            matches.iter().all(|m| m.id != "incidental"),
            "incidental substring match must not be returned"
        );
    }

    #[test]
    fn test_rank_items_no_match_returns_empty() {
        let items = vec![fixture_item("1", "Python is great", vec![])];
        let matches = rank_items(&items, "rust", None, 5);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_rank_items_respects_limit() {
        let items: Vec<MemoryItem> = (0..10)
            .map(|i| fixture_item(&i.to_string(), &format!("match item {}", i), vec![]))
            .collect();
        let matches = rank_items(&items, "match", None, 3);
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_rank_items_empty_query_returns_empty() {
        let items = vec![fixture_item("1", "anything at all", vec![])];
        assert!(rank_items(&items, "", None, 5).is_empty());
        assert!(rank_items(&items, "   ", None, 5).is_empty());
    }

    #[test]
    fn test_rank_items_role_scoping() {
        let items = vec![
            fixture_item("dev", "testing strategy for the backend", vec!["role:developer"]),
            fixture_item("rev", "testing checklist for reviewers", vec!["role:reviewer"]),
            fixture_item("shared", "testing is everyone's job", vec![]),
        ];

        // No role: everything is in scope.
        let all = rank_items(&items, "testing", None, 10);
        assert_eq!(all.len(), 3);

        // Developer role: reviewer-tagged item is excluded, shared stays.
        let dev = rank_items(&items, "testing", Some("developer"), 10);
        assert!(dev.iter().any(|m| m.id == "dev"));
        assert!(dev.iter().any(|m| m.id == "shared"));
        assert!(
            dev.iter().all(|m| m.id != "rev"),
            "reviewer-scoped item must not leak into developer retrieval"
        );
    }

    #[test]
    fn test_in_role_scope_marker_matching() {
        let dev = fixture_item("a", "x", vec!["role:Developer"]);
        assert!(in_role_scope(&dev, Some("developer"))); // case-insensitive
        assert!(!in_role_scope(&dev, Some("reviewer")));
        assert!(in_role_scope(&dev, None));
        let shared = fixture_item("b", "x", vec!["testing"]);
        assert!(in_role_scope(&shared, Some("reviewer")));
    }

    #[test]
    fn test_clamp_limit_contract() {
        assert_eq!(clamp_limit(None), 5, "default is 5");
        assert_eq!(clamp_limit(Some(0)), 1, "minimum is 1");
        assert_eq!(clamp_limit(Some(7)), 7);
        assert_eq!(clamp_limit(Some(20)), 20);
        assert_eq!(clamp_limit(Some(21)), 20, "maximum is 20");
        assert_eq!(clamp_limit(Some(u64::MAX)), 20);
    }

    #[test]
    fn test_parse_retrieve_items_bare_array() {
        let raw = r#"[{"id":"1","content":"hello"}]"#;
        let items = parse_retrieve_items(raw).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "1");
    }

    #[test]
    fn test_parse_retrieve_items_envelope() {
        let raw = r#"{"memory_items":[{"id":"2","content":"world"}],"query":"w"}"#;
        let items = parse_retrieve_items(raw).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "2");

        let raw_alias = r#"{"items":[{"id":"3","content":"alias"}]}"#;
        let items = parse_retrieve_items(raw_alias).unwrap();
        assert_eq!(items[0].id, "3");
    }

    #[test]
    fn test_parse_retrieve_items_rejects_human_readable_output() {
        assert!(parse_retrieve_items("Memory retrieve: routing to search").is_none());
        assert!(parse_retrieve_items("").is_none());
    }

    #[test]
    fn test_rank_items_from_tempdir_fixture_store() {
        // Fixture learnings persisted in a tempdir store: the ranking
        // pipeline runs over exactly what a real `memory export` file
        // would contain, with no mocked collaborators.
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("memory-export.json");
        let fixture = r#"{
            "agent": "fixture",
            "exported_at": "2026-08-16T00:00:00Z",
            "memory_items": [
                {"id": "a", "item_type": "Experience",
                 "content": "Always run cargo nextest before pushing",
                 "importance": "High", "tags": ["role:developer"],
                 "access_count": 3, "created_at": "2026-08-01T00:00:00Z"},
                {"id": "b", "item_type": "Experience",
                 "content": "The cargo ship docked next to the test pier",
                 "importance": "Low", "tags": [],
                 "access_count": 0, "created_at": "2026-08-02T00:00:00Z"},
                {"id": "c", "item_type": "Lesson",
                 "content": "Reviewers run cargo nextest with --all-features",
                 "importance": "Medium", "tags": ["role:reviewer"],
                 "access_count": 1, "created_at": "2026-08-03T00:00:00Z"}
            ],
            "lessons": [],
            "summary": {"memory_count": 3, "lesson_count": 0}
        }"#;
        std::fs::write(&store_path, fixture).unwrap();

        let raw = std::fs::read_to_string(&store_path).unwrap();
        let export: MemoryExport = serde_json::from_str(&raw).unwrap();
        assert_eq!(export.memory_items.len(), 3);

        let ranked = rank_items(&export.memory_items, "cargo nextest", Some("developer"), 5);
        assert_eq!(ranked[0].id, "a", "exact concept match must rank first");
        assert!(
            ranked.iter().all(|m| m.id != "c"),
            "reviewer-scoped item must be excluded for developer role"
        );
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
