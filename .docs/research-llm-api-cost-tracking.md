# Research Document: LLM API Cost Tracking

## 1. Problem Restatement and Scope

### Problem
The project spends real money on LLM API calls across multiple providers (OpenRouter, Anthropic, MiniMax, Z.ai, Ollama), but there is no unified, reliable view of what is being spent. Cost tracking infrastructure exists across **five separate crates** but is fragmented, incomplete, and mostly non-functional:

- `terraphim_usage` has persistent storage and provider integrations, but **3 of 5 providers are stubs** and it is not connected to actual LLM call paths
- `terraphim_multi_agent` tracks tokens per-call in memory, but **never persists** to `terraphim_usage`
- `terraphim_orchestrator` parses CLI tool output for costs, but **only for spawned subprocesses** (Claude Code, OpenCode), not for in-app LLM calls
- `terraphim_ccusage` wraps the npm `ccusage` package, but is **not integrated** into `terraphim_usage`
- The `LlmClient` trait returns only `String`, **discarding all usage metadata** from API responses

### IN Scope
- Extracting token usage from OpenRouter and Ollama API responses
- Returning usage metadata from the `LlmClient` trait so all callers can access it
- Persisting per-call token/cost data via `terraphim_usage::UsageStore`
- Connecting existing in-memory trackers (`TokenUsageTracker`, `CostTracker`) to persistent storage
- Completing stub providers (Claude, Kimi, OpenCode Go) in `terraphim_usage`
- Integrating `terraphim_ccusage` as a provider in `terraphim_usage`
- Making model pricing configurable rather than hardcoded

### OUT of Scope
- Building a web dashboard (separate feature)
- Adding new LLM providers
- Changing the desktop UI
- Modifying the symphony/orchestrator subprocess cost parsing (already works)
- Rate limiting or throttling based on costs (future work)

## 2. User and Business Outcomes

| Stakeholder | Outcome |
|---|---|
| Developer (Alex) | Can see exactly how much was spent on LLM calls today/this week/this month, broken down by model and provider |
| Automation (ADF agents) | Cost data feeds into `Nightwatch` drift detection for budget-aware agent orchestration |
| Financial planning | Monthly spend reports per model enable informed decisions about subscriptions vs pay-per-use |
| Cost optimisation | Per-model cost comparison enables switching to cheaper models where quality is acceptable |
| Alerting | Budget threshold alerts prevent bill shock from runaway agents |

## 3. System Elements and Dependencies

### Existing Infrastructure (What We Have)

| Element | Crate | Location | Status | Role |
|---|---|---|---|---|
| `UsageStore` | `terraphim_usage` | `src/store.rs` | **Working** | Persistent query interface for usage data |
| `ExecutionRecord` | `terraphim_usage` | `src/store.rs` | **Working** | Per-call record: tokens, cost, model, provider, latency |
| `AgentMetricsRecord` | `terraphim_usage` | `src/store.rs` | **Working** | Per-agent aggregated metrics |
| `BudgetSnapshotRecord` | `terraphim_usage` | `src/store.rs` | **Working** | Budget audit snapshots |
| `AlertConfig` | `terraphim_usage` | `src/store.rs` | **Working** | Budget threshold alerts |
| `ProviderUsageSnapshot` | `terraphim_usage` | `src/store.rs` | **Working** | Cached provider API data |
| MiniMax provider | `terraphim_usage` | `src/providers/minimax.rs` | **Working** | Fetches session prompt counts, plan name |
| Z.ai provider | `terraphim_usage` | `src/providers/zai.rs` | **Working** | Fetches session/weekly token limits, plan name |
| Claude provider | `terraphim_usage` | `src/providers/claude.rs` | **Stub** | TODO: OAuth + Anthropic usage API |
| Kimi provider | `terraphim_usage` | `src/providers/kimi.rs` | **Stub** | TODO: Moonshot API |
| OpenCode Go provider | `terraphim_usage` | `src/providers/opencode_go.rs` | **Stub** | TODO: SQLite query |
| `TokenUsageTracker` | `terraphim_multi_agent` | `src/tracking.rs` | **Working** | In-memory per-agent token tracking |
| `CostTracker` | `terraphim_multi_agent` | `src/tracking.rs` | **Working** | In-memory budget management with hardcoded pricing |
| `ModelPricing` | `terraphim_multi_agent` | `src/tracking.rs` | **Hardcoded** | Only 3 models priced: gpt-4, gpt-3.5-turbo, claude-3-opus |
| `CompletionEvent` | `terraphim_orchestrator` | `src/control_plane/telemetry.rs` | **Working** | Per-completion telemetry |
| `TelemetryStore` | `terraphim_orchestrator` | `src/control_plane/telemetry.rs` | **Working** | Rolling window telemetry with persistence |
| `NightwatchMonitor` | `terraphim_orchestrator` | `src/nightwatch.rs` | **Working** | Cost drift detection |
| `TokenUsage` parser | `terraphim_orchestrator` | `src/flow/token_parser.rs` | **Working** | Regex-based token extraction from CLI output |
| `CcusageClient` | `terraphim_ccusage` | `src/lib.rs` | **Working** | Wraps npm ccusage for daily Claude/Codex usage |
| `LlmClient` trait | `terraphim_service` | `src/llm.rs` | **Gap** | Returns only `String`, discards usage |
| `OpenRouterClient` | `terraphim_service` | `src/openrouter.rs` | **Gap** | Does not extract `usage` from response JSON |
| `OllamaClient` | `terraphim_service` | `src/llm.rs` | **Gap** | Does not extract `eval_count`/`prompt_eval_count` |
| CLI commands | `terraphim_usage` | `src/cli.rs` | **Working** | `usage show`, `history`, `export`, `alert`, `budgets` |

