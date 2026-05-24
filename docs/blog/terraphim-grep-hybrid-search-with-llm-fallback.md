# Hybrid Code Search That Knows When to Ask an LLM

*Published 2026-05-24*

Terraphim Grep now does end-to-end hybrid search: ripgrep-fast code retrieval through
[fff-search](https://github.com/AlexMikhalev/fff.nvim), parallel knowledge-graph concept
extraction, and -- when the local retrievers do not return enough signal -- an LLM
synthesises a cited answer.

This post walks through the design decisions in the PR landing on
`task/1743-terraphim-grep`, why we did not write a single mock, and what the first
benchmark numbers tell us.

## The Problem We Hit

The existing code search pipeline answered "where does this string appear" well. It did
not answer "where is retry configured" -- a question that bridges a user's intent and the
code structure. A grep hit list helps only if the user already knows the symbol.

Our V-model verification phase surfaced two defects when we tried to claim completion:

- **D001:** the CLI binary instantiated `TerraphimGrep` without ever wiring an LLM client.
  Any query that landed in the `NeedsSynthesis` branch -- which is most of them, because
  the default sufficiency thresholds require both KG hits and chunk diversity -- hard-errored
  with `LlmNotConfigured`.
- **D005:** even when no LLM was available, the grep refused to return partial results.
  Search-only mode existed on paper but not in code.

Both defects existed because we had been testing the data structures, not the user-visible
behaviour. The verification report turned them up in the first 30 minutes.

## Why We Wired `build_llm_from_role`, Not `RouterBridgeLlmClient`

Terraphim ships a capability-based router in `terraphim_router`. Its keyword router
extracts a `Capability` (CodeGeneration, Explanation, DeepThinking, ...) from a prompt;
its strategies (CostOptimized, CapabilityFirst, LatencyOptimized) pick a provider from a
registry. `RouterBridgeLlmClient` wraps the router so that any consumer of the
`LlmClient` trait gets routing transparently.

There were two ways to plug grep into that:

1. Wire `RouterBridgeLlmClient` directly. Construct the provider registry inside grep.
2. Wire `terraphim_service::llm::build_llm_from_role(&role)`. Let it decide whether to
   return a direct provider or a `RouterBridgeLlmClient` based on `role.llm_router_enabled`.

We chose (2). `build_llm_from_role` already owns the precedence rules -- explicit
provider, nested extra, genai-with-model, OpenRouter config, Ollama hints -- and aligns
grep with how the server, TUI, and RLM consume providers. Whether routing kicks in is a
*role config* decision, not a *grep code* decision. Grep itself never knows whether it
got back an `OllamaClient` or a `RouterBridgeLlmClient`. It just sees `Arc<dyn LlmClient>`.

The wiring is six lines:

```rust
let llm = terraphim_service::llm::build_llm_from_role(&role);
let grep = TerraphimGrep::new(hybrid, judge);
let grep = match llm {
    Some(client) => grep.with_llm_client(client),
    None => grep,
};
```

Everything else -- which model, which routing strategy, which fallback -- lives in the
role JSON.

## Graceful Degradation Is a Feature

Before D005, calling grep without an LLM produced this:

```
Error: Search failed
Caused by: LLM not configured: LLM client not configured
```

After D005, the same call returns the chunks the local retrievers found:

```json
{
  "chunks": [
    { "content": "fn parse_grep_query(input: &str) -> String { ... }",
      "source": "sample.rs", "line_start": 1, "relevance_score": 1.0 }
  ],
  "sufficiency": "SearchOnly",
  "stats": { "search_latency_ms": 7, "chunks_returned": 8, "kg_hits": 0 }
}
```

The principle: if you have the data, return the data. Synthesis is the *enrichment*,
not the entry barrier. The CLI is now usable on a laptop with no API key and no Ollama.

`force_rlm = true` still fails fast when no LLM is configured -- if you explicitly asked
for synthesis, silently dropping back is worse than an error.

## No Mocks: Four Test Layers, All Real

The project's no-mocks rule (in `CLAUDE.md`: *"never use mocks in tests"*) forced a more
honest test pyramid:

- **L1 unit (inline, no network).** Prompt assembly, signature parsing, sufficiency
  routing decisions, and the new graceful-degrade path against a real `fff-search` scan
  of a tempdir corpus.
- **L2 router-capability (no network).** Feed real grep synthesis prompts to a real
  `terraphim_router::Router` with two registered providers. Assert which capability
  was extracted and which provider won. Three tests cover explanation queries, code
  implementation queries, and the no-keyword fallback. This catches the case where
  changing grep's prompt wording silently breaks routing.
- **L3 e2e smoke (`#[ignore]`, live OpenRouter free model).** Full
  `fff → KG → sufficiency → LLM → citations` against
  `liquid/lfm-2.5-1.2b-instruct:free` -- 1.2B parameters, sub-second responses, zero
  cost. Borrowed the `is_account_issue()` pattern from
  `docs/OPENROUTER_TESTING_PLAN.md` so 401/403/429 errors degrade to a skip rather than
  a failure.
- **L4 quality (`#[ignore]`, manual).** Same shape as L3 but pointed at
  `qwen/qwen3-coder:free` for stronger answers on the code-specialised use case. Run
  before release, not on every CI pass.

The free OpenRouter models we settled on:

| Capability target | Model | Reason |
|---|---|---|
| Code-aware synthesis | `qwen/qwen3-coder:free` | Code-specialised |
| Multi-chunk reasoning | `meta-llama/llama-3.3-70b-instruct:free` | Strong general |
| Fast CI smoke | `liquid/lfm-2.5-1.2b-instruct:free` | 1.2B params, lowest quota burn |

OpenRouter's free tier caps at 20 req/min and 200 req/day (1000 if you have ever deposited
$10), so we keep live tests behind `#[ignore]` and run one live call per capability rather
than repeating the same call in multiple tests.

