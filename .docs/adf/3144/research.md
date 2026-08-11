# Research: #3144 TinyClaw memory/learning bridge

## Tool trait contract

The `Tool` trait is defined in `crates/terraphim_tinyclaw/src/tools/mod.rs:57-71`:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError>;
}
```

Key observations:

- **Async**: Uses `async_trait` crate; `execute` is async, enabling `tokio::process::Command` calls.
- **Schema**: `parameters_schema()` returns a raw `serde_json::Value` in JSON Schema format (OpenAI function-calling style). The registry's `to_openai_tools()` (`:113-127`) wraps each tool in `{"type": "function", "function": {"name", "description", "parameters"}}`.
- **Return type**: `execute` returns `Result<String, ToolError>`. The `String` is the tool result fed back to the LLM. `ToolError` variants are: `NotFound`, `InvalidArguments`, `ExecutionFailed`, `Blocked`, `Timeout`, `Io`, `Json`.
- **Registration**: `ToolRegistry::register(&mut self, tool: Box<dyn Tool>)` at `:87`. Name is derived from `tool.name().to_string()`, stored in a `HashMap<String, Box<dyn Tool>>`.

## Existing tool examples

### ShellTool (`src/tools/shell.rs`)

The reference implementation for shell-bridge tools:

- **Struct**: `ShellTool { timeout_seconds: u64 }` -- stateless apart from config.
- **Execution pattern** (`:72-119`): Uses `tokio::process::Command::new("sh").arg("-c").arg(command)` with `Stdio::piped()` for stdout/stderr capture. Wraps in `tokio::time::timeout()`.
- **Safety**: `check_dangerous_patterns()` blocks destructive commands before execution.
- **Error mapping**: Non-zero exit codes map to `ToolError::ExecutionFailed`; timeout maps to `ToolError::Timeout`.
- **Output assembly**: Combines stdout + stderr into the return string.

### SessionTools (`src/tools/session_tools.rs`)

The reference for tools that hold shared state:

- **Pattern**: Each tool struct holds `Arc<Mutex<SessionManager>>`, passed at construction time.
- **Registration** (`mod.rs:176-179`): Conditionally registered when `sessions` is `Some`:
  ```rust
  if let Some(sessions) = sessions {
      registry.register(Box::new(SessionListTool::new(sessions.clone())));
      ...
  }
  ```
- **Schema shape**: JSON Schema objects with `properties`, `required`, `type`, `description`, `enum`, `default`, `minimum`, `maximum` fields.

### Default registry (`mod.rs:156-183`)

`create_default_registry` accepts:
- `sessions: Option<Arc<Mutex<SessionManager>>>` -- for session-aware tools
- `web_tools_config: Option<&WebToolsConfig>` -- for web tool configuration

The new memory tools will follow the same conditional-registration pattern, gated on `config.memory.enabled`.

## agent_loop integration point

### System prompt construction

The system prompt is a `String` field on `ToolCallingLoop` (`agent_loop.rs:431`), set at construction time in `ToolCallingLoop::new()` (`:436-456`). It originates from:

1. `config.agent.system_prompt_path()` -- reads a file (default: `workspace/SYSTEM.md`)
2. Passed through to `ToolCallingLoop::new()` as the final parameter (`main.rs:194`)

The system prompt is passed to every LLM call:
- `run_tool_loop` passes `Some(self.system_prompt.clone())` to `router.tool_call()` (`:678`)
- Text-only fallback also passes it (`:635`)

### Where memory context should be injected

**Option A -- Prepend to system prompt (recommended)**: In `process_message()` at `:606-612`, after building `proxy_messages` from the session and before calling `run_tool_loop`, inject a `memory_apply` call. The result would be prepended to `self.system_prompt` for that specific request. This is the cleanest point because:
1. It is after compression has occurred (`:566-604`)
2. It is before the LLM call (`:630-637`)
3. The system prompt is already cloned per-call (`:678`)

Concretely, the injection point is between lines 612 and 614:
```rust
// BUILD PROXY MESSAGES (existing, line 608-612)
let proxy_messages = { ... };

// NEW: memory context injection
let effective_system_prompt = if memory_enabled {
    let memory_context = memory_retrieve("current query context").await;
    format!("{}\n\n## Memory Context\n{}", self.system_prompt, memory_context)
} else {
    self.system_prompt.clone()
};

