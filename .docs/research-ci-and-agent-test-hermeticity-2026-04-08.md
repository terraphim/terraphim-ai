# Research Document: CI and Agent Test Hermeticity

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-04-08
**Reviewers**: TBD

## Executive Summary

The original CI failures were real, but they also masked a broader and more durable problem: the `terraphim_agent` CLI test suite is not hermetic. Several tests assume specific offline roles such as `Default` or `Rust Engineer`, while the actual role set is determined at runtime by a mixture of host `settings.toml`, persisted embedded config, and fallback embedded defaults. As a result, CI and local runs can observe different role inventories and fail nondeterministically.

The durable fix is not to keep changing hardcoded role names. It is to define a single test contract for CLI/offline behavior, run those tests inside an isolated settings/data directory, and point them at an explicit fixture config. CI should validate that contract directly, while separate tests can cover fallback embedded behavior intentionally.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | CI is currently blocked and hiding new regressions. |
| Leverages strengths? | Yes | This repo already has good Rust test coverage and explicit config machinery; the gap is test isolation and contract clarity. |
| Meets real need? | Yes | Main-branch CI is red, local runs differ from CI, and the current behavior wastes time during every change. |

**Proceed**: Yes

## Problem Statement

### Description

We need to make CI and `terraphim_agent` CLI tests deterministic, so that:

1. CI failures correspond to real regressions, not host-machine configuration drift.
2. Offline/embedded behavior has a clear contract.
3. Workflows fail for the right reason and expose actionable logs.

### Impact

- `CI Main Branch` is red on `main`.
- Test outcomes depend on the runner's `~/.config/terraphim/settings.toml` and persisted embedded state.
- Developers can see different results locally than CI.
- Future fixes risk becoming more brittle because they patch literals instead of the environment model.

### Success Criteria

1. `terraphim_agent` CLI tests run in isolated temp config/data roots.
2. Offline tests never read the developer or runner's real `~/.config/terraphim` state.
3. Tests that need specific roles use a fixture config or discover valid roles from fixture output.
4. CI passes the build stage and reaches the test stage deterministically.
5. Performance benchmarking workflow parses correctly and fails only on actual benchmark/script issues.

## Current State Analysis

### Existing Implementation

There are now three relevant layers:

1. **CI workflow layer**
   - `ci-main.yml` builds/tests the Rust workspace.
   - `performance-benchmarking.yml` posts reports and commits benchmark updates.

2. **Runtime config loading layer**
   - `TuiService::new()` loads settings from `DeviceSettings::load_from_env_and_file(None)`.
   - If `settings.toml` has `role_config`, the service bootstraps from that JSON file and then prefers persisted embedded config.
   - Otherwise, it falls back to persistence and then embedded defaults.

3. **Test harness layer**
   - `integration_tests.rs` and `offline_mode_tests.rs` invoke `cargo run -p terraphim_agent -- ...` directly.
   - Those helpers do not isolate `HOME`, config path, or `TERRAPHIM_*` environment.
   - Cleanup only removes selected `/tmp` data directories, not the effective settings source under `~/.config/terraphim`.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| CI main workflow | `.github/workflows/ci-main.yml` | Main Rust/frontend/WASM/security CI |
| Benchmark workflow | `.github/workflows/performance-benchmarking.yml` | Benchmark execution, commit, and PR comments |
| CLI service loader | `crates/terraphim_agent/src/service.rs` | Chooses settings, role config, persistence, embedded defaults |
| CLI config command | `crates/terraphim_agent/src/main.rs` | Applies `config set selected_role` |
| Embedded defaults builder | `crates/terraphim_config/src/lib.rs` | Defines fallback embedded roles |
| Offline CLI tests | `crates/terraphim_agent/tests/offline_mode_tests.rs` | CLI behavior tests in default/offline mode |
| Integration CLI tests | `crates/terraphim_agent/tests/integration_tests.rs` | End-to-end CLI and server/offline comparison tests |
| Server default role fixture | `terraphim_server/default/default_role_config.json` | Single-role server/default fixture |
| Server engineer fixture | `terraphim_server/default/terraphim_engineer_config.json` | Multi-role fixture including `Default`, `Terraphim Engineer`, others |

