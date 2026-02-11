# Implementation Plan: TinyClaw on Terraphim (terraphim_tinyclaw)

**Status**: Draft
**Research Doc**: [tinyclaw-terraphim-research.md](tinyclaw-terraphim-research.md)
**Author**: Terraphim AI Design
**Date**: 2026-02-11
**Estimated Effort**: ~5,700 LOC Rust + ~400 LOC TypeScript (WhatsApp bridge)

---

## Overview

### Summary

Build a multi-channel AI assistant binary (`terraphim-tinyclaw`) on the Terraphim agent crate ecosystem. The assistant connects to Telegram, Discord, and WhatsApp, routes user messages through a tool-calling agent loop with context compression, and responds via the originating channel. A CLI agent mode enables direct interaction for development and testing.

### Approach

New crate `terraphim_tinyclaw` as a binary crate in the workspace. It depends on existing agent crates (`terraphim_agent_messaging`, `terraphim_multi_agent`, `terraphim_agent_evolution`, `terraphim_service`) and introduces channel adapters, a tool registry, a skills loader, and an orchestrator.

The design follows PicoClaw's proven architecture -- channel trait, message bus, agent loop, session manager -- adapted to Rust async idioms and Terraphim's existing infrastructure.

### Scope

**In Scope (Phase 1 MVP):**
1. Channel abstraction trait + message bus
2. Telegram adapter (most complex channel, proves the abstraction)
3. Discord adapter
4. Tool-calling agent loop with iterative LLM calls
5. Context compression via LLM summarization
6. Session manager with JSONL persistence
7. Tool registry with 5 tools (filesystem, shell, web_search, web_fetch, edit)
8. CLI agent mode for direct interaction
9. Configuration with per-channel allow-lists
10. Markdown-to-platform formatting (Telegram HTML, Discord markdown)

**Out of Scope (Phase 2+):**
- WhatsApp bridge (requires Node.js subprocess)
- Feishu/Lark adapter
- Slack adapter
- Email adapter
- Voice transcription (Groq Whisper)
- Skills system (markdown-based)
- Cron/scheduled tasks
- Onboarding CLI wizard
- Subagent spawning
- Knowledge graph enrichment of responses
- Firecracker VM sandboxed execution

**Avoid At All Cost (5/25 Rule):**
- MaixCam hardware channel (PicoClaw-specific niche)
- MoChat channel (niche platform)
- DingTalk channel (niche platform)
- QQ channel (can reuse Telegram pattern later)
- LiteLLM-style provider registry (Terraphim already has LlmClient trait)
- Matrix bridge for WhatsApp (unproven approach)
- Custom UI/dashboard
- Multi-agent routing workflows (premature for chat assistant)
- Goal alignment integration (premature)
- Task decomposition integration (premature)

---

## Architecture

### Component Diagram

```
                   terraphim_tinyclaw binary
  +---------------------------------------------------------+
  |                                                         |
  |  +----------+  +----------+  +----------+  +--------+  |
  |  | Telegram |  | Discord  |  | CLI      |  | (more  |  |
  |  | Adapter  |  | Adapter  |  | Adapter  |  | later) |  |
  |  +----+-----+  +----+-----+  +----+-----+  +--------+  |
  |       |              |              |                    |
  |       v              v              v                    |
  |  +-------------------------------------------+         |
  |  |        MessageBus (tokio::mpsc)            |         |
  |  |  inbound_tx/rx    outbound_tx/rx           |         |
  |  +-------------------+---+--------------------+         |
  |                       |   |                             |
  |             +---------+   +--------+                    |
  |             v                      v                    |
  |  +-------------------+  +-------------------+          |
  |  |   AgentLoop       |  | OutboundDispatch  |          |
  |  |   - tool calling  |  | - routes by       |          |
  |  |   - LLM calls     |  |   channel name    |          |
  |  |   - compression   |  +-------------------+          |
  |  +--------+----------+                                  |
  |           |                                             |
  |    +------+------+                                      |
  |    v      v      v                                      |
  | +------+ +----+ +----------+                            |
  | |Tools | |Sess| |LlmClient|                            |
  | |Reg.  | |Mgr | |(existing)|                            |
  | +------+ +----+ +----------+                            |
  +---------------------------------------------------------+

  External deps: teloxide, serenity, reqwest, tokio
  Internal deps: terraphim_service, terraphim_agent_evolution,
                 terraphim_agent_messaging
```

