# Research Document: PR #2971 Merge Block

**Status**: Draft
**Author**: OpenCode (Terraphim AI agent)
**Date**: 2026-06-25
**Reviewers**: [Pending human review]
**Linked PR**: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/2971
**Linked Issue**: #2492

## Executive Summary

PR #2971 is a one-line documentation change that adds a `// SAFETY:` comment above an `unsafe { std::env::set_var(...) }` call in a tinyclaw test. All ADF review gates (`adf/pr-reviewer`, `adf/validation`, `adf/verification`) passed, yet the PR cannot merge because the required `native-ci / build (push)` status check is failing. Investigation of the CI logs shows the failure is **not caused by PR #2971**. The native CI build fails during `cargo clippy --workspace --all-targets -- -D warnings` on pre-existing lint errors in `terraphim_merge_coordinator`, `terraphim_server`, and `terraphim_validation`. Until these lint errors are fixed, every PR that branches from the current `main` will be blocked from merging.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Unblocks PR #2971 and the wider merge pipeline; restores trust in the native CI gate. |
| Leverages strengths? | Yes | We already own the CI decision (see `docs/decisions/0001-gitea-actions-authoritative-ci.md`) and the code in question. |
| Meets real need? | Yes | Branch protection requires a green `native-ci / build (push)`; currently no PR can merge. |

**Proceed**: Yes (3/3 YES).

## Problem Statement

### Description
PR #2971 cannot be merged because Gitea branch protection reports `native-ci / build (push)` as failed. The failure is unrelated to the PR's change and is caused by four pre-existing Clippy lint errors that are treated as errors (`-D warnings`) by the native CI workflow.

### Impact
- PR #2971 is stranded despite having no defects and passing all ADF gates.
- Any other PR based on the current `main` will encounter the same merge block.
- Contributors may waste time debugging their own changes when the CI failure is global.

### Success Criteria
1. `cargo clippy --workspace --all-targets -- -D warnings` passes locally.
2. A new native CI run for PR #2971 (or its rebased branch) reports `native-ci / build (push)` as success.
3. PR #2971 can be merged without bypassing branch protection.

## Current State Analysis

### PR #2971 Details
| Attribute | Value |
|-----------|-------|
| Title | Fix #2492: add SAFETY comment to unsafe set_var in tinyclaw test |
| Head SHA | `d27027b62981b314eead86315b8cad3e238c76c0` |
| Branch | `task/2492-tinyclaw-safety-comment-echo-pr2` |
| Changed files | 1 (`crates/terraphim_tinyclaw/src/config.rs`) |
| Additions / deletions | +3 / 0 |
| ADF `pr-reviewer` | pass (5/5) |
| ADF `validation` | pass (5/5) |
| ADF `verification` | pass (4/5) |
| `native-ci / build (push)` | **failure** |

### Native CI Run for the PR
- **Run API ID**: 20166
- **Run number**: 18426
- **Workflow**: `native-ci.yml@refs/heads/task/2492-tinyclaw-safety-comment-echo-pr2`
- **Event**: `push`
- **Duration**: 9 seconds (2026-06-25T17:57:52Z → 2026-06-25T17:58:01Z)
- **Job name**: `build`
- **Runner**: `terraphim-native-af86f9e4-5c4c-422f-ab7b-84a85caf5f11`
- **Conclusion**: failure

### CI Step Results
| Step | Conclusion |
|------|------------|
| `cargo fmt --all -- --check` | success |
| `cargo clippy --workspace --all-targets -- -D warnings` | **failure** |
| `cargo build --workspace` | failure (propagated) |
| `cargo test --workspace --lib --no-fail-fast` | failure (propagated) |
| `cargo test -p terraphim_gitea_runner --no-fail-fast` | failure (propagated) |

### Actual Clippy Errors
The Clippy step fails on four errors in three crates, none of which were modified by PR #2971:

1. **`crates/terraphim_merge_coordinator/src/gitea.rs:215`**
   `clippy::assertions_on_constants`
   `assert!(OPEN_PRS_LIMIT > 50, "...")` where `OPEN_PRS_LIMIT` is a `const u32 = 300`.

2. **`crates/terraphim_merge_coordinator/src/gitea.rs:223`**
   `clippy::assertions_on_constants`
   `assert!(OPEN_PRS_LIMIT <= 300, "...")` where `OPEN_PRS_LIMIT` is a `const u32 = 300`.

