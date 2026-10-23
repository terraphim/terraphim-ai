# 3 Ways to Save Money on Tokens for Your AI Agents

> Originally published on the Terraphim blog: [3 Ways to Save Money on Tokens for Your AI Agents](https://terraphim.ai/posts/save-money-tokens-ai-agents/) (19 October 2026).

synonyms:: 3 ways to save money on tokens for your ai agents, save money tokens ai agents
checklist:: search first let a cheap model summarise, put expensive reasoning at the edges of the v not in the mid, put a proxy in front of the providers, what we do not claim, related


Your agent bill is mostly input tokens. Not the clever parts: the re-reading.

*Three levers we run in production: let a cheap model summarise the evidence, put expensive reasoning only where it earns its keep, and route every request through a proxy that knows the price of each lane.*

Every turn, a coding agent replays its entire transcript to the provider. Add a repository to explore and a handful of tool outputs, and a single session can carry a hundred thousand tokens of context that the model reads over and over at full price. The frontier model you are paying for spends most of its expensive attention budget on text it has already seen, or text a much cheaper model could have filtered first.

We run a fleet of overnight coding agents on a single machine, so we feel this directly. Over the past few months we have landed on three levers that cut token spend without cutting output quality. Each one is running in production today, and each one comes with a caveat we will not hide: we have measured the mechanisms, not the end-to-end savings. The arithmetic below uses published prices and our own logs; treat it as illustrative, not invoiced.

## 1. Search first, let a cheap model summarise

The most expensive way to give a frontier model context is to hand it raw search results. The cheapest way is to let a small model read them first.

The pattern looks like this. A local search tool retrieves candidate chunks from your codebase or knowledge base. Before those chunks reach your GPT-6 or Fable-class session, a lower-cost model synthesises them into a compact answer, with deterministic references back to the source. The expensive model receives the synthesis and the few references it may need to reopen, not ten near-duplicate excerpts.

We do this with `terraphim-grep` (part of the open-source [terraphim-clients](https://github.com/terraphim/terraphim-clients) repository), which resolves query terms through a curated knowledge-graph thesaurus before ranking local matches:

```bash
export OPENROUTER_MODEL='deepseek/deepseek-v4-flash-0731'

terraphim-grep provider \
  --thesaurus docs/src/thesaurus.json \
  --haystack code --paths crates -n 10 \
  --answer --force-rlm --json
```

`--answer` requests an answer when the sufficiency check calls for one; `--force-rlm` forces the synthesis path. Only the selected excerpts leave the machine, through OpenRouter to a DeepSeek-flash-class model. There is also `--search-only`, which hard-disables synthesis and skips building the LLM client entirely, so a stray API key in the environment cannot cost a single network call.

In our verified run, the raw chunk contents measured 505 characters, the evidence with source locations 1,108 characters, and the synthesised answer text 440 characters: the frontier model received about 40% of the context it would otherwise have been fed, with references it could reopen on demand.

Two honest caveats. First, we measured characters, not tokens: the CLI does not expose token usage, so treat the ratio as directional. Second, the win is conditional on the handoff shape. The full answer object with all ten references measured 1,918 characters; forward that wholesale and the saving evaporates. Pass the synthesis plus the references needed for verification, nothing more.

## 2. Put expensive reasoning at the edges of the V, not in the middle

The second lever is not a tool but a lifecycle decision: match the model's price to the phase of work.

We structure agent work as a V-model. The left side asks "what should we build" (research, design, specification); the right side asks "did we build it correctly" (review, validation, judge panels). The long middle (implementation, test-writing, mechanical verification loops) is where most of the tokens burn, and it is precisely the part that needs the least reasoning depth.

So we split the model placement:

- **Edges get the expensive models.** Research synthesis, architecture decisions, and final review stay on high-reasoning lanes, because a bad plan or a rubber-stamped review costs more than the tokens save.
- **The middle runs on fast delivery models.** Implementation and test loops run on grok, MiniMax, or Kimi highspeed lanes over flat-rate subscriptions. Our overnight agents execute on `opencode-go` subscription models; our judge panels run on Kimi K2.5, which reaches 90% verdict agreement with human reviewers at around nine seconds per review.

The arithmetic is worth doing once. At published API prices, an illustrative workload of 100 million input and 20 million output tokens costs $800 if everything runs on a frontier model. Route 80% of it to a mid-tier model and the same token volume costs $480: 40% less, with the model count unchanged. That is a counterfactual at published prices, not a measured bill, but the shape of the saving is real: you are not paying for fewer tokens, you are paying less per token for the phases that do not need frontier reasoning.

The catch is discipline. Left to itself, an agent uses whatever model it started on for everything. The phases have to be explicit, which is why this lever only works inside a structured lifecycle rather than a free-running session.

## 3. Put a proxy in front of the providers

The third lever makes the cheap route the default: an LLM proxy between the agent and every provider, routing each request by what it contains or how fast each lane currently is.

Ours is `terraphim-llm-proxy`, shared with our [GitHub Sponsors](https://github.com/sponsors/terraphim) supporters. It sits on localhost, exposes OpenAI and Anthropic compatible endpoints, and decides where each request goes:

- **Keyword routing.** The proxy matches prompt content against a markdown taxonomy using Aho-Corasick string matching (microsecond overhead, the same crate our knowledge graph uses). A prompt that says "think step by step about this architecture" routes to the `think` chain regardless of the model ID the client requested. In our logs, three consecutive requests landed on three different providers under three different scenarios, transparently to the client.
- **Speed and price routing.** Eligible candidates can be ranked by time to first token, or by cost class: free lanes first, then subscription, then metered. Per scenario you choose whether latency or price wins. Agent idle time waiting on a rate limit is a real cost, just billed in hours rather than tokens.
- **Fallback chains make the cheap lanes safe.** Cheap providers have real reliability gaps: silent API format changes, undocumented capacity errors, higher latency from Europe. Pipe-delimited fallback chains mean a dead provider just moves the request to the next target. We verify this with deliberate failure injection (pointing the primary provider's domain at localhost in `/etc/hosts`) rather than trusting the config.

One caveat that took us by surprise: **prompt caches are per model, not per session**. Measured across real coding sessions, 82% of mid-session model switches collapsed the cache read below 10% of its previous value: a warm 107,000-token prefix became 110,000 tokens of uncached input, roughly ten times the cached price for the same text. The saving pattern is therefore to route the session or the scenario, not each turn at random. Pick a lane for the job and stay on it.

## What we do not claim

Token savings are not measured end-to-end. The synthesis example is measured in characters, not tokens; the model-mix example is arithmetic at published prices, not an invoice; the 82% cache-collapse figure comes from one operator's session logs, not a fleet benchmark. The honest metric is quality-adjusted cost per accepted task, and we are still building the instrumentation to report it.

What we can claim is mechanism: retrieval that filters before the expensive model reads, a lifecycle that spends reasoning budget where judgement lives, and a proxy that prices the lanes before it forwards the request. None of these require trusting a benchmark. All three are running today.

## Related

- [terraphim-grep and role-aware search](https://terraphim.ai/posts/skill-spotlight-terraphim-grep/)
- [Zero-Token LLM Routing](./zero-token-llm-routing.md) -- the deterministic routing decision layer
- [Disciplined engineering: the V-model for AI agents](https://terraphim.ai/posts/disciplined-engineering-ai-systems/)

The grep tooling is in [terraphim-clients](https://github.com/terraphim/terraphim-clients). The routing proxy from section 3 is `terraphim-llm-proxy`, shared with [GitHub Sponsors](https://github.com/sponsors/terraphim) supporters. If you want this running inside your team rather than as homework, we do it as part of our consulting work at [zestic.ai](https://zestic.ai).
