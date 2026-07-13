# OpenRouter model trade-off for `terraphim-grep` RLM fallback

Date: 2026-07-13

## Goal

Pick a cheap, high-throughput OpenRouter model for `terraphim-grep --answer` RLM
fallback, and compare it with `deepseek/deepseek-v4-flash`.

## Methodology

- Query: `fn main`
- Target path: `crates/terraphim_rlm/src`
- Command: `terraphim-grep <query> --haystack code --paths <path> --answer --json`
- Per-model timeout: 45 s
- Pricing and context length fetched live from `https://openrouter.ai/api/v1/models`.
- Models were selected to cover a range of sizes, prices, and providers.

## Results

| Model | Params | Prompt $/M | Completion $/M | Context | RLM latency | Answer quality |
|---|---|---|---|---|---|---|
| `amazon/nova-lite-v1` | — | $0.060 | $0.240 | 300 k | 1.6 s | **No output** (null) |
| `meta-llama/llama-3.2-1b-instruct` | 1 B | $0.027 | $0.201 | 128 k | 1.8 s | **No output** (null) |
| `meta-llama/llama-3.2-3b-instruct` | 3 B | $0.050 | $0.330 | 128 k | 2.4 s | **No output** (null) |
| `deepseek/deepseek-v4-flash` | — | $0.090 | $0.180 | 1 M | 5.6 s | **No output** (null) |
| `amazon/nova-micro-v1` | — | $0.035 | $0.140 | 128 k | 11.0 s | Good, cited synthesis |
| `google/gemma-3-4b-it` | 4 B | $0.050 | $0.100 | 128 k | 12.3 s | **No output** (null) |
| `mistralai/mistral-nemo` | — | $0.020 | $0.030 | 128 k | 31.4 s | Good, cited synthesis |
| `qwen/qwen3-coder:free` | — | $0.000 | $0.000 | 1 M | 0.5 s | Rate limit exceeded |

### Sample successful answers

`amazon/nova-micro-v1`:

> The provided context shows multiple instances of an `async fn main()` function
> with a return type of `Result<(), Box<dyn std::error::Error>>`. This indicates
> that the `main` function in various modules (such as `rlm.rs`, `lib.rs`, and
> `executor/mod.rs`) is asynchronous and designed to handle potential errors by
> returning a `Result` ...

`mistralai/mistral-nemo`:

> The function `main` is defined multiple times in your Rust codebase. It's
> declared as `async` and returns a `Result` with an error type of
> `Box<dyn std::error::Error>`. Here are the locations where it's defined: ...

## Trade-off analysis

- **Cheapest usable model**: `mistralai/mistral-nemo` at ~$0.020 / M prompt
  tokens, but it is the slowest successful model (~31 s).
- **Fastest usable model**: `amazon/nova-micro-v1` at ~11 s, with a good
  synthesis. It is also very cheap (~$0.035 / M prompt tokens).
- **`deepseek/deepseek-v4-flash`**: in this test it returned no output through
  the `terraphim-grep` path, despite working correctly when called directly via
  the OpenRouter chat completions endpoint. It is also more expensive than the
  successful small models.
- **Small models (1 B–4 B)**: fast, but most produced empty answers with the
  current prompt format, suggesting they are too constrained for the RLM
  fallback prompt.
- **Free models**: cheapest ($0), but the tested free model hit OpenRouter's
  free-tier rate limit immediately.

## Recommendations

1. **Default fast+cheap choice for `terraphim-grep --answer`:**
   `amazon/nova-micro-v1`.
   ```bash
   export OPENROUTER_MODEL=amazon/nova-micro-v1
   ```
2. **If latency is less important than price**: `mistralai/mistral-nemo`.
3. **Avoid free models for interactive use** unless you can tolerate rate-limit
   retries.
4. **Do not assume a model works because it works via raw OpenRouter API**;
   always validate through `terraphim-grep --answer` because the prompt/template
  packing in the RLM path can cause some models to return empty output.

## Raw data

Full results are in `scripts/benchmark_openrouter_tradeoff/results-2026-07-13.json`.

## Reproduction

Run the benchmark script:

```bash
python3 scripts/benchmark_openrouter_tradeoff/benchmark.py \
  --query "fn main" \
  --paths crates/terraphim_rlm/src
```

The script fetches live pricing from OpenRouter, runs each model through
`terraphim-grep --answer`, and emits a markdown table.
