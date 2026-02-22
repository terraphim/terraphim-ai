# Terraphim LLM Proxy - Feature Guide

**Version:** 3.0
**Last Updated:** 2026-01-12

A comprehensive guide to all features implemented in terraphim-llm-proxy, organized by functional area.

---

## Table of Contents

1. [Overview](#overview)
2. [Phase 1: OAuth Authentication](#phase-1-oauth-authentication)
3. [Phase 2: Management API](#phase-2-management-api)
4. [Phase 3: Advanced Routing](#phase-3-advanced-routing)
5. [Configuration Reference](#configuration-reference)
6. [Troubleshooting](#troubleshooting)

---

## Overview

Terraphim LLM Proxy is an intelligent API gateway for Large Language Models that provides:

- **Multi-provider routing** - Route requests to OpenRouter, DeepSeek, Anthropic, and more
- **Semantic routing** - Automatically select the best model based on task type
- **Model aliasing** - Map client model names to provider-specific models
- **OAuth authentication** - Authenticate with Claude, Gemini, and GitHub Copilot
- **Management API** - Runtime configuration without restarts
- **Webhooks** - Get notified of important events

### Quick Start

```bash
# Start the proxy
cargo run --release

# Test with Claude Code aliases
curl http://localhost:3456/v1/messages \
  -H "x-api-key: your-key" \
  -H "anthropic-version: 2023-06-01" \
  -d '{"model": "claude-opus-4-5-20251101", "max_tokens": 100, "messages": [{"role": "user", "content": "Hello"}]}'
```

---

## Phase 1: OAuth Authentication

### What It Does

OAuth authentication allows the proxy to authenticate with LLM providers using browser-based login flows instead of static API keys.

### Supported Providers

| Provider | Flow Type | Status |
|----------|-----------|--------|
| Claude (Anthropic) | PKCE with browser callback | Implemented |
| Gemini (Google) | OAuth2 with browser callback | Implemented |
| GitHub Copilot | Device code flow | Implemented |

### How to Use

#### Claude OAuth

1. Start the proxy with OAuth enabled:
   ```toml
   [oauth.claude]
   enabled = true
   callback_port = 9999
   ```

2. Navigate to the login URL (printed at startup)
3. Complete authentication in browser
4. The browser window auto-closes on success

#### GitHub Copilot (Device Flow)

1. Start authentication - the proxy displays a user code
2. Visit https://github.com/login/device
3. Enter the displayed code
4. The proxy polls for completion and stores the token

### Token Storage

Tokens are stored securely in:
- **File-based storage** (default): `~/.terraphim-llm-proxy/tokens/`
- **Redis storage** (optional): Configure with `[oauth.storage]`

Tokens persist across restarts and are automatically refreshed when expired.

### Recovery Behavior

OAuth recovery uses a **fresh start** approach:
- Partial authentication states are discarded
- If auth fails midway, restart the flow from the beginning
- This ensures clean state and prevents stuck flows

---

## Phase 2: Management API

### What It Does

The Management API provides runtime control over the proxy without requiring restarts.

### Authentication

All management endpoints require the `X-Management-Key` header:

```bash
curl -H "X-Management-Key: your-secret" \
  http://localhost:3456/v0/management/config
```

The management secret is separate from API keys and uses argon2 hashing for storage.

### Endpoints

#### Configuration

```http
GET  /v0/management/config          # Get current config (secrets redacted)
PUT  /v0/management/config          # Update configuration
POST /v0/management/config/reload   # Reload from disk
```

**Example: Get Configuration**
```bash
curl -H "X-Management-Key: secret" \
  http://localhost:3456/v0/management/config
```

Response shows configuration with secrets as `[REDACTED]`.

#### API Key Management

```http
GET    /v0/management/api-keys           # List all keys (prefixes only)
POST   /v0/management/api-keys           # Create new key
DELETE /v0/management/api-keys/{key_id}  # Revoke key (immediate)
```

Key deletion is **immediate** - revoked keys stop working instantly.

#### Logging Control

```http
GET /v0/management/logs/level           # Get current level
PUT /v0/management/logs/level           # Change level at runtime
GET /v0/management/logs?lines=100       # Fetch recent logs
```

Change log level without restart:
```bash
curl -X PUT -H "X-Management-Key: secret" \
  -d '{"level": "debug"}' \
  http://localhost:3456/v0/management/logs/level
```

#### Health & Metrics

```http
GET /v0/management/health    # Detailed provider health
GET /v0/management/metrics   # Usage statistics
```

### Hot Reload

Configuration changes take effect immediately for new requests. In-flight requests complete with the original configuration.

Both TOML and YAML configuration files are supported - format is auto-detected from file extension.

---

## Phase 3: Advanced Routing

### Model Aliasing (Model Mappings)

Map client model names to provider-specific models. Perfect for:
- Claude Code compatibility (maps internal model names)
- Creating friendly aliases like `"fast"` → DeepSeek
- Switching providers without client changes

#### Configuration

```toml
[[router.model_mappings]]
from = "claude-opus-4-5-*"
to = "openrouter,anthropic/claude-opus-4.5"
bidirectional = true

[[router.model_mappings]]
from = "claude-sonnet-4-5-*"
to = "openrouter,anthropic/claude-sonnet-4.5"
bidirectional = true

[[router.model_mappings]]
from = "fast"
to = "deepseek,deepseek-chat"
```

#### Features

- **Glob patterns**: Use `*` wildcards (`claude-*` matches `claude-3-opus`)
- **Case-insensitive**: `CLAUDE-*` matches `claude-3-opus`
- **First-match wins**: Order matters - first matching rule is used
- **Bidirectional**: Optionally map response model names back to aliases

#### Data Flow

```
Request: model="claude-opus-4-5-20251101"
    ↓
Model Mapping: matches "claude-opus-4-5-*" → "openrouter,anthropic/claude-opus-4.5"
    ↓
Router: Explicit provider routing to OpenRouter
    ↓
OpenRouter: Receives model="anthropic/claude-opus-4.5"
    ↓
Response: model="anthropic/claude-opus-4.5" (or original if bidirectional)
```

### Model Exclusion

Filter out unwanted models per provider using wildcard patterns.

```toml
[[router.model_exclusions]]
provider = "openrouter"
patterns = ["*-preview", "*-beta", "*-experimental"]

[[router.model_exclusions]]
provider = "deepseek"
patterns = ["*-test", "*-dev"]
```

### Routing Strategies

Choose how the proxy selects among multiple healthy providers.

| Strategy | Behavior |
|----------|----------|
| `FillFirst` (default) | Uses providers in config order until failure |
| `RoundRobin` | Distributes evenly across healthy providers |
| `LatencyOptimized` | Prefers providers with lowest latency |
| `CostOptimized` | Prefers providers with lowest cost |

```toml
[router]
strategy = "RoundRobin"
```

All strategies respect provider health status - unhealthy providers are skipped.

### Semantic Routing

The proxy can automatically select models based on the task type detected in the prompt.

```toml
[router]
# When prompt contains "think", "plan", "reason" → use this model
think = "openrouter,deepseek/deepseek-v3.1-terminus"

# When prompt contains "code", "implement" → use this model
code = "openrouter,anthropic/claude-sonnet-4.5"
```

**Priority**: Model mappings override semantic routing. If a model is explicitly mapped, the mapping wins.

---

## Webhooks

Get notified when important events occur.

### Configuration

```toml
[webhooks]
enabled = true
url = "https://your-server.com/hooks/llm-proxy"
secret = "your-webhook-secret"
events = ["oauth_refresh", "circuit_breaker", "config_updated"]
retry_count = 3
timeout_seconds = 5
```

### Event Types

| Event | Trigger |
|-------|---------|
| `oauth_refresh` | OAuth token refreshed |
| `circuit_breaker` | Provider circuit opened/closed |
| `quota_exceeded` | Provider quota limit hit |
| `config_updated` | Configuration changed |
| `api_key_revoked` | API key deleted |

### Security

Webhooks are signed with HMAC-SHA256. Verify the `X-Webhook-Signature` header:

```python
import hmac
import hashlib

def verify_webhook(payload, signature, secret):
    expected = hmac.new(
        secret.encode(),
        payload,
        hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(f"sha256={expected}", signature)
```

### Delivery

- Async delivery with exponential backoff
- 3 retry attempts on failure
- < 5 second delivery latency target

---

## Configuration Reference

### Complete Example

```toml
[proxy]
port = 3456
log_level = "info"

[oauth.claude]
enabled = true
callback_port = 9999

[webhooks]
enabled = true
url = "https://hooks.example.com/llm"
secret = "webhook-secret"
events = ["circuit_breaker", "config_updated"]

[router]
strategy = "FillFirst"
think = "openrouter,deepseek/deepseek-v3.1-terminus"
code = "openrouter,anthropic/claude-sonnet-4.5"

[[router.model_mappings]]
from = "claude-opus-4-5-*"
to = "openrouter,anthropic/claude-opus-4.5"
bidirectional = true

[[router.model_mappings]]
from = "claude-sonnet-4-5-*"
to = "openrouter,anthropic/claude-sonnet-4.5"
bidirectional = true

[[router.model_exclusions]]
provider = "openrouter"
patterns = ["*-preview", "*-beta"]

[[providers]]
name = "openrouter"
base_url = "https://openrouter.ai/api/v1"
api_key = "$OPENROUTER_API_KEY"
models = ["anthropic/claude-opus-4.5", "anthropic/claude-sonnet-4.5"]

[[providers]]
name = "deepseek"
base_url = "https://api.deepseek.com/v1"
api_key = "$DEEPSEEK_API_KEY"
models = ["deepseek-chat", "deepseek-coder"]

# Fast inference providers (100+ tokens/sec)
[[providers]]
name = "groq"
api_base_url = "https://api.groq.com/openai/v1"
api_key = "$GROQ_API_KEY"
models = ["llama-3.3-70b-versatile", "llama-3.1-8b-instant"]

[[providers]]
name = "cerebras"
api_base_url = "https://api.cerebras.ai/v1"
api_key = "$CEREBRAS_API_KEY"
models = ["llama3.1-8b", "llama3.1-70b"]
```

### Dedicated Provider Clients

The proxy uses dedicated HTTP clients for optimal compatibility with certain providers:

| Provider | Client | URL Path | Notes |
|----------|--------|----------|-------|
| Groq | GroqClient | `/openai/v1/chat/completions` | OpenAI-compatible with custom path |
| Cerebras | CerebrasClient | `/v1/chat/completions` | Standard OpenAI path without `/openai/` prefix |
| OpenRouter | OpenRouterClient | `/api/v1/chat/completions` | Requires HTTP-Referer header |
| Others | genai client | Varies | Generic OpenAI-compatible client |

---

## Troubleshooting

### Model Mapping Not Working

1. Check mapping is in `[[router.model_mappings]]` section
2. Verify pattern matches (case-insensitive, glob patterns)
3. Check logs for mapping resolution messages
4. Ensure target model is in provider's `models` list

### OAuth Flow Stuck

1. OAuth uses fresh-start recovery - restart the flow
2. Check callback port is accessible
3. Verify browser can reach `http://localhost:{port}/oauth/callback`

### Provider Not Receiving Requests

1. Check provider health: `GET /v0/management/health`
2. Verify model is not excluded by `model_exclusions`
3. Check routing strategy isn't skipping the provider
4. Review circuit breaker status

### Webhook Not Firing

1. Verify `webhooks.enabled = true`
2. Check event type is in `events` list
3. Test webhook URL is reachable
4. Check logs for delivery errors

---

## Related Documentation

- [Routing Architecture](ROUTING_ARCHITECTURE.md) - Detailed routing internals
- [Priority Routing Spec](priority_routing_test_specification.md) - Priority routing details
- [Integration Design](integration_design.md) - System integration details
