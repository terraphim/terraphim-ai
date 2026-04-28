# Spec Validation Report: Plans Directory Cross-Reference

**Date:** 2026-04-28
**Agent:** Carthos (spec-validator)
**Scope:** All active spec documents in `plans/` directory

## Verdict: FAIL

---

## 1. Specifications Validated

| Plan Document | Status | Gaps Found |
|---|---|---|
| `design-gitea82-correction-event.md` | **MOSTLY IMPLEMENTED** | 1 minor gap |
| `design-gitea84-trigger-based-retrieval.md` | **PARTIALLY IMPLEMENTED** | 2 gaps |
| `d3-session-auto-capture-plan.md` | **IMPLEMENTED** | None |
| `design-single-agent-listener.md` | **NOT VERIFIABLE** | Operational task, no code changes |
| `learning-correction-system-plan.md` | **MOSTLY ACCURATE** | 1 gap, 2 outdated claims |
| `research-single-agent-listener.md` | **NOT VERIFIABLE** | Research document, no implementation claims |

---

## 2. Detailed Findings

### 2.1 design-gitea82-correction-event.md

**Status:** MOSTLY IMPLEMENTED

**Verified Implementation:**
- `CorrectionType` enum with all 7 variants (lines 44-89, `capture.rs`)
- `CorrectionEvent` struct with full YAML frontmatter roundtrip (lines 499-646, `capture.rs`)
- `LearningEntry` enum unifying Learning, Correction, and Procedure (lines 1225-1295, `capture.rs`)
- `capture_correction()` function with secret redaction (lines 1022-1067, `capture.rs`)
- `list_all_entries()` function (lines 1298-1368, `capture.rs`)
- `query_all_entries()` function (lines 1372-1408, `capture.rs`)
- CLI `LearnSub::Correction` variant (lines 955-972, `main.rs`)
- 8+ unit tests for correction event roundtrip, capture, secret redaction, listing, querying

**Gap:**
- **GAP-1:** CLI uses `query_all_entries_semantic()` instead of `query_all_entries()` specified in design document. The semantic variant adds KG-based entity matching as a fallback, which is an enhancement over the spec but changes the implementation contract.

---

### 2.2 design-gitea84-trigger-based-retrieval.md

**Status:** PARTIALLY IMPLEMENTED

**Verified Implementation:**
- `trigger` and `pinned` fields added to `MarkdownDirectives` (lines 319-322, `terraphim_types/src/lib.rs`)
- Parsing of `trigger::` and `pinned::` directives in `markdown_directives.rs` (lines 215-228) with 5 unit tests
- `TriggerIndex` struct with TF-IDF cosine similarity (lines 51-270, `terraphim_rolegraph/src/lib.rs`)
- `find_matching_node_ids_with_fallback()` two-pass search (lines 451-475)
- `load_trigger_index()` method (lines 478-490)
- 8+ unit tests for TriggerIndex exact match, partial match, threshold filtering, and integration with RoleGraph

**Gaps:**
- **GAP-2:** No CLI flag `--include-pinned` exposed to users. The `include_pinned` parameter exists internally in search functions but is hardcoded to `false` in all 4 call sites in `main.rs` (lines 2029, 3966, 3978, 4915). Users cannot enable pinned entry inclusion.
- **GAP-3:** No `kg list --pinned` command. The spec calls for a `KgSub` enum with a `List { pinned: bool }` variant, but no such subcommand exists. The CLI has `Graph`, `Extract`, `Replace`, and `Validate` commands but no `kg` namespace.

---

### 2.3 d3-session-auto-capture-plan.md

**Status:** IMPLEMENTED

**Verified Implementation:**
- `from_session_commands()` function (line 412, `procedure.rs`)
- `extract_bash_commands_from_session()` function (line 471, `procedure.rs`)
- CLI `ProcedureSub::FromSession` variant (lines 1171-1177, `main.rs`)
- Trivial command filtering (cd, ls, echo, etc.)
- Auto-title generation from first/last commands
- 7+ unit tests covering basic extraction, trivial filtering, failure filtering, auto-title generation, empty input, and all-trivial input

