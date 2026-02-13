# TinyClaw Multi-Channel Validation Plan

## Overview

This plan validates the TinyClaw multi-channel AI assistant across CLI, Telegram, and Discord channels to ensure production readiness.

**Scope**: End-to-end testing of all three channel adapters with live connections
**Duration**: 2-3 hours
**Prerequisites**: Access to Telegram bot token, Discord bot token, and terraphim-llm-proxy

---

## Phase 1: Environment Setup (15 minutes)

### 1.1 Prerequisites Check

```bash
# Verify Rust toolchain
cargo --version
rustc --version

# Check terraphim-llm-proxy availability
curl http://localhost:3001/health  # Adjust port as needed

# Verify tokens are set in environment
echo $TELEGRAM_BOT_TOKEN
echo $DISCORD_BOT_TOKEN
echo $PROXY_API_KEY
```

### 1.2 Build and Verify

```bash
# Build tinyclaw with all features
cargo build -p terraphim_tinyclaw --all-features --release

# Verify binary exists
ls -la target/release/tinyclaw

# Test help output
./target/release/tinyclaw --help
```

### 1.3 Create Test Configuration

Create `/tmp/tinyclaw-test.toml`:

```toml
[proxy]
endpoint = "http://localhost:3001/v1/chat/completions"
api_key = "${PROXY_API_KEY}"
model = "gpt-4o-mini"
max_tokens = 4096

[sessions]
storage_path = "/tmp/tinyclaw-sessions"
autosave_interval_secs = 30

[agent]
system_prompt_path = "/tmp/tinyclaw-system.md"
max_iterations = 20

[channels.telegram]
enabled = true
bot_token = "${TELEGRAM_BOT_TOKEN}"
allow_from = ["@your_telegram_username"]

[channels.discord]
enabled = true
bot_token = "${DISCORD_BOT_TOKEN}"
allow_from = ["your_discord_user_id"]
```

Create `/tmp/tinyclaw-system.md`:

```markdown
# TinyClaw System Prompt

You are TinyClaw, a helpful AI assistant with access to tools.
You can help users with:
- Reading and writing files
- Executing safe shell commands
- Searching the web
- Editing files

Always be helpful, accurate, and safe.
```

---

## Phase 2: CLI Channel Validation (30 minutes)

### 2.1 Agent Mode Testing

```bash
# Terminal 1: Start agent mode
./target/release/tinyclaw agent --config /tmp/tinyclaw-test.toml
```

**Test Cases**:

| Test ID | Input | Expected Result | Status |
|---------|-------|-----------------|--------|
| CLI-001 | "Hello, can you help me?" | Response acknowledging readiness | ⬜ |
| CLI-002 | "/help" | List available slash commands | ⬜ |
| CLI-003 | "What files are in /tmp?" | Uses filesystem tool, lists files | ⬜ |
| CLI-004 | "Create a file /tmp/test.txt with content 'Hello World'" | File created successfully | ⬜ |
| CLI-005 | "Read /tmp/test.txt" | Returns "Hello World" | ⬜ |
| CLI-006 | "Run command: echo 'Test output'" | Returns "Test output" | ⬜ |
| CLI-007 | "Run command: rm -rf /" | BLOCKED with safety message | ⬜ |
| CLI-008 | "/reset" | Session cleared, confirmation message | ⬜ |
| CLI-009 | "What was the first thing I asked?" | Should NOT remember (session reset) | ⬜ |
| CLI-010 | Long message (>4096 chars) | Properly chunked and displayed | ⬜ |

### 2.2 Session Persistence Test

```bash
# Test 1: Send a message
# Test 2: Ctrl+C to exit
# Test 3: Restart agent
# Test 4: Ask "What did we discuss?"
# Verify: Previous context should be retained
```

### 2.3 Error Handling

| Test ID | Scenario | Expected Result | Status |
|---------|----------|-----------------|--------|
| CLI-011 | Invalid config path | Clear error message, graceful exit | ⬜ |
| CLI-012 | Proxy unavailable | Warning message, text-only mode | ⬜ |
| CLI-013 | Malformed user input | Graceful handling, no panic | ⬜ |

---

## Phase 3: Telegram Channel Validation (45 minutes)

### 3.1 Bot Setup

1. Message @BotFather on Telegram
2. Create new bot or use existing
3. Copy bot token
4. Add bot to a test group (optional)
5. Get your user ID via @userinfobot

### 3.2 Gateway Mode Startup

```bash
# Terminal 1: Start gateway mode
./target/release/tinyclaw gateway --config /tmp/tinyclaw-test.toml

# Verify: "Starting gateway mode..." in logs
# Verify: "Telegram bot connected" message
```

