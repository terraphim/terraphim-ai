# Implementation Plan: TinyClaw on Terraphim (terraphim_tinyclaw)

**Status**: Draft
**Research Doc**: [tinyclaw-terraphim-research.md](tinyclaw-terraphim-research.md)
**Author**: Terraphim AI Design
**Date**: 2026-02-11
**Revised**: 2026-02-11 (v3 -- hybrid proxy routing + execution threat scoring)
**Estimated Effort**: ~5,300 LOC Rust + ~400 LOC TypeScript (WhatsApp bridge)

---

## Overview

### Summary

Build a multi-channel AI assistant binary (`terraphim-tinyclaw`) on the Terraphim agent crate ecosystem. The assistant connects to Telegram, Discord, and WhatsApp, routes user messages through a tool-calling agent loop with context compression, and responds via the originating channel. LLM access uses a hybrid architecture: `terraphim-llm-proxy` for tool-calling and quality-critical responses (with intelligent 6-phase routing across 9+ providers), and direct `GenAiLlmClient` for cheap/local tasks like context compression. Tool execution is guarded by Terraphim's existing execution confidence scoring and dangerous pattern hooks. A CLI agent mode enables direct interaction for development and testing.

### Approach

New crate `terraphim_tinyclaw` as a binary crate in the workspace. It composes heavily with `terraphim_multi_agent` for the core agent engine (context management, prompt sanitization, KG enrichment, execution tracking) and `terraphim-llm-proxy` for intelligent LLM routing with full tool-calling support across 9+ providers. Channel adapters, a tool-calling loop, a tool registry, and session persistence are added on top.

The design follows PicoClaw's proven architecture -- channel trait, message bus, agent loop, session manager -- adapted to Rust async idioms. Three key differences from a naive port:

1. **Hybrid LLM routing**: `terraphim-llm-proxy` handles tool-calling and quality-critical responses (6-phase intelligent routing, circuit breaker, SSE streaming, 186 tests). Direct `GenAiLlmClient` handles cheap/local tasks (context compression via Ollama). This avoids reimplementing tool-call format conversion across providers.
2. **Reuse existing agent infrastructure**: `AgentContext` (context windowing), `PromptSanitizer` (injection defense), `CommandHistory` (execution tracking), `TerraphimAgent` (KG enrichment) from `terraphim_multi_agent`.
3. **Execution threat scoring**: Leverage `terraphim_multi_agent::vm_execution` patterns -- `DangerousPatternHook` for shell command safety, `ExecutionConfidence` scoring for code block risk assessment -- instead of building ad-hoc deny lists.

### Reused Components

| Component | Source | What It Provides | What TinyClaw Adds |
|-----------|--------|------------------|-------------------|
| `terraphim-llm-proxy` | Separate binary | 6-phase intelligent routing, tool-calling across 9+ providers, SSE streaming, circuit breaker, KG-based pattern routing (186 tests) | HTTP client wrapper with task-type signaling headers |
| `GenAiLlmClient` | `terraphim_multi_agent` | Multi-provider LLM (Ollama, OpenAI, Anthropic) via rust-genai | Used ONLY for cheap/local tasks (context compression, summarization) |
| `AgentContext` | `terraphim_multi_agent` | Token-aware context windowing with 3 eviction strategies, pinned items | LLM-based summarization trigger before eviction |
| `PromptSanitizer` | `terraphim_multi_agent` | Prompt injection defense (9 patterns, Unicode obfuscation, control chars) | Applied to inbound user messages from channels |
| `DangerousPatternHook` | `terraphim_multi_agent::vm_execution` | Regex-based threat detection (rm -rf, fork bombs, curl\|sh, dd if=, etc.) | Applied to shell tool arguments before execution |
| `ExecutionConfidence` scoring | `terraphim_multi_agent::vm_execution` | Multi-factor confidence scoring (0.0-1.0) for code execution safety | Guards code tool -- auto-execute >0.8, ask user 0.5-0.8, block <0.2 |
| `CommandHistory` + `CommandRecord` | `terraphim_multi_agent` | Execution tracking with quality scores, token/cost stats, step recording | Maps tool-calling iterations to `ExecutionStep` |
| `TerraphimAgent` | `terraphim_multi_agent` | Role-based agent with KG enrichment, state persistence, status tracking | Wraps in iterative tool-calling loop |
| `LlmRequest` / `LlmResponse` / `LlmMessage` | `terraphim_multi_agent` | Typed LLM message types with role enum, token usage | Used for direct GenAiLlmClient calls (compression path) |

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
- Firecracker VM sandboxed execution

