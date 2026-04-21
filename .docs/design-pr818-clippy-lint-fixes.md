# Implementation Plan: Fix CI `sort_by` Clippy Failures for PR #818

**Status**: Approved
**Research Doc**: `.docs/research-pr818-clippy-lint-fixes.md`
**Author**: opencode (disciplined-design)
**Date**: 2026-04-17
**Estimated Effort**: 30 minutes

## Overview

### Summary
Replace all `sort_by` calls flagged by Rust 1.95 clippy with `sort_by_key` equivalents. 13 mechanical conversions + 3 special cases.

### Approach
Mechanical search-and-replace. Each `sort_by` converting to `sort_by_key` either:
- **Ascending**: `sort_by(|a, b| a.x.cmp(&b.x))` -> `sort_by_key(|a| a.x)`
- **Descending**: `sort_by(|a, b| b.x.cmp(&a.x))` -> `sort_by_key(|b| std::cmp::Reverse(b.x))`

### Scope
**In Scope:**
- Fix all 13 clippy-flagged `sort_by` calls across 5 files in 3 crates
- Handle 3 special cases (multi-key, string-parse, IndexMap)

**Out of Scope:**
- `tui_desktop_parity_test.rs` type annotation errors
- Upgrading local Rust version
- Any changes to terraphim-cli or terraphim-automata (branch's own code)

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim-markdown-parser/src/lib.rs` | 1 sort_by -> sort_by_key |
| `crates/terraphim_router/src/keyword.rs` | 1 sort_by -> sort_by_key |
| `crates/terraphim-session-analyzer/src/analyzer.rs` | 4 sort_by -> sort_by_key + 2 IndexMap sort_by |
| `crates/terraphim-session-analyzer/src/parser.rs` | 1 sort_by -> sort_by_key |
| `crates/terraphim-session-analyzer/src/reporter.rs` | 7 sort_by -> sort_by_key |

### No new or deleted files.

## Implementation Steps

### Step 1: terraphim-markdown-parser (1 change)
**File:** `crates/terraphim-markdown-parser/src/lib.rs:144`

```rust
// Before:
edits.sort_by(|a, b| b.range.start.cmp(&a.range.start));
// After:
edits.sort_by_key(|b| std::cmp::Reverse(b.range.start));
```

### Step 2: terraphim_router (1 change)
**File:** `crates/terraphim_router/src/keyword.rs:61`

```rust
// Before:
matched_keywords.sort_by(|a, b| b.1.cmp(&a.1));
// After:
matched_keywords.sort_by_key(|b| std::cmp::Reverse(b.1));
```

### Step 3: terraphim-session-analyzer/analyzer.rs (6 changes)
**File:** `crates/terraphim-session-analyzer/src/analyzer.rs`

| Line | Before | After |
|------|--------|-------|
| 210 | `agents.sort_by(\|a, b\| a.timestamp.cmp(&b.timestamp))` | `agents.sort_by_key(\|a\| a.timestamp)` |
| 564 | `sorted.sort_by(\|a, b\| b.1.cmp(&a.1))` | `sorted.sort_by_key(\|b\| std::cmp::Reverse(b.1))` |
| 637 | `correlations.sort_by(\|a, b\| b.usage_count.cmp(&a.usage_count))` | `correlations.sort_by_key(\|b\| std::cmp::Reverse(b.usage_count))` |
| 718 | `stats.sort_by(\|_, v1, _, v2\| v2.total_invocations.cmp(&v1.total_invocations))` | `stats.sort_by(\|_, v1, _, v2\| v2.total_invocations.cmp(&v1.total_invocations))` **ALLOW** (IndexMap method, not Vec) |
| 739 | `breakdown.sort_by(\|_, v1, _, v2\| v2.cmp(v1))` | `breakdown.sort_by(\|_, v1, _, v2\| v2.cmp(v1))` **ALLOW** (IndexMap method, not Vec) |
| 880 | `chains.sort_by(\|a, b\| b.frequency.cmp(&a.frequency))` | `chains.sort_by_key(\|b\| std::cmp::Reverse(b.frequency))` |

**Lines 718, 739**: These use `IndexMap::sort_by` (not `Vec::sort_by`). Clippy's `unnecessary_sort_by` lint only fires on `Vec::sort_by`, so these should be safe. Verify after fix by running clippy. If they do fire, add `#[allow(clippy::unnecessary_sort_by)]`.

### Step 4: terraphim-session-analyzer/parser.rs (1 change)
**File:** `crates/terraphim-session-analyzer/src/parser.rs:422`

```rust
// Before:
events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
// After:
events.sort_by_key(|a| a.timestamp);
```

### Step 5: terraphim-session-analyzer/reporter.rs (7 changes)
**File:** `crates/terraphim-session-analyzer/src/reporter.rs`

| Line | Before | After |
|------|--------|-------|
| 168 | `events.sort_by(\|a, b\| a.0.cmp(&b.0))` | `events.sort_by_key(\|a\| a.0)` |
| 227 | `sorted_agents.sort_by(\|a, b\| b.1.cmp(&a.1))` | `sorted_agents.sort_by_key(\|b\| std::cmp::Reverse(b.1))` |
| 447 | `tool_stats.sort_by(\|a, b\| b.1.total_invocations.cmp(&a.1.total_invocations))` | `tool_stats.sort_by_key(\|b\| std::cmp::Reverse(b.1.total_invocations))` |
| 548 | Multi-key with string parse | Add `#[allow(clippy::unnecessary_sort_by)]` (see note) |
| 570 | `category_rows.sort_by(\|a, b\| b.1.cmp(&a.1))` | `category_rows.sort_by_key(\|b\| std::cmp::Reverse(b.1))` |
| 763 | `category_rows.sort_by(\|a, b\| b.1.cmp(&a.1))` | `category_rows.sort_by_key(\|b\| std::cmp::Reverse(b.1))` |
| 784 | `tool_list.sort_by(\|a, b\| b.1.total_invocations.cmp(&a.1.total_invocations))` | `tool_list.sort_by_key(\|b\| std::cmp::Reverse(b.1.total_invocations))` |

**Line 548**: Sorts by parsing `count` field from `String` to `u32`. Cannot be expressed as a pure key function because `parse()` is fallible. Add `#[allow(clippy::unnecessary_sort_by)]` attribute above the sort call.

### Step 6: Verify
```bash
rustup update stable  # Update to 1.95
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo fmt --check
```

Then push and monitor CI.

## Test Strategy

No new tests needed -- these are semantically equivalent transformations. Existing tests cover the sorting behaviour.

## Rollback Plan

`git revert` the commit if any sorting behaviour regresses.
