# Spec Validation Report: 2026-04-28

**Agent:** Carthos (spec-validator)  
**Date:** 2026-04-28  
**Scope:** All active plans in `/home/alex/terraphim-ai/plans/`  
**Verdict:** FAIL (spec violations found)

---

## Summary

Cross-referenced 6 specification documents against the current Rust codebase. Four plans are fully or mostly implemented. **Two significant gaps remain** in Plan 3 (Trigger-Based Retrieval), and one minor deviation exists in Plan 2. Plan 5's future-phase items (success capture, replay safety, evolution integration) remain unimplemented.

---

## Plan 1: D3 Session Auto-Capture (`d3-session-auto-capture-plan.md`)

### Status: PARTIAL

| Requirement | Status | Evidence |
|-------------|--------|----------|
| `learn procedure from-session` CLI | FOUND | `main.rs:1157-1163`, `main.rs:3113-3160` |
| `TRIVIAL_COMMANDS` filter | FOUND | `procedure.rs:382` |
| `ProcedureStore::save_with_dedup()` | FOUND | `procedure.rs:142` |
| Session JSON schema with `exit_code` | **MISSING** | `terraphim_sessions` uses `is_error: bool` (model.rs:72-76) rather than `exit_code: int`. Adapter logic translates `is_error` to `0`/`1` (procedure.rs:486), but raw data model does not match spec. |

### Gap
The plan specifies session JSON with `tool_uses[].exit_code`. The actual `terraphim_sessions` crate stores `ToolResult { is_error: bool, ... }`. Functional equivalence is achieved via adapter code, but the data model violates the specification.

---

## Plan 2: CorrectionEvent (`design-gitea82-correction-event.md`)

### Status: MOSTLY IMPLEMENTED (minor deviation)

| Requirement | Status | Evidence |
|-------------|--------|----------|
| `CorrectionType` enum | FOUND | `capture.rs:44` |
| `CorrectionEvent` struct | FOUND | `capture.rs:502` |
| `LearningEntry` enum | FOUND | `capture.rs:1225` (extended with `Procedure` variant) |
| `capture_correction()` | FOUND | `capture.rs:1022` |
| `list_all_entries()` / `query_all_entries()` | FOUND | `capture.rs:1298`, `capture.rs:1372` |
| CLI `learn correction` | FOUND | `main.rs:2837` |
| CLI `learn list` | FOUND | `main.rs:2764` uses `list_all_entries` |
| CLI `learn query` | **DEVIATION** | `main.rs:2798` calls `query_all_entries_semantic()` instead of the plain `query_all_entries()` specified in the design. No functional loss, but interface diverges. |

### Gap
The `learn query` subcommand uses semantic/entity-aware search rather than the simple text search defined in the spec. The spec function exists but is unwired.

---

## Plan 3: Trigger-Based Retrieval (`design-gitea84-trigger-based-retrieval.md`)

### Status: **FAIL** (backend implemented, CLI surface missing)

| Requirement | Status | Evidence |
|-------------|--------|----------|
| `MarkdownDirectives.trigger` / `.pinned` | FOUND | `terraphim_types/src/lib.rs:498-500` |
| Parse `trigger::` / `pinned::` | FOUND | `terraphim_automata/src/markdown_directives.rs:215-229` |
| `TriggerIndex` (TF-IDF) | FOUND | `terraphim_rolegraph/src/lib.rs:51` |
| `RoleGraph` integration | FOUND | `lib.rs:320`, `lib.rs:322` |
| `SerializableRoleGraph` fields | FOUND | `lib.rs:270`, `lib.rs:273` |
| `find_matching_node_ids_with_fallback()` | FOUND | `lib.rs:451` |
| `load_trigger_index()` | FOUND | `lib.rs:478` |
| `query_graph_with_trigger_fallback()` | FOUND | `lib.rs:718` |
| CLI `--include-pinned` flag | **MISSING** | Parameter exists internally (`terraphim_types` search structs) but is **hardcoded to `false`** in `main.rs:1774`, `main.rs:3705`, `main.rs:3717`, `main.rs:4649`. Not exposed to users. |
| CLI `kg list --pinned` | **MISSING** | No `KgSub` enum or `kg` subcommand exists in `terraphim_agent/src/main.rs`. |
| Directive parsing tests | FOUND | `markdown_directives.rs:351-411` |
| TriggerIndex tests | FOUND | `tests/trigger_index_tests.rs`, `lib.rs:2117-2261` |

### Gaps
1. **Critical:** The `--include-pinned` CLI flag is absent. End-users cannot trigger pinned-entry inclusion.
2. **Critical:** The `kg list --pinned` subcommand is absent. No KG management CLI exists.
3. All backend logic (TF-IDF, two-pass search, serialisation) is complete and tested, but the CLI boundary is incomplete.

---

## Plan 4: Single Agent Listener (`design-single-agent-listener.md`)

### Status: PASS

All operational artefacts present:
- `terraphim-agent listen` CLI: `main.rs:883-885`
- `ListenerRuntime` with dedup: `listener.rs:1268-1277`
- `GiteaTracker`: `terraphim_tracker/src/gitea.rs`
- Config file: `~/.config/terraphim/listener-worker.json`
- Launch script: `~/.config/terraphim/scripts/start-listener.sh`
- 12+ unit/integration tests in `listener.rs`

No gaps. This is a pure operational plan and all infrastructure exists.

---

## Plan 5: Learning & Correction System (`learning-correction-system-plan.md`)

### Status: PARTIAL (Phase A-D assessment outdated; future phases not done)

**Items now fixed (outdated claims in plan):**
- `procedure.rs` no longer gated behind `#[cfg(test)]` -- `mod.rs:31`
- `shared_learning` IS wired to CLI -- `main.rs:1003-1045`, `main.rs:3464-3634`
- `ScoredEntry` and `TranscriptEntry` are actively used -- `capture.rs:1515`, `capture.rs:1660`
- `annotate_with_entities` implemented -- `capture.rs:833`

**Items still missing (future phases):**
- Phase B: Success-capture hook pipeline -- hook.rs still only captures failed Bash
- Phase D: Full replay engine safety/confirmation wiring -- `replay.rs` exists but safety flow not integrated into main binary
- `terraphim_agent_evolution` crate -- standalone, not integrated into `terraphim_agent`

---

## Plan 6: Single Agent Listener Research (`research-single-agent-listener.md`)

### Status: PASS

All research assumptions confirmed by codebase. No gaps.

---

## Aggregate Verdict

**FAIL** due to:
1. Plan 3: Missing CLI surface (`--include-pinned`, `kg list --pinned`)
2. Plan 1: Session data model mismatch (`is_error` vs `exit_code`)
3. Plan 2: `learn query` uses unwired `query_all_entries_semantic` instead of spec'd `query_all_entries`

---

## Recommendations

### Priority 1 (Plan 3)
- Add `--include-pinned` flag to `search` subcommand in `main.rs`
- Add `kg list --pinned` subcommand and `KgSub` enum in `main.rs`
- Wire flags through to `query_graph_with_trigger_fallback()`

### Priority 2 (Plan 1)
- Align `terraphim_sessions` model with spec OR update spec to reflect `is_error: bool`
- If updating spec, document the adapter logic in `procedure.rs:486`

### Priority 3 (Plan 2)
- Either wire `query_all_entries` to CLI or update spec to reflect semantic search path

### Priority 4 (Plan 5)
- Implement success-capture hook pipeline (Phase B) when prioritized
- Integrate `terraphim_agent_evolution` when cross-run learning is required
