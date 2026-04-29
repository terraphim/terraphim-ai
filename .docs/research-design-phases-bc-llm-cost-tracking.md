# Research & Design: LLM Cost Tracking -- Phases B+C

## 1. Problem Restatement and Scope

### Problem
Phase A delivered `LlmUsage` types, `chat_completion_with_usage()`, and the `GenAiClient` adapter. Usage metadata now flows from API responses back to callers. However:

- **Pricing is still hardcoded** in two disconnected systems (`terraphim_multi_agent::tracking::ModelPricing` with per-1K pricing for 3 models, `terraphim_types::llm_usage::ModelPricing` with per-million pricing and model patterns)
- **`TokenUsageTracker` has zero persistence** -- all records lost on process exit. No `flush()` method exists.
- **`token_tracker` is never written to in production code** -- `record_usage()` only called from tests
- **3 of 5 providers are stubs** (Claude, OpenCode Go, Kimi return "Not yet implemented")
- **CLI `handle_usage()` creates an empty `UsageRegistry`** -- no providers registered, so `usage show` returns nothing
- **`terraphim_usage` does not depend on `terraphim_ccusage`** -- Claude provider can't access ccusage data
- **`terraphim_multi_agent` does not depend on `terraphim_usage`** -- no bridge exists

### IN Scope (Phases B+C)
- Configurable pricing module (TOML file + embedded defaults)
- Wire `TokenUsageTracker` to persist records into `UsageStore`
- Complete Claude provider (via ccusage)
- Complete OpenCode Go provider (via SQLite)
- Add ccusage as dependency of terraphim_usage and create ccusage adapter provider
- Register all providers in CLI `handle_usage()`
- Keep Kimi as stub (API unknown, low priority)

### OUT of Scope
- Phase D CLI enhancements (`usage history --by model`, `usage alert --budget N`) -- separate PR
- Web dashboard
- Rate limiting based on costs
- Kimi provider implementation (Moonshot API undocumented)

## 2. System Elements and Dependencies

### Current State After Phase A

| Element | Crate | Status | Notes |
|---|---|---|---|
| `LlmUsage` | `terraphim_types` | DONE | input/output tokens, model, provider, cost_usd, latency_ms |
| `LlmResult` | `terraphim_types` | DONE | content + optional LlmUsage |
| `ModelPricing` (types) | `terraphim_types` | DONE | per-million pricing, model_pattern |
| `ModelPricing` (multi_agent) | `terraphim_multi_agent` | LEGACY | per-1K pricing, 3 hardcoded models, to be replaced |
| `chat_completion_with_usage()` | `terraphim_service` | DONE | GenAiClient extracts usage |
| `ExecutionRecord::from_llm_usage()` | `terraphim_usage` | DONE | Bridge from LlmUsage to persistent record |
| `UsageStore::save_execution()` | `terraphim_usage` | DONE | Persists ExecutionRecord |
| `UsageProvider` trait | `terraphim_usage` | DONE | fetch_usage() -> ProviderUsage |
| MiniMax provider | `terraphim_usage` | DONE | Full API integration |
| ZAI provider | `terraphim_usage` | DONE | Full API integration |
| Claude provider | `terraphim_usage` | STUB | Returns error |
| OpenCode Go provider | `terraphim_usage` | STUB | Returns error |
| Kimi provider | `terraphim_usage` | STUB | Returns error |
| `CcusageClient` | `terraphim_ccusage` | DONE | Shells out to `bun dlx ccusage`, returns DailyUsageReport |
| `TokenUsageTracker` | `terraphim_multi_agent` | IN-MEMORY ONLY | No persistence |
| `CostTracker` | `terraphim_multi_agent` | IN-MEMORY ONLY | Hardcoded pricing |
| `handle_usage()` | `terraphim_cli` | BROKEN | Empty registry, no providers registered |

### Dependency Graph (Desired)

```
terraphim_types (LlmUsage, ModelPricing)
    ^           ^
    |           |
terraphim_service    terraphim_usage ----> terraphim_ccusage (new dep)
(return usage)       (persist usage)       (Claude token data)
    |                   |
    v                   v
terraphim_multi_agent ---> terraphim_usage (new dep)
(flush TokenUsageTracker)  (persist records)
```

### Key Constraint: terraphim_multi_agent adding terraphim_usage dependency
Currently `terraphim_multi_agent` does NOT depend on `terraphim_usage`. Adding this dep is safe (no circular risk) since `terraphim_usage` depends only on `terraphim_types`, `terraphim_persistence`, `terraphim_settings`, and `opendal` -- none of which create cycles back to `terraphim_multi_agent`.

## 3. Constraints

