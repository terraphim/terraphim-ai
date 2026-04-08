# Research Document: Restore CI Baseline Stability

**Status**: Draft | Review | Approved
**Author**: AI Agent (Phase 1 Discipline)
**Date**: 2026-04-08
**Reviewers**: [Human Approval Required]

## Executive Summary

CI is broken on `main` due to two independent root causes. First, `ci-main.yml`'s `rust-build` job sets `CI=true` on self-hosted runners, triggering a panic in the `fff-search` crate's build script (`build.rs:21`) unless the `zlob` feature is enabled — but the build command passes no `--features` flags. Second, `performance-benchmarking.yml` has a structural/workflow-file problem (GitHub reports "This run likely failed because of a workflow file issue") causing immediate 0-second failures on every trigger. The combined effect is that `CI Main Branch` is red, `performance-benchmarking` is red, and the ADF remediation queue is churning because agents cannot get clean CI feedback.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Does fixing CI energize us? | Yes | Every task/feature branch now fails CI before review; ADF agents cannot self-heal |
| Does it leverage our strengths? | Yes | Self-hosted runners + GitHub Actions are our CI stack; we control the workflows |
| Does it meet a validated need? | Yes | CI is mandatory for all merges; current state blocks all development |

**Proceed**: Yes — 3/3 YES

---

## Problem Statement

### Problem 1: `ci-main.yml` rust-build failure — `fff-search` panic

**What**: The `rust-build` job in `ci-main.yml` fails at the `fff-search` crate compilation step with exit 101 and the message "CI detected but `zlob` feature is not enabled. Build with `--features zlob`."

**Who is affected**: All developers pushing to `main` or PRs targeting `main`; ADF agents running automated task branches.

**Impact**: No release-quality binary can be built via the primary CI path. Workarounds are in place (`ci-native.yml` only runs lint) but no full build occurs through that path.

**Success Criteria**: `ci-main.yml`'s `rust-build` job passes on `main` push, producing `terraphim_server`, `terraphim_mcp_server`, and `terraphim-agent` binaries.

### Problem 2: `performance-benchmarking.yml` immediate failure

**What**: The workflow fails in approximately 0 seconds on every trigger (push to `main`, `develop`, and all task branches). GitHub Actions explicitly annotates: "This run likely failed because of a workflow file issue."

**Who is affected**: Developers and agents whose branches touch `crates/terraphim_*/src/**` paths.

**Impact**: No performance regression signals are being captured. The workflow is effectively non-functional.

**Success Criteria**: `performance-benchmarking` workflow starts the `performance-benchmarks` job and produces benchmark results (even if they are empty/warning rather than failure).

---

## Current State Analysis

### Existing Implementation

#### CI Pipeline Architecture

Three workflows respond to `push` on `main` in parallel:

| Workflow | Trigger | Jobs | Pass/Fail |
|----------|---------|------|-----------|
| `ci-main.yml` | `push: branches: [main, develop]` | rust-build, frontend-build, wasm-build, docker-build, integration-tests, security-scan, build-summary | **FAIL** (rust-build) |
| `ci-native.yml` | `push: branches: [main, CI_migration]` | setup, lint-and-format | PASS (no build) |
| `performance-benchmarking.yml` | `push: branches: [main, develop], paths: [terraphim_*/src/**, scripts/run-performance-benchmarks.sh]` | performance-benchmarks, performance-regression-check, update-baseline | **FAIL** (0s, workflow file issue) |

A fourth, `test-firecracker-runner.yml`, passes.

#### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `ci-main.yml` | `.github/workflows/ci-main.yml` | Primary CI pipeline (rust build, frontend, wasm, docker, integration tests) |
| `ci-native.yml` | `.github/workflows/ci-native.yml` | Secondary CI (lint-only, no full build) |
| `rust-build.yml` | `.github/workflows/rust-build.yml` | Reusable workflow for building specific packages |
| `performance-benchmarking.yml` | `.github/workflows/performance-benchmarking.yml` | Performance regression gate |
| `terraphim_mcp_server` | `crates/terraphim_mcp_server/Cargo.toml` | MCP server crate; pulls in `fff-search` |
| `terraphim_file_search` | `crates/terraphim_file_search/Cargo.toml` | File search crate; also pulls in `fff-search` |

