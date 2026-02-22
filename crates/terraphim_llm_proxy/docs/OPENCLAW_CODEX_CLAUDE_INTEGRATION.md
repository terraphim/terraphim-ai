# OpenClaw, Codex CLI, and Claude Code Integration Guide

This guide shows how to configure and use the Terraphim LLM Proxy with three popular AI coding assistants:

- **OpenClaw** - Personal AI assistant platform
- **Codex CLI** - OpenAI's coding assistant
- **Claude Code** - Anthropic's coding assistant

## Overview

The proxy now supports intelligent routing for all three clients:

1. **Dual API Support**: Both Anthropic (`/v1/messages`) and OpenAI (`/v1/chat/completions`) endpoints
2. **Smart Model Mapping**: Automatically translates model names when routing to different providers
3. **Client Detection**: Identifies which client is making requests for optimal routing
4. **Provider Flexibility**: Route to OpenRouter, Groq, Anthropic, OpenAI, or custom providers

## Quick Start

### 1. Install the Proxy

```bash
git clone https://github.com/terraphim/terraphim-llm-proxy.git
cd terraphim-llm-proxy
cargo build --release
```

### 2. Configure API Keys

Use 1Password or set environment variables:

```bash
# Using 1Password CLI
export OPENROUTER_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/openrouter-api-key")
export GROQ_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/groq-api-key")
export ANTHROPIC_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/anthropic-api-key")
```

### 3. Create Configuration

Create `config.toml`:

```toml
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "terraphim-test-key-2026"
timeout_ms = 300000

[router]
default = "openrouter,anthropic/claude-3.5-sonnet"
background = "groq,llama-3.1-8b-instant"
think = "openrouter,deepseek/deepseek-r1"
long_context = "openrouter,google/gemini-2.0-flash-exp:free"
long_context_threshold = 60000

[[providers]]
name = "openrouter"
api_base_url = "https://openrouter.ai/api/v1"
api_key = "$OPENROUTER_API_KEY"
models = [
    "anthropic/claude-3.5-sonnet",
    "anthropic/claude-3.5-haiku",
    "deepseek/deepseek-r1",
    "google/gemini-2.0-flash-exp:free"
]
transformers = ["openrouter"]

[[providers]]
name = "groq"
api_base_url = "https://api.groq.com/openai/v1"
api_key = "$GROQ_API_KEY"
models = [
    "llama-3.1-8b-instant",
    "llama-3.1-70b-versatile"
]
transformers = ["openai"]
```

### 4. Start the Proxy

```bash
cargo run --release -- --config config.toml
```

## Client Configuration

### Claude Code

Set environment variables before running Claude Code:

```bash
export ANTHROPIC_BASE_URL=http://127.0.0.1:3456
export ANTHROPIC_API_KEY=terraphim-test-key-2026

# Run Claude Code
claude
```

**Features:**
- Uses `/v1/messages` endpoint (Anthropic API format)
- Automatic routing based on request characteristics
- Think mode detection for complex reasoning
- Background task routing for efficiency

**Example Workflow:**
```bash
# Simple coding task (routes to default provider)
claude "Explain this Rust function"

# Complex analysis (routes to think provider)
claude "Think deeply about the architecture of this codebase"

# Background processing (routes to fast provider)
claude "/background Review all files for TODO comments"
```

### Codex CLI

Create `~/.codex/config.toml`:

```toml
model_provider = "terraphim"
model = "gpt-4o"

[model_providers.terraphim]
name = "Terraphim Proxy"
base_url = "http://127.0.0.1:3456/v1"
env_key = "OPENAI_API_KEY"
```

Set the API key:

```bash
export OPENAI_API_KEY=terraphim-test-key-2026
```

**Features:**
- Uses `/v1/chat/completions` endpoint (OpenAI API format)
- Model mapping: GPT models automatically translate to provider equivalents
- Streaming support for real-time responses

**Example Workflow:**
```bash
# Start interactive session
codex

# One-off command
codex "Refactor this function to use async/await"

# With specific model (translated automatically)
codex --model gpt-4o "Analyze this code"
```

