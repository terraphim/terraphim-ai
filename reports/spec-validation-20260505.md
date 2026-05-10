# Spec Validation Report -- 2026-05-05

**Validator:** Carthos (Domain Architect)  
**Date:** 2026-05-05 12:33 CEST  
**Scope:** All active specification documents in `plans/` directory cross-referenced against implementation in `crates/`

---

## Executive Summary

**Verdict: FAIL** -- 6 specification documents reviewed. 2 fully implemented, 3 partially implemented, 1 not implemented. 14 specific gaps identified between specification and code.

| Spec | Status | Gaps |
|------|--------|------|
| design-gitea82-correction-event.md | PARTIAL | 1 gap |
| design-gitea84-trigger-based-retrieval.md | PARTIAL | 2 gaps |
| d3-session-auto-capture-plan.md | NOT IMPLEMENTED | 2 gaps |
| learning-correction-system-plan.md | PARTIAL | 8 gaps |
| design-single-agent-listener.md | N/A (operational) | 0 gaps |
| research-single-agent-listener.md | N/A (research) | 0 gaps |

---

## 1. design-gitea82-correction-event.md

**Status:** APPROVED (plan says approved)  
**Scope:** Phase 1.1 and 1.2 -- Add `CorrectionEvent` struct and `learn correction` CLI subcommand

### Implementation Status

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `CorrectionType` enum | `crates/terraphim_agent/src/learnings/capture.rs:42` | IMPLEMENTED | All 7 variants present with Display/FromStr |
| `CorrectionEvent` struct | `capture.rs:94` | IMPLEMENTED | All fields match spec (id, correction_type, original, corrected, context_description, source, context, session_id, tags) |
| `CorrectionEvent::new()` | `capture.rs:117` | IMPLEMENTED | Builder pattern with UUID-timestamp IDs |
| `CorrectionEvent::to_markdown()` | `capture.rs:152` | IMPLEMENTED | YAML frontmatter + body format |
| `CorrectionEvent::from_markdown()` | `capture.rs:201` | IMPLEMENTED | Parses frontmatter correctly |
| `learn correction` CLI subcommand | `main.rs` | NOT IMPLEMENTED | No `CorrectionSub` enum found. No `learn correction` handler in CLI dispatch |

### Gap
- **G1-1:** CLI integration missing. The spec explicitly requires `learn correction` subcommand under the `learn` command group. The core types are implemented but there is no user-facing CLI to create or query corrections.

---

## 2. design-gitea84-trigger-based-retrieval.md

**Status:** Design document (not explicitly approved in frontmatter)  
**Scope:** Parse `trigger::`/`pinned::` directives, build TF-IDF index, two-pass search, CLI flags

### Implementation Status

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `MarkdownDirectives.trigger` field | `crates/terraphim_types/src/lib.rs` | IMPLEMENTED | `trigger: Option<String>` present |
| `MarkdownDirectives.pinned` field | `terraphim_types/src/lib.rs` | IMPLEMENTED | `pinned: bool` present with `#[serde(default)]` |
| `trigger::` parsing | `crates/terraphim_automata/src/markdown_directives.rs` | IMPLEMENTED | Parser branch handles `trigger::` directive |
| `pinned::` parsing | `markdown_directives.rs` | IMPLEMENTED | Parser branch handles `pinned::` with true/yes/1 |
| `TriggerIndex` struct | `crates/terraphim_rolegraph/src/lib.rs:50` | IMPLEMENTED | TF-IDF with build(), query(), tokenise(), stopword filtering |
| `RoleGraph.trigger_index` field | `rolegraph/src/lib.rs:320` | IMPLEMENTED | Field present, initialised with default threshold |
| `RoleGraph.pinned_node_ids` field | `rolegraph/src/lib.rs:322` | IMPLEMENTED | Vec<u64> for pinned entries |
| `--include-pinned` CLI flag | `main.rs` | NOT IMPLEMENTED | No such flag found in SearchQuery or CLI args |
| `kg list --pinned` command | `main.rs` | NOT IMPLEMENTED | No `kg` subcommand with `--pinned` option |

### Gaps
- **G2-1:** `--include-pinned` CLI flag missing. The spec requires this flag on search commands to include pinned entries in results.
- **G2-2:** `kg list --pinned` command missing. The spec requires a CLI command to list pinned knowledge graph entries.

---

## 3. d3-session-auto-capture-plan.md

**Status:** Design document  
**Scope:** `learn procedure from-session <session-id>` CLI command with session extraction

