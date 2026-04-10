# Implementation Plan: CI and Agent Test Hermeticity

**Status**: Draft
**Research Doc**: `.docs/research-ci-and-agent-test-hermeticity-2026-04-08.md`
**Author**: OpenCode
**Date**: 2026-04-08
**Estimated Effort**: 1-2 days

## Overview

### Summary

This plan makes `terraphim_agent` CLI tests deterministic by isolating them from host settings and persisted state, while preserving the already-completed workflow fixes for `zlob` and benchmark YAML parsing. It also defines a clean separation between fixture-driven CLI tests and embedded-fallback behavior tests.

### Approach

Use a shared subprocess test harness for CLI tests that always runs with:

- temp config root
- temp data root
- explicit `role_config` fixture when role-sensitive behavior is under test
- explicit cleanup of persisted state created by that harness

Then migrate the unstable suites onto that harness and leave a small targeted set of tests to validate embedded fallback behavior separately.

### Scope

**In Scope:**
- Shared hermetic CLI subprocess helper for `terraphim_agent` tests
- Migration of `offline_mode_tests.rs`
- Migration of `integration_tests.rs`
- Clear split between fixture-based and embedded-fallback assertions
- Verification of `ci-main.yml` behavior after test fixes
- Separate planned repair for benchmark shell script issue #772

**Out of Scope:**
- Re-architecting runtime settings loading
- Full rewrite of all `terraphim_agent` tests
- New user-facing configuration features
- Expanding workflow matrix beyond current coverage

**Avoid At All Cost**:
- More hardcoded role-name substitutions without fixing environment control
- Changing production config-loading semantics just to satisfy tests
- Adding more permissive assertions that hide invalid setup

## Architecture

### Component Diagram

```text
Rust test fn
  -> shared test harness
    -> temp config dir + temp data dir + fixture selection
      -> child `cargo run -p terraphim_agent -- ...`
        -> TuiService::new()
          -> controlled settings.toml/env
            -> controlled role_config fixture or embedded fallback
```

### Data Flow

```text
Test case
-> build TestEnv
-> launch CLI subprocess with env overrides
-> CLI loads deterministic settings
-> CLI validates against deterministic role set
-> test asserts command result
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Add a shared CLI test harness | Centralizes isolation logic and prevents drift between suites | Per-test ad hoc env setup |
| Use explicit fixture config for role-sensitive tests | Deterministic role inventory and readable assertions | Ambient role discovery from host state |
| Keep a separate embedded-fallback test path | Preserves the true fallback contract without contaminating fixture tests | Mixing fallback and fixture assertions in same suite |
| Keep `--features zlob` in CI workflow | Required by current dependency behavior in CI | Removing feature and reintroducing panic |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Replace all role names with `Terraphim Engineer` | Fixes symptom only | Brittle future failures |
| Dynamically parse whatever roles happen to exist on host | Still non-hermetic | CI/local drift remains |
| Rewrite `TuiService` loading order | Too invasive for the problem | Production regressions |

### Simplicity Check

The simplest correct design is to change the tests, not the production loader. Runtime config precedence is useful in real use. The issue is that subprocess tests currently do not control their environment. A small shared helper plus fixture-based assertions solves the problem with minimum blast radius.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_agent/tests/support/cli_test_env.rs` | Shared helper for hermetic CLI subprocess execution |
| `crates/terraphim_agent/tests/fixtures/*.toml` or generated settings in helper | Deterministic settings inputs if needed |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_agent/tests/offline_mode_tests.rs` | Replace direct `cargo run` helpers with hermetic helper; remove hardcoded ambient assumptions |
| `crates/terraphim_agent/tests/integration_tests.rs` | Same migration; use fixture roles explicitly |
| `crates/terraphim_agent/tests/server_mode_tests.rs` | Optional alignment if it also uses subprocess helpers without isolation |
| `.github/workflows/ci-main.yml` | No new functional change expected; verify current zlob fix remains |
| `scripts/run-performance-benchmarks.sh` | Separate patch to fix issue #772 |

### Deleted Files

| File | Reason |
|------|--------|
| None required | N/A |

## API Design

### Public Test Support Types

```rust
struct CliTestEnv {
    config_dir: tempfile::TempDir,
    data_dir: tempfile::TempDir,
    role_config_path: Option<PathBuf>,
}

enum RoleMode {
    EmbeddedFallback,
    Fixture(PathBuf),
}
```

### Public Test Support Functions

```rust
fn make_cli_test_env(role_mode: RoleMode) -> Result<CliTestEnv>;

fn run_offline_command_in_env(
    env: &CliTestEnv,
    args: &[&str],
) -> Result<(String, String, i32)>;

