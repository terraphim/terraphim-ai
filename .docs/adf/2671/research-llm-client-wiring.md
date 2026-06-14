# Research Document: Wire LLM Client into RLM with Smart Cheapest-Model Routing

**Status**: Draft
**Author**: opencode
**Date**: 2026-06-14

## Executive Summary

`terraphim_rlm`'s `LlmBridge` correctly returns `LlmNotConfigured` when no LLM client is injected. The existing `terraphim_service::llm::build_llm_from_role()` factory can construct clients (Ollama, OpenRouter, genai) but requires a `terraphim_config::Role`. The orchestrator has `CostFirst` routing at the provider level but not at the model level. We need to: (1) add `terraphim_config` as an optional dependency, (2) build a role config from environment/runtime detection, (3) select the cheapest available model within each provider, and (4) inject the client into RLM.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Unblocks `rlm_query`, the deterministic-rlm-review skill, and the full RLM query loop |
| Leverages strengths? | Yes | Reuses existing `build_llm_from_role()`, `RouterBridgeLlmClient`, Ollama provider |
| Meets real need? | Yes | `rlm_query` currently returns `LlmNotConfigured`; without this, RLM cannot do LLM-based work |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
RLM's LLM bridge has an injection point (`set_llm_client()`) but no auto-configuration mechanism. The CLI binary starts without an LLM client, making the `query` command always return `LlmNotConfigured`. The deterministic-rlm-review skill and the full RLM query loop both require real LLM calls.

### Impact
- `rlm_query` non-functional
- Deterministic-rlm-review skill cannot spawn reviewers
- RLM query loop cannot do recursive LLM work
- Users must manually configure providers

### Success Criteria
1. `terraphim_rlm query` returns real LLM responses, not `LlmNotConfigured`
2. Smart routing selects fastest/cheapest available model automatically
3. No new external dependencies beyond what's already in the workspace
4. Falls back gracefully when no providers are available

## Current State Analysis

### Existing Implementation

| Component | Location | Purpose |
|-----------|----------|---------|
| `LlmBridge` | `crates/terraphim_rlm/src/llm_bridge.rs` | VM-to-host LLM gateway with `set_llm_client()` |
| `TerraphimRlm::set_llm_client()` | `crates/terraphim_rlm/src/rlm.rs:877` | Injection point for `Arc<dyn LlmClient>` |
| `LlmClient` trait | `terraphim_service::llm` (registry) | Canonical LLM abstraction (`chat_completion`, `summarize`, `list_models`) |
| `OllamaClient` | `terraphim_service::llm` (private) | Full Ollama implementation with `/api/chat`, `/api/tags` |
| `build_llm_from_role()` | `terraphim_service::llm` (public) | Factory that builds `Arc<dyn LlmClient>` from `Role` config |
| `RouterBridgeLlmClient` | `terraphim_service::llm::bridge` | Capability-based routing across providers with `CostFirst`/`QualityFirst`/`Balanced`/`Static` |
| `RouterStrategy::CostFirst` | `terraphim_config::llm_router` | Picks provider with lowest `CostLevel` (Cheap < Moderate < Expensive) |
| `Role` | `terraphim_config::Role` | Holds `llm_enabled`, `extra` map with `llm_provider`, `llm_model`, `ollama_base_url`, etc. |

### Data Flow (current)
```
CLI → TerraphimRlm::new(config) → LlmBridge::new() (no client)
                                    ↓
                              rlm.query(prompt)
                                    ↓
                              LlmBridge::query() → LlmNotConfigured
```

### Integration Points
- `TerraphimRlm::set_llm_client(client: Arc<dyn LlmClient>)` -- mutable access required
- `build_llm_from_role(role: &Role) -> Option<Arc<dyn LlmClient>>` -- needs `terraphim_config::Role`
- Ollama `/api/tags` -- returns `{ "models": [{ "name": "...", "size": 12345, ... }] }`
- OpenRouter API key -- `OPENROUTER_API_KEY` env var

### Local Ollama models (available)

| Model | Size | Chat-capable? |
|-------|------|---------------|
| `gemma3:270m` | 292 MB | Yes (fastest) |
| `all-minilm:latest` | 46 MB | No (embedding) |
| `deepseek-coder:1.3b` | 776 MB | Yes |
| `gemma2:2b` | 1.6 GB | Yes |
| `llama3.2:3b` | 2.0 GB | Yes |
| `qwen3:4b` | 2.5 GB | Yes |
| `qwen2.5-coder:latest` | 4.7 GB | Yes |
| `qwen3:8b` | 5.2 GB | Yes |

## Constraints

