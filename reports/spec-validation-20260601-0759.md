# Spec Validation Report

**Date**: 2026-06-01 07:59 CEST
**Agent**: spec-validator (Carthos, Domain Architect)
**Verdict**: CONDITIONAL_PASS

---

## Summary

All six plans validated. Plans #82, #84, and D3 remain fully implemented with test coverage
unchanged since the 06:40 cycle. The learning-correction-system-plan is CONDITIONAL_PASS
(Phases G/H/I/J explicitly deferred). The single-agent-listener plan is PASS (operational
only; 9 structural tests in listener.rs cover AC3 and AC4).

**New finding**: commit `b84d0bd1` ("docs(terraphim_orchestrator): add missing doc comments
phase 2") improved coverage but left 16 unresolved link warnings in the lib and bin docs.
Filed as P3 gap below.

---

## Plans Validated

### 1. design-gitea82-correction-event.md (Issue #82)

**Status**: PASS

| Acceptance Criterion | Impl Location | Status |
|---|---|---|
| `CorrectionType` enum | `capture.rs:44` | IMPLEMENTED |
| `CorrectionEvent` struct (redaction) | `capture.rs:502` | IMPLEMENTED |
| `capture_correction()` | `capture.rs:1042` | IMPLEMENTED |
| `list_all_entries()` | `capture.rs:1318`, `mod.rs:42` | IMPLEMENTED |
| `query_all_entries()` | `capture.rs`, `mod.rs:48` | IMPLEMENTED |
| `LearnSub::Correction` CLI | `main.rs:3308` | IMPLEMENTED |
| 7 AC unit tests | `capture.rs:2062-2237` | COVERED |

**Gaps**: None. Unchanged from prior cycle.

---

### 2. design-gitea84-trigger-based-retrieval.md (Issue #84)

**Status**: PASS

| Acceptance Criterion | Impl Location | Status |
|---|---|---|
| `MarkdownDirectives.trigger` | `terraphim_types/src/lib.rs:625` | IMPLEMENTED |
| `MarkdownDirectives.pinned` | `terraphim_types/src/lib.rs:628` | IMPLEMENTED |
| `trigger::` directive parsing | `markdown_directives.rs:235` | IMPLEMENTED |
| `pinned::` directive parsing | `markdown_directives.rs:246` | IMPLEMENTED |
| `TriggerIndex` struct (TF-IDF) | `rolegraph/src/lib.rs:89` | IMPLEMENTED |
| `find_matching_node_ids_with_fallback()` | `rolegraph/src/lib.rs:490` | IMPLEMENTED |
| `query_graph_with_trigger_fallback()` | `rolegraph/src/lib.rs:757` | IMPLEMENTED |
| `--include-pinned` CLI flag | `main.rs:718` | IMPLEMENTED |
| `KgSub::List { pinned }` | `main.rs:1241` | IMPLEMENTED |
| 5 AC tests (trigger_index_tests.rs + lib.rs) | `rolegraph/tests/trigger_index_tests.rs` | COVERED |

**Gaps**: None. Unchanged from prior cycle.

---

### 3. d3-session-auto-capture-plan.md (Issue #693 D3)

**Status**: PASS

| Acceptance Criterion | Impl Location | Status |
|---|---|---|
| `learn procedure from-session <id>` CLI | `main.rs:1213` (`ProcedureSub::FromSession`) | IMPLEMENTED |
| `extract_bash_commands_from_session()` | `learnings/procedure.rs` | IMPLEMENTED |
| `from_session_commands()` | `learnings/procedure.rs` | IMPLEMENTED |
| Trivial command filter | `procedure.rs` | IMPLEMENTED |
| Dedup via `save_with_dedup()` | `procedure.rs` | IMPLEMENTED |
| Feature gate `repl-sessions` | `main.rs:3584` | IMPLEMENTED |

**Gaps**: None. Unchanged from prior cycle.

---

### 4. learning-correction-system-plan.md (Research & Design)

**Status**: CONDITIONAL_PASS — Phases A–F implemented; G–J explicitly deferred

| Phase | Issues | Status |
|---|---|---|
| A: Foundation fixes (#480, #578) | Redaction wired; `ImportanceScore` in `capture.rs` | IMPLEMENTED |
| B: Procedural memory (#693) | `procedure.rs` un-gated; all `ProcedureSub` variants wired | IMPLEMENTED |
| C: Entity annotation (#703) | `annotate_with_entities` re-exported `mod.rs:48`; `--semantic` path | IMPLEMENTED |
| D: Procedure replay (#694) | `replay.rs` exists; `replay_procedure` re-exported | IMPLEMENTED |
| E: Multi-hook + ImportanceScore (#599, #686) | `ImportanceScore::calculate` wired; sort test present | IMPLEMENTED |
| F: Self-healing procedures (#695) | `ProcedureSub::Health`, `Enable`, `Disable` wired | IMPLEMENTED |
| G: Shared learning CLI (#727 partial) | `suggest.rs` gated `#[cfg(feature = "shared-learning")]` (mod.rs:34); CLI subcommands TBC | PARTIAL |
| H: Graduated guard (#704) | `guard.rs` not present in `src/learnings/` | MISSING (deferred) |
| I: Agent evolution (#727–#730) | `terraphim_agent_evolution` crate exists; impl status pending | PARTIAL (deferred) |
| J: Validation pipeline (#515–#517) | `ValidationPipeline` symbol not found; KG hooks exist | PARTIAL (deferred) |

**P3 Gaps**: Phases G, H, I, J are explicitly marked as L-complexity future work in the plan.
Not blockers for current cycle. Carry forward to next iteration.

---

### 5. design-single-agent-listener.md + research-single-agent-listener.md

**Status**: PASS (operational setup only; no Rust code changes required per plan)

The plan explicitly states "No code changes to the Rust codebase are required."
Structural test coverage in `crates/terraphim_agent/src/listener.rs` (9 test functions)
covers the codeable acceptance criteria:

| AC | Coverage |
|---|---|
| AC3: Ignores own comments | `listener_runtime_ignores_self_authored_comments` (line 494) |
| AC4: Survives transient errors | `listener_runtime_retries_transient_claim_failures_without_advancing_cursor` (line 897) |
| AC1/AC2/AC5/AC6 | Operational — verifiable only at runtime via tmux/1Password |

---

## New Commit Impact (b84d0bd1)

**Commit**: `docs(terraphim_orchestrator): add missing doc comments phase 2`

**Finding**: The commit added doc comments but 16 unresolved link warnings remain:

```
terraphim_orchestrator (lib): 15 warnings
  - CommandRunner, TokioCommandRunner, PrGateSnapshot, PrGateDecision
  - PrSummary (x2), PrComment (x2), PrTracker, GiteaPrTracker
  - evaluate_pr_verdict, EvaluationOutcome, PrPollRateLimiter
  - AutoMergeDedupeSet, OrchestratorEvent
terraphim_orchestrator (bin adf-ctl): 1 warning (unclosed HTML tag)
```

These are broken doc links referencing types that are not in scope from the public
doc surface of `terraphim_orchestrator`. The referenced types exist in the crate but
are apparently private or in a different module scope.

**Severity**: P3 — doc quality gap, does not affect runtime behaviour or test coverage.
**Action**: File new Gitea issue for Phase 3 doc completion.

---

## Carry-Forward Items

| Issue | Severity | Description |
|---|---|---|
| #1833 | P2 | JMAPClient token visible via Debug |
| #1834 | P2 | Email PII visible via Debug |
| #1938 | P2 | RlmConfig Debug exposes alert_webhook_url |
| #1939 | P2 | PerplexityHaystackIndexer api_key in Debug |
| #1944 | P2 | (carry-forward per prior memory cycle) |
| NEW | P3 | `terraphim_orchestrator` 16 unresolved doc links after b84d0bd1 |

P2 items must be addressed before any release gate. P3 is a follow-up.

---

## Verdict

**CONDITIONAL_PASS**

No regression from prior cycle (06:40 CEST). All core plans (#82, #84, D3) remain fully
implemented and tested. The learning-correction-system-plan's deferred phases (G/H/I/J)
are correctly scoped as future work in the plan itself — not blockers. The single-agent-listener
plan has adequate structural coverage.

One new P3 gap introduced: commit b84d0bd1 improved doc coverage for `terraphim_orchestrator`
but left 16 unresolved link warnings, indicating the doc comment links reference private or
out-of-scope types. This should be resolved in a follow-up commit before the next doc-quality
gate.

P2 security carry-forwards (#1833/#1834/#1938/#1939) remain open and must be prioritised
before any release.
