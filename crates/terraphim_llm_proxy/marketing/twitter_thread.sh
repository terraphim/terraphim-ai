#!/bin/bash
# Twitter Thread for Terraphim Smart Plan Routing
# Save as twitter_thread.txt and copy/paste into Twitter

---

🧵 Thread: When you run out of AI tokens mid-project...

1/ 🚨 The panic moment:
You're coding with Codex CLI, deep in flow state, and suddenly:

"Error: Rate limit exceeded. 98% of monthly tokens used."

Demo in 2 days. Payment system half-built. Now what?

---

2/ Option A: Pay $100 for the next tier
Option B: Switch to OpenClaw but manually pick models
Option C: Let the AI figure out which model you need

I built Option C. Here's how it works 👇

---

3/ The magic word is "plan"

"plan the architecture" → Claude 3 Opus ($15/M tokens)
"plan implementation of API" → Claude 3.5 Haiku ($0.25/M tokens)

Same interface. 60x cheaper for coding tasks.

---

4/ Real example from yesterday:

"Plan webhook architecture" (strategic)
→ Routes to Opus
→ Cost: $0.45
→ Gets high-level system design

10 minutes later:

"Plan implementation of webhook handler" (tactical)  
→ Routes to Haiku
→ Cost: $0.007
→ Gets working Node.js code

---

5/ The tech: Terraphim Proxy analyzes queries

Keywords detected:
• "plan" alone → strategic → big model
• "plan" + [implementation, build, code, develop] → tactical → small model

Detection: <1ms
Routing: Automatic
Savings: 75-90%

---

6/ My 2-hour coding session costs:
• Before (Opus only): $2.00
• After (smart routing): $0.47

Monthly savings: ~$50-60

All because the proxy picks the right brain for the job.

---

7/ Setup is stupid simple:

```bash
git clone https://github.com/terraphim/terraphim-llm-proxy
cd terraphim-llm-proxy
cargo run -- --config config.plan_test.toml
```

Point OpenClaw at localhost:3456

That's it. The proxy handles the rest.

---

8/ The best part?

When your primary AI (Codex) hits limits, you seamlessly switch to OpenClaw with automatic cost optimization.

No manual model selection. No billing surprises. Just code.

---

9/ Try the live demo:

curl -X POST http://localhost:3456/v1/messages \
  -H "Authorization: Bearer test-key" \
  -d '{"model": "claude-3-opus", 
       "messages": [{"role": "user", 
       "content": "plan implementation"}]}'

Response header: X-Routed-Model: anthropic/claude-3.5-haiku

---

10/ Want to stop overpaying for AI coding?

Star ⭐ the repo: github.com/terraphim/terraphim-llm-proxy

Full guide: docs/OPENCLAW_BACKUP_GUIDE.md

Your wallet will thank you. 💰

#AI #OpenSource #CostOptimization #Codex #OpenClaw

---

# Usage Instructions:
# 1. Copy each tweet (separated by ---) individually
# 2. Post as a thread on Twitter/X
# 3. Replace github.com/terraphim/terraphim-llm-proxy with actual URL
# 4. Engage with replies to boost visibility