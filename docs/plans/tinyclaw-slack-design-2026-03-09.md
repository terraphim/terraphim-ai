# Implementation Plan: TinyClaw Slack Channel Adapter

**Status**: Draft
**Research Doc**: [docs/plans/tinyclaw-slack-research-2026-03-09.md](tinyclaw-slack-research-2026-03-09.md)
**Author**: Terraphim AI
**Date**: 2026-03-09
**Estimated Effort**: 8-10 hours (public repo), 6-8 hours (twin-slack in private repo)

## Overview

### Summary

Add a Slack channel adapter to `terraphim_tinyclaw` following the existing Channel trait pattern.
The adapter uses `slack-morphism` for Socket Mode (WebSocket) connectivity with bot self-detection,
@mention stripping, user name caching, and message dedup. Testing uses the private `twin-slack`
digital twin -- no mock objects, no Slack tokens in CI.

### Approach

Mechanical extension: copy the Telegram adapter pattern, replace teloxide calls with slack-morphism
equivalents. Feature-gated behind `slack = ["dep:slack-morphism"]`. The `api_base_url` override
approach from research is NOT viable because slack-morphism hardcodes `SLACK_API_URI_STR`. Instead,
the testing seam is a `SlackApiClient` trait that wraps the three API calls we need (auth.test,
chat.postMessage, users.info), with a production implementation using slack-morphism and a
twin-compatible implementation using plain reqwest in the private test harness.

### Scope

