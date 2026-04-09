# Research Document: ADF Control-Plane Routing for Probe- and Cost-Aware Model Selection

## 1. Problem Restatement and Scope

The ADF orchestrator currently has the pieces needed to reason about provider availability and cost, but those pieces are not yet organised into a complete control-plane decision layer that selects the best model when an agent is mentioned or started. There is already a first-generation routing decision embedded directly in `AgentOrchestrator::spawn_agent()`: it uses KG routing first, skips unhealthy providers using `ProviderHealthMap.unhealthy_providers()`, then falls back to static model config or keyword routing. However, that logic is still local to the spawn path, biased towards provider-level health, and does not use cost or historical execution efficiency.

The immediate problem is not "how to add more routing logic". The immediate problem is that dispatch-time model selection lacks one authoritative decision point that combines:

- route intent from KG/task context,
- intelligent keyword-based routing from the existing `RoutingEngine`,
- current provider and model health from probes,
- subscription availability state and local budget state from cost tracking,
- execution goals such as minimising avoidable spend while maximising useful throughput.

This research focuses on introducing that decision point in the control plane.

IN scope:

- Dispatch-time routing decisions for agent startup and mention-triggered agent runs.
- Reuse of existing `KgRouter`, `ProviderHealthMap`, `CostTracker`, and agent config.
- Explicit control-plane decision records for why a provider/model was chosen or rejected.
- How probe health and cost state should influence routing before spawn.

OUT of scope:

- Full Agent SDK migration.
- Replacing subprocess communication end-to-end.
- Reworking Nightwatch drift logic beyond consuming better routing signals.
- Multi-machine orchestration.
- New pricing backends or external billing integrations.

## 2. User & Business Outcomes

User-visible and operator-visible outcomes:

- When an agent is triggered, it routes to an available model instead of repeatedly failing against an unhealthy provider.
- Lower-cost models are preferred when they satisfy the requested work profile.
- Expensive models are reserved for tasks that genuinely need them.
- The orchestrator can explain why a route was chosen, skipped, or degraded.
- Mention-triggered flows become more reliable during provider incidents.

Business outcomes:

- Reduced wasted spend from failed or repeated invocations.
- Better throughput per unit cost across the ADF fleet.
- Cleaner separation between policy and execution, making future Agent SDK and approval-gate work simpler.
- Fewer noisy probe-related incidents such as Gitea `#510`.

## 3. System Elements and Dependencies

| Component | Location | Current Role | Relevant Dependencies | Notes |
|-----------|----------|--------------|-----------------------|-------|
| `AgentOrchestrator` | `crates/terraphim_orchestrator/src/lib.rs` | Central reconciliation loop and runtime owner | Scheduler, Nightwatch, spawner, router, output poster, flow, cost, probes | Currently too broad; best insertion point for a new control-plane decision boundary |
| `KgRouter` | `crates/terraphim_orchestrator/src/kg_router.rs` | Maps task text to provider/model candidates from markdown taxonomy | `terraphim_automata`, taxonomy markdown | Returns best route and fallback routes, but does not include live cost/budget state |
| `ProviderHealthMap` | `crates/terraphim_orchestrator/src/provider_probe.rs` | Maintains probe results and circuit-breaker state by provider/model | `KgRouter`, `CircuitBreaker` | Has TTL, model health, provider health, unhealthy provider discovery, and persisted probe results |
| `CostTracker` | `crates/terraphim_orchestrator/src/cost_tracker.rs` | Tracks per-agent budget and execution metrics | `ExecutionMetrics`, agent config budgets | Tracks local spend and run history, but subscription-backed providers shift the main routing concern from marginal per-call price to which subscription lane is live and effective |
| `RoutingEngine` | `terraphim_router` used from `crates/terraphim_orchestrator/src/lib.rs` | Keyword/capability-based routing fallback | Provider registry, routing context | Already provides intelligent routing and should be elevated from fallback-only behaviour into a first-class signal in the decision layer |
| `ExecutionMetrics` | `crates/terraphim_orchestrator/src/cost_tracker.rs` | Captures execution cost, latency, success, model, provider | CostTracker | Important because it already stores provider/model attribution |
| Flow executor | `crates/terraphim_orchestrator/src/flow/executor.rs` | Runs action and agent flow steps | `AgentSpawner`, token parser | Parses cost from output after execution rather than influencing route selection before execution |
| Mention handling | `crates/terraphim_orchestrator/src/mention.rs`, `adf_commands.rs` | Resolves `@adf:` mentions to agents/personas | Persona registry, tracker comments | Mention dispatch is a key trigger point for the proposed decision layer |
| Scope/worktree management | `crates/terraphim_orchestrator/src/scope.rs` | Manages file reservations and git worktrees | Git worktrees | Relevant because routing decisions affect where and how expensive work is launched |
| ADF CLI | `crates/terraphim_orchestrator/src/adf_commands.rs`, `src/bin/adf.rs` | Operational commands and parser surface | Orchestrator internals | Natural place for exposing route explanation later |
| ADF architecture plans | `.docs/design-dark-factory-orchestration.md`, `.docs/adf-architecture.md` | Original design intent and current operational model | Human governance | Useful to compare expected controller size vs current growth |
| CTO Executive System plans | `/Users/alex/cto-executive-system/plans/adf-architecture-improvements.md`, `.docs/design-execution-tiers.md`, `.docs/design-unified-routing.md` | Adjacent architectural direction | ADF roadmap issues | Strongly reinforce typed control plane, policy gates, unified routing |
| Developer flywheel methodology | `/Users/alex/cto-executive-system/knowledge/agent-flywheel-methodology.md` | Coordination methodology for agent swarms | Tracker, reservation patterns, planning discipline | Relevant for visible work claiming and advisory file reservations |
| Tracker abstraction | `crates/terraphim_tracker/src/lib.rs`, `crates/terraphim_tracker/src/gitea.rs` | Normalised issue model and Gitea tracker client | ADF tracker workflows, pre-check tracker | Relevant because current claim/reservation mechanics are workflow-only, not tracker-native |