**Avoid At All Cost (5/25 Rule):**
- MaixCam hardware channel (PicoClaw-specific niche)
- MoChat channel (niche platform)
- DingTalk channel (niche platform)
- QQ channel (can reuse Telegram pattern later)
- LiteLLM-style provider registry (GenAiLlmClient already handles multi-provider)
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
  +------------------------------------------------------------------------+
  |                                                                        |
  |  +----------+  +----------+  +----------+  +--------+                  |
  |  | Telegram |  | Discord  |  | CLI      |  | (more  |                  |
  |  | Adapter  |  | Adapter  |  | Adapter  |  | later) |                  |
  |  +----+-----+  +----+-----+  +----+-----+  +--------+                  |
  |       |              |              |                                    |
  |       v              v              v                                    |
  |  +-------------------------------------------+                         |
  |  |        MessageBus (tokio::mpsc)            |                         |
  |  |  inbound_tx/rx    outbound_tx/rx           |                         |
  |  +-------------------+---+--------------------+                         |
  |                       |   |                                             |
  |             +---------+   +--------+                                    |
  |             v                      v                                    |
  |  +--------------------+  +-------------------+                         |
  |  | ToolCallingLoop    |  | OutboundDispatch  |                         |
  |  | (NEW ~250 LOC)     |  | - routes by       |                         |
  |  | - iterates LLM     |  |   channel name    |                         |
  |  | - executes tools   |  +-------------------+                         |
  |  | - max_iterations   |                                                 |
  |  +--------+-----------+                                                 |
  |           |                                                             |
  |    +------+------+------+------+                                        |
  |    v      v      v      v      v                                        |
  | +------+ +----+ +------+ +-------------------------------+              |
  | |Tools | |Sess| |Exec  | | TerraphimAgent (REUSED)       |              |
  | |Reg.  | |Mgr | |Guard | |  +-- AgentContext (windowing)  |              |
  | +------+ +----+ +------+ |  +-- PromptSanitizer          |              |
  |                           |  +-- RoleGraph + Automata (KG)|              |
  |                           |  +-- CommandHistory (tracking) |              |
  |                           +-------------------------------+              |
  |                                                                        |
  |  +-- LLM Access (Hybrid) -----------------------------------------+   |
  |  |                                                                  |   |
  |  |  +-----------------------------+  +---------------------------+  |   |
  |  |  | ProxyClient (NEW ~180 LOC)  |  | GenAiLlmClient (REUSED)  |  |   |
  |  |  | - tool_call requests        |  | - context compression    |  |   |
  |  |  | - final responses           |  | - summarization          |  |   |
  |  |  | - SSE streaming             |  | - cheap/local tasks      |  |   |
  |  |  | - task-type headers         |  | - Ollama direct calls    |  |   |
  |  |  +----------+------------------+  +----------+----------------+  |   |
  |  |             |                                |                   |   |
  |  +-------------|--------------------------------|-------------------+   |
  |                v                                v                       |
  +------------------------------------------------------------------------+
               |                                |
               v                                v
  +----------------------------+   +------------------------+
  | terraphim-llm-proxy        |   | Ollama / local LLM     |
  | (separate process)         |   | (direct HTTP)           |
  | - 6-phase routing          |   +------------------------+
  | - KG pattern matching      |
  | - tool-call conversion     |
  | - circuit breaker          |
  | - 9+ providers             |
  +----------------------------+

  External deps: teloxide, serenity, reqwest, tokio
  Internal deps: terraphim_multi_agent (primary),
                 terraphim_agent_evolution, terraphim_config
  Sidecar: terraphim-llm-proxy (launched separately or by TinyClaw)
```

### Data Flow

```
User message on Telegram
  -> TelegramAdapter.handle_message()
  -> InboundMessage { channel: "telegram", sender_id, chat_id, content }
  -> is_allowed() check (allow-list)
  -> bus.inbound_tx.send(msg)
  -> ToolCallingLoop.consume_inbound()
  -> sanitize_system_prompt(msg.content)       [reuse: PromptSanitizer]
  -> session = SessionManager.get_or_create(session_key)
  -> agent_context.add_item(User, msg.content)  [reuse: AgentContext]
  -> check token limit: if over 75%, trigger compression
       compression = genai_client.generate(summarize_prompt)  [DIRECT: cheap/local]
       agent_context.replace_old_items_with_summary(compression)
  -> kg_context = agent.get_enriched_context_for_query(msg.content)
                                                [reuse: TerraphimAgent KG]
  -> build messages from agent_context.format_for_llm()
  -> for i in 0..max_iterations:
       response = proxy_client.chat_with_tools(  [PROXY: tool-calling path]
           messages, tools,
           task_type: "tool_call",
       )
       if no tool_calls: break
       for each tool_call:
           if tool is "shell":
               DangerousPatternHook.pre_tool(code) [reuse: threat detection]
               if blocked: add error to context, continue
           if tool is "code_execute":
               confidence = calculate_execution_confidence(code)
               if confidence < 0.5: add "low confidence" to context, continue
           result = tool_registry.execute(tool_call)
           agent_context.add_item(Tool, result)
           record ExecutionStep                 [reuse: CommandHistory]
  -> final_response = proxy_client.chat(        [PROXY: quality response]
       messages,
       task_type: "final_response",
  )
  -> agent_context.add_item(Assistant, final_response)
  -> session.add_messages(user + assistant)
  -> session_manager.save(session)
  -> OutboundMessage { channel: "telegram", chat_id, content: response }
  -> bus.outbound_tx.send(msg)
  -> OutboundDispatcher routes to TelegramAdapter.send()
  -> User receives response