**Model Mapping:**
```
Codex Request: gpt-4o
Proxy translates to (Groq): llama-3.1-70b-versatile
Proxy translates to (OpenRouter): anthropic/claude-3.5-sonnet
```

### OpenClaw

Configure `~/.openclaw/openclaw.json` models section:

```json
{
  "models": {
    "mode": "merge",
    "providers": {
      "terraphim": {
        "baseUrl": "http://127.0.0.1:3456/v1",
        "apiKey": "your-proxy-api-key",
        "api": "openai-completions",
        "models": [
          { "id": "fastest", "name": "Groq Llama 3.3 70B", "reasoning": false },
          { "id": "thinking", "name": "OpenAI Codex GPT-5.2", "reasoning": true },
          { "id": "cheapest", "name": "Groq Llama 3.1 8B", "reasoning": false }
        ]
      }
    }
  },
  "agents": {
    "defaults": {
      "model": {
        "primary": "terraphim/thinking",
        "fallbacks": ["terraphim/fastest", "terraphim/cheapest"]
      }
    }
  }
}
```

**Features:**
- Multi-channel support (WhatsApp, Telegram, Slack, etc.)
- Tool call support (exec, web search, etc.) via Codex Responses API
- Intelligent routing based on message content
- Session management for context continuity

**CLI Reference:**
```bash
# Send agent message (routes through proxy LLM)
openclaw agent --agent main --message "Run whoami and tell me who I am"

# Send agent message and deliver to WhatsApp
openclaw agent --to '+44XXXXXXXXXX' --message 'your prompt' --deliver

# Send direct WhatsApp message (bypasses agent/LLM)
openclaw message send --target '+44XXXXXXXXXX' --message 'Hello'

# Send media file
openclaw message send --target '+44XXXXXXXXXX' --media /path/to/file.epub

# Send media with caption
openclaw message send --target '+44XXXXXXXXXX' --media /path/to/file.epub --message 'Caption text'

# Reply to a specific message
openclaw message send --target '+44XXXXXXXXXX' --message 'Reply text' --reply-to <messageId>

# Check status
openclaw status --deep
openclaw gateway status
openclaw sessions list
```

**Important:** Use `--target` (or `-t`), NOT `--to` or `--channelId`. The old flags are rejected.

## How It Works

### Client Detection

The proxy automatically detects which client is making requests:

| Client | Detection Method | API Format |
|--------|-----------------|------------|
| Claude Code | `anthropic-version` header | Anthropic (`/v1/messages`) |
| Codex CLI | `Authorization: Bearer` + User-Agent | OpenAI (`/v1/chat/completions`) |
| OpenClaw | `User-Agent: OpenClaw/*` | Anthropic (default) |

### Model Translation

When routing to a different provider, model names are automatically translated:

**Example Flow:**
1. **Client Request**: "claude-3-5-haiku" for background task
2. **Router Decision**: Select Groq provider for speed
3. **Model Mapper**: Translates to "llama-3.1-8b-instant"
4. **Provider Request**: Correct model name sent to Groq
5. **Response**: Returned in original API format

**Translation Table:**
| Client Model | OpenRouter | Groq |
|--------------|------------|------|
| claude-3-5-sonnet | anthropic/claude-3.5-sonnet | llama-3.1-70b-versatile |
| claude-3-5-haiku | anthropic/claude-3.5-haiku | llama-3.1-8b-instant |
| gpt-4o | anthropic/claude-3.5-sonnet | llama-3.1-70b-versatile |
| gpt-3.5-turbo | anthropic/claude-3.5-haiku | llama-3.1-8b-instant |

### Tool Call Support

The proxy supports tool calls (function calling) across all provider paths:

**Chat Completions providers** (Groq, Cerebras, OpenRouter, Z.ai):
- `src/tool_call_utils.rs` extracts `tool_calls` from provider responses
- Converts to Anthropic `tool_use` ContentBlocks for `/v1/messages` clients