Important dependencies across these elements:

- `KgRouter` provides intent and candidate routes.
- `ProviderHealthMap` provides live availability and degradation state.
- `CostTracker` provides local spend state and execution metrics, but at agent granularity rather than route-option granularity.
- `AgentOrchestrator` currently owns all three, so it can host a control-plane routing decision layer without cross-crate churn.

Observed existing routing path:

- In `crates/terraphim_orchestrator/src/lib.rs:904-1235`, `spawn_agent()` already performs a route decision before spawn.
- The order today is: KG route -> unhealthy-provider skip using `ProviderHealthMap.unhealthy_providers()` -> static `def.model` -> `RoutingEngine` keyword route.
- The chosen model is stored as `ManagedAgent.routed_model` for later logging.
- `spawn_with_fallback()` in `crates/terraphim_spawner/src/lib.rs:481-547` provides a spawn-time provider fallback if the primary spawn fails.
- On exit, `lib.rs:2749-2759` feeds provider-level success/failure back into `ProviderHealthMap`, but this feedback is keyed at provider granularity rather than using the routed model.
- `CostTracker` currently enforces budget after execution or at reconciliation (`lib.rs:2515-2548`), not before route selection.
- Probe refresh currently uses TTL staleness and can re-probe periodically (`lib.rs:2495-2508`), but this is a poor fit if the real decision problem is subscription lane liveness rather than frequent synthetic probing.

Observed coordination pattern from the CTO executive-system knowledge base:

- The developer flywheel emphasises explicit task claiming and visible advisory file reservations.
- In ADF, the closest durable analogue is Gitea issue claiming plus issue comments listing reserved files.
- This matters here because the routing refactor touches high-conflict files such as `lib.rs`, `kg_router.rs`, `provider_probe.rs`, and future `control_plane/*` modules.
- The design should therefore include tracker-visible file reservation as part of the workflow, not just as an informal habit.

Observed tracker gap in current repo:

- `terraphim_tracker::Issue` does not currently model claim or reservation metadata.
- `GiteaTracker` can fetch and post issue/comment data, but it does not expose a normalised contract for `claimed_by` or `reserved_files`.
- A separate tracker enhancement issue is therefore warranted so coordination mechanics can move from convention into reusable tracker capability.

## 4. Constraints and Their Implications

| Constraint | Why it matters here | Implication for a good solution |
|-----------|----------------------|---------------------------------|
| Single-server ADF deployment | Decisions happen on one orchestrator runtime | Avoid distributed coordination; keep control-plane state local and explicit |
| Existing routing is KG-first with keyword fallback | Two useful intent signals already exist | Do not replace KG or keyword routing; combine them coherently in one decision layer |
| Probe health is model-specific | Provider-wide health is too coarse for mixed model fleets | Decision layer should prefer model-level health before provider-level fallback |
| Provider access is subscription-backed | The dominant cost question is often "which paid lane is live" rather than token-by-token marginal price | Good solutions should track subscription availability and recent success as first-class signals, using spend mainly for local governance and budget ceilings |
| Existing spawn path already makes routing decisions | There is live behaviour to preserve while refactoring | Refactor by extraction, not replacement; keep current precedence where sensible |
| CLI/subprocess invocation is still current runtime | Spawn-time decisions must work with today’s runtime | Do not depend on Agent SDK migration to improve route selection |
| Probing every few minutes can waste time and create noise | Frequent synthetic checks are not the same as real workload success | Favour passive liveness from real dispatch outcomes, with probe-on-startup/manual/stale recovery rather than constant 5-minute probing |
| High-conflict orchestration files are shared among agents | Multi-agent changes can overlap easily in `lib.rs` and routing modules | Good solutions must include visible Gitea claiming and advisory file reservations for planning and implementation slices |
| terraphim_tracker does not yet encode claim/reservation state | Workflow can be followed by humans but not reasoned about uniformly by code | A separate tracker issue should introduce a small normalised contract without blocking the routing refactor |
| Reliability matters more than theoretical optimality | Failed work is more expensive than slightly suboptimal work | Prefer healthy-enough routes over cheapest-but-flaky routes |
| Human operators need auditability | ADF already creates issues and comments for operations | Routing must produce a human-readable rationale |
| The orchestrator is already monolithic | New logic added carelessly will worsen structural debt | Introduce a clear boundary, not more inline branching in `lib.rs` |

