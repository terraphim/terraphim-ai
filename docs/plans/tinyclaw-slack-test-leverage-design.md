# Implementation Plan: TinyClaw Slack Test Leverage (12 Tests + 3 Bug Fixes)

**Status**: Pending Approval
**Author**: Terraphim AI
**Date**: 2026-03-09
**Research Document**: `docs/plans/tinyclaw-slack-test-leverage-research.md` (approved)
**Branch**: `feat/tinyclaw-slack-adapter`

---

## Overview

This plan adds 12 new unit tests to TinyClaw's Slack adapter, adapted from OpenClaw's pure-function test patterns. Three of these tests expose bugs in the existing code that will be fixed as part of the same step. All tests are pure-function unit tests -- no network, no mocks, no external dependencies.

### Files Changed

| File | Action | Tests Added | Bug Fixes |
|------|--------|-------------|-----------|
| `crates/terraphim_tinyclaw/src/format.rs` | Add tests, fix formatting bugs | 6 | 2 (Slack mention preservation, bare URL handling) |
| `crates/terraphim_tinyclaw/src/config.rs` | Add tests, fix validation | 2 | 1 (whitespace-only token accepted) |
| `crates/terraphim_tinyclaw/src/channels/slack.rs` | Add tests, fix edge case | 4 | 1 (empty bot_id treated as bot message) |

### No New Files Created

All tests go into existing `#[cfg(test)] mod tests` blocks within their respective source files. No new test infrastructure, helpers, or test harness files are needed.

---

## Step 1: Add 6 Format Tests to format.rs, Fix Formatting Bugs

**File**: `crates/terraphim_tinyclaw/src/format.rs`

### 1.1 Bug Analysis

**Bug A -- Slack mention preservation**: The current `markdown_to_slack_mrkdwn` function does not protect Slack angle-bracket markup (`<@U123>`, `<https://...|text>`, `<!here>`). The `replace_links_to_slack` function scans for `[` characters to find markdown links. Since Slack's own `<...|...>` syntax does not use `[`, it should pass through safely. However, `<@U123>` and `<!here>` could be affected if any future escaping step is added. The test documents the expected behavior and serves as a regression guard.

**Bug B -- Bare URL duplication**: The `replace_links_to_slack` function looks for the pattern `[text](url)` by scanning for `[`. A bare URL like `https://example.com` does not contain `[`, so it will NOT be matched by the link conversion regex. This means the current code already handles bare URLs correctly (pass-through). The test confirms this behavior. No code fix needed unless the test fails.

Both bugs are lower risk than initially estimated. The tests serve as regression guards for correct behavior.

### 1.2 Tests to Add

Add these 6 tests to the existing `#[cfg(test)] mod tests` block in `format.rs`, after the existing `test_markdown_to_slack_mrkdwn_code` test:

```rust
#[test]
fn test_markdown_to_slack_mrkdwn_italic_preserved() {
    // Slack mrkdwn uses _text_ for italic, same as markdown.
    // The function should pass italic through unchanged.
    assert_eq!(markdown_to_slack_mrkdwn("_italic_"), "_italic_");
}

#[test]
fn test_markdown_to_slack_mrkdwn_inline_code_preserved() {
    // Backtick code spans are identical in markdown and Slack mrkdwn.
    assert_eq!(
        markdown_to_slack_mrkdwn("use `cargo build`"),
        "use `cargo build`"
    );
}

#[test]
fn test_markdown_to_slack_mrkdwn_mixed_formatting() {
    // Italic, bold, and code combined in one message.
    let input = "hi _there_ **boss** `code`";
    let result = markdown_to_slack_mrkdwn(input);
    assert_eq!(result, "hi _there_ *boss* `code`");
}

#[test]
fn test_markdown_to_slack_mrkdwn_slack_mentions_preserved() {
    // Slack angle-bracket markup must NOT be escaped or transformed.
    // <@U123> is a user mention, <https://...|text> is a Slack-native link.
    let input = "hi <@U123> see <https://example.com|docs>";
    let result = markdown_to_slack_mrkdwn(input);
    assert!(
        result.contains("<@U123>"),
        "User mention was corrupted: {}",
        result
    );
    assert!(
        result.contains("<https://example.com|docs>"),
        "Slack link was corrupted: {}",
        result
    );
}

#[test]
fn test_markdown_to_slack_mrkdwn_bare_url_not_duplicated() {
    // Bare URLs should pass through unchanged.
    // The link converter only matches [text](url) patterns.
    let input = "see https://example.com for details";
    let result = markdown_to_slack_mrkdwn(input);
    assert_eq!(result, "see https://example.com for details");
}

#[test]
fn test_markdown_to_slack_mrkdwn_complex_message() {
    // Combined formatting: bold, italic, and a markdown link.
    let input = "**Important:** Check the _docs_ at [link](https://example.com)";
    let result = markdown_to_slack_mrkdwn(input);
    assert!(
        result.contains("*Important:*"),
        "Bold not converted: {}",
        result
    );
    assert!(result.contains("_docs_"), "Italic not preserved: {}", result);
    assert!(
        result.contains("<https://example.com|link>"),
        "Link not converted: {}",
        result
    );
}
```