### Data Flow

Current offline CLI test flow:

`cargo test` -> test helper runs `cargo run -p terraphim_agent -- ...` -> `TuiService::new()` -> read settings from default config path -> possibly read `role_config` from host settings -> possibly load persisted embedded config -> CLI command validates requested role against whichever config won

This means test inputs are not fully defined inside the test file.

### Integration Points

- Filesystem config: `~/.config/terraphim/settings.toml`
- Persisted embedded config via configured storage profiles
- JSON role config fixtures under `terraphim_server/default/`
- GitHub Actions workflow execution environment with `CI=true`

## Constraints

### Technical Constraints

- `fff-search` panics in CI unless `zlob` is explicitly enabled; `ci-main.yml` must keep `--features zlob`.
- CLI tests currently spawn child processes, so in-process test setup is not sufficient by itself; environment must be propagated to subprocesses.
- `DeviceSettings::load_from_env_and_file(None)` uses the default config path unless environment overrides are supplied.
- Embedded defaults and JSON fixtures intentionally differ today, so tests must choose whether they are validating embedded fallback or explicit fixture config.

### Business Constraints

- Main branch CI should be restored with the smallest durable change set.
- Fixes must avoid dependence on developer-local config because the repo is used by multiple contributors and agents.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| CI determinism | Same result on fresh runner and local clean env | Fails due to host config leakage |
| Test isolation | No read dependency on `~/.config/terraphim` | Violated |
| Failure observability | Failing job exposes actionable message | Improved for workflows, still weak for test contract |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Hermetic test environment | Without this, tests continue to vary by machine state | Offline and integration tests currently read host settings/persistence |
| Single explicit offline role contract | Without this, role assertions remain brittle | Tests currently mix `Default`, `Rust Engineer`, and live host-selected roles |
| CI must exercise real build + real tests | Without this, broken tests stay hidden behind earlier failures | zlob panic previously masked role test failures |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Redesigning the entire settings system | Not needed to make tests deterministic |
| Adding new user-facing role management features | Not required for CI restoration |
| Refactoring all CLI tests across the repo in one pass | Only the unstable suites need immediate hermeticization |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_settings` | Determines where settings are loaded from | High |
| `terraphim_config` | Defines embedded defaults and persistence behavior | High |
| `terraphim_agent` CLI | Enforces role existence at `config set` time | High |
| workflow YAML | Must continue to enable zlob and benchmark steps | Medium |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| GitHub Actions runner env | Hosted/self-hosted runtime behavior | Medium | None; design around it |
| `actions/github-script` | No bundled `glob` module | Low | Shell `find`, already used |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Tests still leak host state through inherited env | High | High | Add a shared hermetic subprocess helper with explicit env and temp dirs |
| Future tests reintroduce hardcoded role assumptions | High | Medium | Centralize role fixture helper and test utilities |
| Embedded defaults diverge from fixture contract over time | Medium | Medium | Separate fixture-based tests from embedded-fallback tests |
| Benchmark workflow still blocked by script bug | High | Medium | Fix `scripts/run-performance-benchmarks.sh` separately after workflow parse issue |

### Open Questions

1. Should offline CLI tests validate **fixture-driven offline mode** or **pure embedded fallback mode** by default?
2. Do we want one canonical test fixture for CLI/offline behavior, or distinct fixtures for single-role and multi-role cases?
3. Should `ci-main.yml` keep running the full workspace test suite, or temporarily scope known-bad suites while hermetic fixes land?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Offline child-process tests inherit host settings today | Local failures and CI logs show `/home/alex/.config/terraphim/embedded_config.json` | Proposed hermetic helper might be incomplete | Yes |
| The failing role-name assertions are symptoms, not the root cause | Multiple suites fail for the same reason and different literals | Could miss a separate role-validation bug | Yes |
| Keeping `zlob` enabled in CI is required permanently unless upstream build.rs changes | Verified panic behavior in CI and successful build with feature enabled | CI could regress if feature removed later | Yes |
| `performance-benchmarking.yml` now parses, but benchmark script remains broken | YAML issue fixed; issue #772 filed for shell syntax error | Workflow may still fail later for other reasons | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Just replace invalid role names with `Terraphim Engineer` | Fast but brittle; hides environment leak | Rejected as final fix |
| Make tests dynamically parse available roles from current environment | More resilient than literals but still accepts host-state dependence | Rejected as final fix |
| Isolate tests and use explicit fixture config | Deterministic and debuggable | Chosen |

## Research Findings

### Key Insights

1. The zlob and YAML fixes were necessary, but they were only the first layer.
2. The deeper issue is test hermeticity, not merely invalid literals.
3. `offline_mode_tests.rs` is also affected, not just `integration_tests.rs`.
4. The repo already has enough machinery to isolate tests; it is just not being used in these CLI subprocess helpers.
5. There are two legitimate runtime contracts in play: embedded defaults and explicit role-config bootstrap. Tests need to choose one deliberately.

### Relevant Prior Art

- `terraphim_server/default/default_role_config.json`: simple deterministic fixture
- `terraphim_server/default/terraphim_engineer_config.json`: richer deterministic fixture
- `ConfigBuilder::build_default_embedded()`: fallback embedded contract already defined in code

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Hermetic CLI test harness spike | Prove we can run child CLI commands with temp settings/data roots and fixed fixture config | 1-2 hours |
| Embedded-vs-fixture test split spike | Decide whether to split suites or parameterize helper | 1 hour |

## Recommendations

### Proceed/No-Proceed

Proceed. The work is well-scoped, high-value, and the evidence is sufficient to move to design.

### Scope Recommendations

1. Keep the zlob and benchmark YAML fixes already merged.
2. Fix `terraphim_agent` CLI test determinism as the next primary change.
3. Treat benchmark shell script repair as a separate follow-up because it is a different failure class.

### Risk Mitigation Recommendations

1. Introduce a shared test helper that:
   - creates a temp config root
   - creates a temp data root
   - writes a test `settings.toml` or env overrides
   - points `role_config` at a fixture JSON file
   - launches `cargo run` with only the intended `TERRAPHIM_*` variables
2. Rewrite role-sensitive tests to use fixture roles from that helper, not ambient roles.
3. Add at least one explicit embedded-fallback test that runs with no `role_config` to preserve that behavior intentionally.
4. Run targeted suites in CI and locally before changing workflow breadth.

## Next Steps

If approved:

1. Produce a design plan for hermetic CLI test infrastructure and workflow validation.
2. Implement the hermetic helper and migrate `offline_mode_tests.rs` and `integration_tests.rs` first.
3. Re-run `cargo test -p terraphim_agent --test offline_mode_tests` and `cargo test -p terraphim_agent --test integration_tests`.
4. Re-run `ci-main.yml` equivalent locally.
5. Separately fix issue #772 in `scripts/run-performance-benchmarks.sh`.

## Appendix

### Reference Materials

- `.github/workflows/ci-main.yml`
- `.github/workflows/performance-benchmarking.yml`
- `crates/terraphim_agent/src/service.rs`
- `crates/terraphim_agent/src/main.rs`
- `crates/terraphim_agent/tests/offline_mode_tests.rs`
- `crates/terraphim_agent/tests/integration_tests.rs`
- `crates/terraphim_config/src/lib.rs`

### Code Snippets

Current loader precedence in `crates/terraphim_agent/src/service.rs`:

- if `settings.toml` provides `role_config`, bootstrap from that JSON and then prefer persisted config
- else use persisted embedded config
- else use `build_default_embedded()`

This is correct runtime behavior, but unsafe for subprocess tests unless the test controls the settings/data roots.