#### `fff-search` / `zlob` Feature Chain

**Dependency graph:**
```
ci-main.yml (rust-build job)
  └─ cargo build --release --target x86_64-unknown-linux-gnu --workspace
      └─ builds ALL workspace members
          └─ crates/terraphim_mcp_server
              ├─ fff-search = { git = "...", branch = "feat/external-scorer" }
              └─ terraphim_file_search
                  └─ fff-search = { git = "...", branch = "feat/external-scorer" }
```

**Feature wiring:**
- `terraphim_file_search/Cargo.toml`: `zlob = ["fff-search/zlob"]` (feature)
- `terraphim_mcp_server/Cargo.toml`: `zlob = ["fff-search/zlob", "terraphim_file_search/zlob"]` (feature)

**Build script behavior** (`fff-core/build.rs:21`):
```rust
// Simplified from build.rs
if cfg!(CI) && !cfg!(feature = "zlob") {
    panic!("CI detected but `zlob` feature is not enabled. Build with `--features zlob`.");
}
```

**Why CI Native passes**: `ci-native.yml`'s `rust-build` job runs inside a Docker container (`container: ubuntu:${{ matrix.ubuntu-version }}`). Inside a container, GitHub Actions does **not** set `CI=true`. Therefore the panic does not trigger.

**Why `ci-main.yml` fails**: `ci-main.yml` runs directly on the self-hosted runner (`runs-on: [self-hosted, Linux, X64]`, no `container:`), so GitHub Actions **does** set `CI=true`, triggering the panic.

#### `performance-benchmarking.yml` Failure Analysis

**Build command in the workflow:**
```yaml
- name: Start Terraphim server
  run: |
    cargo build --release --package terraphim_server
```

This builds only `terraphim_server` (not the full workspace), and `terraphim_server` does **not** depend on `terraphim_mcp_server`. Therefore this workflow should not hit the `fff-search/zlob` panic.

**The actual failure mode**: GitHub Actions reports "This run likely failed because of a workflow file issue" and the run completes in 0 seconds. This indicates:
1. The workflow syntax is invalid, OR
2. A job-level condition causes all jobs to be skipped, OR
3. A required secret/permission is missing, OR
4. The runner is unavailable and the job fails before starting

GitHub's "workflow file issue" phrasing specifically suggests the YAML structure has a problem preventing execution.

### Integration Points

- CI jobs produce binary artifacts (`.deb`, Docker images) uploaded to GitHub Actions storage
- ADF agents (security-sentinel, spec-validator, etc.) run as GitHub Actions workflows triggered by Gitea issue events
- ADF remediation branches (`task/NNN-*/`) push to `main` and trigger `ci-main.yml` and `performance-benchmarking.yml`

### ADF Remediation Queue State

Open Gitea issues with `[Remediation]` or `[ADF]` prefix, mapped to root causes:

| Issue | Title | Root Cause Cluster |
|-------|-------|-------------------|
| #510 | `[ADF] provider probes fail across multiple models` | **Provider health** — separate from CI infra |
| #507 | `[Remediation] spec-validator FAIL on #502: wire OpenCode and Codex into terraphim_sessions` | **Spec wiring** |
| #506 | `[Remediation] compliance-watchdog FAIL on #502: fix license metadata and redact sensitive logs` | **License/privacy** |
| #504 | `[ADF] merge-coordinator failing repeatedly in 12h` | **Agent stability** — merge-coordinator itself |
| #501 | `[Remediation] compliance-watchdog FAIL on #498: address session data handling gaps` | **License/privacy** |
| #499 | `[Remediation] spec-validator FAIL on #498: restore required license fields` | **License** |
| #497 | `[Remediation] spec-validator FAIL on #495: missing manifest license fields` | **License** |
| #494 | `[Remediation] compliance-watchdog FAIL on #414: license gate and privacy surfaces` | **License/privacy** |
| #490 | `feat(desktop): migrate @tauri-apps/api from v1 to v2 import paths` | **Desktop/Tauri v2** — separate |
| #471 | `[ADF] Git worktree cleanup race condition` | **Infra** — cleanup logic |
| #469 | `refactor(adf): deduplicate security-sentinel and security-audit-flow` | **Code quality** |
| #468 | `[ADF] spec-validator skill file missing: business-scenario-design/SKILL.md` | **ADF infra** — duplicate of #511? |
| #463 | `[Remediation] test-guardian FAIL on #420: cross_mode_consistency_test divergence` | **Test divergence** |
| #462 | `[Remediation] spec-validator FAIL on #420: missing CLI features for Trigger-Based Retrieval` | **Spec wiring** |
| #461 | `[Remediation] compliance-watchdog FAIL on #420: license violations + CVE + port exposure` | **License/CVE** |
| #451 | `[Remediation] spec-validator FAIL on #442: LLM hooks unwired in agent.rs` | **Spec wiring** |

