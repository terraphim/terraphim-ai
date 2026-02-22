# Terraphim LLM Proxy - Announcement Materials

Launch announcements for terraphim-llm-proxy sponsor-only release.

---

## Blog Post

### Title: Introducing Terraphim LLM Proxy: Intelligent Routing for AI Coding Assistants

**TL;DR**: Route your Claude Code, OpenClaw, and Codex CLI requests through a single intelligent proxy. Get 100+ tokens/sec with Groq, save money with DeepSeek, or maximize quality with Claude - all without changing your client config. Available now for $3/month via GitHub Sponsors.

---

We're releasing **Terraphim LLM Proxy** - a production-ready intelligent routing proxy for LLM requests.

#### The Problem

If you use AI coding assistants like Claude Code, you know the pain:
- Claude is expensive at $15/M tokens for Opus
- Switching providers means changing configs
- Some tasks need speed, others need quality
- No easy way to route based on task type

#### The Solution

Terraphim LLM Proxy sits between your client and LLM providers:

```
Claude Code -> Proxy -> Groq (fast)
                    -> DeepSeek (cheap)
                    -> Claude (quality)
                    -> Cerebras (fast)
```

**One config change**. The proxy handles routing automatically.

#### Key Features

- **Sub-millisecond overhead**: 0.21ms routing decisions
- **Pattern-based routing**: "think step by step" -> reasoning model
- **Multi-provider support**: Groq, Cerebras, DeepSeek, OpenRouter, Ollama
- **Client detection**: Works with Claude Code, OpenClaw, Codex CLI
- **Build-time validation**: Model lists fetched at compile time

#### Fastest Routing

We've integrated both Groq and Cerebras for ultra-low latency:

| Provider | Speed | Use Case |
|----------|-------|----------|
| Groq | 100+ tokens/sec | Interactive coding |
| Cerebras | 100+ tokens/sec | Real-time apps |
| DeepSeek | 40-60 tokens/sec | Cost-sensitive |

#### Fair Source License

We're using **FSL-1.1-MIT** (Functional Source License):
- Use it internally, modify it, learn from it
- Builds on open source (Rust, tokio, axum)
- Converts to MIT automatically after 2 years
- Only restriction: don't create a competing SaaS

#### Get Access

**$3/month** via GitHub Sponsors: https://github.com/sponsors/terraphim

You'll get immediate access to the private repository with full source code.

---

## Twitter/X Posts

### Main Announcement

```
Releasing Terraphim LLM Proxy

Route Claude Code requests through Groq (100+ tok/sec), DeepSeek (cheap), or Claude (quality).

One proxy. All providers. 0.21ms overhead.

$3/mo via GitHub Sponsors
https://github.com/sponsors/terraphim

FSL-1.1-MIT license (-> MIT in 2 years)
```

### Thread Format

**1/6**
```
We built an intelligent LLM routing proxy for Claude Code users.

Problem: Claude is expensive. Switching providers is painful. Different tasks need different models.

Solution: One proxy that routes automatically.
```

**2/6**
```
How it works:

"think step by step" -> DeepSeek Reasoner
"quick question" -> Groq (100+ tok/sec)  
"complex code review" -> Claude Opus

Pattern matching in <1ms. No client changes needed.
```

**3/6**
```
Supported providers:
- Groq (blazing fast)
- Cerebras (also blazing fast)
- DeepSeek (dirt cheap)
- OpenRouter (Claude access)
- Ollama (local/free)

One config file. Done.
```

**4/6**
```
Works with:
- Claude Code
- OpenClaw
- Codex CLI
- Any OpenAI-compatible client

Auto-detects client type. Converts formats. Handles auth.
```

**5/6**
```
Why FSL-1.1-MIT license?

- Use it internally: YES
- Modify it: YES
- Education/research: YES
- Run your own instance: YES
- Create competing SaaS: NO (for 2 years)

Then it becomes MIT. Forever.
```

**6/6**
```
$3/month via GitHub Sponsors

https://github.com/sponsors/terraphim

Full source code access. Production-ready.

Built with Rust. 490+ tests. Zero warnings.
```

---

## Reddit Posts

### r/LocalLLaMA

**Title**: Terraphim LLM Proxy - Route Claude Code through Groq/Cerebras/DeepSeek with intelligent pattern matching

**Body**:
```
Hey r/LocalLLaMA,

I've been working on an LLM routing proxy that sits between coding assistants (Claude Code, OpenClaw) and multiple backends.

**The problem I was solving:**
- Claude is expensive for everyday coding
- Groq/Cerebras are fast but not always available
- DeepSeek is cheap but slower
- Switching providers means config changes everywhere

**What it does:**
- Routes based on patterns: "think step by step" -> DeepSeek Reasoner
- Routes based on cost: background tasks -> local Ollama
- Routes based on speed: interactive -> Groq (100+ tok/sec)
- Handles format conversion between Anthropic/OpenAI APIs
- 0.21ms routing overhead

**Tech stack:**
- Rust (axum, tokio)
- Build-time model fetching from Groq/Cerebras APIs
- 118 semantic routing patterns
- 490+ tests

**License:** FSL-1.1-MIT (Fair Source). You can run your own instance, modify it, use it internally. Only restriction is creating a competing SaaS product. Converts to MIT after 2 years.

**Access:** $3/month via GitHub Sponsors: https://github.com/sponsors/terraphim

Happy to answer questions about the routing architecture or implementation!
```

