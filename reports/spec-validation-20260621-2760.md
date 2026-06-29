# Spec Validation Report: Issue #2760

**Date**: 2026-06-21 11:17 CEST
**Validator**: Carthos (spec-validator)
**Issue**: fix(config): terraphim_lsp and terraphim_validation use hardcoded version 0.1.0 instead of version.workspace
**Verdict**: PASS

---

## Summary

Issue #2760 reported that two workspace crates declared a hardcoded `version = "0.1.0"` instead of inheriting the workspace version via `version.workspace = true`. The workspace version is `1.20.5`.

---

## Acceptance Criteria Verification

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `crates/terraphim_lsp/Cargo.toml` uses `version.workspace = true` | PASS | `git show gitea/main:crates/terraphim_lsp/Cargo.toml` line 3: `version.workspace = true` |
| `crates/terraphim_validation/Cargo.toml` uses `version.workspace = true` | PASS | `git show gitea/main:crates/terraphim_validation/Cargo.toml` line 3: `version.workspace = true` |
| `cargo metadata` returns `1.20.5` for both | PASS | Verified by test-guardian 2026-06-21T03:15 (comment on issue) |
| `cargo check --workspace` passes | PASS | Verified by test-guardian 2026-06-21T03:15 (comment on issue) |

---

## PR Lineage

Multiple PRs were created for this issue. Final resolution:

| PR | State | Merged | Notes |
|----|-------|--------|-------|
| #2769 | closed | No | Duplicate — closed this session |
| #2786 | closed | No | Superseded |
| #2809 | closed | No | Superseded |
| #2837 | **closed** | **Yes** | Canonical fix — merged to Gitea main `c93c65e5b` |

---

## Traceability

| Req ID | Requirement | Impl Ref | Evidence | Status |
|--------|-------------|----------|----------|--------|
| REQ-001 | `terraphim_lsp` uses `version.workspace = true` | `crates/terraphim_lsp/Cargo.toml:3` (gitea/main) | git show gitea/main | ✅ |
| REQ-002 | `terraphim_validation` uses `version.workspace = true` | `crates/terraphim_validation/Cargo.toml:3` (gitea/main) | git show gitea/main | ✅ |
| REQ-003 | Both crates report workspace version at runtime | PR #2837 commit `903b59f29` | test-guardian verdict PASS | ✅ |
| REQ-004 | `cargo check --workspace` passes | Workspace build | test-guardian verdict PASS | ✅ |

---

## Observations

1. **Fix confirmed on Gitea main** (`c93c65e5b`), not yet synchronised to GitHub origin (`c22ed90f6`). This is expected — the dual-remote topology has Gitea as the merge target, with GitHub as a periodic sync destination.

2. **Duplicate PR cleanup**: PR #2769 was left open inadvertently after #2837 was merged. Closed in this session.

3. **Issue lifecycle**: Issue #2760 was closed at 2026-06-21T11:13:56 (before the final compound-review GO at 11:15), suggesting the close was triggered by the merge automation. This is correct behaviour.

---

## Verdict: PASS

All acceptance criteria are met on Gitea main. The issue is correctly closed.
