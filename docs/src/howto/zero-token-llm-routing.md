# Zero-Token LLM Routing: Deterministic Decisions Before the Call

> Originally published on the Terraphim blog: [The cheapest routing decision is the one that costs no tokens](https://terraphim.ai/posts/zero-token-routing-vs-cloudflare/) (23 October 2026).

synonyms:: zero-token llm routing, zero token llm routing, zero-token llm routing deterministic decisions before the call
checklist:: two different cost stacks, a worked example on the live proxy, where the decision actually happens, honest limits where the inside-the-call approach still wins, the arithmetic that matters, how to get the proxy, see also


Cloudflare published an excellent post: [Cutting our inference bill by 68% by being obsessive about optimization](https://blog.cloudflare.com/obsessive-inference-optimization/). Their AI code reviewer went from $2.43 to $0.77 per review, and the median review time fell from 4m 09s to 1m 32s. It is a masterclass in squeezing the token economics of a single provider: shared prompt-cache prefixes, fewer LLM calls per session, and relentless model/effort A/B tests.

It is also a study in what optimisation inside the call cannot reach. Every one of their levers operates after the request has already been committed to a provider, a model, and an effort level. Their architecture answered the question of which lane each request deserves by experiment rather than by a decision mechanism. That layer is what terraphim-llm-proxy exists for, and the interesting part is that its answer costs zero tokens.

## Two different cost stacks

Cloudflare's optimisations and Terraphim's routing operate on disjoint parts of the spend. Neither substitutes for the other, but only one of them requires a Cloudflare-scale engineering team and production traffic to discover.

| Lever | Layer | What it costs to make the decision |
|---|---|---|
| Shared cache prefixes (14–26% → 91–99.5% first-request hits) | Inside the call | Engineering time; nothing per request once landed |
| Fewer LLM calls per session | Inside the call | Engineering time |
| Model + effort A/B splits (Sonnet 5 vs 5.5, medium vs high) | Across calls | **Real production traffic and real dollars for days per split** |
| Keyword/synonym routing to provider lanes | Before the call | **Zero tokens, sub-millisecond, deterministic** |
| Latency-probed "fastest" policy | Before the call | Probe requests; zero tokens on the routed request |
| Runtime fallback on outage/rate-limit/credit exhaustion | Before the call | Zero; the fallback is a failover rather than a retry of the same lane |

Cloudflare's answer to "which model should handle this" was a 50/50 production split run for two and a half days, read off dashboards, then rolled out. That is the honest way to do it when your router is a human. A deterministic router answers the same question per-request, for free, in microseconds, and it can change its mind the moment a lane degrades.

## A worked example on the live proxy

These are real requests through the Terraphim Proxy desktop app running on `127.0.0.1:17890` with five configured provider lanes: z.ai GLM-5.3, MiniMax-M3, OpenAI Codex gpt-5.6-sol, xAI Grok-4.3, and a free OpenRouter lane (nvidia/nemotron-3.5-lightning:free). The route policy was `fastest`, which is latency-probed rather than keyword-pinned.

```text
$ curl -X POST http://127.0.0.1:17890/v1/messages \
    -H "Authorization: Bearer <caller-key>" \
    -d '{"model":"auto","max_tokens":32,
         "messages":[{"role":"user","content":"Reply with exactly: ok"}]}'
```

| Request | Route decision | Provider that served it | Wall time | Decision cost |
|---|---|---|---|---|
| "deep security code review…" | `auto` → fastest lane | MiniMax-M3 | 1.28s | 0 tokens |
| "Reply with exactly: ok" | `auto` → fastest lane | MiniMax-M3 | 0.92s | 0 tokens |
| `openrouter:nvidia/...:free` pinned | explicit pin | nemotron-3.5-lightning:free | 1.09s | 0 tokens |
| `xai:grok-4.3` pinned | explicit pin | grok-4.3 | **9.73s** | 0 tokens |

Read the last two rows together. At the moment of measurement, Grok-4.3's lane was roughly **10× slower** than MiniMax's; transient congestion is exactly the condition a human-picked "best model" cannot react to. The `fastest` policy had already steered automatic traffic away from it. No alert fired, no dashboard was watched, no incident was declared: the probe data just ranked the lanes and `auto` avoided the slow one. Pinning is still available for when you *want* a specific lane; the point is that the default behaviour absorbs provider badness for free.

And the free lane is not decorative. `nvidia/nemotron-3.5-lightning:free` answered correctly at 1.09s. A $0 lane exists precisely so that traffic which does not need a frontier model never pays for one.

## Where the decision actually happens

The routing decision is deterministic string matching, not a model call:

1. **Capability keywords** are extracted from the request content using word-boundary matching over the canonical `KeywordRouter` table, where "review", "plan", "security", "think" and friends map to scenario routes.
2. **Keyword remaps and user synonyms** are resolved against the rolegraph-backed probe table before routing. Your synonym ("audit" → "review") is a first-class graph probe rather than a bolted-on regex.
3. **Route policies** (`auto`, `fastest`, `deep-think`, `code-review`, `cheapest`) select the lane; FillFirst chains order fallbacks across *providers*, not just models.
4. If the chosen lane 429s, 5xxs, or runs out of credit, the request **fails over to the next lane in the chain**, so the caller sees one response instead of an outage.

Total inference tokens spent deciding: **zero**. The decision is a lookup, not a completion.

Compare the alternative that is currently fashionable: a model-based "decision layer" or classifier that reads each request and predicts which model should handle it. That decision itself costs tokens on every single request, plus a serialization round-trip of added latency. It is inference spent so you can spend inference. For a single workload the overhead is trivial; at agent-fleet scale you have built a second inference bill whose only product is *billing decisions*.

## Honest limits: where the inside-the-call approach still wins

The two levers stack; pretending otherwise would be dishonest:

- **If you are locked to one provider**, nothing in a router helps. Cloudflare's prefix-cache work (43% of their spend was cache writes + 34% cache reads) is the correct lever, and it composes: route to the lane, then cache within the session.
- **Deterministic routing needs a useful keyword signal.** A request with no capability keywords falls to the default route, which is whatever policy you chose and nothing smarter. Model-based routers can in principle read intent that keywords miss; whether that justifies their per-request token cost is the honest trade-off.
- **Measured evals justify provider choice per workload.** Picking providers by measured quality per scenario rather than by keyword alone is sound; Terraphim's scenario routes are how you *operationalise* that verdict cheaply once reached (review → Opus-tier lane, cheap chat → free lane), rather than re-deriving it on every request.

## The arithmetic that matters

A reviewer that makes ~10 LLM calls per review and spends $0.77 would pay a further 5k tokens per review if a model-based router spent even 500 tokens triaging each call. Small per request, but it buys nothing a deterministic probe does not already know. Multiply by agent-fleet volume and the decision layer becomes a line item.

The inverse is the actual saving: the free OpenRouter lane answered the trivial request for $0.00 where a frontier lane would have billed input + output. At thousands of reviews, "which requests genuinely need the expensive model" is the question worth more than "how do we cache better", and it costs zero tokens to ask when the answer is a keyword rather than a completion.

Obsessive optimisation inside the call is necessary. It is not sufficient. The cheapest token is still the one you never spend, including the ones you would have spent *deciding*.

## How to get the proxy

Two routes, depending on whether you want the binary or the source:

- **Terraphim Proxy (the app):** notarised macOS DMG for direct download today; Mac App Store submission in review. Demo: [terraphim.ai/video/terraphim-proxy-review-demo.mp4](https://terraphim.ai/video/terraphim-proxy-review-demo.mp4).
- **Source access via a sponsor tier, limited time.** `terraphim-llm-proxy` is currently sponsor-gated on GitHub. The **$10/month** tier at [github.com/sponsors/terraphim](https://github.com/sponsors/terraphim) grants repository access: multi-provider routing (Groq, Cerebras, DeepSeek, OpenRouter, z.ai, MiniMax, xAI, Codex), pattern-based semantic routing, sub-millisecond routing overhead, plus the knowledge-graph embedding engine. Everything ships under **FSL-1.1-MIT**, the Functional Source License that converts to plain MIT two years after each release, so sponsor access today becomes an open-source guarantee on a timer rather than a perpetual toll. The sponsor gate is a limited-time arrangement: once the repository goes fully public, this route to early access closes. ($15/month covers all private repositories.)

The arithmetic from above applies directly: if a $10/month tier saves you frontier-model tokens on routing decisions, or lands one free-lane fallback during an outage, it pays for itself in the first bad week.

## See also

- [LLM proxy configuration](https://docs.terraphim.ai/llm-proxy-configuration.html)
- [3 Ways to Save Money on Tokens for Your AI Agents](./save-money-tokens-ai-agents.md)
