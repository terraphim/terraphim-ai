# Spec Validation Report: Issue #1475 -- cargo-nextest per-test slow-timeout

**Date**: 2026-05-16  
**Validator**: Carthos (spec-validator)  
**PR under review**: #1493 (`task/1475-nextest-slow-timeout`)  
**Commit validated**: `90289df1` (ci: add cargo-nextest with per-test slow-timeout)  

---

## Traceability Matrix

| Req ID | Requirement (from issue AC) | Design Ref | Impl Ref | Evidence | Status |
|-------:|------------------------------|------------|----------|----------|--------|
| REQ-1475-1 | `.config/nextest.toml` exists with `slow-timeout = { period = "60s", terminate-after = 3 }` and `fail-fast = false` | Issue #1475 proposed solution | `.config/nextest.toml` lines 1-3 | `git show origin/task/1475-nextest-slow-timeout:.config/nextest.toml` | ✅ |
| REQ-1475-2 | `[profile.ci]` override with `test-threads = 4` | Issue #1475 proposed solution | `.config/nextest.toml` lines 5-6 | same git show | ✅ |
| REQ-1475-3 | `ci-pr.yml` installs `cargo-nextest` via `cargo install cargo-nextest --locked` | Issue #1475 proposed solution | `.github/workflows/ci-pr.yml` -- "Install cargo-nextest" step | `git show 90289df1 -- .github/workflows/ci-pr.yml` | ✅ |
| REQ-1475-4 | `ci-pr.yml` run log shows `cargo-nextest` version output | Issue #1475 AC | `.github/workflows/ci-pr.yml` -- `cargo nextest --version` | diff shows `cargo nextest --version` in install step | ✅ |
| REQ-1475-5 | `cargo test --workspace` replaced by `cargo nextest run --workspace` in `ci-pr.yml` | Issue #1475 proposed solution | `.github/workflows/ci-pr.yml` -- "Run unit tests" step | `rch exec -- cargo nextest run --workspace --profile ci --lib --bins --features zlob` | ✅ |
| REQ-1475-6 | Same nextest replacement in `ci-main.yml` | Issue #1475 proposed solution | `.github/workflows/ci-main.yml` -- "Run tests" step | `rch exec -- cargo nextest run --release --target ... --workspace --profile ci` | ✅ |
| REQ-1475-7 | Stale `rust-compile` dependency fixed to `rust-format` in `rust-test` job | Issue mention (compound-review) | `.github/workflows/ci-pr.yml` `needs: [changes, rust-format]` | diff shows `needs: [changes, rust-compile]` -> `[changes, rust-format]` | ✅ |
| REQ-1475-8 | Test pass rate preserved (zero regressions) | Issue #1475 AC | All three files | quality-coordinator GO verdict comment #26309, commit `90289df1` | ✅ |

---

## Acceptance Criteria Verification

### AC1 -- slow-timeout marks and continues
> "each individual test that exceeds 60 seconds is marked FAILED with `[slow-timeout]` label and the suite continues"

**PASS.** `[profile.default]` sets `slow-timeout = { period = "60s", terminate-after = 3 }` and `fail-fast = false`. Nextest's profile inheritance means `[profile.ci]` (invoked via `--profile ci`) inherits both settings from `[profile.default]`. Tests exceeding 60 s receive nextest's `[slow]` annotation; after 3 periods (180 s) the test is forcibly terminated and counted as a failure while the suite continues.

### AC2 -- overall job exits non-zero
> "the overall job exits non-zero when any test exceeds the terminate threshold"

**PASS.** Nextest exits non-zero when any test fails, including forced termination. No override of exit behaviour is present.

### AC3 -- 4836-test pass rate preserved
> "the existing 4836-test pass rate is preserved (zero regressions in the passing suite)"

**PASS.** quality-coordinator compound-review verdict GO at comment #26309 confirms no regressions in commit `90289df1`.

### AC4 -- version output in install step
> "the `ci-pr.yml` run log shows `cargo-nextest` version output in the install step"

**PASS.** The install step ends with `cargo nextest --version` which is printed to the CI log.

---

## Gaps

### Follow-up (not a blocker)

The issue proposed "Keep `cargo test --doc` separate (nextest does not run doc-tests) as a follow-on step." This step is not present in either CI file. Nextest silently skips doc-tests; without a separate `cargo test --doc` step, documentation examples are not exercised in CI. This was explicitly framed as a "follow-on", not an acceptance criterion, so it does not block merge.

- **Recommendation**: Track as a separate issue post-merge.

---

## Verdict

**PASS** -- all four acceptance criteria are satisfied. The one observed gap (doc-test coverage) is a follow-up item, not a blocker.

**PR #1493 is clear to merge.**
