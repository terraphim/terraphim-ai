# Design & Implementation Plan: Remove Unused Imports

## 1. Summary of Target Behavior

After implementation, `cargo test -p terraphim_agent` and `cargo test -p terraphim_orchestrator` will produce zero unused import warnings. The workspace will pass `cargo clippy --workspace -- -D warnings`.

## 2. Key Invariants and Acceptance Criteria

- No functional code changes
- All existing tests continue to pass
- Zero compiler warnings for unused imports
- Code formatting unchanged (`cargo fmt` passes)

## 3. High-Level Design and Boundaries

This is a pure cleanup task with no architectural changes. Two import statements will be removed from separate crates.

## 4. File/Module-Level Change Plan

| File | Action | Before | After |
|------|--------|--------|-------|
| `crates/terraphim_agent/src/mcp_tool_index.rs:252` | Delete line | `use std::time::Instant;` | (removed) |
| `crates/terraphim_orchestrator/src/learning.rs:1115-1117` | Modify import | `use terraphim_types::shared_learning::{LearningSource, LearningStore as _, SharedLearning};` | `use terraphim_types::shared_learning::{LearningSource, SharedLearning};` |

## 5. Step-by-Step Implementation Sequence

1. **Remove `Instant` import**: Delete `use std::time::Instant;` from `mcp_tool_index.rs`
2. **Remove `LearningStore` import**: Remove `LearningStore as _` from `learning.rs` import statement
3. **Verify**: Run `cargo test -p terraphim_agent` and `cargo test -p terraphim_orchestrator`
4. **Lint**: Run `cargo clippy --workspace -- -D warnings`
5. **Format**: Run `cargo fmt --all`

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test | Command |
|---------------------|------|---------|
| No unused import warnings | Compilation | `cargo test -p terraphim_agent` |
| No unused import warnings | Compilation | `cargo test -p terraphim_orchestrator` |
| Clippy passes | Lint | `cargo clippy --workspace -- -D warnings` |
| Format check passes | Style | `cargo fmt --all -- --check` |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Import used in conditional compilation | Verify with `cargo test` | None - tests pass |
| Import used in different feature flags | Check with `cargo check --all-features` | Minimal |

## 8. Open Questions / Decisions for Human Reviewer

None - this is a straightforward cleanup.
