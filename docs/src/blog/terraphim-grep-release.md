# Grep That Knows When to Ask an LLM

You search a million-line codebase for "where is retry configured." Ripgrep prints thirty hits. None of them are obviously the right one. You read all thirty. Five minutes gone.

Terraphim Grep does something different. It runs an [fff-search](https://github.com/AlexMikhalev/fff.nvim) code scan and a knowledge-graph concept lookup in parallel. A sufficiency judge looks at coverage, diversity, and KG confidence, and decides whether the local retrievers found enough signal. If they did, you get the chunks. If they did not, an LLM synthesises a cited answer.

The CLI works in three modes, picked entirely from environment variables and role config -- no code changes needed:

```bash
# Search-only -- no LLM, no API key required
./terraphim-grep "retry policy" --paths . --thesaurus thesaurus.json --json

# OpenRouter free model -- export key, run with --answer
export OPENROUTER_API_KEY=sk-or-v1-...
export OPENROUTER_MODEL=qwen/qwen3-coder:free
./terraphim-grep "retry policy" --paths . --thesaurus thesaurus.json --answer --json

# Local Ollama
export OLLAMA_BASE_URL=http://localhost:11434
./terraphim-grep "retry policy" --paths . --thesaurus thesaurus.json --answer --json
```

The LLM wiring goes through `terraphim_service::llm::build_llm_from_role` -- the same entry point the server, TUI, and RLM use. Whether routing through capability extraction kicks in is a *role config* decision (`llm_router_enabled = true`), not something grep itself knows about. Every consumer of the `LlmClient` trait now goes through one place.

## What is new in this release

- End-to-end hybrid pipeline: fff-search code chunks + KG concepts + sufficiency judging + LLM synthesis with citations.
- Graceful degradation: no LLM configured? You still get chunks. `force_rlm = true` still fails fast.
- Four-layer test pyramid, zero mocks: inline unit tests, router capability assertions, live OpenRouter free-model smoke (`#[ignore]`), and a manual quality gate.
- Criterion benchmarks for `code_only`, `hybrid_with_kg`, and `fuse_and_rank`. First numbers: hybrid latency flat at ~3.2 ms across thesaurus sizes 10..10,000.

Full design rationale and benchmark walkthrough in the [long-form post](https://github.com/terraphim/terraphim-ai/blob/main/docs/blog/terraphim-grep-hybrid-search-with-llm-fallback.md).
