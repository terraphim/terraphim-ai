# Design & Implementation Plan: ADF Control-Plane Routing Decision Layer

## 1. Summary of Target Behavior

When an agent is started, restarted, or triggered by mention, the ADF orchestrator should make one explicit routing decision before spawn.

That decision should:

- start from the candidate provider/model routes supplied by `KgRouter`,
- incorporate intelligent keyword/capability routing from the existing `RoutingEngine`,
- discard or down-rank unhealthy routes using `ProviderHealthMap`,
- account for subscription liveness and local budget pressure using `CostTracker` and historical execution metrics,
- choose the route that best balances availability, cost, and expected work yield for the specific dispatch context,
- emit a structured rationale describing the chosen route and rejected alternatives.

The control plane remains responsible for decision-making. The execution plane remains responsible for spawning and monitoring agents.

Coordination requirement derived from the developer flywheel methodology:

- Work on this refactor should be claimed in Gitea before implementation begins.
- The active issue should be labelled `status/in-progress`.
- The issue should contain an advisory file-reservation comment listing the files the agent intends to modify.
- This is the Gitea-native analogue of beads/Agent-Mail style reservation and should be treated as part of the implementation workflow for all high-conflict ADF issues.

Tracker follow-up:

- The immediate workflow uses Gitea labels and comments directly.
- A parallel tracker enhancement issue (`#528`) should add a small normalised contract for claim/reservation metadata so this coordination pattern is not limited to free-form comments.

Decisions already made for this plan:

- Use one global routing policy first.
- Safety agents use the shared routing policy.
- Track subscription liveness at exact model granularity first.
- Run tracker issue `#528` in parallel rather than blocking `#524`.
- Parse existing free-form reservation comments first in `terraphim_tracker`.

## 2. Key Invariants and Acceptance Criteria

### Invariants

- A route decision is always made before spawn for eligible agent dispatches.
- KG routing remains the source of semantic intent; the new layer does not replace taxonomy rules.
- Unhealthy routes are never preferred over healthy alternatives unless the system enters explicit degraded fallback mode.
- Budget exhaustion still blocks or pauses agents per existing budget policy.
- Every routing decision produces a structured rationale suitable for logs and future activity events.

### Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC1 | Mention-driven and scheduled agent dispatch both call the same control-plane routing decision layer before spawn |
| AC2 | The decision layer can rank candidate routes using probe health, route priority, and cost/budget signals |
| AC3 | If the primary KG route is unhealthy, a healthy fallback route is selected when available |
| AC4 | If an agent is near budget exhaustion, the decision layer biases towards lower-cost healthy routes |
| AC5 | A structured decision record captures chosen route, rejected routes, and reasons |
| AC6 | Existing agents without enough historical cost data still route safely using health and KG priority alone |
| AC7 | The implementation keeps spawn execution separate from route selection |

## 3. High-Level Design and Boundaries

### Solution concept

Introduce a dedicated control-plane component inside `terraphim_orchestrator`, tentatively named `RoutingDecisionEngine`.

Its responsibility is narrow:

- receive dispatch context,
- gather route candidates,
- apply health and cost policy,
- return a `RoutingDecision`.

It does not spawn processes, parse output, or manage drift.

Important correction based on current code: the orchestrator already contains a first-generation control-plane route selector inside `AgentOrchestrator::spawn_agent()`.
This plan is therefore an extraction-and-upgrade, not a net-new routing feature. The design must preserve the existing precedence where it is already correct:

- KG route first,
- then health-aware fallback,
- then static model,
- then keyword routing,
- then CLI default.

### Control-plane boundaries

Inside the new decision layer:

- route candidate collection,
- health filtering and degraded-mode fallback,
- cost-aware ranking,
- decision explanation.

Outside the decision layer:

- process spawning,
- output capture,
- token parsing,
- Nightwatch drift evaluation,
- issue/comment posting.

Outside the routing logic but inside the workflow:

- Gitea issue claiming,
- advisory file reservation comments,
- progress updates when reservation scope changes.

### Design stance

This plan deliberately avoids:

- a new distributed service,
- a new persistence backend,
- a full pricing engine,
- a full Agent SDK migration in the same change.

The goal is a small, durable control-plane seam that later work can reuse.

