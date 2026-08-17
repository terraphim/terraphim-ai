# Design: #3144 TinyClaw memory/learning bridge

## Overview

This design wires four `terraphim-agent` CLI subcommands (`memory capture`, `memory retrieve`, `memory apply`, `learn capture`) into TinyClaw as first-class tools implementing the existing `Tool` trait (`tools/mod.rs:57-71`). Each tool shells out to the `terraphim-agent` binary via `tokio::process::Command` with timeout, following the `ShellTool` execution pattern (`shell.rs:72-119`). The tools share an `Arc<AgentMemoryConfig>` for binary path, role, and timeout settings. Registration is conditional on `config.memory.enabled` (default `false`), mirroring the `SessionTools` gating pattern (`mod.rs:176-179`). When memory is enabled, `agent_loop.rs` prepends retrieved memory context to the system prompt before each LLM call (`agent_loop.rs:607-612`), using a token-budget cap to prevent context overflow.

## Files to change

| File | Change |
|------|--------|
| `src/tools/agent_memory.rs` | **NEW** -- 4 tool structs + shared subprocess helper (~350 lines) |
| `src/tools/mod.rs` | Add `pub mod agent_memory;`; extend `create_default_registry` signature to accept `Option<&MemoryConfig>`; conditionally register the 4 tools (lines 156-183) |
| `src/config.rs` | Add `MemoryConfig` struct and `#[serde(default)] pub memory: MemoryConfig` field to `Config` (after line 24) |
| `src/agent/agent_loop.rs` | Add `memory_enabled: bool` + `memory_role: Option<String>` fields to `ToolCallingLoop`; inject memory context in `process_message` between lines 612-614 |
| `src/main.rs` | Pass `config.memory` to `create_default_registry` and propagate to `ToolCallingLoop::new` (lines 185-194) |
| `tests/agent_memory_contracts.rs` | **NEW** -- contract + integration tests with shim script |
| `tests/common/mod.rs` | No change required (existing `scrub_env` is sufficient) |

## Tool trait implementation

All four tools implement `Tool` (`tools/mod.rs:57-71`). Each holds `Arc<AgentMemoryConfig>` for shared config.

### Shared config struct

```rust
/// Configuration shared by all agent-memory bridge tools.
#[derive(Debug, Clone)]
pub struct AgentMemoryConfig {
    /// Path to the terraphim-agent binary. Defaults to "terraphim-agent"
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
```

### Shared subprocess helper

```rust
/// Execute terraphim-agent with the given args, optional stdin, and timeout.
/// Returns stdout on success. Maps errors to ToolError variants.
async fn run_agent(
    config: &AgentMemoryConfig,
    args: &[&str],
    stdin_data: Option<&str>,
) -> Result<String, ToolError>
```

Pattern (mirrors `shell.rs:72-119`):
1. `tokio::process::Command::new(&config.binary)` with `.args(args)`.
2. If `stdin_data` is `Some`, set `.stdin(Stdio::piped())`, spawn, write to stdin, then `wait_with_output()`.
3. If `stdin_data` is `None`, use `.stdout(Stdio::piped()).stderr(Stdio::piped()).output()`.
4. Wrap in `tokio::time::timeout(Duration::from_secs(config.timeout_secs), ...)`.
5. Map `io::Error` with `ErrorKind::NotFound` to `ToolError::ExecutionFailed { message: "terraphim-agent binary not found on PATH" }`.
6. Map timeout to `ToolError::Timeout`.
7. Map non-zero exit to `ToolError::ExecutionFailed` with stderr.

### Tool 1: `MemoryCaptureTool`

```rust
pub struct MemoryCaptureTool {
    config: Arc<AgentMemoryConfig>,
}

impl Tool for MemoryCaptureTool {
    fn name(&self) -> &str { "memory_capture" }

    fn description(&self) -> &str {
        "Capture a memory item into the terraphim-agent evolution store. \
         Accepts text content and an optional provenance tag."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
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

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        // Extract args
        // Build CLI: ["memory", "capture", "--provenance-tag", tag]
        // Pipe content JSON to stdin: {"content": "...", "item_type": "Experience"}
        // Return stdout (e.g. "Memory captured: <uuid>")
    }
}
```