```

### LLM Routing Strategy (Hybrid Architecture)

The agent loop classifies each LLM call by **task type** and routes accordingly:

| Task Type | Route | Why | Example Provider |
|-----------|-------|-----|-----------------|
| `compression` | Direct GenAiLlmClient | Cheap, high-volume, tolerates lower quality. Local Ollama avoids network latency and cost | Ollama llama3.2 |
| `tool_call` | terraphim-llm-proxy | Needs tool-call format conversion across providers. Proxy handles OpenAI/Anthropic/DeepSeek format differences with 186 tests | Proxy decides (Groq for speed, Claude for quality) |
| `final_response` | terraphim-llm-proxy | Quality-critical user-facing output. Proxy's KG routing can select model by domain (code->deepseek, creative->claude) | Proxy decides via 6-phase routing |
| `simple_qa` | Direct GenAiLlmClient | Simple factual answers don't need tool calling or intelligent routing | Ollama or cheapest cloud |

**Task-type signaling**: The proxy client sends an `X-Task-Type` header with each request. The proxy's Phase 5 (scenario-based routing) already supports custom scenarios via the `Custom(String)` variant, so task types map to routing scenarios without proxy modifications.

**Fallback**: If proxy is unreachable, all requests fall back to direct `GenAiLlmClient`. Tool calling degrades to "generate text, parse tool calls from markdown" -- functional but less reliable.

### Execution Threat Scoring

Tool execution is guarded by patterns from `terraphim_multi_agent::vm_execution`:

```
Tool call from LLM
  -> ExecutionGuard.evaluate(tool_name, arguments)
  |
  +-- Shell tool:
  |   -> DangerousPatternHook.pre_tool(command)       [7 regex patterns]
  |      - rm -rf, format c:, mkfs, dd if=, fork bomb, curl|sh, wget|sh
  |   -> if blocked: return error with reason
  |   -> Shell deny-list check (additional: shutdown, reboot, passwd)
  |   -> Execute with timeout
  |
  +-- Code/script tool (future):
  |   -> CodeBlockExtractor.calculate_execution_confidence()
  |      - Language score: python/bash +0.4, rust +0.3, text +0.1
  |      - Code characteristics: multi-line +0.2, has functions +0.1
  |      - Context clues: execution keywords +0.2, proximity +0.1
  |   -> if confidence < 0.5: block with "low execution confidence"
  |   -> if confidence 0.5-0.8: log warning, execute
  |   -> if confidence > 0.8: execute silently
  |
  +-- Filesystem/web tools:
      -> Standard argument validation (path traversal, SSRF)
      -> Execute normally
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| New binary crate, not library | Binary is the deliverable; library abstractions live in existing crates | Modifying terraphim_server (wrong responsibility) |
| `tokio::mpsc` for bus, not AgentMailbox | Simpler, proven pattern from PicoClaw; AgentMailbox uses `Box<dyn Any + Send>` (wrong abstraction for concrete chat messages). OpenClaw evaluation confirmed: scheduling/batching/fan-out patterns sit above the queue primitive | AgentMailbox (Erlang-style Call/Cast/Info with type erasure) |
| Compose with `TerraphimAgent` | Reuse AgentContext (windowing), PromptSanitizer, KG enrichment, CommandHistory instead of reimplementing | Write everything from scratch (duplicates ~1,200 LOC) |
| **Hybrid LLM: proxy for tool calls, direct for compression** | `terraphim-llm-proxy` already has tool-call format conversion across 9+ providers (186 tests), 6-phase routing, circuit breaker. rust-genai (used by GenAiLlmClient) does NOT support tool calling. Reimplementing would duplicate 4,200 LOC. Direct path for compression avoids proxy overhead on high-volume cheap calls | Extend GenAiLlmClient with tool calling (rust-genai lacks support), Route ALL traffic through proxy (wasteful for compression), Use only direct calls (lose intelligent routing + tool-call conversion) |
| Reuse `DangerousPatternHook` for shell safety | Already has 7 battle-tested regex patterns (rm -rf, fork bombs, curl\|sh, etc.) from vm_execution. No need to invent new deny lists | Ad-hoc deny strings in shell tool (less thorough, duplicates work) |
| Reuse `ExecutionConfidence` scoring for code tools | Multi-factor 0.0-1.0 scoring with language, code characteristics, and context analysis. Production-ready in vm_execution | Binary allow/block (too coarse), No safety checks (too permissive) |
| `teloxide` for Telegram | Most mature Rust Telegram library, async, well-documented | Raw HTTP (more work), `frankenstein` (less mature) |
| `serenity` for Discord | Batteries-included, good for first implementation | `twilight` (lighter but harder to start with) |
| JSONL session files | Proven by nanobot, append-friendly, human-readable | SQLite (overkill), JSON per session (PicoClaw, less efficient) |
| Feature-gate channels | Keep binary lean; `--features telegram,discord` | Always compile all (slow builds, unnecessary deps) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| **Extend GenAiLlmClient with tool calling** | rust-genai does NOT support tool calling (future planned feature). Would require implementing multi-provider tool-call format conversion from scratch. terraphim-llm-proxy already has this with 186 tests across 9+ providers | Months of work duplicating proxy, fragile provider-specific code |
| **Route ALL LLM traffic through proxy** | Context compression is high-volume, cheap, and tolerates quality variance. Routing through proxy adds network hop + routing overhead for no benefit | Unnecessary latency on compression, proxy becomes bottleneck |
| AgentMailbox as bus | Uses `Box<dyn Any + Send>` payloads requiring downcasting; designed for N:M agent-to-agent routing, not N:1 channel-to-agent chat. OpenClaw evaluation confirmed their patterns (FIFO queuing, debouncing, fan-out) map to tokio::Semaphore/broadcast, not AgentMailbox | Type-safety loss, wrong abstraction |
| `process_command()` as agent loop | Routes by CommandType (Generate/Answer/Search/etc.) -- single-shot dispatch, not iterative tool-calling. TinyClaw sends everything through the same tool-calling loop | Would require heavy adaptation, losing the PicoClaw simplicity |
| VersionedMemory for sessions | Too heavy for per-message chat history; designed for long-lived agent state | Session load/save becomes expensive |
| Multi-agent workflows (pool, registry) | Premature for Phase 1 chat; single TerraphimAgent suffices | Months of integration work before first message delivered |
| Ad-hoc shell deny lists | DangerousPatternHook already provides battle-tested regex patterns. Writing new deny strings is error-prone and duplicates existing security code | Missed dangerous patterns, inconsistent safety across tools |

### Reused Options (from terraphim ecosystem)

| Component Reused | Why Reused | What We Avoid Reimplementing |
|------------------|-----------|------------------------------|
| `terraphim-llm-proxy` (separate process) | 6-phase routing, tool-call conversion across 9+ providers, circuit breaker, SSE streaming, KG pattern matching | ~4,200 LOC proxy + 186 tests of provider-specific tool-call handling |
| `GenAiLlmClient` (compression path only) | Already handles Ollama/OpenAI with base URL config. Perfect for cheap local summarization | ~340 LOC LLM client (reused as-is, no extension needed) |
| `AgentContext` | Token windowing, 3 eviction strategies, pinned items, `format_for_llm()` | ~530 LOC context management |
| `PromptSanitizer` | 9 injection patterns, Unicode obfuscation detection, control char stripping | ~218 LOC security code |
| `DangerousPatternHook` | 7 regex threat patterns, Block/Allow decision, tracing integration | ~55 LOC threat detection (covers rm -rf, fork bombs, pipe-to-shell, etc.) |
| `ExecutionConfidence` scoring algorithm | Language-based + code characteristics + context analysis -> 0.0-1.0 score | ~70 LOC multi-factor risk assessment |
| `CommandHistory` + tracking types | Execution steps, quality scores, token/cost stats | ~200 LOC observability |
| `TerraphimAgent.get_enriched_context_for_query()` | KG node matching, graph path connectivity, related concepts | KG integration for free |

### Simplicity Check

> **What if this could be easy?**

The simplest design: one binary with a channel trait, a message bus (two tokio channels), and a thin tool-calling loop that wraps the existing `TerraphimAgent`. Context management, prompt sanitization, KG enrichment, execution tracking, and threat scoring are already built -- we compose with them. LLM access splits into two paths: a 180-LOC HTTP client to `terraphim-llm-proxy` for tool calls (leveraging 4,200 LOC of existing routing + provider conversion), and direct `GenAiLlmClient` for compression. The new code is: channel adapters, proxy client, execution guard, tool implementations, JSONL sessions, and ~250 LOC of iterative tool-calling glue. PicoClaw proves this architecture works in ~6,000 LOC Go. We target ~3,100 LOC of new Rust code on top of ~5,600 LOC of reused infrastructure (multi_agent + proxy).