## 5. Risks, Unknowns, and Assumptions

### Risks

| Risk | Why it matters | De-risking action |
|------|----------------|------------------|
| Route selection becomes another monolithic branch tree in `lib.rs` | Would deepen current structural debt | Introduce a dedicated control-plane module with a small input/output contract |
| Probe state is too noisy to drive routing directly | Issue `#510` already shows repeated probe failures | Use probe status as one weighted signal, not the only gate; preserve fail-open fallback |
| Over-optimising for token cost is the wrong objective for subscription-backed providers | Could misroute work despite no real marginal savings | Model the decision as subscription-liveness plus work suitability, with local spend used for budget governance |
| Extracting the decision layer may accidentally change current route precedence | Current spawn path already has KG-first semantics and fallback behaviour | Preserve current precedence in tests before adding cost-aware ranking |
| Cheap routes may reduce work quality | Cost minimisation can degrade outcomes | Use work profile and route priority as first-order constraints; cost is a tie-breaker within acceptable candidates |
| Excessive periodic probes may flap healthy/unhealthy state | Synthetic failures can dominate route choices | Prefer real execution feedback and cooldown windows over frequent probe loops |
| Mention-triggered work may need rapid dispatch even with stale probes | Probe TTL introduces staleness windows | Include stale-state handling and explicit degraded mode |

### Unknowns

- Whether current provider/model execution metrics are rich enough to compute a meaningful "work per cost" signal.
- Whether all agent types currently record `provider` and `model` consistently into `ExecutionMetrics`.
- Whether probe actions are representative enough of real agent workloads to guide routing.
- Whether some Safety agents should ignore cost optimisation entirely.
- Whether subscription liveness should be tracked per provider, per model, or per CLI lane.

### Assumptions

- Assumption: `KgRouter` remains the primary source of intent classification.
- Assumption: probe results and circuit-breaker state are trusted enough to suppress obviously bad routes.
- Assumption: budget pressure should influence route choice before agent pause/exhaustion is reached.
- Assumption: the first implementation uses one global routing policy rather than per-agent overrides.
- Assumption: Safety agents participate in the shared routing policy rather than having a special `reliability_first` rule.
- Assumption: subscription liveness is tracked at exact model granularity first.

## 6. Context Complexity vs. Simplicity Opportunities

Current complexity sources:

- Routing intent, probe health, and cost state live in separate modules with no explicit arbitration contract.
- `AgentOrchestrator` owns too many responsibilities, making cross-cutting logic tempting to inline.
- Historical roadmap work spans GitHub and Gitea issues, which risks fragmented implementation.

Simplification opportunities:

1. Create a single control-plane routing decision object.
This object should take route candidates, intelligent keyword/KG intent signals, subscription-health state, budget state, and execution goal, and return one decision plus rationale.

2. Extract the existing route selection from `spawn_agent()` rather than re-invent it.
The orchestrator already decides first, then spawns. The simplification opportunity is to move that decision into a dedicated component and expand it with cost awareness.

3. Treat cost optimisation as bounded policy, not global optimisation.
The decision layer should answer: "Which live subscription-backed route best fits this work profile and current budget pressure?" rather than attempt perfect economic optimisation.

4. Shift from probe-centric liveness to dispatch-informed liveness.
Real run success and recent failures should become the primary signal for whether a subscription lane is live. Synthetic probes should be reserved for startup, manual recovery, or stale-state recovery instead of running every few minutes by default.

5. Make coordination visible in the tracker.
Planning and implementation work on routing should claim the relevant Gitea issue, mark it in progress, and post advisory file reservations so concurrent agents can see intended touch points before editing.

6. Evolve from workflow convention to tracker capability.
The immediate routing work can use Gitea issue comments and labels, but a parallel tracker enhancement should add normalised claim/reservation metadata so other systems can reason about it programmatically.

7. Standardise reservation comments for all high-conflict ADF issues.
This should not remain an orchestrator-only practice. The coordination pattern should apply to any ADF change that touches shared, conflict-prone files or runtime behaviour.

## 7. Questions for Human Reviewer

1. Should mention-triggered dispatch and scheduled dispatch share the same routing policy, or should mentions bias for responsiveness over thrift?
Why it matters: affects whether there is one policy or two profiles.

2. Should the decision rationale be surfaced only in logs/activity, or also in Gitea/GitHub comments for operator visibility?
Why it matters: this changes the output surface and noise profile.

3. Do you want this improvement to fold issue `#510` into the same work item, or remain a dependent follow-up issue?
Why it matters: affects scope and rollout expectations.

4. Should the first tracker contract parse reservation comments opportunistically, or require a stricter comment format/tag from day one?
Why it matters: affects backwards compatibility and implementation friction in `terraphim_tracker`.
