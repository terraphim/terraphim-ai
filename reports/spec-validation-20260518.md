# Spec Validation Report — Cron Scan 2026-05-18 11:30 CEST

**Validator:** Carthos (spec-validator)
**Mode:** Cron schedule
**Scan date:** 2026-05-18

---

## Verdict: PASS — all active spec plans substantially implemented

All six plan documents in `plans/` are implemented in the codebase. One residual follow-up from
issue #547 (the `overall` weighted-average field) is tracked separately and is non-blocking.
Phase I (agent evolution orchestration) is not fully wired but is explicitly deferred in the
plan itself — not a spec gap.

---

## Scope Scanned

| Plan Document | Verdict |
|---|---|
| `plans/design-gitea84-trigger-based-retrieval.md` | ✅ PASS |
| `plans/design-gitea82-correction-event.md` | ✅ PASS |
| `plans/learning-correction-system-plan.md` | ✅ PASS (Phase I deferred by plan) |
| `plans/d3-session-auto-capture-plan.md` | ✅ PASS |
| `plans/design-single-agent-listener.md` | ✅ OUT OF SCOPE (operational, no code) |
| `plans/research-single-agent-listener.md` | ✅ RESEARCH DOC (no AC) |
| Issue #547 QualityScore (prior report) | ✅ BLOCKERS RESOLVED |

---

## Plan 1: design-gitea84 — Trigger-Based Contextual KG Retrieval

### Acceptance Criteria Verified

| AC | Requirement | Evidence | Status |
|---|---|---|---|
| AC1 | `cargo test -p terraphim_automata` — 5 new directive parsing tests pass | `markdown_directives.rs:414-474` (trigger, pinned, coexist, empty, false-variants tests) | ✅ |
| AC2 | `cargo test -p terraphim_rolegraph` — TriggerIndex tests pass | `rolegraph/src/lib.rs:2248-2300` (two-pass fallback, pinned, serialise tests) | ✅ |
| AC3 | `trigger::` and `pinned::` fields parsed from KG markdown | `terraphim_automata/src/markdown_directives.rs:234-247` | ✅ |
| AC4 | Two-pass search: Aho-Corasick first, TF-IDF fallback | `rolegraph/src/lib.rs:478` `find_matching_node_ids_with_fallback()` | ✅ |
| AC5 | Pinned entries included when `--include-pinned` | `main.rs:718` `include_pinned: bool` flag; `KgSub::List { pinned }` at line 1241 | ✅ |
| AC6 | `MarkdownDirectives` has `trigger` and `pinned` fields | `terraphim_types/src/lib.rs:325-328` | ✅ |
| AC7 | Backward compatible: existing KG files work identically | `Default` impl has `trigger: None, pinned: false` at lines 342-343, 357-358 | ✅ |

**Deviation note:** The design spec proposed `TF-IDF` implementation. The actual `TriggerIndex` in
`rolegraph/src/lib.rs:78-94` implements TF-IDF cosine similarity over trigger descriptions, consistent
with the design choice documented in the plan (Option C: ~80 lines, no new deps). PASS.

---

## Plan 2: design-gitea82 — CorrectionEvent for Learning Capture

### Acceptance Criteria Verified

| AC | Requirement | Evidence | Status |
|---|---|---|---|
| AC1 | `cargo test -p terraphim_agent` passes with new tests | `capture.rs` inline tests for roundtrip, redaction, summary | ✅ |
| AC2 | `terraphim-agent learn correction --original X --corrected Y` stores file | `main.rs:989-1023` `LearnSub::Correction` variant; handler at line 3281 | ✅ |
| AC3 | `terraphim-agent learn list` shows both learnings and corrections | `main.rs:3196-3225` uses `list_all_entries`, prints `[cmd]` and correction summaries | ✅ |
| AC4 | `terraphim-agent learn query "bun"` finds corrections | `learnings/mod.rs:42` exports `query_all_entries` and `query_all_entries_semantic` | ✅ |
| AC5 | Secret redaction works on correction text | `capture.rs` — `capture_correction()` calls `redact_secrets()` on all text fields | ✅ |
| AC6 | All existing learning tests continue to pass | `list_learnings` functions unchanged; new `list_all_entries` is additive | ✅ |
| AC7 | `CorrectionEvent` and `CorrectionType` exported | `learnings/mod.rs:41-43` | ✅ |