### Vital Few (Essentialism)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Must use `build_llm_from_role()` | Only public factory for `Arc<dyn LlmClient>`; `OllamaClient` is private | Source: `terraphim_service::llm.rs:78` |
| Must add `terraphim_config` as dep | `Role` type required by factory | Not currently in RLM deps (dev-only) |
| Must select cheapest model at runtime | Statically listing models is fragile; user may pull/unpull models | Ollama `/api/tags` returns sizes |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Writing own `LlmClient` impl from scratch | `OllamaClient` already exists and is battle-tested |
| Multi-provider routing in `RouterBridgeLlmClient` | Single Ollama provider is sufficient; routing adds complexity for no gain right now |
| OpenRouter support in this PR | Needs API key; Ollama is local and free |
| genai provider support | Adds `genai` crate dependency; not needed for cheapest path |
| Model quality-based selection | User asked for cheapest/fastest, not best quality |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_config` (new dep) | Must add as optional dep behind `llm` feature | Low -- same feature gate as `terraphim_service` |
| `terraphim_service` (existing) | Used via `build_llm_from_role()` | Low -- already a dep behind `llm` feature |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Ollama daemon | local | Must be running for auto-detect to work | Falls back to `LlmNotConfigured` |
| `reqwest` | workspace | Used by `terraphim_service` internally; needed for `/api/tags` query to find cheapest model | Could skip cheapest detection if model is hardcoded |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `build_llm_from_role` fails silently if role config incomplete | Medium | Medium | Test with real config; validate before injecting |
| Ollama `/api/tags` unreachable | Low | Low | Fallback to config-specified or default model |
| Non-chat models selected (e.g. `all-minilm`) | Low | Medium | Filter to chat-capable models only (heuristic: min 100MB, exclude embedding models) |
| `terraphim_config` transitively already present | Medium | Low | Cargo resolves correctly either way |

### Open Questions
1. Should we filter to chat-capable models only, or let user handle via config? -- We should filter; non-chat models fail at inference time.
2. Should env vars (`RLM_PROVIDER`, `RLM_MODEL`) override auto-detection? -- Yes, for explicit control.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Ollama models with `gemma`/`qwen`/`llama` prefix are chat-capable | Model family naming convention | Non-chat model selected, inference error | No |
| `all-minilm` can be excluded by name pattern | Known embedding model | Similar non-chat models missed | No |
| `terraphim_config` is already a transitive dep through `terraphim_service` | Common Rust pattern | Need explicit dep declaration | Will verify |
| `build_llm_from_role()` is available when `terraphim_service` feature `ollama` is enabled | Cargo feature resolution | Function not found at compile time | Must verify |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Hardcode `gemma3:270m` as model | Simple, no runtime detection | Rejected: fragile if model pulled/renamed |
| Query `/api/tags` at RLM startup, pick smallest model | Dynamic, always uses best available | Chosen: smart, resilient |
| Use `RouterBridgeLlmClient` with multiple models as "providers" | Multi-model routing | Rejected: adds complexity, models aren't separate providers |
| Let user configure via `RLM_MODEL=gemma3:270m` env var | Explicit, predictable | Chosen as fallback alongside auto-detection |

## Research Findings

### Key Insights

1. **`build_llm_from_role()` is the gateway**: It reads `role.extra["llm_provider"]` and `role.extra["llm_model"]` to construct clients. Setting these to `"ollama"` and any valid model name produces a working `Arc<dyn LlmClient>`.

2. **`CostFirst` routing at provider level, not model level**: The existing `RouterBridgeLlmClient` routes between PROVIDERS (Ollama vs OpenRouter) using cost levels. Within a single provider, model selection is static. For intra-provider model selection, we need custom logic.

3. **Ollama `/api/tags` returns sizes**: The API response includes `size` (bytes) per model. We can query this, find the smallest chat-capable model, and use it.

4. **Cheapest chat models**: `gemma3:270m` (292MB), `deepseek-coder:1.3b` (776MB), `gemma2:2b` (1.6GB). `all-minilm` (46MB) should be excluded as embedding-only.

5. **RLM CLI already uses `RlmConfig::default()`**: The CLI binary has no provider configuration. We add auto-detection in `run()` after RLM construction.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Verify `terraphim_config` compiles as dep of `terraphim_rlm` | Confirm no version conflicts | 5 min |

## Recommendations

### Proceed/No-Proceed
**Proceed**. Risk is low, reuse of existing infrastructure is high, user need is clear.

### Implementation Approach (3 tiers)

**Tier 1 -- Env var override (always respected)**:
- `RLM_PROVIDER=ollama` + `RLM_MODEL=gemma3:270m` → explicit control
- `OPENROUTER_API_KEY` present → auto-select OpenRouter with cheapest model

**Tier 2 -- Ollama auto-detection (if env vars absent)**:
- `GET http://127.0.0.1:11434/api/tags` → parse models
- Filter to chat-capable (exclude embedding/vision-only models)
- Select model with smallest `size` field
- If Ollama unreachable, fall through

**Tier 3 -- Hardcoded fallback**:
- `gemma3:270m` as last resort default (guaranteed chat-capable)

### Files to modify

| File | Change |
|------|--------|
| `crates/terraphim_rlm/Cargo.toml` | Add `terraphim_config = { path = "../terraphim_config", optional = true }`; add to `llm` feature |
| `crates/terraphim_rlm/src/rlm.rs` | Add `auto_configure_llm()` method with 3-tier detection |
| `crates/terraphim_rlm/src/main.rs` | Call `auto_configure_llm()` after `TerraphimRlm::new()` |
| `crates/terraphim_rlm/src/error.rs` | No changes (reuse existing `LlmNotConfigured`) |

## Appendix

### Ollama `/api/tags` response format
```json
{
  "models": [
    { "name": "gemma3:270m", "model": "gemma3:270m", "size": 306651136, "digest": "...", "details": { "parent_model": "", "format": "gguf", "family": "gemma3", "parameter_size": "268.19M", "quantization_level": "Q8_0" } },
    { "name": "gemma2:2b", "model": "gemma2:2b", "size": 1675000000, "digest": "...", "details": { "family": "gemma2", "parameter_size": "2.6B" } }
  ]
}
```

### Key: non-chat models to exclude
- `all-minilm` -- embedding model (Bert architecture)
- `nomic-embed-text` -- embedding model
- Any model with `embed` in name

### Key: `build_llm_from_role()` minimal invocation
```rust
let mut extra = AHashMap::new();
extra.insert("llm_provider".to_string(), serde_json::Value::String("ollama".to_string()));
extra.insert("llm_model".to_string(), serde_json::Value::String("gemma3:270m".to_string()));

let role = Role {
    name: "rlm-auto".to_string(),
    llm_enabled: true,
    extra,
    ..Default::default()  // NOTE: some fields may need explicit values
};
let client = terraphim_service::llm::build_llm_from_role(&role);
```
