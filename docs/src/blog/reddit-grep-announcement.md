# Reddit Announcements -- Terraphim Grep

## r/rust

**Title:** Hybrid code search in Rust: ripgrep-speed retrieval + knowledge graph + LLM fallback, with a four-layer no-mocks test pyramid

**Body:**

We shipped `terraphim_grep` -- a search tool that runs `fff-search` (ripgrep-style code scan) and a knowledge-graph concept lookup in parallel, then asks an LLM to synthesise a cited answer *only when the local retrievers do not return enough signal*. The decision to call the LLM is governed by a `SufficiencyJudge` looking at coverage, diversity, and KG confidence -- not a global flag.

The architectural piece I want to share is how we wired the LLM.

### One entry point: `build_llm_from_role`

Terraphim ships a capability-based router that picks providers by extracting `Capability` (CodeGeneration, Explanation, DeepThinking, ...) from a prompt and applying a strategy (CostOptimized, CapabilityFirst). We did not wire that router directly into grep. We wired `terraphim_service::llm::build_llm_from_role(&role)` -- the same entry point the server, TUI, and RLM use. Whether routing kicks in is a *role config* decision (`llm_router_enabled = true`), not a code decision.

The wiring is six lines:

```rust
let llm = terraphim_service::llm::build_llm_from_role(&role);
let grep = TerraphimGrep::new(hybrid, judge);
let grep = match llm {
    Some(client) => grep.with_llm_client(client),
    None => grep,  // search-only mode is a first-class state
};
```

Grep never knows whether it got back an `OllamaClient`, an `OpenRouterClient`, or a `RouterBridgeLlmClient`. It just sees `Arc<dyn LlmClient>`. When someone adds Claude or Gemini providers, grep picks them up automatically.

### The test pyramid: zero mocks

Our project rule says no mocks. That forced a more honest pyramid:

| Layer | What it proves | Cost |
|---|---|---|
| L1 inline unit | Prompt assembly, signature parsing, sufficiency routing, graceful-degrade against a real fff scan | Free, ms |
| L2 router capability | Real `Router` + two providers, real grep prompts, assert which capability was extracted and which provider won | Free, ms |
| L3 e2e (`#[ignore]`) | Full pipeline against `liquid/lfm-2.5-1.2b-instruct:free` -- 1.2B params, sub-second, free OpenRouter | $0, ~1s |
| L4 quality (`#[ignore]`, manual) | Same shape but `qwen/qwen3-coder:free` for stronger answers on code | $0, ~3s |

The L2 layer is the interesting one. It does not call any LLM. It just asserts that grep's synthesis prompts produce the expected routing decisions. This catches the case where prompt wording changes silently break capability extraction.

We borrowed the `is_account_issue()` helper from an earlier test plan so 401/403/429 from OpenRouter degrade to a skip rather than a failure. CI stays green when the free-tier daily quota runs out.

### What the bench numbers told us

First run of `cargo bench --bench hybrid_search -- hybrid_with_kg`:

```
hybrid_with_kg/thesaurus_terms/10        3.2664 ms
hybrid_with_kg/thesaurus_terms/100       3.4457 ms
hybrid_with_kg/thesaurus_terms/1000      3.2413 ms
hybrid_with_kg/thesaurus_terms/10000     3.2126 ms
```

Hybrid latency stays flat at ~3.2 ms across three orders of magnitude of thesaurus size. The parallel `tokio::spawn` fff scan dominates wall-clock; KG search is fast enough to vanish in the noise. Useful baseline: if anyone proposes KG-pruning as a perf win, the bench will tell them whether it actually moves the needle.

### Free models worth knowing about on OpenRouter today

- `qwen/qwen3-coder:free` -- code-specialised, best for code synthesis
- `meta-llama/llama-3.3-70b-instruct:free` -- strong general reasoning
- `liquid/lfm-2.5-1.2b-instruct:free` -- 1.2B params, sub-second, lowest quota burn (great for CI smoke)

Free tier caps: 20 req/min, 200 req/day (1000 if you have ever deposited $10).

Repo: [github.com/terraphim/terraphim-ai](https://github.com/terraphim/terraphim-ai). PR with full design rationale and benchmarks at issue #1743.

## r/programming

**Title:** Building a code search tool that knows when to ask an LLM instead of returning thirty unranked grep hits

**Body:**

You search a million-line codebase for "where is retry configured." Ripgrep prints thirty hits. None obviously right. You read all thirty.

A simpler question: what if search could *know* when its results are not good enough? When that happens, it asks an LLM to synthesise a cited answer from the chunks it did find. When the local retrievers do find enough, it just returns them -- no LLM call needed.

We shipped this in [terraphim_grep](https://github.com/terraphim/terraphim-ai). The design has four moving parts:

1. **fff-search** for ripgrep-style code retrieval (the speed floor)
2. **Knowledge graph** for concept extraction in parallel (the semantic floor)
3. **Sufficiency judge** -- coverage, diversity, KG confidence heuristics that decide if the local layer is enough
4. **LLM synthesis with citations** -- only triggered when the judge says local is not enough

The "no LLM" path is a first-class state. CLI works without any API key configured, returns the chunks it has, exits cleanly. Synthesis is enrichment, not the entry barrier.

We tested it without writing a single mock. Layer 1: inline unit tests against a real filesystem. Layer 2: real `terraphim_router` with real prompts, asserting which capability was extracted -- no network. Layer 3 (`#[ignore]`): full pipeline against `liquid/lfm-2.5-1.2b-instruct:free` from OpenRouter. 1.06 seconds, $0.

The whole thing is open source. Full write-up at github.com/terraphim/terraphim-ai issue #1743.