**CLI invocation**: `terraphim-agent memory capture --provenance-tag <tag>` with JSON on stdin.

**Stdin JSON shape** (piped via `Stdio::piped()`):
```json
{"content": "<text>", "item_type": "Experience", "importance": "Medium"}
```

### Tool 2: `MemoryRetrieveTool`

```rust
pub struct MemoryRetrieveTool {
    config: Arc<AgentMemoryConfig>,
}

impl Tool for MemoryRetrieveTool {
    fn name(&self) -> &str { "memory_retrieve" }

    fn description(&self) -> &str {
        "Retrieve memory items matching a query from the terraphim-agent \
         evolution store. Returns JSON array of matching items."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        // Extract query, limit
        // Build CLI: ["memory", "export", "--format", "json"]
        // Parse JSON output as MemoryExport
        // Filter memory_items by query (case-insensitive substring on content)
        // Truncate to limit
        // Return JSON array of matching items
        // On empty: return "[]" (not an error)
    }
}
```

**CLI invocation**: `terraphim-agent memory export --format json`.

**Workaround**: `memory retrieve` lacks `--json` output (research.md:183-192). Use `memory export --format json` and filter client-side. This is a known short-term compromise; a prerequisite issue should add `--json` to `retrieve`.

### Tool 3: `MemoryApplyTool`

```rust
pub struct MemoryApplyTool {
    config: Arc<AgentMemoryConfig>,
}

impl Tool for MemoryApplyTool {
    fn name(&self) -> &str { "memory_apply" }

    fn description(&self) -> &str {
        "Retrieve relevant memories and format them for system-prompt injection. \
         Returns formatted memory context suitable for prepending to prompts."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        // Extract prompt
        // Build CLI: ["memory", "apply", "--prompt", prompt]
        // If role is set: add ["--role", role] (note: currently not supported
        // by apply, but forward-compatible)
        // Return raw stdout
        // On empty output: return "" (not an error, signals no memories)
    }
}
```

**CLI invocation**: `terraphim-agent memory apply --prompt <text>`.

**Note**: `memory apply` returns human-readable output only (research.md:185). The raw text is suitable for system-prompt injection as-is. When `--json` is added upstream, this tool should switch to structured parsing.

### Tool 4: `LearnCaptureTool`

```rust
pub struct LearnCaptureTool {
    config: Arc<AgentMemoryConfig>,
}

impl Tool for LearnCaptureTool {
    fn name(&self) -> &str { "learn_capture" }

    fn description(&self) -> &str {
        "Capture a failed command and its correction into the terraphim-agent \
         learning store for future reference."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        // Extract command, error, exit_code
        // Build CLI: ["learn", "capture", command, "--error", error,
        //             "--exit-code", exit_code.to_string()]
        // Return stdout (e.g. "Captured learning: <path>")
    }
}
```

**CLI invocation**: `terraphim-agent learn capture <command> --error <error> --exit-code <code>`.

## JSON parsing strategy

### Export format (used by `MemoryRetrieveTool`)

The `memory export --format json` output (research.md:137-155) is parsed with lenient serde structs. Only the fields we need are captured; unknown fields are ignored via `#[serde(deny_unknown_fields)]` NOT being set.

```rust
/// Top-level export envelope.
#[derive(Debug, Deserialize)]
pub(crate) struct MemoryExport {
    #[serde(default)]
    pub memory_items: Vec<MemoryItem>,
    // Other fields (agent, exported_at, lessons, summary) ignored.
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
```

### Client-side filtering (MemoryRetrieveTool)

Since `memory export` returns all items and `memory retrieve` lacks `--json`:

```rust
fn filter_items(items: &[MemoryItem], query: &str, limit: usize) -> Vec<&MemoryItem> {
    let query_lower = query.to_lowercase();
    items.iter()
        .filter(|item| item.content.to_lowercase().contains(&query_lower))
        .take(limit)
        .collect()
}
```

### Error handling

