# Spec Validation Report — Issue #2246

**Date**: 2026-06-11 21:36 CEST
**Agent**: spec-validator (Carthos, Domain Architect)
**Issue**: #2246 — docs: update Task 2.5 spec checkboxes — Cursor connector implemented via tsa-full feature
**Verdict**: PASS

---

## Executive Summary

The tasks specification document had stale acceptance criteria for Task 2.5 (Previous 2.3). The `ClaCursorConnector` implementation exists in the `terraphim-clients` polyrepo, feature-gated behind `tsa-full`. The main spec (F4.1) already reflected the implemented state. This report validates the spec update and confirms test continuity.

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Status |
|--------|-------------|-----------|----------|-------|--------|
| AC-1 | Task 2.5 ACs updated to `[x]` with tsa-full note | `docs/specifications/terraphim-agent-session-search-tasks.md` | `cla/connector.rs:78-133` | `cla::tests::test_cla_cursor_connector` | PASS |
| AC-2 | Status updated from `🔄 Planned` to reflect actual state | Tasks doc line 400 | Feature gate `#[cfg(feature = "tsa-full")]` | — | PASS |
| AC-3 | `cargo test -p terraphim_sessions` continues to pass | — | All test modules | 55 passed, 0 failed | PASS |

---

## Implementation Verification

### Location
`/data/projects/terraphim/terraphim-clients/crates/terraphim_sessions/src/cla/connector.rs`

### Key Boundaries

| Boundary | Detail |
|----------|--------|
| With `tsa-full` | Full `ClaCursorConnector` struct, delegates to `terraphim_session_analyzer::connectors::cursor::CursorConnector` |
| Without `tsa-full` | Stub struct with `source_id = "cursor-stub"`, `detect()` returns `Error("Cursor support requires tsa-full feature")` |
| SQLite reading | Handled by TSA layer — `terraphim_session_analyzer` manages Cursor's SQLite schema |
| Schema versions | Handled by TSA layer — no separate implementation needed in `terraphim_sessions` |

### Feature Gate Structure
```
[features]
tsa-full = ["terraphim-session-analyzer", "terraphim-session-analyzer/connectors"]
```

---

## Test Results

```
test result: ok. 55 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

Command: `rustup run stable cargo test -p terraphim_sessions --manifest-path /data/projects/terraphim/terraphim-clients/Cargo.toml`

---

## Spec Changes Made

File: `docs/specifications/terraphim-agent-session-search-tasks.md`

```diff
- **Status**: 🔄 Planned
+ **Status**: ✅ Implemented (Cursor connector behind `tsa-full` feature flag)

- - [ ] Reads Cursor SQLite database
- - [ ] Handles different schema versions
+ - [x] Reads Cursor SQLite database — implemented via `ClaCursorConnector` in `cla/connector.rs`, requires `tsa-full` feature flag; stub returns error without it
+ - [x] Handles different schema versions — delegated to `terraphim_session_analyzer::connectors::cursor::CursorConnector`
```

Commit: `8d9394cdbf`  
Branch: `task/2246-update-task-2.5-spec`  
PRs: #2478 (recommended), #2477 (duplicate)

---

## Gaps

None. All three acceptance criteria satisfied. No code changes required — implementation predates this issue; only documentation was stale.

---

## Structural Observations

The tasks document contains a naming collision: two sections titled "Task 2.5" (lines 396 and 443). This is a pre-existing issue not in scope for #2246 but worth noting for future cleanup. The section at line 443 ("Task 2.5: Hybrid KG-BM25 Search") is correctly marked `✅ Implemented`. The section at line 396 ("Task 2.5 (Previous 2.3): Implement Additional Connectors") is the one corrected by this fix.