**No gaps found.**

---

### 2.4 design-single-agent-listener.md

**Status:** NOT VERIFIABLE BY CODE REVIEW

**Notes:**
This is an operational setup document. It explicitly states "No code changes to the Rust codebase are required." The plan describes building, configuring, and launching an existing listener infrastructure. Verification would require runtime testing of the tmux session and Gitea API interactions, which is outside the scope of static code analysis.

---

### 2.5 learning-correction-system-plan.md

**Status:** MOSTLY ACCURATE (with outdated claims)

**Verified Claims:**
- `CapturedLearning` struct with markdown serialization: CONFIRMED
- `CorrectionEvent` with typed corrections: CONFIRMED
- `LearningEntry` enum: CONFIRMED (with additional Procedure variant)
- `ScoredEntry` and `TranscriptEntry`: CONFIRMED as dead code (only used internally in `capture.rs`, not exported or wired to CLI)
- Secret redaction in `redaction.rs`: CONFIRMED as fully wired
- `ProcedureStore` with JSONL storage: CONFIRMED
- Hook pipeline capturing failed Bash commands: CONFIRMED
- `terraphim_agent_evolution` as standalone crate: CONFIRMED

**Inaccurate Claims:**
- **OUTDATED-1:** Plan states `#[cfg(test)] mod procedure;` at line 29-30 of `mod.rs`. Current code shows `pub(crate) mod procedure;` without `cfg(test)` gating. This was fixed since the plan was written.
- **OUTDATED-2:** Plan states `shared_learning/` is "NOT referenced in main.rs at all". Current code shows it IS referenced behind `#[cfg(feature = "shared-learning")]` with full CLI subcommands (`SharedLearningSub` enum, `run_shared_learning_command()` function). The feature gating may not have existed when the plan was written.

**Gaps:**
- **GAP-4:** Auto-suggest from KG is NOT implemented. The `suggest.rs` file (187 lines) only contains `SuggestionMetrics` for tracking approval/rejection rates. There is no actual suggestion generation logic, no KG querying for corrections, and no integration with the capture pipeline. The module docstring mentions "auto-suggesting corrections from the knowledge graph" but the implementation is missing.

---

## 3. Gap Summary

| Gap ID | Spec | Severity | Description |
|---|---|---|---|
| GAP-1 | design-gitea82 | Follow-up | CLI uses `query_all_entries_semantic` instead of spec's `query_all_entries` |
| GAP-2 | design-gitea84 | **Blocker** | `--include-pinned` flag not exposed in CLI (hardcoded false) |
| GAP-3 | design-gitea84 | **Blocker** | `kg list --pinned` command does not exist |
| GAP-4 | learning-correction | Follow-up | Auto-suggest from KG not implemented (only metrics tracking exists) |

---

## 4. Recommendations

1. **Expose `--include-pinned` flag** in `Command::Search` and/or `LearnSub::Query` to allow users to include pinned KG entries. This requires adding the CLI argument and threading it through to the search query construction.

2. **Add `kg list --pinned` command** or equivalent. The spec calls for a dedicated KG subcommand namespace. Consider whether this should be under `Graph` (e.g., `graph --pinned`) or a new `Kg` subcommand as originally designed.

3. **Implement auto-suggest from KG** or update the `suggest.rs` module docstring to reflect the actual scope. The current module only tracks metrics but claims to auto-suggest corrections.

4. **Update `learning-correction-system-plan.md`** to reflect current state: `procedure.rs` is no longer test-only, and `shared_learning` is feature-gated but present in the CLI.

5. **Document semantic query enhancement** in `design-gitea82-correction-event.md` if `query_all_entries_semantic` is the intended final implementation. The semantic variant adds value but diverges from the approved design.

---

**Signed:** Carthos, Domain Architect
**Symbol:** Compass rose (orientation in complexity)