- **JSON parse failure**: If stdout is not valid JSON, return `ToolError::ExecutionFailed` with the raw output included for debugging. This handles CLI version mismatches gracefully.
- **Empty output**: `MemoryRetrieveTool` returns `"[]"`. `MemoryApplyTool` returns `""`. Neither is an error.
- **Partial JSON**: Use `serde_json::from_str::<MemoryExport>()` which ignores unknown fields by default. If the shape changes to include new top-level fields, parsing still succeeds.

## agent_loop integration

### Injection point

In `process_message()` (`agent_loop.rs:512`), between lines 612 (proxy_messages built) and 614 (tool definitions), insert memory context retrieval. This follows Option A from the research (research.md:72-92).

### Exact change

Add two fields to `ToolCallingLoop` (`agent_loop.rs:423-432`):

```rust
pub struct ToolCallingLoop {
    // ... existing fields ...
    /// Whether agent memory injection is enabled.
    memory_enabled: bool,
    /// Shared config for the agent-memory subprocess bridge.
    memory_config: Option<Arc<AgentMemoryConfig>>,
}
```

Update `ToolCallingLoop::new()` (`agent_loop.rs:436-456`) and `with_commands()` (`agent_loop.rs:459-480`) to accept `memory_config: Option<Arc<AgentMemoryConfig>>`:

```rust
pub fn new(
    agent_config: &AgentConfig,
    router: HybridLlmRouter,
    tools: Arc<ToolRegistry>,
    sessions: Arc<Mutex<SessionManager>>,
    system_prompt: String,
    memory_config: Option<Arc<AgentMemoryConfig>>,  // NEW
) -> Self { ... }
```

Insert memory context injection in `process_message()`, between the proxy_messages construction (line 612) and tool definitions (line 614):

```rust
// Line 612: proxy_messages built (existing)
let proxy_messages = { ... };

// NEW: Memory context injection
let effective_system_prompt = if self.memory_enabled {
    if let Some(ref mem_config) = self.memory_config {
        // Extract the user's latest message as the query
        let query = proxy_messages.iter().rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        match run_agent(mem_config, &["memory", "apply", "--prompt", query], None).await {
            Ok(context) if !context.trim().is_empty() => {
                // Token-budget guard: truncate to max_context_chars
                let truncated = if context.len() > mem_config.max_context_chars {
                    &context[..mem_config.max_context_chars]
                } else {
                    &context
                };
                format!("{}\n\n## Memory Context\n{}", self.system_prompt, truncated)
            }
            Ok(_) => self.system_prompt.clone(), // empty = no memories
            Err(e) => {
                log::warn!("Memory apply failed (non-fatal): {}", e);
                self.system_prompt.clone() // graceful degradation
            }
        }
    } else {
        self.system_prompt.clone()
    }
} else {
    self.system_prompt.clone()
};

// Line 614: tool definitions (existing, uses effective_system_prompt instead of self.system_prompt)
```

Then update line 631 and 635 to use `effective_system_prompt` instead of `self.system_prompt.clone()`:

```rust
// Line 630-637 (modified)
let final_response = if self.router.tools_available() && !tool_definitions.is_empty() {
    self.run_tool_loop_with_prompt(proxy_messages, tool_definitions, &effective_system_prompt).await?
} else {
    self.router.text_only(proxy_messages, Some(effective_system_prompt)).await?
};
```

### Token-budget guard

The `max_context_chars` field (default 4000, ~1000 tokens) caps the injected memory context. This prevents large memory stores from consuming the entire context window. The truncation is byte-safe because it operates on the formatted string after `memory apply` output.

### Why system prompt, not user message

The research (research.md:94-96) recommends Option A (system prompt injection) because:
1. System prompt tokens are stable across turns (not compressed away).
2. It mirrors how the existing system prompt file works (`config.rs:104-108`).
3. User-message injection (Option B) would consume token budget from the conversation window and risk being compressed in future turns.

## Config

### New struct

Add after `McpConfig` (`config.rs:999-1009`):

