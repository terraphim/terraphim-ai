# Research Document: Leveraging OpenClaw and NanoClaw Slack Tests for TinyClaw

**Status**: Draft
**Author**: Terraphim AI
**Date**: 2026-03-09
**Reviewers**: AlexMikhalev
**Related Issues**: feat/tinyclaw-slack-adapter branch

---

## Executive Summary

OpenClaw has **39 Slack-specific test files** with comprehensive coverage across formatting, allowlist logic, message handling, threading, streaming, media, config validation, and more. NanoClaw (external, `github.com/qwibitai/nanoclaw`) has no locally available test files -- it is an external TypeScript project not cloned in the workspace.

Of the 39 OpenClaw test files, **8 contain test patterns directly adaptable** to TinyClaw's Slack adapter. The rest cover OpenClaw-specific features (Block Kit, threading, streaming, modals, slash commands, interactions) that are out of scope for TinyClaw's MVP.

**Critical finding**: OpenClaw tests use `vi.mock()` extensively -- every test file that touches Slack API calls uses mocks. TinyClaw's "never use mocks in tests" policy means we cannot port these tests verbatim. We must translate the **test logic and assertions** into TinyClaw's unit-test-with-real-helpers pattern.

**Recommendation**: Adapt 8 test categories from OpenClaw, translating mock-based tests into pure function tests. The existing TinyClaw implementation already has 10 unit tests covering the core helper functions. OpenClaw's tests reveal 12 additional test cases that should be added.

---

## Problem Statement

### Description

TinyClaw's Slack adapter (`crates/terraphim_tinyclaw/src/channels/slack.rs`) on the `feat/tinyclaw-slack-adapter` branch has 10 unit tests. We need to evaluate OpenClaw and NanoClaw Slack tests to identify additional test coverage opportunities.

### Success Criteria

1. Complete catalog of all Slack tests in OpenClaw and NanoClaw
2. Each test mapped to TinyClaw applicability (adaptable / needs modification / irrelevant)
3. Testing patterns and infrastructure identified for reuse
4. Concrete list of new tests to add to TinyClaw
5. No mock-based tests proposed (per project policy)

---

## Source Projects

### OpenClaw

- **Location**: `/Users/alex/projects/terraphim/openclaw/`
- **Language**: TypeScript
- **Test framework**: vitest
- **Slack library**: `@slack/bolt` + `@slack/web-api`
- **Test approach**: Heavy use of `vi.mock()` for `@slack/bolt`, `@slack/web-api`, and internal modules
- **Slack test files**: 39 files across `src/slack/`, `src/config/`, `src/channels/plugins/outbound/`, `src/agents/tools/`, `src/commands/`

### NanoClaw

- **Location**: External (`github.com/qwibitai/nanoclaw`) -- NOT cloned locally
- **Language**: TypeScript
- **Test framework**: vitest (per SPEC.md)
- **Slack library**: `@slack/bolt`
- **Test approach**: Uses `vi.mock()` for `@slack/bolt` (documented in prior research)
- **Status**: No local test files available for analysis. Prior research (in `tinyclaw-slack-research-2026-03-09.md`) documented NanoClaw's Slack patterns but did not catalog specific test files.

### rusty-claw

- **Location**: `/Users/alex/projects/terraphim/rusty-claw/`
- **Language**: Rust
- **Slack content**: None. No Slack-related code or tests found.

---

## Complete OpenClaw Slack Test Catalog

### Test Files by Category

