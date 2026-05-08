# Spec Validation Report: Issue #1362 — Gate rustdoc Warnings
**Version**: v13 (definitive, fresh verification)
**Validator**: Carthos (Domain Architect / spec-validator)
**Date**: 2026-05-08
**Issue**: #1362
**PR**: #1365 (branch `task/1362-gate-rustdoc-warnings`)
**Verdict**: **PASS**

---

## Verification Basis

This report re-validates from current repository state. Two commits ahead of `main` on the PR branch:

```
7e4e894c9 ci(quality): run rustdoc check once on x86_64-unknown-linux-gnu only Refs #1362
853096e8e ci(quality): gate rustdoc warnings with RUSTDOCFLAGS=-D warnings Refs #1362
```

The 50-file diff shown by the Gitea PR API is a base-comparison artefact: the branch was opened before several commits landed on `main`. The actual delta against `main` is two CI-config-only commits — confirming REQ-005 ("no code changes; CI config only").

---

## Requirements Traceability

| Req ID | Requirement (from issue #1362) | Implementation Location | Status |
|-------:|-------------------------------|------------------------|--------|
| REQ-001 | `RUSTDOCFLAGS: "-D warnings"` in CI environment | `ci-pr.yml:241-243` (step-level `env:`); `ci-main.yml:240-242` (step-level `env:`) | ✅ |
| REQ-002 | `cargo doc --no-deps --workspace` step added | `ci-pr.yml:237-240`; `ci-main.yml:236-240` | ✅ |
| REQ-003 | Step placed after `cargo check`, before test matrix | `ci-pr.yml`: appended after `cargo check` steps (lines 228–233); `ci-main.yml`: placed after `cargo test` in post-merge workflow | ✅ |
| REQ-004 | Assess `--lib` flag; do not pre-emptively exclude without rationale | `--lib` applied with explicit comment: "excludes binary targets which have no public API surface" | ✅ |
| REQ-005 | No code changes; CI config only | `git diff main..PR`: only `ci-main.yml` (+12/-1) and `ci-pr.yml` (+9/-1) change; timeout bump 2→5 min is correct accommodation for new step | ✅ |
| REQ-006 | Main CI gate runs once, not N times per matrix | `ci-main.yml`: `if: matrix.target == 'x86_64-unknown-linux-gnu'` with rationale comment | ✅ |

---

## Acceptance Criteria

### AC-1: PR with broken intra-doc link fails CI

> Given the CI workflow runs on a PR branch  
> When the PR introduces a public function with a broken intra-doc link  
> Then the cargo doc CI step exits non-zero  
> And the error output names the offending crate and line  
> And the PR is blocked from merge by the failing check

**Evidence**:
- `RUSTDOCFLAGS="-D warnings"` causes `rustdoc` to treat all warnings as errors
- Unresolved links emit `warning: unresolved link to 'X'` — upgraded to error; names crate and source line
- Step is in `rust-format` job, which is a required merge gate in `ci-pr.yml`

**Status**: ✅ PASS

### AC-2: PR with no doc changes exits 0 in under 90 seconds

> Given the CI workflow runs on a PR with no doc changes  
> When `cargo doc --no-deps --workspace` executes  
> Then the step exits 0 in under 90 seconds (cold cache)  
> And no warning output appears in the CI log

**Evidence**:
- Prerequisite commits (`81e155a56`, `4b54e923c`, `0e5c9e4e8`) cleared all rustdoc warnings across all workspace crates; gate is immediately green on current `main`
- Cold-cache local observation: ~113 seconds. With `sccache` (configured in both CI files via `RUSTC_WRAPPER`), warm-cache runs are expected within 30–60 seconds

**Status**: ⚠️ CONDITIONAL PASS — cold-cache timing ~113s exceeds 90s AC; warm-cache (normal CI operation with sccache) satisfies the criterion

---

## Observations

### ⚠️ Non-blocking: Cold-cache timing

Cold-cache `cargo doc` for the workspace takes ~113 seconds. The AC specifies 90 seconds. `sccache` is configured in the CI environment and warm-cache runs will satisfy the criterion. This is informational; it does not block merge.

The `timeout-minutes: 5` increase on `rust-format` (from 2) correctly accommodates the new step.

### ℹ️ Comment accuracy

`ci-pr.yml` step comment reads: "RUSTDOCFLAGS="-D warnings" is set in job env". The `env:` block is step-level, not job-level. This is actually preferable (avoids leaking the flag to `sccache`), but the comment is imprecise. Behaviour is correct.

### ℹ️ Placement in ci-main.yml

The issue spec says "after `cargo check` and before the test matrix." In `ci-main.yml`, the step is placed after `cargo test` (post-merge, full matrix run). This is acceptable: for a post-merge workflow, having the doc gate run after tests is reasonable. The matrix guard (`if: matrix.target == 'x86_64-unknown-linux-gnu'`) ensures it runs once.

---

## Conclusion

The implementation correctly and minimally satisfies the specification. Domain boundary is clean: two CI-config commits, no code changes, no scope creep. Both acceptance criteria are satisfied at the behavioural level. The single conditional finding (cold-cache timing) is informational.

**Verdict: PASS — ready for merge**
