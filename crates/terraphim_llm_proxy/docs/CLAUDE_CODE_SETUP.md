# Claude Code Setup Guide - Terraphim LLM Proxy

This guide explains how to configure Claude Code to use the Terraphim LLM Proxy for intelligent multi-provider routing.

---

## Prerequisites

1. **Terraphim LLM Proxy Running**
   ```bash
   cd terraphim-llm-proxy
   cargo run --release
   ```
   Default: `http://127.0.0.1:3456`

2. **Provider API Keys Configured**
   - Set environment variables for your chosen providers
   - See [Configuration](#configuration) section below

---

## Quick Start

### 1. Configure Claude Code

Edit your Claude Code settings to point to the proxy:

**Location:** Claude Code settings file
```json
{
  "api_base_url": "http://127.0.0.1:3456",
  "api_key": "your-proxy-api-key-from-config-toml"
}
```

### 2. Start the Proxy

```bash
# Set your proxy API key
export PROXY_API_KEY="sk_your_generated_proxy_api_key_32_characters"

# Set provider API keys
export DEEPSEEK_API_KEY="op://TerraphimPlatform/TruthForge.api-keys/deepseek-api-keys"
export OPENROUTER_API_KEY="sk_your_openrouter_api_key"

# Start proxy
./target/release/terraphim-llm-proxy
```

### 3. Test the Connection

```bash
# Health check
curl http://localhost:3456/health
# Expected: OK

# Token counting test
curl -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
# Expected: {"input_tokens":9}
```

---

## Configuration

### Minimal Configuration

**File:** `config.toml`
```toml
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "$PROXY_API_KEY"

[router]
default = "deepseek,deepseek-chat"

[[providers]]
name = "deepseek"
api_base_url = "https://api.deepseek.com/chat/completions"
api_key = "$DEEPSEEK_API_KEY"
models = ["deepseek-chat"]
transformers = ["deepseek"]
```

### Recommended Configuration (Multi-Provider)

**File:** `config.toml`
```toml
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "$PROXY_API_KEY"
timeout_ms = 600000  # 10 minutes

[router]
# Default: Cost-effective DeepSeek for general requests
default = "deepseek,deepseek-chat"

# Background: Free local Ollama for background tasks
background = "ollama,qwen2.5-coder:latest"

# Thinking: DeepSeek Reasoner for complex reasoning
think = "deepseek,deepseek-reasoner"

# Long Context: Gemini 2.0 Flash for >60K token requests
long_context = "openrouter,google/gemini-2.0-flash-exp"
long_context_threshold = 60000

# Web Search: Perplexity for search-enabled requests
web_search = "openrouter,perplexity/llama-3.1-sonar-large-128k-online"

# Image: Claude 3.5 Sonnet for vision requests
image = "openrouter,anthropic/claude-3.5-sonnet"

[security.rate_limiting]
enabled = true
requests_per_minute = 60
concurrent_requests = 10

[security.ssrf_protection]
enabled = true
allow_localhost = false
allow_private_ips = false

[[providers]]
name = "deepseek"
api_base_url = "https://api.deepseek.com/chat/completions"
api_key = "$DEEPSEEK_API_KEY"
models = ["deepseek-chat", "deepseek-reasoner"]
transformers = ["deepseek"]

[[providers]]
name = "ollama"
api_base_url = "http://localhost:11434/v1/chat/completions"
api_key = "ollama"
models = ["qwen2.5-coder:latest", "llama3.2:3b"]
transformers = ["ollama"]

[[providers]]
name = "openrouter"
api_base_url = "https://openrouter.ai/api/v1/chat/completions"
api_key = "$OPENROUTER_API_KEY"
models = [
    "google/gemini-2.0-flash-exp",
    "anthropic/claude-3.5-sonnet",
    "perplexity/llama-3.1-sonar-large-128k-online"
]
transformers = ["openrouter"]
```

---

## Environment Variables

### Required

```bash
# Proxy authentication
export PROXY_API_KEY="sk_your_proxy_api_key_here"
```

### Provider API Keys (Configure Based on Usage)

```bash
# DeepSeek (recommended for cost-effective default)
export DEEPSEEK_API_KEY="op://TerraphimPlatform/TruthForge.api-keys/deepseek-api-keys"

# OpenRouter (for Gemini, Claude, Perplexity access)
export OPENROUTER_API_KEY="sk_your_openrouter_api_key"

# Anthropic (if using direct Anthropic provider)
export ANTHROPIC_API_KEY="sk_your_anthropic_api_key"

# OpenAI (if using direct OpenAI provider)
export OPENAI_API_KEY="sk_your_openai_api_key"
```

### Optional

```bash
# Ollama base URL (if not default)
export OLLAMA_BASE_URL="http://localhost:11434"

# Custom proxy port
export PROXY_PORT=3456

# Logging level
export RUST_LOG=info  # or debug, trace
```

---

## How Routing Works

The proxy intelligently routes requests based on characteristics:

### 1. Default Routing
```
Any regular request → default provider
```
**Example:** Standard chat message → DeepSeek Chat

### 2. Background Task Routing
```
Model contains "haiku" → background provider
```
**Example:** `claude-3-5-haiku-20241022` → Ollama (free, local)

**Why:** Background tasks don't need premium models

### 3. Thinking Mode Routing
```
Request has thinking field → think provider
```
**Example:** `{thinking: {enabled: true}}` → DeepSeek Reasoner

**Why:** Reasoning tasks need specialized models

### 4. Long Context Routing
```
Token count ≥ 60,000 → long_context provider
```
**Example:** Large codebase analysis → Gemini 2.0 Flash (2M context)

**Why:** Not all models handle large contexts efficiently

### 5. Web Search Routing
```
Tools include web_search → web_search provider
```
**Example:** `{tools: [{name: "web_search"}]}` → Perplexity Sonar

**Why:** Search-enabled models provide better results

### 6. Image Routing
```
Messages contain image blocks → image provider
```
**Example:** `{content: [{type: "image", source: ...}]}` → Claude 3.5 Sonnet

**Why:** Vision models needed for image understanding

---

## Testing Your Setup

### 1. Basic Connectivity

```bash
# Health check
curl http://localhost:3456/health
# Expected: OK
```

### 2. Authentication

```bash
# Test with valid key
curl -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Hi"}]}'
# Expected: {"input_tokens":6}

# Test with invalid key (should fail)
curl -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: wrong_key" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Hi"}]}'
# Expected: 401 Unauthorized
```

### 3. Default Routing

```bash
curl -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": false
  }'
# Expected: JSON response from configured default provider
```

### 4. Background Routing

```bash
curl -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "messages": [{"role": "user", "content": "Quick task"}],
    "stream": false
  }'
# Expected: Routed to background provider (e.g., Ollama)
```

### 5. Streaming Test

```bash
curl -N -X POST http://localhost:3456/v1/messages \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Count to 5"}],
    "stream": true
  }'
# Expected: SSE event stream with real-time response
```

---

## Routing Scenarios Explained

### Scenario Priority

When multiple routing conditions match, priority order is:

1. **Image** (highest) - Has image content
2. **WebSearch** - Has web_search tool
3. **LongContext** - Token count ≥ threshold
4. **Think** - Has thinking field
5. **Background** - Model contains "haiku"
6. **Default** (lowest) - Fallback for everything else

**Example:** Request with both images and web_search tool → Routes to Image provider

### Cost Optimization Examples

**Scenario 1: Background Tasks**
```toml
# Route background tasks to free local Ollama
background = "ollama,qwen2.5-coder:latest"
```
**Savings:** ~$0.15 per 1K tokens vs DeepSeek, ~$1.50 vs Claude

**Scenario 2: Long Context**
```toml
# Use Gemini for large contexts (2M tokens, cheap)
long_context = "openrouter,google/gemini-2.0-flash-exp"
```
**Benefit:** Handles large codebases that would exceed other models' limits

**Scenario 3: Reasoning Tasks**
```toml
# Use DeepSeek Reasoner for complex logic
think = "deepseek,deepseek-reasoner"
```
**Benefit:** Better reasoning quality vs general chat models

---

## Troubleshooting

### Proxy Won't Start

**Problem:** Configuration error on startup

**Solutions:**
```bash
# Check configuration syntax
cargo run -- --config config.toml

# Verify environment variables are set
echo $PROXY_API_KEY
echo $DEEPSEEK_API_KEY

# Check logs
RUST_LOG=debug cargo run
```

### Authentication Fails

**Problem:** Claude Code reports "Unauthorized"

**Solutions:**
1. Verify PROXY_API_KEY matches config.toml
2. Check API key is at least 32 characters
3. Ensure no extra whitespace in API key
4. Try Bearer token format: `Authorization: Bearer $PROXY_API_KEY`

### Provider Errors

**Problem:** "Provider error: deepseek - ..."

**Solutions:**
1. Verify provider API key is set: `echo $DEEPSEEK_API_KEY`
2. Check provider API is accessible: `curl https://api.deepseek.com`
3. Verify model name is correct in config
4. Check provider API key permissions

### No Routing to Expected Provider

**Problem:** Request routes to wrong provider

**Solutions:**
1. Check routing hints in logs: `RUST_LOG=debug cargo run`
2. Verify routing configuration matches your intent
3. Check scenario priority (Image > WebSearch > LongContext...)
4. Ensure provider exists in configuration

### Ollama Not Working

**Problem:** Background routing fails with Ollama

**Solutions:**
```bash
# Start Ollama if not running
ollama serve

# Verify Ollama is accessible
curl http://localhost:11434/api/tags

# Pull required model
ollama pull qwen2.5-coder:latest

# Test Ollama directly
curl -X POST http://localhost:11434/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "qwen2.5-coder:latest",
    "messages": [{"role": "user", "content": "Hi"}]
  }'
```

---

## Advanced Configuration

### Custom Provider

```toml
[[providers]]
name = "custom"
api_base_url = "https://api.custom-llm.com/v1/chat/completions"
api_key = "$CUSTOM_API_KEY"
models = ["custom-model-name"]
transformers = ["openai"]  # Use openai transformer for OpenAI-compatible APIs
```

### Multiple Models Per Provider

```toml
[[providers]]
name = "deepseek"
api_base_url = "https://api.deepseek.com/chat/completions"
api_key = "$DEEPSEEK_API_KEY"
models = [
    "deepseek-chat",
    "deepseek-reasoner",
    "deepseek-coder"
]
transformers = ["deepseek"]
```

Then route different scenarios to different models:
```toml
[router]
default = "deepseek,deepseek-chat"
think = "deepseek,deepseek-reasoner"
```

### Security Settings

```toml
[security.rate_limiting]
enabled = true
requests_per_minute = 100  # Increase for higher throughput
concurrent_requests = 20   # Allow more concurrent requests

[security.ssrf_protection]
enabled = true
allow_localhost = false    # Set true only for development
allow_private_ips = false  # Set true only for internal networks
```

---

## Logging and Monitoring

### Enable Debug Logging

```bash
# Detailed request/response logging
RUST_LOG=debug ./target/release/terraphim-llm-proxy

# Trace-level logging (very verbose)
RUST_LOG=trace ./target/release/terraphim-llm-proxy

# JSON logging for production
./target/release/terraphim-llm-proxy --log-json
```

### Log Levels

- **ERROR**: Critical failures, proxy won't work
- **WARN**: Routing fallbacks, invalid inputs, provider errors
- **INFO**: Request processing, routing decisions, token counts
- **DEBUG**: Detailed request/response data, transformer application
- **TRACE**: Every operation, all data structures

### What to Monitor

**Key Metrics:**
- Request count per routing scenario
- Provider errors and timeouts
- Token usage per provider
- Response latency (target: <100ms)
- Authentication failures

**Example Log Messages:**
```
INFO Request analyzed: token_count=150, is_background=false, has_thinking=false
INFO Routing decision: provider=deepseek, model=deepseek-chat, scenario=Default
DEBUG Applied transformer chain: transformers=1
INFO Request completed successfully: input_tokens=150, output_tokens=45
```

---

## Performance Tuning

### Reduce Latency

1. **Use Local Ollama for Background Tasks**
   ```toml
   background = "ollama,qwen2.5-coder:latest"
   ```
   Typical latency: 50-200ms (vs 500-1000ms for API providers)

2. **Optimize Transformer Chains**
   ```toml
   # Minimal transformers for Anthropic (pass-through)
   transformers = ["anthropic"]

   # vs multiple transformers
   transformers = ["deepseek", "tooluse", "maxtoken"]
   ```

3. **Adjust Timeouts**
   ```toml
   timeout_ms = 300000  # 5 minutes for faster failure
   ```

### Increase Throughput

1. **Increase Concurrent Requests**
   ```toml
   [security.rate_limiting]
   concurrent_requests = 50
   ```

2. **Use Faster Models**
   ```toml
   default = "deepseek,deepseek-chat"  # Fast and cheap
   ```

### Reduce Costs

1. **Route Background to Ollama**
   ```toml
   background = "ollama,qwen2.5-coder:latest"  # Free!
   ```

2. **Use DeepSeek for Default**
   ```toml
   default = "deepseek,deepseek-chat"  # ~20x cheaper than Claude
   ```

3. **Reserve Premium Models**
   ```toml
   image = "openrouter,anthropic/claude-3.5-sonnet"  # Only for vision
   think = "deepseek,deepseek-reasoner"  # Only for complex reasoning
   ```

---

## Example Configurations for Different Use Cases

### 1. Cost-Optimized Setup (Minimal Spending)

```toml
[router]
default = "ollama,qwen2.5-coder:latest"  # Free local
background = "ollama,qwen2.5-coder:latest"
think = "ollama,llama3.2:3b"  # Free local reasoning
long_context = "ollama,llama3.2:3b"
# No web_search or image (requires paid APIs)

[[providers]]
name = "ollama"
api_base_url = "http://localhost:11434/v1/chat/completions"
api_key = "ollama"
models = ["qwen2.5-coder:latest", "llama3.2:3b"]
transformers = ["ollama"]
```

**Cost:** $0/month (all local)

### 2. Balanced Setup (Cost + Quality)

```toml
[router]
default = "deepseek,deepseek-chat"  # Cheap API
background = "ollama,qwen2.5-coder:latest"  # Free local
think = "deepseek,deepseek-reasoner"  # Good reasoning
long_context = "openrouter,google/gemini-2.0-flash-exp"  # Large context
# Premium features when needed
web_search = "openrouter,perplexity/llama-3.1-sonar-large-128k-online"
image = "openrouter,anthropic/claude-3.5-sonnet"
```

**Cost:** ~$10-50/month depending on usage

### 3. Premium Setup (Maximum Quality)

```toml
[router]
default = "anthropic,claude-3-5-sonnet-20241022"  # Best quality
background = "anthropic,claude-3-5-haiku-20241022"  # Fast background
think = "anthropic,claude-3-5-sonnet-20241022"
long_context = "anthropic,claude-3-5-sonnet-20241022"
web_search = "openrouter,perplexity/llama-3.1-sonar-large-128k-online"
image = "anthropic,claude-3-5-sonnet-20241022"

[[providers]]
name = "anthropic"
api_base_url = "https://api.anthropic.com/v1/messages"
api_key = "$ANTHROPIC_API_KEY"
models = ["claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022"]
transformers = ["anthropic"]
```

**Cost:** ~$100-500/month depending on usage

---

## FAQ

### Q: Can I use the proxy with multiple Claude Code instances?

**A:** Yes! Each Claude Code instance needs the same PROXY_API_KEY. The proxy handles concurrent requests.

### Q: How do I know which provider is being used?

**A:** Enable INFO logging to see routing decisions:
```bash
RUST_LOG=info ./target/release/terraphim-llm-proxy
```
Look for: `INFO Routing decision: provider=deepseek, model=deepseek-chat, scenario=Default`

### Q: Can I override routing for specific requests?

**A:** Currently routing is automatic based on request characteristics. Manual override support coming in Phase 2.

### Q: What happens if a provider is down?

**A:** The proxy returns a 502 Bad Gateway error with details. Configure fallback providers in Phase 2 for automatic failover.

### Q: How accurate is token counting?

**A:** 95%+ accuracy compared to Claude's official token counting. Uses the same tiktoken-rs library with cl100k_base encoding.

### Q: Does the proxy log my requests?

**A:** Only metadata is logged (model, token count, routing decision). Request content is never logged by default. Use DEBUG level to see request summaries (truncated).

---

## Next Steps

1. **Start the proxy** with your configuration
2. **Configure Claude Code** to use proxy URL
3. **Test routing** with different request types
4. **Monitor logs** to verify routing decisions
5. **Optimize configuration** based on your usage patterns

---

## Support

For issues or questions:
- Check logs with `RUST_LOG=debug`
- Review [SECURITY.md](../SECURITY.md) for security configuration
- See [THREAT_MODEL.md](../THREAT_MODEL.md) for security considerations
- See [README.md](README.md) for development setup

---

**Status:** Production-ready for internal testing | E2E validation with Claude Code in progress
