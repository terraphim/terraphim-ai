# Keyword-Based Intelligent Routing: Let Your Words Choose the Model

**Author:** Terraphim Team
**Date:** 2026-02-01
**Tags:** LLM, Routing, Pattern Matching, Taxonomy, Cost Optimization

---

## The Problem: Manual Model Selection

Every LLM has strengths:
- **DeepSeek Reasoner**: Excellent at step-by-step reasoning
- **DeepSeek Chat**: Extremely cost-effective for simple tasks
- **Claude Sonnet**: Best overall quality for complex work
- **Groq Llama**: Fastest inference for real-time needs

But manually selecting the right model for each task is tedious:

```python
# The tedious way
if "think" in user_message or "plan" in user_message:
    model = "deepseek-reasoner"
elif "cheap" in user_message or "budget" in user_message:
    model = "deepseek-chat"
elif "fast" in user_message or "urgent" in user_message:
    model = "claude-sonnet"
else:
    model = "default"
```

What if the proxy could do this automatically?

---

## The Solution: Taxonomy-Driven Pattern Matching

Terraphim LLM Proxy uses a knowledge graph approach to route requests based on message content. Simply send `model: "auto"` and let keywords in your message determine the optimal model.

### Architecture

```
docs/taxonomy/routing_scenarios/
    think_routing.md        -> deepseek-reasoner
    low_cost_routing.md     -> deepseek-chat
    high_throughput_routing.md -> claude-sonnet-4.5
    web_search_routing.md   -> perplexity-sonar
    ...
```

Each taxonomy file contains:
1. **route::** directive specifying provider and model
2. **synonyms::** list of keywords that trigger this route

At startup, the proxy builds an Aho-Corasick automaton with all 118 patterns for sub-millisecond matching.

---

## Live Demonstration

### Setup

```bash
# Start proxy with pattern routing enabled
op run --env-file=.env.openclaw-test -- \
  ./target/release/terraphim-llm-proxy -c config.openclaw-test.toml
```

### Test 1: Thinking Keywords

```bash
curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "auto",
    "max_tokens": 50,
    "messages": [{
      "role": "user",
      "content": "I need to think step by step about this architecture plan."
    }]
  }'
```

**Result:**
```json
{
  "model": "deepseek-reasoner",
  "content": [{"type": "text", "text": "Let me break this down systematically..."}]
}
```

**Proxy Log:**
```
INFO Routing decision made scenario=Pattern("think_routing")
     provider=deepseek model=deepseek-reasoner
```

### Test 2: Budget Keywords

```bash
curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "auto",
    "max_tokens": 50,
    "messages": [{
      "role": "user",
      "content": "I need a cheap budget solution for this task."
    }]
  }'
```

**Result:**
```json
{
  "model": "deepseek-chat",
  "content": [{"type": "text", "text": "Here's a cost-effective approach..."}]
}
```

**Proxy Log:**
```
INFO Routing decision made scenario=Pattern("slow_&_cheap_routing")
     provider=deepseek model=deepseek-chat
```

### Test 3: Speed Keywords

```bash
curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "auto",
    "max_tokens": 50,
    "messages": [{
      "role": "user",
      "content": "I need a fast response urgently for this production issue."
    }]
  }'
```

**Result:**
```json
{
  "model": "llama-3.3-70b-versatile",
  "content": [{"type": "text", "text": "OK"}]
}
```

**Proxy Log:**
```
INFO Routing decision made scenario=Pattern("fast_&_high_throughput_routing")
     provider=groq model=llama-3.3-70b-versatile
```

Note: Speed keywords now route to Groq for ultra-low latency (100+ tokens/sec). The proxy correctly passes the resolved model to Groq's API, supporting all 20+ models fetched dynamically at build time.

---

## Taxonomy File Format

Each routing scenario is defined in a markdown file:

```markdown
# Think Routing

Think routing is used for complex problem-solving tasks that require
deep reasoning, step-by-step analysis, or extended chain-of-thought.

route:: deepseek, deepseek-reasoner

Optimal for:
- Architecture design and system planning
- Complex debugging and root cause analysis
- Multi-step problem decomposition

synonyms:: think, plan, reason, analyze, break down, step by step,
think through, reason through, consider carefully, work through,
design thinking, systematic thinking, logical reasoning, critical thinking
```

The proxy parses:
- **route::** -> provider and model to use
- **synonyms::** -> keywords that trigger this route

### Pattern Matching Algorithm

1. All synonyms from all taxonomy files are compiled into an Aho-Corasick automaton
2. User message is lowercased and scanned
3. All matching patterns are scored by:
   - Match length (longer = better)
   - Match position (earlier = slightly better)
4. Highest-scoring match determines the route
5. If no match, falls back to default provider

---

## Routing Priority

The 6-phase routing system applies rules in order:

| Phase | Check | Example |
|-------|-------|---------|
| 0 | Explicit provider in model | `groq:llama-3.3-70b` |
| 1 | Priority pattern matching | Keywords in message |
| 2 | Session-aware routing | User preferences |
| 3 | Cost optimization | Budget constraints |
| 4 | Performance optimization | Latency requirements |
| 5 | Scenario hints | Background, images, etc. |

