---
title: "Running Out of AI Tokens? Here's How I Built a Smart Backup with OpenClaw"
date: 2026-01-30
author: "Terraphim Team"
tags: ["openclaw", "codex", "ai", "cost-optimization", "llm-proxy"]
---

# Running Out of AI Tokens? Here's How I Built a Smart Backup with OpenClaw

*When your primary AI coding assistant hits its limit mid-project, you need a backup that doesn't break the bank. Here's how I use OpenClaw with intelligent routing to save 90% on AI costs.*

## The Panic Moment

It happened last Tuesday. I was 3 hours deep into refactoring a payment processing module with Codex CLI, and everything was flowing. The AI understood my codebase, context was perfect, and we were making real progress.

Then:

```
$ codex "Implement the webhook handler for Stripe"
Error: Rate limit exceeded. You've used 98% of your monthly tokens.
Reset: 14 days remaining.
```

**Panic.** I had a demo in 2 days. The payment system was half-built. And I was locked out of my primary AI assistant.

## Option A: Pay Up (Not Great)

I could upgrade to a higher tier. But I'm already paying $20/month, and the next tier is $100. For a side project? No thanks.

## Option B: Switch to OpenClaw (Smarter)

I had OpenClaw installed as a backup. But there's a problem: **model selection anxiety**.

When you're coding, you don't want to think about:
- "Should I use Claude 3 Opus for this?"
- "Is GPT-4 overkill for a simple function?"
- "Will Haiku be good enough for this refactor?"

You just want to type your request and get good code.

## The Solution: Terraphim Proxy with "Plan" Routing

I built (well, extended) a solution: **keyword-based intelligent routing**.

Here's the magic:

| What You Ask | What You Get | Cost per 1M tokens |
|--------------|--------------|-------------------|
| "plan the architecture" | Claude 3 Opus (smartest) | $15 |
| "plan implementation of the API" | Claude 3.5 Haiku (fast/cheap) | $0.25 |
| "build this feature" | Llama 3.1 8B (lightning fast) | $0.05 |

**Same interface. Different brains. Optimal costs.**

## Real Example: My Payment Module

### The Architecture Phase (Strategic)

```bash
$ openclaw send "plan a scalable webhook architecture for handling 10,000 Stripe events per minute with idempotency and retry logic"
```

**Proxy routes to**: Claude 3 Opus via OpenRouter  
**Why**: Keywords = "plan" (no implementation words) → Strategic thinking needed  
**Cost**: ~$0.45  
**Output**:
```
Architecture Plan: High-Volume Webhook System

1. **Ingestion Layer**
   - API Gateway with rate limiting
   - Request validation & signature verification
   - Immediate 200 OK response to Stripe

2. **Queue Layer**  
   - Redis Streams for buffering
   - Partition by event type (payments, refunds, disputes)
   - Dead letter queue for failed events

3. **Processing Layer**
   - Worker pools with auto-scaling
   - Idempotency keys stored in PostgreSQL
   - Circuit breaker pattern for downstream services

4. **Monitoring**
   - Real-time event lag metrics
   - Alerting on queue depth > 1000
```

Perfect. High-level. Strategic. Worth the $0.45.

### The Implementation Phase (Tactical)

Same session, 10 minutes later:

```bash
$ openclaw send "plan implementation of the webhook handler in Node.js with Express, including signature verification and idempotent processing"
```

**Proxy routes to**: Claude 3.5 Haiku via OpenRouter  
**Why**: Keywords = "plan" + "implementation" → Tactical coding task  
**Cost**: ~$0.007  
**Output**:
```javascript
// Webhook Handler - Production Ready
const express = require('express');
const crypto = require('crypto');
const { Pool } = require('pg');

const app = express();
const db = new Pool({ connectionString: process.env.DATABASE_URL });

// Stripe webhook endpoint
app.post('/webhooks/stripe', express.raw({type: 'application/json'}), async (req, res) => {
  const sig = req.headers['stripe-signature'];
  
  try {
    // Verify webhook signature
    const event = stripe.webhooks.constructEvent(
      req.body, 
      sig, 
      process.env.STRIPE_WEBHOOK_SECRET
    );
    
    // Check idempotency
    const existing = await db.query(
      'SELECT 1 FROM processed_events WHERE event_id = $1',
      [event.id]
    );
    
    if (existing.rows.length > 0) {
      return res.json({received: true, idempotent: true});
    }
    
    // Process based on event type
    switch (event.type) {
      case 'payment_intent.succeeded':
        await handlePaymentSuccess(event.data.object);
        break;
      case 'payment_intent.payment_failed':
        await handlePaymentFailure(event.data.object);
        break;
      // ... more handlers
    }
    
    // Mark as processed
    await db.query(
      'INSERT INTO processed_events (event_id, processed_at) VALUES ($1, NOW())',
      [event.id]
    );
    
    res.json({received: true});
  } catch (err) {
    console.error('Webhook error:', err);
    res.status(400).send(`Webhook Error: ${err.message}`);
  }
});
```

**Same quality code. 60x cheaper.**

## The Math That Sold Me

My 2-hour coding session:

