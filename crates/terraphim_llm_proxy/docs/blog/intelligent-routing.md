# Intelligent LLM Routing: Route to Fastest with Automatic Fallback

**Author:** Terraphim Team
**Date:** 2026-02-01
**Tags:** LLM, Proxy, Routing, Performance, Cost Optimization

---

## The Problem

Managing multiple LLM providers is complex:

- **Different APIs**: Anthropic, OpenAI, Groq, DeepSeek all have different formats
- **Different strengths**: Groq is fast, Claude is accurate, DeepSeek is cheap
- **Fallback handling**: What happens when your primary provider is down?
- **Cost management**: Premium models for important tasks, cheap models for background work
- **Client compatibility**: Claude Code, OpenClaw, Codex CLI all expect different APIs

Most teams solve this with spaghetti code:

```python
# The nightmare scenario
if request.needs_vision:
    provider = claude
elif request.needs_speed:
    provider = groq
elif budget_remaining < threshold:
    provider = deepseek
elif groq.is_down():
    provider = deepseek
elif deepseek.is_down():
    provider = claude
# ... more conditions until your eyes bleed
```

There has to be a better way.

---

## The Solution: 6-Phase Intelligent Routing

Terraphim LLM Proxy implements a 6-phase routing system that makes intelligent decisions with sub-millisecond overhead:

```
Request arrives
     |
     v
+------------------+
| Phase 1: Explicit |  "groq,llama-3.3-70b" in request?
+------------------+  Direct to specified provider
     |
     v
+------------------+
| Phase 2: Pattern |  Model patterns: "claude-*" -> OpenRouter
+------------------+  RoleGraph taxonomy matching
     |
     v
+------------------+
| Phase 3: Session |  Conversation context? Same model
+------------------+  Planning session? Planning model
     |
     v
+------------------+
| Phase 4: Cost    |  Budget constraints? Cheaper model
+------------------+  Context too large? Long-context model
     |
     v
+------------------+
| Phase 5: Perf    |  Latency requirements? Fastest model
+------------------+  Token generation speed
     |
     v
+------------------+
| Phase 6: Hints   |  Scenario-based selection
+------------------+  background/think/image routing
```

**Key features:**
- **0.21ms overhead** - Routing decisions are nearly instant
- **Automatic fallback** - Provider failures trigger next in chain
- **Multi-client support** - One proxy serves all your AI tools
- **Pattern matching** - Route by model name patterns
- **Scenario hints** - Different models for different use cases

---

## Live Demo: Fastest Model Routing

Let's see intelligent routing in action. We'll configure the proxy to prioritize speed:

### Configuration (config.fastest.toml)

```toml
[router]
# Primary: Groq for ultra-low latency (100+ tokens/sec)
default = "groq,llama-3.3-70b-versatile"

# Background: Local Ollama for batch tasks
background = "ollama,qwen2.5-coder:latest"

# Think: DeepSeek Reasoner for complex reasoning
think = "deepseek,deepseek-reasoner"

# Model mappings for Claude Code compatibility
[[router.model_mappings]]
from = "claude-sonnet-4-5-*"
to = "groq,llama-3.3-70b-versatile"

[[router.model_mappings]]
from = "claude-opus-4-5-*"
to = "groq,llama-3.3-70b-versatile"
```

### Start the Proxy

```bash
# Using 1Password for secrets
op run --env-file=.env.fastest -- ./target/release/terraphim-llm-proxy -c config.fastest.toml
```

### Test Speed

```bash
# Time a request
time curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-sonnet-4-5-20251101",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# Result: routed to Groq, ~300ms total (including network)
```

### Proxy Logs Show Routing

```
INFO terraphim_llm_proxy::client_detection: Detected client client_type=claude-code
INFO terraphim_llm_proxy::server: Model mapping applied from=claude-sonnet-4-5-20251101 to=groq,llama-3.3-70b-versatile
INFO terraphim_llm_proxy::server: Routing to provider=groq model=llama-3.3-70b-versatile
INFO terraphim_llm_proxy::server: Request completed latency_ms=287
```