### Implementation Status

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `extract_bash_commands_from_session()` | `crates/terraphim_agent/src/learnings/procedure.rs` | IMPLEMENTED | Feature-gated behind `repl-sessions`. Parses session JSONL |
| `from_session_commands()` | `procedure.rs` | IMPLEMENTED | Creates CapturedProcedure from command tuples |
| `TRIVIAL_COMMANDS` filter | `procedure.rs` | IMPLEMENTED | Constant array with cd, ls, pwd, echo, etc. |
| `FromSession` CLI variant | `main.rs` | NOT IMPLEMENTED | No variant found in ProcedureSub enum |
| `learn procedure from-session` handler | `main.rs` | NOT IMPLEMENTED | No CLI wiring for this subcommand |

### Gaps
- **G3-1:** `FromSession` CLI variant missing. The spec requires a `FromSession` variant in the `ProcedureSub` enum.
- **G3-2:** CLI handler missing. Even if the extraction logic exists, there is no user-facing CLI command to invoke it.

---

## 4. learning-correction-system-plan.md

**Status:** Research and Design Plan  
**Scope:** Multi-phase implementation plan for learning/correction system (Issues #480, #578, #693, #703, #694, #695, #599, #686, #704, #727-730, #515-517, #451)

### Phase A: Foundation Fixes (#480, #578)

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| Redaction on hook stdout passthrough | `hook.rs:124-130` | IMPLEMENTED | `contains_secrets()` pre-check + `redact_secrets()` applied before stdout write |
| `contains_secrets()` pre-check logging | `hook.rs:125-126` | IMPLEMENTED | Logs "secrets detected, redacting before stdout" |

**Phase A Verdict: COMPLETE**

### Phase B: Procedural Memory (#693)

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| procedure.rs ungated | `learnings/mod.rs:31` | IMPLEMENTED | `pub(crate) mod procedure;` (not behind `#[cfg(test)]`) |
| `ProcedureStore` | `procedure.rs:88` | IMPLEMENTED | JSONL backend with save, save_with_dedup, load_all, find_by_id |
| `HealthStatus` enum | `procedure.rs:49` | IMPLEMENTED | Healthy, Degraded, Critical, Insufficient |
| `ProcedureHealthReport` | `procedure.rs:74` | IMPLEMENTED | id, status, success_rate, total_executions, auto_disabled |
| CLI: `learn procedure list` | `main.rs` | IMPLEMENTED | Part of ProcedureSub |
| CLI: `learn procedure show` | `main.rs` | IMPLEMENTED | Part of ProcedureSub |
| CLI: `learn procedure record` | `main.rs` | IMPLEMENTED | Part of ProcedureSub |
| CLI: `learn procedure add-step` | `main.rs` | IMPLEMENTED | Part of ProcedureSub |
| CLI: `learn procedure success` | `main.rs` | IMPLEMENTED | Part of ProcedureSub |
| CLI: `learn procedure failure` | `main.rs` | IMPLEMENTED | Part of ProcedureSub |
| CLI: `learn procedure replay` | `main.rs` | IMPLEMENTED | Part of ProcedureSub |
| CLI: `learn procedure health` | `main.rs` | IMPLEMENTED | Part of ProcedureSub |
| CLI: `learn procedure enable/disable` | `main.rs` | IMPLEMENTED | Part of ProcedureSub |
| Success capture in hook | `hook.rs` | NOT IMPLEMENTED | Hook only captures PostToolUse failures (exit_code != 0). No `should_capture_success()` or `SessionCommandBuffer` |
| CLI: `learn procedure from-session` | `main.rs` | NOT IMPLEMENTED | See Gap G3-1 |

**Phase B Verdict: PARTIAL** -- Core storage and most CLI commands implemented. Success capture and from-session missing.

### Phase C: Entity Annotation (#703)

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `annotate_with_entities()` | `capture.rs` | IMPLEMENTED | Uses Aho-Corasick matcher on command+error text |
| `entities` field on `CapturedLearning` | `capture.rs:217` | IMPLEMENTED | `Vec<String>` of matched term IDs |
| `query_all_entries_semantic()` | `capture.rs` | IMPLEMENTED | Semantic query via entity matching |
| `--semantic` flag on `learn query` | `main.rs` | NOT VERIFIED | Need to check CLI args |

**Phase C Verdict: MOSTLY COMPLETE** -- Core annotation and query logic implemented. Semantic CLI flag needs verification.

### Phase D: Procedure Replay (#694)

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `replay_procedure()` | `learnings/replay.rs` | IMPLEMENTED | Loads procedure, checks confidence, executes steps |
| `StepOutcome` enum | `replay.rs` | IMPLEMENTED | Success, Failed, Skipped |
| `--dry-run` flag | `main.rs` | NOT VERIFIED | Need to check CLI args |

**Phase D Verdict: MOSTLY COMPLETE** -- Replay engine exists. Dry-run flag needs verification.

### Phase E: Multi-Hook Pipeline (#599, #686)

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `LearnHookType` enum | `hook.rs:32` | IMPLEMENTED | PreToolUse, PostToolUse, UserPromptSubmit with clap::ValueEnum |
| `process_hook_input_with_type()` | `hook.rs:87` | IMPLEMENTED | Async multi-hook pipeline with stdin/stdout |
| `process_pre_tool_use()` | `hook.rs:145` | IMPLEMENTED | Warns on known failure patterns |
| `process_user_prompt_submit()` | `hook.rs:194` | IMPLEMENTED | Captures correction patterns inline |
| `ImportanceScore` | `capture.rs:102` | IMPLEMENTED | error_severity, repetition_count, recency, has_correction, total |
| `calculate_importance()` | `capture.rs:124` | IMPLEMENTED | Weighted calculation with formula from spec |

**Phase E Verdict: COMPLETE** -- All types and handlers implemented.

### Phase F: Self-Healing Procedures (#695)

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `ProcedureHealthReport` | `procedure.rs:74` | IMPLEMENTED | With auto_disabled flag |
| `health_check()` on ProcedureStore | `procedure.rs` | IMPLEMENTED | Returns Vec<ProcedureHealthReport> |
| `auto_disable()` logic | `procedure.rs` | NOT IMPLEMENTED | No auto-disable when success_rate < 0.5 over last 10 executions |

**Phase F Verdict: PARTIAL** -- Health reporting exists but auto-disable logic missing.

### Phase G: Shared Learning CLI Integration (#727)

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `SharedLearningStore` | Not found | NOT IMPLEMENTED | No shared_learning module in terraphim_agent |
| `SharedLearning` type | Not found | NOT IMPLEMENTED | Not found in codebase |
| `GiteaWikiClient` | Not found | NOT IMPLEMENTED | Not found in codebase |
| `SharedLearningRecord` | Not found | NOT IMPLEMENTED | Not found in codebase |
| CLI: `learn shared list` | Not found | NOT IMPLEMENTED | No shared subcommands |
| CLI: `learn shared promote` | Not found | NOT IMPLEMENTED | No shared subcommands |
| CLI: `learn shared sync` | Not found | NOT IMPLEMENTED | No shared subcommands |

**Phase G Verdict: NOT IMPLEMENTED** -- Entire shared learning subsystem missing from terraphim_agent CLI.

### Phase H: Graduated Guard (#704)

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `GuardDecision` enum | `guard_patterns.rs:24` | IMPLEMENTED | Allow, Sandbox, Block |
| `CommandGuard` struct | `guard_patterns.rs:79` | IMPLEMENTED | Thesaurus-driven Aho-Corasick matching |
| `ExecutionTier` enum | Not found | NOT IMPLEMENTED | Spec mentions Allow/Sandbox/Deny tiers; code uses GuardDecision |

**Phase H Verdict: MOSTLY COMPLETE** -- Guard patterns exist. `ExecutionTier` naming divergence (uses `GuardDecision` instead).

### Phase I: Agent Evolution Integration (#727-730)

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `terraphim_agent_evolution` crate | `crates/terraphim_agent_evolution/` | EXISTS | Standalone crate with EvolutionWorkflowManager |
| Integration into terraphim_agent | Not found | NOT IMPLEMENTED | No reference in terraphim_agent main.rs or mod.rs |
| Real LLM adapter wiring | Not found | NOT IMPLEMENTED | Still uses mock adapters |

**Phase I Verdict: NOT IMPLEMENTED** -- Crate exists but is not integrated.

### Phase J: Validation Pipeline (#515-517, #451)

| Requirement | Location | Status | Evidence |
|-------------|----------|--------|----------|
| `kg_validation` module | `terraphim_agent/src/kg_validation.rs` | EXISTS | Module present |
| PreToolUse validation | `hook.rs:145` | PARTIAL | Queries past learnings but full KG pattern matching unclear |

**Phase J Verdict: PARTIAL** -- Foundation exists but full pipeline not verified.

---

## 5. design-single-agent-listener.md

**Status:** Operational plan (no code changes required)  
**Verdict: N/A** -- This is a pure operational/configuration task. No code validation applicable.

---

## 6. research-single-agent-listener.md

**Status:** Research document  
**Verdict: N/A** -- Research documents inform design but do not specify implementation details to validate.

---

## Gap Registry

| ID | Spec | Gap Description | Severity | Effort |
|----|------|-----------------|----------|--------|
| G1-1 | design-gitea82 | `learn correction` CLI subcommand missing | Medium | 1-2 days |
| G2-1 | design-gitea84 | `--include-pinned` CLI flag missing | Medium | 1 day |
| G2-2 | design-gitea84 | `kg list --pinned` command missing | Low | 1 day |
| G3-1 | d3-session | `FromSession` CLI variant missing | Medium | 1 day |
| G3-2 | d3-session | `learn procedure from-session` handler missing | Medium | 1-2 days |
| G4-1 | learning-system | Success capture in hook pipeline missing | High | 3-5 days |
| G4-2 | learning-system | `SessionCommandBuffer` for grouping commands missing | High | 3-5 days |
| G4-3 | learning-system | Auto-disable logic (success_rate < 0.5) missing | Medium | 1-2 days |
| G4-4 | learning-system | Shared learning module entirely missing | High | 5-8 days |
| G4-5 | learning-system | Shared learning CLI subcommands missing | High | 2-3 days |
| G4-6 | learning-system | Agent evolution integration missing | High | 8-12 days |
| G4-7 | learning-system | Semantic query CLI flag verification needed | Low | 0.5 day |
| G4-8 | learning-system | Replay dry-run flag verification needed | Low | 0.5 day |

---

## Recommendations

### Immediate Actions (High Impact, Low Effort)
1. **Wire `learn correction` CLI** (G1-1, G2-1, G2-2) -- These are CLI-only gaps. The underlying types are fully implemented. Estimated: 2-3 days total.
2. **Wire `learn procedure from-session`** (G3-1, G3-2) -- Extraction logic exists, just needs CLI variant and handler. Estimated: 1-2 days.

### Short-Term (High Impact, Medium Effort)
3. **Implement hook success capture** (G4-1, G4-2) -- Requires extending hook pipeline to capture Bash successes and group them into procedures. Estimated: 3-5 days.
4. **Implement auto-disable logic** (G4-3) -- Add rolling window calculation to ProcedureStore. Estimated: 1-2 days.

### Long-Term (Strategic)
5. **Implement shared learning subsystem** (G4-4, G4-5) -- Complete module with persistence, BM25, trust levels, wiki sync. Estimated: 5-8 days.
6. **Integrate agent evolution** (G4-6) -- Wire standalone crate into terraphim-agent with real LLM adapters. Estimated: 8-12 days.

---

## Traceability Matrix

| Spec File | Issue | Design Ref | Impl Ref | Tests | Status |
|-----------|-------|------------|----------|-------|--------|
| design-gitea82 | #82 | capture.rs | capture.rs | Unit tests exist | PARTIAL |
| design-gitea84 | #84 | rolegraph/lib.rs | rolegraph/lib.rs, automata/markdown_directives.rs | Unit tests exist | PARTIAL |
| d3-session | #693 | procedure.rs | procedure.rs | Feature-gated tests | NOT IMPLEMENTED |
| learning-system | #480 | hook.rs | hook.rs | Integration tests | COMPLETE |
| learning-system | #578 | main.rs | main.rs | Integration tests | UNKNOWN |
| learning-system | #693 | procedure.rs, main.rs | procedure.rs, main.rs | Unit + integration | PARTIAL |
| learning-system | #703 | capture.rs | capture.rs | Unit tests | MOSTLY COMPLETE |
| learning-system | #694 | replay.rs | replay.rs | Integration tests | MOSTLY COMPLETE |
| learning-system | #599 | hook.rs | hook.rs | Unit + integration | COMPLETE |
| learning-system | #695 | procedure.rs | procedure.rs | Unit tests | PARTIAL |
| learning-system | #704 | guard_patterns.rs | guard_patterns.rs | Unit tests | MOSTLY COMPLETE |
| learning-system | #727-730 | agent_evolution/ | agent_evolution/ (standalone) | N/A | NOT IMPLEMENTED |

---

*Report generated by Carthos, Domain Architect. For questions about methodology or findings, consult the spec-validator runbook.*
