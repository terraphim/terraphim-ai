# Design and Implementation Plan: LLM API Cost Tracking

## 1. Summary of Target Behaviour

After implementation:

```
$ terraphim-agent usage show
Agent: default | Today: $0.42 (12,340 tokens) | This week: $3.18 | This month: $14.72
Budget: $50.00/month | 29.4% used | On track

$ terraphim-agent usage history --last 7d --by model
Model                       Tokens    Cost
openai/gpt-4o              142,000   $4.26
anthropic/claude-sonnet      89,000   $2.67
google/gemini-2.0-flash     234,000   $0.47
ollama/llama3               12,000   $0.00 (local)

$ terraphim-agent usage alert --budget 50 --threshold 80
Alert: monthly spend at $41.20 (82.4% of $50.00 budget)
```

Every LLM call through `LlmClient` automatically records token usage and estimated cost. Data flows from API response extraction through to persistent storage in `terraphim_usage::UsageStore`.

## 2. Key Invariants and Acceptance Criteria

### Invariants
- **I1**: No public-API breakage -- `LlmClient` existing methods keep their signatures
- **I2**: Cost extraction never blocks or fails the LLM call -- fire-and-forget persistence
- **I3**: Sub-cent precision for all monetary values (1/1,000,000 cent, matching `AgentMetricsRecord`)
- **I4**: `terraphim_service` does not depend on `terraphim_usage` -- types flow through `terraphim_types`
- **I5**: All cost data is per-call attributable to model, provider, and timestamp

### Acceptance Criteria
- [ ] AC1: `LlmUsage` struct in `terraphim_types` with `input_tokens`, `output_tokens`, `model`, `provider`, `cost_usd`, `latency_ms`
- [ ] AC2: `LlmClient` has new `chat_completion_with_usage()` method returning `(String, Option<LlmUsage>)`
- [ ] AC3: OpenRouter `chat_completion()` extracts `usage.prompt_tokens` and `usage.completion_tokens` from response
- [ ] AC4: Ollama `summarize()` extracts `prompt_eval_count` and `eval_count` from response
- [ ] AC5: Callers in `terraphim_multi_agent` pipe `LlmUsage` into existing `TokenUsageTracker`
- [ ] AC6: `terraphim_usage::UsageStore::record_execution()` accepts `LlmUsage` and persists as `ExecutionRecord`
- [ ] AC7: Model pricing loaded from configurable JSON/TOML file, not hardcoded
- [ ] AC8: `terraphim-agent usage show` displays today/week/month spend with budget status
- [ ] AC9: `terraphim-agent usage history --by model` groups costs by model
- [ ] AC10: `terraphim-agent usage alert --budget N` triggers at configurable thresholds
- [ ] AC11: `cargo test -p terraphim_service` passes
- [ ] AC12: `cargo test -p terraphim_usage` passes
- [ ] AC13: `cargo clippy` clean on both crates

## 3. High-Level Design and Boundaries

### Architecture: Return-and-Delegate Pattern

```
terraphim_types         (shared LlmUsage type)
    ^       ^              ^
    |       |              |
    |       |              |
terraphim_service      terraphim_usage
(LlmClient returns     (UsageStore persists
 LlmUsage to caller)   LlmUsage data)
    |
    v
terraphim_multi_agent  (delegates to UsageStore)
```

**Key principle**: `terraphim_service` returns usage metadata. The caller decides what to do with it. This avoids circular dependencies.

### Data Flow

```
1. Caller invokes chat_completion_with_usage()
2. OpenRouter/Ollama extracts usage from API response
3. Returns (content, Some(LlmUsage{...}))
4. Caller (multi_agent or server handler):
   a. Records in TokenUsageTracker (in-memory)
   b. Calls UsageStore::record_execution() (persistent)
```

## 4. File/Module-Level Change Plan