Important policy correction: for subscription-backed providers, the primary economic goal is not minimising marginal token cost. The primary operational goal is choosing the most suitable currently-live subscription lane for the work. Local spend and budget remain relevant, but mostly as governance constraints and tie-breakers.

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/terraphim_orchestrator/src/lib.rs` | Modify | Orchestrator owns routing logic inline and broad runtime state | Delegates pre-spawn model selection to a dedicated decision engine | New decision module, existing spawn paths |
| `crates/terraphim_orchestrator/src/provider_probe.rs` | Modify | Probe results and health checks are queryable but not framed as route-quality input | Exposes the data needed for route scoring and decision reasoning | Existing probe results, breakers |
| `crates/terraphim_orchestrator/src/cost_tracker.rs` | Modify | Tracks budgets and execution metrics by agent | Exposes lightweight historical route-efficiency signals for control-plane ranking | Existing `ExecutionMetrics` |
| `crates/terraphim_orchestrator/src/kg_router.rs` | Modify | Returns primary route and fallbacks | Supplies candidate routes cleanly into the decision layer, preserving priority semantics | Existing route definitions |
| `terraphim_router` integration in `lib.rs` | Modify | Keyword routing is fallback-only | Elevated into a first-class routing signal alongside KG routing | Existing routing engine |
| `crates/terraphim_orchestrator/src/flow/executor.rs` | Modify | Agent flow steps spawn directly from step config | Flow agent execution optionally uses the same decision layer when model/provider are not pinned | Decision engine contract |
| `crates/terraphim_orchestrator/src/mention.rs` | Modify | Mention resolution identifies which agent to run | Mention-triggered dispatch includes a dispatch profile that can influence routing policy | Orchestrator dispatch integration |
| `crates/terraphim_orchestrator/src/config.rs` | Modify | Agent config contains budgets, model defaults, routing config | Adds explicit route policy knobs such as optimisation mode or reliability floor | Existing config loading |
| `crates/terraphim_orchestrator/src/control_plane/routing.rs` | Create | No explicit control-plane routing decision module | Owns `RoutingDecisionEngine`, `RoutingDecision`, ranking inputs, and reasoning output | `KgRouter`, `ProviderHealthMap`, `CostTracker` |
| `crates/terraphim_orchestrator/src/control_plane/mod.rs` | Create | No control-plane namespace | Clear namespace for future policy and approval control-plane work | New module structure |
| `crates/terraphim_orchestrator/tests/...` or inline tests | Modify/Create | Existing tests cover individual modules | Adds route-decision tests across health/cost scenarios | Decision engine |
| Gitea issue `#524` | Update | Open design issue | Claimed, labelled in progress, contains reserved file set for this work slice | Gitea CLI workflow |
| `crates/terraphim_tracker/src/lib.rs` and `src/gitea.rs` | Parallel follow-up | Tracker abstraction lacks claim/reservation contract | Future support for `claimed_by` and `reserved_files` metadata | Separate issue `#528` |
| Gitea process docs / AGENTS conventions | Future follow-up | Reservation comments are practice, not standard rule | Standard requirement for all high-conflict ADF issues | Workflow standardisation |

## 5. Step-by-Step Implementation Sequence

1. Define the decision contract around the existing spawn path.
Purpose: create `RoutingDecision`, `RouteCandidate`, `DispatchProfile`, and `DecisionReason` types that can represent the current KG/static/keyword precedence without changing behaviour first.
Deployable state: yes.

2. Establish coordination metadata before editing high-conflict files.
Purpose: claim the Gitea issue, mark it in progress, and post advisory file reservations for the planned module set.
Deployable state: yes.

3. Extract current route selection from `spawn_agent()` into the control-plane routing module.
Purpose: move existing KG route, unhealthy-provider fallback, static model fallback, and keyword-route fallback into `RoutingDecisionEngine` without changing semantics yet.
Deployable state: yes.

4. Expose minimal probe, keyword, and cost inputs.
Purpose: add the read-side methods required for route ranking, while keeping current probe execution and budget enforcement semantics stable.
Deployable state: yes.

5. Upgrade the extracted selector to combine KG and keyword routing explicitly.
Purpose: treat keyword routing as an intelligent signal rather than only a last resort, while preserving KG as the strongest semantic signal.
Deployable state: yes.

6. Upgrade the selector from probe-centric health to subscription-liveness-aware ranking.
Purpose: use recent real dispatch outcomes and cooldown windows as the primary liveness signal, with synthetic probes used on startup, manual recovery, or stale recovery rather than every 5 minutes.
Deployable state: yes, if fallback preserves current defaults when no decision can be made.

7. Integrate local budget pressure as a bounded policy input.
Purpose: bias away from costly or low-yield routes when an agent is near exhaustion, without pretending subscription lanes have precise marginal cost curves.
Deployable state: yes.

8. Integrate orchestrator startup, restart, and mention dispatch with the decision engine.
Purpose: ensure all orchestrator-driven spawns use the same extracted path.
Deployable state: yes.

