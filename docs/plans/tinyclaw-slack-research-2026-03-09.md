# Research Document: TinyClaw Slack Channel Adapter

**Status**: Review
**Author**: Terraphim AI
**Date**: 2026-03-09
**Reviewers**: AlexMikhalev
**Related Issues**: #519, #590

## Executive Summary

TinyClaw already has a mature channel-trait pattern with Telegram and Discord adapters (merged to main via PR #527, 220 tests passing). Adding Slack requires a new `SlackChannel` adapter using `slack-morphism` (v2.18.0) with Socket Mode -- no HTTP endpoint exposure needed. The adapter follows the exact same pattern as the existing Telegram/Discord adapters: implement the `Channel` trait, add a feature-gated dependency, extend config, and wire into `build_channels_from_config()`.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Multi-channel reach is core TinyClaw value; user explicitly requested Slack |
| Leverages strengths? | Yes | Existing Channel trait + MessageBus + format module provide 80% of the plumbing |
| Meets real need? | Yes | Slack is the dominant team chat platform; #519 lists it as Phase 2+ (now is Phase 2) |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
Add Slack as a channel adapter to TinyClaw so users can interact with the Terraphim AI assistant from Slack DMs and channels.

### Impact
- Slack is the primary team communication tool for most engineering organizations
- Without Slack support, TinyClaw is limited to Telegram (personal), Discord (community), and CLI (developer)
- Slack integration unlocks enterprise/team use cases

### Success Criteria
1. SlackChannel implements the Channel trait
2. Socket Mode connection handles DMs and channel mentions
3. Message formatting converts markdown to Slack Block Kit / mrkdwn
4. Allowlist-based access control (consistent with Telegram/Discord pattern)
5. Feature-gated (`slack` feature flag) to avoid pulling dependencies when not needed
6. Tests pass without requiring a live Slack workspace

## Current State Analysis

### Existing Implementation

TinyClaw was implemented in full (PR #527, merged 2026-02-13) with 220+ tests. The crate lives at `crates/terraphim_tinyclaw/` on the `main` branch.

**NOTE**: The current working branch (`dependabot/github_actions/docker/build-push-action-6`) does NOT contain the tinyclaw crate. Work must be done from `main` or a branch off `main`.

### Code Locations

| Component | Location (on main) | Purpose |
|-----------|----------|---------|
| Channel trait | `crates/terraphim_tinyclaw/src/channel.rs` | `Channel` trait + `ChannelManager` + `build_channels_from_config()` |
| Message bus | `crates/terraphim_tinyclaw/src/bus.rs` | `InboundMessage` / `OutboundMessage` + tokio mpsc bus |
| Telegram adapter | `crates/terraphim_tinyclaw/src/channels/telegram.rs` | Reference implementation (~150 LOC) |
| Discord adapter | `crates/terraphim_tinyclaw/src/channels/discord.rs` | Second reference (~130 LOC) |
| Channels mod | `crates/terraphim_tinyclaw/src/channels/mod.rs` | Feature-gated module declarations |
| Config | `crates/terraphim_tinyclaw/src/config.rs` | `ChannelsConfig` + per-channel config structs |
| Format | `crates/terraphim_tinyclaw/src/format.rs` | `markdown_to_telegram_html()`, `chunk_message()` |

### Data Flow

```
Slack message (Socket Mode WebSocket)
  -> SlackChannel::start() spawned task
  -> Parse event, check allowlist
  -> InboundMessage::new("slack", sender_id, channel_id, text)
  -> bus.inbound_sender().send(inbound)
  -> [agent loop processes, produces OutboundMessage]
  -> ChannelManager::send(outbound)
  -> SlackChannel::send(msg)
  -> Slack Web API: chat.postMessage / chat.update
```

### Integration Points
- `Channel` trait (5 methods: `name`, `start`, `stop`, `send`, `is_running`, `is_allowed`)
- `MessageBus` (tokio mpsc, 1000-capacity bounded channels)
- `build_channels_from_config()` in `channel.rs` (factory function)
- `ChannelsConfig` struct in `config.rs` (feature-gated fields)
- `format.rs` for platform-specific formatting + chunking

## Constraints

### Technical Constraints
- **Socket Mode preferred**: No public HTTP endpoint needed (matches TinyClaw's local-first philosophy)
- **slack-morphism v2.18.0**: Only mature Rust Slack library; uses tokio, hyper, serde -- compatible with existing stack
- **Feature-gated**: Must be behind `slack` feature flag (like telegram/discord)
- **Slack message limit**: 4000 characters per message (Block Kit sections: 3000 chars)
- **Slack ack timeout**: Callbacks must respond within 2-3 seconds or Slack retries; use `tokio::spawn` for heavy processing
- **Two tokens required**: `botToken` (xoxb-) for API calls + `appToken` (xapp-) for Socket Mode

### Business Constraints
- User wants this as the next step -- prioritize working integration over completeness
- Terraphim uses 1Password for secrets management (tokens should come from env vars / `op inject`)

### Non-Functional Requirements

| Requirement | Target | Rationale |
|-------------|--------|-----------|
| Startup latency | < 3s | Socket Mode WebSocket connection |
| Message delivery | < 1s | After agent loop completes |
| Memory per channel | < 5MB | Consistent with other adapters |
| Reconnection | Automatic | slack-morphism handles multi-connection resilience |

## Vital Few (Max 3 Essential Constraints)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Socket Mode (no HTTP endpoint) | TinyClaw is a local/personal assistant -- no public server | Design doc #519, local-first architecture |
| Feature-gated dependency | slack-morphism pulls in hyper/tokio-tungstenite -- must not bloat default builds | Existing pattern: telegram/discord are feature-gated |
| 2-3s ack deadline | Slack will retry if callbacks block; must spawn async processing | slack-morphism docs explicitly warn about this |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| HTTP Events API mode | Socket Mode sufficient; no public endpoint needed |
| Thread support | Phase 2; basic DM + channel mention first |
| Slash command registration | Phase 2; use TinyClaw's existing /command routing via message text |
| Block Kit rich messages | Phase 2; plain mrkdwn formatting sufficient initially |
| Streaming/typing indicators | Phase 2; OpenClaw supports this but not essential for MVP |
| Multi-workspace support | Phase 2; single workspace first |
| Channel access policies | Phase 2; allowlist per user sufficient initially |
| File/media uploads | Phase 2; text messages first |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| Channel trait (`channel.rs`) | Must implement 6 methods | Low -- stable, well-tested |
| MessageBus (`bus.rs`) | Send InboundMessage, receive OutboundMessage | Low -- stable |
| format module | Need `markdown_to_slack()` + chunk_message for 4000 char limit | Low -- extend existing |
| Config (`config.rs`) | Add `SlackConfig` + wire into `ChannelsConfig` | Low -- follows pattern |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| slack-morphism | 2.18.0 | Low -- actively maintained, Feb 2025 release | None in Rust ecosystem |
| tokio-tungstenite | (transitive via slack-morphism) | Low | N/A |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| slack-morphism API differs from docs | Low | Medium | Check examples, test with real workspace |
| Socket Mode requires Slack app with specific scopes | Medium | Low | Document required scopes in README |
| Dependency conflicts with existing crates | Low | Medium | Feature-gate; test with `--all-features` |
| Slack rate limits on message send | Low | Low | chunk_message already handles this pattern |

### Open Questions

1. **Which Slack app scopes are needed?** -- Research Slack App configuration docs (bot events: `app_mention`, `message.im`, `message.channels`; OAuth scopes: `chat:write`, `connections:write`)
2. **Should we support Slack thread replies from the start?** -- Recommend NO for MVP; add in Phase 2
3. **How does slack-morphism expose message events in Socket Mode?** -- Via `SlackPushEventCallback` with `SlackEventCallbackBody::Message` variant

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| slack-morphism Socket Mode works with tokio 1.x | Docs + examples use tokio | Would need alternative library | Yes (docs confirm) |
| Socket Mode handles reconnection automatically | Docs: "multiple connections per token (default: 2)" | Would need manual reconnect logic | Yes (docs) |
| Slack mrkdwn is close enough to standard markdown | Slack docs | Would need custom formatter | Partially -- differs in link syntax |
| Single workspace is sufficient for MVP | User request context | Would need multi-workspace support sooner | No -- confirm with user |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| A: Add Slack to existing terraphim_agent | Would mix TUI/REPL concerns with chat adapters | Rejected -- terraphim_agent is a CLI tool, not a chat gateway |
| B: Add Slack adapter to terraphim_tinyclaw (Channel trait) | Clean separation, follows existing pattern | **Chosen** -- consistent with design |
| C: Create standalone terraphim_slack crate | Over-engineering for a single adapter | Rejected -- adapters belong in tinyclaw |
| D: Reuse SLB TS Slack client as sidecar | Would add Node.js dependency, defeats Rust-native goal | Rejected -- TinyClaw is a single binary |
| E: Use HTTP Events API (like SLB) instead of Socket Mode | Requires public endpoint, reverse proxy, webhook verification | Rejected -- TinyClaw runs locally, Socket Mode is simpler |

### Socket Mode vs HTTP Events API Decision

The SLB production experience provides important context:

| Factor | Socket Mode (TinyClaw) | HTTP Events API (SLB) |
|--------|----------------------|----------------------|
| Public endpoint | NOT required | Required (webhook URL) |
| Deployment | Local binary, laptop, VPS | Cloudflare Workers, serverless |
| Connection model | Persistent WebSocket | Stateless HTTP POST per event |
| Reconnection | Automatic (slack-morphism: 2 connections) | N/A (stateless) |
| Ack mechanism | WebSocket ack frame | HTTP 200 within 3 seconds |
| Signature verification | Not needed (app-level token authenticates) | Required (HMAC signing secret) |
| Multi-workspace | One connection per workspace | One webhook URL serves all |
| Complexity | Lower (no server, no TLS, no DNS) | Higher (needs public URL, HTTPS, verification) |

**Decision**: Socket Mode for TinyClaw. The SLB uses HTTP Events because Cloudflare Workers cannot maintain WebSocket connections. TinyClaw is a long-running local binary where Socket Mode is the natural fit.

## Research Findings

### Key Insights

1. **TinyClaw is production-ready on main** -- 220+ tests, Telegram + Discord adapters working. The channel-trait pattern is proven and stable.

2. **slack-morphism is the only viable Rust Slack library** -- v2.18.0 (Feb 2025), actively maintained, supports Socket Mode with automatic reconnection. Uses tokio, compatible with TinyClaw's async runtime.

3. **The adapter pattern is mechanical** -- Each existing adapter follows the same ~130-150 LOC pattern:
   - Struct holding config + `Arc<AtomicBool>` for running state
   - `start()` spawns a tokio task that listens for events and forwards to `bus.inbound_sender()`
   - `send()` calls the platform API to deliver messages
   - `is_allowed()` delegates to `is_sender_allowed()` helper

4. **Slack-specific considerations**:
   - Socket Mode requires TWO tokens: bot token (xoxb-) + app-level token (xapp-)
   - Messages must be acked within 2-3 seconds (spawn processing as separate task)
   - Slack uses `mrkdwn` (not standard markdown): `*bold*` (not `**bold**`), `_italic_`, `~strike~`, `<url|text>` for links
   - Message limit is 4000 chars (section blocks: 3000)

5. **OpenClaw Slack reference** -- OpenClaw supports Socket Mode + HTTP Events API, threading, slash commands, streaming, channel policies. For TinyClaw MVP, only Socket Mode + basic DM/mention handling is needed.

### Relevant Prior Art

| Project | Location | Relevance |
|---------|----------|-----------|
| [slack-morphism](https://github.com/abdolence/slack-morphism-rust) | External | Primary Rust library; Socket Mode + events + Block Kit |
| [OpenClaw Slack channel](https://docs.openclaw.ai/channels/slack) | External | Full-featured reference; config format, threading, policies |
| TinyClaw Telegram adapter | `crates/terraphim_tinyclaw/src/channels/telegram.rs` (main) | Direct template for Slack adapter structure |
| TinyClaw Discord adapter | `crates/terraphim_tinyclaw/src/channels/discord.rs` (main) | Second template showing simpler send pattern |
| **Slack-Linear Bridge (TS)** | `~/Projects/zestic/slack-linear-bridge/` | **Production Slack bot** -- shipped, multi-tenant, HTTP Events API on Cloudflare Workers |
| **Slack-Linear Bridge (Rust)** | `~/Projects/zestic-ai/slack-linear-bridge-rust/` | Phase 4 verified Rust migration -- 7-crate workspace, `SlackApi` trait with noop impl |
| OpenClaw Use Cases Analysis | `~/cto-executive-system/docs/OPENCLAW_USECASES_ANALYSIS.md` | Multi-agent patterns, Telegram routing, scheduled tasks |
| OpenClaw Kimiko Workspace | `~/cto-executive-system/knowledge/openclaw-workspace-kimiko.md` | Agent identity stack, multi-channel patterns |
| SLB ADR Suite | `~/cto-executive-system/projects/slack-linear-bridge/adr/` | 6 ADRs covering SDK strategy, LLM, deployment, multi-tenancy, architecture |

### Key Findings from cto-executive-system Investigation

**1. Production Slack Bot Already Exists (TypeScript)**

The `slack-linear-bridge` project at `~/Projects/zestic/slack-linear-bridge/` is a **shipped, production** Slack bot:
- Cloudflare Workers + Hono + D1 + Queues
- HTTP Events API (NOT Socket Mode -- Workers has 90s connection limit)
- Multi-tenant: tenant resolved by channel mapping
- Anthropic Agent SDK with 9 tool skills
- Event dedup via KV, rate limiting per tenant
- Ack within 3s via queue-based async processing
- `src/slack/client.ts`: thin wrapper around `chat.postMessage`, `reactions.add/remove`, `users.info`
- `src/routes/slack-events.ts`: webhook receiver with signature verification, bot-loop prevention, dedup

**2. Rust Migration at Phase 4 (Not Production)**

The `slack-linear-bridge-rust` project has a 7-crate Rust workspace:
- `bridge-integrations/src/slack.rs` defines a `SlackApi` trait with `post_final_message()` -- currently `NoopSlackApi`
- Real Slack API client NOT yet implemented in Rust
- Linear integration uses TS sidecar adapter pattern
- 15 tests passing, quality gates green
- Strategic decision: "Run TypeScript in production now. Migrate to Rust when triggers met."

**3. Two Distinct Use Cases**

| Aspect | Slack-Linear Bridge | TinyClaw Slack |
|--------|-------------------|----------------|
| Purpose | Project management bot (Linear CRUD, spec interviews) | Personal AI assistant (search, tools, KG) |
| Deployment | Cloudflare Workers (serverless, multi-tenant) | Local binary (single-user, Socket Mode) |
| Slack API | HTTP Events API (webhooks) | Socket Mode (WebSocket, no public endpoint) |
| LLM | Anthropic Agent SDK (TS) | terraphim-llm-proxy / Ollama |
| Auth | Slack signing secret + OAuth | Bot token + App token + allowlist |
| Threading | Full thread support (thread_ts) | MVP: flat messages only |
| Reactions | Eyes/checkmark ack pattern | Not in MVP |

**4. Reusable Patterns from SLB**

From the production TS implementation, these patterns should inform TinyClaw Slack:
- **Bot message filtering**: `if event.bot_id || event.subtype === "bot_message"` -- prevent self-loops
- **Ack-first, process-async**: Queue/spawn pattern to respond within 3s
- **User info enrichment**: Resolve Slack user ID to display name for session context
- **Reaction-based status**: `eyes` on receipt, `white_check_mark` on completion (Phase 2 for TinyClaw)
- **Message dedup**: Event ID dedup to handle Slack retries (important for Socket Mode too)

**5. No NanoClaw Implementation Found**

No nanoclaw-specific code or documentation exists in `~/cto-executive-system/`. The term "nanoclaw" appears only in external GitHub repos (forks/alternatives to OpenClaw). There is no Terraphim nanoclaw project.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| slack-morphism Socket Mode hello world | Verify library works with our tokio setup | 1-2 hours |
| Slack App creation + token provisioning | Get bot token + app token for testing | 30 min |
| Review SLB bot-loop prevention | Ensure TinyClaw handles Slack retry/dedup correctly | 30 min |
| NanoClaw outgoing queue pattern | Evaluate if TinyClaw needs pre-connect message buffering | 30 min |

### NanoClaw Evaluation (github.com/qwibitai/nanoclaw)

NanoClaw is a TypeScript AI agent framework with multi-channel support. Its Slack adapter provides a well-tested reference implementation.

#### NanoClaw Architecture Overview

| Aspect | NanoClaw | TinyClaw |
|--------|----------|----------|
| Language | TypeScript (Node.js) | Rust |
| Slack library | `@slack/bolt` (official SDK) | `slack-morphism` (community Rust lib) |
| Transport | Socket Mode (`socketMode: true`) | Socket Mode (same) |
| Channel abstraction | `Channel` interface: `connect/sendMessage/disconnect/ownsJid` | `Channel` trait: `start/stop/send/is_running/is_allowed` |
| Registration | Factory pattern: `registerChannel(name, factory)` returns `Channel \| null` | `build_channels_from_config()` in ChannelManager |
| Config approach | `.env` file (dotenv-safe) | TOML with `${VAR}` expansion |
| Tokens | `SLACK_BOT_TOKEN` + `SLACK_APP_TOKEN` from .env | Same two tokens from TOML/env vars |
| Message limit | 4000 chars (chunked) | 4000 chars (chunked) -- same |
| Test framework | vitest with mocked `@slack/bolt` | `#[tokio::test]` -- no mocks (project policy) |
| Skills system | Git-merged `.claude/skills/add-slack/` | Feature-gated `#[cfg(feature = "slack")]` |

#### NanoClaw Slack Patterns Worth Adopting

**1. Bot Self-Detection** (MUST HAVE for MVP)
```typescript
// NanoClaw: fetch bot user ID on connect, filter own messages
const authResult = await this.app.client.auth.test();
this.botUserId = authResult.user_id;
// In message handler:
if (event.bot_id || event.user === this.botUserId) return; // skip own messages
```
TinyClaw equivalent: Call `auth.test` after Socket Mode connect, store bot user ID, filter in event handler.

**2. @Mention Translation** (SHOULD HAVE for MVP)
```typescript
// NanoClaw: translate <@UBOTID> to @AssistantName in incoming messages
if (event.text.includes(`<@${this.botUserId}>`)) {
    text = text.replace(`<@${this.botUserId}>`, '').trim();
    text = `@${this.assistantName} ${text}`;
}
```
TinyClaw equivalent: Strip `<@BOT_ID>` from incoming text, optionally prepend assistant name.

**3. User Name Resolution with Cache** (SHOULD HAVE for MVP)
```typescript
// NanoClaw: resolve Slack user ID to display name, cache in Map
private userNameCache = new Map<string, string>();
async getUserName(userId: string): Promise<string> {
    if (this.userNameCache.has(userId)) return this.userNameCache.get(userId)!;
    const result = await this.app.client.users.info({ user: userId });
    const name = result.user?.profile?.display_name || result.user?.real_name || userId;
    this.userNameCache.set(userId, name);
    return name;
}
```
TinyClaw equivalent: `HashMap<String, String>` behind `RwLock`, populate on first encounter.

**4. Channel JID Ownership** (NICE TO HAVE)
```typescript
// NanoClaw: prefix-based routing
ownsJid(jid: string): boolean { return jid.startsWith('slack:'); }
```
TinyClaw equivalent: Not needed -- TinyClaw uses `channel: String` field in `InboundMessage` for routing.

**5. Outgoing Message Queue** (DEFER to Phase 2)
```typescript
// NanoClaw: buffer messages before connection, flush on connect
private outgoingQueue: Array<{jid: string, text: string}> = [];
async sendMessage(jid, text) {
    if (!this.connected) { this.outgoingQueue.push({jid, text}); return; }
    // ... send via API
}
async connect() {
    // ... after connected:
    for (const msg of this.outgoingQueue) await this.sendMessage(msg.jid, msg.text);
    this.outgoingQueue = [];
}
```
TinyClaw: MessageBus already handles queueing via tokio mpsc. Outgoing queue only needed if Slack connection drops mid-session.

**6. Channel Metadata Sync** (DEFER to Phase 2)
```typescript
// NanoClaw: sync channel list with cursor pagination on startup
async syncChannelMetadata() {
    let cursor;
    do {
        const result = await this.app.client.conversations.list({ cursor, limit: 200 });
        // ... sync channel names to DB
        cursor = result.response_metadata?.next_cursor;
    } while (cursor);
}
```
TinyClaw: Not needed for MVP (direct messages + mention handling sufficient).

**7. Typing Indicator** (NOT POSSIBLE)
```typescript
// NanoClaw: no-op -- Slack bot API does not support typing indicators
async setTyping(): Promise<void> { /* no-op */ }
```
TinyClaw: Same limitation. No typing indicator for bots in Slack API.

#### NanoClaw vs TinyClaw: Key Differences

| Pattern | NanoClaw Approach | TinyClaw Approach | Decision |
|---------|------------------|-------------------|----------|
| Channel lifecycle | `connect()` / `disconnect()` | `start(bus)` / `stop()` | TinyClaw -- bus injection is cleaner |
| Message routing | JID prefix (`slack:channelId`) | `channel` field in InboundMessage | TinyClaw -- explicit field is type-safe |
| Config validation | Runtime null checks on env vars | `validate()` method with anyhow errors | TinyClaw -- compile-time feature gates + validation |
| Error handling | try/catch with console.error | `Result<T, anyhow::Error>` propagation | TinyClaw -- Rust error handling is stronger |
| Bot detection | `auth.test()` + `bot_id` field check | Same pattern via slack-morphism | Adopt NanoClaw pattern |
| Message chunking | Manual split at 4000 chars | Reuse existing `chunk_message()` helper | TinyClaw -- helper already exists |
| Testing | vitest with full mocks | Real integration tests (no mocks policy) | TinyClaw -- more reliable but needs tokens |
| Graceful null factory | `registerChannel()` returns null if no creds | Feature gate excludes at compile time | TinyClaw -- zero-cost when disabled |

#### NanoClaw Evaluation Summary

NanoClaw's Slack adapter is a clean ~250 LOC implementation that validates our planned approach. The key takeaways:

1. **Socket Mode is correct**: Both NanoClaw and our plan use Socket Mode. This is validated.
2. **Bot self-detection is essential**: NanoClaw's `auth.test()` + message filtering pattern MUST be adopted. Without it, the bot creates infinite message loops.
3. **@mention stripping improves UX**: NanoClaw strips `<@BOTID>` from messages before processing. This prevents the LLM from seeing raw Slack mention syntax.
4. **User name cache is practical**: Simple in-memory cache avoids repeated API calls. ~10 LOC in Rust.
5. **Outgoing queue is premature for MVP**: TinyClaw's MessageBus handles this. Only needed if we want to buffer while Slack reconnects.
6. **Channel metadata sync is Phase 2**: Only needed for multi-channel routing or channel picker UI.
7. **4000 char limit confirmed**: Same as our plan. `chunk_message()` helper already exists in TinyClaw.

#### Updated MVP Scope (Post-NanoClaw Evaluation)

Based on NanoClaw patterns, add to MVP scope:
- **Bot self-detection** via `auth.test()` on connect -- MUST HAVE
- **@mention stripping** from incoming messages -- SHOULD HAVE
- **User name resolution** with in-memory cache -- SHOULD HAVE
- **Message dedup** for Slack retries (event ID tracking) -- SHOULD HAVE

Defer to Phase 2:
- Outgoing message queue (pre-connect buffering)
- Channel metadata sync
- Thread reply support
- Reaction-based status indicators

## Recommendations

### Proceed/No-Proceed

**Proceed**. The channel-trait pattern is proven, slack-morphism is mature, and the adapter is mechanical (~200 LOC new code). Estimated effort: 4-6 hours including tests.

### Scope Recommendations (MVP -- Updated Post-NanoClaw Evaluation)

1. `SlackChannel` implementing `Channel` trait via Socket Mode
2. `SlackConfig` with `bot_token`, `app_token`, `allow_from`
3. Handle: DMs + channel messages where bot is mentioned
4. `markdown_to_slack_mrkdwn()` formatting function
5. `chunk_message()` reuse with 4000 char limit
6. Feature flag: `slack = ["dep:slack-morphism"]`
7. **Bot self-detection**: `auth.test()` on connect, filter own messages (from NanoClaw)
8. **@mention stripping**: Remove `<@BOT_ID>` from incoming text (from NanoClaw)
9. **User name cache**: `HashMap<String, String>` behind `RwLock` for display names (from NanoClaw)
10. **Message dedup**: Track event IDs to handle Slack retries (from SLB)
11. Unit tests (config validation, formatting, allowlist, bot detection)
12. Integration test scaffold (feature-gated, requires tokens)

### Phase 2 (Future)

- Thread reply support (`OutboundMessage.reply_to` -> Slack `thread_ts`)
- Slash command handling
- Block Kit rich formatting
- Typing/ack reaction indicators
- File/media upload support
- Multi-workspace support
- Channel access policies (requireMention, allowlist/denylist per channel)

### Risk Mitigation

- Feature-gate to avoid dependency bloat
- Socket Mode avoids public endpoint exposure
- Allowlist security matches existing pattern
- Use `tokio::spawn` for message processing to meet 2-3s ack deadline

## Next Steps

If approved:
1. Branch from `main` (NOT current dependabot branch)
2. Create GitHub issue for Slack adapter implementation
3. Proceed to Phase 2 (Design) -- detailed implementation plan
4. Implement following the Channel trait pattern
5. Test with real Slack workspace

## Appendix

### Slack App Required Configuration

**OAuth Bot Token Scopes:**
- `chat:write` -- Send messages
- `app_mentions:read` -- Receive mention events
- `im:read` -- Read DM messages
- `im:history` -- Access DM history
- `channels:read` -- List channels
- `connections:write` -- Socket Mode connections

**Event Subscriptions (Bot Events):**
- `app_mention` -- When bot is @mentioned in a channel
- `message.im` -- DM messages to the bot

**Socket Mode:** Must be enabled in app settings

### slack-morphism Cargo.toml

```toml
[dependencies]
slack-morphism = { version = "2.18", features = ["hyper_tokio"] }
```

### Reference: Channel Trait

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    async fn start(&self, bus: Arc<MessageBus>) -> anyhow::Result<()>;
    async fn stop(&self) -> anyhow::Result<()>;
    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()>;
    fn is_running(&self) -> bool;
    fn is_allowed(&self, sender_id: &str) -> bool;
}
```

### Reference: Config Pattern

```rust
// In ChannelsConfig
#[cfg(feature = "slack")]
pub slack: Option<SlackConfig>,

// SlackConfig struct
pub struct SlackConfig {
    pub bot_token: String,   // xoxb-...
    pub app_token: String,   // xapp-... (Socket Mode)
    pub allow_from: Vec<String>,
}
```

### Reference: TOML Config

```toml
[channels.slack]
bot_token = "${SLACK_BOT_TOKEN}"
app_token = "${SLACK_APP_TOKEN}"
allow_from = ["U01234567", "username"]
```

### Sources

- [slack-morphism-rust GitHub](https://github.com/abdolence/slack-morphism-rust)
- [slack-morphism Socket Mode docs](https://slack-rust.abdolence.dev/socket-mode.html)
- [slack-morphism crates.io](https://crates.io/crates/slack-morphism)
- [OpenClaw Slack Channel docs](https://docs.openclaw.ai/channels/slack)
- [Slack API Community Tools](https://api.slack.com/community)
- [NanoClaw GitHub](https://github.com/qwibitai/nanoclaw)
- [NanoClaw Slack adapter](https://github.com/qwibitai/nanoclaw/.claude/skills/add-slack/add/src/channels/slack.ts)
- [NanoClaw SPEC.md](https://github.com/qwibitai/nanoclaw/blob/main/SPEC.md)
- [Slack-Linear Bridge (production TS)](~/Projects/zestic/slack-linear-bridge/)
- [Slack-Linear Bridge (Rust migration)](~/Projects/zestic-ai/slack-linear-bridge-rust/)
- [SLB ADR Suite](~/cto-executive-system/projects/slack-linear-bridge/adr/)