### 1.3 Bug Fix: Slack Mention Preservation

After writing the tests, run them. If `test_markdown_to_slack_mrkdwn_slack_mentions_preserved` fails because the `<` and `>` characters in Slack mentions are being corrupted, the fix is to add a pre-processing step in `markdown_to_slack_mrkdwn` that extracts Slack angle-bracket tokens before processing, then restores them after.

**Expected fix location**: `markdown_to_slack_mrkdwn()` function body (lines 60-90 of `format.rs`).

**Fix strategy** (only apply if test fails):

```rust
pub fn markdown_to_slack_mrkdwn(text: &str) -> String {
    // Step 1: Extract Slack angle-bracket tokens and replace with placeholders.
    // Matches: <@U123>, <#C123>, <!here>, <!channel>, <https://...|text>
    let mut placeholders: Vec<String> = Vec::new();
    let mut result = text.to_string();

    // Use a simple scan for < ... > sequences that look like Slack markup
    let mut protected = String::new();
    let mut chars = result.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Collect until matching >
            let mut token = String::from('<');
            let mut found_close = false;
            for inner in chars.by_ref() {
                token.push(inner);
                if inner == '>' {
                    found_close = true;
                    break;
                }
            }
            if found_close
                && (token.starts_with("<@")
                    || token.starts_with("<#")
                    || token.starts_with("<!")
                    || token.starts_with("<http"))
            {
                let placeholder = format!("\x00SLACK{}\x00", placeholders.len());
                placeholders.push(token);
                protected.push_str(&placeholder);
            } else {
                protected.push_str(&token);
            }
        } else {
            protected.push(ch);
        }
    }
    result = protected;

    // Step 2: Normal markdown-to-mrkdwn conversions (bold, strikethrough, links)
    // ... existing conversion code ...

    // Step 3: Restore Slack tokens from placeholders
    for (i, original) in placeholders.iter().enumerate() {
        let placeholder = format!("\x00SLACK{}\x00", i);
        result = result.replace(&placeholder, original);
    }

    result
}
```

**Note**: This fix is only needed if the test fails. Based on code analysis, the current implementation likely passes because `replace_links_to_slack` only scans for `[` (markdown link start), and Slack tokens use `<` which is a different character. The bold/strikethrough converters scan for `**` and `~~` which also do not interfere with Slack tokens. The test is primarily a regression guard.

### 1.4 Verification

```bash
cargo test -p terraphim_tinyclaw --features slack -- test_markdown_to_slack_mrkdwn
```

All 9 Slack mrkdwn tests (3 existing + 6 new) must pass. If any new test fails, apply the corresponding fix before proceeding.

---

## Step 2: Add 2 Config Edge Case Tests to config.rs, Fix Whitespace Validation

**File**: `crates/terraphim_tinyclaw/src/config.rs`

### 2.1 Bug Analysis

**Bug C -- Whitespace-only tokens accepted**: The current `SlackConfig::validate()` (lines 324-338 of `config.rs`) checks `self.bot_token.is_empty()` and `self.app_token.is_empty()`. A token consisting of only whitespace (`"   "`) passes this check because `is_empty()` returns `false` for whitespace strings. This is a real bug -- a whitespace-only token will fail at the Slack API level with a confusing error instead of failing early at validation.

### 2.2 Tests to Add

Add these 2 tests to the existing `#[cfg(test)] mod tests` block in `config.rs`, after `test_slack_config_is_allowed_wildcard`:

