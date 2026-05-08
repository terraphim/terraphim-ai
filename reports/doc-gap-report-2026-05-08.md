# Documentation Gap Report 2026-05-08

**Generated:** 2026-05-08 12:43 CEST
**Scope:** Full workspace (`cargo doc --no-deps --workspace`)
**Outcome:** ZERO rustdoc warnings

## Summary

| Category | Count | Status |
|----------|-------|--------|
| Crate-level `//!` docs missing | 0 | PASS |
| `rustdoc::missing_docs` warnings | 0 | PASS |
| `rustdoc::private_intra_doc_links` warnings | 1 (fixed) | FIXED |
| Total doc warnings after fix | 0 | PASS |

## Fix Applied

**File:** `terraphim_server/src/error.rs:116`

**Warning:** `public documentation for Result links to private item APIError`

**Root cause:** The public `Result<T>` type alias had an intra-doc link
`` [`APIError`] `` pointing to the private newtype `APIError(StatusCode, anyhow::Error)`.
rustdoc emits a `private_intra_doc_links` warning for such links.

**Fix:** Replaced `` [`APIError`] `` with `` `APIError` `` (backtick reference, no link).
The documentation still names the type; it simply does not attempt to hyperlink to a
private item.

## Crate Coverage Audit

All 56 workspace crates now have crate-level `//!` documentation.

Previously undocumented crates documented in prior sessions:
- `terraphim_Service`, `terraphim_settings`, `terraphim_agent`, `terraphim_file_Search`
- `terraphim_Graph_linter`, `terraphim_ccusage`, `terraphim_usage`, `terraphim_build_args`
- `terraphim_lsp`, `terraphim_automata_py`, `terraphim_roleTerraphim-graph_py`
- `terraphim-markdown-parser`, `Haystack_core`, `Haystack_atlassian`, `Haystack_discourse`
- `Haystack_grepapp`, `Haystack_jmap`, `terraphim_Database`, `terraphim_mcp_server`
- `terraphim_config`, `terraphim_roleTerraphim-graph`, `terraphim_Service`
- `terraphim_dsm`, `terraphim_github_runner_server`

## No Further Gaps Found

`cargo doc --no-deps --workspace` produced **zero warnings** after the fix.

Theme-ID: doc-gap