// GET TOOL DEFINITIONS (existing, line 615)
```

**Option B -- Inject as synthetic user message**: Add a `[Memory context]` user message at the start of `proxy_messages`, similar to how summaries are injected in `build_proxy_messages()` (`:396-420`). This is simpler but consumes token budget from the message window rather than the system prompt.

**Recommendation**: Option A. It mirrors how existing context (system prompt file) works, and system prompt tokens are generally more stable across turns.

### Tool result feedback

Tool results are fed back to the LLM in `run_tool_loop()` at `:741`:
```rust
messages.push(Message::tool(&tool_call.id, tool_result));
```

All four new tools (`memory_capture`, `memory_retrieve`, `memory_apply`, `learn_capture`) will return their results through this standard path. No special handling needed.

## terraphim-agent CLI surface

### `terraphim-agent memory` subcommands

| Subcommand | Arguments / Flags | Purpose |
|---|---|---|
| `capture` | `--provenance-tag <TAG>` (stdin: JSON) | Capture memory item; reads JSON from stdin |
| `retrieve` | `<QUERY>` `--role <ROLE>` | Search memory items by query within role scope |
| `apply` | `--prompt <PROMPT>` | Show what hooks would inject for a prompt |
| `list` | `--item-type <TYPE>` `--limit <N>` | List memory items (default limit 20) |
| `show` | `<ID>` `--json` | Show detail for one item; `--json` gives structured output |
| `export` | `--format json\|markdown` `--output <PATH>` | Bulk export |
| `validate` | (flags vary) | Validate against reliability rubric |
| `retire` | (flags vary) | Propose retirement of a memory item |
| `distill` | (none documented) | Distill learnings into KG |
| `rubric` | (none documented) | Full diagnostic |
| `second-run` | (none documented) | Token delta between ADF runs |

### `terraphim-agent learn` subcommands

| Subcommand | Arguments / Flags | Purpose |
|---|---|---|
| `capture` | `<COMMAND>` `--error <ERROR>` `--exit-code <N>` `--debug` | Capture a failed command |
| `list` | `--recent <N>` `--global` | List recent learnings |
| `query` | `<PATTERN>` `--exact` `--global` `--semantic` | Query learnings by pattern |
| `correct` | `<ID>` `<CORRECTION>` | Add correction to learning |
| `procedure` | (subcommands) | Manage captured procedures |

### JSON output shapes

**`memory export --format json`** (verified):
```json
{
  "agent": "cli-agent",
  "exported_at": "2026-08-11T18:47:22Z",
  "memory_items": [
    {
      "id": "uuid",
      "item_type": "Experience",
      "content": "string",
      "importance": "Medium",
      "tags": [],
      "access_count": 0,
      "created_at": "2026-07-01T10:55:54Z"
    }
  ],
  "lessons": [],
  "summary": { "memory_count": 2, "lesson_count": 0 }
}
```

**`memory show <ID> --json`** (verified):
```json
{
  "status": "ok",
  "action": "show",
  "id": "uuid",
  "memory_item": {
    "id": "uuid",
    "item_type": "Experience",
    "content": "string",
    "created_at": "ISO8601",
    "last_accessed": null,
    "access_count": 0,
    "importance": "Medium",
    "tags": [],
    "associations": {}
  },
  "lesson": null
}
```

**`memory capture`** (verified): Reads JSON from stdin, writes to evolution store. Output: `Memory captured: <uuid>\n  provenance_tag: <tag>`.

**`learn capture`** (verified): Positional `<COMMAND>` + `--error <ERROR>`. Output: `Captured learning: <file_path>`.

**`memory retrieve`** (verified): Human-readable output only. No `--json` flag currently exists. Output: `Memory retrieve: routing to search (role: Some("..."))\n  query: <q>`.

**`memory apply`** (verified): Human-readable output only. No `--json` flag. Output: `Memory apply: showing what hooks would inject for prompt\n  prompt: <text>`.

### Missing `--format json` / `--robot` flags

The issue spec mentions `--format json --robot` flags, but these do NOT exist on `memory retrieve` or `memory apply`. Only `memory show` has `--json` and `memory export` has `--format json`. The bridge tools will need to:
1. Use `memory export --format json` as a workaround for retrieve (filter client-side), OR
2. Parse human-readable output, OR
3. **Preferred**: Add `--json` flags to `retrieve` and `apply` in `terraphim_tui` first (prerequisite task).

## Config addition

The `Config` struct (`config.rs:5-25`) uses a flat section pattern with `#[serde(default)]` for optional sections. The `[memory]` section should follow the `CredentialsConfig` pattern:

