# Research Document: TLA+ Formal Validation of the Agent Dispatch Framework (ADF)

**Status**: Draft (v2 -- expanded to full ADF scope)
**Author**: Terraphim AI
**Date**: 2026-04-04
**Reviewers**: Alex

## Executive Summary

The Terraphim Agent Dispatch Framework (ADF) is a suite of concurrent, async Rust crates that together manage AI agent lifecycle, dispatch, supervision, messaging, and coordination. The ADF comprises the Symphony orchestrator (`crates/terraphim_symphony/`), agent supervisor (`terraphim_agent_supervisor`), messaging system (`terraphim_agent_messaging`), registry (`terraphim_agent_registry`), goal alignment (`terraphim_goal_alignment`), task decomposition (`terraphim_task_decomposition`), multi-agent coordination (`terraphim_multi_agent`), and KG orchestration (`terraphim_kg_orchestration`). This research evaluates using TLA+ formal verification via the existing `terraphim/tlaplus-ts` TypeScript bindings to prove safety and liveness properties across the entire ADF's concurrency model.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Formal verification of the orchestrator prevents subtle concurrency bugs (double-dispatch, deadlocks, starvation) that are nearly impossible to catch with unit tests alone |
| Leverages strengths? | Yes | We already have `terraphim/tlaplus-ts` (TypeScript bindings for TLA+, all 8 issues closed, tree-sitter parser, evaluator, formatter, TLC bridge, CLI) AND production Symphony orchestrator code to model |
| Meets real need? | Yes | Symphony runs multiple concurrent agents with retry queues and dependency graphs -- the concurrency state space is too large for manual reasoning; prior runs already hit edge cases (retry loops, stall timeout races) |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
The ADF manages concurrent AI agent sessions with complex state transitions across multiple subsystems: Symphony dispatch/retry/reconciliation, OTP-style supervision trees with restart strategies, Erlang-style message passing with delivery guarantees, agent registry with capacity limits, and supervised workflow coordination. The interaction of concurrent events with shared state across these subsystems creates a state space too large to test exhaustively with traditional testing.

### Impact
Concurrency bugs in the ADF can cause:
- **Double-dispatch**: same issue dispatched to two agents simultaneously (Symphony)
- **Starvation**: issues permanently stuck in retry queue (Symphony)
- **Claim leaks**: claims never released, blocking future dispatch (Symphony)
- **Dependency deadlocks**: circular dependency cascades preventing any issue from being dispatched (Symphony)
- **Shutdown races**: workers not properly terminated on shutdown (Symphony, Supervisor)
- **Restart storms**: cascading restarts under OneForAll/RestForOne strategies (Supervisor)
- **Message loss**: messages dropped or duplicated under concurrent delivery (Messaging)
- **Duplicate registration**: agents registered twice causing routing conflicts (Registry, Messaging)
- **Workflow resource exhaustion**: unbounded concurrent workflows consuming all capacity (KG Orchestration)
- **Escalation cascades**: supervisor escalation loops when all children fail (Supervision Tree)

### Success Criteria
1. TLA+ specs modelling the core ADF state machines (Symphony, Supervisor, Messaging)
2. Safety properties verified: no double-dispatch, no claim leaks, bounded retry, restart intensity limits, message delivery invariants
3. Liveness properties verified: every eligible issue eventually dispatched, no starvation, eventual message delivery
4. TypeScript test harness using `tlaplus-ts` to run TLC model checking in CI

## Current State Analysis

### Existing Implementation

**Symphony Orchestrator (Rust)**: Production code in `crates/terraphim_symphony/`

| Component | Location | Purpose |
|-----------|----------|---------|
| Main loop | `src/orchestrator/mod.rs` | `tokio::select!` over poll tick, worker exit, agent events, retry timer, ctrl-c |
| Dispatch logic | `src/orchestrator/dispatch.rs` | Eligibility checks, PageRank-based sorting |
| State management | `src/orchestrator/state.rs` | `OrchestratorRuntimeState` with running/claimed/retry/completed sets |
| Reconciliation | `src/orchestrator/reconcile.rs` | Stall detection, tracker state refresh |
| Issue model | `src/tracker/mod.rs` | `Issue`, `BlockerRef`, `IssueTracker` trait |
| Workflow config | `src/config/workflow.rs` | YAML front-matter + Liquid template parsing |