```rust
/// Agent memory bridge configuration. When `enabled = false` (the default),
/// no memory tools are registered and no memory context is injected.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemoryConfig {
    /// Master switch. `false` = memory bridge disabled.
    #[serde(default)]
    pub enabled: bool,

    /// Optional role for scoped memory retrieval. When set, memory
    /// retrieve/apply calls include `--role <role>`.
    #[serde(default)]
    pub role: Option<String>,

    /// Path to the terraphim-agent binary. Defaults to "terraphim-agent"
    /// (resolved via PATH lookup).
    #[serde(default = "default_agent_binary")]
    pub binary: String,

    /// Timeout for terraphim-agent subprocess calls in seconds.
    #[serde(default = "default_memory_timeout")]
    pub timeout_secs: u64,

    /// Maximum characters of memory context injected into the system
    /// prompt per request. Prevents token-budget overflow.
    #[serde(default = "default_max_context_chars")]
    pub max_context_chars: usize,
}

fn default_agent_binary() -> String {
    "terraphim-agent".to_string()
}

fn default_memory_timeout() -> u64 {
    10
}

fn default_max_context_chars() -> usize {
    4000
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            role: None,
            binary: default_agent_binary(),
            timeout_secs: default_memory_timeout(),
            max_context_chars: default_max_context_chars(),
        }
    }
}
```

### Addition to Config struct

Add to `Config` (`config.rs:6-25`), after the `mcp` field:

```rust
/// Agent memory bridge configuration. **Default: disabled.**
/// When `memory.enabled = true`, memory tools are registered and
/// memory context is injected into the system prompt.
#[serde(default)]
pub memory: MemoryConfig,
```

### TOML usage

```toml
[memory]
enabled = true
role = "Terraphim Engineer"
# binary = "/usr/local/bin/terraphim-agent"  # optional override
# timeout_secs = 10                          # optional override
# max_context_chars = 4000                   # optional override
```

### Conversion to AgentMemoryConfig

A `From<&MemoryConfig>` impl converts config to the runtime struct shared by tools:

```rust
impl From<&MemoryConfig> for AgentMemoryConfig {
    fn from(cfg: &MemoryConfig) -> Self {
        Self {
            binary: PathBuf::from(&cfg.binary),
            role: cfg.role.clone(),
            timeout_secs: cfg.timeout_secs,
            max_context_chars: cfg.max_context_chars,
        }
    }
}
```

## Test plan

### Unit tests (in `src/tools/agent_memory.rs`, `#[cfg(test)] mod tests`)

1. **`test_memory_capture_schema`** -- `MemoryCaptureTool::parameters_schema()` returns valid JSON Schema with `content` as required string.
2. **`test_memory_retrieve_schema`** -- `MemoryRetrieveTool::parameters_schema()` returns valid JSON Schema with `query` as required string, `limit` as optional integer.
3. **`test_learn_capture_schema`** -- `LearnCaptureTool::parameters_schema()` has `command` and `error` as required strings.
4. **`test_memory_apply_schema`** -- `MemoryApplyTool::parameters_schema()` has `prompt` as required string.
5. **`test_tool_names_unique`** -- All 4 tools have distinct `name()` values.
6. **`test_export_json_parsing`** -- Parse the verified export JSON shape (research.md:138-155) into `MemoryExport`. Assert `memory_items` length, field extraction.
7. **`test_export_json_empty_items`** -- Parse export with `"memory_items": []`. Assert empty vec, no error.
8. **`test_export_json_unknown_fields`** -- Parse export with extra fields not in the struct. Assert parsing succeeds (forward compatibility).
9. **`test_filter_items_case_insensitive`** -- `filter_items` matches "rust" against "Writing Rust code". Assert match.
10. **`test_filter_items_no_match`** -- `filter_items` returns empty vec when no content matches.
11. **`test_missing_args_returns_invalid_arguments`** -- Calling `execute(json!({}))` returns `ToolError::InvalidArguments`.

### Contract tests (in `tests/agent_memory_contracts.rs`)

All tests call `common::scrub_env()` as their first line.

