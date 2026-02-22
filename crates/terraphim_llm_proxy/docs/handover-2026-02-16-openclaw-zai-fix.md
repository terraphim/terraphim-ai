# Session Handover: Feb 16 2026 - OpenClaw / ZAI Fix

## Branch & Commits

- **Branch**: `main`
- **Working tree**: Clean (no uncommitted changes)

```
70ae5f4 fix: convert "developer" role to "system" for Cerebras and ZAI
b885512 fix(zai): add SSE headers and stream_options for Z.ai streaming
```

## What Was Done

### 1. Root Cause Analysis
- OpenClaw sends messages with role `"developer"` for system prompts
- Cerebras and ZAI APIs don't support `"developer"` role - they expect `"system"` role
- This caused 422 (Cerebras) and 400 (ZAI) errors when proxy forwarded requests

### 2. Fix Implementation
- **Files**: `src/cerebras_client.rs`, `src/zai_client.rs`
- Added role conversion: `"developer"` → `"system"` in both clients
- The fix iterates over messages and converts the role before sending to providers

### 3. Debugging Steps Taken
1. First tried tool format conversion (ZAI needs Anthropic format) - didn't fix the issue
2. Disabled tools to isolate the problem - still failed
3. Added request body logging - discovered messages missing "role" field
4. Realized OpenClaw uses "developer" role which providers reject

### 4. Release
- Created v0.1.10 release
- Pushed to main branch

## Deployment State (linux-small-box)

- **Service**: `terraphim-llm-proxy.service` - active (running)
- **Binary**: `/usr/local/bin/terraphim-llm-proxy` (built on linux-small-box)
- **Config**: `/etc/terraphim-llm-proxy/config.toml`

### Deployed Routes
```toml
default = "zai,glm-5|cerebras,llama-3.3-70b"
background = "openai-codex,gpt-5.2-codex"
think = "openai-codex,gpt-5.2|zai,glm-5|minimax,MiniMax-M2.5"
```

### Provider Status
| Provider | Status | Notes |
|----------|--------|-------|
| openai-codex | 429 rate limited | Usage limit reached |
| zai (GLM-5) | Working | Catches fallback from Codex |
| cerebras | Working | Final fallback in chain |

## Test Results

| Test | Result |
|------|--------|
| Plain text ("hello") | PASS |
| Tool query ("list files in /tmp") | PASS |
| Time query | PASS |

All requests: Codex 429 → ZAI success

## Open Issues

- **#110** - Streaming mid-stream provider failure cannot fall back [enhancement]
- **#72** - API key comparison uses non-constant-time equality [security]
- **#70** - Rate limiter is a stub [security]
- **#69** - SSRF protection is a stub [security]

## Known Gotchas

- linux-small-box builds require `CC=gcc-10 CXX=g++-10` (gcc-9 memcmp bug in aws-lc-sys)
- Rust at `~/.cargo/bin/` on linux-small-box (not in default SSH PATH)
- ZAI streaming may occasionally fail with mid-stream 500 errors (handled by fallback)

## Commands
```bash
# Deploy to linux-small-box
rsync -av --exclude target --exclude .git src/ linux-small-box:/home/alex/terraphim-llm-proxy/src/
ssh linux-small-box "cd /home/alex/terraphim/llm-proxy && export PATH=\$HOME/.cargo/bin:\$PATH && export CC=gcc-10 && export CXX=g++-10 && cargo build --release"
ssh linux-small-box "sudo systemctl stop terraphim-llm-proxy && sudo cp /home/alex/terraphim-llm-proxy/target/release/terraphim-llm-proxy /usr/local/bin/terraphim-llm-proxy && sudo systemctl start terraphim-llm-proxy"

# Test OpenClaw
ssh linux-small-box "export PATH=\$HOME/.bun/install/global/bin:\$PATH && node /home/alex/.bun/install/global/node_modules/openclaw/openclaw.mjs agent -m 'hello' -t '+447842912714' --deliver --thinking low"
```