---

## Configuration Examples

### Fastest (Latency Optimized)

Use `config.fastest.toml` when speed is critical:

| Scenario | Provider | Model | Why |
|----------|----------|-------|-----|
| Default | Groq | llama-3.3-70b | 100+ tokens/sec |
| Background | Ollama | qwen2.5-coder | Local, no network latency |
| Think | DeepSeek | deepseek-reasoner | Fast + quality reasoning |
| Image | OpenRouter | claude-sonnet-4.5 | Best multimodal |

**Best for:** Interactive coding, real-time applications, impatient developers

### Cheapest (Cost Optimized)

Use `config.cheapest.toml` when budget matters:

| Scenario | Provider | Model | Cost |
|----------|----------|-------|------|
| Default | DeepSeek | deepseek-chat | $0.14/M input |
| Background | Ollama | qwen2.5-coder | FREE |
| Think | DeepSeek | deepseek-reasoner | $0.55/M input |
| Image | OpenRouter | gemini-flash | Cheap multimodal |

**Best for:** Development, batch processing, budget-conscious teams

### Quality First (Premium)

Use `config.quality-first.toml` when output quality matters most:

| Scenario | Provider | Model | Why |
|----------|----------|-------|-----|
| Default | OpenRouter | claude-sonnet-4.5 | Best code quality |
| Background | Groq | llama-3.3-70b | Good quality, fast |
| Think | OpenRouter | claude-opus-4.5 | Maximum reasoning |
| Image | OpenRouter | claude-sonnet-4.5 | Best multimodal |

**Best for:** Production code, complex reasoning, enterprise applications

---

## Multi-Client Support

One proxy serves all your AI tools. The proxy automatically detects which client is connecting:

### Claude Code

```bash
export ANTHROPIC_BASE_URL=http://127.0.0.1:3456
export ANTHROPIC_API_KEY=your_proxy_key

claude "Explain this function"
# Routes to your configured provider
```

### OpenClaw

Configure `~/.openclaw/openclaw.json`:

```json
{
  "models": {
    "providers": {
      "terraphim-proxy": {
        "baseUrl": "http://127.0.0.1:3456",
        "api": "anthropic-messages",
        "apiKey": "your_proxy_key"
      }
    }
  }
}
```

### Codex CLI

```bash
export OPENAI_API_BASE=http://127.0.0.1:3456/v1
export OPENAI_API_KEY=your_proxy_key

codex "What does this do?"
# OpenAI format automatically converted
```

### Direct API Access

```bash
# Anthropic format
curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "anthropic-version: 2023-06-01" \
  -H "x-api-key: $KEY" \
  -d '{"model": "claude-sonnet-4-5", "messages": [...]}'

# OpenAI format
curl -X POST http://127.0.0.1:3456/v1/chat/completions \
  -H "Authorization: Bearer $KEY" \
  -d '{"model": "gpt-4", "messages": [...]}'
```

---

## Performance Results

We benchmark the proxy regularly. Here are real numbers:

### Routing Overhead

| Metric | Value |
|--------|-------|
| Pattern match latency | 0.21ms |
| Token counting speed | 2.8M tokens/sec |
| Request throughput | >4000 req/sec |
| Memory per connection | ~2KB |

### Provider Comparison

| Provider | Tokens/sec | TTFT | Quality | Cost/M tokens |
|----------|------------|------|---------|---------------|
| Groq | 100+ | <100ms | Good | $0.05 |
| Cerebras | 100+ | <100ms | Good | $0.05 |
| DeepSeek | 40-60 | 200-400ms | Very Good | $0.14 |
| Claude | 30-50 | 500-1000ms | Excellent | $3-15 |
| Ollama (local) | 20-40 | <50ms | Good | FREE |

### Build-Time Model Validation

The proxy fetches available models at build time from provider APIs, ensuring:
- **Groq**: 20+ models fetched from `api.groq.com/openai/v1/models`
- **Cerebras**: Models fetched from `api.cerebras.ai/v1/models`
- Model validation with warnings for unknown models
- Fallback lists for offline builds (CI/CD environments without API keys)