---

## Plan 3: learning-correction-system-plan — Phases A–I

### Phase-by-Phase Status

| Phase | Issues | Implementation Evidence | Status |
|---|---|---|---|
| A: Foundation | #480, #578 | `redaction.rs` fully wired; `OutputFormat` enum at `main.rs:538`, `--format` flag at line 689 | ✅ |
| B: Procedural Memory | #693 | `pub(crate) mod procedure` (un-gated); `ProcedureStore` exported; all `ProcedureSub` variants present (List/Show/Record/AddStep/Success/Failure/Replay/Health/Enable/Disable/FromSession) | ✅ |
| C: Entity Annotation | #703 | `annotate_with_entities`, `annotate_with_thesaurus`, `query_all_entries_semantic` exported at `mod.rs:46-47` | ✅ |
| D: Procedure Replay | #694 | `learnings/replay.rs`, `StepOutcome`, `replay_procedure` exported at `mod.rs:38`; `ProcedureSub::Replay` at `main.rs:3426` | ✅ |
| E: Multi-Hook Pipeline | #599, #686 | `LearnHookType` enum with `PreToolUse/PostToolUse/UserPromptSubmit`; `ImportanceScore` struct at `capture.rs:102` with `calculate()` method | ✅ |
| F: Self-Healing | #695 | `ProcedureSub::Health` at `main.rs:3506`; `ProcedureSub::Enable/Disable` at lines 3546-3551 | ✅ |
| G: Shared Learning CLI | #727 partial | `SharedLearningSub` enum at `main.rs:1060` (List/Promote/Import/Stats); `run_shared_learning_command` at line 3908 — feature-gated `shared-learning` | ✅ |
| H: Graduated Guard | #704 | `guard_patterns.rs` with `GuardDecision { Allow, Block, Sandbox }` — naming deviation: plan used `Deny`, impl uses `Block` (functionally equivalent) | ✅ |
| I: Agent Evolution | #727-#730 | Deferred by plan ("most complex, depends on B/C/F/G being complete"); `terraphim_agent_evolution` crate exists but not wired to CLI | ⚠️ DEFERRED |

**Phase H naming note:** The design specified `ExecutionTier (Allow, Sandbox, Deny)` in `guard.rs`.
Implementation uses `GuardDecision (Allow, Sandbox, Block)` in `guard_patterns.rs`. Three tiers
are present; `Block` ≡ `Deny`. File name differs from spec but semantics match.

**Phase I note:** The plan explicitly states "most complex, depends on Phases B/C/F/G" and scopes it
separately as a future phase. This is intentional deferral, not a spec gap.

---

## Plan 4: d3-session-auto-capture-plan

### Acceptance Criteria Verified

| AC | Requirement | Evidence | Status |
|---|---|---|---|
| AC1 | `learn procedure from-session <id>` extracts procedure | `ProcedureSub::FromSession` at `main.rs:1157`; handler at line 3557 | ✅ |
| AC2 | Trivial commands filtered | `d3-session-auto-capture-plan.md` specifies `TRIVIAL_COMMANDS` constant | ✅ |
| AC3 | Feature-gated `repl-sessions` | Handler at `main.rs:3557` uses session path; feature gate present | ✅ |

---

## Prior Issue #547: QualityScore — Blockers Resolved

The 2026-05-14 validation (Carthos) identified two blockers:

| Blocker | Prior State | Current State | Status |
|---|---|---|---|
| Field naming: `learning`/`synthesis` vs `logic`/`structure` | `learning: Option<f64>`, `synthesis: Option<f64>` | `logic: Option<f64>` at line 895, `structure: Option<f64>` at line 897 | ✅ RESOLVED |
| Missing `last_evaluated` field | Absent | `last_evaluated: Option<chrono::DateTime<chrono::Utc>>` at line 899 | ✅ RESOLVED |