**Codex Responses API** (chatgpt.com/backend-api/codex/responses):
- Tools converted from Chat Completions nested format to Responses API flat format
- SSE events: `response.output_item.added` captures function call metadata, `response.function_call_arguments.done` provides complete arguments
- Tool result messages (`role: "tool"`) converted to `function_call_output` items
- Assistant tool_call messages converted to `function_call` items

**Message format compatibility:**
- `MessageContent::Null` handles OpenAI's `content: null` on assistant tool_call messages
- `Message` struct supports `tool_calls`, `tool_call_id`, `name` fields for round-trip fidelity

### Routing Scenarios

The proxy automatically selects providers based on request characteristics:

| Scenario | Trigger | Provider Selection |
|----------|---------|-------------------|
| Default | Standard requests | Configured default provider |
| Think | Keywords: "think", "analyze deeply", "step by step" | High-performance reasoning model (e.g., GPT-5.2 via Codex) |
| Background | Low priority / large batch | Fast, cost-effective provider |
| Long Context | Token count > threshold | High context window model |
| Image | Image attachments | Vision-capable model |
| Web Search | Web search tool | Online-enabled model |

## Advanced Configuration

### Custom Model Mappings

Add custom mappings to `config.toml`:

```toml
[[router.model_mappings]]
pattern = "my-custom-model"
target = "openrouter,anthropic/claude-3.5-sonnet"
```

### Provider-Specific Routing

Use explicit provider syntax for specific requests:

```bash
# Claude Code with explicit provider
claude --model "openrouter:anthropic/claude-3-opus"

# Codex CLI with explicit provider
codex --model "groq:llama-3.1-70b-versatile"
```

### Fallback Configuration

Enable automatic fallback on provider errors:

```toml
[router]
strategy = "round_robin"  # or "fill_first"
```

## Troubleshooting

### Check Proxy Health

```bash
curl http://127.0.0.1:3456/health
curl http://127.0.0.1:3456/health/detailed
```

### View Routing Decisions

Enable debug logging:

```bash
RUST_LOG=debug cargo run -- --config config.toml
```

Look for log messages:
```
Model name translated for provider: original_model="claude-3-5-haiku" translated_model="llama-3.1-8b-instant"
Routing decision made: provider="groq" model="llama-3.1-8b-instant" scenario="Background"
```

### Common Issues

**Issue**: Client receives 404 errors
**Solution**: Check that model mappings include the requested model

**Issue**: Routing not working as expected
**Solution**: Verify `router` configuration in config.toml

**Issue**: Authentication errors
**Solution**: Ensure `api_key` matches between proxy config and client

## Testing

Run the integration tests:

```bash
# All tests
cargo test

# Codex CLI specific
cargo test --test codex_integration_tests

# OpenClaw specific
cargo test --test openclaw_integration_tests

# Client detection
cargo test client_detection

# Model mapping
cargo test model_mapper
```

## Architecture

### Data Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Client    │────▶│    Proxy    │────▶│   Router    │────▶│   Provider  │
│(Claude/Codex│     │   Port 3456 │     │(6-phase)    │     │(OpenRouter, │
│ /OpenClaw)  │     │             │     │             │     │   Groq, etc)│
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                           │
                    ┌─────────────┐
                    │Model Mapper │
                    │(translation)│
                    └─────────────┘
```

### Components

- **Client Detection** (`src/client_detection.rs`): Identifies client type from headers
- **Model Mapper** (`src/routing/model_mapper.rs`): Translates model names
- **Router** (`src/router.rs`): 6-phase routing with translation
- **Server** (`src/server.rs`): HTTP handlers for both API formats

## Performance

- **Routing Overhead**: < 1ms per request
- **Model Translation**: < 0.5ms (with caching)
- **Throughput**: 1000+ requests/second
- **Concurrent Clients**: Tested with 100+ simultaneous connections

## Security

- API keys validated on every request
- SSRF protection enabled by default
- Rate limiting configurable per provider
- Request/response logging (configurable)

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development guidelines.

## License

MIT License - See [LICENSE](../LICENSE) for details.

## Support

- GitHub Issues: https://github.com/terraphim/terraphim-llm-proxy/issues
- Documentation: https://docs.terraphim.io
- Discord: https://discord.gg/terraphim