### Data Flow

```
User message on Telegram
  -> TelegramAdapter.handle_message()
  -> InboundMessage { channel: "telegram", sender_id, chat_id, content }
  -> is_allowed() check (allow-list)
  -> bus.inbound_tx.send(msg)
  -> AgentLoop.consume_inbound()
  -> session = SessionManager.get_or_create(session_key)
  -> check context compression trigger
  -> build messages: [system_prompt, summary?, history, user_msg]
  -> for i in 0..max_iterations:
       response = llm_client.chat_with_tools(messages, tools)
       if no tool_calls: break
       execute tools, append results
  -> session.add_messages(user + assistant)
  -> session_manager.save(session)
  -> OutboundMessage { channel: "telegram", chat_id, content: response }
  -> bus.outbound_tx.send(msg)
  -> OutboundDispatcher routes to TelegramAdapter.send()
  -> User receives response
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| New binary crate, not library | Binary is the deliverable; library abstractions live in existing crates | Modifying terraphim_server (wrong responsibility) |
| `tokio::mpsc` for bus, not AgentMailbox | Simpler, proven pattern from PicoClaw; AgentMailbox is overkill for channel<->agent routing | AgentMailbox (Erlang-style, too complex for this) |
| `teloxide` for Telegram | Most mature Rust Telegram library, async, well-documented | Raw HTTP (more work), `frankenstein` (less mature) |
| `serenity` for Discord | Batteries-included, good for first implementation | `twilight` (lighter but harder to start with) |
| JSONL session files | Proven by nanobot, append-friendly, human-readable | SQLite (overkill), JSON per session (PicoClaw, less efficient) |
| Extend `LlmClient` trait for tool calls | Existing trait returns plain string; tool calling needs structured response | New trait (duplication), raw HTTP (loses abstraction) |
| Feature-gate channels | Keep binary lean; `--features telegram,discord` | Always compile all (slow builds, unnecessary deps) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| AgentMailbox as bus | 2,174 LOC of Erlang patterns for what is a simple channel | Complexity without benefit for 1:1 message routing |
| process_command() as agent loop | Designed for structured CommandTypes, not iterative tool-calling chat | Would require heavy adaptation, losing the PicoClaw simplicity |
| VersionedMemory for sessions | Too heavy for per-message chat history; designed for long-lived agent state | Session load/save becomes expensive |
| Multi-agent workflows | Premature for Phase 1 chat; no evidence of need from reference projects | Months of integration work before first message delivered |
| Custom LLM provider registry | Terraphim has LlmClient trait + Ollama + OpenRouter; that's enough for MVP | Delays shipping for marginal flexibility |

### Simplicity Check

> **What if this could be easy?**

The simplest design: one binary with a channel trait, a message bus (two tokio channels), an agent loop that calls the LLM iteratively, and session files on disk. Everything else is a channel adapter or a tool implementation. PicoClaw proves this architecture works in ~6,000 LOC. We target the same in Rust, reusing existing LLM and persistence infrastructure.

**Senior Engineer Test**: This is a straightforward port of a working Go architecture to Rust. No novel algorithms, no distributed systems, no custom protocols. The only complexity is in the channel SDK integrations, which are well-documented third-party libraries.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request (channels + tools + agent loop only)
- [x] No abstractions "in case we need them later" (no generic plugin system)
- [x] No flexibility "just in case" (channels are feature-gated, not dynamically loaded)
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization (no connection pooling, no caching)

---

## File Changes

### New Files (terraphim_tinyclaw crate)

| File | Purpose | Est. LOC |
|------|---------|----------|
| `terraphim_tinyclaw/Cargo.toml` | Crate manifest with feature flags | 60 |
| `terraphim_tinyclaw/src/main.rs` | CLI entry point (agent + gateway modes) | 150 |
| `terraphim_tinyclaw/src/config.rs` | Configuration types and loading | 200 |
| `terraphim_tinyclaw/src/bus.rs` | InboundMessage, OutboundMessage, MessageBus | 120 |
| `terraphim_tinyclaw/src/channel.rs` | Channel trait + ChannelManager | 200 |
| `terraphim_tinyclaw/src/channels/mod.rs` | Feature-gated channel modules | 15 |
| `terraphim_tinyclaw/src/channels/telegram.rs` | Telegram adapter via teloxide | 550 |
| `terraphim_tinyclaw/src/channels/discord.rs` | Discord adapter via serenity | 400 |
| `terraphim_tinyclaw/src/channels/cli.rs` | Interactive CLI adapter (stdin/stdout) | 80 |
| `terraphim_tinyclaw/src/agent/mod.rs` | Agent module root | 10 |
| `terraphim_tinyclaw/src/agent/loop.rs` | Tool-calling agent loop + context compression | 350 |
| `terraphim_tinyclaw/src/agent/context.rs` | System prompt builder | 150 |
| `terraphim_tinyclaw/src/session.rs` | Session + SessionManager with JSONL persistence | 200 |
| `terraphim_tinyclaw/src/tools/mod.rs` | Tool trait + ToolRegistry | 100 |
| `terraphim_tinyclaw/src/tools/filesystem.rs` | read_file, write_file, list_dir | 140 |
| `terraphim_tinyclaw/src/tools/edit.rs` | File edit with uniqueness guard | 100 |
| `terraphim_tinyclaw/src/tools/shell.rs` | Shell exec with deny patterns | 120 |
| `terraphim_tinyclaw/src/tools/web.rs` | web_search (Brave), web_fetch | 150 |
| `terraphim_tinyclaw/src/format.rs` | Markdown-to-platform formatting | 120 |
| **Total** | | **~3,215** |

### Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` (workspace) | Add `terraphim_tinyclaw` to members |
| `crates/terraphim_service/src/llm.rs` | Extend `LlmClient` trait with `chat_with_tools()` method (default impl) |