### Data Flow (Current vs Desired)

**Current (broken):**
```
LlmClient::chat_completion() -> String (usage discarded)
                                |
                                v
                    No one persists cost data
```

**Desired:**
```
LlmClient::chat_completion() -> LlmResult { content, usage }
                                |
                    +-----------+-----------+
                    |                       |
                    v                       v
          terraphim_usage           multi_agent
          ExecutionRecord           TokenUsageTracker
          (persistent)              (in-memory, feeds into UsageStore)
```

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
|---|---|---|
| Backward compatibility | Many callers use `LlmClient` trait | Return type change must be backward-compatible or use a new method |
| Feature flags | `terraphim_usage` is behind `persistence`, `cli`, `providers` feature flags | Cost tracking must respect existing feature gates |
| Provider API auth | Claude uses OAuth, Kimi uses API keys, OpenCode uses local SQLite | Each provider has different auth requirements |
| Pricing volatility | Model prices change frequently | Pricing must be configurable, not hardcoded |
| Precision | Small costs (fractions of cents) accumulate | Use sub-cent precision (already in `AgentMetricsRecord`: 1/1,000,000 cent) |
| Async runtime | All LLM calls are async | Cost persistence must also be async, non-blocking |
| Cross-crate dependencies | `terraphim_service` cannot depend on `terraphim_usage` (circular risk) | Need a shared type crate (`terraphim_types`) or callback pattern |

### Critical Dependency Constraint

The `LlmClient` trait lives in `terraphim_service`. The `UsageStore` lives in `terraphim_usage`. `terraphim_service` cannot depend on `terraphim_usage` without risking circular dependencies. Solutions:

1. **Shared types in `terraphim_types`**: Define `TokenUsage` struct there, both crates use it
2. **Callback/hook pattern**: `LlmClient` accepts an optional `on_usage: Box<dyn Fn(TokenUsage)>` callback
3. **Return and let caller persist**: `LlmClient` returns usage metadata, caller decides whether to persist

Option 3 is the simplest and most testable.

## 5. Risks, Unknowns, and Assumptions

### Assumptions
- [ASSUMPTION] OpenRouter API responses include `usage.prompt_tokens` and `usage.completion_tokens` in the standard format (likely true based on OpenRouter's OpenAI-compatible API)
- [ASSUMPTION] The `terraphim_types` crate is a suitable location for shared cost types
- [ASSUMPTION] The `terraphim_ccusage` npm dependency (`bun dlx ccusage`) is available in the build environment
- [ASSUMPTION] Claude's usage API is accessible with the same OAuth token used for the CLI

### Unknowns
- What does the Anthropic usage API actually return? Need to check docs.
- Does Kimi/Moonshot have a public usage API?
- What schema does OpenCode Go's SQLite database use for cost data?
- Are there other LLM call sites beyond `terraphim_service` and `terraphim_multi_agent` that need usage extraction?
- Does the Z.ai provider's subscription data include actual spend or just limits?

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `LlmClient` return type change breaks callers | High | High | Add new method `chat_completion_with_usage()`, deprecate old one |
| Circular dependency between service and usage crates | Medium | High | Keep types in `terraphim_types`, use dependency injection |
| Provider APIs change or require auth changes | Medium | Medium | Pin API versions, test with real keys in CI |
| Hardcoded pricing becomes stale | High | Medium | Make configurable from JSON file; OpenRouter has a pricing API |
| Cost persistence adds latency to LLM calls | Low | Low | Fire-and-forget async persist |

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
1. **Five crates with overlapping tracking**: `terraphim_usage`, `terraphim_multi_agent::tracking`, `terraphim_orchestrator::telemetry`, `terraphim_ccusage`, `terraphim_symphony::runner` -- all track tokens independently
2. **Two pricing systems**: `CostTracker::default_model_pricing()` (hardcoded) and `terraphim_router` cost levels -- neither comprehensive
3. **Multiple storage backends**: JSON files via opendal, in-memory HashMaps, SQLite (planned) -- no single source of truth
4. **Incomplete provider coverage**: 3 of 5 providers are stubs

### Simplification Strategies
1. **`terraphim_usage::UsageStore` as single source of truth**: All other trackers pipe their data into it. One query interface.
2. **Shared `TokenUsage` type in `terraphim_types`**: All crates use the same struct. No conversion needed.
3. **Configurable pricing from JSON file**: One `pricing.json` (or TOML) that all pricing lookups use. No more hardcoded tables.
4. **Phased provider completion**: Fix the most impactful provider first (OpenRouter extraction), then complete stubs.

## 7. Questions for Human Reviewer

1. **Priority**: Should we focus first on extracting actual usage from API responses (OpenRouter/Ollama), or on completing the stub providers (Claude/Kimi/OpenCode Go)?
2. **Pricing approach**: Should model pricing be a JSON config file in the repo, fetched from OpenRouter's `/api/v1/models` endpoint at runtime, or both (local fallback)?
3. **Return type strategy**: Add a new `chat_completion_with_usage()` method to `LlmClient`, or change the existing `chat_completion()` return type?
4. **Scope**: Should this be a single PR or split into: (a) usage extraction from LLM responses, (b) persistence wiring, (c) provider completion, (d) pricing configurability?
5. **CLI integration**: Should `terraphim-agent usage show` be the primary interface, or do you also want a `terraphim-agent cost today` shortcut?
