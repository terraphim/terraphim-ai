# Implementation Plan: TLA+ Formal Validation of Symphony Orchestrator

**Status**: Draft
**Research Doc**: `.docs/research-tlaplus-symphony-validation.md`
**Author**: Terraphim AI
**Date**: 2026-04-04
**Estimated Effort**: 3 days

## Overview

### Summary
Write a TLA+ formal specification of the Symphony orchestrator's concurrency model, run TLC model checking via the existing `terraphim/tlaplus-ts` TypeScript bindings, and integrate into CI. The spec models dispatch, worker completion/failure, retry scheduling, reconciliation, dependency blocking, and shutdown -- proving safety (no double-dispatch, bounded slots, claim consistency) and liveness (every eligible issue eventually dispatched).

### Approach
Write TLA+ `.tla` files directly (no DSL generation). Use `tlaplus-ts` TLCBridge to invoke TLC from vitest tests. Bounded model: 3 issues, 2 concurrent agents, 3 max retries. Incremental build across 4 phases, each verifiable independently.

### Scope
**In Scope:**
1. TLA+ spec modelling the orchestrator state machine (dispatch, complete, fail, retry, reconcile, shutdown)
2. Safety invariants: NoDoubleDispatch, SlotBound, ClaimedCovers, NoTerminalRunning, RetryBound
3. Liveness properties: EventualDispatch, NoStarvation
4. Dependency blocking rule (`all_blockers_terminal`)
5. TypeScript test harness using tlaplus-ts TLCBridge
6. TLC model configuration (.cfg files)

**Out of Scope:**
- Runner internals (Claude Code / Codex session protocol)
- Workspace filesystem operations
- Agent event processing (observability only)
- Token counting and rate limit tracking
- Config hot-reload / watcher
- Per-state concurrency limits (simplify to global limit)
- PageRank sort order (affects dispatch priority, not correctness)

**Avoid At All Cost** (5/25 elimination):
- Generating TLA+ from Rust code automatically (over-engineering)
- Modelling network failure modes in the tracker API (adds state explosion without verifying orchestrator logic)
- Modelling real time / wallclock (TLA+ stuttering handles fairness; timing is irrelevant to safety)
- Building a custom TLC output parser (tlaplus-ts bridge already does this)
- Attempting to verify the tlaplus-ts library itself (trust it as infrastructure)

## Architecture

### Component Diagram
```
specs/
  symphony/
    SymphonyOrchestrator.tla     -- Main TLA+ spec (all actions, invariants)
    SymphonyOrchestrator.cfg     -- TLC configuration (constants, invariants, properties)
    MC_SymphonyOrchestrator.tla  -- Model-checking wrapper (instantiates constants)
test/
  symphony/
    tlc-safety.test.ts           -- vitest: runs TLC safety check via TLCBridge
    tlc-liveness.test.ts         -- vitest: runs TLC liveness check via TLCBridge
```

All files live in the `terraphim/tlaplus-ts` Gitea repository (extending the existing project).

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Write raw .tla files | Maximum TLC compatibility; TLA+ toolbox support; no translation layer | TypeScript DSL -> TLA+ (adds fragile codegen) |
| Model `claimed` as explicit set | Matches Rust `HashSet<String>` in state.rs; enables claim-leak invariants | Derive from running+retrying (hides bugs in claim management) |
| Model retry as counter per issue | Simpler than modelling timer handles; captures attempt exhaustion | Model with sequence of events (over-complex) |
| Separate continuation vs failure retry | Rust code treats Normal exit differently (attempt=1, is_continuation=true) | Single retry path (misses the continuation-retry subtlety) |
| Use `SUBSET IssueIDs` for running | TLC can enumerate all subsets for small |IssueIDs| (2^3 = 8) | Partial functions (harder TLC symmetry) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Auto-generate TLA+ from Rust AST | Enormous implementation effort; spec would mirror code bugs | 2+ weeks of work for marginal value |
| Model per-state concurrency limits | Adds a dimension per state; global limit suffices for core properties | State explosion from extra dimension |
| Model stall detection timing | Continuous time doesn't map to TLA+; stall is just "environment aborts worker" | Infinite state space from real-valued time |
| Equivalence checking (tla-precheck style) | Requires maintaining a parallel TypeScript state machine | Double the maintenance for one codebase |
| Model agent events | Events update observability metadata only; no scheduling decisions | Irrelevant state transitions inflate model |