| File | Action | Before | After |
|---|---|---|---|
| `terraphim_types/src/lib.rs` | Add | No LlmUsage type | Add `LlmUsage` struct and `ModelPricing` struct |
| `terraphim_service/src/llm.rs:32-56` | Modify | `LlmClient` trait returns `String` | Add `chat_completion_with_usage()` returning `LlmResult` |
| `terraphim_service/src/openrouter.rs:296-344` | Modify | Extracts only content | Extract `usage` from response JSON, return in `LlmResult` |
| `terraphim_service/src/openrouter.rs:153-200` | Modify | `generate_summary()` returns `String` | Extract usage from summary response |
| `terraphim_service/src/llm.rs:274-310` | Modify | `OpenRouterClient` adapter | Implement `chat_completion_with_usage()` |
| `terraphim_service/src/llm.rs:330-460` | Modify | `OllamaClient` adapter | Implement `chat_completion_with_usage()` with token extraction |
| `terraphim_service/src/llm/proxy_client.rs` | Modify | Returns `String` | Implement `chat_completion_with_usage()` (pass-through) |
| `terraphim_service/src/llm/bridge.rs` | Modify | Returns `String` | Implement `chat_completion_with_usage()` (delegate to inner) |
| `terraphim_usage/src/store.rs` | Modify | `ExecutionRecord` exists | Add `From<LlmUsage>` impl, add `record_execution_from_usage()` |
| `terraphim_usage/src/pricing.rs` | Create | N/A | Configurable pricing from JSON/TOML file |
| `terraphim_usage/src/providers/claude.rs` | Modify | Stub | Implement Anthropic usage API via OAuth |
| `terraphim_usage/src/providers/opencode_go.rs` | Modify | Stub | Implement SQLite query for cost data |
| `terraphim_usage/src/cli.rs` | Modify | Basic `show`/`history` | Add `--by model`, budget display, alert commands |
| `terraphim_multi_agent/src/tracking.rs` | Modify | Separate from UsageStore | Add `flush_to_store()` method to pipe records into UsageStore |

## 5. Step-by-Step Implementation Sequence

### Phase A: Foundation (shared types + trait extension)

**Step 1**: Add `LlmUsage` and `LlmResult` to `terraphim_types`
- File: `terraphim_types/src/lib.rs`
- Add structs:
  ```rust
  pub struct LlmUsage {
      pub input_tokens: u64,
      pub output_tokens: u64,
      pub model: String,
      pub provider: String,
      pub cost_usd: Option<f64>,
      pub latency_ms: u64,
  }

  pub struct LlmResult {
      pub content: String,
      pub usage: Option<LlmUsage>,
  }
  ```
- Deployable: Yes (additive, no breakage)

**Step 2**: Extend `LlmClient` trait with usage-aware method
- File: `terraphim_service/src/llm.rs:32-56`
- Add default method to trait:
  ```rust
  async fn chat_completion_with_usage(
      &self,
      messages: Vec<serde_json::Value>,
      opts: ChatOptions,
  ) -> ServiceResult<LlmResult> {
      let content = self.chat_completion(messages, opts).await?;
      Ok(LlmResult { content, usage: None })
  }
  ```
- Deployable: Yes (default impl falls back to existing method, returns None usage)

**Step 3**: Implement usage extraction for OpenRouter
- File: `terraphim_service/src/openrouter.rs:296-344`
- Add new method `chat_completion_with_usage()` to `OpenRouterService`
- After parsing response JSON, extract:
  ```rust
  let usage = response_json.get("usage").and_then(|u| {
      Some(LlmUsage {
          input_tokens: u.get("prompt_tokens")?.as_u64()?,
          output_tokens: u.get("completion_tokens")?.as_u64()?,
          model: self.model.clone(),
          provider: "openrouter".to_string(),
          cost_usd: None, // OpenRouter doesn't report cost per-call
          latency_ms: start.elapsed().as_millis() as u64,
      })
  });
  ```