```rust
// In config.rs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemoryConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub role: Option<String>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self { enabled: false, role: None }
    }
}
```

Add to `Config`:
```rust
pub struct Config {
    // ... existing fields ...
    #[serde(default)]
    pub memory: MemoryConfig,
}
```

TOML usage:
```toml
[memory]
enabled = true
role = "Terraphim Engineer"
```

This mirrors the `credentials` pattern exactly (`config.rs:897-993`): `enabled` flag defaults to `false`, optional fields with `#[serde(default)]`, `Default` impl, missing-section roundtrip test.

## Test patterns

### House style for contract tests

Contract tests live in `crates/terraphim_tinyclaw/tests/` as separate files. Key patterns observed:

1. **File naming**: `<feature>_contracts.rs` (e.g., `memory_contracts.rs`, `proxy_contracts.rs`, `acp_contracts.rs`, `mcp_contracts.rs`, `cron_contracts.rs`).

2. **Common module**: `tests/common/mod.rs` provides shared helpers like `scrub_env()`.

3. **Structure** (`memory_contracts.rs`):
   - Shared helper function for the test scenario (e.g., `round_trip_test`)
   - Factory functions for backends (e.g., `make_jsonl_backend`, `make_sqlite_backend`)
   - Tests named `contract_<behaviour>_<variant>` (e.g., `contract_jsonl_round_trip_preserves_messages`)
   - Use `#[tokio::test]` for async tests
   - Use `tempfile::tempdir()` for filesystem isolation
   - Use `uuid::Uuid` for unique identifiers
   - No mocks -- real implementations with in-memory or temp backends

4. **Inline unit tests**: Each tool module has `#[cfg(test)] mod tests` with simpler tests (e.g., `shell.rs:169-236`).

5. **Assertion style**: Direct `assert!`, `assert_eq!`, `assert!(matches!(...))`. String containment checks with `assert!(result.contains("..."))`.

6. **Credentials test pattern** (`credentials_pool_tests.rs`):
   - Custom `InMemorySource` implementing the trait for hermetic tests
   - `common::scrub_env()` before each test
   - Factory helpers: `fn entry(provider, class, env) -> PoolEntry`

### Recommended test approach for memory bridge tools

- **Unit tests**: In `src/tools/agent_memory.rs` `mod tests`, test:
  - Schema shape (`parameters_schema()` returns valid JSON Schema)
  - Tool name/description
  - Error handling for missing arguments (`ToolError::InvalidArguments`)
  - Binary-not-found error mapping

- **Contract tests**: New file `tests/agent_memory_contracts.rs`:
  - Test that tools register correctly in the registry
  - Test tool execution with a mock `terraphim-agent` script (shell script that echoes expected JSON)
  - Test JSON parsing of real CLI output shapes

## Risks / edge cases

### 1. Missing binary
`terraphim-agent` may not be on `PATH` in all deployment environments. The shell bridge must:
- Check binary existence at tool registration time or first call
- Return a clear `ToolError::ExecutionFailed` with message "terraphim-agent binary not found on PATH"
- Consider making the binary path configurable in `MemoryConfig`

### 2. Empty store
`memory retrieve` and `memory export` return empty results when no memories exist. The tools must handle:
- `memory_items: []` in export JSON
- Empty human-readable output from `retrieve`
- Return a helpful message like "No memories found for this query" rather than empty string

### 3. Token budget
Injecting memory context into the system prompt expands the token count. Risks:
- Large memory stores could exceed context window limits
- Memory context competes with the conversation summary for token budget
- **Mitigation**: Limit `memory retrieve` results (e.g., top 5 items), truncate individual items to ~200 tokens, and document the token budget impact in the `MemoryConfig`

### 4. JSON parse failures
The CLI output format may change between versions. Risks:
- `memory show --json` shape evolution
- `memory export` adding/removing fields
- **Mitigation**: Parse leniently with `serde_json::from_str::<Value>()` rather than strongly-typed structs. Extract only the fields needed.

### 5. Missing `--json` flags on `retrieve` and `apply`
As documented above, `memory retrieve` and `memory apply` lack `--json` output modes. Two strategies:
- **Short-term**: Use `memory export --format json` for retrieval (filter by query client-side), parse human-readable output for `apply`
- **Long-term**: Add `--json` flags to `terraphim_tui` CLI (separate prerequisite issue)

