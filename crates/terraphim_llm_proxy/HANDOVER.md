# Handover Document - terraphim-llm-proxy

**Date**: 2026-02-08
**Branch**: master
**Last Commit**: 33c9ef6
**Session Focus**: Tool Call Support for Codex Responses API + OpenAI Message Format Compatibility

---

## Progress Summary

### Completed This Session

1. **Tool Call Support for All Providers (1 commit)**

   | Commit | Description |
   |--------|-------------|
   | `9d1c95d` | Stop silently dropping tool_calls from LLM responses. Added `tool_call_utils.rs` with `extract_tool_calls()` for Chat Completions providers (Groq, Cerebras, OpenRouter, Z.ai, genai). |

   The proxy was silently discarding `tool_calls` from LLM responses. Chat Completions providers return tool calls in `response.choices[0].message.tool_calls` but the proxy only extracted `content`. Now tool_calls are properly converted to Anthropic `tool_use` ContentBlocks.

2. **Codex Responses API Tool Call Support (5 commits)**

   The Codex path (`chatgpt.com/backend-api/codex/responses`) uses an entirely different request/response format from Chat Completions. Tool calls required 5 fixes across 3 files:

   | Commit | Description |
   |--------|-------------|
   | `a99bca3` | Add tool_call support: convert tools to Responses API flat format, handle `response.output_item.added` and `response.function_call_arguments.done` SSE events in both streaming and non-streaming paths |
   | `67e1d88` | Remove unsupported params (`max_output_tokens`, `temperature`, `top_p`) from ChatGPT backend-api requests (400 Bad Request) |
   | `6257cbd` | Use fallback name from `output_item.added` for function_calls (ChatGPT sometimes omits name in `arguments.done` event) |
   | `b07c74a` | Support OpenAI tool message format: add `MessageContent::Null` variant, `tool_calls`/`tool_call_id`/`name` fields to `Message`, implement `Default` for `Message` and `MessageContent` |
   | `33c9ef6` | Convert `role: "tool"` messages to `function_call_output` items and assistant tool_call messages to `function_call` items (Responses API rejects `role: "tool"`) |

3. **OpenClaw End-to-End Tool Call Verification**

   Verified on linux-small-box via OpenClaw agent:
   - First turn: `"Check my username by running whoami"` -> exec tool call -> returned `"Your current username is: **alex**"`
   - Second turn: `"What directory am I in? Run pwd"` -> exec tool call -> returned `"You're in: /home/alex/clawd"`
   - Multi-turn tool_call conversation works correctly through the full pipeline

### Previous Session (still deployed)

4. **ChatGPT Backend-API Codex Responses Support (4 commits)**

   Rewrote `OpenAiCodexClient` to use `chatgpt.com/backend-api/codex/responses` endpoint with Responses API format. Debugged and fixed SSE streaming across 3 iterations.

   | Commit | Description |
   |--------|-------------|
   | `8dba71e` | Rewrite OpenAiCodexClient: new endpoint, Responses API format, SSE event parsing, streaming dispatch in client.rs |
   | `e38e676` | Attempt 1: prevent EventSource reconnection on stream end (did not fix root cause) |
   | `d520078` | Attempt 2: force HTTP/1.1 with lenient headers (did not fix root cause) |
   | `33370b2` | Working fix: replace EventSource entirely with raw reqwest + tokio-util StreamReader for SSE parsing |

5. **Knowledge Graph (RoleGraph) Routing - Deployed and Verified on linux-small-box**

   - Deployed taxonomy files to `/etc/terraphim-llm-proxy/taxonomy/` (systemd `ProtectHome=true` blocks `/home/`)
   - Set `ROLEGRAPH_TAXONOMY_PATH=/etc/terraphim-llm-proxy/taxonomy` in service env
   - Verified: 9 taxonomy files loaded, 120 Aho-Corasick patterns built

