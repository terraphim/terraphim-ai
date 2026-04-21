# Research Document: PR #818 CI Failures -- Clippy/Lint Fixes

**Status**: Approved
**Author**: opencode (disciplined-research)
**Date**: 2026-04-17
**Related**: GitHub PR #818, Gitea #576 (closed)

## Executive Summary

PR #818 (feat/automata-eval-cli branch) fails CI due to `sort_by` usages that Rust 1.95's clippy now flags as errors with `-D warnings`. The failures are in **4 crates** not touched by this branch. The fix is mechanical: replace `sort_by` with `sort_by_key` at 16 locations across 5 files.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Unblocks PR merge, cleans tech debt |
| Leverages strengths? | Yes | Mechanical refactoring, well-understood |
| Meets real need? | Yes | PR cannot merge without this |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
GitHub PR #818 has 3 failing CI checks:
1. **Rust Clippy** -- `sort_by_key` lint errors in `terraphim-markdown-parser` and `terraphim-router`
2. **lint-and-format** -- `sort_by_key` lint errors in `terraphim-session-analyzer` (11 errors)
3. **Rust Unit Tests** -- CANCELLED (dependency of failed checks)

### Root Cause
CI uses Rust 1.95.0 (released 2026-04-14) while local dev is on 1.94.1. Rust 1.95 promoted `clippy::unnecessary_sort_by` to a hard error under `-D warnings`.

### Impact
- PR #818 cannot merge
- Gitea #576 is prematurely closed
- Any future PR will hit the same CI wall until these are fixed

### Success Criteria
- All CI checks pass on PR #818
- No new code changes in the branch's own files (fixes are in other crates)

## Current State Analysis

### Files With `sort_by` Needing Conversion

| File | Line | Current | Fix |
|------|------|---------|-----|
| `terraphim-markdown-parser/src/lib.rs` | 144 | `sort_by(\|a, b\| b.range.start.cmp(&a.range.start))` | `sort_by_key(\|b\| std::cmp::Reverse(b.range.start))` |
| `terraphim_router/src/keyword.rs` | 61 | `sort_by(\|a, b\| b.1.cmp(&a.1))` | `sort_by_key(\|b\| std::cmp::Reverse(b.1))` |
| `terraphim-session-analyzer/src/analyzer.rs` | 210 | `sort_by(\|a, b\| a.timestamp.cmp(&b.timestamp))` | `sort_by_key(\|a\| a.timestamp)` |
| `terraphim-session-analyzer/src/analyzer.rs` | 564 | `sort_by(\|a, b\| b.1.cmp(&a.1))` | `sort_by_key(\|b\| std::cmp::Reverse(b.1))` |
| `terraphim-session-analyzer/src/analyzer.rs` | 637 | `sort_by(\|a, b\| b.usage_count.cmp(&a.usage_count))` | `sort_by_key(\|b\| std::cmp::Reverse(b.usage_count))` |
| `terraphim-session-analyzer/src/analyzer.rs` | 880 | `sort_by(\|a, b\| b.frequency.cmp(&a.frequency))` | `sort_by_key(\|b\| std::cmp::Reverse(b.frequency))` |
| `terraphim-session-analyzer/src/parser.rs` | 422 | `sort_by(\|a, b\| a.timestamp.cmp(&b.timestamp))` | `sort_by_key(\|a\| a.timestamp)` |
| `terraphim-session-analyzer/src/reporter.rs` | 168 | `sort_by(\|a, b\| a.0.cmp(&b.0))` | `sort_by_key(\|a\| a.0)` |
| `terraphim-session-analyzer/src/reporter.rs` | 227 | `sort_by(\|a, b\| b.1.cmp(&a.1))` | `sort_by_key(\|b\| std::cmp::Reverse(b.1))` |
| `terraphim-session-analyzer/src/reporter.rs` | 447 | `sort_by(\|a, b\| b.1.total_invocations.cmp(&a.1.total_invocations))` | `sort_by_key(\|b\| std::cmp::Reverse(b.1.total_invocations))` |
| `terraphim-session-analyzer/src/reporter.rs` | 548 | Multi-key sort | See note below |
| `terraphim-session-analyzer/src/reporter.rs` | 570 | `sort_by(\|a, b\| b.1.cmp(&a.1))` | `sort_by_key(\|b\| std::cmp::Reverse(b.1))` |
| `terraphim-session-analyzer/src/reporter.rs` | 763 | `sort_by(\|a, b\| b.1.cmp(&a.1))` | `sort_by_key(\|b\| std::cmp::Reverse(b.1))` |
| `terraphim-session-analyzer/src/reporter.rs` | 784 | `sort_by(\|a, b\| b.1.total_invocations.cmp(&a.1.total_invocations))` | `sort_by_key(\|b\| std::cmp::Reverse(b.1.total_invocations))` |

**Note on line 548**: Multi-key sort may need to remain as `sort_by` or be refactored to a tuple key. Needs inspection.

### Additional: `sort_by` in analyzer.rs line 718 and 739

```
718: stats.sort_by(|_, v1, _, v2| v2.total_invocations.cmp(&v1.total_invocations));
739: breakdown.sort_by(|_, v1, _, v2| v2.cmp(v1));
```

These use 4-param closures (iterating over `(K, V)` pairs). The clippy lint may not fire for these since the unused params are named `_`, but they should be verified.

## Constraints

### Technical
- Must work on Rust 1.95.0 (CI)
- Must not break Rust 1.94.1 (local dev)
- `std::cmp::Reverse` is stable since Rust 1.0 -- no compatibility issue
- `sort_by_key` does not support descending order natively -- requires `Reverse` wrapper

### Non-Functional
- Zero runtime behaviour change (semantically equivalent)
- No new dependencies

## Vital Few

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Fix all 16 sort_by locations | Any one failure blocks CI | CI uses `-D warnings` (all warnings = errors) |
| Keep changes on feat/automata-eval-cli branch | Fixes are needed for this PR to pass | PR branch is the target |

## Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| `tui_desktop_parity_test.rs` type annotation errors | Not part of this CI run's failures; separate issue |
| Upgrading local Rust to 1.95 | Not needed for the fix; fix works on both versions |

## Assumptions

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `sort_by_key` with `Reverse` is semantically identical | Both produce descending sort | Low -- same comparison | Yes |
| Line 548 is a multi-key sort needing special handling | Clippy output shows 11 errors, not 16 | Medium -- may need `sort_by` retention with allow attribute | No (needs inspection) |

## Recommendations

**Proceed**: Yes. Mechanical fix, low risk.

1. Fix all `sort_by` -> `sort_by_key` conversions in the 5 files
2. Push to `feat/automata-eval-cli` branch
3. Verify CI passes
4. Optionally reopen Gitea #576 until PR merges
