# Spec Validation Report: 2026-04-28

**Agent:** Carthos (spec-validator)  
**Date:** 2026-04-28 05:33 CEST  
**Scope:** All active plans in `plans/` directory  
**Method:** Static code analysis against approved design documents  

---

## Plans Validated

| Plan | File | Status | Notes |
|------|------|--------|-------|
| Session Auto-Capture | `d3-session-auto-capture-plan.md` | **FAIL** | Data model mismatch |
| Single Agent Listener | `design-single-agent-listener.md` | **PASS** | Operational spec, no code changes |
| Listener Research | `research-single-agent-listener.md` | **PASS** | Research document, aligned |
| CorrectionEvent | `design-gitea82-correction-event.md` | **PASS** | Fully implemented |
| Trigger-Based Retrieval | `design-gitea84-trigger-based-retrieval.md` | **PARTIAL** | Backend complete, CLI surface missing |
| Learning System | `learning-correction-system-plan.md` | **PARTIAL** | Foundation complete, advanced phases unimplemented |

---

## Critical Gaps

### 1. Trigger-Based Retrieval -- CLI Surface Missing (Plan 3 / Gitea #84)

**Severity:** High  
**Files:** `crates/terraphim_agent/src/main.rs`  

**Specification requires:**
- `--include-pinned` flag on `search` subcommand
- `kg list --pinned` subcommand via `KgSub` enum

**Actual implementation:**
- `include_pinned` hardcoded to `false` at 4 call sites:
  - `main.rs:1793`
  - `main.rs:3727`
  - `main.rs:3739`
  - `main.rs:4673`
- No `KgSub` enum exists; no `kg` CLI subcommand group

**Backend status:** FULLY IMPLEMENTED and tested
- `TriggerIndex` with TF-IDF cosine similarity: `crates/terraphim_rolegraph/src/lib.rs:51`
- `find_matching_node_ids_with_fallback()` two-pass search: `lib.rs:451`
- `load_trigger_index()` builder: `lib.rs:478`
- `query_graph_with_trigger_fallback()` query method: `lib.rs:718`
- Markdown directive parsing for `trigger::` and `pinned::`: `crates/terraphim_automata/src/markdown_directives.rs:215-228`
- 14 unit/integration tests covering all trigger/TF-IDF scenarios

**Gap:** The backend is production-ready but inaccessible to users. The CLI surface must be wired to expose the implemented functionality.

---

### 2. Session Data Model Mismatch (Plan 1 / D3)

**Severity:** Medium  
**Files:** `crates/terraphim_sessions/src/model.rs`, `crates/terraphim_agent/src/learnings/procedure.rs`  

**Specification requires:**
```json
{
  "tool_uses": [
    { "tool": "Bash", "command": "cargo build", "exit_code": 0 }
  ]
}
```

**Actual implementation:**
```rust
// crates/terraphim_sessions/src/model.rs:75
is_error: bool,
```

**Impact:** The spec defines `exit_code: int` for rich failure analysis (distinguishing exit codes 1, 2, 127, etc.). The boolean `is_error` loses this granularity. Adapter code in `procedure.rs` translates `is_error` to `0`/`1`, but the raw model violates the approved design.

---

### 3. Query Function Interface Deviation (Plan 2 / Gitea #82)

**Severity:** Low  
**Files:** `crates/terraphim_agent/src/main.rs:2819`  

**Specification requires:**
- `query_all_entries()` function for unified learning + correction search

**Actual implementation:**
- `query_all_entries_semantic()` is used instead
- Function signature includes additional `semantic` parameter

**Impact:** No functional loss. The semantic variant provides additional search capability. However, the interface diverges from the approved design document, creating documentation drift.

---

## Partial Implementation Gaps (Learning System)

### 4. Success Capture in Hook Pipeline (Phase B)

**Severity:** Medium  
**Files:** `crates/terraphim_agent/src/learnings/hook.rs`  

**Specification requires:**
- `should_capture_success()` method
- `capture_success_from_hook()` function
- `SessionCommandBuffer` for grouping successful commands into procedures

**Actual implementation:**
- Only `test_should_not_capture_success()` exists (line 435)
- Hook only captures Bash failures (exit_code != 0)
- No session tracking or command grouping for successful sequences

**Impact:** Issue #693 (auto-capture from sessions) cannot be fully realised without this component.

---

### 5. Agent Evolution Integration (Phase I)

**Severity:** Low  
**Files:** `crates/terraphim_agent_evolution/`, `crates/terraphim_agent/src/main.rs`  

**Specification requires:**
- Wire `EvolutionWorkflowManager` to real LLM adapters
- Connect learning capture events to evolution system
- CLI subcommands for evolution management