6. **Fix: Model Mappings for Streaming Requests (1 commit)**

   | Commit | Description |
   |--------|-------------|
   | `11b2247` | `handle_chat_completions` applied model mappings AFTER the streaming branch, so streaming requests never got mapped. Moved `apply_model_mappings()` before the streaming/non-streaming dispatch. |

7. **Openclaw WhatsApp Integration - End-to-End Verified**

   - Installed `openclaw-gateway.service` (v2026.2.6-3)
   - Changed openclaw primary model to `terraphim/thinking` (via proxy)
   - Verified WhatsApp message delivery through proxy

8. **Model Switch: gpt-5.2-codex -> gpt-5.2 (2 commits)**

   | Commit | Description |
   |--------|-------------|
   | `aa0593d` | Updated `think_routing.md` route to `openai-codex, gpt-5.2-codex` |
   | `b6440a0` | Switch model from `gpt-5.2-codex` to `gpt-5.2` |

### What's Working

- **Tool calls via Codex Responses API**: Full round-trip: tools sent in flat format, function_call SSE events parsed, tool results sent as `function_call_output` items
- **Tool calls via Chat Completions providers**: `tool_call_utils.rs` extracts tool_calls from Groq, Cerebras, OpenRouter, Z.ai, genai responses
- **OpenAI message format compatibility**: `MessageContent::Null`, `tool_calls`, `tool_call_id`, `name` fields on `Message` struct
- **ChatGPT Backend-API streaming**: Raw reqwest + tokio-util StreamReader parses SSE from `chatgpt.com/backend-api/codex/responses`
- **Knowledge graph routing**: 120 Aho-Corasick patterns, sub-millisecond matching; "think", "step by step" trigger codex routing
- **Model mappings**: `thinking` -> `openai-codex,gpt-5.2` works for both streaming and non-streaming
- **Openclaw via WhatsApp**: Messages route through proxy -> GPT-5.2, responses delivered to WhatsApp
- **All tests pass**: 567 lib tests, codex client tests (9), tool_call_utils tests

### Verified End-to-End Data Flow (with tool calls)

```
OpenClaw agent sends: "Run whoami"
  -> POST http://127.0.0.1:3456/v1/chat/completions (with tools: [{type:"function", function:{name:"exec",...}}])
  -> Proxy: model mapping "thinking" -> "openai-codex,gpt-5.2"
  -> OpenAiCodexClient.build_request_body(): converts tools to flat Responses API format
  -> POST https://chatgpt.com/backend-api/codex/responses (tools + input)
  -> SSE: response.output_item.added (function_call, captures call_id + name)
  -> SSE: response.function_call_arguments.done (arguments, fallback name from added event)
  -> Proxy: returns tool_use ContentBlock to OpenClaw
  -> OpenClaw executes tool, sends result back with role:"tool" + tool_call_id
  -> Proxy: converts role:"tool" -> function_call_output item, assistant tool_calls -> function_call items
  -> POST https://chatgpt.com/backend-api/codex/responses (conversation with tool results)
  -> GPT-5.2 responds with text incorporating tool output
  -> User receives: "Your current username is: alex"
```

### Useful openclaw commands

```bash
# Send direct WhatsApp message (bypasses agent/LLM)
openclaw message send --target '+44XXXXXXXXXX' --message 'Hello'

# Send message through agent (routes through proxy LLM, then delivers)
openclaw agent --to '+44XXXXXXXXXX' --message 'your prompt' --deliver

# Check WhatsApp connection and channel health
openclaw status --deep

# Check gateway service
openclaw gateway status

# List sessions
openclaw sessions list
```

---

## Technical Context

```
Branch: master
Up to date with origin/master (pushed)

Recent commits:
33c9ef6 fix(codex): convert tool/assistant messages to Responses API format
b07c74a fix(compat): support OpenAI tool message format in MessageContent
6257cbd fix(codex): use fallback name from output_item.added for function_calls
67e1d88 fix(codex): remove unsupported params from ChatGPT backend-api requests
a99bca3 fix(codex): add tool_call support to Codex Responses API path
9d1c95d fix: stop silently dropping tool_calls from LLM responses
```