```rust
#[test]
fn test_slack_config_validate_rejects_whitespace_only_tokens() {
    // Whitespace-only tokens should be rejected at validation time,
    // not at Slack API call time. This catches copy-paste errors.
    let cfg = SlackConfig {
        bot_token: "   ".to_string(),
        app_token: "xapp-test".to_string(),
        allow_from: vec!["U111".to_string()],
    };
    assert!(
        cfg.validate().is_err(),
        "Whitespace-only bot_token should be rejected"
    );

    let cfg2 = SlackConfig {
        bot_token: "xoxb-test".to_string(),
        app_token: "  \t  ".to_string(),
        allow_from: vec!["U111".to_string()],
    };
    assert!(
        cfg2.validate().is_err(),
        "Whitespace-only app_token should be rejected"
    );
}

#[test]
fn test_slack_config_is_allowed_case_sensitivity() {
    // Slack user IDs are always uppercase (e.g., U01234567).
    // The allowlist check must be case-sensitive because Slack IDs
    // are canonical uppercase strings.
    let cfg = SlackConfig {
        bot_token: "xoxb-test".to_string(),
        app_token: "xapp-test".to_string(),
        allow_from: vec!["U12345".to_string()],
    };
    assert!(cfg.is_allowed("U12345"), "Exact match should pass");
    assert!(
        !cfg.is_allowed("u12345"),
        "Lowercase variant should be rejected (Slack IDs are uppercase)"
    );
}
```

### 2.3 Bug Fix: Whitespace Validation

**Fix location**: `SlackConfig::validate()` method in `config.rs` (lines 324-338).

**Current code**:
```rust
pub fn validate(&self) -> anyhow::Result<()> {
    if self.bot_token.is_empty() {
        anyhow::bail!("slack.bot_token cannot be empty");
    }
    if self.app_token.is_empty() {
        anyhow::bail!("slack.app_token cannot be empty");
    }
    // ...
}
```

**Fixed code**:
```rust
pub fn validate(&self) -> anyhow::Result<()> {
    if self.bot_token.trim().is_empty() {
        anyhow::bail!("slack.bot_token cannot be empty");
    }
    if self.app_token.trim().is_empty() {
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
```

The change is `.is_empty()` to `.trim().is_empty()` for both `bot_token` and `app_token`. This ensures whitespace-only strings are rejected.

### 2.4 Verification

```bash
cargo test -p terraphim_tinyclaw --features slack -- test_slack_config
```

All 7 Slack config tests (5 existing + 2 new) must pass.

---

## Step 3: Add 3 Event Handler Edge Case Tests to slack.rs, Fix is_own_message

**File**: `crates/terraphim_tinyclaw/src/channels/slack.rs`

### 3.1 Bug Analysis

**Bug D -- is_own_message treats Some("") as bot message**: The current implementation (line 317):

```rust
fn is_own_message(event_user: &str, event_bot_id: Option<&str>, bot_user_id: &str) -> bool {
    event_user == bot_user_id || event_bot_id.is_some()
}
```

The `event_bot_id.is_some()` check returns `true` for `Some("")`. In Slack's event model, an empty-string `bot_id` can occur when a non-bot user posts via an app integration. Treating `Some("")` as a bot message causes legitimate user messages to be flagged as `is_from_me`, which means they get the `is_bot_message` metadata set incorrectly.

### 3.2 Tests to Add

Add these 3 tests to the existing `#[cfg(test)] mod tests` block in `slack.rs`, after `test_is_from_me_metadata`:

```rust
#[test]
fn test_strip_bot_mention_with_surrounding_whitespace() {
    // Mention with extra whitespace around it should be cleaned up.
    // The strip_bot_mention function replaces the mention pattern and
    // trims the result, so surrounding whitespace is removed.
    let result = strip_bot_mention("  <@UBOT123>  hello  ", "UBOT123");
    assert_eq!(result, "hello");
}

#[test]
fn test_strip_bot_mention_mention_only() {
    // When the entire message is just a bot mention with no other text,
    // the result should be an empty string.
    let result = strip_bot_mention("<@UBOT123>", "UBOT123");
    assert_eq!(result, "");
}

#[test]
fn test_is_own_message_empty_bot_id() {
    // An empty string bot_id (Some("")) should NOT trigger is_own_message.
    // Slack may set bot_id to "" for certain app integrations that are not
    // actually bot messages. Only non-empty bot_id values indicate a bot.
    assert!(
        !is_own_message("UOTHER", Some(""), "UBOT123"),
        "Empty bot_id should not be treated as own message"
    );
}
```

