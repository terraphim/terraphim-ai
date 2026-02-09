# Rebuilding TinyClaw with Terraphim AI

## Executive Summary

**Verdict: Yes, it is feasible.** Terraphim already has most of the building blocks needed to rebuild TinyClaw — and can deliver a significantly more capable version. The key gaps are channel adapters (Discord/WhatsApp clients) and the orchestrator glue, which are straightforward to implement on top of Terraphim's existing agent infrastructure.

---

## What TinyClaw Does

TinyClaw is a lightweight multi-channel AI assistant wrapper around Claude Code CLI. Its core components:

| Component | Purpose |
|---|---|
| `queue-processor.ts` | File-based message queue, polls incoming/, invokes `claude -c -p`, writes to outgoing/ |
| `discord-client.ts` | Discord bot (discord.js), writes DMs to queue, polls responses |
| `whatsapp-client.ts` | WhatsApp client (whatsapp-web.js), QR auth, message routing |
| `tinyclaw.sh` | tmux-based process orchestrator (start/stop/status/attach) |
| `setup-wizard.sh` | Interactive first-run configuration |
| `heartbeat-cron.sh` | 5-minute heartbeat for proactive engagement |

**Key architectural choices:**
- File-based queue (`incoming/` → `processing/` → `outgoing/`) as implicit lock mechanism
- Sequential single-threaded message processing (no concurrency)
- 4000-char response truncation, 2-minute timeout, 10MB buffer
- Direct `claude` CLI subprocess invocation with `--dangerously-skip-permissions`

---

## Terraphim Component Mapping

### What Terraphim Already Has

| TinyClaw Need | Terraphim Component | Status |
|---|---|---|
| Message queue | `terraphim_agent_messaging` — Erlang-style mailboxes with Call/Cast/Info patterns, delivery guarantees | **Ready** |
| Sequential processing | `terraphim_agent_messaging` — single-consumer mailbox per agent | **Ready** |
| Agent lifecycle | `terraphim_agent_supervisor` — OTP supervision trees, restart strategies | **Ready** |
| Agent state/context | `terraphim_multi_agent::TerraphimAgent` — status, context, command history, token tracking | **Ready** |
| Agent memory | `terraphim_agent_evolution` — versioned memory (short/long-term), task lists, lessons learned | **Ready** |
| LLM integration | `terraphim_service` — `LlmClient` trait with Ollama + OpenRouter providers | **Ready** |
| Command execution | `terraphim_agent::commands` — registry, executor, validation, risk levels | **Ready** |
| Secure execution | `terraphim_firecracker` — sub-2s VM isolation for untrusted commands | **Ready** |
| Knowledge graph | `terraphim_rolegraph` + `terraphim_automata` — semantic matching, autocomplete | **Ready** |
| Multi-agent workflows | `terraphim_multi_agent::workflows` — chaining, routing, parallelization, lead+specialists | **Ready** |
| Configuration | `terraphim_config` — role-based config with JSON, env var overrides | **Ready** |
| HTTP API | `terraphim_server` — Salvo-based REST endpoints | **Ready** |
| MCP tools | `terraphim_mcp_server` — autocomplete, matching, thesaurus, connectivity | **Ready** |
| Process management | tmux in TinyClaw; Terraphim uses tokio tasks + supervision | **Better** |
| Goal alignment | `terraphim_goal_alignment` — multi-level goals, conflict detection | **Ready** |
| Task decomposition | `terraphim_task_decomposition` — complexity analysis, dependency planning | **Ready** |

### What Needs to Be Built

| Component | Effort | Description |
|---|---|---|
| Discord channel adapter | Medium | Discord bot using `serenity` (Rust) or `twilight`, maps messages to agent mailbox |
| WhatsApp channel adapter | Medium | WhatsApp Business API client or bridge (e.g., via Matrix/mautrix-whatsapp) |
| Channel abstraction trait | Small | `ChannelAdapter` trait: `recv_message()`, `send_response()`, `channel_id()` |
| Orchestrator binary | Small | New binary crate that wires channels → agent messaging → LLM → responses |
| Heartbeat service | Small | Tokio interval task that sends periodic prompts to the agent |
| Setup CLI | Small | Interactive setup using `dialoguer` crate for channel config |

---

## Architecture: Terraphim TinyClaw