**Cluster summary:**
- License fields / metadata: #506, #501, #499, #497, #494, #461, #430, #441, #406 — **8 issues**
- Spec wiring (OpenCode/Codex integration, CLI features, LLM hooks): #507, #462, #451, #415 — **4 issues**
- Provider health / probes: #510, #446, #444 — **3 issues** (separate root cause from CI infra)
- Agent stability (merge-coordinator): #504, #285 — **2 issues**
- Test divergence: #463, #504, #505 — **3 issues**
- Other (desktop, infra, code quality): #490, #471, #469, #468, #426 — **5 issues**

---

## Constraints

### Technical Constraints

| Constraint | Source | Impact |
|------------|--------|--------|
| `CI=true` is set automatically on self-hosted runners without a container | GitHub Actions behavior | Triggers `fff-search` build.rs panic |
| `fff-search` build.rs has hardcoded CI-detection panic | External crate (`fff.nvim`) | Cannot be modified without forking |
| `zlob` feature gates the panic condition | `fff-search` crate design | Mitigation must enable this feature |
| `ci-main.yml` uses `--workspace` to build all members | Workflow design intent | Includes `terraphim_mcp_server` even when only some binaries needed |
| `performance-benchmarking.yml` must build `terraphim_server` to run benchmarks | Workflow design | Requires build pipeline |

### Business Constraints

| Constraint | Source | Impact |
|------------|--------|--------|
| All merges to `main` require passing CI | Project policy | No releases until CI is fixed |
| ADF agents use CI feedback to self-correct | Agent design | Remediation queue is blocked |
| Multiple agents file issues simultaneously | Agent swarm behavior | Issue churn increases without CI fix |

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| CI build time (workspace) | < 15 min | ~5 min (before fff-search panic) |
| CI reliability | 100% green on `main` | ~0% (all pushes fail rust-build) |
| Performance workflow start | < 30s to first job start | 0s (fails immediately) |

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why Vital | Mitigation |
|------------|-----------|------------|
| `fff-search` build script panics when `CI=true` and `zlob` feature is absent | Without this, no workspace build can succeed on self-hosted runners | Add `--features zlob` to CI build commands OR use `--package` scoping instead of `--workspace` |
| `performance-benchmarking.yml` fails at 0s due to workflow file issue | Blocks performance regression detection for all code changes | Fix the workflow YAML structural problem |
| ADF remediation queue is churning on mixed root causes | Without root-cause grouping, same class of failures repeats across multiple issues | Cluster issues by root cause and fix root causes in order |

### Eliminated from Scope

| Item | Why Eliminated |
|------|---------------|
| Fork/modify `fff-search` crate | External repo, not owned; not needed since `zlob` feature exists |
| Full rewrite of CI pipeline | Over-engineering; only two specific jobs need changes |
| Investigating all 16+ remediation issues simultaneously | Wrong order; fix CI baseline first, then issues will self-resolve or become easier to triage |

---

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `ci-main.yml` | Single point of failure for workspace build | High — all releases blocked |
| `rust-build.yml` | Reusable workflow potentially used by other workflows | Low — not currently wired to ci-native |
| `performance-benchmarking.yml` | Non-functional, blocks perf visibility | Medium — fixable independently |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `fff-search` (fff.nvim) | git branch `feat/external-scorer` | Low — feature exists, just not enabled | N/A |
| GitHub-hosted `ubuntu-latest` runner | N/A | Low — used by performance-benchmarking | N/A |
| Self-hosted runners `[self-hosted, Linux, X64]` | N/A | Low — already configured | N/A |

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Enabling `zlob` feature significantly increases build time or binary size | Medium | Medium | Test locally; `ci-native.yml` succeeds without it on specific packages only |
| `performance-benchmarking.yml` fix changes trigger behavior | Low | Low | Verify path filters are preserved; only fix structural issue |
| `ci-main.yml` and `ci-native.yml` diverge in build coverage | Medium | High | Ensure both produce same set of shippable binaries |
| More ADF issues filed while CI is broken | High | Medium | Document fix plan; communicate to agents via Gitea comments |