### 3.3 Telegram Test Cases

**Direct Messages**:

| Test ID | Action | Expected Result | Status |
|---------|--------|-----------------|--------|
| TEL-001 | Send "Hello" to bot | Response received | ⬜ |
| TEL-002 | Send "/help" | Slash command list | ⬜ |
| TEL-003 | Send **bold** text | Response formatted with HTML | ⬜ |
| TEL-004 | Send `code` text | Response formatted with HTML | ⬜ |
| TEL-005 | Send [link](http://example.com) | Clickable link in response | ⬜ |
| TEL-006 | Request file listing | Uses tool, returns formatted list | ⬜ |
| TEL-007 | Send long message (>4096 chars) | Multiple messages, properly split | ⬜ |
| TEL-008 | Send "/reset" | Session cleared | ⬜ |

**Group Chat** (if applicable):

| Test ID | Action | Expected Result | Status |
|---------|--------|-----------------|--------|
| TEL-009 | Add bot to group | Bot acknowledges | ⬜ |
| TEL-010 | Mention bot in message | Bot responds to mention | ⬜ |
| TEL-011 | Non-whitelisted user messages | Message ignored/no response | ⬜ |

### 3.4 Security Testing

| Test ID | Action | Expected Result | Status |
|---------|--------|-----------------|--------|
| TEL-012 | Whitelist bypass attempt | Access denied | ⬜ |
| TEL-013 | Malformed message | Graceful handling | ⬜ |
| TEL-014 | Rapid message spam | Rate limiting / graceful handling | ⬜ |

---

## Phase 4: Discord Channel Validation (45 minutes)

### 4.1 Bot Setup

1. Go to Discord Developer Portal: https://discord.com/developers/applications
2. Create new application or use existing
3. Go to "Bot" section, copy token
4. Enable "Message Content Intent"
5. Go to OAuth2 → URL Generator
6. Select scopes: `bot`
7. Select permissions: `Send Messages`, `Read Message History`
8. Use generated URL to invite bot to test server
9. Get your user ID (right-click profile → Copy User ID with Developer Mode)

### 4.2 Gateway Mode with Discord

```bash
# Use same config with Discord enabled
./target/release/tinyclaw gateway --config /tmp/tinyclaw-test.toml

# Verify: "Discord bot connected" message
```

### 4.3 Discord Test Cases

**Direct Messages**:

| Test ID | Action | Expected Result | Status |
|---------|--------|-----------------|--------|
| DSC-001 | Send "Hello" to bot | Response received | ⬜ |
| DSC-002 | Send "/help" | Slash command list | ⬜ |
| DSC-003 | Send **bold** text | Markdown preserved | ⬜ |
| DSC-004 | Request tool execution | Tool runs, result returned | ⬜ |
| DSC-005 | Send long message (>2000 chars) | Properly chunked for Discord | ⬜ |

**Server/Guild Testing**:

| Test ID | Action | Expected Result | Status |
|---------|--------|-----------------|--------|
| DSC-006 | Mention @bot in channel | Bot responds | ⬜ |
| DSC-007 | Reply to bot message | Context maintained | ⬜ |
| DSC-008 | Non-whitelisted user messages | Ignored | ⬜ |

### 4.4 Discord-Specific Features

| Test ID | Action | Expected Result | Status |
|---------|--------|-----------------|--------|
| DSC-009 | Use Discord emoji in message | Handled gracefully | ⬜ |
| DSC-010 | Send code block (```) | Formatted response | ⬜ |
| DSC-011 | Bot disconnects/reconnects | Auto-reconnection | ⬜ |

---

## Phase 5: Multi-Channel Concurrent Testing (30 minutes)

### 5.1 Session Isolation Test

| Test ID | Action | Expected Result | Status |
|---------|--------|-----------------|--------|
| MUL-001 | Send message via CLI | Response via CLI | ⬜ |
| MUL-002 | Send different message via Telegram | Response via Telegram | ⬜ |
| MUL-003 | Send third message via Discord | Response via Discord | ⬜ |
| MUL-004 | Verify: Each channel has separate session | No cross-contamination | ⬜ |

### 5.2 Concurrent Load Test

```bash
# Send messages rapidly across all three channels
# Observe: No dropped messages, proper queue handling
```

### 5.3 Graceful Shutdown

| Test ID | Action | Expected Result | Status |
|---------|--------|-----------------|--------|
| MUL-005 | Press Ctrl+C in gateway | "Shutting down gracefully" | ⬜ |
| MUL-006 | Verify sessions saved | Check /tmp/tinyclaw-sessions/*.jsonl | ⬜ |
| MUL-007 | Restart gateway | Previous sessions loaded | ⬜ |

---

## Phase 6: Tool Execution Validation (30 minutes)

Test tool functionality across all channels:

### 6.1 Filesystem Tool

| Channel | Test | Command | Expected Result | Status |
|---------|------|---------|-----------------|--------|
| CLI | Read file | "Read /etc/hosts" | File contents | ⬜ |
| Telegram | List directory | "List files in /tmp" | File list | ⬜ |
| Discord | Write file | "Create /tmp/discord-test.txt" | File created | ⬜ |

### 6.2 Shell Tool (Safe Commands Only)

| Channel | Test | Command | Expected Result | Status |
|---------|------|---------|-----------------|--------|
| CLI | Echo | "Run: echo 'test'" | "test" | ⬜ |
| Telegram | Date | "What's the current date?" | Date output | ⬜ |
| Discord | Dangerous command | "Run: rm -rf /" | BLOCKED | ⬜ |

### 6.3 Edit Tool

| Channel | Test | Command | Expected Result | Status |
|---------|------|---------|-----------------|--------|
| CLI | Replace text | "Replace 'old' with 'new' in /tmp/test.txt" | Success | ⬜ |
| Telegram | Ambiguous replace | "Replace 'a' with 'b' in /tmp/test.txt" | Uniqueness error | ⬜ |

### 6.4 Web Tool

| Channel | Test | Command | Expected Result | Status |
|---------|------|---------|-----------------|--------|
| CLI | Web search | "Search for Rust programming" | Results | ⬜ |
| Telegram | URL fetch | "Fetch https://example.com" | Content summary | ⬜ |
| Discord | SSRF attempt | "Fetch http://localhost:3001" | BLOCKED | ⬜ |

---

## Phase 7: Reporting & Documentation (15 minutes)

### 7.1 Test Results Summary

Create validation report with:

```markdown
# TinyClaw Multi-Channel Validation Report

**Date**: [Date]
**Tester**: [Name]
**Branch**: claude/tinyclaw-terraphim-plan-lIt3V
**Commit**: [Commit hash]

## Test Execution Summary

| Phase | Tests | Passed | Failed | Skipped |
|-------|-------|--------|--------|---------|
| CLI | 13 | | | |
| Telegram | 14 | | | |
| Discord | 11 | | | |
| Multi-Channel | 7 | | | |
| Tools | 12 | | | |
| **Total** | **57** | | | |

## Issues Found

| ID | Severity | Description | Steps to Reproduce | Expected | Actual |
|----|----------|-------------|-------------------|----------|--------|

## Sign-off

- [ ] CLI channel validated
- [ ] Telegram channel validated
- [ ] Discord channel validated
- [ ] All tools working
- [ ] Security measures effective
- [ ] Sessions persisting correctly

**Status**: [READY FOR PRODUCTION / NEEDS FIXES]
```

### 7.2 Evidence Collection

Save the following:
- Screenshot of CLI interactions
- Screenshot of Telegram bot responses
- Screenshot of Discord bot responses
- Session files from `/tmp/tinyclaw-sessions/`
- Gateway logs (redact tokens)

---

## Rollback Plan

If critical issues are found:

```bash
# Stop gateway
Ctrl+C

# Clean up sessions
rm -rf /tmp/tinyclaw-sessions

# Clean up test files
rm /tmp/test.txt /tmp/discord-test.txt

# Disable bots
# - Telegram: Message @BotFather, /setprivacy → Disable
# - Discord: Developer Portal → Bot → Disable
```

---

## Appendix: Quick Reference

### Commands

```bash
# Build
cargo build -p terraphim_tinyclaw --all-features --release

# CLI mode
./target/release/tinyclaw agent --config /tmp/tinyclaw-test.toml

# Gateway mode
./target/release/tinyclaw gateway --config /tmp/tinyclaw-test.toml

# Run tests
cargo test -p terraphim_tinyclaw --all-features

# Check logs
tail -f /tmp/tinyclaw-sessions/*.log  # If logging configured
```

### Environment Variables

```bash
export TELEGRAM_BOT_TOKEN="your_token_here"
export DISCORD_BOT_TOKEN="your_token_here"
export PROXY_API_KEY="your_key_here"
```

### Troubleshooting

| Issue | Solution |
|-------|----------|
| Bot not responding | Check token, verify bot is running |
| 403 Forbidden | Check allow_from whitelist |
| Proxy errors | Verify terraphim-llm-proxy is running |
| Session not persisting | Check write permissions on storage_path |
| Message too long | Should auto-chunk, check logs |

---

**Next Steps After Validation**:
1. Document any issues found
2. Create GitHub issues for bugs
3. Update deployment documentation
4. Plan production deployment