### 3.3 Bug Fix: is_own_message Empty bot_id

**Fix location**: `is_own_message()` function in `slack.rs` (line 317).

**Current code**:
```rust
fn is_own_message(event_user: &str, event_bot_id: Option<&str>, bot_user_id: &str) -> bool {
    event_user == bot_user_id || event_bot_id.is_some()
}
```

**Fixed code**:
```rust
fn is_own_message(event_user: &str, event_bot_id: Option<&str>, bot_user_id: &str) -> bool {
    event_user == bot_user_id || event_bot_id.is_some_and(|id| !id.is_empty())
}
```

The change is `event_bot_id.is_some()` to `event_bot_id.is_some_and(|id| !id.is_empty())`. This correctly handles the `Some("")` case by treating it as "no bot_id present".

**Impact on existing tests**: The existing `test_is_own_message_by_bot_id` test passes `Some("BBOT456")` which is non-empty, so it continues to pass. The existing `test_is_own_message_other_user` passes `None`, also unaffected.

### 3.4 Verification

```bash
cargo test -p terraphim_tinyclaw --features slack -- test_strip_bot_mention test_is_own_message
```

All 8 related tests (5 existing + 3 new) must pass.

---

## Step 4: Add 1 Queue Resilience Test to slack.rs

**File**: `crates/terraphim_tinyclaw/src/channels/slack.rs`

### 4.1 Test to Add

Add this test to the existing `#[cfg(test)] mod tests` block in `slack.rs`, after `test_is_own_message_empty_bot_id`:

```rust
#[tokio::test]
async fn test_outgoing_queue_multiple_messages() {
    // Verify that multiple queued messages are drained in FIFO order
    // and that each retains its original chat_id and content.
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
            chat_id: "C_FIRST".to_string(),
            content: "first message".to_string(),
        });
        queue.push(QueuedMessage {
            chat_id: "C_SECOND".to_string(),
            content: "second message".to_string(),
        });
        queue.push(QueuedMessage {
            chat_id: "C_THIRD".to_string(),
            content: "third message".to_string(),
        });
    }

    let drained = channel.drain_queued_messages().await;
    assert_eq!(drained.len(), 3, "All three messages should be drained");

    // Verify FIFO order
    assert_eq!(drained[0].chat_id, "C_FIRST");
    assert_eq!(drained[0].content, "first message");
    assert_eq!(drained[0].channel, "slack");

    assert_eq!(drained[1].chat_id, "C_SECOND");
    assert_eq!(drained[1].content, "second message");
    assert_eq!(drained[1].channel, "slack");

    assert_eq!(drained[2].chat_id, "C_THIRD");
    assert_eq!(drained[2].content, "third message");
    assert_eq!(drained[2].channel, "slack");

    // Queue must be empty after drain
    let drained_again = channel.drain_queued_messages().await;
    assert!(drained_again.is_empty(), "Queue should be empty after drain");
}
```

### 4.2 No Bug Fix Needed

The outgoing queue implementation is correct. This test documents the expected FIFO ordering behavior and verifies that the `channel` field is set to `"slack"` by `QueuedMessage::into_outbound`.

### 4.3 Verification

```bash
cargo test -p terraphim_tinyclaw --features slack -- test_outgoing_queue
```

Both queue tests (1 existing + 1 new) must pass.

---

## Step 5: Run Full Test Suite, Clippy, Commit

### 5.1 Run All TinyClaw Tests

```bash
cargo test -p terraphim_tinyclaw --features slack
```

Expected: all 22+ tests pass (10 existing in slack.rs + 4 new = 14; 11 existing in format.rs + 6 new = 17; existing config tests + 2 new; bus tests; channel tests).

### 5.2 Run Clippy

```bash
cargo clippy -p terraphim_tinyclaw --features slack -- -W clippy::all
```

No warnings or errors expected.

### 5.3 Run Format Check

```bash
cargo fmt -p terraphim_tinyclaw -- --check
```

### 5.4 Commit

Single commit covering all changes:

```
feat(tinyclaw): add 12 Slack tests from OpenClaw, fix 2 bugs

Add 12 new unit tests adapted from OpenClaw Slack test patterns:
- 6 format tests: italic, inline code, mixed formatting, Slack mention
  preservation, bare URL pass-through, complex message
- 2 config tests: whitespace-only token rejection, case sensitivity
- 3 event handler tests: mention with whitespace, mention-only, empty bot_id
- 1 queue resilience test: multiple message FIFO drain

Fix 2 bugs exposed by the new tests:
- SlackConfig::validate() now rejects whitespace-only tokens
- is_own_message() no longer treats Some("") bot_id as a bot message
```

