# Research Document: Remove Unused Imports Causing Compiler Warnings

## 1. Problem Restatement and Scope

The Rust compiler generates warnings for two unused imports in workspace crates:
1. `std::time::Instant` in `crates/terraphim_agent/src/mcp_tool_index.rs`
2. `LearningStore` trait in `crates/terraphim_orchestrator/src/learning.rs`

These warnings appear during `cargo test` and `cargo clippy`, creating noise in CI output and local development.

**IN scope**: Removing the two unused import statements.
**OUT of scope**: Any functional code changes, refactoring, or adding new features.

## 2. User & Business Outcomes

- Cleaner CI output with zero unused import warnings
- Reduced cognitive load for developers reading compiler output
- Compliance with project's clippy standards (`-D warnings`)

## 3. System Elements and Dependencies

| File | Role | Dependencies |
|------|------|-------------|
| `crates/terraphim_agent/src/mcp_tool_index.rs` | MCP tool indexing | Tests use `Instant` but not in the added import scope |
| `crates/terraphim_orchestrator/src/learning.rs` | Learning orchestration | `LearningStore` already imported via `terraphim_types::shared_learning` in test module |

## 4. Constraints and Their Implications

- **Constraint**: Must not break any tests or compilation
- **Constraint**: Must pass `cargo clippy --workspace -- -D warnings`
- **Constraint**: Must pass `cargo fmt --all -- --check`
- **Implication**: Changes are purely deletions; no risk of functional regression

## 5. Risks, Unknowns, and Assumptions

- **ASSUMPTION**: The imports are genuinely unused (confirmed by compiler warnings)
- **RISK**: Minimal - removing unused imports cannot break functionality
- **UNKNOWN**: Whether these imports were intended for future use (but warnings suggest they were added inadvertently)

## 6. Context Complexity vs. Simplicity Opportunities

This is a trivial cleanup task with no complexity. The fix is straightforward:
- Delete line 252 in `mcp_tool_index.rs`
- Delete or adjust line 1116 in `learning.rs`

## 7. Questions for Human Reviewer

1. Should we enable `#![deny(unused_imports)]` at the crate level to prevent future occurrences?
2. Is there a CI check that should have caught this earlier?