```
                    ┌──────────────────────────────────────────┐
                    │         terraphim_tinyclaw (binary)       │
                    │                                          │
                    │  ┌─────────┐  ┌──────────┐  ┌────────┐  │
  Discord ◄────────┤  │ Discord │  │ WhatsApp │  │ Slack  │  │
                    │  │ Adapter │  │ Adapter  │  │Adapter │  │
  WhatsApp ◄───────┤  └────┬────┘  └────┬─────┘  └───┬────┘  │
                    │       │            │            │        │
  Slack ◄──────────┤       ▼            ▼            ▼        │
                    │  ┌─────────────────────────────────────┐ │
                    │  │     ChannelAdapter trait             │ │
                    │  │  recv() -> ChannelMessage            │ │
                    │  │  send(ChannelResponse)               │ │
                    │  └──────────────┬──────────────────────┘ │
                    │                 │                         │
                    │                 ▼                         │
                    │  ┌─────────────────────────────────────┐ │
                    │  │   terraphim_agent_messaging          │ │
                    │  │   (Erlang-style mailbox per agent)   │ │
                    │  └──────────────┬──────────────────────┘ │
                    │                 │                         │
                    │                 ▼                         │
                    │  ┌─────────────────────────────────────┐ │
                    │  │   TerraphimAgent                     │ │
                    │  │   - Role context & knowledge graph   │ │
                    │  │   - Versioned memory & lessons       │ │
                    │  │   - Command history & token tracking │ │
                    │  │   - Goal alignment                   │ │
                    │  └──────────────┬──────────────────────┘ │
                    │                 │                         │
                    │                 ▼                         │
                    │  ┌─────────────────────────────────────┐ │
                    │  │   LLM Backend                        │ │
                    │  │   Ollama / OpenRouter / LLM Router   │ │
                    │  └─────────────────────────────────────┘ │
                    │                                          │
                    │  ┌────────────────┐  ┌────────────────┐  │
                    │  │ Heartbeat Task │  │ Supervisor     │  │
                    │  │ (tokio timer)  │  │ (OTP restart)  │  │
                    │  └────────────────┘  └────────────────┘  │
                    └──────────────────────────────────────────┘
```

---

## Advantages Over Original TinyClaw

| Aspect | TinyClaw | Terraphim TinyClaw |
|---|---|---|
| Language | TypeScript + Bash | Rust (single binary, no runtime) |
| Queue system | File-based (fragile) | In-memory Erlang-style mailboxes with delivery guarantees |
| Fault tolerance | Manual tmux restart | OTP supervision trees with automatic restart strategies |
| LLM backend | Claude Code CLI only | Any LLM (Ollama local, OpenRouter, custom) |
| Memory | None (stateless per-session) | Versioned short/long-term memory, lessons learned |
| Knowledge | None | Full knowledge graph with semantic search |
| Security | `--dangerously-skip-permissions` | Firecracker VM isolation for command execution |
| Multi-agent | Single agent | Multi-agent workflows (chaining, routing, parallel) |
| Concurrency | Single-threaded sequential | Tokio async with configurable parallelism |
| Extensibility | New channel = new .ts file | `ChannelAdapter` trait, plug-in pattern |
| Goal tracking | None | Goal alignment with conflict detection |
| Task management | None | Task decomposition with dependency tracking |

---

## Implementation Plan

### Phase 1: Channel Abstraction Layer (New Crate)

**Crate: `crates/terraphim_channels/`**

```rust
/// Core trait for messaging channel adapters
#[async_trait]
pub trait ChannelAdapter: Send + Sync + 'static {
    /// Unique channel identifier (e.g., "discord", "whatsapp")
    fn channel_id(&self) -> &str;

    /// Start listening for messages, sending them to the provided mailbox
    async fn start(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()>;

    /// Send a response back to the channel
    async fn send_response(&self, response: ChannelResponse) -> Result<()>;

    /// Graceful shutdown
    async fn shutdown(&self) -> Result<()>;
}

pub struct ChannelMessage {
    pub channel: String,
    pub sender_id: String,
    pub sender_name: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub message_id: String,
    pub metadata: HashMap<String, String>,
}

pub struct ChannelResponse {
    pub channel: String,
    pub recipient_id: String,
    pub content: String,
    pub reply_to: Option<String>,
}
```

**Tasks:**
1. Define `ChannelAdapter` trait and message types
2. Implement channel router that multiplexes multiple adapters into a single message stream
3. Add response routing back to correct adapter

### Phase 2: Discord Adapter

**Option A (recommended): `serenity` crate** — mature, well-documented Rust Discord library.

**Tasks:**
1. Create `DiscordAdapter` implementing `ChannelAdapter`
2. Handle bot authentication via token
3. Listen for DMs and mentions
4. Map Discord messages to `ChannelMessage`
5. Send responses with chunking (2000-char Discord limit)
6. Typing indicator support

### Phase 3: WhatsApp Adapter

**Options (in order of preference):**
1. **Matrix bridge** via `matrix-sdk` + mautrix-whatsapp — cleanest Rust integration
2. **WhatsApp Business API** — official HTTP API, no QR code needed
3. **FFI bridge** to whatsapp-web.js — wrap the Node.js library

**Tasks:**
1. Evaluate and select integration approach
2. Implement `WhatsAppAdapter` with `ChannelAdapter` trait
3. Handle authentication flow (QR or API key)
4. Message serialization and response routing

### Phase 4: Orchestrator Binary

**Crate: `terraphim_tinyclaw/` (binary)**

