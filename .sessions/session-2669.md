# Session #2669 — Step 2: terraphim_lsp KG Analysis Engine (property test gap)

**Agent**: Echo (implementation-swarm-A) — Twin Maintainer
**Issue**: #2669 (priority/P1-high, component/lsp, type/enhancement, epic #2667)
**Branch**: task/2669-lsp-kg-analysis-proptest
**Started**: 2026-06-21
**Status**: in progress

## Pre-flight / checkpoint
- Skipped #2839/#4028 (server.rs tests) — already has PR #2847 (task/2839-lsp-server-unit-tests)
- Skipped all grep issues (#2721/#2722) — terraphim_grep lives in terraphim-clients polyrepo, NOT this worktree
- Skipped #2754 — conflicts with in-flight #2821 (MSRV 1.91.0 makes floor_char_boundary legal)
- Verified #2669 has NO branch (416 remote task branches checked) and NO PR

## Current state of the code (verified on main)
`crates/terraphim_lsp/src/kg_analysis.rs` already implements the FULL #2669 API:
- `KgAnalysis { matched_terms, unknown_terms }` ✓
- `TermMatch { term, range, description }` ✓
- `analyse_kg_document(text, thesaurus) -> KgAnalysis` using terraphim_automata::find_matches (Aho-Corasick) ✓
- 6 unit tests pass: empty_text, empty_thesaurus, matched_terms_found, unknown_terms_found, positions_populated, never_panics(5 hand-picked inputs)

## Unmet acceptance criterion (the gap)
- [x] Unit tests for term matching against sample markdown — DONE
- [ ] **Property test: analyse_kg_document never panics on arbitrary input** — NOT DONE
  (current "never panics" test uses only 5 hand-picked strings, not a true property test)

## Plan
1. Add `proptest` to `crates/terraphim_lsp/Cargo.toml` [dev-dependencies]
2. Add `proptest!` block to `kg_analysis.rs` tests asserting analyse_kg_document never panics
   on arbitrary `String` input (strategy: any::<String>(), including invalid UTF-8-adjacent
   edge cases handled by split_whitespace). Run a configurable number of cases (default 256).
3. Quality gates: cargo fmt --check, cargo clippy -D warnings, cargo test -p terraphim_lsp
4. Commit + push + PR titled "Fix #2669: add property test for kg_analysis never-panic invariant"

## Key decisions
- proptest dev-dep only (no runtime dep) — minimal blast radius
- Keep existing 6 unit tests unchanged (surgical changes protocol)
- Property asserts Ok-or-panic-free only; semantic correctness covered by unit tests
