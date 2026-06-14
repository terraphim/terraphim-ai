# Implementation Report: terraphim_lsp Foundation (Gitea #2668)

**Issue**: terraphim/terraphim-ai#2668
**Epic**: terraphim/terraphim-ai#2667
**Branch**: `task/2668-terraphim-lsp-foundation`
**Date**: 2026-06-13

## Summary

Restored `terraphim_lsp` to a compilable workspace member. The crate now builds from the workspace root with minimal KG-focused dependencies and a no-op `tower-lsp` server skeleton.

## Changes Made

### Root `Cargo.toml`
- Removed `"crates/terraphim_lsp"` from the `exclude` array so the `crates/*` glob includes it as a workspace member.

### `crates/terraphim_lsp/Cargo.lock`
- Deleted the orphaned per-crate lockfile. Resolution is now handled by the workspace `Cargo.lock`.

### `crates/terraphim_lsp/Cargo.toml`
- Changed `edition = "2021"` to `edition.workspace = true` to align with workspace edition 2024.
- Added minimal dependencies:
  - `terraphim_automata` (path, 1.20.2) -- Aho-Corasick term matching for Step 2
  - `terraphim_types` (path, 1.20.2) -- Shared domain types
  - `terraphim_rolegraph` (path, 1.20.2) -- KG connectivity for Step 3
  - `tower-lsp = "0.20"` -- LSP server framework
  - `tokio` (workspace) -- Async runtime
  - `serde_json` (workspace) -- JSON-RPC serialization
  - `log` (workspace) -- Logging
- Added `tower` dev-dependency for future integration tests.

### `crates/terraphim_lsp/src/lib.rs`
- Added module declarations: `pub mod kg_analysis; pub mod server;`.
- Re-exported `TerraphimLspServer`.
- Added a compilation smoke test.

### `crates/terraphim_lsp/src/kg_analysis.rs` (new)
- Placeholder module with module-level documentation for the Step 2 KG analysis engine.

### `crates/terraphim_lsp/src/server.rs` (new)
- Defined `TerraphimLspServer` with a `tower_lsp::Client` handle.
- Implemented `tower_lsp::LanguageServer` with no-op `initialize`, `initialized`, and `shutdown` methods.
- Added `run_stdio()` helper to launch the server over stdin/stdout.

## Verification

All verification commands were executed via `rch` remote compilation offloading:

```bash
rch exec -- env CARGO_TARGET_DIR=${TMPDIR:-/tmp}/rch_target_lsp cargo check -p terraphim_lsp
rch exec -- env CARGO_TARGET_DIR=${TMPDIR:-/tmp}/rch_target_workspace cargo check --workspace
rch exec -- env CARGO_TARGET_DIR=${TMPDIR:-/tmp}/rch_target_clippy cargo clippy -p terraphim_lsp --all-targets -- -D warnings
cargo fmt --all -- --check
rch exec -- env CARGO_TARGET_DIR=${TMPDIR:-/tmp}/rch_target_test cargo test -p terraphim_lsp
```

Results:
- `cargo check -p terraphim_lsp`: PASS
- `cargo check --workspace`: PASS (no regressions)
- `cargo clippy -p terraphim_lsp --all-targets -- -D warnings`: PASS
- `cargo fmt --all -- --check`: PASS
- `cargo test -p terraphim_lsp`: PASS (1 test)

## ADF Agent Usage

Local ADF agents were leveraged during this task:

1. **Useful-work-proof flow** -- `adf-ctl flow adf-useful-work-proof --context "issue=2668"` proved local flow execution.
2. **Disciplined-implementation-agent** -- Dispatched via `adf-ctl --local trigger terraphim-ai/disciplined-implementation-agent --context "issue=2668 stage=disciplined-implementation" --direct` with the local orchestrator running a Unix domain socket listener.

## Next Steps

- Step 2 (#2669): Implement `kg_analysis.rs` with Aho-Corasick term matching.
- Step 3 (#2670): Implement real LSP handlers for hover, completion, and diagnostics.

## Artefacts

- Research: `.docs/adf/2668/research.md`
- Design: `.docs/adf/2668/design.md`
- Implementation: `.docs/adf/2668/implementation.md` (this file)