Pattern matching (Phase 1) runs early, but explicit provider specification (Phase 0) takes precedence.

---

## Adding Custom Routes

### Step 1: Create Taxonomy File

```markdown
# docs/taxonomy/routing_scenarios/code_review_routing.md

# Code Review Routing

For thorough code review tasks requiring careful analysis.

route:: openrouter, anthropic/claude-opus-4.5

synonyms:: review, code review, audit, check my code, find bugs,
security review, vulnerability, code quality, best practices
```

### Step 2: Restart Proxy

```bash
# Proxy reloads taxonomy on startup
./target/release/terraphim-llm-proxy -c config.toml
```

**Log output:**
```
INFO Found 10 taxonomy files
INFO Built automaton with 130 patterns
```

### Step 3: Test New Route

```bash
curl -X POST http://127.0.0.1:3456/v1/messages \
  -d '{"model": "auto", "messages": [{"role": "user",
       "content": "Please review my code for security issues"}]}'
```

Routes to `claude-opus-4.5` for thorough code review.

---

## Cost Savings Example

Consider a development workflow:

| Task Type | Without Routing | With Pattern Routing | Savings |
|-----------|-----------------|---------------------|---------|
| Simple questions | Claude Sonnet ($3/M) | DeepSeek Chat ($0.14/M) | 95% |
| Reasoning tasks | Claude Sonnet ($3/M) | DeepSeek Reasoner ($0.55/M) | 82% |
| Urgent fixes | Claude Sonnet ($3/M) | Claude Sonnet ($3/M) | 0% |

With pattern routing, the proxy automatically selects the most cost-effective model for each task type.

---

## Best Practices

### 1. Use Clear Keywords

The more explicit your keywords, the better the routing:

```json
// Good - clear intent
{"content": "I need to think step by step about this complex algorithm"}

// Less clear - may not trigger think routing
{"content": "Help me with this algorithm"}
```

### 2. Combine with Model Aliases

For clients that always specify a model (like OpenClaw), use model aliases:

```toml
# Route claude-sonnet requests through pattern matching
[[router.model_mappings]]
from = "claude-sonnet-auto"
to = "auto"  # Triggers pattern matching
```

### 3. Monitor Routing Decisions

Enable debug logging to see routing decisions:

```bash
RUST_LOG=debug ./target/release/terraphim-llm-proxy -c config.toml
```

Look for:
```
DEBUG Phase 1: Priority pattern match
INFO  Routing decision made scenario=Pattern("think_routing")
```

---

## Performance

Pattern matching adds minimal overhead:

| Metric | Value |
|--------|-------|
| Patterns loaded | 118 |
| Automaton build time | ~3ms (startup only) |
| Match time per request | < 0.1ms |
| Memory overhead | < 1MB |

The Aho-Corasick algorithm scans the message once, finding all pattern matches simultaneously.

---

## Supported Patterns

| Scenario | Keywords (partial list) | Routes To |
|----------|------------------------|-----------|
| think_routing | think, plan, reason, analyze, step by step | deepseek-reasoner |
| slow_&_cheap_routing | cheap, budget, economy, affordable, slow | deepseek-chat |
| fast_&_high_throughput_routing | fast, urgent, realtime, critical, quick | groq/llama-3.3-70b-versatile |
| web_search_routing | search, lookup, current events, latest news | perplexity-sonar |
| long_context_routing | (token count > 60K) | gemini-flash |
| background_routing | background, batch, async, queue | claude-haiku |
| image_routing | image, picture, screenshot, diagram | claude-sonnet-4.5 |

---

## Conclusion

Keyword-based routing lets your message content drive model selection:

- **Automatic optimization**: Right model for each task
- **Cost savings**: Cheap models for simple tasks
- **Quality preservation**: Premium models when needed
- **Zero client changes**: Just use `model: "auto"`

The taxonomy-driven approach makes it easy to add custom routing rules without code changes.

---

## Related Documentation

- [Multi-Client Integration Guide](../MULTI_CLIENT_INTEGRATION.md)
- [Intelligent Routing Blog Post](intelligent-routing.md)
- [Routing Architecture](../ROUTING_ARCHITECTURE.md)

---

## Try It Yourself

```bash
# Clone and build
git clone https://github.com/terraphim/terraphim-llm-proxy
cd terraphim-llm-proxy
cargo build --release

# Configure (add your API keys)
cp .env.example .env.openclaw-test
vim .env.openclaw-test

# Start with pattern routing
op run --env-file=.env.openclaw-test -- \
  ./target/release/terraphim-llm-proxy -c config.openclaw-test.toml

# Test pattern routing
curl -X POST http://127.0.0.1:3456/v1/messages \
  -H "anthropic-version: 2023-06-01" \
  -H "x-api-key: your_key" \
  -d '{"model": "auto", "messages": [{"role": "user",
       "content": "I need to think carefully about this design"}]}'
```
