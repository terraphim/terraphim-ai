# Traceability Report: commit a67f129b1 — fix(tests): replace cargo-run subprocess with assert_cmd timeout

**Validator**: Carthos (Domain Architect)
**Date**: 2026-05-08
**Commit**: a67f129b1aa34eba1b5fb12cc478c9f636bd8dc8
**Closes**: #1353, #1355
**Scope**: `crates/terraphim_cli/tests/integration_tests.rs`, `.github/workflows/ci-main.yml`, `.github/workflows/ci-pr.yml`

---

## Requirements Enumerated

- **REQ-001** (from #1353/#1355): The CLI integration test suite must not hang indefinitely when the terraphim service is absent in CI.
- **REQ-002** (from #1353/#1355): Per-test invocation must not trigger a recompile via `cargo run`; use the pre-built binary.
- **REQ-003** (inferred): Test timeout must be bounded and explicit.
- **REQ-004** (inferred from CI): All crate failures must surface in a single CI run rather than stopping at first failure.

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Status |
|-------:|-------------|------------|----------|-------|--------|
| REQ-001 | No indefinite hang when service absent | Issue #1353/#1355 root-cause | `integration_tests.rs`: `run_cli_json` replaced `StdCommand::new("cargo")` with `cli_command().timeout(Duration::from_secs(30))` | Changed lines ARE the test harness; timeout enforcement is structural | ✅ |
| REQ-002 | Use pre-built binary, not `cargo run` subprocess | Issue #1353/#1355 | `cli_command()` uses `Command::cargo_bin("terraphim-cli")` (assert_cmd) — no recompile per invocation | `test_json_pretty_output`, `test_text_output`, `run_cli_json` call sites (lines ~33, ~695, ~713) | ✅ |
| REQ-003 | Timeout bounded at 30 s per invocation | Commit message | `Duration::from_secs(30)` applied at all three call sites | Timeout is enforced by `assert_cmd::Command::timeout` which panics on expiry | ✅ |
| REQ-004 | All crate failures visible in single CI run | Commit message | `--no-fail-fast` added to `cargo test` in `ci-main.yml:2` and `ci-pr.yml:2` | Structural; verified by diff | ✅ |

---

## Gaps

**None blocking.**

**ℹ️ Note — `assert_cmd::Command::cargo_bin` deprecation warning acknowledged**

The `cli_command()` helper already carried `#[allow(deprecated)]` before this commit. The change does not introduce new deprecation surface. This is a known, accepted posture.

**ℹ️ Note — 30 s timeout is heuristic**

The 30-second ceiling is appropriate for a local CLI invocation with a pre-built binary. If the service initialisation path ever becomes legitimately slow (e.g., large config warm-up), this may produce false timeouts. Tracked in commit message as a known trade-off; not a blocker.

**ℹ️ Note — `--no-fail-fast` CI behaviour change**

Adding `--no-fail-fast` increases CI duration when multiple crates fail, because all failures run to completion before the job exits. This is the intended trade-off: complete failure visibility at the cost of potentially longer failing runs. Acceptable.

---

## Verdict: pass

The implementation is minimal, correctly targeted, and closes the stated issues. Domain boundary is clean: three call sites in one test file plus two CI config lines. No production code changed. Traceability is complete.

---

<sub>Last spec-validated commit: a67f129</sub>
