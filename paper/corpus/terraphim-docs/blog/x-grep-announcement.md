# X/Twitter Thread -- Terraphim Grep

## Tweet 1

You search a 1M-line codebase for "where is retry configured."

Ripgrep prints 30 hits. None obviously right. You read all 30.

Five minutes gone.

We built grep that knows when its results are not good enough -- and only then calls an LLM. Thread.

## Tweet 2

terraphim_grep runs three things in parallel:

- fff-search (ripgrep-style code scan)
- Knowledge graph concept extraction
- Sufficiency judge: coverage + diversity + KG confidence

If the judge says "enough", you get chunks. If not, an LLM synthesises a cited answer.

No LLM call when not needed.

## Tweet 3

The wiring uses Terraphim's existing `build_llm_from_role` -- same entry point the server, TUI, and RLM use.

```rust
let llm = build_llm_from_role(&role);
let grep = TerraphimGrep::new(hybrid, judge);
let grep = match llm {
    Some(c) => grep.with_llm_client(c),
    None => grep,  // search-only mode
};
```

Six lines. Grep never knows which provider it got.

## Tweet 4

The "no LLM" path is a first-class state. CLI works with no API key, returns the chunks it has, exits clean.

Synthesis is enrichment. It is not the entry barrier.

Before this fix, the CLI hard-errored with "LLM not configured" on any partial result. Now it degrades gracefully.

## Tweet 5

We tested it with zero mocks. Project rule.

L1: inline unit + real fff scan against a tempdir
L2: real Router + real grep prompts, assert which capability extracted -- no network
L3 (#[ignore]): full pipeline against liquid/lfm-2.5-1.2b-instruct:free on OpenRouter
L4 (manual): qwen/qwen3-coder:free for code quality

## Tweet 6

L3 cost per run: $0. Latency: 1.06s.

Free OpenRouter models we settled on:
- qwen/qwen3-coder:free -- code-specialised
- meta-llama/llama-3.3-70b-instruct:free -- strong reasoning
- liquid/lfm-2.5-1.2b-instruct:free -- 1.2B params, fast smoke

Cap: 20 req/min, 200 req/day on free tier.

## Tweet 7

First benchmark surprise:

hybrid latency stays flat at ~3.2ms across thesaurus sizes 10 .. 10,000 terms.

Parallel `tokio::spawn` fff scan dominates wall-clock. KG search vanishes in the noise.

If anyone proposes KG-pruning as a perf win, the bench will tell them if it matters.

## Tweet 8

Open source. Rust. Workspace at github.com/terraphim/terraphim-ai.

PR with the full V-model verification report, four-layer test design, and bench numbers at issue #1743.

Best part: when someone adds a new LLM provider, grep picks it up automatically. That is the test that matters.