### Simplicity Check

> **What if this could be easy?**

Write one `.tla` file with ~8 actions mapping 1:1 to the Rust orchestrator's event handlers. Run TLC with 3 issues and 2 slots. If it passes, we have mathematical proof of safety. If it fails, TLC gives us a counterexample trace showing exactly how the bug manifests.

The simplest design: one spec file, one config file, two test files. No code generation, no DSLs, no frameworks.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimisation

## TLA+ State Machine Design

### State Variables

```tla
VARIABLES
    issueState,    \* [IssueIDs -> States]  -- tracker state per issue
    running,       \* SUBSET IssueIDs       -- currently executing
    claimed,       \* SUBSET IssueIDs       -- claimed (running + retrying)
    retrying,      \* [IssueIDs -> Nat]     -- retry attempt counter (partial fn)
    completed,     \* SUBSET IssueIDs       -- bookkeeping
    shutdownFlag   \* BOOLEAN               -- graceful shutdown requested
```

### Constants

```tla
CONSTANTS
    IssueIDs,        \* {"i1", "i2", "i3"}
    MaxConcurrent,   \* 2
    MaxRetries,      \* 3
    ActiveStates,    \* {"Todo"}
    TerminalStates,  \* {"Done", "Closed"}
    Deps             \* [IssueIDs -> SUBSET IssueIDs]  e.g. [i1 |-> {}, i2 |-> {}, i3 |-> {i1, i2}]
```

### Actions (8 total, mapping to Rust event handlers)

| # | Action | Rust Source | Pre-conditions | Post-conditions |
|---|--------|-------------|----------------|-----------------|
| 1 | `PollDispatch(i)` | `on_tick` + `dispatch_issue` | i not claimed, active state, slots available, blockers terminal | running' + claimed' includes i |
| 2 | `WorkerCompleteOK(i)` | `on_worker_exit` Normal branch | i in running | running' removes i, completed' adds i, continuation retry scheduled |
| 3 | `WorkerFail(i)` | `on_worker_exit` Failed branch | i in running | running' removes i, retrying' adds i with attempt+1 |
| 4 | `RetryFire(i)` | `on_retry_timer` issue found | i in retrying, attempts < max, slots available | running' adds i, retrying' removes i |
| 5 | `RetryGiveUp(i)` | `on_retry_timer` not found OR exhausted | i in retrying, attempts >= max OR issue gone | claimed' removes i, retrying' removes i |
| 6 | `ReconcileTerminal(i)` | `reconcile` TerminateAndCleanup | i in running, issueState[i] terminal | running' removes i, claimed' removes i |
| 7 | `ReconcileStall(i)` | `reconcile` find_stalled | i in running, stall detected | running' removes i, retrying' adds i |
| 8 | `Shutdown` | ctrl-c handler | shutdownFlag = FALSE | all running/retrying cleared, shutdownFlag = TRUE |

Plus an **environment action** for external state changes:
| 9 | `TrackerStateChange(i, s)` | External (Gitea) | issueState[i] != s | issueState' updates i to s |

### Invariants (Safety Properties)