#### 1. Formatting (DIRECTLY ADAPTABLE)

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/slack/format.test.ts` | 3 tests | `markdownToSlackMrkdwn`: bold, italic, strikethrough, code, links, lists, headings, blockquotes, nested formatting, unsafe char escaping, Slack mention preservation | None (pure function) | **YES** |
| `src/slack/blocks-fallback.test.ts` | 3 tests | Fallback text derivation from Block Kit blocks (header, image, file, unknown) | None (pure function) | No (Block Kit not in MVP) |

**OpenClaw format.test.ts test cases vs TinyClaw**:

| OpenClaw Test Case | TinyClaw Has? | Action |
|-------------------|---------------|--------|
| Bold `**text**` -> `*text*` | YES (`test_markdown_to_slack_mrkdwn_bold`) | Already covered |
| Italic `_text_` preserved | NO | **ADD** |
| Strikethrough `~~text~~` -> `~text~` | YES (`test_markdown_to_slack_mrkdwn_strikethrough`) | Already covered |
| Mixed inline formatting | NO | **ADD** |
| Inline code preserved | NO | **ADD** |
| Fenced code blocks (strip lang) | NO | **ADD** |
| Links `[text](url)` -> `<url\|text>` | YES (`test_markdown_to_slack_mrkdwn_link`) | Already covered |
| Bare URLs not duplicated | NO | **ADD** |
| Unsafe chars `& < >` escaped | NO | **ADD** |
| Slack mention `<@U123>` preserved | NO | **ADD** |
| Raw HTML escaped | NO | **ADD** |
| Bullet lists `- item` -> `bullet item` | NO | Consider for Phase 2 |
| Ordered lists preserved | NO | Consider for Phase 2 |
| Headings as bold | NO | Consider for Phase 2 |
| Blockquotes preserved | NO | Consider for Phase 2 |
| Nested lists | NO | Consider for Phase 2 |
| Complex multi-element messages | NO | **ADD** (simplified version) |

#### 2. Allowlist / Access Control (DIRECTLY ADAPTABLE)

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/slack/monitor/allow-list.test.ts` | 3 tests | `normalizeAllowList`, `normalizeSlackSlug`, `resolveSlackAllowListMatch`, `resolveSlackUserAllowed`: wildcard, ID matching, name matching, empty list behavior | None (pure functions) | **YES** |
| `src/slack/accounts.test.ts` | 4 tests | `resolveSlackAccount`: allowFrom precedence (account vs top-level vs dm), named account fallback | None (config resolution logic) | Partial (TinyClaw has simpler config) |

**OpenClaw allowlist test cases vs TinyClaw**:

| OpenClaw Test Case | TinyClaw Has? | Action |
|-------------------|---------------|--------|
| Wildcard `*` matches all | YES (via `is_sender_allowed` in channel.rs) | Already covered |
| ID-based matching | YES (via `is_sender_allowed`) | Already covered |
| Empty allowlist = deny all | Different (TinyClaw rejects empty in validation) | **ADD** (validate rejects empty) |
| Name-based matching with prefix | No (TinyClaw uses ID only) | Not applicable |
| Case normalization | NO | **ADD** (test case-insensitive matching if supported) |

#### 3. Message Handling & Dedup (PARTIALLY ADAPTABLE)

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/slack/monitor/message-handler.test.ts` | 3 tests | Dedup via `markMessageSeen`, non-message event filtering, accepted message tracking | Mocks for debounce, thread resolution | **PARTIAL** (dedup logic adaptable) |
| `src/slack/monitor/events/messages.test.ts` | 5 tests | `message_changed`, `message_deleted`, `thread_broadcast` event handling with policy enforcement (open/disabled/allowlist), channel user allowlist | Mocks for system events, pairing store | **PARTIAL** (policy enforcement patterns) |

**Adaptable test logic**:

| OpenClaw Test Case | TinyClaw Equivalent | Action |
|-------------------|---------------------|--------|
| Duplicate message detection | YES (`test_is_duplicate_event`) | Already covered |
| Non-message event type filtering | Implicit in `push_events_handler` match arm | **ADD** (test that non-message/non-mention events are ignored) |
| Bot message filtering | YES (`test_is_own_message_*`) | Already covered |
| Allowlist enforcement per message | Via `is_sender_allowed` check | Already covered (via `is_allowed` tests) |

#### 4. Config Validation (DIRECTLY ADAPTABLE)

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/config/slack-token-validation.test.ts` | 4 tests | Token config fields (botToken, appToken, userToken, userTokenReadOnly types) | None (config schema validation) | **YES** |
| `src/config/slack-http-config.test.ts` | 4 tests | HTTP mode + signingSecret validation | None (config validation) | No (TinyClaw uses Socket Mode only) |

**Adaptable test cases**:

| OpenClaw Test Case | TinyClaw Has? | Action |
|-------------------|---------------|--------|
| Valid config passes validation | YES (via `SlackConfig::validate()` tests on adapter branch) | Already covered |
| Empty bot_token rejected | YES | Already covered |
| Empty app_token rejected | YES | Already covered |
| Empty allow_from rejected | YES | Already covered |
| Invalid token types rejected | NO | **ADD** (test non-string token handling in deserialization) |