**Senior Engineer Test**: This is a straightforward port of a working Go architecture to Rust, with the LLM layer delegated to an existing proxy rather than reimplemented. No novel algorithms, no distributed systems, no custom protocols. The proxy sidecar is a standard pattern (envoy, linkerd). The only complexity is in the channel SDK integrations, which are well-documented third-party libraries.

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
| `terraphim_tinyclaw/Cargo.toml` | Crate manifest with feature flags | 80 |
| `terraphim_tinyclaw/src/main.rs` | CLI entry point (agent + gateway modes) | 150 |
| `terraphim_tinyclaw/src/config.rs` | Configuration types and loading (includes proxy URL, task routing) | 250 |
| `terraphim_tinyclaw/src/bus.rs` | InboundMessage, OutboundMessage, MessageBus | 120 |
| `terraphim_tinyclaw/src/channel.rs` | Channel trait + ChannelManager | 200 |
| `terraphim_tinyclaw/src/channels/mod.rs` | Feature-gated channel modules | 15 |
| `terraphim_tinyclaw/src/channels/telegram.rs` | Telegram adapter via teloxide | 550 |
| `terraphim_tinyclaw/src/channels/discord.rs` | Discord adapter via serenity | 400 |
| `terraphim_tinyclaw/src/channels/cli.rs` | Interactive CLI adapter (stdin/stdout) | 80 |
| `terraphim_tinyclaw/src/agent/mod.rs` | Agent module root | 10 |
| `terraphim_tinyclaw/src/agent/loop.rs` | Tool-calling loop wrapping TerraphimAgent + hybrid LLM | 300 |
| `terraphim_tinyclaw/src/agent/proxy_client.rs` | HTTP client for terraphim-llm-proxy (tool calls, streaming, task-type headers) | 180 |
| `terraphim_tinyclaw/src/agent/execution_guard.rs` | Tool safety: DangerousPatternHook + ExecutionConfidence scoring (wraps vm_execution) | 120 |
| `terraphim_tinyclaw/src/session.rs` | Session + SessionManager with JSONL persistence | 200 |
| `terraphim_tinyclaw/src/tools/mod.rs` | Tool trait + ToolRegistry | 100 |
| `terraphim_tinyclaw/src/tools/filesystem.rs` | read_file, write_file, list_dir | 140 |
| `terraphim_tinyclaw/src/tools/edit.rs` | File edit with uniqueness guard | 100 |
| `terraphim_tinyclaw/src/tools/shell.rs` | Shell exec (guarded by ExecutionGuard) | 100 |
| `terraphim_tinyclaw/src/tools/web.rs` | web_search (Brave), web_fetch | 150 |
| `terraphim_tinyclaw/src/format.rs` | Markdown-to-platform formatting | 120 |
| **Total new code** | | **~3,365** |

### Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` (workspace) | Add `terraphim_tinyclaw` to members |

### Deleted Files

None.

### Reused Files (no modifications needed)

| File | Source | What It Provides |
|------|--------|------------------|
| `crates/terraphim_multi_agent/src/context.rs` | terraphim_multi_agent | `AgentContext` with token windowing and eviction strategies |
| `crates/terraphim_multi_agent/src/prompt_sanitizer.rs` | terraphim_multi_agent | `sanitize_system_prompt()` for prompt injection defense |
| `crates/terraphim_multi_agent/src/history.rs` | terraphim_multi_agent | `CommandHistory`, `CommandRecord`, `ExecutionStep` for tracking |
| `crates/terraphim_multi_agent/src/agent.rs` | terraphim_multi_agent | `TerraphimAgent` with KG enrichment and state management |
| `crates/terraphim_multi_agent/src/genai_llm_client.rs` | terraphim_multi_agent | `GenAiLlmClient` for direct LLM calls (compression path) |
| `crates/terraphim_multi_agent/src/vm_execution/hooks.rs` | terraphim_multi_agent | `DangerousPatternHook` for threat detection patterns |
| `crates/terraphim_multi_agent/src/vm_execution/code_extractor.rs` | terraphim_multi_agent | `calculate_execution_confidence()` algorithm |
| `terraphim-llm-proxy` (separate binary) | terraphim-llm-proxy repo | Intelligent LLM routing, tool-call conversion, SSE streaming |

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
// agent/loop.rs -- Tool-calling loop composing with TerraphimAgent

/// Configuration for the tool-calling loop.
pub struct ToolCallingConfig {
    pub max_iterations: usize,           // Default: 20
    pub summarize_at_token_ratio: f32,   // Default: 0.75
    pub keep_last_messages: usize,       // Default: 4
}

/// Tool-calling loop that wraps TerraphimAgent with iterative tool execution.
///
/// Composes with existing terraphim_multi_agent components:
/// - `TerraphimAgent` for KG enrichment and state management
/// - `AgentContext` for token-aware context windowing (RelevanceFirst/Balanced)
/// - `HybridLlmRouter` for proxy (tool calls) + direct (compression) LLM routing
/// - `ExecutionGuard` for tool safety (DangerousPatternHook + confidence scoring)
/// - `PromptSanitizer` for input sanitization
/// - `CommandHistory` for execution tracking
pub struct ToolCallingLoop {
    config: ToolCallingConfig,
    agent: TerraphimAgent,        // reuse: KG, context, tracking
    llm: HybridLlmRouter,        // proxy + direct LLM routing
    guard: ExecutionGuard,        // tool safety evaluation
    tools: Arc<ToolRegistry>,
    sessions: SessionManager,
}

impl ToolCallingLoop {
    pub async fn run(&mut self, bus: Arc<MessageBus>) -> Result<()>;
    async fn process_message(&mut self, msg: InboundMessage) -> Result<OutboundMessage>;
    async fn compress_if_needed(&self, session: &mut Session) -> Result<()>;
}
```

```rust
// config.rs -- Configuration

/// Root configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub llm: LlmConfig,
    pub channels: ChannelsConfig,
    pub tools: ToolsConfig,
}

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: usize,        // Default: 20
    pub workspace: PathBuf,
    pub system_prompt: Option<String>,
}

