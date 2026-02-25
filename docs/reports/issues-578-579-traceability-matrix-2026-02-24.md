# Traceability Matrix: Issues #578 and #579

**Date**: 2026-02-24
**Research**: `docs/plans/issues-578-579-research-2026-02-24.md`
**Design**: `docs/plans/issues-578-579-design-2026-02-24.md`

## Requirement -> Design -> Code -> Test -> Evidence

| Ticket | Requirement | Design Ref | Implementation | Verification Tests | Evidence |
|---|---|---|---|---|---|
| #578 | `terraphim-agent search` must honor machine-readable output mode (`--robot`, `--format json*`) | Output mode plumbing, Step 1/2 | `crates/terraphim_agent/src/main.rs` (`CommandOutputConfig`, search JSON output in server/offline commands) | `crates/terraphim_agent/tests/robot_search_output_regression_tests.rs` (3 tests) | `cargo test -p terraphim_agent --test robot_search_output_regression_tests` passed |
| #578 | Search output must not be polluted by TerraphimGraph status banner | Noise removal, Step 4 | `crates/terraphim_service/src/lib.rs` (`eprintln!` -> `log::debug!`) | Covered by robot JSON parsing tests (first JSON object parse + contract checks) | Robot-mode search output is parseable JSON |
| #579 | TerraphimGraph search must not false-zero when graph query is empty but haystack has lexical hits | Fallback behavior, Step 3 | `crates/terraphim_service/src/lib.rs` (graph-empty lexical fallback branch) | `crates/terraphim_service/tests/terraphim_graph_lexical_fallback_test.rs` | `cargo test -p terraphim_service --test terraphim_graph_lexical_fallback_test` passed |
| #579 | Multi-term behavior remains compatible while fallback is applied | Fallback behavior, Step 3 | `crates/terraphim_service/src/lib.rs` (`apply_logical_operators_to_documents` before lexical sort for multi-term) | Existing search flow + targeted fallback test | Code path verified in diff + targeted test for single-term fallback |

## Gaps and Follow-ups

| Gap | Impact | Loop-back Phase |
|---|---|---|
| UBS scan on `terraphim_service` diff workspace reports 4 critical findings (summary only, not line-level in generated report) | Verification gate is **conditional** until triaged | Phase 3 (implementation) / Phase 2 if architectural |
| Workspace clippy with `-D warnings` blocked by pre-existing `terraphim_types` derivable-impl warnings | Not introduced by this change, but blocks strict workspace lint gate | Phase 3 (existing technical debt) |