1. **`contract_memory_tools_register_in_registry`** -- Create a `ToolRegistry`, register all 4 tools, assert `registry.len()` increases by 4 and each tool name is findable via `registry.get()`.

2. **`contract_memory_capture_with_shim`** -- Write a shell shim script to `tempdir/terraphim-agent` that echoes `"Memory captured: test-uuid\n  provenance_tag: tinyclaw"` when invoked with `memory capture`. Set `AgentMemoryConfig.binary` to the shim path. Call `MemoryCaptureTool.execute()` with `{"content": "test", "provenance_tag": "tc"}`. Assert result contains "Memory captured".

3. **`contract_memory_retrieve_with_shim`** -- Write a shim that echoes the verified export JSON (research.md:138-155) when invoked with `memory export --format json`. Call `MemoryRetrieveTool.execute()` with `{"query": "test"}`. Assert result is a JSON array containing matching items.

4. **`contract_learn_capture_with_shim`** -- Shim echoes `"Captured learning: /tmp/learn.json"`. Call `LearnCaptureTool.execute()` with `{"command": "npm install", "error": "not found", "exit_code": 127}`. Assert result contains "Captured learning".

5. **`contract_missing_binary_returns_error`** -- Set `AgentMemoryConfig.binary` to `/nonexistent/terraphim-agent`. Call any tool's `execute()`. Assert `ToolError::ExecutionFailed` with message containing "not found".

6. **`contract_timeout_returns_error`** -- Shim runs `sleep 999`. Set `timeout_secs: 1`. Assert `ToolError::Timeout`.

7. **`contract_capture_retrieve_round_trip`** -- Shim script maintains a simple JSON file: `memory capture` appends to it, `memory export --format json` reads it. Call `MemoryCaptureTool` then `MemoryRetrieveTool` and assert the captured content appears in the retrieved results.

### Shim approach

Each test writes a POSIX shell script to a `tempfile::tempdir()`:

```bash
#!/bin/sh
# Shim: terraphim-agent for memory capture test
case "$1 $2" in
  "memory capture")
    cat > /dev/null  # consume stdin
    echo "Memory captured: 00000000-0000-0000-0000-000000000001"
    echo "  provenance_tag: tinyclaw"
    ;;
  "memory export")
    cat <<'EOF'
{"agent":"test","exported_at":"2026-01-01T00:00:00Z","memory_items":[{"id":"001","item_type":"Experience","content":"test memory content","importance":"Medium","tags":[],"access_count":0,"created_at":"2026-01-01T00:00:00Z"}],"lessons":[],"summary":{"memory_count":1,"lesson_count":0}}
EOF
    ;;
  *)
    echo "unknown command: $*" >&2
    exit 1
    ;;
esac
```

The shim is made executable with `std::fs::set_permissions` and its path is used as `AgentMemoryConfig.binary`. This avoids requiring the real `terraphim-agent` binary or a live evolution store.

## Edge cases

### Missing binary

- **Detection**: `tokio::process::Command::new(binary).output()` returns `io::Error` with `ErrorKind::NotFound` when the binary does not exist.
- **Handling**: `run_agent()` maps this to `ToolError::ExecutionFailed { tool: "<tool_name>", message: "terraphim-agent binary not found at <path>. Install terraphim-agent or set memory.binary in config." }`.
- **Registration**: Tools are still registered (so the LLM sees them in the schema). The error is returned at call time, not registration time. This matches the issue requirement: "if terraphim-agent binary is missing at call time, return a structured error and skip for the session."
- **Session skip**: After the first `ExecutionFailed` from binary-not-found, the LLM receives the error as a tool result and will stop calling memory tools for that session. No explicit session-level disable flag is needed.

### Empty store

- `MemoryRetrieveTool`: When `memory export` returns `{"memory_items": []}`, the filter produces an empty vec. The tool returns `"[]"` (valid JSON array). The LLM sees this and knows there are no memories.
- `MemoryApplyTool`: When `memory apply` returns empty/whitespace output, the agent_loop injection skips prepending (the `!context.trim().is_empty()` guard at the injection point).
- Neither case is treated as an error.

### Huge memory dump