#### 5. Threading (NOT APPLICABLE for MVP)

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/slack/threading.test.ts` | 8 tests | Thread targets resolution, thread context, reply modes | Pure functions | Not in MVP scope |
| `src/slack/threading-tool-context.test.ts` | Tests | Tool context in threads | Pure functions | Not in MVP scope |
| `src/slack/sent-thread-cache.test.ts` | 7 tests | Thread participation cache with TTL, eviction | Pure functions | **Pattern reusable** for event dedup TTL (Phase 2) |
| `src/slack/monitor/monitor.threading.missing-thread-ts.test.ts` | Tests | Thread TS recovery | Mocks | Not applicable |

**Reusable pattern**: The `sent-thread-cache.test.ts` TTL and eviction tests demonstrate a bounded cache pattern that could inform Phase 2 event dedup improvements (currently TinyClaw uses unbounded `HashSet`).

#### 6. Streaming / Draft (NOT APPLICABLE)

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/slack/stream-mode.test.ts` | 4 groups | Stream mode resolution, streaming config, append-only updates, status dots | Pure functions | Not in MVP |
| `src/slack/draft-stream.test.ts` | 6 tests | Draft stream lifecycle (send, edit, dedup, force new, max chars, clear) | Mocks for send/edit/remove | Not in MVP |
| `src/slack/monitor/message-handler/dispatch.streaming.test.ts` | 2 groups | Native streaming enablement, thread hint resolution | Pure functions | Not in MVP |

#### 7. Send / Outbound (PARTIALLY ADAPTABLE)

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/slack/send.blocks.test.ts` | 10 tests | NO_REPLY suppression, block posting, fallback text derivation, validation (empty blocks, max count, missing type), mediaUrl+blocks rejection | Mocks for WebClient | **PARTIAL** (NO_REPLY pattern) |
| `src/slack/send.upload.test.ts` | 4 tests | File upload with user ID resolution, DM channel opening | Mocks for WebClient | Not applicable (no file upload in MVP) |
| `src/channels/plugins/outbound/slack.test.ts` | 7 tests | Hook wiring (message_sending hook, cancel, modify, identity forwarding) | Mocks for sendMessageSlack, hookRunner | Not applicable |
| `src/slack/monitor/replies.test.ts` | 3 tests | Identity passthrough for text/media replies | Mocks for send | Not applicable |

**Adaptable pattern**: The outgoing queue retry-on-failure pattern is already implemented in TinyClaw. OpenClaw's NO_REPLY suppression is worth noting but not needed for MVP.

#### 8. Monitor Context & Policy (PARTIALLY ADAPTABLE)

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/slack/monitor/context.test.ts` | 2 tests | Mismatched app/team event dropping | Pure function on context | **PARTIAL** (event validation pattern) |
| `src/slack/monitor/monitor.test.ts` | 10+ tests | Channel config resolution (requireMention, wildcard, direct match, case normalization), channel type normalization (D-prefix = DM), group policy enforcement, thread starter cache with TTL | Mix of pure and mocked | **PARTIAL** |
| `src/slack/monitor/provider.group-policy.test.ts` | 3 tests | Group policy resolution (fail-closed when config missing) | Pure function | Not applicable (TinyClaw has simple allowlist) |
| `src/slack/monitor/provider.reconnect.test.ts` | 2 tests | Socket disconnect/error waiter using FakeEmitter | FakeEmitter (no real mocks) | **Pattern reusable** |
| `src/slack/monitor/events/channels.test.ts` | 2 tests | Channel creation event tracking, mismatch guard | Mocks for system events | Not applicable |
| `src/slack/monitor/events/reactions.test.ts` | 7 tests | Reaction event handling with DM policy, allowlist, channel users | Mocks for system events | Not applicable |
| `src/slack/monitor/events/interactions.test.ts` | 15+ tests | Block action handling, modal submissions, access control | Mocks | Not applicable |
| `src/slack/monitor/events/members.test.ts` | Tests | Member join/leave events | Mocks | Not applicable |
| `src/slack/monitor/events/pins.test.ts` | Tests | Pin/unpin events | Mocks | Not applicable |

**Adaptable patterns from monitor.test.ts**:

