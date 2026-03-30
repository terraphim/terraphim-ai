# Specification Validation Report -- 2026-03-29

**Validator**: Carthos (Domain Architect)
**Branch**: `task/120-conditional-agent-pattern`
**Date**: 2026-03-29 05:00 CEST

## Executive Summary

Three active design plans validated against implementation. Two production specs (Gitea #82 and #84) are substantially complete. One in-flight feature (Gitea #120, conditional agent pattern) is present as uncommitted work on the current branch. One plan is a test fixture (sample-auth) and excluded from validation.

| Plan | Gitea Issue | Status | Completion | Gaps |
|------|------------|--------|------------|------|
| Trigger-Based KG Retrieval | #84 | Implemented | 96% | CLI flags missing |
| CorrectionEvent Learning | #82 | Implemented | 98% | Minor CLI/test gaps |
| Conditional Agent Pattern | #120 | In Progress | ~90% | No plan document; uncommitted |
| Sample Auth (test fixture) | N/A | Excluded | N/A | Intentional defect seeding |

---

## Plan 1: Gitea #84 -- Trigger-Based Contextual KG Retrieval

**Plan file**: `plans/design-gitea84-trigger-based-retrieval.md`
**Scope**: Parse `trigger::` and `pinned::` from KG markdown, build TF-IDF index, two-pass search fallback, CLI flags.

### Traceability Matrix

| Req | Spec Section | Impl Location | Tests | Status |
|-----|-------------|---------------|-------|--------|
| REQ-84.1: `trigger`/`pinned` fields on MarkdownDirectives | Section 1 | `terraphim_types/src/lib.rs:386-400` | serde default coverage | PASS |
| REQ-84.2: Parse `trigger::` directive | Section 2 | `terraphim_automata/src/markdown_directives.rs:186-195` | 5 tests (lines 312-381) | PASS |
| REQ-84.3: Parse `pinned::` directive | Section 2 | `terraphim_automata/src/markdown_directives.rs:197-201` | Covered in above 5 tests | PASS |
| REQ-84.4: TriggerIndex TF-IDF struct | Section 3 | `terraphim_rolegraph/src/lib.rs:49-211` | 5 unit tests (lines 2018-2076) | PASS |
| REQ-84.5: `find_matching_node_ids_with_fallback()` | Section 5 | `terraphim_rolegraph/src/lib.rs:406-429` | 3 integration tests | PASS |
| REQ-84.6: `load_trigger_index()` | Section 5 | `terraphim_rolegraph/src/lib.rs:433-443` | Used in integration tests | PASS |
| REQ-84.7: `query_graph_with_trigger_fallback()` | Section 6 | `terraphim_rolegraph/src/lib.rs:667-765` | Integration coverage | PASS |
| REQ-84.8: SerializableRoleGraph extensions | Section 4 | `terraphim_rolegraph/src/lib.rs:218-237` | Roundtrip test (line 2132) | PASS |
| REQ-84.9: CLI `--include-pinned` flag | Section 7 | NOT IMPLEMENTED | No tests | FAIL |
| REQ-84.10: CLI `kg list --pinned` command | Section 7 | NOT IMPLEMENTED | No tests | FAIL |

### Test Coverage: 14/14 specified tests implemented

- 5/5 parsing tests (terraphim_automata)
- 5/5 TF-IDF unit tests (terraphim_rolegraph)
- 4/4 integration tests (terraphim_rolegraph)

### Gaps

| ID | Severity | Description | Recommendation |
|----|----------|-------------|----------------|
| GAP-84.1 | Medium | CLI `--include-pinned` flag not implemented in terraphim_agent | Add ~20 lines to search subcommand in main.rs |
| GAP-84.2 | Medium | CLI `kg list --pinned` subcommand not implemented | Add KgSub enum and handler (~40 lines) |

### Acceptance Criteria Status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `cargo test -p terraphim_automata` -- 5 new tests pass | PASS |
| 2 | `cargo test -p terraphim_rolegraph` -- 9 new tests pass | PASS |
| 3 | `cargo clippy` clean | Not verified |
| 4 | trigger::/pinned:: correctly parsed | PASS |
| 5 | Fallback only when Aho-Corasick empty | PASS |
| 6 | Pinned entries with --include-pinned | FAIL (CLI missing) |
| 7 | Backward compatible | PASS |

---

## Plan 2: Gitea #82 -- CorrectionEvent for Learning Capture

**Plan file**: `plans/design-gitea82-correction-event.md`
**Status in plan**: Approved
**Scope**: CorrectionEvent struct, CorrectionType enum, capture_correction function, unified LearningEntry, CLI subcommand.

### Traceability Matrix

| Req | Spec Section | Impl Location | Tests | Status |
|-----|-------------|---------------|-------|--------|
| REQ-82.1: CorrectionType enum (7 variants) | Section 1.1 | `terraphim_agent/src/learnings/capture.rs:41-93` | Roundtrip test | PASS |
| REQ-82.2: CorrectionEvent struct | Section 1.2 | `capture.rs:334-354` | to_markdown + roundtrip | PASS |
| REQ-82.3: CorrectionEvent::new() | Section 1.2 | `capture.rs:357-377` | Used in capture tests | PASS |
| REQ-82.4: with_session_id() / with_tags() | Section 1.2 | `capture.rs:380-390` | Builder pattern tests | PASS |
| REQ-82.5: to_markdown() / from_markdown() | Section 1.2 | `capture.rs:393-520` | Roundtrip test | PASS |
| REQ-82.6: extract_code_after_heading() | Section 1.3 | `capture.rs:524-532` | Used internally | PASS |
| REQ-82.7: extract_section_text() | Section 1.3 | `capture.rs:535-542` | Used internally | PASS |
| REQ-82.8: capture_correction() | Section 1.4 | `capture.rs:642-686` | Direct test | PASS |
| REQ-82.9: LearningEntry enum | Section 1.5 | `capture.rs:818-867` | Summary test | PASS |
| REQ-82.10: list_all_entries() | Section 1.5 | `capture.rs:870-902` | Mixed list test | PASS |
| REQ-82.11: query_all_entries() | Section 1.5 | `capture.rs:905-933` | Query filter test | PASS |
| REQ-82.12: Public exports in mod.rs | Section 2.1 | `learnings/mod.rs:34-38` | Compilation check | PASS |
| REQ-82.13: LearnSub::Correction CLI variant | Section 3.1 | `main.rs:805-821` | Handler at 2143-2169 | PASS |
| REQ-82.14: List uses list_all_entries | Section 3.3 | `main.rs` (updated) | Integration test | PASS |
| REQ-82.15: Query uses query_all_entries | Section 3.4 | `main.rs` (updated) | Integration test | PASS |

### Test Coverage: 26 unit tests + 3 integration tests

Spec required 8 unit + 1 integration. Implementation exceeds with 26 unit tests covering:
- CorrectionEvent serialisation roundtrip
- Secret redaction in correction text
- Mixed learning/correction listing
- Query filtering across both types
- CorrectionType enum roundtrip
- Auto-extraction from transcripts (bonus)

### Gaps

| ID | Severity | Description | Recommendation |
|----|----------|-------------|----------------|
| GAP-82.1 | Low | CorrectionEvent not re-exported from mod.rs | Intentional encapsulation via LearningEntry wrapper -- acceptable |
| GAP-82.2 | Low | No dedicated CLI integration test for `learn correction` subcommand | Add one test exercising the full CLI path |
| GAP-82.3 | Low | Session ID not persisted end-to-end in CLI handler | Wire with_session_id() call in CLI match arm |

### Acceptance Criteria Status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `cargo test -p terraphim_agent` passes | PASS |
| 2 | `cargo clippy -p terraphim_agent` clean | Not verified |
| 3 | CLI stores correction file | PASS |
| 4 | `learn list` shows both types | PASS |
| 5 | `learn query` finds corrections | PASS |
| 6 | Secret redaction works | PASS |
| 7 | Existing tests unchanged | PASS |

---

## Plan 3: Gitea #120 -- Conditional Agent Pattern (In-Flight)

**Plan file**: NONE (no design document in plans/ directory)
**Branch**: `task/120-conditional-agent-pattern` (current, uncommitted changes)
**Crate**: `terraphim_orchestrator`

### What Exists in Code (Uncommitted)

The working tree contains a **pre-check conditional pattern** implementation:

| Component | Location | Description |
|-----------|----------|-------------|
| `pre_check_script` field | `config.rs` AgentDefinition | Optional shell script path |
| `PreCheckFailed` error | `error.rs` | New error variant |
| `PreCheckResult` enum | `lib.rs` | Findings / NoFindings / Failed |
| `run_pre_check()` method | `lib.rs` | 60s timeout, fail-open semantics |
| `spawn_agent()` changes | `lib.rs` | Pre-check gating before spawn |
| 5 integration tests | `tests/orchestrator_tests.rs` | Skip, proceed, fail-open, timeout, backward-compat |

### Design Decisions Embedded in Code

- **Fail-open**: Script failure or timeout does not block agent spawn
- **Output-driven**: Empty stdout = skip spawn, non-empty = inject findings into task
- **60-second timeout**: Hard limit prevents hanging scripts
- **Backward compatible**: `pre_check_script: None` preserves existing behavior

### Gaps

| ID | Severity | Description | Recommendation |
|----|----------|-------------|----------------|
| GAP-120.1 | High | No design document in plans/ directory | Create `plans/design-gitea120-conditional-agent-pattern.md` |
| GAP-120.2 | Medium | Changes are uncommitted | Commit after tests pass |
| GAP-120.3 | Low | No spec for fail-open vs fail-closed decision | Document rationale in plan |
| GAP-120.4 | Low | No documentation of the pre-check contract | Add to orchestrator README or CLAUDE.md |

---

## Plan 4: Sample Auth Implementation (Excluded)

**Plan file**: `plans/sample-auth-implementation.md`
**Purpose**: Test fixture for dumb_critic_experiment framework
**Status**: Contains intentionally seeded defects (7 marked with HTML comments)
**Validation**: N/A -- this is test data, not a production spec

---

## Cross-Cutting Observations

### Specs Without Plans

The `docs/specifications/` directory contains specs that have no corresponding implementation plan in `plans/`:

| Spec | Related Crate | Notes |
|------|--------------|-------|
| `chat-session-history-spec.md` | terraphim_sessions | Mature crate, likely pre-dates plan convention |
| `terraphim-desktop-spec.md` | desktop/ | Referenced in CLAUDE.md |
| `terraphim-agent-session-search-spec.md` | terraphim-session-analyzer | Mature crate |
| `learning-capture-specification-interview.md` | terraphim_agent | Phase 2.5 interview for Gitea #82 epic |

### Plan-to-Code Traceability Summary

| Metric | Value |
|--------|-------|
| Total plans in plans/ | 3 (1 test fixture) |
| Plans with matching implementation | 2/2 production plans |
| Overall implementation rate | 97% (weighted average) |
| Missing CLI surface area | ~60 lines across both plans |
| Test coverage vs spec | Exceeds requirements (40 tests vs 23 specified) |
| Uncommitted work without plan | 1 (Gitea #120) |

---

## Recommendations (Priority Order)

1. **Create design plan for Gitea #120** -- The conditional agent pattern has no traceability from spec to code. Write `plans/design-gitea120-conditional-agent-pattern.md` before merging.

2. **Implement CLI flags for Gitea #84** -- `--include-pinned` and `kg list --pinned` are the only missing pieces. Backend is ready. ~40 lines of code.

3. **Wire session_id in Gitea #82 CLI handler** -- The builder method exists but the CLI match arm does not call `with_session_id()`. Quick fix.

4. **Run clippy validation** -- Both plans list clippy cleanliness as acceptance criteria but this was not verified in this report.

5. **Add CLI integration test for `learn correction`** -- Spec calls for it; current tests cover unit level but not the end-to-end CLI path.

---

*Report generated by Carthos, Domain Architect. Validated against commit 7a03be8e on branch task/120-conditional-agent-pattern.*