**Agent Supervisor** (`crates/terraphim_agent_supervisor/`): OTP-style supervision trees

| Component | Location | Purpose |
|-----------|----------|---------|
| Supervisor | `src/supervisor.rs` | `AgentSupervisor` with `Arc<RwLock<HashMap<AgentPid, SupervisedAgentInfo>>>` children map |
| Restart strategies | `src/supervisor.rs` | `OneForOne` (restart failed), `OneForAll` (restart all), `RestForOne` (restart failed + later siblings) |
| Restart intensity | `src/supervisor.rs` | `RestartIntensity { max_restarts, time_window }` -- rate-limited restarts |
| Health checks | `src/supervisor.rs` | Background task polling agent health at intervals |
| Agent lifecycle | `src/supervisor.rs` | `spawn_agent`, `stop_agent`, `handle_agent_exit`, `restart_agent`, `restart_all_agents`, `restart_from_agent` |

**Agent Messaging** (`crates/terraphim_agent_messaging/`): Erlang-style message passing

| Component | Location | Purpose |
|-----------|----------|---------|
| Message types | `src/message.rs` | `AgentMessage` enum: Call (sync), Cast (fire-and-forget), Info (system), Reply, Ack |
| Mailbox | `src/mailbox.rs` | `AgentMailbox` with `mpsc::UnboundedSender/Receiver`, bounded check, stats |
| Router | `src/router.rs` | `DefaultMessageRouter` with `Arc<RwLock<HashMap<AgentPid, MailboxSender>>>`, retry task, shutdown signal |
| Delivery | `src/delivery.rs` | `DeliveryManager` with `AtMostOnce`/`AtLeastOnce`/`ExactlyOnce` guarantees, dedup cache, retry candidates |
| Message system | `src/router.rs` | `MessageSystem` combining router + mailbox manager |

**Agent Registry** (`crates/terraphim_agent_registry/`): Agent discovery with KG integration

| Component | Location | Purpose |
|-----------|----------|---------|
| Registry | `src/registry.rs` | `KnowledgeGraphAgentRegistry` with `Arc<RwLock<HashMap<AgentPid, AgentMetadata>>>` |
| Discovery | `src/registry.rs` | `find_by_role`, `find_by_capability`, `find_by_supervisor` |
| Capacity | `src/registry.rs` | `max_agents` limit check on registration |
| Cleanup | `src/registry.rs` | Background task removing stale agents |

**Goal Alignment** (`crates/terraphim_goal_alignment/`): Goal hierarchy and conflict resolution

| Component | Location | Purpose |
|-----------|----------|---------|
| Goal aligner | `src/alignment.rs` | `KnowledgeGraphGoalAligner` with `Arc<RwLock<GoalHierarchy>>` |
| Cycle detection | `src/alignment.rs` | Dependency cycle detection in goal graph |
| Conflict resolution | `src/alignment.rs` | Goal conflict detection and resolution strategies |

**Task Decomposition** (`crates/terraphim_task_decomposition/`): KG-based task breakdown

| Component | Location | Purpose |
|-----------|----------|---------|
| Decomposer | `src/decomposition.rs` | `KnowledgeGraphTaskDecomposer` with concept extraction, connectivity analysis |
| Circular dep check | `src/decomposition.rs` | `has_circular_dependency` DFS-based cycle detection |
| Caching | `src/decomposition.rs` | `Arc<RwLock<HashMap<String, DecompositionResult>>>` decomposition cache |

**Multi-Agent** (`crates/terraphim_multi_agent/`): Agent coordination

| Component | Location | Purpose |
|-----------|----------|---------|
| TerraphimAgent | `src/agent.rs` | Core agent with `Arc<RwLock<AgentStatus>>`, command processing, LLM integration |
| Status machine | `src/agent.rs` | States: Initializing -> Ready <-> Busy -> Error -> Terminating -> Offline |
| Persistence | `src/agent.rs` | `save_state`/`load_state` with `DeviceStorage` |

**KG Orchestration** (`crates/terraphim_kg_orchestration/`): Supervision tree workflow orchestration