### Deleted Files

None.

---

## API Design

### Public Types

```rust
// bus.rs -- Message types

/// Message received from a chat channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub channel: String,
    pub sender_id: String,
    pub chat_id: String,
    pub content: String,
    pub media: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl InboundMessage {
    pub fn session_key(&self) -> String {
        format!("{}:{}", self.channel, self.chat_id)
    }
}

/// Message to send to a chat channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
}

/// Async message bus using tokio mpsc channels.
pub struct MessageBus {
    pub inbound_tx: mpsc::Sender<InboundMessage>,
    pub inbound_rx: Mutex<mpsc::Receiver<InboundMessage>>,
    pub outbound_tx: mpsc::Sender<OutboundMessage>,
    pub outbound_rx: Mutex<mpsc::Receiver<OutboundMessage>>,
}
```

```rust
// channel.rs -- Channel abstraction

/// Trait for chat platform adapters.
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    async fn start(&self, bus: Arc<MessageBus>) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn send(&self, msg: OutboundMessage) -> Result<()>;
    fn is_running(&self) -> bool;
    fn is_allowed(&self, sender_id: &str) -> bool;
}

/// Manages channel lifecycle and outbound dispatch.
pub struct ChannelManager {
    channels: HashMap<String, Box<dyn Channel>>,
    bus: Arc<MessageBus>,
}
```

```rust
// tools/mod.rs -- Tool abstraction

/// A tool call request from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Response from an LLM that may include tool calls.
#[derive(Debug)]
pub struct LlmToolResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: TokenUsage,
}

/// Tool interface for agent capabilities.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<String>;
}

/// Registry of available tools with JSON Schema export.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn register(&mut self, tool: Box<dyn Tool>);
    pub fn get(&self, name: &str) -> Option<&dyn Tool>;
    pub fn to_openai_tools(&self) -> Vec<serde_json::Value>;
    pub async fn execute(&self, call: &ToolCall) -> Result<String>;
}
```

