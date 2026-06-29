# Gap Remediation Plan — Disciplined Research & Design

**Date**: 2026-06-29
**Gaps identified**: 3 active, 1 already fixed

---

## Gap 1: BlockerKind Classification (PR #2588, Issue #2465)

### Research

**What**: Add `BlockerKind` enum to distinguish CI failures (`ci_failed`, `ci_pending`, `ci_no_status`) from policy holds (`not_mergeable`) in merge coordinator logs.

**Problem**: Operators cannot tell whether a blocked PR failed CI (needs code fix), is pending CI (needs patience), or hit a policy gate (needs human review). Example: PR can be held by CI failure alone, but logs just say "not mergeable".

**Current state on main**: No `BlockerKind` enum exists. `evaluate_one` is synchronous, no CI status querying. `PrSummary` has no `sha` field for commit lookup.

**PR #2588 branch analysis**: 44 files, 27K lines — 90%+ noise (3 session dumps at 9K lines, 2 reconcile snapshots, handoff files). Core code is ~200 lines in `types.rs` (BlockerKind enum), `gitea.rs` (CommitCombinedStatus, get_commit_status, sha field), `evaluator.rs` (async evaluate_one with classification), and `lib.rs` (poll_pending_reviews).

**Essential Questions**:
| Question | Answer |
|----------|--------|
| Energising? | Yes — operators need this daily |
| Leverages strengths? | Yes — core merge coordinator code |
| Meets real need? | Yes — Issue #2465 has real-world examples |

### Design

**Approach**: Cherry-pick only the BlockerKind core from the branch. Create a clean branch from main with 4 files touched.

**Scope**:
| File | Change |
|------|--------|
| `types.rs` | Add `BlockerKind` enum + `Display` impl |
| `gitea.rs` | Add `PrSummary.sha`, `CommitCombinedStatus` struct, `get_commit_status()` |
| `evaluator.rs` | Make `evaluate_one` async, classify blocker via CI status |
| `lib.rs` | Add `poll_pending_reviews()` or enrich existing poll with blocker_kind |

**API Design**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BlockerKind {
    CiFailed,
    CiPending,
    CiNoStatus,
    NotMergeable,
}
```

**Test Strategy**:
| Test | Purpose |
|------|---------|
| `test_blocker_kind_ci_failed` | CI status "failure" → CiFailed |
| `test_blocker_kind_ci_pending` | CI status "pending" → CiPending |
| `test_blocker_kind_no_status` | No CI status → CiNoStatus |
| `test_blocker_kind_not_mergeable` | Mergeable=False, CI green → NotMergeable |

**Estimated effort**: 1 hour

**Verdict**: IMPLEMENT on clean branch. Close PR #2588 with comment linking to new PR.

---

## Gap 2: FirecrackerExecutor::ensure_pool() (PR #2590, #2521, Issue #2521)

### Research

**What**: Implement Firecracker VM pool warming so `ensure_pool()` doesn't always return Err.

**Current state on main**: `ensure_pool()` IS implemented. The function:
1. Checks if pool already exists (returns clone)
2. Creates PoolConfig from RLM config
3. Initialises pool with min/max/target sizes

The function signature exists and compiles. Issue #2521 reported "always returns Err" but the current implementation shows it CAN return Ok.

**PR #2590**: Closed. 42 files, 27K lines (similar noise pattern to #2588).

**Verdict**: **ALREADY FIXED**. The `ensure_pool()` stub was replaced with a real implementation during the merge sprint (likely via PR #2902 or executor changes). Issue #2521 should be closed.

**Close comment**: "`ensure_pool()` is now implemented in main (firecracker.rs) with real PoolConfig from RlmConfig. Verify: `grep -A20 'async fn ensure_pool' crates/terraphim_rlm/src/executor/firecracker.rs`"

---

## Gap 3: Path::starts_with in validate_path (PR #2898, Issue #2884)

### Research

**What**: Use `Path::starts_with` to prevent sibling-directory bypass in `validate_path()`. String-based `starts_with` allows `/safe/a` to match `/safe-attack/x` — Path-based comparison doesn't.

**Current state on main**: `validate_path()` does NOT exist in `gitea.rs`. There's no path validation function in the merge coordinator. This gap is larger than expected — the entire path validation is missing, not just the Path::starts_with fix.

**PR #2898**: Closed (mergeable=True when closed). 1 file, +48/-9 lines. The PR adds `validate_path()` to `gitea.rs` with `Path::starts_with` and tests.

**Verdict**: **NEEDS REOPENING + REBASE**. The PR was closed as "already merged" but the code is not in main. The branch `task/2884-path-starts-with-fix` likely has the changes. Rebase and re-merge.

**Essential Questions**:
| Question | Answer |
|----------|--------|
| Energising? | Medium — security fix for path traversal |
| Leverages strengths? | Yes — simple Rust standard library usage |
| Meets real need? | Yes — Issue #2884 describes a real sibling-dir bypass |

**Estimated effort**: 15 minutes (rebase + merge)

---

## Gap 4: Contamination Gate Pagination (Issue #2409)

### Research

**What**: `list_pr_files` doesn't paginate — returns only the first page of files. PRs with >50 changed files have their contamination gate silently skipped because files from page 2+ are missed.

**Current state on main**: 
- `list_pr_files()` exists in `gitea.rs` — calls Gitea API but without `?page=2` pagination
- Contamination gate in `evaluator.rs` does NOT exist (empty grep)
- The entire contamination check is missing from main
- Issue #2409 is open

**More complex than initially thought**: The gap has TWO parts:
1. Add pagination to `list_pr_files()` (loop through pages)
2. Wire the contamination gate into `evaluate_all()` (check file lists for banned patterns)

**Verdict**: **MEDIUM EFFORT**. Split into two sub-tasks:
- Sub-task A: Paginate `list_pr_files()` (simple API loop)
- Sub-task B: Wire contamination gate into evaluation (new logic)

**Estimated effort**: 2-3 hours total

---

## Summary

| # | Gap | Status | Effort | Next Step |
|---|-----|--------|--------|-----------|
| 1 | BlockerKind classification | NOT in main | 1h | Extract from PR #2588, create clean PR |
| 2 | ensure_pool() | ALREADY FIXED | 0 | Close issue #2521 |
| 3 | Path::starts_with validate_path | PR closed, code missing | 15min | Rebase PR #2898, re-merge |
| 4a | list_pr_files pagination | NOT in main | 1h | Add page loop to API call |
| 4b | Contamination gate wiring | NOT in main | 2h | New feature in evaluator.rs |

### Priority Order
1. **Gap 3** (15 min) — Quickest win, PR already written
2. **Gap 2** (0 min) — Already fixed, just close issue
3. **Gap 1** (1h) — Most operator-facing value
4. **Gap 4** (3h) — Largest scope, split into A/B
