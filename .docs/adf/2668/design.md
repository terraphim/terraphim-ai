# Implementation Plan: terraphim_lsp Foundation (Gitea #2668)

**Status**: Draft
**Research Doc**: `.docs/adf/2668/research.md`
**Author**: AI Agent
**Date**: 2026-06-13
**Issue**: terraphim/terraphim-ai#2668
**Epic**: terraphim/terraphim-ai#2667
**Estimated Effort**: 1 hour

## Overview

### Summary
Restore `terraphim_lsp` to a compilable workspace member by deleting its orphaned `Cargo.lock`, aligning its `Cargo.toml` with the workspace, declaring minimal KG-focused dependencies, and replacing the placeholder `lib.rs` with working module declarations and a no-op `tower-lsp` server.

### Approach
Make the smallest possible set of file changes so that `cargo check -p terraphim_lsp` succeeds from the workspace root. Do not implement real handlers; reserve module slots for Step 2 (`kg_analysis`) and Step 3 (`server`).

### Scope

**In Scope:**
- Root `Cargo.toml`: remove `crates/terraphim_lsp` from `exclude`.
- Delete `crates/terraphim_lsp/Cargo.lock`.
- Rewrite `crates/terraphim_lsp/Cargo.toml` (edition 2024, dependencies, metadata).
- Rewrite `crates/terraphim_lsp/src/lib.rs` (module declarations, no-op LSP server).
- Verification commands.

**Out of Scope:**
- Real hover/completion/diagnostics handlers (Step 3).
- KG term matching implementation (Step 2).
- LSP binary target (Step 3).
- CI workflow changes (Step 11).

**Avoid At All Cost:**
- Re-introducing EDM/negative-contribution diagnostics (historical scope, replaced by KG focus).
- Adding dependencies beyond the six listed in #2668.
- Implementing handler logic under the guise of "foundation".

## Architecture

### Component Diagram
```
Workspace
  └── crates/terraphim_lsp
        ├── Cargo.toml          # workspace edition, minimal deps
        └── src/lib.rs
              ├── mod kg_analysis;   # placeholder for Step 2
              ├── mod server;        # placeholder for Step 3
              └── TerraphimLspServer # no-op tower-lsp impl
```

### Data Flow
Not applicable for foundation step. Future flow (Step 3):
```
Editor LSP request → tower-lsp → TerraphimLspServer → kg_analysis (Step 2) → LSP response
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Path dependencies for core crates | `terraphim_automata`, `terraphim_types`, `terraphim_rolegraph` are local workspace members in this branch | Registry deps (unnecessary indirection) |
| No-op `LanguageServer` impl | Satisfies `tower-lsp` trait without implementing Step 2/3 logic | Partial handlers (scope creep) |
| Module declarations without bodies | Provides compile-time structure for Steps 2 and 3 | Inline everything in `lib.rs` (harder to review) |
| Remove `terraphim-lsp` binary feature gate | Step 3 will add the binary; Step 1 does not need it | Keep historical binary config (broken without server) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Full EDM diagnostic server | Historical scope, replaced by KG analysis in epic #2667 | Reintroduces dead code and `terraphim_negative_contribution` dependency |
| Registry dependencies for core crates | Adds registry auth complexity when local paths are available | Slower builds, potential version skew |
| Implement `kg_analysis.rs` in Step 1 | Blurs Step 1/2 boundary; Step 2 has its own issue (#2669) | Larger, harder-to-review PR |
| Add binary target now | Needs `server.rs` implementation first | Would not compile or would be empty |

### Simplicity Check

**What if this could be easy?** The easiest correct foundation is: un-exclude the crate, delete the stale lockfile, add six dependencies, and write a `lib.rs` that declares modules and implements the `tower-lsp` trait with empty methods. That is exactly this plan.

**Senior Engineer Test**: A senior engineer would call this appropriately minimal for a foundation step.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_lsp/src/kg_analysis.rs` | Module placeholder for Step 2 (empty or minimal docs) |
| `crates/terraphim_lsp/src/server.rs` | Module placeholder for Step 3 (no-op server struct) |

### Modified Files
| File | Changes |
|------|---------|
| `Cargo.toml` | Remove `"crates/terraphim_lsp"` from `exclude` |
| `crates/terraphim_lsp/Cargo.toml` | Edition 2024, add dependencies, keep metadata |
| `crates/terraphim_lsp/src/lib.rs` | Module declarations, re-exports, no-op server |