### r/MachineLearning

**Title**: [P] Terraphim LLM Proxy - Sub-millisecond intelligent routing for LLM requests

**Body**:
```
I've released an intelligent LLM routing proxy designed for AI coding assistants.

**Key technical details:**
- Pattern matching using Aho-Corasick automaton (<0.1ms match time)
- 6-phase routing: explicit -> pattern -> session -> cost -> performance -> hints
- Dedicated HTTP clients for Groq/Cerebras to handle API differences
- Build-time model validation with fallbacks for CI/CD
- Token counting at 2.8M tokens/sec with tiktoken-rs

**Performance:**
- Routing overhead: 0.21ms
- Memory footprint: ~2KB per connection
- Throughput: >4000 req/sec
- Zero-copy streaming support

**Architecture:**
```
Client -> API Detection -> Format Conversion -> Pattern Matching -> Provider Selection -> Response Streaming
```

The proxy automatically detects client type (Claude Code uses anthropic-version header, OpenClaw uses User-Agent) and converts between Anthropic Messages and OpenAI Chat Completions formats.

Source available via GitHub Sponsors ($3/mo): https://github.com/sponsors/terraphim

License: FSL-1.1-MIT (source-available, converts to MIT after 2 years)
```

### r/rust

**Title**: [Show r/rust] Terraphim LLM Proxy - Async LLM router with build-time code generation

**Body**:
```
Hey rustaceans! Sharing a project I've been working on.

**What it is:**
An intelligent routing proxy for LLM requests. Routes Claude Code/OpenClaw requests to Groq, Cerebras, DeepSeek, etc. based on patterns, cost, and performance.

**Interesting Rust patterns used:**

1. **Build-time code generation** - `build.rs` fetches model lists from provider APIs and generates Rust code:
```rust
// build.rs fetches from https://api.groq.com/openai/v1/models
// Generates src/groq_models_generated.rs
include!(concat!(env!("OUT_DIR"), "/groq_models_generated.rs"));
```

2. **Dedicated provider clients** - Different providers use different URL paths:
```rust
// Groq: /openai/v1/chat/completions
// Cerebras: /v1/chat/completions
// Can't use generic client, need dedicated impls
```

3. **Zero-copy SSE streaming** - Stream LLM responses without buffering full response

4. **Aho-Corasick pattern matching** - 118 patterns matched in <0.1ms

**Stats:**
- 490+ tests
- Zero clippy warnings
- 0.21ms routing overhead

**Crates used:** axum, tokio, reqwest, serde, tiktoken-rs, aho-corasick

Source: $3/mo via GitHub Sponsors (FSL-1.1-MIT license)
https://github.com/sponsors/terraphim

Feedback welcome!
```

---

## Hacker News

**Title**: Show HN: Intelligent LLM routing proxy for Claude Code ($3/mo, source-available)

**Body**:
```
Hi HN,

I built an LLM routing proxy that sits between AI coding assistants and multiple providers.

Problem: I use Claude Code daily but Claude is expensive. I wanted to route simple queries to Groq (fast, cheap) and complex ones to Claude (quality). Switching providers manually was tedious.

Solution: Terraphim LLM Proxy

- Pattern-based routing: "think step by step" -> DeepSeek Reasoner
- Speed-based routing: interactive queries -> Groq (100+ tok/sec)
- Cost-based routing: background tasks -> local Ollama
- 0.21ms routing overhead
- Works with Claude Code, OpenClaw, Codex CLI

Tech: Rust, axum, tokio. Build-time model fetching. 490+ tests.

License: FSL-1.1-MIT (Fair Source). Use internally, modify, self-host - all allowed. Only restriction is creating a competing SaaS. Converts to MIT after 2 years.

Why sponsor-only? I want to focus on building tools, not fighting free-riders. $3/mo seems fair for full source access.

GitHub Sponsors: https://github.com/sponsors/terraphim

Happy to discuss the routing architecture or answer questions!
```

---

## LinkedIn

**Title**: Launching Terraphim LLM Proxy

**Body**:
```
Excited to announce Terraphim LLM Proxy - an intelligent routing solution for AI coding assistants.

If you use Claude Code, OpenClaw, or similar tools, you know the challenge: Claude provides excellent quality but at $15/M tokens for Opus. Meanwhile, Groq offers 100+ tokens/sec at a fraction of the cost.

Terraphim LLM Proxy solves this by:

- Automatically routing requests based on task complexity
- Supporting multiple providers (Groq, Cerebras, DeepSeek, Claude)
- Adding <1ms overhead to routing decisions
- Detecting your client and handling format conversion

Built in Rust with 490+ tests and production-ready reliability.

Available now via GitHub Sponsors at $3/month.

License: FSL-1.1-MIT (Fair Source - converts to MIT after 2 years)

#AI #LLM #Rust #OpenSource #FairSource #Developer #Productivity
```
