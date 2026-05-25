# Spec Validation Report: Issue #1311 — cargo-nextest Workspace Adoption

**Date**: 2026-05-24
**Branch**: `task/1311-cargo-nextest-workspace`
**Validator**: Carthos (spec-validator)
**Verdict**: PASS (with follow-up notes)

---

## Requirements Enumerated

Derived directly from issue #1311 body:

| Req ID | Requirement |
|--------|-------------|
| REQ-1 | `.config/nextest.toml` exists with `fail-fast = false` and `slow-timeout = { period = "60s", terminate-after = 3 }` at `[profile.default]` |
| REQ-2 | CI workflows replace `cargo test --workspace` with `cargo nextest run --workspace` |
| REQ-3 | `Makefile` provides `make test` as local nextest entrypoint for consistency |
| REQ-4 | Override for `test(selected_role_tests)` with `slow-timeout = { period = "10s", terminate-after = 1 }` (issue #1305 hotspot) |

Acceptance criteria (Gherkin from issue body):

| AC ID | Criterion |
|-------|-----------|
| AC-1 | `selected_role_tests` terminates within 15s with TIMEOUT result when LLM proxy unreachable |
| AC-2 | Exit code non-zero; CI fails fast rather than stalling |
| AC-3 | All suites not depending on external services complete successfully in same run |
| AC-4 | Total wall-clock time does not exceed `cargo test` baseline by more than 20% |

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Evidence | Status |
|--------|-------------|------------|----------|----------|--------|
| REQ-1 | nextest.toml with default profile timeouts | issue #1311 | `.config/nextest.toml` lines 1-3 | File present; `fail-fast = false`, `slow-timeout = { period = "60s", terminate-after = 3 }` confirmed | ✅ |
| REQ-2 | CI workflows use nextest | issue #1311 | `.github/workflows/ci-pr.yml`, `.github/workflows/rust-build.yml`, `.github/workflows/ci-firecracker.yml` | Three workflows updated; installation guards added in each; `--profile ci` used | ✅ |
| REQ-3 | Makefile `make test` target | issue #1311 | `Makefile` (untracked) | `make test` → `cargo nextest run --workspace`; `make test-ci` → `cargo nextest run --workspace --profile ci` | ⚠️ UNTRACKED |
| REQ-4 | Override for `test(selected_role_tests)` | issue #1311 | `.config/nextest.toml` lines 8-10 | `filter = 'test(selected_role_tests)'`, `slow-timeout = { period = "10s", terminate-after = 1 }` confirmed | ✅ |
| AC-1 | 15s termination for selected_role_tests | REQ-4 | `.config/nextest.toml` override | 10s period × 1 terminate = SIGTERM at 10s; test killed before 15s threshold | ✅ |
| AC-2 | Non-zero exit code | nextest semantics | All CI workflow steps | nextest exits non-zero on any test failure or timeout by design | ✅ |
| AC-3 | Non-external suites complete | REQ-2 | All CI files | `--profile ci` uses 4 threads; `fail-fast = false` preserves all-suite runs | ✅ |
| AC-4 | ≤20% wall-clock overhead | performance budget | Not measurable statically | Must be verified by CI run — nextest's process-per-test has modest overhead; parallel execution typically recovers it | ℹ️ RUNTIME |

---

## Additional Scope

Changes outside the explicit spec that are present on the branch:

| File | Change | Assessment |
|------|--------|------------|
| `scripts/ci-check-tests.sh` | Unit/integration tests → nextest; doc tests remain on `cargo test` (nextest does not support doctests) | Correct — nextest has no doctest support; comment explains the split |
| `scripts/hooks/pre-commit` | `cargo test --workspace --lib` → `cargo nextest run --workspace --lib` | Consistent with spec intent |
| `.pre-commit-config.yaml` | `cargo-test` hook entry updated to nextest | Consistent |
| `crates/terraphim_spawner/src/config.rs` | Clippy idiom fix (`match … => true/false` → `!matches!`) | Unrelated cleanup; no spec impact |

---

## Gaps

### Follow-up Notes (not blockers)

**NOTE-1 — Makefile not committed** (⚠️)
The `Makefile` is listed as untracked (`?? Makefile` in `git status`). REQ-3 is satisfied in file content but the file will not be included in the PR unless staged. Action: `git add Makefile` before creating PR.

**NOTE-2 — No nextest installation guard in `scripts/hooks/pre-commit`** (⚠️)
`ci-check-tests.sh` installs nextest if absent. The pre-commit hook does not — if a developer does not have nextest installed locally, the hook fails with `command not found: cargo-nextest`. Recommended: add the same installation guard or a clear error message directing the developer to install nextest. Not a blocker for CI.

**NOTE-3 — `scripts/ci-check-tests.sh` does not pass `--profile ci`** (ℹ️)
Unit and integration test runs in `ci-check-tests.sh` use the default profile (60s slow-timeout, unbounded threads) rather than `--profile ci` (4 threads). Inconsistency is minor; the timeout policy still applies. Not a spec violation.

**NOTE-4 — AC-4 performance budget unverifiable statically** (ℹ️)
The ≤20% wall-clock requirement can only be validated by a before/after CI run comparison. Baseline measurement should be recorded in the PR description or a CI comment to allow future regression detection.

---

## Overall Verdict

**PASS**

All four specified requirements (REQ-1 through REQ-4) are implemented and verifiable from the diff. All Gherkin acceptance criteria are addressed by the implementation. The Makefile staging issue (NOTE-1) must be resolved before merging but does not constitute a spec violation — the content is correct. Follow-up items NOTE-2 through NOTE-4 are quality improvements for subsequent issues.