| OpenClaw Test Case | TinyClaw Equivalent | Action |
|-------------------|---------------------|--------|
| D-prefix channel = DM inference | Not needed (TinyClaw routes by channel field) | Skip |
| Case-insensitive channel ID matching | Not in TinyClaw | Skip (Phase 2 if needed) |
| Event mismatch guard (wrong app/team) | Not in TinyClaw | **Consider** for robustness |

#### 9. Targets / Channel Resolution (PARTIALLY ADAPTABLE)

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/slack/targets.test.ts` | 3 groups | User mention parsing, channel target parsing, invalid target rejection | Pure functions | Not applicable (TinyClaw uses simple IDs) |
| `src/slack/resolve-channels.test.ts` | 2 tests | Channel resolution by name with active/archived preference | Mocked WebClient | Not applicable |
| `src/slack/channel-migration.test.ts` | Tests | Channel config migration | Pure functions | Not applicable |
| `src/slack/http/registry.test.ts` | Tests | HTTP request routing | Mocks | Not applicable |

#### 10. Message Preparation (NOT APPLICABLE)

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/slack/monitor/message-handler/prepare.test.ts` | 20+ tests | Full message preparation (inbound context, forwarded attachments, file-only messages, bot message handling, thread sessions, DM classification, sender prefix) | Real filesystem + mocks | Not applicable (OpenClaw-specific message model) |
| `src/slack/monitor/media.test.ts` | 20+ tests | Slack file download (auth redirect, SSRF protection, MIME handling, max files cap) | Mocked fetch | Not applicable |

#### 11. Other

| File | Tests | What It Tests | Dependencies | Adaptable? |
|------|-------|---------------|-------------|------------|
| `src/slack/client.test.ts` | 3 tests | WebClient config, retry defaults, merged options | Mocked `@slack/web-api` | Not applicable |
| `src/slack/actions.read.test.ts` | Tests | Action reading | Mocks | Not applicable |
| `src/slack/actions.blocks.test.ts` | Tests | Action block building | Pure functions | Not applicable |
| `src/slack/blocks-input.test.ts` | Tests | Block input validation | Pure functions | Not applicable |
| `src/slack/modal-metadata.test.ts` | Tests | Modal metadata serialization | Pure functions | Not applicable |
| `src/slack/monitor/slash.test.ts` | 20+ tests | Slash command registration, arg menus, policy enforcement | Heavy mocks | Not applicable |
| `src/agents/tools/slack-actions.test.ts` | Tests | Slack agent tools | Mocks | Not applicable |
| `src/commands/doctor.migrates-slack-discord-dm-policy-aliases.test.ts` | Tests | Config migration CLI | Mocks | Not applicable |

---

## Test Infrastructure Analysis

### OpenClaw Test Helpers

| Helper File | Location | Purpose | Reusable? |
|-------------|----------|---------|-----------|
| `src/slack/monitor.test-helpers.ts` | OpenClaw | Full Slack monitor test harness: mocked `@slack/bolt` App, client, handlers, config, send/reply/react mocks | No (mock-heavy, TS-specific) |
| `src/slack/blocks.test-helpers.ts` | OpenClaw | Block Kit test client factories, config mocks | No (Block Kit not in MVP) |
| `src/slack/monitor/events/system-event-test-harness.ts` | OpenClaw | System event test harness with configurable policy overrides | No (OpenClaw-specific) |
| `test/helpers/inbound-contract.ts` | OpenClaw | Inbound message contract validation | No (OpenClaw message model) |

### OpenClaw Testing Patterns

1. **Mock-first architecture**: Every test file that touches `@slack/bolt` or `@slack/web-api` uses `vi.mock()`. This is fundamentally incompatible with TinyClaw's "never use mocks" policy.

2. **Pure function isolation**: Several test files test pure functions WITHOUT mocks:
   - `format.test.ts` -- markdown formatting (no mocks)
   - `allow-list.test.ts` -- allowlist matching (no mocks)
   - `threading.test.ts` -- thread resolution (no mocks)
   - `stream-mode.test.ts` -- stream mode resolution (no mocks)
   - `targets.test.ts` -- target parsing (no mocks)
   - `sent-thread-cache.test.ts` -- cache logic (no mocks, uses `vi.spyOn(Date)` for time)
   - `blocks-fallback.test.ts` -- fallback text (no mocks)