```rust
// session.rs -- Session management

/// A conversation session with message history.
#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub key: String,
    pub messages: Vec<ChatMessage>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Session {
    pub fn get_history(&self, max_messages: usize) -> Vec<ChatMessage>;
    pub fn add_message(&mut self, msg: ChatMessage);
    pub fn set_summary(&mut self, summary: String);
    pub fn clear(&mut self);
}

/// Manages sessions with JSONL file persistence.
pub struct SessionManager {
    sessions_dir: PathBuf,
    cache: HashMap<String, Session>,
}

impl SessionManager {
    pub fn new(sessions_dir: PathBuf) -> Self;
    pub fn get_or_create(&mut self, key: &str) -> &mut Session;
    pub fn save(&self, session: &Session) -> Result<()>;
    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>>;
}
```

```rust
// agent/loop.rs -- Core agent loop

/// Configuration for the agent loop.
pub struct AgentLoopConfig {
    pub max_iterations: usize,         // Default: 20
    pub compress_at_messages: usize,   // Default: 20
    pub compress_at_token_ratio: f32,  // Default: 0.75
    pub keep_last_messages: usize,     // Default: 4
    pub model: String,
}

/// The core agent processing engine.
pub struct AgentLoop {
    config: AgentLoopConfig,
    llm: Arc<dyn LlmClient>,
    tools: Arc<ToolRegistry>,
    sessions: SessionManager,
    context: ContextBuilder,
}

impl AgentLoop {
    pub async fn run(&mut self, bus: Arc<MessageBus>) -> Result<()>;
    async fn process_message(&mut self, msg: InboundMessage) -> Result<OutboundMessage>;
    async fn compress_context(&self, session: &mut Session) -> Result<()>;
}
```

```rust
// config.rs -- Configuration

/// Root configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub channels: ChannelsConfig,
    pub tools: ToolsConfig,
}

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub model: String,
    pub max_iterations: usize,
    pub workspace: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct ChannelsConfig {
    #[cfg(feature = "telegram")]
    pub telegram: Option<TelegramConfig>,
    #[cfg(feature = "discord")]
    pub discord: Option<DiscordConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramConfig {
    pub token: String,
    pub allow_from: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    pub token: String,
    pub allow_from: Vec<String>,
}
```

### LlmClient Trait Extension

```rust
// Addition to crates/terraphim_service/src/llm.rs

/// Extended chat completion with tool-calling support.
/// Default implementation calls chat_completion() and returns no tool calls.
async fn chat_with_tools(
    &self,
    messages: Vec<serde_json::Value>,
    tools: Option<Vec<serde_json::Value>>,
    opts: ChatOptions,
) -> ServiceResult<LlmToolResponse> {
    // Default: delegate to chat_completion, return content-only response
    let content = self.chat_completion(messages, opts).await?;
    Ok(LlmToolResponse {
        content: Some(content),
        tool_calls: vec![],
        usage: TokenUsage::default(),
    })
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum TinyClawError {
    #[error("channel error ({channel}): {message}")]
    Channel { channel: String, message: String },

    #[error("agent loop error: {0}")]
    AgentLoop(String),

    #[error("tool execution error ({tool}): {message}")]
    ToolExecution { tool: String, message: String },

    #[error("session error: {0}")]
    Session(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
```

---

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_inbound_session_key` | `bus.rs` | Verify session key format `channel:chat_id` |
| `test_message_bus_roundtrip` | `bus.rs` | Send inbound, receive inbound |
| `test_tool_registry_schema_export` | `tools/mod.rs` | Verify OpenAI-format JSON Schema output |
| `test_tool_execute_read_file` | `tools/filesystem.rs` | Read existing file, read missing file |
| `test_tool_execute_shell_deny` | `tools/shell.rs` | Verify deny patterns block dangerous commands |
| `test_session_add_get_history` | `session.rs` | Add messages, get truncated history |
| `test_session_jsonl_persistence` | `session.rs` | Save and reload from JSONL file |
| `test_context_compression_trigger` | `agent/loop.rs` | Trigger compression at message count threshold |
| `test_is_allowed_empty_list` | `channel.rs` | Empty allow-list permits all |
| `test_is_allowed_whitelist` | `channel.rs` | Only listed senders permitted |
| `test_config_from_toml` | `config.rs` | Parse config with defaults |
| `test_markdown_to_telegram_html` | `format.rs` | Convert markdown bold/italic/code to HTML |
| `test_markdown_to_discord` | `format.rs` | Pass-through (Discord supports markdown natively) |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_agent_loop_no_tools` | `tests/agent_loop.rs` | Message in -> LLM call -> response out (no tool calls) |
| `test_agent_loop_with_tool_call` | `tests/agent_loop.rs` | Message in -> LLM returns tool call -> execute -> LLM final response |
| `test_agent_loop_max_iterations` | `tests/agent_loop.rs` | Verify loop stops at max_iterations |
| `test_channel_manager_dispatch` | `tests/channel_manager.rs` | Outbound message routes to correct channel |
| `test_full_roundtrip_cli` | `tests/cli_roundtrip.rs` | CLI input -> bus -> agent -> bus -> CLI output |

