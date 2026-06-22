# Spec Validation Report: Issue #2886

**Date:** 2026-06-22
**Validator:** Carthos (spec-validator)
**Issue:** [#2886 — test(merge-coordinator): PrFile deserialization path has no test](https://git.terraphim.cloud/terraphim/terraphim-ai/issues/2886)
**Verdict:** FAIL (gap confirmed on main; fix present in PR #2891, not yet merged)

---

## Executive Summary

Commit `2f0c6af6e` added `PrFile` struct and `list_pr_files` method to
`crates/terraphim_merge_coordinator/src/gitea.rs` but included no deserialization
test.  The acceptance criteria in issue #2886 are not met on `main`.

PR #2891 (`task/2886-pr-file-deserialization-test`) supplies the required tests.
All three acceptance criteria are satisfied on that branch and the test suite
passes locally.  The PR is currently blocked by `native-ci / build (push) - native
build failed` and is `mergeable=false`.

---

## Gap Analysis

### Current State (main, HEAD `2f0c6af6e`)

| Symbol | Location | Status |
|--------|----------|--------|
| `PrFile` struct | `crates/terraphim_merge_coordinator/src/gitea.rs:36` | Present, no test |
| `list_pr_files` | `crates/terraphim_merge_coordinator/src/gitea.rs:93` | Present, no test |
| `pr_file_deserialises_filename` test | `gitea.rs` tests module | **MISSING** |
| `pr_file_list_extracts_filenames` test | `gitea.rs` tests module | **MISSING** |

### Fix State (PR #2891 branch `task/2886-pr-file-deserialization-test`)

| Acceptance Criterion | Status |
|----------------------|--------|
| `pr_file_deserialises_filename` test added | PASS |
| Test verifies `[{"filename":"src/main.rs"}]` round-trip | PASS |
| `cargo test -p terraphim_merge_coordinator` passes | PASS (26/26 locally) |

Additional tests beyond acceptance criteria (hardening):
- `pr_file_list_extracts_filenames` — list with multiple entries
- `pr_file_unknown_fields_ignored` — tolerance for extra Gitea API fields

---

## Traceability Matrix

| Req ID | Requirement | Impl Ref | Test Ref | Status |
|--------|-------------|----------|----------|--------|
| REQ-1 | `pr_file_deserialises_filename` test in `gitea.rs` | `gitea.rs:36` | PR #2891 `gitea.rs:231` | ❌ on main / ✅ on PR branch |
| REQ-2 | JSON `[{"filename":"..."}]` round-trip verified | `gitea.rs:105-108` | PR #2891 `gitea.rs:238` | ❌ on main / ✅ on PR branch |
| REQ-3 | `cargo test -p terraphim_merge_coordinator` passes | whole crate | 26/26 locally with PR | ❌ CI failing on PR branch |

---

## Blocker

PR #2891 CI status: `native-ci / build (push) - native build failed`.

The test suite itself passes locally (26/26).  The CI build failure is a separate
concern (likely a workspace-level compilation issue unrelated to the new tests).
This must be resolved before merge.

---

## Recommendations

1. Diagnose the `native build failed` CI failure on PR #2891.  The test logic is
   correct; the blocker appears to be a workspace compilation step.
2. Once CI is green, merge PR #2891 to close issue #2886.
3. No scope changes needed — the acceptance criteria are precise and the PR satisfies
   all three.