### Dedicated Provider Clients

For optimal compatibility, the proxy uses dedicated HTTP clients for certain providers:
- **GroqClient**: Handles Groq's `/openai/v1/chat/completions` endpoint
- **CerebrasClient**: Handles Cerebras's `/v1/chat/completions` endpoint (different from OpenAI path)
- **OpenRouterClient**: Handles OpenRouter's API with required headers

This ensures correct URL construction and provider-specific optimizations.

### Fallback Performance

When primary provider fails:
- Detection time: <100ms
- Fallback switch: <50ms
- No requests lost with SSE streaming

---

## How Fallback Works

The proxy maintains a provider health map:

```
Provider A: Healthy (last success: 50ms ago)
Provider B: Healthy (last success: 200ms ago)
Provider C: Degraded (2 failures in last minute)
Provider D: Down (5 consecutive failures)
```

When a request arrives:
1. Try primary provider
2. If 5xx error or timeout, mark degraded
3. Retry with next provider in chain
4. Successful response clears degraded status

Example fallback chain:
```
Request -> Groq (timeout) -> DeepSeek (success) -> Response
           |                 |
           Mark degraded     Clear Groq degraded after 60s
```

---

## Scenario-Based Routing

Different tasks need different models. Use scenario hints:

### Background Tasks

```toml
background = "ollama,qwen2.5-coder:latest"
```

Low priority, cost-sensitive tasks route to local inference. Perfect for:
- Code linting
- Documentation generation
- Batch processing

### Think/Reasoning

```toml
think = "deepseek,deepseek-reasoner"
```

Complex reasoning tasks get dedicated models:
- Multi-step planning
- Code architecture
- Debugging analysis

### Image/Multimodal

```toml
image = "openrouter,anthropic/claude-sonnet-4.5"
```

Vision tasks route to models with multimodal capability:
- Screenshot analysis
- Diagram understanding
- UI debugging

### Long Context

```toml
long_context = "openrouter,google/gemini-2.5-flash-preview-09-2025"
long_context_threshold = 60000
```

Large codebases automatically route to models with extended context:
- Full repository analysis
- Large file processing
- Cross-file refactoring

---

## Getting Started

### 1. Clone and Build

```bash
git clone https://github.com/terraphim/terraphim-llm-proxy
cd terraphim-llm-proxy
cargo build --release
```

### 2. Configure Secrets

Create `.env.fastest`:
```bash
PROXY_API_KEY=your_proxy_key
GROQ_API_KEY=gsk_...
DEEPSEEK_API_KEY=sk-...
OPENROUTER_API_KEY=sk-or-...
```

### 3. Start Proxy

```bash
op run --env-file=.env.fastest -- ./target/release/terraphim-llm-proxy -c config.fastest.toml
```

### 4. Configure Your Client

```bash
export ANTHROPIC_BASE_URL=http://127.0.0.1:3456
export ANTHROPIC_API_KEY=your_proxy_key
```

### 5. Use Normally

```bash
claude "Hello, world!"
# Routed through proxy to fastest available provider
```

---

## Conclusion

Intelligent LLM routing transforms how you work with AI:

- **Stop managing providers manually** - Let the proxy handle it
- **Optimize for your priority** - Speed, cost, or quality
- **One config for all clients** - Claude Code, OpenClaw, Codex CLI, and more
- **Never worry about fallback** - Automatic provider switching
- **Sub-millisecond overhead** - Routing is essentially free

Try it today:

- [GitHub Repository](https://github.com/terraphim/terraphim-llm-proxy)
- [Keyword-Based Routing Demo](keyword-routing-demo.md) - Deep dive into pattern matching
- [Multi-Client Integration Guide](../MULTI_CLIENT_INTEGRATION.md)
- [Routing Architecture](../ROUTING_ARCHITECTURE.md)

---

**Questions?** Open an issue on GitHub or check the troubleshooting section in the integration guide.