What we explicitly did *not* do:

- No hand-rolled `MockLlmClient`. It would only prove the trait wiring, which
  `cargo check` already proves.
- No `wiremock`. The recorded responses go stale and the dependency adds nothing.
- No deterministic-temperature assertions on response text. Even at temperature 0, model
  upgrades break those tests.

## Benchmarks: Where Does the Time Go?

Three criterion groups in `crates/terraphim_grep/benches/hybrid_search.rs`:

- `code_only` -- fff-search alone, no KG, against 10/100/500 file corpora
- `hybrid_with_kg` -- parallel fff + KG concept extraction, varying the thesaurus from
  10 to 10,000 terms
- `fuse_and_rank` -- isolated sort/rank cost across chunk batches from 10 to 10,000

The first finding from `hybrid_with_kg`:

```
hybrid_with_kg/thesaurus_terms/10        3.2664 ms   30.6 Kelem/s
hybrid_with_kg/thesaurus_terms/100       3.4457 ms   29.0 Kelem/s
hybrid_with_kg/thesaurus_terms/1000      3.2413 ms   30.9 Kelem/s
hybrid_with_kg/thesaurus_terms/10000     3.2126 ms   31.1 Kelem/s
```

Hybrid latency stays flat at ~3.2 ms across three orders of magnitude of thesaurus size.
The parallel fff scan dominates wall-clock; the KG search is fast enough to vanish in
the noise. This is a useful baseline: if anyone proposes KG-pruning as a perf win, the
bench will tell them whether the optimisation actually moves the needle.

`fuse_and_rank` scales roughly linearly with chunk count, as expected for a comparator
sort:

```
fuse_and_rank/chunks/10      445 ns    22.4 Melem/s
fuse_and_rank/chunks/100     5.38 us   18.6 Melem/s
fuse_and_rank/chunks/1000    65.0 us   15.4 Melem/s
fuse_and_rank/chunks/10000   849 us    11.8 Melem/s
```

Throughput drops at scale because the sort fights cache locality, but absolute latency
stays well below the network round-trip budget.

## Your Knowledge Tops the Results

`fff-search` returns a uniform `relevance_score = 1.0` per match. Sorting by that score
is meaningless; without an ordering signal you would be back to "thirty hits, read them
all." We added a **KG-aware boost** so the chunks whose source path or content matches
your thesaurus concepts move to the top.