### Key Files Changed (this session)

| File | Change |
|------|--------|
| `src/tool_call_utils.rs` | NEW: `extract_tool_calls()` for Chat Completions providers, converts `tool_calls` array to `tool_use` ContentBlocks |
| `src/openai_codex_client.rs` | Tool conversion in `build_request_body()`, function_call SSE handling in `send_request()`, assistant/tool message conversion to Responses API format |
| `src/client.rs` | Streaming converter handles `response.output_item.added` + `response.function_call_arguments.done` SSE events with `fc_state` HashMap |
| `src/token_counter.rs` | `MessageContent::Null` variant, `tool_calls`/`tool_call_id`/`name` fields on `Message`, `Default` impls |
| `src/server.rs` | Tool call extraction in non-streaming handlers |
| `src/cerebras_client.rs` | Tool call extraction + new Message fields |
| `src/groq_client.rs` | New Message fields |
| `src/openrouter_client.rs` | Tool call extraction + new Message fields |
| `src/zai_client.rs` | Tool call extraction + new Message fields |
| `src/analyzer.rs` | `MessageContent::Null` match arm + new Message fields |
| `src/transformer/*.rs` | `MessageContent::Null` match arms + new Message fields |
| `tests/*.rs` | New Message fields (`..Default::default()`) |
| `examples/*.rs` | New Message fields |

### Key Files (previous session, still relevant)

| File | Change |
|------|--------|
| `src/openai_codex_client.rs` | Raw reqwest + tokio-util StreamReader SSE parsing, Responses API format |
| `src/client.rs` | `"openai-codex"` arm in streaming dispatch |
| `src/server.rs` | `apply_model_mappings()` before streaming/non-streaming branch |
| `Cargo.toml` | `tokio-util = { version = "0.7.18", features = ["io"] }` |
| `docs/taxonomy/routing_scenarios/think_routing.md` | Route: `openai-codex, gpt-5.2` |

### linux-small-box Configuration (deployed and verified)

| File | Value |
|------|-------|
| `/etc/terraphim-llm-proxy/env` | `ROLEGRAPH_TAXONOMY_PATH=/etc/terraphim-llm-proxy/taxonomy` |
| `/etc/terraphim-llm-proxy/taxonomy/` | 9 routing scenario files, 120 Aho-Corasick patterns |
| `/etc/terraphim-llm-proxy/taxonomy/routing_scenarios/think_routing.md` | `route:: openai-codex, gpt-5.2` |
| `/etc/terraphim-llm-proxy/config.toml` | `thinking` -> `openai-codex,gpt-5.2` |
| `~/.openclaw/openclaw.json` | Primary model -> `terraphim/thinking`; fallbacks -> `terraphim/fastest`, `terraphim/cheapest` |
| `~/.config/systemd/user/openclaw-gateway.service` | Installed via `openclaw daemon install`, running |

---

## Architecture: ChatGPT Backend-API

```
Why reqwest_eventsource cannot be used:
- chatgpt.com returns `x-codex-credits-balance:` header with empty value
- reqwest_eventsource internally validates all response headers
- Empty header values cause "Invalid header value" error before any SSE events
- This happens at HTTP response level, not during SSE streaming

Solution: raw reqwest + tokio-util StreamReader
- reqwest .send() handles headers more leniently
- response.bytes_stream() gives raw byte stream
- tokio_util::io::StreamReader wraps it for AsyncBufRead
- Manual line-by-line SSE parsing: strip "data: " prefix, yield JSON
```

---

## GitHub Issues

- **#63, #64, #65, #91, #92, #93, #94, #98, #99**: CLOSED
- **#95** (runtime API key override), **#96** (dead code warnings): open
- **#66-#90**: open (refactor, tech-debt, performance)

---

## Architecture: Codex Responses API Tool Call Format

### Request: Tools

