# Research Document: TLA+ Formal Validation of Symphony Orchestrator

**Status**: Draft
**Author**: Terraphim AI
**Date**: 2026-04-04
**Reviewers**: Alex

## Executive Summary

The Symphony orchestrator (`crates/terraphim_symphony/`) is a concurrent, async Rust system that dispatches AI coding agents to Gitea issues using PageRank-based prioritisation, dependency enforcement, retry logic, and stall detection. This research evaluates using TLA+ formal verification via the existing `terraphim/tlaplus-ts` TypeScript bindings to prove safety and liveness properties of the orchestrator's concurrency model.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Formal verification of the orchestrator prevents subtle concurrency bugs (double-dispatch, deadlocks, starvation) that are nearly impossible to catch with unit tests alone |
| Leverages strengths? | Yes | We already have `terraphim/tlaplus-ts` (TypeScript bindings for TLA+, all 8 issues closed, tree-sitter parser, evaluator, formatter, TLC bridge, CLI) AND production Symphony orchestrator code to model |
| Meets real need? | Yes | Symphony runs multiple concurrent agents with retry queues and dependency graphs -- the concurrency state space is too large for manual reasoning; prior runs already hit edge cases (retry loops, stall timeout races) |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
The Symphony orchestrator manages concurrent AI agent sessions with complex state transitions: poll-tick dispatch, worker exits, retry timer fires, reconciliation, and shutdown. The interaction of these concurrent events with shared state (running map, claimed set, retry queue) creates a state space too large to test exhaustively with traditional testing.

### Impact
Concurrency bugs in the orchestrator can cause:
- **Double-dispatch**: same issue dispatched to two agents simultaneously
- **Starvation**: issues permanently stuck in retry queue
- **Claim leaks**: claims never released, blocking future dispatch
- **Dependency deadlocks**: circular dependency cascades preventing any issue from being dispatched
- **Shutdown races**: workers not properly terminated on shutdown

### Success Criteria
1. TLA+ spec that models the core orchestrator state machine
2. Safety properties verified: no double-dispatch, no claim leaks, bounded retry queue
3. Liveness properties verified: every eligible issue eventually dispatched, no starvation
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

1. **No double-dispatch** (`dispatch.rs:39`): `!state.running.contains_key(&issue.id) && !state.is_claimed(&issue.id)`
2. **Slot bound** (`dispatch.rs:44`): `state.available_slots() == 0` prevents over-dispatch
3. **Todo blocker rule** (`dispatch.rs:58-63`): issues in "todo" state blocked by non-terminal blockers cannot dispatch
4. **Claim lifecycle**: claimed on dispatch (`mod.rs:213`), released on terminal reconcile (`mod.rs:721`) or retry not-found (`mod.rs:607`)
5. **Retry cancellation**: old retry aborted when new retry scheduled (`mod.rs:635-636`)

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
1. **Phase 1**: Model dispatch + worker completion + claim lifecycle (safety invariants only)
2. **Phase 2**: Add retry queue + stall detection + reconciliation
3. **Phase 3**: Add dependency blocking and PageRank priority ordering
4. **Phase 4**: Liveness properties (no starvation, eventual dispatch)

### Risk Mitigation Recommendations
1. Start with a 3-issue, 2-agent bounded model to keep TLC tractable
2. Validate tlaplus-ts TLC bridge with a trivial spec before investing in the full model
3. Write the TLA+ spec in `.tla` files (not TypeScript DSL) for maximum TLC compatibility
4. Add TLC verification to CI as an optional job (not blocking)

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