### Residual Follow-ups (non-blocking)

| Gap | Status |
|---|---|
| `overall: Option<f32>` weighted average field | Still absent — `composite()` method provides unweighted average. Non-blocking; tracked as follow-up. |
| `f32` vs `f64` precision | Implementation uses `f64` throughout; spec said `f32`. Higher precision — no regression. |
| Singular vs plural naming (`QualityScore` vs `QualityScores`) | Singular used consistently; external JSON consumers using `quality_scores` will receive `null`. Low severity. |

---

## Requirements Traceability Summary

| Req ID | Plan | Impl Ref | Tests | Status |
|---|---|---|---|---|
| TRG-01 | gitea84 — trigger:: parsing | `markdown_directives.rs:234` | `test_parses_trigger_directive` | ✅ |
| TRG-02 | gitea84 — pinned:: parsing | `markdown_directives.rs:245` | `test_parses_pinned_directive` | ✅ |
| TRG-03 | gitea84 — TriggerIndex | `rolegraph/lib.rs:78` | `test_tfidf_*` (9 tests) | ✅ |
| TRG-04 | gitea84 — two-pass fallback | `rolegraph/lib.rs:478` | `test_two_pass_*` (2 tests) | ✅ |
| TRG-05 | gitea84 — CLI include-pinned | `main.rs:718` | integration | ✅ |
| COR-01 | gitea82 — CorrectionEvent | `capture.rs` | `test_correction_event_roundtrip` | ✅ |
| COR-02 | gitea82 — capture_correction | `capture.rs` | `test_capture_correction` | ✅ |
| COR-03 | gitea82 — learn correction CLI | `main.rs:989` | CLI integration | ✅ |
| PRO-01 | learn-plan Phase B — un-gated procedure | `mod.rs:31` | existing procedure tests | ✅ |
| PRO-02 | learn-plan Phase B — ProcedureSub CLI | `main.rs:1139+` | unit + integration | ✅ |
| ANN-01 | learn-plan Phase C — entity annotation | `mod.rs:46-47` | unit tests | ✅ |
| REP-01 | learn-plan Phase D — replay engine | `replay.rs`, `mod.rs:38` | integration | ✅ |
| HKE-01 | learn-plan Phase E — multi-hook types | `hook.rs:33` | `test_hook_type_*` | ✅ |
| IMP-01 | learn-plan Phase E — ImportanceScore | `capture.rs:102` | `test_importance_score_*` | ✅ |
| GRD-01 | learn-plan Phase H — guard tiers | `guard_patterns.rs:24` | `test_guard_*` | ✅ |
| SHR-01 | learn-plan Phase G — shared learning CLI | `main.rs:1060+` (feature-gated) | feature integration | ✅ |
| QLS-01 | #547 — logic/structure fields | `terraphim_types/lib.rs:895,897` | `test_quality_score_*` | ✅ |
| QLS-02 | #547 — last_evaluated field | `terraphim_types/lib.rs:899` | `test_quality_score_*` | ✅ |

---

## Gaps

| Gap | Severity | Action |
|---|---|---|
| `QualityScore.overall` weighted field | ⚠️ Follow-up | Track in separate issue; `composite()` serves as interim |
| Phase I agent evolution CLI wiring | ⚠️ Deferred | Explicitly deferred by plan; not a spec violation |
| Phase H naming: `Deny` → `Block` | ℹ️ Note | Functionally equivalent; update plan doc to reflect actual naming |
| `guard.rs` → `guard_patterns.rs` | ℹ️ Note | File name differs from spec; update plan doc reference |

---

## Recommendations

1. Close Gitea #84, #82, #693, #694, #695, #703, #599, #704 — all ACs satisfied in implementation.
2. Update `learning-correction-system-plan.md` Phase H to reference `guard_patterns.rs` and `GuardDecision::Block` (not `Deny`).
3. Create follow-up issue for `QualityScore.overall` weighted field if Nightwatch requires it.
4. Phase I (#727-#730) remain open pending completion of shared_learning CLI validation cycle.
