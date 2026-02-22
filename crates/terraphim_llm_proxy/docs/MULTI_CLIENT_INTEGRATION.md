# Multi-Client Integration Guide

This guide shows how to integrate terraphim-llm-proxy with popular LLM clients including Claude Code, OpenClaw, Codex CLI, and direct Groq access.

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Intelligent Pattern Routing](#intelligent-pattern-routing)
4. [Claude Code Integration](#claude-code-integration)
5. [OpenClaw Integration](#openclaw-integration)
6. [Codex CLI Integration](#codex-cli-integration)
7. [Direct Groq Integration](#direct-groq-integration)
8. [Direct Cerebras Integration](#direct-cerebras-integration)
9. [Configuration Profiles](#configuration-profiles)
10. [Troubleshooting](#troubleshooting)

---

## Overview

### Supported Clients

| Client | API Format | Detection Method | Primary Use Case |
|--------|------------|------------------|------------------|
| Claude Code | Anthropic Messages | `anthropic-version` header | Coding assistance |
| OpenClaw | Anthropic Messages | User-Agent: `OpenClaw/*` | Personal assistant |
| Codex CLI | OpenAI Chat | User-Agent: `codex*` | Command-line AI |
| Generic | OpenAI Chat | Path: `/v1/chat/completions` | Any OpenAI-compatible client |

### How Client Detection Works

The proxy automatically detects which client is making requests:

```
Request arrives
    |
    v
Check anthropic-version header? --> Yes --> Claude Code
    |
    No
    v
Check User-Agent contains "openclaw"? --> Yes --> OpenClaw
    |
    No
    v
Check User-Agent contains "codex"? --> Yes --> Codex CLI
    |
    No
    v
Check path /v1/chat/completions? --> Yes --> OpenAI Generic
    |
    No
    v
Default to Unknown (Anthropic format)
```

---

## Quick Start

### 1. Start the Proxy

Choose your configuration profile:

```bash
# Fastest (Groq-first, lowest latency)
op run --env-file=.env.fastest -- ./target/release/terraphim-llm-proxy -c config.fastest.toml

# Cheapest (DeepSeek-first, lowest cost)
op run --env-file=.env.cheapest -- ./target/release/terraphim-llm-proxy -c config.cheapest.toml

# Quality First (Claude-first, best output)
op run --env-file=.env.quality -- ./target/release/terraphim-llm-proxy -c config.quality-first.toml
```

### 2. Verify Health

```bash
curl http://127.0.0.1:3456/health
# {"status":"healthy","version":"0.1.5",...}
```

### 3. Test with curl

```bash
curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-sonnet-4-5",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

---

## Intelligent Pattern Routing

The proxy supports keyword-based intelligent routing using taxonomy patterns. When a client sends `model: "auto"`, the proxy analyzes message content and routes to the optimal model.

### How Pattern Matching Works

```
User message arrives with model="auto"
    |
    v
Extract text from message content
    |
    v
Match against 118 taxonomy patterns (Aho-Corasick)
    |
    v
Found "think", "plan", "step by step"? --> deepseek-reasoner
    |
    No
    v
Found "cheap", "budget", "economy"? --> deepseek-chat
    |
    No
    v
Found "fast", "urgent", "premium"? --> claude-sonnet-4.5
    |
    No
    v
Use default provider
```

### Supported Routing Patterns

| Pattern | Trigger Keywords | Routes To | Use Case |
|---------|-----------------|-----------|----------|
| think_routing | think, plan, reason, step by step, analyze, chain-of-thought | deepseek-reasoner | Complex reasoning |
| slow_&_cheap_routing | cheap, budget, slow, economy, affordable, cost-effective | deepseek-chat | Cost-sensitive tasks |
| fast_&_expensive_routing | fast, urgent, premium, realtime, speed, critical | claude-sonnet-4.5 | Time-critical tasks |
| web_search_routing | search, lookup, find online, current events | perplexity-sonar | Web search needed |
| long_context_routing | (auto-detected by token count > 60K) | gemini-flash | Large documents |

### Example: Pattern-Based Routing

```bash
# Request with "think" keyword -> routes to deepseek-reasoner
curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "auto",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "I need to think step by step about this architecture."}]
  }'
# Response comes from deepseek-reasoner

# Request with "cheap" keyword -> routes to deepseek-chat
curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "auto",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "I need a cheap budget solution for this."}]
  }'
# Response comes from deepseek-chat
```

### Explicit vs Auto Model Selection

| Model Parameter | Behavior |
|-----------------|----------|
| `model: "auto"` | Pattern matching on message content |
| `model: "claude-sonnet-4-5"` | Model alias applied, pattern matching bypassed |
| `model: "groq:llama-3.3-70b"` | Explicit provider:model, direct routing |

When a client specifies a model, the proxy respects that choice. Pattern matching only activates with `model: "auto"`.

### Adding Custom Patterns

Patterns are defined in taxonomy markdown files at `docs/taxonomy/routing_scenarios/`:

```markdown
# my_custom_routing.md

route:: provider_name, model_name

synonyms:: keyword1, keyword2, keyword3, phrase with spaces
```

After adding a file, restart the proxy to load the new patterns.

---

## Claude Code Integration

Claude Code is Anthropic's official CLI for Claude. The proxy seamlessly integrates as a drop-in replacement.

### Setup

```bash
# Set environment variables
export ANTHROPIC_BASE_URL=http://127.0.0.1:3456
export ANTHROPIC_API_KEY=your_proxy_api_key

# Use Claude Code normally
claude "Help me refactor this function"
```

### Model Mapping

Claude Code sends versioned model names that the proxy maps to your configured providers:

| Claude Code Sends | Proxy Routes To |
|-------------------|-----------------|
| `claude-opus-4-5-20251101` | Configured opus provider (e.g., OpenRouter) |
| `claude-sonnet-4-5-20251101` | Configured sonnet provider |
| `claude-haiku-4-5-20251101` | Configured haiku provider (or background) |

### Configuration Example

Add to your config.toml:

```toml
# Model mappings for Claude Code
[[router.model_mappings]]
from = "claude-opus-4-5-*"
to = "openrouter,anthropic/claude-opus-4.5"

[[router.model_mappings]]
from = "claude-sonnet-4-5-*"
to = "groq,llama-3.3-70b-versatile"  # Route to Groq for speed

[[router.model_mappings]]
from = "claude-haiku-4-5-*"
to = "ollama,qwen2.5-coder:latest"   # Route to local for background
```

### Verify Integration

```bash
# Start proxy
./target/release/terraphim-llm-proxy -c config.fastest.toml &

# Test with Claude Code
export ANTHROPIC_BASE_URL=http://127.0.0.1:3456
export ANTHROPIC_API_KEY=sk_test_proxy_key_for_claude_code_testing_12345

claude "What is 2+2?"
# Response should come from your configured provider
```

### Proxy Log Output

```
INFO terraphim_llm_proxy::client_detection: Detected client client_type=claude-code
INFO terraphim_llm_proxy::server: Model mapping applied from=claude-sonnet-4-5-20251101 to=groq,llama-3.3-70b-versatile
INFO terraphim_llm_proxy::server: Routing to provider=groq model=llama-3.3-70b-versatile
```

---

## OpenClaw Integration

OpenClaw is a personal AI assistant that supports custom model providers.

### Configuration File

Create or update `~/.openclaw/openclaw.json`:

```json
{
  "agents": {
    "defaults": {
      "workspace": "~/.openclaw/workspace",
      "model": {
        "primary": "terraphim-proxy/claude-sonnet-4-5"
      }
    }
  },
  "models": {
    "mode": "merge",
    "providers": {
      "terraphim-proxy": {
        "baseUrl": "http://127.0.0.1:3456",
        "apiKey": "your_proxy_api_key",
        "api": "anthropic-messages",
        "auth": "api-key",
        "authHeader": true,
        "models": [
          {
            "id": "claude-opus-4-5",
            "name": "Claude Opus 4.5 via Proxy",
            "reasoning": true,
            "input": ["text", "image"],
            "cost": { "input": 15, "output": 75, "cacheRead": 1.5, "cacheWrite": 18.75 },
            "contextWindow": 200000,
            "maxTokens": 32000
          },
          {
            "id": "claude-sonnet-4-5",
            "name": "Claude Sonnet 4.5 via Proxy",
            "reasoning": true,
            "input": ["text", "image"],
            "cost": { "input": 3, "output": 15, "cacheRead": 0.3, "cacheWrite": 3.75 },
            "contextWindow": 200000,
            "maxTokens": 8192
          },
          {
            "id": "claude-haiku-3-5",
            "name": "Claude Haiku 3.5 via Proxy",
            "reasoning": false,
            "input": ["text", "image"],
            "cost": { "input": 0.25, "output": 1.25, "cacheRead": 0.025, "cacheWrite": 0.3 },
            "contextWindow": 200000,
            "maxTokens": 8192
          }
        ]
      }
    }
  }
}
```

### Proxy Model Mappings

Add to your proxy config:

```toml
# OpenClaw model mappings
[[router.model_mappings]]
from = "claude-sonnet-4-5"
to = "openrouter,anthropic/claude-sonnet-4.5"

[[router.model_mappings]]
from = "claude-opus-4-5"
to = "openrouter,anthropic/claude-opus-4.5"

[[router.model_mappings]]
from = "claude-haiku-3-5"
to = "groq,llama-3.1-8b-instant"
```

### Test Integration

```bash
# Using OpenClaw's local agent mode
cd /path/to/openclaw
node openclaw.mjs agent --local --session-id test --message "Hello!"
```

### Expected Output

```json
{
  "payloads": [{ "text": "Hello! How can I help you today?" }],
  "meta": {
    "agentMeta": {
      "provider": "terraphim-proxy",
      "model": "claude-sonnet-4-5"
    }
  }
}
```

---

## Codex CLI Integration

Codex CLI uses the OpenAI API format. The proxy automatically handles format conversion.

### Setup

```bash
# Set environment variables
export OPENAI_API_BASE=http://127.0.0.1:3456/v1
export OPENAI_API_KEY=your_proxy_api_key

# Use Codex CLI normally
codex "Explain this code"
```

### API Endpoint

Codex CLI sends requests to `/v1/chat/completions`. The proxy:
1. Detects OpenAI format from the path
2. Converts to Anthropic format internally
3. Routes to configured provider
4. Converts response back to OpenAI format

### Configuration Example

```toml
# GPT model mappings for Codex CLI
[[router.model_mappings]]
from = "gpt-4*"
to = "deepseek,deepseek-chat"

[[router.model_mappings]]
from = "gpt-3.5*"
to = "groq,llama-3.1-8b-instant"

[[router.model_mappings]]
from = "o1*"
to = "deepseek,deepseek-reasoner"
```

### Test with curl (OpenAI format)

```bash
curl -X POST http://127.0.0.1:3456/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PROXY_API_KEY" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 100
  }'
```

---

## Direct Groq Integration

For ultra-low latency, you can configure the proxy to route all requests to Groq.

### Configuration

```toml
[router]
# Route everything to Groq
default = "groq,llama-3.3-70b-versatile"
background = "groq,llama-3.1-8b-instant"
think = "groq,llama-3.3-70b-versatile"

[[providers]]
name = "groq"
api_base_url = "https://api.groq.com/openai/v1"
api_key = "$GROQ_API_KEY"
models = ["llama-3.3-70b-versatile", "llama-3.1-8b-instant"]
transformers = ["openai"]
```

### Performance Expectations

| Metric | Groq | Claude | DeepSeek |
|--------|------|--------|----------|
| Tokens/sec | 100+ | 30-50 | 40-60 |
| TTFT | <100ms | 500-1000ms | 200-400ms |
| Quality | Good | Excellent | Very Good |
| Cost | $0.05/M | $3-15/M | $0.14/M |

### Test Latency

```bash
time curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "groq:llama-3.3-70b-versatile",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
# real    0m0.312s  (including network round-trip)
```

---

## Direct Cerebras Integration

Cerebras offers ultra-fast inference similar to Groq. The proxy includes a dedicated CerebrasClient for proper API integration.

### Configuration

```toml
[router]
# Route to Cerebras for speed
default = "cerebras,llama3.1-70b"
background = "cerebras,llama3.1-8b"

[[providers]]
name = "cerebras"
api_base_url = "https://api.cerebras.ai/v1"
api_key = "$CEREBRAS_API_KEY"
models = ["llama3.1-70b", "llama3.1-8b"]
transformers = ["openai"]
```

### Why a Dedicated Client?

Cerebras uses `/v1/chat/completions` (standard OpenAI path), while Groq uses `/openai/v1/chat/completions`. The proxy's CerebrasClient handles this correctly:

| Provider | URL Path | Client |
|----------|----------|--------|
| Groq | `/openai/v1/chat/completions` | GroqClient |
| Cerebras | `/v1/chat/completions` | CerebrasClient |
| OpenRouter | `/api/v1/chat/completions` | OpenRouterClient |

### Performance Comparison

| Metric | Cerebras | Groq | DeepSeek |
|--------|----------|------|----------|
| Tokens/sec | 100+ | 100+ | 40-60 |
| TTFT | <100ms | <100ms | 200-400ms |
| Quality | Good | Good | Very Good |
| Cost | $0.05/M | $0.05/M | $0.14/M |

### Test Cerebras Latency

```bash
time curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "cerebras,llama3.1-8b",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
# real    0m0.350s  (including network round-trip)
```

---

## Configuration Profiles

### Profile: Fastest (Latency Optimized)

Use `config.fastest.toml`:
- Primary: Groq (100+ tokens/sec)
- Fallback: DeepSeek, OpenRouter
- Local: Ollama for offline

Best for: Interactive coding, real-time applications

### Profile: Cheapest (Cost Optimized)

Use `config.cheapest.toml`:
- Primary: DeepSeek ($0.14/M tokens)
- Background: Ollama (FREE)
- Fallback: Groq

Best for: Batch processing, development, budget-conscious usage

### Profile: Quality First (Premium)

Use `config.quality-first.toml`:
- Primary: Claude Sonnet 4.5
- Think: Claude Opus 4.5
- Fallback: Groq for speed

Best for: Production code, complex reasoning, enterprise applications

---

## Troubleshooting

### Client Not Detected

**Symptom:** Requests default to Unknown client type

**Fix:** Check headers are being sent correctly:
```bash
# Claude Code should send
anthropic-version: 2023-06-01

# OpenClaw should send
User-Agent: OpenClaw/X.X

# Codex CLI should send
User-Agent: codex-cli/X.X
```

### Model Mapping Not Applied

**Symptom:** Original model name passed through

**Debug:**
```bash
RUST_LOG=debug ./target/release/terraphim-llm-proxy -c config.toml
# Look for: "Model mapping applied" or "No mapping found"
```

**Fix:** Check mapping patterns:
```toml
# Pattern must match exactly or with glob
[[router.model_mappings]]
from = "claude-sonnet-4-5-*"  # Glob pattern
to = "groq,llama-3.3-70b-versatile"
```

### Connection Refused

**Symptom:** `Connection refused` errors

**Fix:**
1. Check proxy is running: `curl http://127.0.0.1:3456/health`
2. Check port is correct in client config
3. Check firewall allows localhost connections

### Authentication Failed

**Symptom:** 401 Unauthorized

**Fix:**
1. Check API key matches `config.toml`:
   ```toml
   [proxy]
   api_key = "your_key_here"
   ```
2. Check client sends correct header:
   - Anthropic: `x-api-key: your_key`
   - OpenAI: `Authorization: Bearer your_key`

### Provider Errors

**Symptom:** 500 errors from provider

**Debug:**
```bash
# Check provider health
curl http://127.0.0.1:3456/health/detailed

# Check provider API key
RUST_LOG=debug cargo run -- -c config.toml
# Look for: "Provider error" messages
```

### Slow Responses

**Symptom:** High latency even with Groq

**Fix:**
1. Check you're routing to Groq, not Claude:
   ```bash
   # Look for in logs
   INFO terraphim_llm_proxy::server: Routing to provider=groq
   ```
2. Verify model mapping is working
3. Check network latency to provider

---

## Environment Variables

Create `.env` files for your secrets:

**.env.fastest:**
```bash
PROXY_API_KEY=your_proxy_key
GROQ_API_KEY=gsk_...
DEEPSEEK_API_KEY=sk-...
OPENROUTER_API_KEY=sk-or-...
```

**.env.cheapest:**
```bash
PROXY_API_KEY=your_proxy_key
DEEPSEEK_API_KEY=sk-...
GROQ_API_KEY=gsk_...
```

**.env.quality:**
```bash
PROXY_API_KEY=your_proxy_key
OPENROUTER_API_KEY=sk-or-...
ANTHROPIC_API_KEY=sk-ant-...
GROQ_API_KEY=gsk_...
DEEPSEEK_API_KEY=sk-...
```

---

## Next Steps

- [Routing Architecture](ROUTING_ARCHITECTURE.md) - Deep dive into 6-phase routing
- [Blog: Intelligent Routing](blog/intelligent-routing.md) - Live demonstrations
- [Quality Gate Report](../.docs/quality-gate-openclaw-compatibility.md) - OpenClaw compatibility verification

---

**Last Updated:** 2026-02-02
**Version:** 1.1