/// Hybrid LLM configuration.
/// Proxy handles tool calls + quality responses.
/// Direct handles compression + simple QA.
#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub proxy: ProxyConfig,
    pub direct: DirectLlmConfig,
}

#[derive(Debug, Deserialize)]
pub struct ProxyConfig {
    pub base_url: String,             // e.g., "http://127.0.0.1:3456"
    pub api_key: String,              // Proxy API key (env var expansion)
    pub timeout_ms: u64,              // Default: 60_000
    pub model: Option<String>,        // Override proxy routing (optional)
}

#[derive(Debug, Deserialize)]
pub struct DirectLlmConfig {
    pub provider: String,             // e.g., "ollama"
    pub model: String,                // e.g., "llama3.2"
    pub base_url: Option<String>,     // e.g., "http://127.0.0.1:11434"
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

### ProxyClient (HTTP client for terraphim-llm-proxy)

```rust
// agent/proxy_client.rs -- HTTP client for tool-calling via terraphim-llm-proxy

/// Task type determines proxy routing scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    ToolCall,         // Needs tool-call format conversion -> proxy routes to capable model
    FinalResponse,    // Quality-critical user-facing output -> proxy routes to quality model
    CodeAnalysis,     // Code understanding tasks -> proxy KG routes to code-specialized model
}

/// Configuration for the proxy client.
#[derive(Debug, Clone, Deserialize)]
pub struct ProxyClientConfig {
    pub base_url: String,           // e.g., "http://127.0.0.1:3456"
    pub api_key: String,            // Proxy API key
    pub timeout_ms: u64,            // Request timeout (default: 60_000)
    pub model: Option<String>,      // Override model (default: proxy decides via routing)
}

/// HTTP client wrapping terraphim-llm-proxy's Anthropic-compatible API.
///
/// Sends requests to `/v1/messages` with `X-Task-Type` header for routing hints.
/// The proxy's 6-phase routing (Pattern-KG, Session, Cost, Performance, Scenario,
/// Default) selects the optimal provider+model. Tool-call format conversion is
/// handled transparently by the proxy.
pub struct ProxyClient {
    config: ProxyClientConfig,
    http: reqwest::Client,
}

impl ProxyClient {
    pub fn new(config: ProxyClientConfig) -> Self;

    /// Send a chat request with tool definitions through the proxy.
    /// Returns parsed response with optional tool_calls.
    /// Proxy handles format conversion (OpenAI/Anthropic/DeepSeek tool formats).
    pub async fn chat_with_tools(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        tools: Vec<ToolDefinition>,
        task_type: TaskType,
    ) -> Result<ProxyResponse>;

    /// Send a simple chat request (no tools) through the proxy.
    pub async fn chat(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        task_type: TaskType,
    ) -> Result<ProxyResponse>;

    /// Send a streaming chat request. Returns an async stream of SSE events.
    /// Proxy streams in Anthropic format: message_start, content_block_delta,
    /// tool_use blocks, message_stop.
    pub async fn chat_stream(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        tools: Vec<ToolDefinition>,
        task_type: TaskType,
    ) -> Result<impl Stream<Item = Result<StreamEvent>>>;

    /// Check proxy health. Falls back to direct GenAiLlmClient if unhealthy.
    pub async fn health_check(&self) -> Result<bool>;
}

/// Parsed response from the proxy.
#[derive(Debug)]
pub struct ProxyResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub model: String,              // Actual model used (proxy may have rerouted)
    pub stop_reason: String,        // "end_turn", "tool_use", "max_tokens"
    pub usage: TokenUsage,
}

/// SSE stream events from the proxy.
#[derive(Debug)]
pub enum StreamEvent {
    TextDelta(String),
    ToolUseStart { id: String, name: String },
    ToolInputDelta { id: String, partial_json: String },
    ThinkingDelta(String),
    MessageStop { stop_reason: String, usage: TokenUsage },
    Ping,
}
```

### ExecutionGuard (tool safety via vm_execution patterns)

```rust
// agent/execution_guard.rs -- Wraps vm_execution hooks for tool call safety

use terraphim_multi_agent::vm_execution::hooks::DangerousPatternHook;
use terraphim_multi_agent::vm_execution::code_extractor::CodeBlockExtractor;

/// Decision from execution guard evaluation.
#[derive(Debug)]
pub enum GuardDecision {
    /// Safe to execute.
    Allow,
    /// Blocked with explanation (sent back to LLM as tool error).
    Block { reason: String },
    /// Low confidence -- execute but log warning.
    Warn { confidence: f64, reason: String },
}

/// Guards tool execution using Terraphim's existing threat detection patterns.
///
/// Composes `DangerousPatternHook` (7 regex patterns for destructive commands)
/// and `CodeBlockExtractor::calculate_execution_confidence()` (multi-factor
/// 0.0-1.0 scoring) from terraphim_multi_agent::vm_execution.
pub struct ExecutionGuard {
    dangerous_patterns: DangerousPatternHook,
    code_extractor: CodeBlockExtractor,
    /// Additional deny patterns specific to chat assistant context
    shell_deny_patterns: Vec<regex::Regex>,
}

impl ExecutionGuard {
    pub fn new() -> Self;

    /// Evaluate a tool call for safety before execution.
    ///
    /// For shell tools: runs DangerousPatternHook + shell deny patterns.
    /// For code/script tools: calculates execution confidence score.
    /// For filesystem/web tools: validates paths (no traversal) and URLs (no SSRF).
    pub fn evaluate(&self, tool_name: &str, arguments: &serde_json::Value) -> GuardDecision;

    /// Evaluate shell command specifically.
    /// Checks dangerous_patterns (rm -rf, fork bombs, curl|sh, etc.)
    /// and additional deny list (shutdown, reboot, passwd, etc.)
    fn evaluate_shell(&self, command: &str) -> GuardDecision;

    /// Evaluate code block for execution confidence.
    /// Returns Allow if confidence > 0.8, Warn if 0.5-0.8, Block if < 0.5.
    fn evaluate_code(&self, language: &str, code: &str) -> GuardDecision;
}
```

### HybridLlmRouter (task-type aware routing)

```rust
// agent/loop.rs -- Part of ToolCallingLoop

/// Routes LLM calls to either proxy or direct client based on task type.
///
/// - tool_call, final_response, code_analysis -> ProxyClient (intelligent routing)
/// - compression, simple_qa -> GenAiLlmClient (direct, cheap/local)
/// - If proxy unreachable -> all fall back to GenAiLlmClient (degraded mode)
pub struct HybridLlmRouter {
    proxy: ProxyClient,
    direct: GenAiLlmClient,           // For compression and fallback
    proxy_healthy: AtomicBool,         // Cached health status
}

impl HybridLlmRouter {
    /// Route a tool-calling request through the proxy.
    /// Falls back to direct client if proxy is down (tool calls parsed from text).
    pub async fn tool_call(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
        tools: Vec<ToolDefinition>,
    ) -> Result<ProxyResponse>;

    /// Route a quality-critical final response through the proxy.
    /// Falls back to direct client if proxy is down.
    pub async fn final_response(
        &self,
        messages: Vec<Message>,
        system: Option<String>,
    ) -> Result<ProxyResponse>;

    /// Route context compression through direct GenAiLlmClient (cheap/local).
    /// Never goes through proxy.
    pub async fn compress(
        &self,
        messages: Vec<Message>,
        system: String,
    ) -> Result<String>;

    /// Periodic health check of proxy. Updates proxy_healthy flag.
    pub async fn check_proxy_health(&self);
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
| `test_tool_registry_schema_export` | `tools/mod.rs` | Verify Anthropic-format tool definition export |
| `test_tool_execute_read_file` | `tools/filesystem.rs` | Read existing file, read missing file |
| `test_tool_execute_shell_blocked` | `tools/shell.rs` | Verify ExecutionGuard blocks rm -rf, fork bombs, curl\|sh |
| `test_tool_execute_shell_allowed` | `tools/shell.rs` | Verify safe commands pass ExecutionGuard |
| `test_execution_guard_dangerous_patterns` | `agent/execution_guard.rs` | All 7 DangerousPatternHook patterns produce Block |
| `test_execution_guard_shell_deny_list` | `agent/execution_guard.rs` | Additional denials (shutdown, reboot, passwd) |
| `test_execution_confidence_python` | `agent/execution_guard.rs` | Python multi-line code scores > 0.8 |
| `test_execution_confidence_plaintext` | `agent/execution_guard.rs` | Plaintext scores < 0.5 |
| `test_proxy_response_parse` | `agent/proxy_client.rs` | Parse Anthropic-format JSON with tool_calls |
| `test_proxy_response_parse_no_tools` | `agent/proxy_client.rs` | Parse text-only response |
| `test_hybrid_router_compression_direct` | `agent/loop.rs` | Compression always uses direct client, never proxy |
| `test_hybrid_router_fallback` | `agent/loop.rs` | Proxy down -> falls back to direct |
| `test_session_add_get_history` | `session.rs` | Add messages, get truncated history |
| `test_session_jsonl_persistence` | `session.rs` | Save and reload from JSONL file |
| `test_context_eviction_trigger` | `agent/loop.rs` | Verify AgentContext evicts when over token limit |
| `test_summarization_trigger` | `agent/loop.rs` | Trigger LLM summarization at 75% token ratio |
| `test_prompt_sanitization` | `agent/loop.rs` | Verify PromptSanitizer strips injection attempts |
| `test_is_allowed_empty_list` | `channel.rs` | Empty allow-list permits all |
| `test_is_allowed_whitelist` | `channel.rs` | Only listed senders permitted |
| `test_config_from_toml` | `config.rs` | Parse config with proxy + direct LLM sections |
| `test_markdown_to_telegram_html` | `format.rs` | Convert markdown bold/italic/code to HTML |
| `test_markdown_to_discord` | `format.rs` | Pass-through (Discord supports markdown natively) |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_tool_calling_loop_no_tools` | `tests/agent_loop.rs` | Message in -> proxy call -> response out (no tool calls) |
| `test_tool_calling_loop_with_tool` | `tests/agent_loop.rs` | Message in -> proxy returns tool_use -> ExecutionGuard -> execute -> proxy final response |
| `test_tool_calling_loop_max_iterations` | `tests/agent_loop.rs` | Verify loop stops at max_iterations |
| `test_tool_calling_loop_blocked_tool` | `tests/agent_loop.rs` | LLM requests dangerous shell command -> ExecutionGuard blocks -> error returned to LLM -> LLM adjusts |
| `test_kg_enrichment_in_context` | `tests/agent_loop.rs` | Verify KG concepts appear in LLM context |
| `test_channel_manager_dispatch` | `tests/channel_manager.rs` | Outbound message routes to correct channel |
| `test_full_roundtrip_cli` | `tests/cli_roundtrip.rs` | CLI input -> bus -> agent -> bus -> CLI output |
| `test_proxy_health_fallback` | `tests/proxy_fallback.rs` | Proxy process stopped -> tool calls fall back to direct + text parsing |

### Live Tests (gated by env vars)

| Test | Gate | Purpose |
|------|------|---------|
| `test_telegram_send_receive` | `TELEGRAM_BOT_TOKEN` | Send and receive via real Telegram bot |
| `test_discord_send_receive` | `DISCORD_BOT_TOKEN` | Send and receive via real Discord bot |
| `test_proxy_tool_calling` | `PROXY_API_KEY` + running proxy | Real tool-calling roundtrip through proxy |
| `test_proxy_streaming` | `PROXY_API_KEY` + running proxy | SSE streaming with tool use blocks |

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

### Step 5: Proxy Client + Execution Guard
**Files:** `src/agent/proxy_client.rs`, `src/agent/execution_guard.rs`
**Description:** Two components:

**ProxyClient** (~180 LOC): HTTP client for `terraphim-llm-proxy`'s `/v1/messages` endpoint (Anthropic format). Sends `X-Task-Type` header for routing hints. Parses responses including `tool_use` content blocks. Supports both blocking and SSE streaming modes. Health check via `/health`.

**ExecutionGuard** (~120 LOC): Wraps `DangerousPatternHook` from `terraphim_multi_agent::vm_execution` for shell command safety (7 regex patterns: rm -rf, fork bombs, curl|sh, dd if=, etc.). Wraps `CodeBlockExtractor::calculate_execution_confidence()` for code execution risk scoring (language + characteristics + context -> 0.0-1.0). Additional shell deny patterns for chat context (shutdown, reboot, passwd). Returns Allow/Block/Warn decisions.

**Tests:**
- `test_proxy_response_parse`: Parse Anthropic-format JSON with tool_calls from fixture
- `test_proxy_response_parse_no_tools`: Parse text-only response
- `test_execution_guard_dangerous_patterns`: All 7 patterns produce Block
- `test_execution_guard_shell_deny_list`: Additional denials
- `test_execution_confidence_python`: Python multi-line > 0.8
- `test_execution_confidence_plaintext`: Plaintext < 0.5

**Dependencies:** Step 4 (ToolCall types)
**Estimated:** 5 hours

### Step 6: Tool-Calling Loop with Hybrid Routing
**Files:** `src/agent/mod.rs`, `src/agent/loop.rs`
**Description:** `ToolCallingLoop` wraps `TerraphimAgent` and composes with existing components via `HybridLlmRouter`:
- Creates `TerraphimAgent` from Role config (provides AgentContext, KG, CommandHistory)
- Creates `HybridLlmRouter` with ProxyClient (tool calls) + GenAiLlmClient (compression)
- Consumes inbound messages from bus
- Sanitizes input via `sanitize_system_prompt()` (reuse PromptSanitizer)
- Manages `AgentContext` window: adds User/Assistant/Tool items, relies on built-in eviction strategies
- Enriches context via `agent.get_enriched_context_for_query()` (reuse KG)
- Iterative loop:
  - `hybrid_router.tool_call(messages, tools)` -> proxy handles format conversion
  - For each tool_call: `execution_guard.evaluate(tool, args)` -> execute if allowed
  - Record `ExecutionStep` (reuse CommandHistory)
  - Repeat until no tool_calls or max_iterations
  - If proxy down: falls back to `hybrid_router.direct` with text-parsing for tool calls
- Saves session and publishes outbound
**Tests:** `test_context_eviction_trigger`, `test_hybrid_router_compression_direct`, `test_hybrid_router_fallback`, integration tests
**Dependencies:** Steps 2, 3, 4, 5
**Estimated:** 7 hours

### Step 7: LLM-Based Context Compression (Direct Path)
**Files:** `src/agent/loop.rs` (extend)
**Description:** Add LLM summarization as a pre-step before AgentContext's built-in eviction. When `agent_context.current_tokens > max_tokens * 0.75`, summarize the non-pinned history via `hybrid_router.compress()` -- this uses the **direct GenAiLlmClient** path (local Ollama), NOT the proxy. Compression is high-volume and tolerates lower quality, so it avoids proxy overhead and cloud costs. Replace old items with a single Memory-typed summary item (pinned=false, high relevance).
**Tests:** `test_summarization_trigger`, `test_compression_uses_direct_client` (verify proxy not called), integration test verifying summary item replaces history
**Dependencies:** Step 6
**Estimated:** 2 hours

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
| `reqwest-eventsource` | latest | always | SSE stream parsing for proxy streaming responses |
| `regex` | 1.x | always | ExecutionGuard shell deny patterns (additional to DangerousPatternHook) |

### Internal Crate Dependencies

| Crate | What It Provides |
|-------|------------------|
| `terraphim_multi_agent` | `TerraphimAgent`, `GenAiLlmClient` (compression path), `AgentContext`, `PromptSanitizer`, `CommandHistory`, `DangerousPatternHook`, `CodeBlockExtractor`, `LlmRequest`/`LlmResponse` types |
| `terraphim_config` | `Role` configuration type |
| `terraphim_agent_evolution` | `VersionedMemory`, `VersionedTaskList`, `VersionedLessons` (via TerraphimAgent) |
| `terraphim_rolegraph` | `RoleGraph` for KG enrichment (via TerraphimAgent) |
| `terraphim_persistence` | `DeviceStorage` for agent state persistence (via TerraphimAgent) |

### External Process Dependencies

| Process | Purpose | Required? |
|---------|---------|-----------|
| `terraphim-llm-proxy` | Intelligent LLM routing, tool-call conversion, SSE streaming | Yes for tool calling; degraded mode without it |
| Ollama (or local LLM) | Direct compression/summarization path | Recommended; cloud fallback via GenAiLlmClient |

### Transitive Dependencies Reused (via terraphim_multi_agent)

- `tokio` (runtime, mpsc, signal, fs)
- `serde` + `serde_json` (serialization)
- `reqwest` (HTTP for proxy client, web tools)
- `tracing` (structured logging)
- `thiserror` (error types)
- `uuid` (message IDs)
- `chrono` (timestamps)
- `genai` (rust-genai for direct LLM calls on compression path)

### Feature Flags

```toml
[dependencies]
terraphim_multi_agent = { path = "../crates/terraphim_multi_agent" }
terraphim_config = { path = "../crates/terraphim_config" }
terraphim_persistence = { path = "../crates/terraphim_persistence" }

# LLM access
reqwest = { version = "0.12", features = ["json", "stream"] }
reqwest-eventsource = "0.6"

# Channel adapters (feature-gated)
teloxide = { version = "0.13", optional = true }
serenity = { version = "0.12", optional = true }

# Execution guard
regex = "1"

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

Since this is a new crate with zero modifications to existing crates, rollback is trivial:
1. Remove `terraphim_tinyclaw` from workspace members
2. Delete the `terraphim_tinyclaw/` directory

No database migrations, no shared state changes, no modifications to existing crates. The proxy is a separate process and remains unaffected. All integration is via HTTP calls to the proxy and Rust imports from terraphim_multi_agent (read-only).

---

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Proxy auto-launch: should TinyClaw spawn terraphim-llm-proxy as a sidecar, or require it pre-started? | Decision needed | Alex |
| Proxy taxonomy customization: should TinyClaw ship its own taxonomy files for chat-specific routing rules? | Decision needed | Alex |
| Degraded mode UX: when proxy is down, should tool calls be attempted (text-parsing) or disabled entirely? | Decision needed | Alex |
| WhatsApp bridge strategy (Phase 2) | Deferred | Alex |
| Feishu SDK availability in Rust | Deferred | Alex |
| Voice transcription provider choice | Deferred | Alex |
| Skills system design | Deferred | Alex |

---

## Specification Interview Findings

**Interview Date**: 2026-02-11
**Dimensions Covered**: Proxy lifecycle, Execution guard, Concurrency, Sessions, Channel auth, Secrets/errors, Role management, System prompts, Graceful shutdown
**Convergence Status**: Complete (7 rounds, all critical dimensions explored)

### Key Decisions from Interview

#### Proxy Lifecycle & Routing (Failure Modes)

- **Proxy launch model**: Pre-started (separate process). TinyClaw only needs `proxy.base_url` in config. User manages proxy lifecycle via systemd, docker, or manual start. Simplest Phase 1 approach.
- **Proxy fallback when down**: Disable tools entirely, text-only mode. Tell user "tools unavailable, answering from knowledge only". No fragile text-parsed tool calls. Clean failure.
- **Health detection**: On-failure only. No background polling. When a proxy request fails, mark `proxy_healthy = false`. Try again after 60s backoff. No wasted health-check requests when healthy.
- **Task-type signaling**: Skip for Phase 1. Use proxy's default routing for all requests. Add `X-Task-Type` header support in Phase 2 after validating basic flow works.

#### Execution Guard & Tool Safety

- **Blocked tool message**: Specific pattern message sent to LLM. Example: "Command blocked: contains destructive pattern (rm -rf). Suggest alternative: list files first, then remove specific items." Helps LLM adapt without revealing evasion paths.
- **Shell timeout**: Configurable per-tool via `tools.shell.timeout_seconds` in config. Default 120 seconds. User can raise for build-heavy workflows.
- **Filesystem boundaries**: No restriction (anywhere on filesystem). Full access like PicoClaw. User trusts the agent. Simpler implementation.
- **Web tools**: Configurable provider. `tools.web_search.provider = 'brave' | 'searxng' | 'google'`. `tools.web_fetch.mode = 'readability' | 'raw'`. Maximum flexibility.

#### Concurrency & Sessions

- **Message processing**: Serial (one at a time). Single consumer on inbound_rx. While processing User A's multi-iteration loop, User B waits in queue. Acceptable for personal assistant with few users. No race conditions on AgentContext/sessions.
- **Group session model**: Per-chat with user attribution. One session per group chat. Each message records sender_id. LLM sees "User A said X, User B said Y". Shared context with user awareness.
- **Session size cap**: Cap at N messages (200) + summary. Context compression already summarizes old messages. JSONL file keeps growing but only loads recent N + summary on startup. Old messages available in file but not in memory.

#### Channel Auth & Message Handling

- **Auth default**: Require non-empty `allow_from`. Config validation refuses to start if any enabled channel has empty allow_from. Forces user to think about auth before deploying. Prevents accidental exposure of filesystem+shell tools.
- **Message chunking**: Simple paragraph split using `find_paragraph_end()` from `terraphim_automata`. Split at `\n\n` boundaries, greedily pack into platform-sized chunks (4096 Telegram, 2000 Discord). Single paragraph exceeding limit splits at line boundaries. No full markdown normalization needed for Phase 1.
- **Build order**: All three channel adapters (CLI, Telegram, Discord) in parallel after the Channel trait is defined. Channel trait is simple enough.

#### Secrets & Error Handling

- **Secret management**: Environment variable expansion in TOML config values using `$ENV_VAR_NAME` syntax. Matches terraphim-llm-proxy pattern. Config files can be committed without secrets.
- **Error UX**: Error message to user AND admin notification. Send "Sorry, I encountered an error" to originating channel. Log structured error event. If monitoring channel is configured, notify there too.

#### Role Management & System Prompts

- **KG enrichment**: Configurable via `/role list` and `/role select <name>` commands, available in all channels. Switching roles swaps TerraphimAgent's RoleGraph+Automata. KG enrichment is naturally scoped to the active role's domain.
- **Role scope**: Global (one active role for entire TinyClaw instance). `/role select` in any channel changes it everywhere. Simpler state management.
- **Role switch during processing**: Queued. `/role select` waits until current message processing finishes. Next message uses new role. No mid-response context mixing.
- **System prompt**: Two-layer. SYSTEM.md file in workspace provides persona/instructions. Role's KG enrichment adds domain knowledge on top. Combined as: `[SYSTEM.md content]\n\n[KG concepts from active role]`.
- **System prompt token budget**: Uncapped. System prompt is part of AgentContext's total token window. Large system prompts reduce conversation space. User's responsibility to keep SYSTEM.md reasonable.
- **Binary relationship**: Separate binary (`terraphim-tinyclaw`), coexists with `terraphim-agent`. Different use cases: tinyclaw for multi-channel gateway, terraphim-agent for local REPL/search.

#### Graceful Shutdown

- **Shutdown behavior**: On SIGINT/SIGTERM, let current tool iteration finish (with timeout), save session including tool result. Don't start next LLM iteration. Send partial response to user if available.
- **Channel disconnection**: Silent disconnect. Just close connections. No farewell messages. Simple.

### Deferred Items

- Task-type signaling (`X-Task-Type` header): Deferred to Phase 2. Use proxy default routing for Phase 1.
- Proxy sidecar mode (auto-launch): Deferred. Pre-started only for now.
- Custom taxonomy for chat routing: Deferred. Use proxy's existing taxonomy.
- Voice transcription: Deferred to Phase 2+.
- Skills/plugin system: Deferred to Phase 2+.
- Subagent spawning: Deferred to Phase 2+.

### Interview Summary

The specification interview resolved 22 decisions across 9 dimensions. The most impactful findings:

1. **Proxy fallback simplification**: Instead of fragile text-parsed tool calls when proxy is down, TinyClaw cleanly disables tools and operates in text-only mode. This eliminates the complex `HybridLlmRouter` fallback path and makes the degraded mode reliable.

2. **Security-first auth default**: Requiring non-empty `allow_from` prevents accidental exposure of filesystem+shell tools to unauthorized users. This is a breaking change from the original design's "empty = allow all" default.

3. **Role management via `/role` command**: KG enrichment is controlled through the same role-switching mechanism as terraphim-agent's REPL, making it a natural extension rather than a new concept. Global role scope keeps state management simple for Phase 1.

4. **Two-layer system prompt (SYSTEM.md + role KG)**: Combines PicoClaw's SYSTEM.md pattern with Terraphim's role-based KG enrichment, giving users both persona customization and domain-specific knowledge injection.

---

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Specification interview complete
- [ ] Human approval received