3. **Test harness pattern**: Complex tests use a harness factory (`createSlackSystemEventTestHarness`, `createContext`) that pre-configures context objects. TinyClaw can adopt this pattern with Rust builder structs.

4. **Event-driven testing**: OpenClaw tests simulate Slack events by calling registered handler functions directly. TinyClaw's approach of testing helper functions in isolation is cleaner.

### NanoClaw Testing Patterns (from prior research)

Per the existing research document (`tinyclaw-slack-research-2026-03-09.md`):
- NanoClaw uses vitest with full mocks of `@slack/bolt`
- Test patterns focus on: bot self-detection, @mention stripping, user name cache, channel JID ownership
- These patterns were already incorporated into TinyClaw's implementation and existing tests

---

## Mapping to TinyClaw Implementation

### Current TinyClaw Slack Tests (on `feat/tinyclaw-slack-adapter` branch)

Located in `crates/terraphim_tinyclaw/src/channels/slack.rs` (`#[cfg(test)] mod tests`):

| Test | What It Covers |
|------|---------------|
| `test_slack_channel_name` | Channel returns "slack" |
| `test_strip_bot_mention` | `<@UBOT123> hello` -> `hello` |
| `test_strip_bot_mention_no_match` | Text without mention unchanged |
| `test_strip_bot_mention_multiple` | Multiple mentions stripped |
| `test_is_own_message_by_user_id` | Own message detected by user ID match |
| `test_is_own_message_by_bot_id` | Own message detected by bot_id presence |
| `test_is_own_message_other_user` | Other user not detected as own |
| `test_is_duplicate_event` | First pass, second blocked, new event passes |
| `test_outgoing_queue_on_disconnect` | Queue push, drain, empty after drain |
| `test_is_from_me_metadata` | is_from_me metadata set on InboundMessage |

Located in `crates/terraphim_tinyclaw/src/format.rs` (test module):

| Test | What It Covers |
|------|---------------|
| `test_markdown_to_slack_mrkdwn_bold` | `**bold**` -> `*bold*` |
| `test_markdown_to_slack_mrkdwn_strikethrough` | `~~strike~~` -> `~strike~` |
| `test_markdown_to_slack_mrkdwn_link` | `[text](url)` -> `<url\|text>` |

### Recommended New Tests from OpenClaw Analysis

#### Priority 1: Format tests (from `format.test.ts`)

```rust
// In format.rs tests

#[test]
fn test_markdown_to_slack_mrkdwn_italic_preserved() {
    assert_eq!(markdown_to_slack_mrkdwn("_italic_"), "_italic_");
}

#[test]
fn test_markdown_to_slack_mrkdwn_inline_code_preserved() {
    assert_eq!(
        markdown_to_slack_mrkdwn("use `cargo build`"),
        "use `cargo build`"
    );
}

#[test]
fn test_markdown_to_slack_mrkdwn_mixed_formatting() {
    let input = "hi _there_ **boss** `code`";
    let result = markdown_to_slack_mrkdwn(input);
    assert_eq!(result, "hi _there_ *boss* `code`");
}

#[test]
fn test_markdown_to_slack_mrkdwn_slack_mentions_preserved() {
    // Slack angle-bracket markup must NOT be escaped
    let input = "hi <@U123> see <https://example.com|docs>";
    let result = markdown_to_slack_mrkdwn(input);
    assert!(result.contains("<@U123>"));
    assert!(result.contains("<https://example.com|docs>"));
}

#[test]
fn test_markdown_to_slack_mrkdwn_bare_url_not_duplicated() {
    let input = "see https://example.com";
    let result = markdown_to_slack_mrkdwn(input);
    // Should NOT wrap bare URLs in <url|url> -- just pass through
    assert_eq!(result, "see https://example.com");
}

#[test]
fn test_markdown_to_slack_mrkdwn_complex_message() {
    let input = "**Important:** Check the _docs_ at [link](https://example.com)";
    let result = markdown_to_slack_mrkdwn(input);
    assert!(result.contains("*Important:*"));
    assert!(result.contains("_docs_"));
    assert!(result.contains("<https://example.com|link>"));
}

#[test]
fn test_chunk_message_slack_4000() {
    let long_text = "a".repeat(5000);
    let chunks = chunk_message(&long_text, 4000);
    assert!(chunks.len() >= 2);
    for chunk in &chunks {
        assert!(chunk.len() <= 4000);
    }
}
```