| Component | Location | Purpose |
|-----------|----------|---------|
| Scheduler | `src/scheduler.rs` | `TaskScheduler` integrating decomposition + agent pool |
| Supervision tree | `src/supervision.rs` | `SupervisionTreeOrchestrator` combining supervisor + scheduler + coordinator |
| Workflow management | `src/supervision.rs` | `SupervisedWorkflow` with `active_workflows: Arc<RwLock<HashMap>>`, fault recovery |
| Health monitoring | `src/supervision.rs` | Background health check loop, workflow timeout detection |
| Message handling | `src/supervision.rs` | `SupervisionMessage` enum: AgentFailed, AgentRecovered, WorkflowTimeout, HealthCheck, Escalation, Shutdown |

**tlaplus-ts (TypeScript)**: Complete library at `terraphim/tlaplus-ts` on Gitea

| Component | Issue | Status |
|-----------|-------|--------|
| Project scaffold (TypeScript 5.x, tsup, vitest) | #1 | Closed |
| AST types for TLA+ | #2 | Closed |
| Tree-sitter parser wrapper (CST-to-AST) | #3 | Closed |
| Expression evaluator (sets, logic, quantifiers) | #4 | Closed |
| Source code formatter | #5 | Closed |
| TLC CLI bridge for model checking | #6 | Closed |
| CLI with parse/format/validate/check | #7 | Closed |
| Documentation and npm publishing | #8 | Closed |

### Data Flow: Orchestrator Main Loop

```
poll_tick -> fetch_candidate_issues -> sort_for_dispatch -> is_dispatch_eligible -> dispatch_issue
                                                                                        |
                                                                                   [spawn worker]
                                                                                        |
worker_exit_rx <- WorkerExit { outcome: Normal|Failed }
                        |
                   on_worker_exit -> run after_run hook -> schedule_retry (continuation or failure)
                        |
retry_fire_rx <- retry timer fires -> on_retry_timer -> re-fetch + dispatch_issue
                        |
reconcile -> find_stalled_issues -> abort + schedule_retry
           -> fetch_issue_states_by_ids -> determine_action -> TerminateAndCleanup | KeepRunning
```

### State Variables (to model in TLA+)

| Variable | Rust Type | TLA+ Model |
|----------|-----------|------------|
| `running` | `HashMap<String, RunningEntry>` | Function `running: [IssueID -> RunState]` |
| `claimed` | `HashSet<String>` | Set `claimed \subseteq IssueIDs` |
| `retry_attempts` | `HashMap<String, RetryEntry>` | Function `retrying: [IssueID -> RetryInfo]` |
| `completed` | `HashSet<String>` | Set `completed \subseteq IssueIDs` |
| `available_slots` | derived: `max - |running|` | `max_concurrent - Cardinality(DOMAIN running)` |

### Key Invariants Identified from Code

**Symphony Orchestrator:**
1. **No double-dispatch** (`dispatch.rs:39`): `!state.running.contains_key(&issue.id) && !state.is_claimed(&issue.id)`
2. **Slot bound** (`dispatch.rs:44`): `state.available_slots() == 0` prevents over-dispatch
3. **Todo blocker rule** (`dispatch.rs:58-63`): issues in "todo" state blocked by non-terminal blockers cannot dispatch
4. **Claim lifecycle**: claimed on dispatch (`mod.rs:213`), released on terminal reconcile (`mod.rs:721`) or retry not-found (`mod.rs:607`)
5. **Retry cancellation**: old retry aborted when new retry scheduled (`mod.rs:635-636`)

**Agent Supervisor:**
6. **Restart intensity bound**: `max_restarts` within `time_window` (`supervisor.rs:RestartIntensity`)
7. **No orphaned children**: every child has a valid supervisor_id; supervisor tracks all spawned agents
8. **Restart strategy correctness**: OneForOne restarts only the failed agent; OneForAll restarts all; RestForOne restarts failed + those started after it
9. **Health check liveness**: background health task runs at configured intervals

**Agent Messaging:**
10. **No duplicate registration**: `register_agent` rejects duplicate AgentPid (`router.rs:260`)
11. **Mailbox capacity bound**: bounded mailboxes reject sends at capacity (`mailbox.rs:128-132`)
12. **Exactly-once deduplication**: ExactlyOnce mode deduplicates by MessageId (`delivery.rs:158-163`)
13. **Retry bound**: message retry capped at `max_retries` (`delivery.rs:276`)
14. **Delivery status machine**: Pending -> InTransit -> Delivered -> Acknowledged (or Failed/Expired); no backwards transitions