```tla
\* INV1: No issue is simultaneously running AND in the retry queue
NoDoubleDispatch == \A i \in IssueIDs:
    ~(i \in running /\ i \in DOMAIN retrying)

\* INV2: Number of running issues never exceeds MaxConcurrent
SlotBound == Cardinality(running) <= MaxConcurrent

\* INV3: Every running issue is claimed
ClaimedCoversRunning == running \subseteq claimed

\* INV4: Every retrying issue is claimed
ClaimedCoversRetrying == {i \in DOMAIN retrying : TRUE} \subseteq claimed

\* INV5: No running issue has a terminal tracker state
NoTerminalRunning == \A i \in running:
    issueState[i] \notin TerminalStates

\* INV6: Retry attempts are bounded
RetryBound == \A i \in DOMAIN retrying:
    retrying[i] <= MaxRetries

\* INV7: Dependency rule -- no running todo issue has non-terminal deps
DepRule == \A i \in running:
    (issueState[i] = "Todo") =>
        \A d \in Deps[i]: issueState[d] \in TerminalStates

\* Combined safety invariant for TLC
SafetyInvariant ==
    /\ NoDoubleDispatch
    /\ SlotBound
    /\ ClaimedCoversRunning
    /\ ClaimedCoversRetrying
    /\ NoTerminalRunning
    /\ RetryBound
    /\ DepRule
```

### Liveness Properties (Temporal)

```tla
\* PROP1: Every eligible issue is eventually dispatched or completed
\* (weak fairness on PollDispatch needed)
EventualDispatch == \A i \in IssueIDs:
    [](issueState[i] \in ActiveStates /\ i \notin claimed /\
       \A d \in Deps[i]: issueState[d] \in TerminalStates)
    ~> (i \in completed \/ i \in running)

\* PROP2: No issue is stuck in retry forever
NoRetryStarvation == \A i \in IssueIDs:
    [](i \in DOMAIN retrying) ~> (i \notin DOMAIN retrying)
```

### Known Bug Discovery

During design, I identified a likely bug in the Rust code at `orchestrator/mod.rs:496-501`:

```rust
// In on_worker_exit, Failed branch:
let current_attempt = self.state.running.get(&exit.issue_id) // BUG: running.remove() already called at line 452
    .and_then(|e| e.retry_attempt)
    .unwrap_or(0);
```

The `running.remove(issue_id)` at line 452 executes before the match, so `self.state.running.get(&exit.issue_id)` will always return `None`, making `current_attempt` always `0`. This means every failure retry starts at attempt 1, never incrementing. The TLA+ model should capture the INTENDED behaviour (incrementing attempts) and separately verify the Rust code matches.

## File Changes

### New Files (in terraphim/tlaplus-ts repo)

| File | Purpose |
|------|---------|
| `specs/symphony/SymphonyOrchestrator.tla` | Main TLA+ specification |
| `specs/symphony/SymphonyOrchestrator.cfg` | TLC model config (constants, invariants) |
| `specs/symphony/MC_SymphonyOrchestrator.tla` | Model-checking wrapper with concrete constants |
| `test/symphony/tlc-safety.test.ts` | vitest: safety invariant verification via TLCBridge |
| `test/symphony/tlc-liveness.test.ts` | vitest: liveness property verification via TLCBridge |
| `specs/symphony/README.md` | Spec documentation and property catalogue |

### Modified Files (in terraphim/tlaplus-ts repo)

| File | Changes |
|------|---------|
| `package.json` | Add `test:symphony` script |

### No Deleted Files

## Test Strategy

### TLC Model Checking Tests

| Test | File | Properties Checked |
|------|------|--------------------|
| `safety with 3 issues 2 agents` | `tlc-safety.test.ts` | All 7 safety invariants (NoDoubleDispatch, SlotBound, ClaimedCoversRunning, ClaimedCoversRetrying, NoTerminalRunning, RetryBound, DepRule) |
| `safety with 2 issues 1 agent` | `tlc-safety.test.ts` | Same invariants, smaller model (smoke test) |
| `liveness with 3 issues 2 agents` | `tlc-liveness.test.ts` | EventualDispatch, NoRetryStarvation |
| `dependency cascade 3 issues` | `tlc-safety.test.ts` | DepRule with i3 blocked by i1 and i2 |

### Test Configuration