The shape:

```rust
pub fn score_kg_boost(chunk: &RetrievedChunk, concepts: &[KgConcept], weight: f64) -> f64;
pub fn boost_chunks_with_kg(chunks: Vec<RetrievedChunk>, concepts: &[KgConcept]) -> Vec<RetrievedChunk>;
```

For each chunk we lowercase the source path and content once, then for each matched
concept we check whether its `name` (or `display_value`) appears in either. Matching
concepts contribute their normalised score; unmatched ones do not. The boost is added
to the chunk's existing `relevance_score` and the JSON output shows the boosted number,
so downstream tools can see *why* a chunk ranked where it did.

Default weight is `1.0`: a fully-KG-matched chunk roughly doubles its score versus an
unmatched chunk. The unit tests pin this contract down:

```rust
#[test]
fn kg_boost_promotes_matching_chunks_to_top() {
    let chunks = vec![
        chunk("src/parse_csv.rs", "fn parse_csv() {}", 1.0),
        chunk("src/retry_policy.rs", "pub struct RetryPolicy {}", 1.0),
    ];
    let concepts = vec![concept("retry_policy", 0.9)];
    let ranked = boost_chunks_with_kg(chunks, &concepts);
    assert_eq!(ranked[0].source, "src/retry_policy.rs");
}
```

And the `kg_boost_overhead` benchmark group quantifies the cost. First numbers, 1000
chunks against varying concept counts:

```
kg_boost_overhead/concepts/0       49.7 us    sort only, no concepts
kg_boost_overhead/concepts/10      478 us
kg_boost_overhead/concepts/100     3.85 ms
kg_boost_overhead/concepts/1000    36.0 ms
```

The cost grows linearly with `chunks * concepts` (one substring search per pair). At
typical grep scale -- say 50 chunks and a handful of KG concepts matched per query --
the boost adds under 25 microseconds to a 3.2 ms hybrid search. Less than 1% overhead.

The 1000-concept case (36 ms) is pathological -- it would require every term in the
thesaurus to have matched the query, which does not happen in practice. The bench
exists so future regressions on the algorithm get caught.

## What Did Not Change

- The fff-search integration itself. It was already correct -- proven by `fff_search::file_picker`
  log lines on every run.
- The sufficiency judge thresholds. The defaults (min_coverage 0.7, min_kg_confidence 0.5,
  min_diversity 2, min_results 3) still steer most queries into NeedsSynthesis. Tuning
  those is a separate conversation -- the bench gives us the latency data to do it on.
- The fff-search integration itself.

## What This Unlocks

A grep that knows when it is not sure and asks for help. The CLI works without any LLM
configured (search-only mode). With OpenRouter or Ollama in the role config, it routes
the synthesis call by capability through the existing router infrastructure -- same
strategy as the rest of the platform. No new orchestration code, no new model
hardcoding, no mocks in the test pyramid.

The architectural payoff is small but real: every consumer of `LlmClient` in the
codebase now goes through the same `build_llm_from_role` entry point. When someone adds
a new provider (Claude, Gemini, a self-hosted model), it shows up in grep automatically.
When someone changes the routing strategy, it changes everywhere at once.

That is the test that matters: not "does this PR work" but "did we make the next PR
easier."

## Try It

```bash
# Search-only mode (no LLM required)
cargo build -p terraphim_grep --features code-search --release
./target/release/terraphim-grep "retry policy" --paths . \
  --thesaurus terraphim_server/fixtures/thesaurus_Default.json --json

# With OpenRouter synthesis
export OPENROUTER_API_KEY=sk-or-v1-...
export OPENROUTER_MODEL=qwen/qwen3-coder:free
cargo build -p terraphim_grep --features "code-search openrouter" --release
./target/release/terraphim-grep "retry policy" --paths . \
  --thesaurus terraphim_server/fixtures/thesaurus_Default.json --answer --json
```

The PR is [#1825](https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1825), tracking
Gitea issue [#1743](https://git.terraphim.cloud/terraphim/terraphim-ai/issues/1743).