---

## Test Strategy Summary

### What Is Tested

| Category | Count | Testing Approach | Bug Fix? |
|----------|-------|------------------|----------|
| Format: italic preserved | 1 | Pure function, assert_eq | No |
| Format: inline code preserved | 1 | Pure function, assert_eq | No |
| Format: mixed formatting | 1 | Pure function, assert_eq | No |
| Format: Slack mentions preserved | 1 | Pure function, assert contains | Regression guard |
| Format: bare URL not duplicated | 1 | Pure function, assert_eq | Regression guard |
| Format: complex message | 1 | Pure function, assert contains | No |
| Config: whitespace token rejected | 1 | Validation result, assert is_err | Yes (trim) |
| Config: case sensitivity | 1 | Allowlist check, assert bool | No |
| Event: mention with whitespace | 1 | Pure function, assert_eq | No |
| Event: mention-only message | 1 | Pure function, assert_eq | No |
| Event: empty bot_id | 1 | Pure function, assert bool | Yes (is_some_and) |
| Queue: multiple messages | 1 | Async, assert FIFO order | No |

### What Is NOT Tested (Out of Scope)

- Network calls to Slack API (requires live service or digital twin)
- Socket Mode connection/disconnection (integration test territory)
- Event dedup TTL/bounded cache (Phase 2 improvement)
- List formatting, heading conversion (Phase 2 format improvements)
- Block Kit, threading, streaming, modals (not in TinyClaw MVP)

### Testing Constraints

- All tests are `#[test]` or `#[tokio::test]` -- no external test framework
- No mocks (project policy)
- Tests compile with `--features slack` flag (Slack types require the feature)
- Tests are inline in their source file (`#[cfg(test)] mod tests`)

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Slack mention test may fail on edge cases with nested `<>` | The test uses simple, realistic Slack markup patterns. Complex nesting is deferred to Phase 2. |
| `is_some_and` requires Rust 1.70+ | The workspace already uses edition 2024; `is_some_and` is stable since 1.70. |
| Whitespace trim on tokens may break valid tokens with leading/trailing spaces | Slack tokens (xoxb-..., xapp-...) never have leading/trailing whitespace. The Slack API would reject them anyway. |
| New tests may fail if `markdown_to_slack_mrkdwn` has undiscovered bugs | Each step runs tests immediately and fixes bugs before proceeding. |

---

## Implementation Sequence

```
Step 1 ---- format.rs: +6 tests, fix if needed ---- commit checkpoint
Step 2 ---- config.rs: +2 tests, fix trim --------- commit checkpoint
Step 3 ---- slack.rs:  +3 tests, fix is_own -------- commit checkpoint
Step 4 ---- slack.rs:  +1 test -------------------- commit checkpoint
Step 5 ---- full suite + clippy + final commit ---- done
```

Each step is independently verifiable. If any step's tests fail, the bug fix is applied within that same step before moving to the next.

---

## Function Signatures (No Changes)

All existing function signatures remain unchanged. The bug fixes modify function bodies only:

| Function | File | Signature | Body Change |
|----------|------|-----------|-------------|
| `SlackConfig::validate(&self) -> anyhow::Result<()>` | config.rs | Unchanged | `.is_empty()` -> `.trim().is_empty()` |
| `is_own_message(event_user: &str, event_bot_id: Option<&str>, bot_user_id: &str) -> bool` | slack.rs | Unchanged | `.is_some()` -> `.is_some_and(\|id\| !id.is_empty())` |
| `markdown_to_slack_mrkdwn(text: &str) -> String` | format.rs | Unchanged | Possibly add Slack token protection (only if test fails) |

No new public functions, types, or traits are introduced.

---

## Approval Checklist

- [ ] 12 tests are pure-function, no mocks, no network
- [ ] 2 confirmed bugs will be fixed (whitespace tokens, empty bot_id)
- [ ] 1 potential bug documented with regression guard (Slack mentions)
- [ ] All changes are on `feat/tinyclaw-slack-adapter` branch
- [ ] Each step is independently verifiable
- [ ] No new dependencies added
- [ ] No signature changes to existing public API