```typescript
// tlc-safety.test.ts
import { TLCBridge } from '../src/bridge/index.js';

describe('Symphony Orchestrator Safety', () => {
  it('verifies safety invariants with 3 issues 2 agents', async () => {
    const bridge = new TLCBridge();
    const result = await bridge.check(
      'specs/symphony/SymphonyOrchestrator.tla',
      'specs/symphony/SymphonyOrchestrator.cfg',
      { workers: 4, deadlockDetection: true }
    );
    expect(result.outcome).toBe('no_error');
    expect(result.violations).toHaveLength(0);
  }, 300_000); // 5 minute timeout for TLC
});
```

### Expected TLC Output

For a passing model: `Model checking completed. No error has been found.`
For an invariant violation: counterexample trace showing the sequence of actions leading to violation.

## Implementation Steps

### Step 1: Verify tlaplus-ts Toolchain (Spike)
**Files:** None (verification only)
**Description:** Clone `terraphim/tlaplus-ts` on bigbox, run `bun install && bun test`, confirm TLCBridge works with a trivial spec.
**Tests:** Existing tlaplus-ts test suite passes
**Estimated:** 1 hour

### Step 2: Core Spec -- Dispatch + Complete + Claim (Phase 1)
**Files:** `specs/symphony/SymphonyOrchestrator.tla`, `specs/symphony/SymphonyOrchestrator.cfg`
**Description:** Write the TLA+ spec with 3 actions: `PollDispatch`, `WorkerCompleteOK`, `TrackerStateChange`. Define `NoDoubleDispatch`, `SlotBound`, `ClaimedCoversRunning` invariants. No retry or reconciliation yet.
**Tests:** Manual TLC run via CLI: `tlaplus check specs/symphony/SymphonyOrchestrator.tla --config specs/symphony/SymphonyOrchestrator.cfg`
**Dependencies:** Step 1
**Estimated:** 3 hours

Key TLA+ code:
```tla
Init ==
    /\ issueState = [i \in IssueIDs |-> "Todo"]
    /\ running = {}
    /\ claimed = {}
    /\ completed = {}
    /\ retrying = <<>>  \* empty function
    /\ shutdownFlag = FALSE

PollDispatch(i) ==
    /\ ~shutdownFlag
    /\ i \notin claimed
    /\ issueState[i] \in ActiveStates
    /\ issueState[i] \notin TerminalStates
    /\ Cardinality(running) < MaxConcurrent
    /\ (issueState[i] = "Todo" =>
        \A d \in Deps[i]: issueState[d] \in TerminalStates)
    /\ running' = running \cup {i}
    /\ claimed' = claimed \cup {i}
    /\ UNCHANGED <<issueState, retrying, completed, shutdownFlag>>
```

### Step 3: Add Retry + Failure (Phase 2)
**Files:** `specs/symphony/SymphonyOrchestrator.tla` (extend)
**Description:** Add `WorkerFail`, `RetryFire`, `RetryGiveUp` actions. Add `RetryBound`, `ClaimedCoversRetrying` invariants. Model continuation retry on Normal completion.
**Tests:** TLC run with retry-specific scenarios
**Dependencies:** Step 2
**Estimated:** 3 hours

Key addition:
```tla
WorkerFail(i) ==
    /\ i \in running
    /\ running' = running \ {i}
    /\ LET attempt == IF i \in DOMAIN retrying THEN retrying[i] + 1 ELSE 1
       IN retrying' = [j \in (DOMAIN retrying \cup {i}) |->
                         IF j = i THEN attempt ELSE retrying[j]]
    /\ UNCHANGED <<issueState, claimed, completed, shutdownFlag>>
```

### Step 4: Add Reconciliation + Shutdown (Phase 2 continued)
**Files:** `specs/symphony/SymphonyOrchestrator.tla` (extend)
**Description:** Add `ReconcileTerminal`, `ReconcileStall`, `Shutdown` actions. Add `NoTerminalRunning` invariant.
**Tests:** TLC run including reconciliation paths
**Dependencies:** Step 3
**Estimated:** 2 hours

