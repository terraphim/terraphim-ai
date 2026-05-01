# Research Document: opencode-weather ADF-Backed Model Weather

## 1. Problem Restatement and Scope

The goal is to create a standalone `opencode-weather` repository that gives opencode users a personalised model weather report: fastest available model, cheapest effective model, best thinking model, best coding model, and models to avoid right now.

The core problem is not lack of routing logic. Terraphim already has substantial routing, telemetry, health, budget, and cost logic. The problem is how to expose that logic as a coherent, low-friction report inside opencode without duplicating the existing decision engines.

In scope:

| Area | Scope |
|---|---|
| Source of truth | Use `terraphim-ai` ADF/orchestrator as the canonical weather engine |
| Optional enrichment | Use `terraphim-llm-proxy` when traffic flows through it or when its APIs are available |
| opencode integration | Create a separate `opencode-weather` repo with plugin/tool presentation |
| Outputs | Human-readable report and versioned JSON snapshot |
| Data provenance | Show whether data came from ADF, proxy, opencode plugin capture, usage history, or static config |

Out of scope for first delivery:

| Area | Reason |
|---|---|
| Automatic opencode model switching | Recommendations should be proven before automation |
| Reimplementing ADF routing | Existing routing and telemetry code already handles this |
| Reimplementing proxy cost/performance databases | `terraphim-llm-proxy` already has those components |
| Hosted/global benchmarking | User asked for “for me now”, not general benchmark data |
| Invoice-grade billing | Weather decisions only require operational cost estimates |

## 2. User & Business Outcomes

User-visible outcomes:

| Outcome | Description |
|---|---|
| Fast choice | The user can see which model is currently fast for their own workflow |
| Cost-aware choice | The user can avoid expensive models when a cheaper/subscription model is good enough |
| Thinking guidance | The user can choose an appropriate reasoning model for deep-thinking work |
| Coding guidance | The user can choose an implementation-oriented model for coding tasks |
| Avoid list | The user can avoid quota-limited, unhealthy, stale, or failing models |

Business and engineering outcomes:

| Outcome | Description |
|---|---|
| Lower duplication | Existing ADF/proxy logic remains authoritative |
| Better reliability | Decisions are based on the same telemetry and budget gates used by ADF |
| Better maintainability | `opencode-weather` stays small and focused on display/integration |
| Cross-tool reuse | Weather snapshots can serve opencode, ADF, docs, dashboards, and future routing automation |

## 3. System Elements and Dependencies

| Element | Location | Responsibility | Relevance |
|---|---|---|---|
| ADF routing engine | `crates/terraphim_orchestrator/src/control_plane/routing.rs` | Combines KG, keyword, static config, budget pressure, provider budgets, telemetry | Primary source for candidate scoring and rationale |
| KG router | `crates/terraphim_orchestrator/src/kg_router.rs` | Loads markdown `route::`, `action::`, `synonyms::`, `priority::` rules | Primary model-to-keyword map and fallback route source |
| ADF telemetry | `crates/terraphim_orchestrator/src/control_plane/telemetry.rs` | Stores `CompletionEvent` and `ModelPerformanceSnapshot` | Source for success rate, latency, subscription limits |
| Provider probe | `crates/terraphim_orchestrator/src/provider_probe.rs` | Probes KG route action templates and records health | Source for current provider/model availability |
| Provider budgets | `crates/terraphim_orchestrator/src/provider_budget.rs` | Tracks provider spend in hour/day windows | Source for provider exhaustion and near-exhaustion |
| Usage store | `crates/terraphim_usage/src/store.rs` | Persists execution records with model/provider/cost/latency | Durable history fallback |
| Pricing table | `crates/terraphim_usage/src/pricing.rs` | Terraphim-compatible pricing and local overrides | Cost fallback when proxy cost data unavailable |
| Proxy router | `../terraphim-llm-proxy/src/router.rs` | Scenario routing, provider/model decisions | Optional enrichment when proxy is in path |
| Proxy performance | `../terraphim-llm-proxy/src/performance/*.rs` | Latency, p95/p99, throughput, success/error rate, stale metrics | Optional richer performance data |
| Proxy cost | `../terraphim-llm-proxy/src/cost/*.rs` | Pricing DB, cost estimates, budgets, analytics | Optional richer cost data |
| Proxy health | `../terraphim-llm-proxy/src/provider_health.rs` | Provider health and circuit breakers | Optional live health source |
| opencode SDK/API | `~/.config/opencode/node_modules/@opencode-ai/sdk` | Configured providers/models and events | Inventory and plugin integration source |
| `opencode-weather` | New repo | Thin CLI/plugin presenter | Should not own core ranking logic |

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
|---|---|---|
| We control all repositories | Cross-repo API changes are possible | Put logic in the right repo, not wherever easiest |
| Traffic may bypass `terraphim-llm-proxy` | Proxy metrics can be absent or incomplete | ADF must be primary, proxy optional |
| ADF already has keyword/model maps | Rebuilding maps in `opencode-weather` creates drift | Weather must consume ADF/KG routes |
| opencode plugin API is likely tool/hook-oriented | Persistent custom UI may not be available | First UI should be `model_weather` tool output |
| Weather is time-sensitive | Stale metrics can mislead | Snapshot must include timestamps, freshness, and confidence |
| User data is sensitive | Usage can reveal work patterns and costs | Local-first by default, no external telemetry |
| Subscription models are not simple “free” models | They can be quota-limited | Scoring must separate marginal cost from quota risk |

