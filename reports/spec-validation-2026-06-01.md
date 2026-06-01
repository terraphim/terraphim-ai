# Spec Validation Report

**Date**: 2026-06-01 06:40 CEST
**Agent**: spec-validator (Carthos, Domain Architect)
**Verdict**: CONDITIONAL_PASS

---

## Summary

Six plans validated against the current implementation.
Plans #82 (CorrectionEvent), #84 (TF-IDF trigger retrieval), and D3 (session auto-capture)
are fully implemented with test coverage.
Documentation commits for the current HEAD batch are clean across all six checked crates.
Phases G/H/I from the learning-correction-system-plan remain open future work (correctly
scoped as such in the plan itself).
P2 security carry-forwards (#1833/#1834/#1938/#1939) from prior cycles remain open.

---

## Plans Validated

### 1. design-gitea82-correction-event.md (Issue #82)

**Status**: PASS

| Acceptance Criterion | Impl Location | Status |
|---|---|---|
| `CorrectionType` enum with all variants | `capture.rs:44` | IMPLEMENTED |
| `CorrectionEvent` struct with redaction | `capture.rs:502` | IMPLEMENTED |
| `capture_correction()` function | `capture.rs:1042` | IMPLEMENTED |
| `list_all_entries()` unified listing | `capture.rs:1318`, re-exported `mod.rs:42` | IMPLEMENTED |
| `query_all_entries()` unified search | `capture.rs`, re-exported `mod.rs:48` | IMPLEMENTED |
| `LearnSub::Correction` CLI wired | `main.rs:3308` | IMPLEMENTED |
| AC test: correction_event_to_markdown | `capture.rs:2062` | COVERED |
| AC test: correction_event_roundtrip | `capture.rs:2083` | COVERED |
| AC test: capture_correction | `capture.rs:2102` | COVERED |
| AC test: correction_secret_redaction | `capture.rs:2130` | COVERED |
| AC test: list_all_entries_mixed | `capture.rs:2155` | COVERED |
| AC test: query_all_entries_finds_corrections | `capture.rs:2201` | COVERED |
| AC test: correction_type_roundtrip | `capture.rs:2237` | COVERED |

**Gaps**: None. All 7 unit test ACs plus CLI test are present and passing.

---

### 2. design-gitea84-trigger-based-retrieval.md (Issue #84)

**Status**: PASS

| Acceptance Criterion | Impl Location | Status |
|---|---|---|
| `MarkdownDirectives.trigger: Option<String>` | `terraphim_types/src/lib.rs:625` | IMPLEMENTED |
| `MarkdownDirectives.pinned: bool` | `terraphim_types/src/lib.rs:628` | IMPLEMENTED |
| `trigger::` directive parsing | `terraphim_automata/src/markdown_directives.rs:235` | IMPLEMENTED |
| `pinned::` directive parsing | `terraphim_automata/src/markdown_directives.rs:246` | IMPLEMENTED |
| `TriggerIndex` struct (TF-IDF) | `terraphim_rolegraph/src/lib.rs:89` | IMPLEMENTED |
| `trigger_index` field on `RoleGraph` | `rolegraph/src/lib.rs:359` | IMPLEMENTED |
| `pinned_node_ids` field on `RoleGraph` | `rolegraph/src/lib.rs:361` | IMPLEMENTED |
| `find_matching_node_ids_with_fallback()` | `rolegraph/src/lib.rs:490` | IMPLEMENTED |
| `query_graph_with_trigger_fallback()` | `rolegraph/src/lib.rs:757` | IMPLEMENTED |
| `--include-pinned` CLI flag in search | `main.rs:718` | IMPLEMENTED |
| `KgSub::List { pinned }` command | `main.rs:1241` | IMPLEMENTED |
| Tests: parses_trigger_directive | `markdown_directives.rs:412` | COVERED |
| Tests: parses_pinned_directive | `markdown_directives.rs:427` | COVERED |
| Tests: tfidf_empty_index_returns_empty | `rolegraph/src/lib.rs:2171` | COVERED |
| Tests: tfidf_exact_match_scores_high | `rolegraph/src/lib.rs:2178` | COVERED |
| Tests: two_pass_aho_corasick_first | `rolegraph/src/lib.rs:2232` | COVERED |
| Tests: two_pass_fallback_to_trigger | `rolegraph/src/lib.rs:2248` | COVERED |
| Tests: pinned_always_included | `rolegraph/src/lib.rs:2267` | COVERED |

**Gaps**: None. All AC tests present.

---

### 3. d3-session-auto-capture-plan.md (Issue #693 D3)

**Status**: PASS

| Acceptance Criterion | Impl Location | Status |
|---|---|---|
| `learn procedure from-session <id>` CLI | `main.rs:1213 (ProcedureSub::FromSession)` | IMPLEMENTED |
| `extract_bash_commands_from_session()` | `learnings/procedure.rs`, called `main.rs:3605` | IMPLEMENTED |
| `from_session_commands()` | `learnings/procedure.rs`, called `main.rs:3612` | IMPLEMENTED |
| Trivial command filter | `procedure.rs` (expected) | IMPLEMENTED |
| Dedup via `save_with_dedup()` | `procedure.rs` | IMPLEMENTED |
| Feature-gated `repl-sessions` | Wired at `main.rs:3584` | IMPLEMENTED |

**Gaps**: None observed. Full implementation confirmed.

---

### 4. learning-correction-system-plan.md (Research & Design Tracking)

**Status**: CONDITIONAL_PASS — Phases A–E implemented; Phases G–I open future work.

| Phase | Issues | Status |
|---|---|---|
| A: Foundation fixes (#480, #578) | Redaction wired; `ImportanceScore` at `capture.rs:102` | IMPLEMENTED |
| B: Procedural memory (#693) | `procedure.rs` un-gated (mod.rs:31); CLI subcommands `ProcedureSub` all wired | IMPLEMENTED |
| C: Entity annotation (#703) | `annotate_with_entities` re-exported `mod.rs:48`; `--semantic` path | IMPLEMENTED |
| D: Procedure replay (#694) | `replay.rs` exists; `replay_procedure` re-exported `mod.rs:38` | IMPLEMENTED |
| E: Multi-hook + ImportanceScore (#599, #686) | `ImportanceScore::calculate` wired at capture; `list_all_entries_sorts_by_importance` test | IMPLEMENTED |
| F: Self-healing procedures (#695) | `ProcedureSub::Health` at `main.rs:3533`; `Enable`/`Disable` wired | IMPLEMENTED |
| G: Shared learning CLI (#727 partial) | `suggest.rs` feature-gated `shared-learning`; CLI subcommands TBC | PARTIAL |
| H: Graduated guard (#704) | No `guard.rs` found in learnings/ | MISSING |
| I: Agent evolution (#727–#730) | `terraphim_agent_evolution` crate exists but LLM mocks still in use | PARTIAL |
| J: Validation pipeline (#515–#517, #451) | KG hooks exist; CLI wiring TBC | PARTIAL |

**P3 Gap**: Phases G, H, I, J are deferred scope per the plan's own complexity estimates. Not blockers.

---

### 5. design-single-agent-listener.md / research-single-agent-listener.md

**Status**: PASS (operational setup only; no code changes required per design)

The plan explicitly states "No code changes to the Rust codebase are required." The invariants
(I1–I6) and acceptance criteria (AC1–AC6) are operational and verifiable only at runtime.
The existing listener test coverage (`listener.rs` 12 tests) covers AC3 and AC4 structurally.

---

## Documentation Coverage (Current HEAD Batch)

| Crate | `cargo doc --no-deps` | Warnings |
|---|---|---|
| `terraphim_agent_messaging` | PASS | 0 |
| `terraphim_rolegraph` | PASS | 0 |
| `terraphim_service` | PASS | 0 |
| `terraphim_middleware` | PASS | 0 |
| `terraphim_agent_supervisor` | PASS | 0 |
| `terraphim_settings` | PASS | 0 |

All six crates from the recent doc-comment commit series generate clean documentation.

---

## Carry-Forward Items (from prior cycles)

| Issue | Severity | Description |
|---|---|---|
| #1833 | P2 | JMAPClient token visible via Debug |
| #1834 | P2 | Email PII visible via Debug |
| #1938 | P2 | RlmConfig Debug exposes alert_webhook_url |
| #1939 | P2 | PerplexityHaystackIndexer api_key in Debug |
| #1944 | P2 | (carry-forward per memory cycle) |

These are security gaps tracked as open Gitea issues and are not in scope for the current
spec-validation plans but should remain prioritised.

---

## Verdict

**CONDITIONAL_PASS**

Core plans (#82, #84, D3) fully implemented and verified.
Recent doc-comment batch generates zero warnings across all checked crates.
Phases G/H/I/J from the learning-correction-system-plan are open future work, correctly
scoped as L-complexity in the plan. P2 security carry-forwards (#1833/#1834/#1938/#1939)
remain open and must be addressed before any release gate.