fn run_server_command_in_env(
    env: &CliTestEnv,
    server_url: &str,
    args: &[&str],
) -> Result<(String, String, i32)>;
```

### Error Strategy

- Test helpers return `anyhow::Result<_>`
- Fail fast if fixture paths are missing
- Emit the effective settings path and role fixture in assertion messages

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_cli_env_writes_isolated_settings` | new support module tests | Confirms helper isolates settings/data roots |
| `test_fixture_mode_exposes_expected_roles` | support module tests or offline suite | Confirms fixture role inventory |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| Updated `offline_mode_tests` | existing file | Validate offline CLI using deterministic fixture roles |
| Updated `integration_tests` | existing file | Validate end-to-end offline/server flows using deterministic env |
| New embedded fallback smoke test | offline or dedicated test file | Validate no-`role_config` fallback behavior intentionally |

### Verification Commands

```bash
cargo test -p terraphim_agent --test offline_mode_tests -- --test-threads=1
cargo test -p terraphim_agent --test integration_tests -- --test-threads=1
cargo test -p terraphim_agent --test server_mode_tests -- --test-threads=1
cargo test --release --target x86_64-unknown-linux-gnu --workspace --features "self_update/signatures,zlob"
bash -n scripts/run-performance-benchmarks.sh
```

## Implementation Steps

### Step 1: Build Hermetic CLI Test Harness
**Files:** `crates/terraphim_agent/tests/support/cli_test_env.rs` and call sites
**Description:** Create one shared helper that launches child commands with explicit `TERRAPHIM_*` env and temp directories.
**Tests:** Helper-focused tests or first migrated suite
**Estimated:** 3 hours

Key requirements:
- temp config root
- temp data root
- fixture or embedded mode toggle
- no inherited dependency on `~/.config/terraphim`

### Step 2: Migrate `offline_mode_tests.rs`
**Files:** `crates/terraphim_agent/tests/offline_mode_tests.rs`
**Description:** Replace direct `cargo run` helper with hermetic helper; update assertions to fixture-backed roles.
**Tests:** `cargo test -p terraphim_agent --test offline_mode_tests -- --test-threads=1`
**Dependencies:** Step 1
**Estimated:** 2 hours

Specific expectations:
- role-sensitive tests use fixture roles intentionally
- embedded fallback tests are separated and named clearly

### Step 3: Migrate `integration_tests.rs`
**Files:** `crates/terraphim_agent/tests/integration_tests.rs`
**Description:** Use the same hermetic helper for offline and server subprocesses; remove hardcoded ambient assumptions.
**Tests:** `cargo test -p terraphim_agent --test integration_tests -- --test-threads=1`
**Dependencies:** Step 1
**Estimated:** 3 hours

Specific expectations:
- `test_end_to_end_offline_workflow` uses a fixture role that is guaranteed to exist
- `test_role_consistency_across_commands` uses fixture role names, not host-dependent names
- `test_full_feature_matrix` uses the same fixture contract for all `--role` calls

### Step 4: Validate Adjacent Suites
**Files:** `crates/terraphim_agent/tests/server_mode_tests.rs` and any shared helpers
**Description:** Check whether server-mode tests also need the hermetic helper; align if needed.
**Tests:** targeted server suite
**Dependencies:** Step 1
**Estimated:** 1-2 hours

### Step 5: Verify CI Contract
**Files:** `.github/workflows/ci-main.yml`
**Description:** Re-run the local command equivalent and ensure the workflow assumptions still match the repo.
**Tests:** full Rust release build/test command with `zlob`
**Dependencies:** Steps 2-4
**Estimated:** 1 hour

### Step 6: Fix Benchmark Script Separately
**Files:** `scripts/run-performance-benchmarks.sh`
**Description:** Repair the shell syntax error from issue #772 so the benchmark workflow can execute after parsing correctly.
**Tests:** `bash -n` and targeted benchmark dry-run if feasible
**Dependencies:** none for test hermeticity, but needed for full CI recovery
**Estimated:** 1 hour

## Rollback Plan

If the harness migration causes unexpected regressions:

1. Keep the zlob and workflow parsing fixes in place.
2. Revert only the test harness migration commit.
3. Temporarily pin unstable tests behind `#[ignore]` only with explicit issue references if required for emergency unblocking.

## Migration

### Test Migration Strategy

1. Introduce helper without changing all tests at once.
2. Migrate the known failing suites first.
3. If successful, optionally move other subprocess-based CLI suites onto the same helper.

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| None expected | N/A | Existing `tempfile` may already be available transitively; add explicitly only if needed for test support clarity |

### Dependency Updates

| Crate | From | To | Reason |
|-------|------|----|--------|
| None planned | N/A | Not needed for this fix |

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Test startup overhead | Small increase acceptable | Compare current vs hermetic CLI suite runtime |
| CI determinism | High priority over raw speed | Green/red stability across runs |

### Benchmarks to Add

No new benchmarks required. Determinism is the priority.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Choose canonical fixture for offline CLI tests | Pending | Human review |
| Decide whether `server_mode_tests.rs` must migrate in same change | Pending | Implementer |
| Confirm issue #772 is included in same PR or follow-up PR | Pending | Human review |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Scope for fixture vs embedded fallback agreed
- [ ] Human approval received