#### Priority 2: Allowlist edge cases (from `allow-list.test.ts`)

```rust
// In slack.rs tests or config.rs tests

#[test]
fn test_slack_config_validate_rejects_whitespace_only_tokens() {
    let config = SlackConfig {
        bot_token: "   ".to_string(),
        app_token: "xapp-test".to_string(),
        allow_from: vec!["U111".to_string()],
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_is_allowed_case_sensitivity() {
    // Slack user IDs are case-sensitive (always uppercase)
    let config = SlackConfig {
        bot_token: "xoxb-test".to_string(),
        app_token: "xapp-test".to_string(),
        allow_from: vec!["U12345".to_string()],
    };
    assert!(config.is_allowed("U12345"));
    // Slack IDs are always uppercase, but test defensive behavior
    assert!(!config.is_allowed("u12345"));
}
```

#### Priority 3: Event handler edge cases (from `message-handler.test.ts`, `monitor.test.ts`)

```rust
// In slack.rs tests

#[test]
fn test_strip_bot_mention_with_surrounding_whitespace() {
    // OpenClaw: strip mention + trim surrounding whitespace
    let result = strip_bot_mention("  <@UBOT123>  hello  ", "UBOT123");
    assert_eq!(result, "hello");
}

#[test]
fn test_strip_bot_mention_mention_only() {
    // When the entire message is just a mention
    let result = strip_bot_mention("<@UBOT123>", "UBOT123");
    assert_eq!(result, "");
}

#[test]
fn test_is_own_message_empty_bot_id() {
    // Empty string bot_id should not trigger is_from_me
    assert!(!is_own_message("UOTHER", Some(""), "UBOT123"));
}
```

#### Priority 4: Outgoing queue resilience (from NanoClaw patterns)

```rust
// In slack.rs tests

#[tokio::test]
async fn test_outgoing_queue_multiple_messages() {
    let config = SlackConfig {
        bot_token: "xoxb-test".to_string(),
        app_token: "xapp-test".to_string(),
        allow_from: vec!["U111".to_string()],
    };
    let channel = SlackChannel::new(config);

    // Queue multiple messages
    {
        let mut queue = channel.outgoing_queue.lock().await;
        queue.push(QueuedMessage {
            chat_id: "C1".to_string(),
            content: "first".to_string(),
        });
        queue.push(QueuedMessage {
            chat_id: "C2".to_string(),
            content: "second".to_string(),
        });
    }

    let drained = channel.drain_queued_messages().await;
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].content, "first");
    assert_eq!(drained[1].content, "second");
}

#[tokio::test]
async fn test_outgoing_queue_preserves_channel_field() {
    let config = SlackConfig {
        bot_token: "xoxb-test".to_string(),
        app_token: "xapp-test".to_string(),
        allow_from: vec!["U111".to_string()],
    };
    let channel = SlackChannel::new(config);

    channel.outgoing_queue.lock().await.push(QueuedMessage {
        chat_id: "C_TARGET".to_string(),
        content: "test content".to_string(),
    });

    let drained = channel.drain_queued_messages().await;
    assert_eq!(drained[0].channel, "slack");
    assert_eq!(drained[0].chat_id, "C_TARGET");
}
```

---

## Constraints

### Technical Constraints

| Constraint | Description | Impact |
|------------|-------------|--------|
| No mocks | TinyClaw project policy: "never use mocks in tests" | Cannot port OpenClaw mock-based tests directly |
| Feature-gated compilation | Slack code requires `--features slack` | Tests must compile without feature or be behind `#[cfg(feature = "slack")]` |
| Pure function testing only | No Slack API calls in unit tests | Helper functions must be extractable and testable in isolation |
| slack-morphism types in tests | `SlackApiToken`, `SlackChannelId` etc. require the crate | Tests using these types need the `slack` feature |

### Testing Architecture Difference