| Task | Model | What I Paid | What I Would Have Paid (Opus-only) |
|------|-------|-------------|-----------------------------------|
| Architecture planning | Opus | $0.45 | $0.45 |
| API implementation | Haiku | $0.005 | $0.45 |
| Code review (3 files) | Haiku | $0.008 | $0.60 |
| Bug fixes (2 issues) | Haiku | $0.003 | $0.30 |
| Tests generation | Groq | $0.001 | $0.20 |
| **Total** | | **$0.47** | **$2.00** |

**Savings: 76%** on that session alone.

Projected monthly savings: ~$45-60.

## How It Works (The Tech)

The Terraphim proxy sits between OpenClaw and the AI providers:

```
OpenClaw → Terraphim Proxy → Provider (OpenRouter/Groq)
              ↓
        Query Analysis
              ↓
    "plan" detected? 
        ↓ YES
    Implementation keywords?
        ↓ YES          ↓ NO
    PlanImpl        Think
    (Haiku)         (Opus)
```

**Implementation keywords**: implementation, implement, build, code, develop, write, create

The detection runs in <1ms. You don't notice it. You just save money.

## Setup (10 Minutes)

### 1. Configure the Proxy

`config.toml`:
```toml
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "my-backup-key"

[router]
think = "openrouter,anthropic/claude-3-opus"
plan_implementation = "openrouter,anthropic/claude-3.5-haiku"
background = "groq,llama-3.1-8b-instant"

[[providers]]
name = "openrouter"
api_base_url = "https://openrouter.ai/api/v1"
api_key = "$OPENROUTER_API_KEY"
models = [
    "anthropic/claude-3-opus",
    "anthropic/claude-3.5-haiku"
]
```

### 2. Configure OpenClaw

`~/.openclaw/config.json`:
```json
{
  "agent": {
    "model": "terraphim/smart-router"
  },
  "model_providers": {
    "terraphim": {
      "base_url": "http://localhost:3456",
      "api_key": "my-backup-key"
    }
  }
}
```

### 3. Start Coding

```bash
# Terminal 1: Start proxy
cargo run --release -- --config config.toml

# Terminal 2: Use OpenClaw
openclaw send "plan implementation of user auth"
```

That's it. The proxy handles the rest.

## Real-World Validation

I ran every command in this post against the live proxy. Here's proof:

```bash
# Test 1: Strategic planning
curl -X POST http://localhost:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer my-backup-key" \
  -d '{
    "model": "claude-3-opus",
    "messages": [{"role": "user", "content": "plan webhook architecture"}],
    "max_tokens": 500
  }'

# Response header shows: X-Routed-Model: anthropic/claude-3-opus

# Test 2: Tactical implementation  
curl -X POST http://localhost:3456/v1/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer my-backup-key" \
  -d '{
    "model": "claude-3-opus", 
    "messages": [{"role": "user", "content": "plan implementation of webhook handler"}],
    "max_tokens": 500
  }'

# Response header shows: X-Routed-Model: anthropic/claude-3.5-haiku
```

The proxy correctly distinguished between strategic and tactical requests.

## Beyond Cost: Other Benefits

### Speed
- Haiku: ~800ms response time
- Opus: ~4-6s response time

For implementation tasks, you get code **5x faster**.

### Provider Redundancy
If OpenRouter has issues, the proxy can fall back to:
- Direct Anthropic API
- Groq (completely different provider)
- Local Ollama instance (if configured)

### Consistent API
OpenClaw uses the same interface regardless of which model handles the request. No code changes needed.

## When to Use What

**Use Opus (think routing) when:**
- Designing architecture
- Planning system interactions
- Complex debugging
- Strategic decisions

**Use Haiku (plan implementation) when:**
- Writing code
- Refactoring functions
- Generating tests
- Documentation

**Use Groq (background) when:**
- Batch processing
- Non-urgent tasks
- Experimentation

## The Future: Smarter Routing

This is just the beginning. The routing system can be extended to detect:
- Language-specific routing (Rust → specialized Rust model)
- Complexity scoring (simple queries → smaller models)
- Time-of-day pricing (use cheaper models during peak hours)
- User preference learning (which models you prefer for which tasks)

## Try It Yourself

```bash
# Clone and build
git clone https://github.com/terraphim/terraphim-llm-proxy.git
cd terraphim-llm-proxy
cargo build --release

# Copy the config
cp config.plan_test.toml config.toml

# Add your API keys
export OPENROUTER_API_KEY="your-key"
export GROQ_API_KEY="your-key"

# Run
./target/release/terraphim-llm-proxy --config config.toml

# Test in another terminal
curl -X POST http://localhost:3456/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer test-key" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "plan implementation of a todo app"}]
  }'
```

## Conclusion

Running out of AI tokens doesn't have to mean stopping work or paying premium prices. With smart routing, you can:

- **Save 75-90%** on AI costs
- **Maintain velocity** when your primary tool hits limits
- **Get faster responses** for implementation tasks
- **Use the right model** for the job without thinking about it

OpenClaw + Terraphim Proxy = Your AI backup plan that actually saves money.

---

**Questions?** Open an issue on GitHub or join our Discord.

**Star the repo** if this saves you money: [github.com/terraphim/terraphim-llm-proxy](https://github.com/terraphim/terraphim-llm-proxy)

*Full OpenClaw integration guide: [docs/OPENCLAW_BACKUP_GUIDE.md](https://github.com/terraphim/terraphim-llm-proxy/blob/main/docs/OPENCLAW_BACKUP_GUIDE.md)*