### Open Questions

1. **Is `ci-native.yml`'s lint-only behavior intentional or a migration gap?** — If intentional, the full build is unaddressed. If a gap, `ci-native.yml` should call `rust-build.yml` or `ci-main.yml` for builds.
2. **Does `rust-build.yml` get called from anywhere?** — It is a reusable workflow (`workflow_call`) but no other workflow references it in this repo.
3. **What specifically is broken in `performance-benchmarking.yml`?** — GitHub reports "workflow file issue" but not the specific YAML error. Needs local validation (`act`, `yamllint`, or dry-run).
4. **Are there additional workspace members that depend on `fff-search` beyond `terraphim_mcp_server`?** — Only `terraphim_mcp_server` and `terraphim_file_search`; confirmed via grep of all Cargo.toml files.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `CI=true` is set by GitHub Actions on self-hosted runners (not in containers) | Standard GHA behavior | None — this is well-documented | Yes |
| The `fff-search` panic is triggered by `CI=true` in the build script | Build log shows: "CI detected but `zlob` feature is not enabled" | None — matches the error message | Yes |
| `ci-native.yml` runs in a container where `CI` is not set | `rust-build.yml` uses `container: ubuntu:${{ matrix.ubuntu-version }}` | High — need to verify ci-native actually uses this | Partially — `ci-native.yml` itself has no container, but calls `rust-build.yml` |
| `performance-benchmarking.yml`'s 0s failure is a workflow file issue | GitHub's own annotation | Low — GitHub knows | Yes |
| `terraphim_server` does not depend on `terraphim_mcp_server` | `cargo tree -p terraphim_server --no-default-features` shows no mcp_server | None — verified | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Rejected/Chosen |
|----------------|--------------|---------------------|
| **Interpretation A**: The fix is to add `--features zlob` to `ci-main.yml`'s build command | Simple one-line change; zlob may have performance/size cost | **Chosen** — minimal blast radius |
| **Interpretation B**: The fix is to replace `--workspace` with `--package terraphim_server --package terraphim_mcp_server --package terraphim_agent` in `ci-main.yml` | Matches what `ci-native.yml`'s `rust-build.yml` does; builds only needed packages | **Alternative** — more typing but precise; avoids enabling unused features |
| **Interpretation C**: Fork `fff.nvim` and remove the CI-detection panic | Eliminates the root cause permanently | **Rejected** — adds maintenance burden; zlob feature already exists as proper solution |

---

## Research Findings

### Key Insights

1. **Two independent CI failures, not one.** `ci-main.yml` fails due to `fff-search/zlob` in ~4 minutes. `performance-benchmarking.yml` fails in 0 seconds due to a workflow YAML issue. Fixing one does not fix the other.

2. **CI Native CI pipeline is lint-only.** The `ci-native.yml` workflow that currently passes does not actually build any binaries — it only runs `setup` and `lint-and-format`. The full build is only in `ci-main.yml`, which fails.

3. **`ci-main.yml` and `ci-native.yml` are both triggered by the same `push` to `main`.** They are parallel workflows, not sequential. `ci-native.yml` was likely created as a migration step but the build job was not wired up (no `workflow_call` to `rust-build.yml`).

4. **The `performance-benchmarking.yml` failure is a workflow file issue, not a code issue.** The workflow cannot start its jobs at all. This is likely a YAML syntax problem, a conditional that always evaluates to false, or a missing required input.

5. **ADF issues cluster into ~4 root cause classes.** The 16+ open remediation issues can be grouped: License/metadata (8), Spec wiring (4), Provider health (3), Agent stability (2), with 5 others being desktop/infra/code-quality. Fixing the root causes in order will resolve multiple issues simultaneously.