| Aspect | OpenClaw | TinyClaw |
|--------|----------|----------|
| Framework | vitest (TypeScript) | `#[test]` / `#[tokio::test]` (Rust) |
| Mocking | `vi.mock()` for modules, `vi.fn()` for functions | Not allowed |
| API simulation | Mocked WebClient, mocked `@slack/bolt` App | Digital twin (Phase 2) or env-var-gated live tests |
| State isolation | `beforeEach`/`afterEach` with mock resets | Fresh struct construction per test |
| Async testing | Native promises | `#[tokio::test]` |

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| OpenClaw format edge cases not covered | Medium | Low | Add the 7 recommended format tests |
| is_own_message with empty bot_id string | Medium | Medium | Add edge case test; fix if behavior incorrect |
| Event dedup unbounded memory growth | Low (long sessions) | Low | Log note for Phase 2; OpenClaw's TTL cache pattern available |
| Slack mention preservation in mrkdwn conversion | Medium | Medium | Add test; may need to fix conversion function |

### Open Questions

1. **Should `is_own_message` treat `Some("")` as equivalent to `None`?** -- OpenClaw checks `event.bot_id` presence, but an empty string bot_id is technically present. Current TinyClaw code treats `Some("")` as a bot message (returns `true`). This may be incorrect.

2. **Does `markdown_to_slack_mrkdwn` preserve Slack angle-bracket markup?** -- OpenClaw explicitly preserves `<@U123>`, `<https://...|text>`, and `<!here>`. TinyClaw's current implementation may incorrectly transform these. Needs test and potential fix.

3. **Should bare URLs be left unchanged?** -- OpenClaw preserves bare URLs as-is (does not wrap in `<url|url>`). TinyClaw's link regex may incorrectly match bare URLs.

---

## Recommendations

### Tests to Add (12 new tests)

**Immediate (add to `feat/tinyclaw-slack-adapter` branch)**:

1. `test_markdown_to_slack_mrkdwn_italic_preserved` -- from OpenClaw format.test.ts
2. `test_markdown_to_slack_mrkdwn_inline_code_preserved` -- from OpenClaw format.test.ts
3. `test_markdown_to_slack_mrkdwn_mixed_formatting` -- from OpenClaw format.test.ts
4. `test_markdown_to_slack_mrkdwn_slack_mentions_preserved` -- from OpenClaw format.test.ts
5. `test_markdown_to_slack_mrkdwn_bare_url_not_duplicated` -- from OpenClaw format.test.ts
6. `test_markdown_to_slack_mrkdwn_complex_message` -- from OpenClaw format.test.ts
7. `test_chunk_message_slack_4000` -- from OpenClaw 4000 char limit
8. `test_slack_config_validate_rejects_whitespace_only_tokens` -- from OpenClaw token validation
9. `test_strip_bot_mention_with_surrounding_whitespace` -- from OpenClaw monitor tests
10. `test_strip_bot_mention_mention_only` -- edge case from OpenClaw patterns
11. `test_is_own_message_empty_bot_id` -- edge case from OpenClaw patterns
12. `test_outgoing_queue_multiple_messages` -- from NanoClaw queue pattern

**Phase 2 (future work)**:

- Event dedup TTL (bounded `HashSet` or LRU) -- informed by OpenClaw `sent-thread-cache.test.ts`
- Socket reconnect testing -- informed by OpenClaw `provider.reconnect.test.ts` FakeEmitter pattern
- List formatting (bullets, ordered lists) -- from OpenClaw `format.test.ts`
- Heading conversion -- from OpenClaw `format.test.ts`

### Patterns to NOT Adopt

1. **vi.mock() equivalent in Rust** -- No trait objects or test doubles for Slack API. Test helper functions in isolation instead.
2. **OpenClaw's system event model** -- TinyClaw uses `InboundMessage` on a `MessageBus`, not system events.
3. **Block Kit tests** -- Not in MVP scope.
4. **Threading tests** -- Not in MVP scope.
5. **Streaming tests** -- Not in MVP scope.
6. **Modal/interaction tests** -- Not in MVP scope.
7. **Slash command tests** -- TinyClaw handles slash commands via message text, not Slack's slash command API.

### Test Infrastructure Recommendations

1. **No new test helpers needed** -- TinyClaw's existing test pattern (construct structs, call functions, assert results) is sufficient for all recommended tests.

2. **Integration tests remain env-var-gated** -- The existing `tests/slack_integration.rs` scaffold with `#[ignore]` is correct. Digital twin (twin-slack) is the Phase 2 solution.

3. **Keep tests in the same file** -- Inline `#[cfg(test)] mod tests` in `slack.rs` and `format.rs` is the right location for these unit tests.

---

## Summary Table