### Live Tests (gated by env vars)

| Test | Gate | Purpose |
|------|------|---------|
| `test_telegram_send_receive` | `TELEGRAM_BOT_TOKEN` | Send and receive via real Telegram bot |
| `test_discord_send_receive` | `DISCORD_BOT_TOKEN` | Send and receive via real Discord bot |
| `test_llm_tool_calling` | `OPENROUTER_API_KEY` | Real LLM tool-calling roundtrip |

---

## Implementation Steps

### Step 1: Crate Scaffold + Bus + Config
**Files:** `Cargo.toml`, `src/main.rs`, `src/bus.rs`, `src/config.rs`
**Description:** Create crate, define message types, message bus, config parsing. Binary prints "tinyclaw starting" and exits.
**Tests:** `test_inbound_session_key`, `test_message_bus_roundtrip`, `test_config_from_toml`
**Dependencies:** None
**Estimated:** 4 hours

### Step 2: Channel Trait + CLI Adapter
**Files:** `src/channel.rs`, `src/channels/mod.rs`, `src/channels/cli.rs`
**Description:** Define Channel trait, ChannelManager, and CLI adapter for stdin/stdout interaction. Binary runs in agent mode reading from terminal.
**Tests:** `test_is_allowed_empty_list`, `test_is_allowed_whitelist`
**Dependencies:** Step 1
**Estimated:** 3 hours

### Step 3: Session Manager
**Files:** `src/session.rs`
**Description:** Session struct with JSONL persistence, in-memory cache, truncated history retrieval.
**Tests:** `test_session_add_get_history`, `test_session_jsonl_persistence`
**Dependencies:** Step 1
**Estimated:** 3 hours

### Step 4: Tool Trait + Registry + 5 Tools
**Files:** `src/tools/mod.rs`, `src/tools/filesystem.rs`, `src/tools/edit.rs`, `src/tools/shell.rs`, `src/tools/web.rs`
**Description:** Tool trait, registry with JSON Schema export, and 5 tool implementations. Shell tool includes deny patterns for dangerous commands (`rm -rf`, `shutdown`, etc.).
**Tests:** `test_tool_registry_schema_export`, `test_tool_execute_read_file`, `test_tool_execute_shell_deny`
**Dependencies:** Step 1
**Estimated:** 6 hours

### Step 5: Extend LlmClient for Tool Calling
**Files:** `crates/terraphim_service/src/llm.rs` (modify), `crates/terraphim_service/src/openrouter.rs` (modify)
**Description:** Add `chat_with_tools()` to LlmClient trait with default impl. Implement for OpenRouter provider to send tools in request body and parse tool_calls from response.
**Tests:** Unit test with mock response JSON. Live test gated by `OPENROUTER_API_KEY`.
**Dependencies:** Step 4
**Estimated:** 4 hours

### Step 6: Agent Loop + Context Builder
**Files:** `src/agent/mod.rs`, `src/agent/loop.rs`, `src/agent/context.rs`
**Description:** Core agent loop: consume inbound -> get/create session -> build context -> iterative LLM+tool loop -> save session -> publish outbound. Context builder assembles system prompt from workspace bootstrap files.
**Tests:** `test_context_compression_trigger`, integration tests for agent loop
**Dependencies:** Steps 2, 3, 4, 5
**Estimated:** 8 hours