3. **`terraphim_server/src/lib.rs:633`**
   `clippy::items_after_test_module`
   `mod tests { ... }` is followed by `pub async fn build_router_for_tests()` at line 864.

4. **`crates/terraphim_validation/src/lib.rs:65`**
   `clippy::assertions_on_constants`
   `assert!(true); // Basic creation test`

### Branch Protection Rule
The `main` branch protection rule (last updated 2026-06-22) requires:
- `native-ci / build (push)`
- `adf/pr-reviewer`
- `adf/validation`
- `adf/verification`

Only `native-ci / build (push)` is red for PR #2971.

### Runner Observations
The job log contains a warning from `rch::hook`:

```
WARN rch::hook: Project path normalization failed for /home/alex/.local/share/terraphim-gitea-runner/work-2/terraphim/terraphim-ai: input resolves outside canonical root (input: ..., detail: resolved=... root=/data/projects)
INFO rch::hook: Selected worker: bigbox-local at alex@127.0.0.1 (4 slots, speed 50.0)
WARN rch::hook: Remote execution failed: ..., running locally
```

This warning did not block the build; the runner fell back to local execution and proceeded to run `cargo clippy`. It is therefore informational and out of scope for the immediate fix.

## Constraints

### Technical Constraints
- The native CI workflow (`native-ci.yml`) is not stored in this repo's `.gitea/workflows/` directory; it appears to be served by the Gitea instance or a shared workflow path. Changes to the workflow itself are therefore not part of this fix.
- Clippy is invoked with `-D warnings`, so any warning is fatal.
- The fix must not change runtime behaviour.

### Business Constraints
- Merges are gated on Gitea, not GitHub (see `docs/decisions/0001-gitea-actions-authoritative-ci.md`).
- The fix must land on `main` before PR #2971 can merge; we cannot simply override the status check.

### Non-Functional Requirements
| Requirement | Target |
|-------------|--------|
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| CI run time | unchanged (lint step should remain sub-minute) |

## Vital Few (Essentialism)