### Step 5: Add Dependency Blocking (Phase 3)
**Files:** `specs/symphony/SymphonyOrchestrator.tla` (extend), `specs/symphony/MC_SymphonyOrchestrator.tla`
**Description:** Wire up `Deps` constant. Add `DepRule` invariant. Create MC wrapper with concrete dependency graph: i3 depends on {i1, i2}.
**Tests:** TLC with dependency scenario; verify i3 cannot dispatch until i1 and i2 are Done
**Dependencies:** Step 4
**Estimated:** 2 hours

### Step 6: Add Liveness Properties (Phase 4)
**Files:** `specs/symphony/SymphonyOrchestrator.tla` (extend), `specs/symphony/SymphonyOrchestrator.cfg` (add PROPERTIES)
**Description:** Add `EventualDispatch` and `NoRetryStarvation` temporal properties. Add weak fairness constraints on `PollDispatch` and `RetryFire`.
**Tests:** TLC liveness check (may need `-lncheck` flag)
**Dependencies:** Step 5
**Estimated:** 3 hours

### Step 7: TypeScript Test Harness
**Files:** `test/symphony/tlc-safety.test.ts`, `test/symphony/tlc-liveness.test.ts`, `package.json`
**Description:** Write vitest tests that invoke TLCBridge. Assert `result.outcome === 'no_error'` and `result.violations.length === 0`.
**Tests:** `bun test test/symphony/`
**Dependencies:** Step 6
**Estimated:** 2 hours

### Step 8: Documentation
**Files:** `specs/symphony/README.md`
**Description:** Document the spec structure, property catalogue, how to run, how to extend, and the known bug discovery.
**Dependencies:** Step 7
**Estimated:** 1 hour

## TLC Configuration

### Safety Config (`SymphonyOrchestrator.cfg`)
```
CONSTANTS
    IssueIDs = {i1, i2, i3}
    MaxConcurrent = 2
    MaxRetries = 3
    ActiveStates = {"Todo"}
    TerminalStates = {"Done", "Closed"}
    Deps = (i1 :> {} @@ i2 :> {} @@ i3 :> {i1, i2})

INIT Init
NEXT Next

INVARIANTS
    SafetyInvariant

CHECK_DEADLOCK TRUE
```

### Liveness Config (additional)
```
PROPERTIES
    EventualDispatch
    NoRetryStarvation

\* Fairness: all dispatch and retry actions are weakly fair
CONSTRAINT
    Cardinality(completed) <= Cardinality(IssueIDs)
```

## Dependencies

### New Dependencies
| Package | Version | Justification |
|---------|---------|---------------|
| None | -- | tlaplus-ts already has TLCBridge, vitest, tree-sitter |

### External Requirements
| Tool | Version | Location |
|------|---------|----------|
| JDK | 11+ | bigbox: pre-installed |
| TLC | 1.8.0+ | Downloaded by TLCBridge or manual install |

## Performance Considerations

### Expected TLC Performance
| Model | Issues | Agents | Max Retries | Est. States | Est. Time |
|-------|--------|--------|-------------|-------------|-----------|
| Smoke | 2 | 1 | 2 | ~10^4 | < 10s |
| Standard | 3 | 2 | 3 | ~10^6 | < 2 min |
| Extended | 4 | 3 | 3 | ~10^8 | < 30 min |

### Mitigation for State Explosion
- Start with smoke model, graduate to standard
- Use symmetry sets if TLC supports it for IssueIDs
- Constrain `completed` cardinality as state-space bound

## Rollback Plan

All changes are in the separate `terraphim/tlaplus-ts` repository. No changes to Symphony Rust code. If verification reveals bugs in the Rust code, those are filed as Gitea issues against `terraphim/terraphim-ai`.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify tlaplus-ts clone works on bigbox | Pending | Step 1 |
| Confirm TLCBridge handles liveness output | Pending | Step 6 |
| File Gitea issue for retry attempt bug (mod.rs:496-501) | Pending | After Step 3 |

## Approval

- [ ] Research document approved
- [ ] Implementation plan reviewed
- [ ] File structure agreed
- [ ] Safety invariants catalogue complete
- [ ] Liveness properties catalogue complete
- [ ] Human approval received
