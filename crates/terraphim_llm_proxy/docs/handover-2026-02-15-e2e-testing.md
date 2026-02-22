# Session Handover: Feb 15 2026 - E2E Testing & Provider Fixes

## Branch & Commits

- **Branch**: `main`
- **Working tree**: Clean (no uncommitted changes)

```
e461905 fix: add fallback chain to streaming path for provider failover
2965add fix: convert OpenAI-format tools to Anthropic format for MiniMax
a70f00b fix: make HTTP 500 errors fallback-eligible in provider chain
```

## What Was Done

### 1. HTTP 500 Fallback Eligibility (Issue #107, closed)
- **File**: `src/retry.rs`
- ZAI (Zhipu AI) returns `"HTTP 500 - ..."` format errors that didn't match `is_fallback_eligible()` patterns
- Added `"http 5"` and `" 500"` pattern matches
- Added 3 unit tests for fallback eligibility

### 2. MiniMax Tool Format Conversion (Issue #108, closed)
- **File**: `src/minimax_client.rs`
- MiniMax uses Anthropic API format but proxy sent OpenAI-format tools (400 error)
- Added `convert_tool_to_anthropic()` to transform tools before sending
- Added 2 unit tests for tool conversion (OpenAI->Anthropic, passthrough)

### 3. Streaming Fallback Chain (Issue #109, closed)
- **File**: `src/server.rs`
- `create_openai_sse_stream()` only tried primary provider, no fallback on failure
- Restructured to iterate over `execution_targets` with `is_fallback_eligible()` checks
- Deferred initial `role: assistant` SSE chunk until provider succeeds
- Pre-stream failures (429, connection errors) now trigger fallback; mid-stream failures cannot

### 4. End-to-End Testing on linux-small-box
- Ran 12 tests across all routes, model mappings, streaming/non-streaming, with/without tools
- **11 of 12 passed**
- Filed #110 for the one failure (mid-stream ZAI 500 during streaming)

## Deployment State (linux-small-box)

- **Service**: `terraphim-llm-proxy.service` - active (running)
- **Binary**: `/usr/local/bin/terraphim-llm-proxy` (built on linux-small-box with `CC=gcc-10 CXX=g++-10`)
- **Config**: `/etc/terraphim-llm-proxy/config.toml`

### Deployed Routes
```toml
default = "zai,glm-5|cerebras,llama-3.3-70b"
think = "openai-codex,gpt-5.2|zai,glm-5|minimax,MiniMax-M2.5"
background = "openai-codex,gpt-5.2-codex"
```

### Provider Status
| Provider | Status | Notes |
|----------|--------|-------|
| openai-codex | 429 rate limited | `usage_limit_reached`, all requests fall through |
| zai (GLM-5) | Intermittent 500s | Works most of the time; some requests fail mid-stream |
| cerebras | Healthy | Catches fallback from default route |
| minimax (M2.5) | Healthy | Anthropic-compatible endpoint, tool calls work |

## E2E Test Results

### Route: `default` (zai,glm-5 | cerebras,llama-3.3-70b)
| Test | Result | Provider |
|------|--------|----------|
| Non-streaming, plain text | PASS | cerebras/llama-3.3-70b (ZAI failed, fell back) |
| Non-streaming, tools | PASS | zai/glm-5 |
| Streaming, plain text | FAIL | zai/glm-5 (mid-stream 500) |
| Streaming, tools | PASS | zai/glm-5 |

### Route: `think` (openai-codex,gpt-5.2 | zai,glm-5 | minimax,MiniMax-M2.5)
| Test | Result | Provider |
|------|--------|----------|
| Non-streaming, plain text | PASS | cerebras/llama-3.3-70b (Codex+ZAI failed) |
| Non-streaming, tools | PASS | zai/glm-5 |
| Streaming, plain text | PASS | zai/glm-5 |
| Streaming, tools | PASS | zai/glm-5 |

### Route: `background` (openai-codex,gpt-5.2-codex)
| Test | Result | Provider |
|------|--------|----------|
| Non-streaming, plain text | PASS | cerebras/llama-3.3-70b (Codex 429, fell back) |

### Model Mappings
| Mapping | Result | Provider |
|---------|--------|----------|
| thinking -> openai-codex,gpt-5.2 | PASS | cerebras/llama-3.3-70b |
| cheapest -> cerebras,llama3.1-8b | PASS | cerebras/llama3.1-8b |
| fastest -> cerebras,llama-3.3-70b | PASS | cerebras/llama-3.3-70b |

## Open Issues

### Recent (this session)
- **#110** - Streaming mid-stream provider failure cannot fall back [enhancement]

### High Priority (pre-existing)
- **#72** - API key comparison uses non-constant-time equality [security]
- **#71** - Management API allow_remote not enforced [security]
- **#70** - Rate limiter is a stub [security]
- **#69** - SSRF protection is a stub [security]
- **#68** - Custom provider clients bypass genai unnecessarily [tech-debt]

### Medium Priority (pre-existing)
- **#101** - Codex token import: support new auth.json schema
- **#100** - Cerebras capability-aware routing
- **#82** - Management API auth bypass when no secret_key set
- **#78** - XSS in OAuth callback HTML
- **#77** - Config save writes expanded API keys to disk

## Known Gotchas

- linux-small-box builds require `CC=gcc-10 CXX=g++-10` (gcc-9 memcmp bug in aws-lc-sys)
- Rust at `~/.cargo/bin/` on linux-small-box (not in default SSH PATH)
- Build script fetches from Groq/Cerebras APIs at compile time (Cerebras often 403s)
- chatgpt.com returns empty header values that break reqwest_eventsource
- ZAI streaming sends `reasoning_content` (silently dropped) not `content` for chain-of-thought
- systemd ProtectHome=true - taxonomy must be in /etc/ or /var/
- Router model format uses `:` or `,` separators, NOT `/`

## Test Commands
```bash
cargo test --lib -- retry          # 19 retry tests
cargo test --lib -- minimax        # 5 minimax tests
cargo test --lib                   # 567+ lib tests
cargo test                         # 718+ total tests
```
