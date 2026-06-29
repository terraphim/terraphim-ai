# Implementation Plan: Extract remaining value from PR #2692

**Status**: Draft
**Research Doc**: `.docs/research-pr2692-kg-validation.md`
**Author**: AI Agent
**Date**: 2026-06-29
**Estimated Effort**: 30 minutes

## Overview

### Summary
Add `from_config()` method to `KnowledgeGraphValidator` (deduplicating `build_validator` in rlm.rs and executor/mod.rs), add `#[derive(Debug)]` to `ExecuteResult`, and add 5 tests for `from_config()`. Close PR #2692 as superseded.

### Scope
**In Scope:**
- `from_config()` on `KnowledgeGraphValidator`
- `strictness()` accessor
- 5 unit tests for `from_config()`
- `#[derive(Debug)]` on `ExecuteResult`

**Out of Scope:**
- Restructuring QueryLoop validation (main has a better architecture)
- Adding `kg_thesaurus_path` config field (main uses `thesaurus` field differently)
- Noise files from PR (39 files of handoff/review artifacts)

**Avoid At All Cost:**
- Rewriting existing validation architecture
- Cherry-picking the PR's competing QueryLoop approach
- Pulling in .review_tmp/, .handoff/, .beads/, .sessions/ files

## Architecture

### Design Decision
Main already has a working validation architecture (excutor-based validate_command). The PR's approach (direct validator field in QueryLoop) is incompatible and unnecessary. We extract only the complementary pieces.

### Simplicity Check
The simplest approach: add `from_config()` to `KnowledgeGraphValidator` as a convenience constructor. Don't change anything else. Don't break existing code.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_rlm/src/validator.rs` | Add `from_config()`, `strictness()` accessor, 5 tests |
| `crates/terraphim_rlm/src/query_loop.rs` | Add `#[derive(Debug)]` on `ExecuteResult` |

### Files NOT Touched
- `rlm.rs` — existing `build_validator` stays (migration deferred)
- `executor/mod.rs` — existing pattern stays
- `config.rs` — existing fields stay
- All noise files — excluded

## API Design

### `KnowledgeGraphValidator::from_config()`
```rust
/// Build a validator from RLM config fields.
///
/// Eliminates the build_validator() duplication between rlm.rs and
/// executor/mod.rs. Optionally loads a thesaurus from disk.
pub fn from_config(
    strictness: KgStrictness,
    max_retries: u32,
    thesaurus_path: Option<&str>,
) -> Self
```

### `KnowledgeGraphValidator::strictness()`
```rust
/// Return the strictness level this validator is configured with.
pub fn strictness(&self) -> KgStrictness
```

## Test Strategy

### Tests to Add (in validator.rs)
| Test | Purpose |
|------|---------|
| `test_from_config_normal_no_path_has_no_thesaurus` | GAP-2: deduplication |
| `test_from_config_permissive_no_path_passes_silently` | Permissive always passes |
| `test_from_config_strict_respects_max_retries` | Config propagation |
| `test_from_config_bad_path_falls_back_gracefully` | GAP-1: no panic on bad path |
| `test_from_config_valid_thesaurus_json` | GAP-1: real thesaurus loading |

## Implementation Steps

### Step 1: Add `from_config()` and `strictness()` to validator.rs
**Files:** `crates/terraphim_rlm/src/validator.rs`
**Description:** Add the two methods after the existing constructors
**Tests:** Implicit (next step)

### Step 2: Add tests for `from_config()`
**Files:** `crates/terraphim_rlm/src/validator.rs`
**Description:** Add 5 unit tests in the `mod tests` block
**Dependencies:** Step 1

### Step 3: Add `#[derive(Debug)]` to `ExecuteResult`
**Files:** `crates/terraphim_rlm/src/query_loop.rs`
**Description:** Add `Debug` derive to the enum

### Step 4: Verify build + close PR
**Files:** None
**Description:** `cargo check --workspace`, `cargo test -p terraphim_rlm --lib`, close PR #2692