- `memory export --format json` could return a very large JSON blob if the store has many items.
- **Guard 1**: `MemoryRetrieveTool` caps results at `limit` (default 5, max 20).
- **Guard 2**: The agent_loop injection point caps at `max_context_chars` (default 4000).
- **Guard 3**: Individual items are not truncated at the tool level (the LLM handles its own output window), but the `max_context_chars` cap on the system-prompt injection provides a hard ceiling.

### JSON parse failure

- If the CLI output format changes between versions, `serde_json::from_str::<MemoryExport>()` will fail.
- The tool returns `ToolError::ExecutionFailed` with both the parse error and the raw stdout, giving the user actionable diagnostics.
- Forward compatibility: the `MemoryExport` struct uses `#[serde(default)]` on all non-essential fields and does not use `deny_unknown_fields`.

### Subprocess timeout

- All `run_agent` calls are wrapped in `tokio::time::timeout(Duration::from_secs(config.timeout_secs), ...)`, exactly as `ShellTool` does (`shell.rs:77-88`).
- Default timeout: 10 seconds. This is shorter than `ShellTool`'s 120 seconds because `terraphim-agent` operations should be fast (file I/O, not network).
- Timeout returns `ToolError::Timeout { tool, seconds }`.

### Stdin pipe for memory capture

- `memory capture` reads JSON from stdin (research.md:179, 309-310).
- `run_agent` handles this via the `stdin_data: Option<&str>` parameter.
- When `stdin_data` is `Some`, the command is spawned (not run to completion directly), stdin is written, stdin is dropped (sends EOF), then `wait_with_output()` collects stdout/stderr.
- The spawned process inherits no other stdin -- `Stdio::piped()` prevents it from reading the terminal.

### Concurrency

- Multiple concurrent tool calls to `terraphim-agent` could contend on the evolution store's file lock (research.md:313).
- The CLI handles locking internally; temporary contention results in a retry or a short block, not data corruption.
- No serialisation mutex is added in this design. If concurrent contention becomes a practical issue, a per-tool `tokio::sync::Semaphore(1)` can be added later without API changes.

### Non-zero exit code from terraphim-agent

- `run_agent` treats any non-zero exit as `ToolError::ExecutionFailed`, including stderr in the message.
- This covers cases where `terraphim-agent` reports errors (invalid flags, corrupt store, permission denied).

## Risks

1. **`memory retrieve`/`memory apply` lack `--json` flags** (research.md:187-193). The `MemoryRetrieveTool` works around this by using `memory export --format json` with client-side filtering. This is functionally correct but retrieves the entire store on every call. For small stores (<1000 items) this is acceptable; for large stores it could be slow. **Mitigation**: File a prerequisite issue to add `--json` to `retrieve`/`apply` in `terraphim_tui`. When those flags land, `MemoryRetrieveTool` switches to `memory retrieve --json <query>` (a 2-line change).

2. **Binary version drift**. The `MemoryExport` serde struct could diverge from the CLI's actual output if the CLI is upgraded independently. **Mitigation**: Lenient parsing (no `deny_unknown_fields`, `#[serde(default)]` everywhere). Contract tests pin the expected shape and will catch breakage.

3. **System prompt token budget**. Injecting memory context increases the system prompt size. With the default 4000-char cap (~1000 tokens), this consumes ~5-10% of a typical 16k-token context window. For models with smaller windows, operators should lower `max_context_chars`. **Mitigation**: The cap is configurable and documented.

4. **No structured error propagation to the session**. If the binary is missing, the LLM receives a tool-result error string. It may try calling the tool again on subsequent turns. **Mitigation**: The error message explicitly says "Install terraphim-agent or set memory.binary", guiding the LLM to stop retrying. If this proves insufficient, a future iteration could add a session-level disable flag set on first binary-not-found.

5. **Test shim portability**. POSIX shell shims work on Linux and macOS. On Windows (if TinyClaw is ever ported), the shims would need to be `.bat` or `.ps1` scripts. **Mitigation**: Out of scope for this issue; all CI runs on Linux/macOS.