### Essential Constraints
1. **Do not bypass branch protection.** The fix must make the CI green, not force-merge.
2. **Do not change runtime behaviour.** The failing code is in tests or const validation; replacements must be semantically equivalent.
3. **Fix the root cause on `main`.** Patching only PR #2971 would leave every other PR blocked.

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Re-writing the `native-ci.yml` workflow | The workflow file is not in this repo and the workflow is functioning as intended; it correctly surfaced real lint errors. |
| Addressing the `rch::hook` path-normalisation warning | It is a warning, not an error, and the runner fell back successfully. Tackle separately if it becomes blocking. |
| Switching required checks back to `adf/build` | The authoritative CI decision mandates `native-ci / build (push)`. |
| Running `cargo fix --clippy` across the whole workspace | Too broad and risky; only the four known errors are blocking merges. |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_merge_coordinator` | Two const-assert tests need updating. | Low — const validation semantics must be preserved. |
| `terraphim_server` | Test helper must be moved before `mod tests`. | Low — pure reordering. |
| `terraphim_validation` | Vacuous `assert!(true)` must be removed or replaced. | Low — test still validates `ValidationSystem::new()`. |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `static_assertions` crate | n/a (already available via std `const { assert!(...) }`) | Low | Use `const { assert!(...) }` blocks (Rust 1.57+). |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| More lint errors appear once the first four are fixed | Medium | Medium | Run the exact CI Clippy command locally before pushing. |
| The fix branch also fails CI for unrelated reasons | Low | Medium | Rebase on latest `main` and run full lint/build/test locally. |
| `build_router_for_tests` is used by integration tests in other crates | Low | High | Compile and run workspace tests locally after moving it. |

### Open Questions
1. Does the `native-ci.yml` workflow run on every push, or does it need a manual re-trigger after the lint fixes land on `main`?
   *Answer: it runs on push; PR #2971 will need a rebase or fresh push to pick up the green `main` base.*
2. Are there other failing branches that should be rebased after this fix?
   *Answer: yes — all recent PRs observed in the action-run list were failing with the same pattern.*

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| The four Clippy errors are the only blockers. | CI log excerpt ends with these four errors and no further compilation. | If more errors exist, a second fix cycle is needed. | Partially — local Clippy run in design/implementation will verify. |
| `OPEN_PRS_LIMIT` will remain a `const` near its call sites. | It is declared `const OPEN_PRS_LIMIT: u32 = 300;` at module level. | If it becomes a runtime value, compile-time asserts would be wrong. | Yes, inspected file. |
| `build_router_for_tests` has no callers inside `mod tests`. | It is defined after the module and is `pub`, suggesting external use. | If tests inside the module call it, moving it above is still fine because it is `pub` and in scope. | Will verify by compiling. |

### Multiple Interpretations Considered
| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| A. PR #2971 itself introduced the lint errors. | Would require changing the tinyclaw SAFETY comment. | Rejected — logs show errors in unrelated files. |
| B. The native CI workflow is misconfigured and should ignore warnings. | Would weaken quality gates. | Rejected — the workflow correctly enforces `-D warnings`; the errors are real. |
| C. The codebase has pre-existing lint debt that must be paid on `main`. | Requires a small, targeted fix on `main`, then rebasing PR #2971. | Chosen — it unblocks all PRs and preserves CI authority. |

## Research Findings

### Key Insights
1. **The merge block is systemic, not PR-specific.** Every recent action run inspected (run numbers 18408–18427) concluded with `failure` and very short durations, indicating the same fast-failing Clippy step.
2. **ADF gates and native CI are independent.** ADF passed because it reviews the diff; native CI validates the whole workspace including pre-existing issues on `main`.
3. **The fix is small and low-risk.** Only four lint errors in three files need correction, all in test or const-validation code.
4. **The `rch::hook` warning is a red herring.** It logs at `WARN` but the runner falls back to local execution and proceeds.

### Relevant Prior Art
- `docs/decisions/0001-gitea-actions-authoritative-ci.md` — establishes Gitea Actions as the authoritative CI and `native-ci / build (push)` as the merge gate.
- Recent CI run logs for runs 18408–18427 — show the same failure pattern across unrelated branches.

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Local Clippy reproduction | Confirm the four errors and detect any additional ones. | 5 minutes |
| Verify const-assert replacement | Ensure `const { assert!(...) }` compiles and fails compilation if the invariant is violated. | 10 minutes |

## Recommendations

### Proceed/No-Proceed
**Proceed.** Fix the four Clippy errors on `main`, then rebase PR #2971.

### Scope Recommendations
- Limit the fix to the four identified lint errors.
- Do not refactor unrelated code.
- Do not change the CI workflow.

### Risk Mitigation Recommendations
- Run the exact CI Clippy command locally before committing.
- Run `cargo fmt --all -- --check` and `cargo build --workspace` as a sanity check.
- After merging the fix to `main`, rebase PR #2971 and confirm the native CI run turns green.

## Next Steps

If this research document is approved:
1. Create an implementation plan (Phase 2) with exact file changes and verification steps.
2. Implement the Clippy fixes on a new branch.
3. Open a PR for the fixes.
4. After merge, rebase PR #2971 onto the updated `main`.
5. Confirm PR #2971's `native-ci / build (push)` turns green and merge it.

## Appendix

### Reference Materials
- PR #2971: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/2971
- CI run #18426 (API ID 20166): https://git.terraphim.cloud/terraphim/terraphim-ai/actions/runs/18426
- Branch protection API response for `main`
- `docs/decisions/0001-gitea-actions-authoritative-ci.md`

### Code Snippets

`crates/terraphim_merge_coordinator/src/gitea.rs`:
```rust
const OPEN_PRS_LIMIT: u32 = 300;

#[test]
fn open_prs_limit_exceeds_gitea_default_of_50() {
    assert!(
        OPEN_PRS_LIMIT > 50,
        "OPEN_PRS_LIMIT must exceed 50 so PRs beyond position 50 are not silently dropped"
    );
}

#[test]
fn open_prs_limit_within_gitea_max_page_size() {
    assert!(
        OPEN_PRS_LIMIT <= 300,
        "Gitea max page size is 300; limit must not exceed it"
    );
}
```

`terraphim_server/src/lib.rs`:
```rust
#[cfg(test)]
mod tests {
    // ... many tests ...
}

/// Constructs a minimal Axum router suitable for integration tests.
pub async fn build_router_for_tests() -> Router {
    // ...
}
```

`crates/terraphim_validation/src/lib.rs`:
```rust
#[tokio::test]
async fn test_validation_system_creation() {
    let system = ValidationSystem::new().unwrap();
    assert!(true); // Basic creation test
}
```