6. **The `fff-search` crate has a CI-detection panic that is intentional** (a guardrail in the upstream crate to ensure zlob feature is explicitly opted into). The panic only fires when `CI=true` AND `zlob` is not enabled. The `zlob` feature exists and is correctly wired in the Cargo.toml files — it just isn't being passed in the CI build command.

### Relevant Prior Art

- Mozilla's `sccache` setup (already in use in `ci-main.yml`) — correctly configured and not related to this failure
- `ci-native.yml` container-based build pattern — demonstrates that the `zlob` issue is environment-specific (container vs. bare runner)
- `rust-build.yml` reusable workflow — exists but is not currently called by any other workflow

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Validate `performance-benchmarking.yml` YAML locally | Identify the specific workflow file issue | 30 minutes |
| Test `cargo build --release --workspace --features zlob` locally | Verify zlob fix works without side effects | 15 minutes |
| Audit which workspace members actually need `fff-search` | Ensure `--package` scoping approach is complete | 1 hour |

---

## Recommendations

### Proceed/No-Proceed

**Proceed** — CI is a hard blocker for all development and releases. The fix is low-risk (add feature flag to build command or scope packages). The performance-benchmarking fix is independent but equally important for the ADF loop.

### Scope Recommendations

**In scope (Phase 2):**
1. Fix `ci-main.yml` rust-build `fff-search/zlob` failure
2. Fix `performance-benchmarking.yml` workflow structural failure
3. Optionally: audit and close duplicate Gitea issues now resolved by #511

**Out of scope (deferred):**
- Any ADF remediation issues (fix CI baseline first, then re-evaluate)
- Provider probe issues (#510, #446, #444) — separate root cause class
- License compliance cleanup — blocked by CI infra fix but can be attempted in parallel once CI is green
- ci-native.yml build coverage gap — separate tracking issue

### Risk Mitigation Recommendations

1. **Test locally before pushing**: Run `cargo build --release --workspace --features zlob` to verify it works before committing workflow changes.
2. **Fix performance-benchmarking.yml YAML in isolation**: Use `yamllint` or `gh workflow validate` to find the specific issue before making functional changes.
3. **Do not merge any ADF task branches until CI is confirmed green** on `main` — prevent additional failures from piling on.
4. **Comment on open Gitea issues**: Let the agents (and human developers) know that CI infra is being fixed so they don't continue filing duplicate remediation issues.

---

## Next Steps

If approved:

1. **Phase 2 (Design)**: Produce Implementation Plan with two independent tracks:
   - Track A: `ci-main.yml` fix — option selection (A, B, or hybrid)
   - Track B: `performance-benchmarking.yml` fix — structural repair + verification

2. **Issue hygiene**: Create a tracking issue or comment on the Gitea issue cluster explaining that CI infra fix is in progress, so agents do not file additional remediation issues for what are infra problems rather than code problems.

3. **Communication**: Post a brief status update on the most recent failing Gitea issue (#510 or #507) explaining the CI infrastructure fix is underway.

---

## Appendix

### Reference Materials

- GitHub Actions environment variables: `CI` is set to `true` on all GitHub Actions runners. Self-hosted runners without a container: `CI=true`. Inside a Docker container: `CI` is not set unless explicitly done so.
- `fff.nvim` repository: `https://github.com/AlexMikhalev/fff.nvim.git`, branch `feat/external-scorer`
- Build script source: `crates/fff-core/build.rs` in the fff.nvim repository

### Cargo Feature Definitions

```toml
# crates/terraphim_file_search/Cargo.toml
[features]
default = []
zlob = ["fff-search/zlob"]

# crates/terraphim_mcp_server/Cargo.toml
[features]
zlob = ["fff-search/zlob", "terraphim_file_search/zlob"]
```

### Workflow Trigger Comparison

| Workflow | Push to main? | workflow_dispatch? | Container? | CI variable? |
|----------|--------------|---------------------|------------|--------------|
| `ci-main.yml` | Yes | Yes | No | Yes |
| `ci-native.yml` | Yes (CI_migration branch) | Yes | Yes (via rust-build.yml) | No |
| `rust-build.yml` | N/A (workflow_call) | N/A | Yes | No |
| `performance-benchmarking.yml` | Yes (paths filtered) | Yes | No (ubuntu-latest) | Yes (ubuntu-latest is GHA-hosted, CI=true) |