**KG Orchestration:**
15. **Workflow concurrency bound**: `max_concurrent_workflows` checked before starting new workflow (`supervision.rs:598`)
16. **Escalation terminates**: supervisor escalation triggers graceful shutdown of affected workflows (`supervision.rs:934-938`)

## Constraints

### Technical Constraints
- **Java requirement for TLC**: TLC model checker requires JDK 11+ (already available on bigbox)
- **tlaplus-ts bridge**: `TLCBridge` class wraps TLC Java CLI, parsing structured output
- **Bounded model checking**: TLC requires finite state spaces; must bound IssueIDs, max_concurrent, retry counts
- **No temporal logic in evaluator**: the TypeScript evaluator handles expressions but temporal formulas need TLC

### Business Constraints
- **Time**: Model construction estimated at 2-3 days; TLC verification runs in minutes for bounded models
- **Scope**: Focus on orchestrator core (dispatch + reconcile + retry), not runner internals

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| TLC verification time | < 5 minutes | N/A (no spec exists) |
| State space | ~10^6 states (3 issues, 2 agents) | N/A |
| CI integration | Runs in GitHub Actions | tlaplus-ts has vitest runner |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Model the claim lifecycle correctly | Double-dispatch is the most dangerous bug; a claim leak blocks all future dispatch for that issue | Memory: retry loops already observed in production (MEMORY.md) |
| Model dependency blocking | PageRank dispatch with `all_blockers_terminal` is the core scheduling innovation; incorrect modelling invalidates results | `dispatch.rs:58-63` -- this is the blocker rule |
| Keep state space bounded | TLC exhaustive check requires finite domains; overbounding wastes time, underbounding misses bugs | 3 issues + 2 concurrent agents = tractable (~10^6 states) |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Runner internals (Claude Code / Codex protocol) | Worker outcomes are abstracted as Normal/Failed; internal session logic is orthogonal |
| Workspace management | File system operations are idempotent; not a concurrency concern |
| Agent event processing | Event handling updates observability metadata only; no scheduling decisions |
| Token counting | Pure accounting; no state machine impact |
| Config hot-reload | Watcher is separate from dispatch loop |
| Network failures in tracker | Modelled as non-deterministic fetch results (empty or stale) |
| Goal alignment algorithms | Mostly sequential KG analysis; no concurrent shared state worth modelling |
| Task decomposition KG logic | Single-threaded concept extraction and connectivity analysis; cache is RwLock but not safety-critical |
| Agent evolution/learning | Versioned memory/tasks/lessons are write-once-read-many; no concurrent mutation hazards |
| LLM client calls | External service calls are orthogonal to ADF coordination logic |
| VM execution (TerraphimAgent) | Sandboxed execution is isolated from agent lifecycle |
| Agent persistence serialisation | Serialise/deserialise is sequential; no state machine impact |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim/tlaplus-ts` | Required for TypeScript-based TLA+ spec writing and TLC bridge | Low -- all 8 issues closed, library complete |
| Symphony orchestrator code | Source of truth for state machine behaviour | Low -- code is stable, 157 tests passing |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| TLC (Java) | 1.8.0+ | Low -- mature, stable | None (TLC is the standard TLA+ model checker) |
| tree-sitter-tlaplus | Latest | Low -- used by tlaplus-ts | Write TLA+ directly (bypass parser) |
| JDK 11+ | 11+ | Low -- available on bigbox | GraalVM |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| TLC state explosion with too many issues | Medium | High | Start with 3 issues / 2 agents; increase if tractable |
| tlaplus-ts TLC bridge may not parse all TLC output formats | Low | Medium | Test with simple spec first; extend bridge if needed |
| TLA+ spec may not capture all Rust async subtleties | Medium | Medium | Model at message-passing level (channels), not await-point level |
| Java not installed on CI runner | Low | Low | Add JDK setup step to GitHub Actions |

### Open Questions
1. **Is tlaplus-ts cloned locally?** -- Need to check bigbox for working clone
2. **Does TLC bridge support liveness checking?** -- TLC can check liveness but bridge may only parse safety violations
3. **Should the spec model retry backoff timing or just retry attempts?** -- Timing is continuous; model as discrete attempts

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `tokio::select!` is fair (all branches get eventual service) | Tokio documentation confirms fair polling | Starvation properties would be invalid | Yes |
| Worker exit is guaranteed (process terminates or times out) | Code aborts stalled workers, timeouts on turns | Phantom workers consuming slots | Yes (stall detection in reconcile) |
| Gitea API returns consistent state | Tracker refreshes are best-effort with debug fallback | Model would over-approximate safety | Partially (network errors handled) |
| tlaplus-ts TLCBridge parses invariant violations | Issue #6 description says it does | Would need manual TLC output parsing | No -- needs verification |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Model at Rust async/await level | Would need to model every .await point; massive state space | Rejected -- too fine-grained |
| Model at message-passing level (channels) | Natural fit for TLA+; channels map to TLA+ sequences | **Chosen** -- matches orchestrator's channel architecture |
| Model at API call level only | Would miss internal state transitions | Rejected -- too coarse |

## Research Findings

### Key Insights

1. **The orchestrator is fundamentally a message-passing state machine** with 5 event sources in `tokio::select!`: poll tick, worker exit, agent event, retry timer, shutdown. This maps directly to TLA+ actions.

2. **The tlaplus-ts library is complete and production-ready** -- all 8 issues closed, includes AST types, parser, evaluator, formatter, TLC bridge, and CLI. Created on 2026-03-14, last updated 2026-03-17.

3. **The `tla-precheck` approach (kingbootoshi/tla-precheck)** demonstrates a compelling pattern: generate TLA+ from a DSL, run TLC for exhaustive state exploration, then validate that the TypeScript implementation matches the spec. We can adapt this: write the TLA+ spec manually (modelling the Rust orchestrator), then use tlaplus-ts to run TLC and assert properties.

4. **Critical state invariants are already documented in the Rust code** via dispatch eligibility checks. These translate directly to TLA+ invariants:
   - `NoDoubleDispatch == \A i \in IssueIDs: ~(i \in DOMAIN running /\ i \in DOMAIN retrying)`
   - `SlotBound == Cardinality(DOMAIN running) <= MaxConcurrent`
   - `ClaimedSuperset == DOMAIN running \cup DOMAIN retrying \subseteq claimed`

5. **The dependency/blocker rule is the most complex property** -- it involves checking that all blockers of a "todo" issue are in terminal states before dispatch. This requires modelling issue state transitions across the dependency graph.

6. **The ADF has three layers of concurrency worth modelling separately**:
   - **Layer 1 (Symphony)**: Issue dispatch, retry, reconciliation -- the "outer loop" that assigns work
   - **Layer 2 (Supervisor)**: Agent lifecycle with OTP restart strategies -- manages agent failures
   - **Layer 3 (Messaging)**: Message delivery with guarantees -- communication fabric between agents

7. **The supervisor restart strategies are classically verified with TLA+**. The OneForAll and RestForOne strategies have subtle failure modes (restart storms, cascading failures) that are well-suited to bounded model checking. The `RestartIntensity` rate limiter adds a safety bound that should be formally verified.

8. **The messaging delivery guarantees map directly to TLA+ properties**:
   - AtMostOnce: `\A m: delivered(m) => ~redelivered(m)`
   - AtLeastOnce: `\A m: sent(m) ~> delivered(m) \/ failed(m)`
   - ExactlyOnce: AtLeastOnce /\ `\A m: |{d \in deliveries: d.id = m.id}| <= 1`

9. **Three ADF crates have low TLA+ value** (mostly sequential/algorithmic): `terraphim_goal_alignment` (graph algorithms), `terraphim_task_decomposition` (KG analysis with RwLock cache), `terraphim_agent_evolution` (versioned state). These use `Arc<RwLock<>>` but their concurrent access patterns are simple (read-heavy, infrequent writes) and not prone to the invariant violations TLA+ excels at finding.

10. **The KG Orchestration supervision tree** (`supervision.rs`) composes supervisor + scheduler + coordinator into a higher-level workflow manager. Its `max_concurrent_workflows` bound and escalation logic add properties worth verifying alongside the base supervisor.

### Relevant Prior Art
- **tla-precheck** (kingbootoshi): TypeScript DSL -> TLA+ -> TLC bounded model checking. Validates spec/implementation equivalence.
- **Amazon Web Services TLA+ usage**: AWS uses TLA+ to verify distributed protocols (DynamoDB, S3, EBS). Demonstrates real-world value for concurrent systems.
- **Hillel Wayne's "Practical TLA+"**: Standard reference for modelling concurrent systems in TLA+.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Clone tlaplus-ts, run `bun test` to verify it works | Confirm library is functional | 30 minutes |
| Write minimal TLA+ spec (2 issues, 1 agent, dispatch+complete) and run through TLC bridge | Validate end-to-end toolchain | 2 hours |
| Model the full dispatch loop with retry and reconcile | Core specification work | 1 day |
| Add liveness checking (every eligible issue eventually dispatched) | Temporal property verification | 4 hours |

## Recommendations

### Proceed/No-Proceed
**Proceed.** The orchestrator's concurrency model has sufficient complexity to benefit from formal verification, the tooling (tlaplus-ts) is already built and complete, and the state machine structure maps cleanly to TLA+.

### Scope Recommendations

**Three TLA+ modules** (one per concurrency layer, composed for cross-layer properties):

**Module 1 -- Symphony Orchestrator** (dispatch, retry, reconciliation):
1. Phase 1a: Dispatch + worker completion + claim lifecycle (safety invariants)
2. Phase 1b: Retry queue + stall detection + reconciliation
3. Phase 1c: Dependency blocking and PageRank priority ordering
4. Phase 1d: Liveness properties (no starvation, eventual dispatch)

**Module 2 -- Agent Supervisor** (OTP restart strategies):
1. Phase 2a: OneForOne restart + restart intensity bound
2. Phase 2b: OneForAll and RestForOne strategies
3. Phase 2c: Health check integration and escalation termination

**Module 3 -- Messaging Delivery** (delivery guarantees):
1. Phase 3a: AtMostOnce + AtLeastOnce delivery with retry bound
2. Phase 3b: ExactlyOnce deduplication
3. Phase 3c: Mailbox capacity bound + routing table consistency

**Cross-layer composition** (optional Phase 4):
- Compose Symphony dispatch with Supervisor restart to verify that supervisor restart storms do not violate dispatch slot bounds
- Compose Messaging delivery with Supervisor health checks to verify that health check messages are eventually delivered

### Risk Mitigation Recommendations
1. Start with a 3-issue, 2-agent bounded model to keep TLC tractable
2. Validate tlaplus-ts TLC bridge with a trivial spec before investing in full models
3. Write the TLA+ specs in `.tla` files (not TypeScript DSL) for maximum TLC compatibility
4. Add TLC verification to CI as an optional job (not blocking)
5. Keep modules independent initially; compose only after each module passes individually
6. For Supervisor module, bound to 3 children and 2 max restarts to avoid state explosion

## Proposed TLA+ Spec Structure

```
---- MODULE SymphonyOrchestrator ----
EXTENDS Integers, FiniteSets, Sequences

