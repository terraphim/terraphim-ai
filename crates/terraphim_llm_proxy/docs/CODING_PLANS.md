# Coding Plan Provider Integration

Use third-party "coding plan" subscriptions with Terraphim LLM Proxy. These providers offer Anthropic-compatible APIs at lower costs than direct Claude API access.

---

## Provider Overview

| Provider | Monthly Cost | Models | Status |
|----------|-------------|--------|--------|
| **Z.ai** | $3/month | GLM-4.7, GLM-4.5 | Verified Working |
| **MiniMax** | Pay-per-use | MiniMax-M2.5 | Rate Limited |
| **Kimi** | Subscription | kimi-1.5-pro | Membership Required |

---

## Z.ai GLM Coding Plan

**Best value**: $3/month for unlimited Claude Code usage.

### Setup

1. Sign up at [zhipu.ai](https://open.bigmodel.cn/) and subscribe to GLM Coding Plan
2. Get your API key from the console
3. Configure Claude Code:

**Direct usage** (without proxy):
```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.z.ai/api/anthropic",
    "ANTHROPIC_AUTH_TOKEN": "your-zai-api-key",
    "API_TIMEOUT_MS": "3000000"
  }
}
```

**Via Terraphim Proxy** (with routing):
```bash
export ZAI_API_KEY="your-zai-api-key"
./target/release/terraphim-llm-proxy -c config.coding-plans.toml
```

### Verified Working

```bash
curl -s https://api.z.ai/api/anthropic/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: YOUR_ZAI_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 50,
    "messages": [{"role": "user", "content": "Say hello"}]
  }'
```

Response includes model name like `glm-4.7` (Z.ai routes Claude model names to GLM internally).

---

## MiniMax M2.5 Coding Plan

**Good for complex tasks**: Pay-per-use with monthly usage limits.

### Setup

1. Sign up at [platform.minimax.io](https://platform.minimax.io/)
2. Subscribe to M2.5 Coding Plan
3. Get your API key (JWT token)

**Direct usage**:
```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.io/anthropic",
    "ANTHROPIC_AUTH_TOKEN": "eyJhbGciOiJSUzI1NiI...",
    "ANTHROPIC_MODEL": "MiniMax-M2.5"
  }
}
```

### Usage Limits

MiniMax has usage caps that reset monthly. Error when exceeded:
```json
{"type":"error","error":{"type":"rate_limit_error","message":"usage limit exceeded (2056)"}}
```

---

## Kimi Coding API

**Moonshot AI's coding-focused model**.

### Setup

1. Sign up at [kimi.moonshot.cn](https://kimi.moonshot.cn/)
2. Subscribe to membership (required for API access)
3. Generate API key

**Direct usage**:
```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.kimi.com/coding/",
    "ANTHROPIC_AUTH_TOKEN": "sk-kimi-xxx",
    "API_TIMEOUT_MS": "3000000"
  }
}
```

### Membership Required

Error when membership inactive:
```json
{"error":{"type":"invalid_request_error","message":"We're unable to verify your membership benefits at this time."}}
```

---

## Proxy Configuration

### config.coding-plans.toml

```toml
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "$PROXY_API_KEY"

[router]
default = "zai,glm-4.7"
background = "kimi,kimi-1.5-pro"
think = "minimax,MiniMax-M2.5"

# Z.ai uses OpenAI-compatible endpoint (NOT Anthropic endpoint)
# The proxy has a dedicated ZaiClient for proper URL handling
[[providers]]
name = "zai"
api_base_url = "https://api.z.ai/api/coding/paas/v4"
api_key = "$ZAI_API_KEY"
models = ["glm-4.7", "glm-4.5"]

[[providers]]
name = "minimax"
api_base_url = "https://api.minimax.io/anthropic"
api_key = "$MINIMAX_API_KEY"
models = ["MiniMax-M2.5", "MiniMax-M2.1"]

[[providers]]
name = "kimi"
api_base_url = "https://api.kimi.com/coding"
api_key = "$KIMI_API_KEY"
models = ["kimi-1.5-pro"]
```

**Important**: Z.ai requires the OpenAI-compatible endpoint (`/api/coding/paas/v4`), not the Anthropic endpoint (`/api/anthropic`). The proxy includes a dedicated `ZaiClient` that handles the correct URL construction.

### Environment Variables

```bash
export PROXY_API_KEY="sk-proxy-your-key"
export ZAI_API_KEY="your-zai-token"
export MINIMAX_API_KEY="your-minimax-jwt"
export KIMI_API_KEY="sk-kimi-xxx"
```

### Claude Code Settings

Create `~/.claude/settings_coding_plans.json`:

```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "http://127.0.0.1:3456",
    "ANTHROPIC_API_KEY": "sk-proxy-your-key",
    "API_TIMEOUT_MS": "3000000"
  }
}
```

Launch with:
```bash
claude --settings ~/.claude/settings_coding_plans.json
```

---

## Model Mapping

The proxy automatically maps Claude model names to coding plan models:

| Requested Model | Routed To |
|-----------------|-----------|
| `claude-3-5-sonnet-20241022` | `zai,glm-4.7` |
| `claude-sonnet-4-20250514` | `zai,glm-4.7` |
| `claude-3-5-haiku-*` | `kimi,kimi-1.5-pro` |
| `claude-3-opus-*` | `minimax,MiniMax-M2.5` |
| `auto` | `zai,glm-4.7` |

---

## Cost Comparison

| Approach | Monthly Cost | Quality |
|----------|-------------|---------|
| Claude Pro | $20/month | Best |
| Z.ai Coding | $3/month | Good (95%) |
| MiniMax | Pay-per-use | Good |
| Kimi | Subscription | Good |

**Savings**: 85% with Z.ai vs Claude Pro subscription.

---

## Risk Assessment

| Provider | Risk Level | Notes |
|----------|------------|-------|
| Z.ai | None | Standard API key authentication |
| MiniMax | Low | Usage limits apply |
| Kimi | Low | Membership verification required |

All providers use standard API key authentication. No OAuth token extraction or header spoofing required.

---

## Troubleshooting

### Z.ai: 401 Unauthorized

Verify API key:
```bash
echo $ZAI_API_KEY | head -c 20
```

### MiniMax: Rate Limit Exceeded

Wait for monthly reset or upgrade plan.

### Kimi: Membership Not Active

Renew membership at [kimi.moonshot.cn](https://kimi.moonshot.cn/).

### Proxy: Connection Refused

Check proxy is running:
```bash
curl -s http://127.0.0.1:3456/health | jq '.'
```

---

## Related Documentation

- [CLI Login Setup](CLI_LOGIN_SETUP.md) - Claude Code and Codex CLI configuration
- [Cost Savings Estimate](COST_SAVINGS_ESTIMATE.md) - Detailed cost analysis
- [Multi-Client Integration](MULTI_CLIENT_INTEGRATION.md) - Advanced routing setup