Chat Completions format (what clients send):
```json
{"type": "function", "function": {"name": "exec", "description": "...", "parameters": {...}}}
```

Responses API format (what chatgpt.com expects -- flat, no nesting):
```json
{"type": "function", "name": "exec", "description": "...", "parameters": {...}}
```

### Request: Tool Result Messages

Clients send `role: "tool"` with `tool_call_id`. Responses API rejects `role: "tool"` entirely. Must convert:
- `role: "assistant"` with `tool_calls` -> `type: "function_call"` items (with `call_id`, `name`, `arguments`)
- `role: "tool"` with `tool_call_id` -> `type: "function_call_output"` items (with `call_id`, `output`)

### Response: SSE Events for Tool Calls

1. `response.output_item.added` (item.type == "function_call") -- captures `call_id` and `name`
2. `response.function_call_arguments.delta` -- incremental arguments (ignored, wait for done)
3. `response.function_call_arguments.done` -- complete call with `arguments` (may lack `name` -- fallback to added event)

### Unsupported Parameters

ChatGPT backend-api returns 400 for: `max_output_tokens`, `temperature`, `top_p`. These are stripped in `build_request_body()`.

---

## Next Steps

### Prioritized Action Items

1. **Test more routing scenarios** -- Only `think_routing` tested end-to-end. Other taxonomy files (`background_routing`, `low_cost_routing`, `image_routing`, etc.) should be verified.

2. **Monitor WhatsApp reliability** -- Gateway shows periodic "closed before connect" WebSocket errors. WhatsApp channel is linked and functional but monitor for disconnections.

3. **Issue #95** (runtime API key override) -- More relevant now since OAuth tokens override provider keys.

4. **Issue #96** (dead code warnings) -- Pre-existing warnings in oauth modules.

### Blockers

- None. All systems deployed and verified.

### Recommended Approach

- Run openclaw coding tasks through the proxy to validate under real workloads
- Monitor for token expiry during long coding sessions (TokenManager handles refresh, but verify)
- Consider adding more taxonomy routes for different routing scenarios

---

## Deployment Notes

### systemd Service (terraphim-llm-proxy)
- `ProtectHome=true` -- service CANNOT access `/home/`. Taxonomy files must be in `/etc/` or `/var/`
- Service runs as `User=root`
- Env file: `/etc/terraphim-llm-proxy/env`
- Config: `/etc/terraphim-llm-proxy/config.toml`
- Binary: `/usr/local/bin/terraphim-llm-proxy`
- Must build ON linux-small-box (macOS binaries cause Exec format error)

### Openclaw Gateway (openclaw-gateway.service)
- User systemd service: `~/.config/systemd/user/openclaw-gateway.service`
- Binary: `~/.bun/bin/openclaw` (installed via bun)
- Config: `~/.openclaw/openclaw.json`
- Workspace: `~/clawd`
- Version: 2026.2.6-3
- Gateway port: 18789 (loopback only)
- WhatsApp: linked, dm:allowlist, allow:+44XXXXXXXXXX

### Build Notes
- linux-small-box needs `CC=gcc-10 CXX=g++-10` for release builds (gcc-9 memcmp bug in aws-lc-sys)
- `source ~/.cargo/env` required before cargo commands on linux-small-box
- Build time: ~3 minutes for release profile
- SSH: use `ssh linux-small-box` (NOT `ssh alex@linux-small-box` -- the latter causes auth failures)
- OpenClaw on linux-small-box: `export PATH=$PATH:/home/alex/.npm-global/bin`

### OpenClaw Session Reset
If OpenClaw sessions get corrupted (422 deserialization errors), reset by renaming the session file:
```bash
# Find session ID from sessions.json
cat ~/.openclaw/agents/main/sessions/sessions.json
# Rename the problematic session
mv ~/.openclaw/agents/main/sessions/{sessionId}.jsonl ~/.openclaw/agents/main/sessions/{sessionId}.jsonl.bak
```