CONSTANTS
    IssueIDs,        \* e.g. {"i1", "i2", "i3"}
    MaxConcurrent,   \* e.g. 2
    MaxRetries,      \* e.g. 3
    TerminalStates,  \* e.g. {"Done", "Closed"}
    ActiveStates,    \* e.g. {"Todo", "InProgress"}
    Dependencies     \* e.g. [i3 |-> {i1, i2}] -- i3 blocked by i1 and i2

VARIABLES
    issueState,      \* [IssueIDs -> {"Todo", "InProgress", "Done", ...}]
    running,         \* SUBSET IssueIDs
    claimed,         \* SUBSET IssueIDs
    retrying,        \* [IssueIDs -> 0..MaxRetries] (partial function)
    completed        \* SUBSET IssueIDs

\* === Actions ===

PollDispatch(i) ==    \* Dispatch eligible issue i
    /\ i \notin running
    /\ i \notin claimed
    /\ issueState[i] \in ActiveStates
    /\ issueState[i] \notin TerminalStates
    /\ Cardinality(running) < MaxConcurrent
    /\ \* Blocker rule: if Todo, all deps must be terminal
       (issueState[i] = "Todo" =>
           \A d \in Dependencies[i]: issueState[d] \in TerminalStates)
    /\ running' = running \cup {i}
    /\ claimed' = claimed \cup {i}
    /\ UNCHANGED <<issueState, retrying, completed>>