**Actual implementation:**
- `terraphim_agent_evolution` crate exists with mock LLM adapters
- Zero references in `main.rs` -- no CLI integration
- Not wired to the ADF orchestrator

**Impact:** Issues #727-#730 blocked. The crate is complete but orphaned.

---

## Passing Plans (Verified)

### CorrectionEvent (Gitea #82) -- PASS
- `CorrectionType` enum with Display + FromStr: `capture.rs:44`
- `CorrectionEvent` struct with full markdown roundtrip: `capture.rs:502`
- `capture_correction()` with secret redaction: `capture.rs:582`
- `LearningEntry` unified enum: `capture.rs:728`
- `list_all_entries()` and `query_all_entries_semantic()`: `capture.rs:773`
- CLI `learn correction` subcommand: `main.rs:942`
- 8 unit tests + integration test coverage

### Shared Learning CLI (Phase G) -- PASS
- `SharedLearningSub` enum with List, Promote, Import, Stats, Inject: `main.rs:1013`
- `run_shared_learning_command()` fully implemented: `main.rs:3485`
- Trust level management (L1/L2/L3)
- BM25 deduplication during import
- Learning injection into agent context

### Procedure Management (Phase B) -- PASS
- `ProcedureSub` enum with 8 subcommands: `main.rs:1092`
- `ProcedureStore` ungated and publicly exported: `mod.rs:31`
- `from_session_commands()` with trivial command filtering: `procedure.rs:412`
- `extract_bash_commands_from_session()` adapter: `procedure.rs:471`
- Replay engine with dry-run support: `main.rs:3003`
- Health monitoring with auto-disable: `main.rs:3083`

### Redaction Pipeline (Phase A) -- PASS
- `redact_secrets()` applied in hook stdout passthrough: `hook.rs:125-127`
- `contains_secrets()` pre-check implemented: `hook.rs:26`
- 12 secret patterns + env var patterns: `redaction.rs`

---

## Traceability Matrix

| Requirement | Design Ref | Implementation | Tests | Status |
|-------------|-----------|----------------|-------|--------|
| CorrectionType enum | `design-gitea82:1.1` | `capture.rs:44` | `capture.rs` tests | PASS |
| CorrectionEvent struct | `design-gitea82:1.2` | `capture.rs:502` | Roundtrip test | PASS |
| capture_correction() | `design-gitea82:1.4` | `capture.rs:582` | Unit + redaction test | PASS |
| CLI learn correction | `design-gitea82:3.1` | `main.rs:942` | Integration test | PASS |
| Trigger parsing | `design-gitea84:2` | `markdown_directives.rs:215` | 5 directive tests | PASS |
| TriggerIndex TF-IDF | `design-gitea84:3` | `rolegraph.rs:51` | 8 index tests | PASS |
| Two-pass search | `design-gitea84:5` | `rolegraph.rs:451` | Integration tests | PASS |
| CLI --include-pinned | `design-gitea84:7` | **MISSING** | N/A | **FAIL** |
| CLI kg list --pinned | `design-gitea84:7` | **MISSING** | N/A | **FAIL** |
| Session exit_code field | `d3:Session data model` | `model.rs:75` (is_error) | N/A | **FAIL** |
| Success capture hook | `learning-plan:Phase B.2` | **MISSING** | N/A | **FAIL** |
| Agent evolution CLI | `learning-plan:Phase I` | **MISSING** | N/A | **FAIL** |

---

## Recommendations

### Immediate Actions (High Priority)
1. **Wire `--include-pinned` flag**: Add the flag to `SearchSub` and propagate to `query_graph_with_trigger_fallback()` calls
2. **Add `kg list --pinned` subcommand**: Create `KgSub` enum with List variant and `--pinned` flag
3. **Migrate session model to exit_code**: Replace `is_error: bool` with `exit_code: i32` in `terraphim_sessions/src/model.rs`

### Deferred Actions (Medium Priority)
4. **Implement success capture in hook**: Add `SessionCommandBuffer` and `capture_success_from_hook()` for issue #693
5. **Wire agent evolution CLI**: Add evolution subcommands to main.rs for issues #727-#730

### Documentation
6. **Update query function docs**: Document that `query_all_entries_semantic()` supersedes the spec'd `query_all_entries()`

---

## Verdict

**FAIL** -- Three critical gaps and two partial gaps identified. The backend for trigger-based retrieval is production-ready but unreachable. The session data model loses information specified in the design. Foundation phases of the learning system are complete, but downstream phases remain unimplemented.

**Next Review:** After CLI surface for trigger-based retrieval is wired.

---
*Report generated by Carthos, Domain Architect (spec-validator)*  
*Terraphim AI -- Spec Validation Protocol*
