# Spec Validation Report

**Agent:** Carthos (Domain Architect / spec-validator)
**Date:** 2026-05-05
**Report:** reports/spec-validation-20260505.md

---

## Summary

| Spec | Status | Notes |
|------|--------|-------|
| design-gitea82-correction-event.md | **PASS** | All types, CLI, handlers, and tests implemented and wired |
| d3-session-auto-capture-plan.md | **PASS** | from_session, extraction, filtering, CLI all implemented |
| learning-correction-system-plan.md | **PARTIAL** | Phases A-H, J implemented; Phase I (agent evolution) not wired |
| design-gitea84-trigger-based-retrieval.md | **FAIL** | Parsing and index implemented; production wiring incomplete |
| design-single-agent-listener.md | **OPERATIONAL** | Config exists; tmux session not currently running |

---

## Detailed Findings

### 1. design-gitea82-correction-event.md -- PASS

- `CorrectionType` enum with all 7 variants: `capture.rs:42`
- `CorrectionEvent` struct with full fields: `capture.rs:134`
- `to_markdown()` / `from_markdown()` serialization: `capture.rs:210-320`
- CLI `learn correction` subcommand: `main.rs:965`
- Handler wired in main dispatch: `main.rs:3480`

### 2. d3-session-auto-capture-plan.md -- PASS

- `from_session_commands()`: `procedure.rs:470`
- `extract_bash_commands_from_session()`: `procedure.rs:520`
- Trivial command filtering (cd, ls, echo, etc.): `procedure.rs:490`
- Auto-title generation: `procedure.rs:510`
- CLI `learn procedure from-session`: `main.rs:1179`
- Feature-gated behind `repl-sessions`: `main.rs:1179`

### 3. learning-correction-system-plan.md -- PARTIAL

**IMPLEMENTED (PASS):**
- Phase A (#480 redaction): `redaction.rs` wired in capture pipeline
- Phase B (#693 procedure memory): `procedure.rs` ungated, CLI commands: list, show, record, add-step, success, failure, replay, health, enable, disable, from-session
- Phase C (#703 entity annotation): `annotate_with_entities()` in `capture.rs`, `--semantic` flag on `learn query`
- Phase D (#694 replay): `replay.rs` with `StepOutcome`, `ReplayResult`, dry-run support
- Phase E (#599 multi-hook): `LearnHookType` enum: PreToolUse, PostToolUse, UserPromptSubmit
- Phase F (#695 health monitoring): `ProcedureHealthReport`, `health_check()`, auto-disable on critical
- Phase G (#727 shared learning): `SharedLearningSub` CLI: List, Promote, Import, Stats, Inject
- Phase H (#704 guard): `guard_patterns.rs` with GuardDecision: Allow, Block, Sandbox
- Phase J (#515-517 validation): `terraphim_hooks::ValidationService` exists with Aho-Corasick pattern matching

**NOT IMPLEMENTED (FAIL):**
- Phase I (#727-730 agent evolution): `terraphim_agent_evolution` crate exists but is **NOT a workspace member** and **NOT wired into `terraphim_agent` main binary**

### 4. design-gitea84-trigger-based-retrieval.md -- FAIL

**IMPLEMENTED:**
- `trigger::` and `pinned::` directive parsing: `markdown_directives.rs:180`
- `TriggerIndex` with TF-IDF: `terraphim_rolegraph/src/lib.rs:380`
- `find_matching_node_ids_with_fallback()`: `terraphim_rolegraph/src/lib.rs:450`
- `query_graph_with_trigger_fallback()`: `terraphim_rolegraph/src/lib.rs:470`
- `include_pinned` parameter on search query: `terraphim_config/src/lib.rs:1153`
- `graph --pinned` CLI flag: `main.rs:738`

**CRITICAL GAP:**
- `load_trigger_index()` is **never called in production code**. It is only invoked in unit tests (`terraphim_rolegraph/src/lib.rs:2205`).
- Result: The `trigger_index` field in `RoleGraph` remains empty in production. `query_graph_with_trigger_fallback()` falls back to an empty index, making the TF-IDF fallback path a no-op.
- Pinned entries DO work via `include_pinned` because `pinned_node_ids` is populated through the SerializableRoleGraph roundtrip, but trigger-based retrieval is dead code.

**MINOR GAP:**
- Spec requests `kg list --pinned` command. Implementation provides `graph --pinned` instead. Functionality exists but command naming diverges from spec.

### 5. design-single-agent-listener.md -- OPERATIONAL

- Config file `listener-worker.json` exists
- Launch script `start-listener.sh` exists
- Binary `~/.cargo/bin/terraphim-agent` exists
- **Gap:** tmux session `terraphim-worker` is not currently running

---

## Recommendations

1. **Wire `load_trigger_index()` in production**: After building the RoleGraph from KG markdown, extract `trigger::` and `pinned::` directives and call `load_trigger_index()`. This likely belongs in `terraphim_config/src/lib.rs` during role graph construction.

2. **Wire agent evolution crate**: Add `terraphim_agent_evolution` to workspace members and wire `EvolutionWorkflowManager` into the main binary behind a feature flag.

3. **Start listener tmux session**: Run `start-listener.sh` to activate the Gitea listener.

4. **Align CLI naming**: Consider adding `kg list --pinned` as an alias for `graph --pinned` to match spec.

---

Theme-ID: spec-gap