WorkerComplete(i) ==  \* Worker finishes successfully
    /\ i \in running
    /\ running' = running \ {i}
    /\ completed' = completed \cup {i}
    /\ issueState' = [issueState EXCEPT ![i] = "Done"]
    /\ claimed' = claimed \ {i}
    /\ UNCHANGED <<retrying>>

WorkerFail(i) ==      \* Worker fails, schedule retry
    /\ i \in running
    /\ running' = running \ {i}
    /\ retrying' = retrying @@ (i :> 1)
    /\ UNCHANGED <<issueState, claimed, completed>>

RetryFire(i) ==       \* Retry timer fires
    /\ i \in DOMAIN retrying
    /\ retrying[i] < MaxRetries
    /\ Cardinality(running) < MaxConcurrent
    /\ running' = running \cup {i}
    /\ retrying' = [retrying EXCEPT ![i] = @ + 1]  \* or remove
    /\ UNCHANGED <<issueState, claimed, completed>>

RetryGiveUp(i) ==     \* Retry exhausted
    /\ i \in DOMAIN retrying
    /\ retrying[i] >= MaxRetries
    /\ claimed' = claimed \ {i}
    /\ retrying' = [d \in DOMAIN retrying \ {i} |-> retrying[d]]
    /\ UNCHANGED <<issueState, running, completed>>