Wire everything together:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load config
    let config = load_tinyclaw_config()?;

    // 2. Initialize TerraphimAgent with role + knowledge graph
    let agent = TerraphimAgent::new(config.role, config.agent_config).await?;

    // 3. Create channel adapters
    let mut adapters: Vec<Box<dyn ChannelAdapter>> = vec![];
    if config.discord.enabled {
        adapters.push(Box::new(DiscordAdapter::new(config.discord)?));
    }
    if config.whatsapp.enabled {
        adapters.push(Box::new(WhatsAppAdapter::new(config.whatsapp)?));
    }

    // 4. Start channel router
    let (msg_tx, msg_rx) = mpsc::channel(256);
    for adapter in &adapters {
        adapter.start(msg_tx.clone()).await?;
    }

    // 5. Start heartbeat
    let heartbeat = HeartbeatTask::new(config.heartbeat_interval);

    // 6. Start supervisor
    let supervisor = AgentSupervisor::new(RestartStrategy::OneForOne);

    // 7. Main message loop
    while let Some(msg) = msg_rx.recv().await {
        let response = agent.process_message(msg).await?;
        route_response(&adapters, response).await?;
    }
}
```

**Tasks:**
1. Create binary crate with main orchestration loop
2. Config loading (JSON + env vars)
3. Agent initialization with role and LLM backend
4. Channel adapter startup and multiplexing
5. Heartbeat tokio interval task
6. Supervisor integration for fault tolerance
7. Graceful shutdown handling (SIGINT/SIGTERM)

### Phase 5: Setup CLI

**Tasks:**
1. Interactive setup using `dialoguer` for channel selection, tokens, model choice
2. Config file generation (`.terraphim-tinyclaw/settings.json`)
3. Validation of credentials (test Discord token, etc.)

### Phase 6: Enhanced Features (Terraphim-native)

These go beyond TinyClaw's capabilities, leveraging what Terraphim already provides:

1. **Knowledge-graph-aware responses**: Agent uses role graph to enrich responses with domain knowledge
2. **Multi-agent routing**: Route messages to specialized agents based on content (engineering questions → engineer agent, ops questions → sysadmin agent)
3. **Persistent memory**: Agent remembers past conversations across restarts
4. **Secure command execution**: Commands mentioned in chat execute in Firecracker VMs
5. **MCP tool exposure**: Channel users can invoke MCP tools (autocomplete, search, etc.)
6. **Additional channels**: Slack, Telegram, Matrix adapters via the same trait

---

## Dependency Analysis

### New Dependencies Needed

| Dependency | Purpose | Maturity |
|---|---|---|
| `serenity` | Discord bot framework | Stable, widely used |
| `matrix-sdk` | Matrix client (for WhatsApp bridge) | Stable |
| `dialoguer` | Interactive CLI prompts | Stable |
| `signal-hook` | Unix signal handling | Stable |

### Existing Terraphim Dependencies Leveraged

- `tokio` — async runtime, channels, timers, signal handling
- `serde` / `serde_json` — message serialization
- `tracing` — structured logging
- `chrono` — timestamps
- `ahash` — fast hashmaps for routing tables
- `uuid` — message and agent IDs

---

## Estimated Scope

| Phase | New Code | Reused Code | Risk |
|---|---|---|---|
| 1. Channel abstraction | ~300 LOC | ~0 | Low |
| 2. Discord adapter | ~500 LOC | ~0 | Low |
| 3. WhatsApp adapter | ~400-800 LOC | ~0 | Medium (API choice) |
| 4. Orchestrator binary | ~400 LOC | ~2000 LOC (agent, messaging, supervisor) | Low |
| 5. Setup CLI | ~200 LOC | ~0 | Low |
| 6. Enhanced features | ~500 LOC | ~3000 LOC (knowledge graph, evolution, workflows) | Low |

**Total new code: ~2000-2700 LOC**
**Total reused Terraphim code: ~5000+ LOC**

The reuse ratio is strong — roughly 2:1 in favor of existing code.

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|---|---|---|
| WhatsApp integration complexity | Medium | Start with Matrix bridge; fall back to Business API |
| Discord API rate limits | Low | Built-in rate limiting in `serenity` |
| LLM response latency | Medium | Use Terraphim's existing timeout + retry patterns |
| Memory consumption with many channels | Low | Bounded channels, configurable limits in `AgentConfig` |
| Supervision tree complexity | Low | Start with `OneForOne` strategy, iterate |

---

## Conclusion

Rebuilding TinyClaw with Terraphim is not only feasible but advantageous. The original TinyClaw is ~1000 lines of TypeScript with file-based queues and manual tmux orchestration. Terraphim provides a production-grade foundation with:

- **Fault-tolerant agent lifecycle** (vs. tmux + manual restart)
- **Erlang-style messaging** (vs. file-based queue)
- **Any LLM backend** (vs. Claude CLI only)
- **Knowledge graph intelligence** (vs. stateless)
- **Persistent agent memory** (vs. ephemeral)
- **Secure execution** (vs. `--dangerously-skip-permissions`)
- **Multi-agent workflows** (vs. single sequential processor)

The primary work is building channel adapters (~1000-1300 LOC) and the thin orchestrator binary (~400 LOC). Everything else already exists in Terraphim's crate ecosystem.