9. Integrate flow agent steps where provider/model are not pinned.
Purpose: reuse the same decision layer in flow execution rather than creating a second route-selection path.
Deployable state: yes.

10. Emit structured decision rationale.
Purpose: write decision summaries to tracing now and leave them ready for later `activity.rs` work.
Deployable state: yes.

11. Add policy tuning via config.
Purpose: support a global default policy such as `balanced` or `throughput_first` first, without introducing per-agent overrides or a complex DSL.
Deployable state: yes.

12. Add focused tests and a rollout check.
Purpose: prove fallback selection, budget-aware biasing, and degraded-mode behaviour.
Deployable state: yes.

13. Update the issue reservation comment when the file set changes materially.
Purpose: keep coordination visible and reduce edit collisions for concurrent agents.
Deployable state: yes.

14. Track the parallel tracker enhancement.
Purpose: ensure the comment-and-label workflow used here is later captured in `terraphim_tracker` via issue `#528`, without blocking the routing extraction itself.
Deployable state: yes.

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| AC1 | Integration test | `crates/terraphim_orchestrator/tests/control_plane_routing.rs` or equivalent |
| AC2 | Unit test | `control_plane/routing.rs` tests matching current route precedence |
| AC3 | Unit test | `control_plane/routing.rs` tests combining KG and keyword signals |
| AC4 | Unit test | `control_plane/routing.rs` tests with unhealthy primary route |
| AC5 | Unit test | `control_plane/routing.rs` tests asserting structured rationale contents |
| AC6 | Unit test | `control_plane/routing.rs` tests with near-exhaustion budget state |
| AC7 | Integration test | orchestrator spawn path test asserting decision-before-spawn |
| AC8 | Unit test | liveness tests showing real execution feedback suppresses excessive probe dependence |
| AC9 | Process verification | Gitea issue shows claim status and advisory reserved files for the work slice |
| AC10 | Parallel design linkage | Plan references tracker issue `#528` for future normalised claim/reservation support |
| AC11 | Policy verification | Implementation assumes a global routing policy and shared Safety-agent policy for phase one |

Additional regression expectation:

- Before cost-aware ranking is enabled, extracted routing must preserve the current behaviour observed in `spawn_agent()`.
- Probe cadence must not assume a fixed 5-minute loop when recent real execution evidence is available.

Verification expectations:

- Existing provider probes continue to run unchanged.
- Existing budget enforcement continues to pause exhausted agents.
- Routing falls back safely when probe data is stale or absent.
- Mention-driven dispatch does not regress in responsiveness.

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Decision logic becomes a second monolith | Keep one dedicated module with a small input/output API | Medium |
| Subscription-aware routing still gets mistaken for token-price optimisation | Make subscription liveness and work suitability primary; budgets and spend remain bounded policy inputs | Low |
| Probe results are noisy | Health is a gate/weight, not an absolute oracle except for clearly unhealthy routes | Medium |
| Extraction changes current routing semantics unintentionally | Add regression tests for today’s KG/static/keyword fallback order before behavioural upgrades | Medium |
| Keyword routing gets accidentally demoted again | Make combined KG+keyword inputs explicit in the decision contract and tests | Medium |
| Flow execution diverges from orchestrator dispatch | Reuse the same engine contract from both call sites | Low |
| Concurrent agents edit the same orchestration files | Use Gitea issue claim plus advisory file reservations and update them when scope changes | Medium |
| Coordination remains comment-only forever | Create and track a separate terraphim_tracker issue for normalised claim/reservation metadata | Medium |
| Config growth becomes unwieldy | Add only a small route policy surface in this phase | Low |

Complexity review:

- This plan reduces complexity if it replaces inline routing decisions rather than layering more branches into `lib.rs`.
- The main failure mode is not technical difficulty but boundary slippage.
- The correct shape is one reusable decision component, not several special-case selectors.

## 8. Open Questions / Decisions for Human Review

1. Should the first implementation expose a single global optimisation profile, or allow per-agent policy overrides immediately?

2. Should Safety agents be hard-wired to `reliability_first` in this phase?

3. Do you want route rationale surfaced only in tracing/activity, or also in issue comments for high-cost or degraded dispatches?

4. Should issue `#510` be treated as an acceptance target for this design, or as a follow-on beneficiary once the control-plane layer exists?

5. Should mention-triggered dispatch and scheduled dispatch share exactly the same global policy, or should mentions later get a responsiveness bias?

6. Should issue `#528` be treated as a required prerequisite for broader multi-agent rollout, or remain a parallel hardening track while `#524` proceeds with comment-based reservations?