## 5. Risks, Unknowns, and Assumptions

Unknowns:

| Unknown | De-risking Step |
|---|---|
| Exact opencode completion event payloads | Add diagnostic plugin or inspect SDK/global events |
| Whether opencode exposes token/cost details directly | Test events and session APIs |
| Whether ADF should expose weather through `adf-ctl` or `terraphim-agent` | Prefer both eventually; start with whichever has existing operational path |
| Current divergence in `terraphim-llm-proxy` checkout | Treat proxy as optional until branch state is reconciled |

Assumptions:

| Assumption | Impact If Wrong |
|---|---|
| ADF can run dry-route decisions without spawning agents | If false, add explicit dry-run API to `RoutingDecisionEngine` |
| KG taxonomy routes represent the model/task map the user wants | If incomplete, weather quality depends on taxonomy improvement |
| `TelemetryStore` has enough recent data for high-confidence rankings after regular ADF use | If sparse, weather will fall back to lower-confidence static rules |
| `opencode-weather` can shell out to Terraphim CLIs | If not, use local HTTP endpoint instead |

Risks:

| Risk | Mitigation |
|---|---|
| Duplicate scoring logic between ADF and `opencode-weather` | Keep scoring in ADF weather service; plugin only renders |
| Proxy and ADF disagree | Snapshot records provenance and treats proxy as enrichment, not authority |
| Sparse telemetry produces overconfident recommendations | Include sample size and confidence labels |
| Active probes consume quota | Keep probes opt-in or reuse ADF startup probes |
| Model IDs vary across opencode, ADF, and proxy | Reuse existing model mapping and provider key functions where possible |

## 6. Context Complexity vs. Simplicity Opportunities

Complexity sources:

| Source | Complexity |
|---|---|
| Three execution paths | Direct opencode, ADF-spawned opencode, proxy-routed traffic |
| Multiple identifiers | Provider/model names vary across opencode, ADF routes, proxy config, and pricing |
| Multiple health concepts | ADF provider probes, proxy circuit breakers, quota/subscription telemetry |
| Multiple cost concepts | API list price, subscription marginal cost, provider budget exhaustion |

Simplicity opportunities:

| Opportunity | Benefit |
|---|---|
| ADF-owned weather service | One canonical scoring/rationale engine |
| Versioned JSON snapshot | Stable contract for opencode and future dashboards |
| Source precedence | Avoids ambiguous truth when data sources disagree |
| Thin opencode plugin | Easy install, low maintenance, little duplicated logic |

Recommended source precedence:

1. ADF live routing and telemetry.
2. Direct opencode plugin capture, if available.
3. `terraphim-llm-proxy` metrics, if configured and reachable.
4. Terraphim durable usage history.
5. Static KG/pricing/capability metadata.

## 7. Questions for Human Reviewer

| Question | Why It Matters |
|---|---|
| Should the first CLI be `adf-ctl weather` or `terraphim-agent weather`? | Determines user-facing entry point |
| Should `opencode-weather` require Terraphim CLIs, or support a degraded standalone mode? | Determines install complexity |
| Which default categories are mandatory: fastest, cheapest, thinking, coding, review, docs, testing? | Determines initial dry-run task set |
| Should direct opencode event capture write into Terraphim telemetry, or only `opencode-weather` cache? | Determines data ownership |
| Should proxy enrichment be enabled automatically when `TERRAPHIM_LLM_PROXY_URL` is set? | Determines default behaviour |
| Should active probes ever run from `opencode-weather`, or only from ADF/proxy? | Affects quota and operational risk |