### 6. Subprocess timeout
`terraphim-agent` commands could hang (e.g., waiting for a lock on the evolution store). Must wrap all `Command` calls in `tokio::time::timeout()`, matching the `ShellTool` pattern (`shell.rs:77-88`).

### 7. Stdin pipe for `memory capture`
`memory capture` reads JSON from stdin. The tool must pipe input via `Command::new(...).stdin(Stdio::piped())` and write the JSON payload before awaiting output. This is slightly more complex than the other tools which use only command-line arguments.

### 8. Concurrency
Multiple concurrent tool calls to `terraphim-agent` could contend on the evolution store file lock. The CLI handles this internally, but rapid concurrent calls might cause temporary failures. Consider serialising memory operations or adding retry logic.

## Recommendation

### File layout

```
crates/terraphim_tinyclaw/src/tools/
  mod.rs              -- add `pub mod agent_memory;` + conditional registration
  agent_memory.rs     -- NEW: 4 tool structs implementing Tool trait
```

```
crates/terraphim_tinyclaw/tests/
  agent_memory_contracts.rs  -- NEW: contract tests
```

### Concrete file-level plan

1. **`src/tools/agent_memory.rs`** (new file, ~300 lines):
   - `AgentMemoryConfig` struct: `{ binary_path: PathBuf, role: Option<String>, timeout_secs: u64 }`
   - Shared helper: `async fn run_terraphim_agent(args: &[&str], stdin: Option<&str>, timeout: Duration) -> Result<String, ToolError>` -- wraps `tokio::process::Command` with timeout, stdout/stderr capture, exit-code checking
   - `MemoryCaptureTool` -- calls `terraphim-agent memory capture --provenance-tag <session_id>` with JSON on stdin
   - `MemoryRetrieveTool` -- calls `terraphim-agent memory export --format json`, filters by query client-side (until `--json` flag added to `retrieve`)
   - `MemoryApplyTool` -- calls `terraphim-agent memory apply --prompt <text>`, returns raw output
   - `LearnCaptureTool` -- calls `terraphim-agent learn capture <cmd> --error <err> --exit-code <code>`
   - Each struct holds `Arc<AgentMemoryConfig>` for shared configuration

2. **`src/tools/mod.rs`** changes:
   - Add `pub mod agent_memory;`
   - In `create_default_registry`, add a new parameter `memory_config: Option<&MemoryConfig>` (or accept the full `Config`)
   - Conditionally register all 4 tools when `memory_config.enabled`

3. **`src/config.rs`** changes:
   - Add `MemoryConfig` struct with `enabled: bool` and `role: Option<String>`
   - Add `#[serde(default)] pub memory: MemoryConfig` to `Config`

4. **`src/agent/agent_loop.rs`** changes:
   - Add `memory_config: MemoryConfig` field to `ToolCallingLoop`
   - In `process_message()`, between line 612 (proxy_messages built) and line 614 (tool definitions), if `memory_config.enabled`, call `memory retrieve` via the shell bridge and prepend result to system prompt for that request
   - Alternative: the `MemoryApplyTool` can be invoked by the LLM itself as a regular tool call (no agent_loop modification needed if we trust the LLM to use it). Both approaches have merit; the automatic injection is more reliable.

5. **`src/main.rs`** changes:
   - Pass `config.memory` to `create_default_registry` and `ToolCallingLoop::new`

6. **`tests/agent_memory_contracts.rs`** (new file):
   - Contract: each tool registers and has valid schema
   - Contract: `MemoryCaptureTool` with mock script echoing capture output
   - Contract: `MemoryRetrieveTool` parses export JSON correctly
   - Contract: `LearnCaptureTool` handles non-zero exit codes from the CLI

### Prerequisite issue

Consider filing a separate issue to add `--json` output flags to `terraphim-agent memory retrieve` and `terraphim-agent memory apply` in `crates/terraphim_tui`. This would simplify the bridge tools and avoid fragile human-readable output parsing.

### Relationship to existing `memory/` module

The existing `memory/` module (`src/memory/mod.rs`) defines `MemoryBackend` trait for **session storage** (JSONL, SQLite). This is a different concern from the `terraphim-agent` memory lifecycle (capture/retrieve/apply). The new `agent_memory.rs` should sit in `src/tools/` alongside other tools, NOT in `src/memory/`. The naming distinction is:
- `memory/` = session persistence backend (where TinyClaw stores chat history)
- `tools/agent_memory.rs` = bridge to `terraphim-agent`'s cross-session memory/learning system