| Category | OpenClaw Tests | Directly Adaptable | Partially Adaptable | Not Applicable |
|----------|---------------|-------------------|--------------------|-|
| Formatting | 3 | **3** | 0 | 0 |
| Allowlist | 7 | **2** | 2 | 3 |
| Message handling | 8 | 0 | **3** | 5 |
| Config validation | 8 | **2** | 0 | 6 |
| Threading | 15+ | 0 | 0 | **15+** |
| Streaming | 12 | 0 | 0 | **12** |
| Send/outbound | 24 | 0 | **1** | 23 |
| Monitor/context | 30+ | 0 | **3** | 27+ |
| Targets/channels | 7 | 0 | 0 | **7** |
| Message prep | 40+ | 0 | 0 | **40+** |
| Other | 20+ | 0 | 0 | **20+** |
| **Total** | **~175** | **7** | **9** | **~160** |

**Bottom line**: Of ~175 OpenClaw Slack tests, 7 are directly adaptable as pure-function tests, 9 have partially adaptable logic, and ~160 are not applicable (either OpenClaw-specific features or mock-dependent patterns that conflict with TinyClaw's testing policy). The 12 recommended new tests are derived from the adaptable subset and fill concrete coverage gaps.

---

## Next Steps

If approved:
1. Add the 12 recommended tests to `feat/tinyclaw-slack-adapter` branch
2. Verify and fix `markdown_to_slack_mrkdwn` for Slack mention preservation and bare URL handling
3. Address the `is_own_message` empty bot_id edge case
4. Document the OpenClaw TTL cache pattern for Phase 2 event dedup improvement
5. Close this research item

---

## Appendix

### OpenClaw Slack Test File Inventory (complete paths)

```
src/slack/format.test.ts
src/slack/client.test.ts
src/slack/accounts.test.ts
src/slack/actions.read.test.ts
src/slack/actions.blocks.test.ts
src/slack/blocks-fallback.test.ts
src/slack/blocks-input.test.ts
src/slack/channel-migration.test.ts
src/slack/draft-stream.test.ts
src/slack/modal-metadata.test.ts
src/slack/monitor.test.ts
src/slack/monitor.threading.missing-thread-ts.test.ts
src/slack/monitor.tool-result.test.ts
src/slack/resolve-channels.test.ts
src/slack/send.blocks.test.ts
src/slack/send.upload.test.ts
src/slack/sent-thread-cache.test.ts
src/slack/stream-mode.test.ts
src/slack/targets.test.ts
src/slack/threading.test.ts
src/slack/threading-tool-context.test.ts
src/slack/http/registry.test.ts
src/slack/monitor/allow-list.test.ts
src/slack/monitor/context.test.ts
src/slack/monitor/media.test.ts
src/slack/monitor/message-handler.test.ts
src/slack/monitor/message-handler/dispatch.streaming.test.ts
src/slack/monitor/message-handler/prepare.test.ts
src/slack/monitor/monitor.test.ts
src/slack/monitor/provider.group-policy.test.ts
src/slack/monitor/provider.reconnect.test.ts
src/slack/monitor/replies.test.ts
src/slack/monitor/slash.test.ts
src/slack/monitor/events/channels.test.ts
src/slack/monitor/events/interactions.test.ts
src/slack/monitor/events/members.test.ts
src/slack/monitor/events/messages.test.ts
src/slack/monitor/events/pins.test.ts
src/slack/monitor/events/reactions.test.ts
src/config/slack-http-config.test.ts
src/config/slack-token-validation.test.ts
src/channels/plugins/outbound/slack.test.ts
src/commands/doctor.migrates-slack-discord-dm-policy-aliases.test.ts
src/agents/tools/slack-actions.test.ts
```

### OpenClaw Test Helpers (complete paths)

```
src/slack/monitor.test-helpers.ts
src/slack/blocks.test-helpers.ts
src/slack/monitor/events/system-event-test-harness.ts
src/slack/monitor/slash.test-harness.ts
```

### TinyClaw Existing Slack Tests (on feat/tinyclaw-slack-adapter branch)

```
crates/terraphim_tinyclaw/src/channels/slack.rs  (10 tests in #[cfg(test)] mod tests)
crates/terraphim_tinyclaw/src/format.rs  (3 slack mrkdwn tests + other format tests)
crates/terraphim_tinyclaw/tests/slack_integration.rs  (2 #[ignore] integration test scaffolds)
```
