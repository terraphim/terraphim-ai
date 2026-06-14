# Revised Design: Full LLM Stack -- Ollama + OpenRouter with Capability Routing

**Status**: Draft (revised per user: all 4 items in scope)
**Research Doc**: `.docs/adf/2671/research-llm-client-wiring.md`
**Author**: opencode
**Date**: 2026-06-14

---

## Overview

Wire both Ollama (local, free, fast) and OpenRouter (cloud, free tier) into RLM, with capability-based routing so simple tasks hit cheap Ollama and complex tasks hit OpenRouter. This unblocks `rlm_query`, the deterministic-rlm-review skill, and the full RLM query loop.

### Providers

| Provider | Model | Cost | Latency | Capabilities |
|----------|-------|------|---------|-------------|
| Ollama local | `gemma3:270m` (cheapest auto-detected) | Free | ~100ms | FastThinking, CodeGeneration, Explanation, Documentation |
| OpenRouter | `meta-llama/llama-3.2-3b-instruct:free` | Free | ~500ms | DeepThinking, CodeGeneration, CodeReview, Architecture |

### Routing strategy

`CapabilityFirst` (`RouterStrategy::QualityFirst`): each `rlm_query` prompt is keyword-matched against provider capabilities. Simple code-gen goes to Ollama. Security audits, architecture reviews go to OpenRouter. Falls back to cheapest if no match.

### Scope (all 4 items included)

1. **Multi-provider routing**: `RouterBridgeLlmClient` with `CapabilityFirst` strategy
2. **OpenRouter**: API key from 1Password, free tier model
3. **genai crate**: Still skipped (redundant; direct Ollama + OpenRouter clients exist)
4. **deterministic-rlm-review**: Unblocked -- capability routing selects the right provider per reviewer role

## Updated Architecture

```
RLM startup
  │
  ├── auto_configure_llm()
  │     │
  │     ├── env overrides: RLM_PROVIDER, RLM_MODEL, RLM_ROUTER_STRATEGY
  │     │
  │     ├── Build OllamaClient (local)
  │     │     auto-detect cheapest chat model via /api/tags
  │     │     → gemma3:270m (292MB)
  │     │
  │     ├── Build OpenRouterClient (cloud)
  │     │     API key from 1Password or env var
  │     │     → meta-llama/llama-3.2-3b-instruct:free
  │     │
  │     └── Wrap in RouterBridgeLlmClient
  │           strategy: CapabilityFirst (QualityFirst)
  │           register both providers with capability profiles
  │
  └── set_llm_client(router_bridge)
        ↓
  rlm_query("audit for SQL injection...") → keyword "security"
    → RouterBridgeLlmClient::resolve_client()
      → "security" ∉ Ollama caps, ∈ OpenRouter caps → OpenRouter
      → POST api.openrouter.ai/v1/chat/completions
    → real LLM response
```

## Provider capability matrix

```
Prompt keywords → Capability mapping:

"audit security vulnerability exploit injection" → DeepThinking  → OpenRouter
"review code correctness edge case pattern"     → CodeReview    → OpenRouter
"analyse hot path allocation performance"       → FastThinking   → Ollama
"evaluate API design breaking change"           → Architecture   → OpenRouter
"implement function write code generate"        → CodeGeneration → Ollama (cheaper)
"explain document describe"                     → Explanation    → Ollama
```

## File changes

| File | Change |
|------|--------|
| `Cargo.toml` | Add `terraphim_config` dep behind `llm` feature; add `ollama` feature to `terraphim_service` |
| `rlm.rs` | Rewrite `auto_configure_llm()`: build dual clients, wrap in RouterBridgeLlmClient |
| `main.rs` | Call `auto_configure_llm()` in `run()` |

## Implementation notes

### Building the OpenRouter client

The `build_llm_from_role()` factory needs `role.extra["llm_provider"] = "openrouter"` and `role.extra["llm_model"] = "meta-llama/llama-3.2-3b-instruct:free"`. The API key must be available in `role.extra["llm_api_key"]` or via the `OPENROUTER_API_KEY` env var (which `build_openrouter_from_role` reads).

### Building the router bridge

`build_llm_from_role()` already has code that constructs `RouterBridgeLlmClient` when `role.llm_router_enabled = true` (`llm.rs:200-231`). We just need to set the right fields on the role:

```rust
role.llm_router_enabled = true;
role.llm_router_config = Some(LlmRouterConfig {
    enabled: true,
    strategy: RouterStrategy::QualityFirst,
    mode: RouterMode::Library,
    ..Default::default()
});
```

This triggers the code path that:
1. Builds both Ollama and OpenRouter clients
2. Wraps them in `RouterBridgeLlmClient` with `strategy_from_config(&RouterStrategy::QualityFirst)` (= `CapabilityFirst`)
3. Returns the bridge as `Arc<dyn LlmClient>`

### API key retrieval

From 1Password at startup: `op read "op://OdiloVault/OpenRouterTesting-api-key/password"`. Cache in env var to avoid repeated 1Password calls.

## Test strategy

| Test | Purpose |
|------|---------|
| `test_auto_configure_dual_providers` | Both Ollama and OpenRouter clients built |
| `test_routing_codegen_to_ollama` | "Write a function" → Ollama (CodeGeneration, cheaper) |
| `test_routing_security_to_openrouter` | "Audit for SQL injection" → OpenRouter (DeepThinking) |
| `test_rlm_query_returns_real_response` | `rlm_query` returns non-stub text |
| `test_ollama_unreachable_falls_back` | OpenRouter used when Ollama down |