- Deployable: Yes (new method, doesn't change existing `chat_completion()`)

**Step 4**: Implement usage extraction for Ollama
- File: `terraphim_service/src/llm.rs:330-460` (OllamaClient)
- In `chat_completion_with_usage()`, extract `prompt_eval_count` and `eval_count`
- Deployable: Yes

**Step 5**: Wire `OpenRouterClient` and `OllamaClient` adapters
- File: `terraphim_service/src/llm.rs` (adapter impls)
- Override `chat_completion_with_usage()` in both adapters to delegate to inner service's new method
- Deployable: Yes

### Phase B: Persistence (connecting to UsageStore)

**Step 6**: Add `From<LlmUsage>` for `ExecutionRecord`
- File: `terraphim_usage/src/store.rs`
- Implement conversion: `LlmUsage` -> `ExecutionRecord` with cost calculation from pricing table
- Add `UsageStore::record_execution_from_usage()` convenience method
- Deployable: Yes

**Step 7**: Create configurable pricing module
- File: `terraphim_usage/src/pricing.rs` (new)
- Load model pricing from `~/.config/terraphim/pricing.toml` (or embedded defaults)
- Pricing struct: `{ model_pattern, input_cost_per_1m_tokens, output_cost_per_1m_tokens }`
- Pattern matching for model families (e.g., `openai/gpt-4o*` matches all GPT-4o variants)
- Default pricing for common models embedded as fallback
- Deployable: Yes

**Step 8**: Wire `TokenUsageTracker::flush_to_store()`
- File: `terraphim_multi_agent/src/tracking.rs`
- Add method to drain in-memory records into `UsageStore`
- Async, fire-and-forget via `tokio::spawn`
- Deployable: Yes

### Phase C: Provider completion (finishing stubs)

**Step 9**: Implement Claude provider
- File: `terraphim_usage/src/providers/claude.rs`
- Read OAuth token from `~/.claude/.credentials.json`
- Call Anthropic usage API (or use `terraphim_ccusage` as backend)
- Deployable: Yes

**Step 10**: Implement OpenCode Go provider
- File: `terraphim_usage/src/providers/opencode_go.rs`
- Query `~/.local/share/opencode/opencode.db` SQLite for cost data
- Deployable: Yes

**Step 11**: Integrate `terraphim_ccusage` into `UsageRegistry`
- File: `terraphim_usage/src/providers/` (add ccusage adapter)
- Wrap `CcusageClient` as a `UsageProvider` trait impl
- Deployable: Yes

### Phase D: CLI surface

**Step 12**: Enhance `usage show` with budget display
- File: `terraphim_usage/src/cli.rs`
- Show today/week/month totals, budget percentage, on-track status
- Deployable: Yes

**Step 13**: Add `usage history --by model` grouping
- File: `terraphim_usage/src/cli.rs`
- Group `ExecutionRecord`s by model, display table
- Deployable: Yes

**Step 14**: Add `usage alert --budget N` command
- File: `terraphim_usage/src/cli.rs`
- Set budget, configure threshold alerts, display current status
- Deployable: Yes

### Phase E: Verification

**Step 15**: Add tests
- Unit tests: `LlmUsage` construction, `ExecutionRecord` conversion, pricing calculation
- Integration: Mock OpenRouter response with usage field, verify extraction
- CLI: Test `usage show` output format
- Deployable: Yes

**Step 16**: Clippy + full test suite
- `cargo clippy -p terraphim_service -- -D warnings`
- `cargo clippy -p terraphim_usage -- -D warnings`
- `cargo test -p terraphim_service`
- `cargo test -p terraphim_usage`

## 6. Testing and Verification Strategy

| Acceptance Criteria | Test Type | Test Location | Verification |
|---|---|---|---|
| AC1: `LlmUsage` struct | Unit | `terraphim_types` inline | Construction, serialisation |
| AC2: `chat_completion_with_usage()` | Unit | `terraphim_service/src/llm.rs` inline | Default impl returns None usage |
| AC3: OpenRouter usage extraction | Unit | `terraphim_service/src/openrouter.rs` inline | Mock JSON response, verify token extraction |
| AC4: Ollama usage extraction | Unit | `terraphim_service/src/llm.rs` inline | Mock JSON response, verify token extraction |
| AC5: Multi-agent piping | Integration | `terraphim_multi_agent` tests | Record created in tracker |
| AC6: UsageStore persistence | Integration | `terraphim_usage` tests | TempDir, write+read ExecutionRecord |
| AC7: Configurable pricing | Unit | `terraphim_usage/src/pricing.rs` inline | Load from TOML, pattern match, cost calc |
| AC8: `usage show` | CLI integration | `terraphim_usage` CLI tests | Output format contains expected fields |
| AC9: `usage history --by model` | CLI integration | `terraphim_usage` CLI tests | Grouped output by model |
| AC10: `usage alert` | Unit | `terraphim_usage/src/cli.rs` | Budget threshold triggers correctly |
| AC11-13: Clippy + tests | CI | Full suite | Exit code 0 |

## 7. Risk and Complexity Review

| Risk | Mitigation | Residual Risk |
|---|---|---|
| `LlmClient` trait change breaks callers | New method with default impl, existing methods unchanged | None |
| Circular deps between service and usage | Types in `terraphim_types`, service returns data, caller persists | None |
| OpenRouter API changes `usage` format | Pin response parsing, log raw JSON on extraction failure | Low |
| Pricing stale vs reality | Configurable TOML + OpenRouter pricing API as future enhancement | Pricing estimates may be off for new models |
| Fire-and-forget persistence loses data on crash | Acceptable trade-off; data catches up on next call | One call's cost may be lost |
| Provider auth complexity (Claude OAuth) | Use `terraphim_ccusage` as simpler alternative | Low |

## 8. Open Questions / Decisions for Human Review

1. **Phasing**: This is a large change (4 phases, 16 steps). Should we do it all in one PR, or split into: (A) types + extraction, (B) persistence, (C) providers, (D) CLI?

2. **Pricing source**: Should the default pricing file be maintained in the repo (manually updated), or fetched from OpenRouter's `/api/v1/models` endpoint which includes pricing data?

3. **Claude provider approach**: Use the npm `ccusage` wrapper (`terraphim_ccusage`) which already works, or implement direct Anthropic API calls? ccusage is simpler but requires bun.

4. **Kimi/Moonshot provider**: Is this worth implementing now, or can it stay as a stub? Do you have access to their usage API?

5. **Cost attribution**: Should costs be attributed per-role, per-agent, or per-session? The `ExecutionRecord` supports `agent_name` -- is that sufficient?
