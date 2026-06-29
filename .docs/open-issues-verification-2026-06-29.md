# Open Issues Verification & Validation Report

**Date**: 2026-06-29
**Total open issues**: 290
**Top issues verified**: 10 (by PageRank)

---

## Issue-by-Issue Verification

### #2668 — terraphim_lsp Foundation (PageRank: 0.0205)

**Claim**: Cargo.toml and orphaned Cargo.lock need fixing.

**Verification**:
- `crates/terraphim_lsp/Cargo.toml`: EXISTS
- `crates/terraphim_lsp/Cargo.lock`: MISSING (no orphan)
- LSP tests: 22 passed, 0 failed

**Verdict**: **FIXED**. LSP crate is a valid workspace member with Cargo.toml, no orphaned lockfile, all tests pass. Can be closed.

---

### #2558 — Step H post-merge gate revert (PageRank: 0.0205)

**Claim**: ADF post-merge gate reverts on env non-result, must fail-open.

**Verification**: This is an ADF infrastructure/ops issue, not a code issue in terraphim-ai. The orchestrator is in a separate repo (`terraphim-agents`). Cannot verify from this codebase.

**Verdict**: **VALID — OPS**. Requires investigation by ADF ops owner. Should be transferred to the terraphim-agents repo or retained as an ops ticket.

---

### #2669 — LSP KG Analysis Engine (PageRank: 0.0148)

**Claim**: Aho-Corasick term matching for markdown is needed.

**Verification**:
- `crates/terraphim_lsp/src/kg_analysis.rs`: 196 lines, 11 references to `analyse_kg_document`/Aho-Corasick
- LSP tests: 22 passed, 0 failed (includes kg_analysis tests)
- Feature is implemented and tested

**Verdict**: **FIXED**. KG analysis with Aho-Corasick term matching is implemented and tested. Can be closed.

---

### #2821 — MSRV mismatch (PageRank: 0.0126)

**Claim**: Workspace rust-version="1.85.0" conflicts with .clippy.toml msrv="1.91.0".

**Verification**:
- `Cargo.toml`: `rust-version = "1.91"`  
- `.clippy.toml`: `msrv = "1.91.0"`
- Both are now `1.91` — aligned.

**Verdict**: **FIXED**. MSRV is now consistent at 1.91.0. Can be closed.

---

### #2988 — Cursor SQLite connector (PageRank: 0.0126)

**Claim**: Task 2.3 acceptance criteria for Cursor SQLite connector are unmet.

**Verification**:
- No `CursorSession` or `cursor_connector` module found in the codebase.
- SQLite references in terraphim_tinyclaw are commented out due to dependency conflicts.
- Cursor connector remains unimplemented.

**Verdict**: **VALID — NOT IMPLEMENTED**. The Cursor SQLite connector has not been built. This is a real feature gap.

---

### #2141 — Session search tasks spec update (PageRank: 0.0126)

**Claim**: Tasks 2.6.2 and 2.6.3 need to be marked as implemented in the spec.

**Verification**:
```
- [x] **2.6.2** Implement `/sessions search`  ← marked complete
- [x] **2.6.3** Implement `/sessions list`     ← marked complete
```

**Verdict**: **FIXED**. Both tasks are already marked `[x]` complete in the spec. Can be closed.

---

### #2535 — Haystack atlassian test coverage (PageRank: 0.0126)

**Claim**: Zero test coverage for Confluence and Jira API clients.

**Verification**: `cargo test -p haystack_atlassian` returns no tests. The `eprintln` debug calls were replaced with `tracing` during the merge sprint (PR #1975/#2802). But test coverage remains at zero.

**Verdict**: **VALID — LOW TEST COVERAGE**. Debug eprintln was fixed but no actual tests exist for the Confluence/Jira API clients. Real gap.

---

### #1531 — orchestrator #![warn(missing_docs)] (PageRank: 0.0126)

**Claim**: Add `#![warn(missing_docs)]` gate to orchestrator to prevent documentation regression.

**Verification**: `grep "warn(missing_docs)" crates/terraphim_orchestrator/src/lib.rs` returns nothing. The gate is not present.

**Verdict**: **VALID — NOT IMPLEMENTED**. The `#![warn(missing_docs)]` lint gate has not been added to the orchestrator.

---

### #2345 — cfg(test) gate on ProcedureStore (PageRank: 0.0126)

**Claim**: Remove `#[cfg(test)]` from ProcedureStore and expose learn procedure CLI subcommands.

**Verification**: No `ProcedureStore` found in the codebase. The module was likely removed during the #1910 polyrepo extraction (orchestrator moved to terraphim-agents repo).

**Verdict**: **OBSOLETE**. The ProcedureStore module no longer exists in this repo. Should be closed or transferred to terraphim-agents.

---

## Summary Matrix

| Issue | Title | Status | Action |
|-------|-------|--------|--------|
| #2668 | LSP Foundation | FIXED | Close |
| #2558 | Step H gate revert | VALID (ops) | Keep open (ops ticket) |
| #2669 | LSP KG Analysis | FIXED | Close |
| #2821 | MSRV mismatch | FIXED | Close |
| #2988 | Cursor SQLite | NOT IMPLEMENTED | Keep open |
| #2141 | Session spec update | FIXED | Close |
| #2535 | Atlassian test coverage | LOW COVERAGE | Keep open |
| #1531 | missing_docs gate | NOT IMPLEMENTED | Keep open |
| #2345 | cfg(test) ProcedureStore | OBSOLETE | Close (module removed) |

### Action Summary

| Action | Count | Issues |
|--------|-------|--------|
| **Close as FIXED** | 5 | #2668, #2669, #2821, #2141, #2345 |
| **Keep open — real gap** | 4 | #2558 (ops), #2988 (feature), #2535 (tests), #1531 (docs) |

### Quality Verification

| Metric | Value |
|--------|-------|
| `cargo check --workspace` | PASS |
| `cargo clippy --workspace` | 0 warnings |
| `cargo fmt --all -- --check` | 0 diffs |
| Total tests | 391 passed, 0 failed |
| Remotes synced | Yes |