| Constraint | Implication |
|---|---|
| No circular deps | terraphim_usage cannot depend on terraphim_multi_agent (safe: it doesn't) |
| Fire-and-forget persistence | flush_to_store must not block LLM calls |
| Sub-cent precision | ExecutionRecord uses cost_sub_cents (1/1,000,000 cent) |
| Feature flags | providers gated behind `providers` feature in terraphim_usage |
| ccusage requires bun/pnpm | Claude provider graceful degradation if runner missing |
| SQLite dependency for OpenCode Go | Need rusqlite or shell out to sqlite3 CLI |
| Existing test suite must pass | 128+ tests, clippy clean |

## 4. Risks and Unknowns

| Risk | Likelihood | Mitigation |
|---|---|---|
| Adding terraphim_usage dep to multi_agent causes compile issues | Low | Check dep graph first; terraphim_usage only pulls in terraphim_types/persistence |
| ccusage not installed / bun missing | Medium | Return ProviderNotFound gracefully, same as OpenCode Go pattern |
| OpenCode SQLite schema differs from assumed | Medium | Query sqlite_master first; handle missing columns gracefully |
| Two ModelPricing types confuse callers | High | Unify: use terraphim_types::ModelPricing everywhere, deprecate multi_agent version |
| flush_to_store loses data on crash | Low | Acceptable trade-off per design; records accumulate in memory until flush |

### Assumptions
- [ASSUMPTION] OpenCode Go SQLite at `~/.local/share/opencode/opencode.db` has a `message` table with cost fields
- [ASSUMPTION] ccusage@18.0.10 JSON output format is stable (DailyUsageReport already handles it)
- [ASSUMPTION] Kimi/Moonshot usage API is not publicly documented -- keep as stub

## 5. Design: Step-by-Step Plan

### Step B1: Configurable Pricing Module

**File**: `crates/terraphim_usage/src/pricing.rs` (NEW)

Create `PricingTable` struct:
- `entries: Vec<terraphim_types::ModelPricing>` -- reuses existing type
- `load(path: &Path) -> Result<Self>` -- reads TOML, falls back to embedded defaults
- `embedded_defaults() -> Self` -- comprehensive default pricing for 20+ models
- `find_pricing(model: &str) -> Option<&ModelPricing>` -- glob/substring match on model_pattern
- `calculate_cost(model: &str, input: u64, output: u64) -> Option<f64>` -- convenience wrapper

TOML format:
```toml
[[models]]
pattern = "openai/gpt-4o*"
input_per_m = 2.50
output_per_m = 10.00

[[models]]
pattern = "anthropic/claude-sonnet*"
input_per_m = 3.00
output_per_m = 15.00
```

Config file location: `~/.config/terraphim/pricing.toml` (optional, falls back to embedded)

Also add `toml` dependency to terraphim_usage Cargo.toml.

### Step B2: TokenUsageTracker flush_to_store

**File**: `crates/terraphim_multi_agent/src/tracking.rs` (MODIFY)

Add to `TokenUsageTracker`:
```rust
pub fn drain_records(&mut self) -> Vec<TokenUsageRecord> {
    std::mem::take(&mut self.records)
    // Keep totals intact for in-memory queries
}
```

**File**: `crates/terraphim_multi_agent/src/agent.rs` (MODIFY)

Add a `flush_usage` method to `TerraphimAgent` that:
1. Takes `self.token_tracker.write().drain_records()`
2. Converts each `TokenUsageRecord` into `LlmUsage` (already has all fields)
3. Creates `ExecutionRecord::from_llm_usage()` for each
4. Calls `UsageStore::save_execution()` for each
5. Fire-and-forget via `tokio::spawn`

Requires: Add `terraphim_usage` dependency to `terraphim_multi_agent/Cargo.toml`

### Step B3: Wire Production Usage Recording

**File**: `crates/terraphim_multi_agent/src/genai_llm_client.rs` (MODIFY)

After each `exec_chat()` call that returns usage metadata, record it into `token_tracker`:
- Extract usage from genai response
- Create `TokenUsageRecord`
- Call `token_tracker.write().record_usage(record)`

This is the missing production write path -- currently only tests call `record_usage()`.

### Step C1: Add ccusage Dependency + Adapter Provider

**File**: `crates/terraphim_usage/Cargo.toml` (MODIFY)
- Add `terraphim_ccusage = { path = "../terraphim_ccusage", optional = true }`
- Add to `providers` feature: `"dep:terraphim_ccusage"`

**File**: `crates/terraphim_usage/src/providers/ccusage.rs` (NEW)
- `CcusageProvider` struct wrapping `terraphim_ccusage::CcusageClient`
- Implements `UsageProvider` trait
- `fetch_usage()` queries ccusage for last 30 days
- Builds `MetricLine::Progress` entries for daily/weekly/monthly spend
- `id() -> "ccusage"`, `display_name() -> "Claude Code (ccusage)"`

**File**: `crates/terraphim_usage/src/providers/mod.rs` (MODIFY)
- Add `#[cfg(feature = "providers")] pub mod ccusage;`

### Step C2: Complete Claude Provider

**File**: `crates/terraphim_usage/src/providers/claude.rs` (MODIFY)

Two-layer approach:
1. Primary: Read OAuth token from `~/.claude/.credentials.json`, call Anthropic usage API
2. Fallback: If OAuth fails or API unavailable, delegate to ccusage adapter

Since Anthropic's OAuth usage API (`/api/oauth/usage`) endpoint and response format are not publicly documented, implement via ccusage as primary method for now. The Claude provider will:
- Create internal `CcusageClient::new(CcusageProvider::Claude)`
- Query last 7 days of usage
- Build `MetricLine::Progress` for daily and weekly windows
- Add `terraphim_ccusage` as internal dependency

### Step C3: Complete OpenCode Go Provider

**File**: `crates/terraphim_usage/src/providers/opencode_go.rs` (MODIFY)

SQLite query approach (avoid rusqlite heavy dep):
- Shell out to `sqlite3` CLI to query the database
- `SELECT role, COUNT(*), SUM(cost) FROM message GROUP BY role`
- Parse TSV output
- If `sqlite3` not available, return ProviderNotFound
- Build `MetricLine::Progress` entries

Alternative: Use `rusqlite` with `bundled-sqlite` feature. This is more robust but adds a heavy C dependency. Decision: use `sqlite3` CLI shell-out for now (matches ccusage pattern of shelling out).

### Step C4: Register All Providers in CLI

**File**: `crates/terraphim_cli/src/main.rs` (MODIFY)

In `handle_usage()`, register all available providers:
```rust
let mut registry = UsageRegistry::new();
registry.register(Box::new(ClaudeProvider::new()));
registry.register(Box::new(OpenCodeGoProvider::new()));
registry.register(Box::new(MiniMaxProvider::new()));
registry.register(Box::new(ZaiProvider::new()));
registry.register(Box::new(CcusageProvider::new()));
```

This fixes the "empty registry" bug where `usage show` returns nothing.

### Step C5: Keep Kimi as Stub

No changes -- Kimi/Moonshot usage API is undocumented. Leave as-is with clear TODO.

## 6. File Change Summary

| File | Action | Purpose |
|---|---|---|
| `terraphim_usage/src/pricing.rs` | CREATE | Configurable pricing from TOML + defaults |
| `terraphim_usage/Cargo.toml` | MODIFY | Add `toml` dep, add `terraphim_ccusage` dep |
| `terraphim_usage/src/lib.rs` | MODIFY | Add `pub mod pricing;` |
| `terraphim_usage/src/providers/ccusage.rs` | CREATE | CcusageProvider wrapping CcusageClient |
| `terraphim_usage/src/providers/claude.rs` | MODIFY | Implement via ccusage |
| `terraphim_usage/src/providers/opencode_go.rs` | MODIFY | Implement via sqlite3 CLI |
| `terraphim_usage/src/providers/mod.rs` | MODIFY | Add ccusage module |
| `terraphim_multi_agent/Cargo.toml` | MODIFY | Add `terraphim_usage` dependency |
| `terraphim_multi_agent/src/tracking.rs` | MODIFY | Add `drain_records()` method |
| `terraphim_multi_agent/src/agent.rs` | MODIFY | Add `flush_usage()` method |
| `terraphim_multi_agent/src/genai_llm_client.rs` | MODIFY | Record usage after each LLM call |
| `terraphim_cli/src/main.rs` | MODIFY | Register all providers in handle_usage() |

## 7. Testing Strategy

| AC / Invariant | Test Type | Location |
|---|---|---|
| PricingTable loads from TOML | Unit | `terraphim_usage/src/pricing.rs` |
| PricingTable embedded defaults cover 20+ models | Unit | `terraphim_usage/src/pricing.rs` |
| PricingTable pattern matching (gpt-4o-mini matches openai/gpt-4o*) | Unit | `terraphim_usage/src/pricing.rs` |
| TokenUsageTracker::drain_records() empties vec, keeps totals | Unit | `terraphim_multi_agent/src/tracking.rs` |
| ExecutionRecord::from_llm_usage() preserves all fields | Unit | `terraphim_usage/src/store.rs` (existing) |
| CcusageProvider returns ProviderNotFound if no runner | Unit | `terraphim_usage/src/providers/ccusage.rs` |
| OpenCode Go provider returns ProviderNotFound if no db | Unit | `terraphim_usage/src/providers/opencode_go.rs` |
| Full test suite passes | Integration | `cargo test --workspace` |
| Clippy clean | Lint | `cargo clippy --workspace` |

## 8. Implementation Order

1. **B1**: Pricing module (no deps on other steps)
2. **C1**: ccusage dependency + adapter (no deps on other steps)
3. **C2**: Claude provider (depends on C1)
4. **C3**: OpenCode Go provider (independent)
5. **B2**: TokenUsageTracker drain + flush (depends on B1 for pricing)
6. **B3**: Wire production recording in genai_llm_client (depends on B2)
7. **C4**: Register providers in CLI (depends on C1, C2, C3)
8. **Verify**: Full test suite + clippy

Steps 1-4 can be done in parallel. Steps 5-6 are sequential. Step 7 is last.