### Deleted Files
| File | Reason |
|------|---------|
| `crates/terraphim_lsp/Cargo.lock` | Orphaned; conflicts with workspace `Cargo.lock` |

## API Design

### Public Types
```rust
/// Placeholder LSP server implementing the tower-lsp LanguageServer trait.
///
/// Step 3 will add document tracking, KG analysis, and handlers.
#[derive(Debug)]
pub struct TerraphimLspServer;

impl TerraphimLspServer {
    pub fn new() -> Self {
        Self
    }
}
```

### Public Functions
```rust
/// Run the LSP server (placeholder for Step 3).
pub async fn run_lsp_server() -> anyhow::Result<()> {
    Ok(())
}
```

### Error Types
No custom error types in Step 1; use `anyhow::Result` for the placeholder runner.

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_server_can_be_constructed` | `src/lib.rs` | Verify `TerraphimLspServer::new()` works |

### Integration Tests
None for Step 1; integration tests for LSP protocol will be added in Step 3.

### Verification Commands
```bash
cargo check -p terraphim_lsp
cargo check --workspace
cargo clippy -p terraphim_lsp --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Implementation Steps

### Step 1: Un-exclude crate from workspace
**Files:** `Cargo.toml`
**Description:** Remove `"crates/terraphim_lsp"` from the `exclude` array.
**Tests:** `cargo check -p terraphim_lsp` will start resolving the crate.
**Estimated:** 5 minutes

### Step 2: Delete orphaned Cargo.lock
**Files:** `crates/terraphim_lsp/Cargo.lock`
**Description:** Delete the file; workspace `Cargo.lock` will take over.
**Tests:** `cargo check -p terraphim_lsp` uses workspace lockfile.
**Estimated:** 2 minutes

### Step 3: Fix Cargo.toml
**Files:** `crates/terraphim_lsp/Cargo.toml`
**Description:**
- Set `edition.workspace = true`.
- Add dependencies: `tower-lsp`, `tokio` (workspace), `serde_json` (workspace), `terraphim_automata`, `terraphim_types`, `terraphim_rolegraph`.
- Keep existing package metadata.
**Tests:** `cargo check -p terraphim_lsp` resolves dependencies.
**Estimated:** 15 minutes

### Step 4: Add module placeholders and no-op server
**Files:** `crates/terraphim_lsp/src/lib.rs`, `crates/terraphim_lsp/src/kg_analysis.rs`, `crates/terraphim_lsp/src/server.rs`
**Description:**
- `src/lib.rs`: declare `pub mod kg_analysis; pub mod server;`, re-export `TerraphimLspServer`, add unit test.
- `src/kg_analysis.rs`: empty module with module-level docs.
- `src/server.rs`: define `TerraphimLspServer` and implement `tower_lsp::LanguageServer` with empty async methods.
**Tests:** `cargo check -p terraphim_lsp` and `cargo test -p terraphim_lsp` pass.
**Dependencies:** Step 3
**Estimated:** 25 minutes

### Step 5: Verification
**Description:** Run the verification commands and fix any fmt/clippy issues.
**Tests:** All verification commands pass.
**Dependencies:** Step 4
**Estimated:** 15 minutes

## Rollback Plan

If issues discovered:
1. Restore `"crates/terraphim_lsp"` to `exclude` in root `Cargo.toml`.
2. Revert `crates/terraphim_lsp/Cargo.toml` and `src/lib.rs`.
3. Restore `crates/terraphim_lsp/Cargo.lock` if needed.

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| `tower-lsp` | 0.20 | LSP server framework |
| `tokio` | workspace | Async runtime |
| `serde_json` | workspace | LSP JSON-RPC |
| `terraphim_automata` | path | Aho-Corasick term matching (Step 2) |
| `terraphim_types` | path | Shared domain types |
| `terraphim_rolegraph` | path | KG connectivity (Step 3) |

### Dependency Updates
None.

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| `cargo check -p terraphim_lsp` | < 30s | Stopwatch |
| `cargo check --workspace` | No regression | Compare before/after |

### Benchmarks to Add
None in Step 1.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm `tower-lsp` 0.20 resolves with workspace tokio | Pending | Implementer |
| Decide whether to keep `readme = "../../README.md"` | Pending | Reviewer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