### Step 7: Context Compression
**Files:** `src/agent/loop.rs` (extend)
**Description:** Port PicoClaw's context compression algorithm. Trigger when history > 20 messages OR estimated tokens > 75% of context window. Summarize via LLM, replace history with summary + last 4 messages. Multi-part summarization for large histories.
**Tests:** `test_context_compression_trigger`, integration test with mock LLM
**Dependencies:** Step 6
**Estimated:** 3 hours

### Step 8: Markdown Formatting
**Files:** `src/format.rs`
**Description:** Convert LLM markdown output to platform-specific formats. Telegram: bold -> `<b>`, italic -> `<i>`, code blocks -> `<pre>`. Discord: pass-through. Message chunking for platform character limits (Telegram: 4096, Discord: 2000).
**Tests:** `test_markdown_to_telegram_html`, `test_markdown_to_discord`
**Dependencies:** None (can run in parallel with Steps 3-7)
**Estimated:** 2 hours

### Step 9: Telegram Adapter
**Files:** `src/channels/telegram.rs`
**Description:** Feature-gated Telegram channel using teloxide. Long polling for messages, send responses with HTML formatting, typing indicators (3s refresh), message chunking, photo download, `/reset` command.
**Tests:** Compilation test (feature-gated). Live test gated by `TELEGRAM_BOT_TOKEN`.
**Dependencies:** Steps 2, 8
**Estimated:** 8 hours

### Step 10: Discord Adapter
**Files:** `src/channels/discord.rs`
**Description:** Feature-gated Discord channel using serenity. DM handling, typing indicators, message splitting at 2000 chars, `/reset` slash command.
**Tests:** Compilation test (feature-gated). Live test gated by `DISCORD_BOT_TOKEN`.
**Dependencies:** Steps 2, 8
**Estimated:** 6 hours

### Step 11: Gateway Mode + Orchestrator
**Files:** `src/main.rs` (extend)
**Description:** Wire everything together. `agent` subcommand runs CLI mode. `gateway` subcommand starts all enabled channels + agent loop + outbound dispatcher. Graceful shutdown on SIGINT/SIGTERM via tokio::signal.
**Tests:** `test_full_roundtrip_cli`
**Dependencies:** Steps 6, 9, 10
**Estimated:** 4 hours

---

## Dependency Management

### New Dependencies

| Crate | Version | Feature Gate | Justification |
|-------|---------|-------------|---------------|
| `teloxide` | latest | `telegram` | Telegram bot SDK; standard Rust choice |
| `serenity` | 0.12.x | `discord` | Discord bot SDK; batteries-included |
| `clap` | 4.x | always | CLI argument parsing (already in workspace for other bins) |
| `toml` | 0.8.x | always | Config file parsing |
| `dirs` | 5.x | always | Platform-appropriate config/data directories |

### Existing Dependencies Reused

- `tokio` (runtime, mpsc, signal, fs)
- `serde` + `serde_json` (serialization)
- `reqwest` (HTTP for web tools, voice later)
- `tracing` (structured logging)
- `thiserror` (error types)
- `uuid` (message IDs)
- `chrono` (timestamps)

### Feature Flags

```toml
[features]
default = ["telegram", "discord"]
telegram = ["dep:teloxide"]
discord = ["dep:serenity"]
voice = ["dep:reqwest"]  # Future: Groq Whisper
```

---

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Bus routing latency | < 1ms | tokio channel send/recv |
| Session load (cold) | < 10ms | JSONL file read |
| Session save | < 5ms | JSONL file write |
| Tool execution (filesystem) | < 50ms | File I/O |
| Memory per idle channel | < 5MB | Heap profiling |
| Startup time | < 2s | Wall clock |

### No Benchmarks Needed for Phase 1

The performance-critical path is the LLM call (seconds), not the local routing (microseconds). Benchmarks are premature at this stage.

---

## Rollback Plan

Since this is a new crate, rollback is trivial:
1. Remove `terraphim_tinyclaw` from workspace members
2. Revert any changes to `crates/terraphim_service/src/llm.rs`

No database migrations, no shared state changes.

---

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| WhatsApp bridge strategy (Phase 2) | Deferred | Alex |
| Feishu SDK availability in Rust | Deferred | Alex |
| Voice transcription provider choice | Deferred | Alex |
| Skills system design | Deferred | Alex |

---

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