\* === Invariants (Safety) ===

NoDoubleDispatch == \A i \in IssueIDs:
    ~(i \in running /\ i \in DOMAIN retrying /\ retrying[i] > 0)

SlotBound == Cardinality(running) <= MaxConcurrent

ClaimedCovers == running \subseteq claimed

NoTerminalRunning == \A i \in running: issueState[i] \notin TerminalStates

\* === Liveness ===

EventualDispatch == \A i \in IssueIDs:
    (issueState[i] \in ActiveStates /\
     \A d \in Dependencies[i]: issueState[d] \in TerminalStates)
    ~> (i \in completed)

====
```

## Next Steps

If approved:
1. **Spike**: Clone tlaplus-ts on bigbox, verify `bun test` passes
2. **Write TLA+ spec**: Start with Phase 1 (dispatch + complete + claim lifecycle)
3. **Run TLC**: Use tlaplus-ts TLC bridge to model-check with 3 issues, 2 agents
4. **Iterate**: Add retry, reconcile, dependency properties
5. **CI integration**: Add TLC verification as optional CI job
6. **Proceed to Phase 2 (Design)**: Create implementation plan for the spec and test harness

## Appendix

### Reference Materials
- [TLA+ Tools (official)](https://github.com/tlaplus/tlaplus)
- [tla-precheck (spec/code drift prevention)](https://github.com/kingbootoshi/tla-precheck)
- [terraphim/tlaplus-ts on Gitea](https://git.terraphim.cloud/terraphim/tlaplus-ts)
- [Practical TLA+ by Hillel Wayne](https://learntla.com/)

### tlaplus-ts Repository Structure (Gitea)
- **Issues**: 9 total (8 feature + 1 test), all closed
- **Language**: TypeScript
- **Description**: "TypeScript bindings for TLA+ formal specifications"
- **Created**: 2026-03-14
- **Features**: AST types, tree-sitter parser, evaluator, formatter, TLC bridge, CLI

### Symphony Orchestrator Concurrency Model

**Event sources** (5 branches in `tokio::select!`):
1. `poll_tick.tick()` -- periodic poll for new candidates
2. `worker_exit_rx.recv()` -- worker completion/failure
3. `agent_event_rx.recv()` -- session observability updates
4. `retry_fire_rx.recv()` -- retry timer expiry
5. `tokio::signal::ctrl_c()` -- graceful shutdown

**State transitions**:
- Dispatch: `{} -> claimed + running`
- Worker OK: `running -> completed + retry(continuation)`
- Worker Fail: `running -> retry(failure)`
- Retry Fire: `retrying -> running`
- Retry Exhaust: `retrying -> {} (claim released)`
- Reconcile Terminal: `running -> {} (claim released + workspace cleanup)`
- Reconcile Stall: `running -> retrying`
- Shutdown: `running -> {} + retrying -> {}`