**In Scope:**
1. `SlackChannel` struct implementing `Channel` trait
2. `SlackConfig` struct with validation
3. Socket Mode event listener (message.im + app_mention)
4. Bot self-detection via auth.test
5. @mention stripping from incoming text
6. User name resolution with in-memory cache
7. Message dedup via event ID tracking
8. Markdown-to-Slack-mrkdwn formatting
9. Message chunking at 4000 chars
10. Feature flag and Cargo.toml changes
11. Unit tests (all logic, no network, public CI)
12. Integration test scaffold (#[ignore], env-var-gated)

**Out of Scope:**
- Thread reply support
- Slash command handling
- Block Kit rich formatting
- Reaction-based status indicators
- File/media upload
- Multi-workspace support
- Channel access policies
- Outgoing message queue (pre-connect buffering)
- Channel metadata sync

**Avoid At All Cost** (5/25 analysis):
- Block Kit message builder (over-engineering for text-only MVP)
- Slack interactive components (buttons, modals)
- OAuth installation flow (single-workspace bot, manual token setup)
- Unfurling / link previews
- Slack app manifest auto-provisioning
- Custom slash command framework
- Workspace-level admin features
- Message editing / deletion handling
- Presence / online status
- Custom emoji handling

## Architecture

### Component Diagram

```
                    terraphim_tinyclaw (public, open source)
 +---------------------------------------------------------------+
 |                                                                |
 |  SlackConfig ----> SlackChannel ----> Channel trait            |
 |       |                |       |                               |
 |       |        +-------+-------+-------+                      |
 |       |        |       |       |       |                       |
 |       v        v       v       v       v                       |
 |  validate()  start()  stop()  send()  is_running()            |
 |               |                |                               |
 |       +-------+------+  +-----+------+                        |
 |       |              |  |            |                         |
 |   SocketMode     auth.test   chat.postMessage                 |
 |   listener         |              |                            |
 |       |         bot_user_id   chunk_message()                  |
 |       v             |         mrkdwn_format()                  |
 |   message event     v                                          |
 |       |        filter_own()                                    |
 |       v             |                                          |
 |   strip_mention()   |                                          |
 |       |             |                                          |
 |       v             |                                          |
 |   resolve_user_name() (cache)                                  |
 |       |                                                        |
 |       v                                                        |
 |   dedup_event()                                                |
 |       |                                                        |
 |       v                                                        |
 |   InboundMessage --> MessageBus                                |
 |                                                                |
 +---------------------------------------------------------------+

                    zestic-ai/digital-twins (PRIVATE)
 +---------------------------------------------------------------+
 |   twin-slack                                                   |
 |     auth.test   --> returns configured bot_user_id             |
 |     chat.postMessage --> stores in DashMapStore                |
 |     users.info  --> returns configurable profiles              |
 |     (Phase 2: WebSocket Socket Mode simulation)                |
 +---------------------------------------------------------------+
```

### Data Flow

```
[Slack workspace] --(Socket Mode WS)--> [slack-morphism listener]
    --> [event handler: filter bot, dedup, strip mention, resolve name]
    --> [InboundMessage::new("slack", sender_id, chat_id, cleaned_text)]
    --> [MessageBus.inbound_tx]

[MessageBus.outbound_rx] --> [ChannelManager.send()]
    --> [SlackChannel.send(OutboundMessage)]
    --> [markdown_to_slack_mrkdwn()]
    --> [chunk_message(4000)]
    --> [slack-morphism chat.postMessage per chunk]
    --> [Slack workspace]
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| slack-morphism for Slack API | Only mature Rust Slack library with Socket Mode | Raw reqwest (too much boilerplate), @slack/bolt via FFI (absurd) |
| Socket Mode (not HTTP Events) | TinyClaw is a local binary, no public endpoint | HTTP Events API requires public URL, TLS, webhook verification |
| In-memory user name cache | Simple, sufficient for single-user bot | Database cache (over-engineering), no cache (API rate limit risk) |
| HashSet for event dedup | Simple, bounded by session lifetime | LRU cache (premature), database (over-engineering) |
| Feature gate `slack` | Zero cost when disabled, follows existing pattern | Always-on (bloats binary), runtime config only (still compiles dep) |
| No `api_base_url` override | slack-morphism hardcodes `SLACK_API_URI_STR` | Forking slack-morphism (maintenance burden), monkey-patching (fragile) |
| Testing via digital twin at HTTP level | Complies with "no mocks" policy, reusable by SLB | Mocks (prohibited), real Slack only (no CI), forking slack-morphism |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Block Kit message builder | Plain text + mrkdwn sufficient for MVP | Doubles API surface, 3x code for formatting |
| OAuth installation flow | Single workspace, manual token setup is fine | Weeks of work for a feature used once |
| Thread reply support | Flat messages work for personal assistant | Complicates OutboundMessage, needs thread_ts tracking |
| Slack interactive components | Bot only sends/receives text | Massive API surface (buttons, modals, views) |
| Custom hyper connector for URL rewrite | Complex, fragile, depends on slack-morphism internals | Breaks on library updates, hard to debug |

### Simplicity Check

> **What if this could be easy?**

Copy `telegram.rs` (150 LOC). Replace `teloxide::Bot` with `SlackClient`. Replace
`teloxide::Dispatcher` with Socket Mode listener. Replace `markdown_to_telegram_html` with
`markdown_to_slack_mrkdwn`. Add ~50 LOC for bot detection + mention stripping + name cache +
dedup. Total: ~200 LOC new code + ~80 LOC tests.

**Senior Engineer Test**: A senior engineer would look at the Telegram adapter, look at
slack-morphism docs, and produce the same design. Nothing clever here.

**Nothing Speculative Checklist**:
- [x] No features the user did not request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_tinyclaw/src/channels/slack.rs` | Slack channel adapter (~200 LOC) |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_tinyclaw/Cargo.toml` | Add `slack-morphism` dep + `slack` feature |
| `crates/terraphim_tinyclaw/src/channels/mod.rs` | Add `#[cfg(feature = "slack")] pub mod slack;` |
| `crates/terraphim_tinyclaw/src/config.rs` | Add `SlackConfig` struct + field in `ChannelsConfig` |
| `crates/terraphim_tinyclaw/src/channel.rs` | Add slack branch in `build_channels_from_config()` |
| `crates/terraphim_tinyclaw/src/format.rs` | Add `markdown_to_slack_mrkdwn()` function |

### Deleted Files

None.

## API Design

### Public Types

```rust
// In config.rs

/// Slack channel configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackConfig {
    /// Bot token (xoxb-...) from Slack App settings.
    pub bot_token: String,

    /// App-level token (xapp-...) for Socket Mode connections.
    pub app_token: String,

    /// List of allowed sender IDs (Slack user IDs like "U01234567").
    /// Must be non-empty for security.
    pub allow_from: Vec<String>,
}
```

```rust
// In channels/slack.rs

/// Slack channel adapter using slack-morphism Socket Mode.
pub struct SlackChannel {
    config: SlackConfig,
    running: Arc<AtomicBool>,
}
```

### Public Functions

```rust
// In config.rs
impl SlackConfig {
    /// Validate the Slack configuration.
    ///
    /// # Errors
    /// Returns error if bot_token, app_token are empty or allow_from is empty.
    pub fn validate(&self) -> anyhow::Result<()>;

    /// Check if a sender is allowed.
    pub fn is_allowed(&self, sender_id: &str) -> bool;
}
```

```rust
// In channels/slack.rs
impl SlackChannel {
    /// Create a new Slack channel adapter.
    pub fn new(config: SlackConfig) -> Self;
}

// Channel trait implementation (start, stop, send, is_running, is_allowed)
```

```rust
// In format.rs

/// Convert markdown to Slack mrkdwn format.
///
/// Slack mrkdwn differences from standard markdown:
/// - Bold: *text* (not **text**)
/// - Italic: _text_ (same)
/// - Strikethrough: ~text~ (not ~~text~~)
/// - Code: `code` (same)
/// - Code block: ```code``` (same)
/// - Links: <url|text> (not [text](url))
/// - No nested formatting
pub fn markdown_to_slack_mrkdwn(text: &str) -> String;
```

### Internal Functions (in slack.rs)

```rust
/// Strip bot @mention from incoming message text.
/// Converts "<@U_BOT_ID> hello" to "hello".
fn strip_bot_mention(text: &str, bot_user_id: &str) -> String;

/// Check if a message event is from the bot itself.
fn is_own_message(event_user: &str, event_bot_id: Option<&str>, bot_user_id: &str) -> bool;

/// Resolve a Slack user ID to a display name, using cache.
async fn resolve_user_name(
    client: &SlackHyperClient,
    token: &SlackApiToken,
    user_id: &str,
    cache: &RwLock<HashMap<String, String>>,
) -> String;

/// Check if an event has already been processed (dedup).
fn is_duplicate_event(event_id: &str, seen: &RwLock<HashSet<String>>) -> bool;
```

## Test Strategy

### Unit Tests (public CI, no network, no tokens)

| Test | Location | Purpose |
|------|----------|---------|
| `test_slack_config_validate_valid` | `config.rs` | Valid config passes |
| `test_slack_config_validate_empty_bot_token` | `config.rs` | Rejects empty bot_token |
| `test_slack_config_validate_empty_app_token` | `config.rs` | Rejects empty app_token |
| `test_slack_config_validate_empty_allow_from` | `config.rs` | Rejects empty allow_from |
| `test_slack_config_is_allowed` | `config.rs` | Allowlist matching |
| `test_slack_config_is_allowed_wildcard` | `config.rs` | Wildcard "*" allows all |
| `test_slack_channel_name` | `slack.rs` | Returns "slack" |
| `test_strip_bot_mention` | `slack.rs` | Strips `<@UBOTID>` from text |
| `test_strip_bot_mention_no_match` | `slack.rs` | Leaves text unchanged if no mention |
| `test_strip_bot_mention_multiple` | `slack.rs` | Handles multiple mentions |
| `test_is_own_message_by_user_id` | `slack.rs` | Detects own message by user match |
| `test_is_own_message_by_bot_id` | `slack.rs` | Detects own message by bot_id field |
| `test_is_own_message_other_user` | `slack.rs` | Allows other users' messages |
| `test_is_duplicate_event` | `slack.rs` | First occurrence passes, second blocked |
| `test_markdown_to_slack_mrkdwn_bold` | `format.rs` | `**bold**` -> `*bold*` |
| `test_markdown_to_slack_mrkdwn_strikethrough` | `format.rs` | `~~text~~` -> `~text~` |
| `test_markdown_to_slack_mrkdwn_link` | `format.rs` | `[text](url)` -> `<url\|text>` |
| `test_markdown_to_slack_mrkdwn_code` | `format.rs` | Backticks pass through |
| `test_chunk_message_slack` | `format.rs` | Chunks at 4000 chars |

### Integration Tests (public repo, #[ignore], env-var-gated)

| Test | Location | Purpose |
|------|----------|---------|
| `test_slack_channel_lifecycle` | `tests/slack_integration.rs` | start -> verify running -> stop |
| `test_slack_send_message` | `tests/slack_integration.rs` | Send message via channel, verify delivery |

These tests run against either twin-slack (private CI) or real Slack (manual validation).
Gated by `SLACK_BOT_TOKEN` + `SLACK_APP_TOKEN` env vars.

### E2E Tests (private repo, twin-slack, no tokens needed)

Located in `zestic-ai/digital-twins` -- not part of this plan. Separate work item.

## Implementation Steps

### Step 1: SlackConfig and Validation

**Files:** `crates/terraphim_tinyclaw/src/config.rs`
**Description:** Add `SlackConfig` struct, wire into `ChannelsConfig`, add validation
**Tests:** 6 unit tests for config validation and allowlist
**Estimated:** 1 hour

```rust
// Add to config.rs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackConfig {
    pub bot_token: String,
    pub app_token: String,
    pub allow_from: Vec<String>,
}

impl SlackConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.bot_token.is_empty() {
            anyhow::bail!("slack.bot_token cannot be empty");
        }
        if self.app_token.is_empty() {
            anyhow::bail!("slack.app_token cannot be empty");
        }
        if self.allow_from.is_empty() {
            anyhow::bail!(
                "slack.allow_from cannot be empty - \
                 at least one user must be authorized for security"
            );
        }
        Ok(())
    }

    pub fn is_allowed(&self, sender_id: &str) -> bool {
        crate::channel::is_sender_allowed(&self.allow_from, sender_id)
    }
}

// Add to ChannelsConfig:
#[cfg(feature = "slack")]
pub slack: Option<SlackConfig>,

// Add to ChannelsConfig::validate():
#[cfg(feature = "slack")]
if let Some(ref cfg) = self.slack {
    cfg.validate()?;
}
```

### Step 2: Slack mrkdwn Formatting

**Files:** `crates/terraphim_tinyclaw/src/format.rs`
**Description:** Add `markdown_to_slack_mrkdwn()` function
**Tests:** 5 unit tests for formatting conversions
**Dependencies:** None
**Estimated:** 1 hour

```rust
// Add to format.rs
pub fn markdown_to_slack_mrkdwn(text: &str) -> String {
    let mut result = text.to_string();

    // Code blocks (preserve, same syntax)
    // Must be handled first to avoid transforming code content

    // Bold: **text** -> *text*
    // Must happen before italic to avoid ambiguity
    while let Some(start) = result.find("**") {
        if let Some(end) = result[start + 2..].find("**") {
            let end = start + 2 + end;
            let content = &result[start + 2..end].to_string();
            result.replace_range(start..end + 2, &format!("*{}*", content));
        } else {
            break;
        }
    }

    // Strikethrough: ~~text~~ -> ~text~
    while let Some(start) = result.find("~~") {
        if let Some(end) = result[start + 2..].find("~~") {
            let end = start + 2 + end;
            let content = &result[start + 2..end].to_string();
            result.replace_range(start..end + 2, &format!("~{}~", content));
        } else {
            break;
        }
    }

    // Links: [text](url) -> <url|text>
    // (reuse link regex pattern)
    result = replace_markdown_links_to_slack(&result);

    result
}
```

### Step 3: Cargo.toml and Module Registration

**Files:** `Cargo.toml`, `channels/mod.rs`, `channel.rs`
**Description:** Add slack-morphism dependency, feature flag, module declaration, factory branch
**Tests:** Compile check
**Dependencies:** Step 1
**Estimated:** 30 min

```toml
# Add to Cargo.toml [dependencies]
slack-morphism = { version = "2.18", optional = true, features = ["hyper_tokio"] }

# Add to [features]
slack = ["dep:slack-morphism"]

# Update default (do NOT include slack in default -- opt-in)
# default = ["telegram", "discord"]  (unchanged)
```

```rust
// channels/mod.rs
#[cfg(feature = "slack")]
pub mod slack;

// channel.rs -- add to build_channels_from_config()
#[cfg(feature = "slack")]
{
    use crate::channels::slack::SlackChannel;
    if let Some(ref cfg) = config.slack {
        channels.push(Box::new(SlackChannel::new(cfg.clone())));
    }
}
```

### Step 4: SlackChannel Implementation

**Files:** `crates/terraphim_tinyclaw/src/channels/slack.rs`
**Description:** Core adapter -- Socket Mode listener, event handler, send method
**Tests:** 8 unit tests for helper functions (strip_mention, is_own_message, dedup)
**Dependencies:** Steps 1, 2, 3
**Estimated:** 4-5 hours

```rust
//! Slack channel adapter using slack-morphism Socket Mode.

use crate::bus::{InboundMessage, MessageBus, OutboundMessage};
use crate::channel::Channel;
use crate::config::SlackConfig;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

pub struct SlackChannel {
    config: SlackConfig,
    running: Arc<AtomicBool>,
}

impl SlackChannel {
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn start(&self, bus: Arc<MessageBus>) -> anyhow::Result<()> {
        self.running.store(true, Ordering::SeqCst);

        #[cfg(feature = "slack")]
        {
            use slack_morphism::prelude::*;

            let client = Arc::new(SlackClient::new(
                SlackClientHyperConnector::new()?,
            ));
            let token = SlackApiToken::new(self.config.bot_token.clone().into());

            // Fetch bot user ID for self-detection
            let session = client.open_session(&token);
            let auth_response = session.auth_test(&SlackApiAuthTestRequest::new()).await?;
            let bot_user_id = auth_response.user_id.to_string();
            log::info!("Slack bot user ID: {}", bot_user_id);

            let app_token = SlackApiToken::new(self.config.app_token.clone().into());
            let allow_from = self.config.allow_from.clone();
            let inbound_tx = bus.inbound_sender();
            let running = self.running.clone();

            // Shared state for dedup and user name cache
            let seen_events: Arc<RwLock<HashSet<String>>> =
                Arc::new(RwLock::new(HashSet::new()));
            let user_cache: Arc<RwLock<HashMap<String, String>>> =
                Arc::new(RwLock::new(HashMap::new()));

            tokio::spawn(async move {
                // Socket Mode listener setup
                let socket_mode_callbacks = SlackSocketModeListenerCallbacks::new()
                    .with_push_events(move |event, _client, _state| {
                        // Clone captures for async block
                        let tx = inbound_tx.clone();
                        let allowed = allow_from.clone();
                        let bot_id = bot_user_id.clone();
                        let seen = seen_events.clone();
                        let cache = user_cache.clone();
                        let client_clone = client.clone();
                        let token_clone = token.clone();

                        async move {
                            // Extract message event, filter, forward to bus
                            // (implementation in Step 4)
                            Ok(())
                        }
                    });

                let listener = SlackClientSocketModeListener::new(
                    &SlackClientSocketModeConfig::new(),
                    socket_mode_callbacks,
                    Arc::new(app_token),
                );

                if let Err(e) = listener.listen().await {
                    log::error!("Slack Socket Mode listener error: {}", e);
                }
                running.store(false, Ordering::SeqCst);
            });

            Ok(())
        }

        #[cfg(not(feature = "slack"))]
        {
            let _ = bus;
            anyhow::bail!("Slack feature not enabled")
        }
    }

    async fn stop(&self) -> anyhow::Result<()> {
        log::info!("Slack channel stopping");
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> anyhow::Result<()> {
        #[cfg(feature = "slack")]
        {
            use slack_morphism::prelude::*;

            let client = SlackClient::new(SlackClientHyperConnector::new()?);
            let token = SlackApiToken::new(self.config.bot_token.clone().into());
            let session = client.open_session(&token);

            let formatted = crate::format::markdown_to_slack_mrkdwn(&msg.content);
            let chunks = crate::format::chunk_message(&formatted, 4000);

            let channel_id: SlackChannelId = msg.chat_id.clone().into();

            for chunk in chunks {
                let req = SlackApiChatPostMessageRequest::new(
                    channel_id.clone(),
                    SlackMessageContent::new().with_text(chunk),
                );
                session.chat_post_message(&req).await?;
            }
            Ok(())
        }

        #[cfg(not(feature = "slack"))]
        {
            let _ = msg;
            anyhow::bail!("Slack feature not enabled")
        }
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.config.is_allowed(sender_id)
    }
}

// --- Helper functions (testable without network) ---

fn strip_bot_mention(text: &str, bot_user_id: &str) -> String {
    let mention = format!("<@{}>", bot_user_id);
    text.replace(&mention, "").trim().to_string()
}

fn is_own_message(event_user: &str, event_bot_id: Option<&str>, bot_user_id: &str) -> bool {
    event_user == bot_user_id || event_bot_id.is_some()
}

fn is_duplicate_event(event_id: &str, seen: &mut HashSet<String>) -> bool {
    !seen.insert(event_id.to_string())
}
```

### Step 5: Integration Test Scaffold

**Files:** `crates/terraphim_tinyclaw/tests/slack_integration.rs`
**Description:** Ignored integration tests that run against twin-slack or real Slack
**Tests:** 2 integration tests (lifecycle, send message)
**Dependencies:** Step 4
**Estimated:** 1 hour

```rust
//! Slack integration tests.
//! Run with: SLACK_BOT_TOKEN=xoxb-... SLACK_APP_TOKEN=xapp-... cargo test -p terraphim_tinyclaw --features slack -- --ignored

#[cfg(feature = "slack")]
mod slack_tests {
    use terraphim_tinyclaw::bus::MessageBus;
    use terraphim_tinyclaw::channel::Channel;
    use terraphim_tinyclaw::channels::slack::SlackChannel;
    use terraphim_tinyclaw::config::SlackConfig;
    use std::sync::Arc;

    fn slack_config_from_env() -> Option<SlackConfig> {
        let bot_token = std::env::var("SLACK_BOT_TOKEN").ok()?;
        let app_token = std::env::var("SLACK_APP_TOKEN").ok()?;
        Some(SlackConfig {
            bot_token,
            app_token,
            allow_from: vec!["*".to_string()],
        })
    }

    #[tokio::test]
    #[ignore]
    async fn test_slack_channel_lifecycle() {
        let config = slack_config_from_env()
            .expect("Set SLACK_BOT_TOKEN and SLACK_APP_TOKEN");
        let channel = SlackChannel::new(config);
        let bus = Arc::new(MessageBus::new());

        assert!(!channel.is_running());
        channel.start(bus).await.unwrap();
        assert!(channel.is_running());
        channel.stop().await.unwrap();
    }
}
```

## Rollback Plan

Feature-gated -- if issues discovered:
1. Remove `slack` from default features (already not in default)
2. Users simply don't enable `--features slack`
3. No data migration, no state changes, no external side effects

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| `slack-morphism` | 2.18 | Socket Mode + events + chat API. Only mature Rust Slack lib. |

Note: `slack-morphism` brings transitive deps (hyper, hyper-rustls, tokio-tungstenite).
These are already present in the workspace via other crates. Feature-gated so only
included when `--features slack` is used.

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Socket Mode connect | < 3s | Manual timing |
| Message processing | < 100ms | Log timestamps |
| User name cache hit | < 1us | HashMap lookup |
| Event dedup check | < 1us | HashSet lookup |

### Memory

- User name cache: ~100 bytes per user, bounded by workspace size (typically < 100 users)
- Event dedup set: ~50 bytes per event ID, grows over session lifetime. For a long-running
  bot processing 1000 messages/day, this is ~50KB/day. Acceptable for MVP. Phase 2 can
  add LRU eviction if needed.

## twin-slack (Private Repo) -- Companion Work

This section documents what needs to happen in `zestic-ai/digital-twins` (private).
It is NOT part of the terraphim-ai PR.

### twin-slack Crate (Phase 1: HTTP only)

**New files in `zestic-ai/digital-twins`:**

| File | Purpose |
|------|---------|
| `crates/twin-slack/Cargo.toml` | Crate manifest |
| `crates/twin-slack/src/lib.rs` | Router + state |
| `crates/twin-slack/src/auth.rs` | `auth.test` endpoint |
| `crates/twin-slack/src/chat.rs` | `chat.postMessage` endpoint |
| `crates/twin-slack/src/users.rs` | `users.info` endpoint |
| `specs/slack/API_SPECIFICATION.md` | Slack API subset spec |

**Endpoints:**
- `POST /auth.test` -- returns `{ ok: true, user_id: "U_BOT", team_id: "T_TEST" }`
- `POST /chat.postMessage` -- stores message, returns `{ ok: true, ts: "..." }`
- `POST /users.info?user=U123` -- returns configurable user profile

**twin-server mount:**
```rust
#[cfg(feature = "slack")]
{
    app = app.nest("/slack/api", twin_slack::slack_router(app_state.slack.clone()));
}
```

### Testing Strategy in Private CI

1. Private CI workflow in `zestic-ai/digital-twins` spawns `twin-server` with `--features slack`
2. Adds `terraphim_tinyclaw` as a git dependency (from terraphim-ai main branch)
3. Runs `cargo test -p terraphim_tinyclaw --features slack -- --ignored`
   with `SLACK_BOT_TOKEN=xoxb-test SLACK_APP_TOKEN=xapp-test` pointing to twin-slack
4. This validates the integration without real Slack tokens

Note: Phase 1 twin-slack has HTTP endpoints only. The Socket Mode WebSocket simulation
is Phase 2 work -- for MVP, the integration tests verify API calls (auth.test, chat.postMessage)
but not the full Socket Mode event flow. The event handling logic is covered by unit tests.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify slack-morphism 2.18 compiles with workspace tokio version | Pending | Implementation |
| Create Slack App and provision tokens | Pending | Alex |
| Create GitHub issue for Slack adapter (#terraphim-ai) | Pending | Implementation |
| Create twin-slack work item (digital-twins repo) | Pending | Separate |

## Approval

- [ ] File changes reviewed
- [ ] Public APIs reviewed
- [ ] Test strategy approved (unit + integration + twin)
- [ ] twin-slack boundary approved (no private leakage)
- [ ] Feature flag approach approved (opt-in, not default)
- [ ] Human approval received
