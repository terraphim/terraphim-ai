# Research Document: Native CI Ignore Policy Coverage Gap

**Status**: APPROVED / PHASE 1 COMPLETE
**Author**: Codex
**Date**: 2026-08-14
**Scope**: Phase 1 research only. No design, implementation, exact YAML, commit, push, network, or Docker use authorized.
**Issue**: https://git.terraphim.cloud/terraphim/terraphim-ai/issues/3222

## Executive Summary

The authoritative native CI workflow currently runs `cargo test --workspace --lib --no-fail-fast`, which intentionally limits test execution to workspace library tests. That workspace `--lib` scope excludes integration tests, trybuild UI contract tests, and rustdoc doctest targets for `terraphim_gitea_runner` and `terraphim_llm_runner`.

Root cause: authoritative native CI's blanket workspace `--lib` test command omits deterministic contract targets outside library unit tests. The rustdoc class is companion-only: `terraphim_gitea_runner` has 0 doctests, while `terraphim_llm_runner` has 7 compile-fail rustdoc contracts that are executed by the package-default crate command and missed by `--tests`.

## Essential Questions Check

| Question | Answer | Evidence |
|---|---:|---|
| Does this problem energize us to solve it? | Yes | The gap affects the merge-authoritative CI lane and security policy enforcement tests. |
| Does solving this leverage our capabilities? | Yes | The project already uses Rust crate-scoped tests, trybuild contracts, and native CI policy contracts; the issue is a precise CI coverage omission. |
| Does this meet a significant, validated need? | Yes | Reproduced package-default inventories show `--lib` does not execute deterministic tests that targeted crate inventories contain. |

**Proceed**: Yes for research completion and design-gate review. No implementation is authorized in this document.

## Problem, Impact, Success Criteria

### Problem

`.gitea/workflows/native-ci.yml` is the authoritative native CI lane, but its test execution scope is currently limited to workspace library tests. Several deterministic contract tests live outside that scope in integration, trybuild, and rustdoc doctest targets.

### Impact

- Security policy regressions can merge without executing tests that assert terminal policy rejection, secret-pattern rejection, Docker sandbox public API opacity, closed read-only executable probe variants, and rustdoc-enforced native diagnostics API boundaries.
- Runner reliability regressions can merge without executing tests that cover the register/fetch/execute cycle, polling continuation after policy rejection, workflow contracts, protocol smoke, terminalization, and watchdog behavior.
- `cargo clippy --workspace --all-targets -- -D warnings` compiles non-lib targets for linting, but it does not execute those tests. Treating clippy compilation as test coverage leaves behavioral and compile-fail contracts unverified.

### Success Criteria

- The authoritative native CI lane preserves the known-stable workspace `--lib` test step.
- Deterministic, crate-scoped integration, trybuild, and rustdoc contract tests for the runner and advisory companion are investigated for targeted CI execution.
- The solution class avoids workspace-wide `--all-targets` as the default answer unless future design evidence proves it is safe for runtime budget and failure semantics.
- The live Docker test remains explicitly environment-dependent and ignored unless an environment explicitly opts into it.

## Current-State Map

### Authority Boundary

`docs/decisions/0001-gitea-actions-authoritative-ci.md` establishes that Gitea Actions served by the native `terraphim_gitea_runner` is the authoritative CI for branch protection and merges, while GitHub Actions is release-only. This research relies on that authority decision only; it does not import or design branch-protection context mechanics.

### Authoritative Workflow

Command:

```bash
nl -ba .gitea/workflows/native-ci.yml | sed -n '36,46p'
```

Evidence:

```text
    36	        run: bash -c 'if [ -f ./scripts/check-tinyclaw-test-hermeticity.sh ]; then bash ./scripts/check-tinyclaw-test-hermeticity.sh; else echo preflight-script-absent-on-this-ref-skipping; fi'
    37	      - run: cargo fmt --all -- --check
    38	      - run: cargo clippy --workspace --all-targets -- -D warnings
    39	      - run: cargo build --workspace
    40	      # Deliberate scope: `--lib` only. Widening to `--all-targets` changes both
    41	      # the runtime budget and the failure semantics of the merge gate, and is
    42	      # tracked separately rather than folded into this recovery (Refs #3222).
    43	      - run: cargo test --workspace --lib --no-fail-fast
```

Finding: lines 40-43 deliberately run only `cargo test --workspace --lib --no-fail-fast`.

### Workspace Lib Inventory Gap

Command:

```bash
cargo test --workspace --lib --no-fail-fast -- --list
```

Evidence recorded from the read-only repository/test inventory audit: the command exited 0 but did not inventory these tests:

- `native_ci_workflow_compiles_under_the_default_runner_policy`
- `native_ci_workflow_declares_push_pull_request_and_dispatch_triggers`
- `runner_register_declare_fetch_execute_cycle`
- `policy_rejection_is_terminal_and_polling_continues`
- `strict_docker_sandbox_public_api_is_opaque`
- `rejects_common_unredacted_secret_patterns`
- `probe_variants_are_closed_to_read_only_executables`

Finding: a passing workspace `--lib -- --list` inventory is not sufficient evidence that integration, trybuild, or rustdoc doctest targets are included.

### Package-Default Inventories

Commands:

```bash
cargo test -p terraphim_gitea_runner --no-fail-fast -- --list
cargo test -p terraphim_llm_runner --no-fail-fast -- --list
```

Evidence recorded from reproduced package-default inventories: the package-default inventories contain the missing tests listed above. `terraphim_gitea_runner` has 0 doctests, while `terraphim_llm_runner` has 7 doctests from `crates/terraphim_llm_runner/src/lib.rs:7-51`.

Finding: the fixtures exist and are reachable through package-default crate test commands. They are not deleted, hidden, or unavailable to Cargo.

### Package-Default Test Results

Command:

```bash
cargo test -p terraphim_gitea_runner --no-fail-fast
```

Evidence recorded from reproduced command output:

```text
terraphim_gitea_runner package-default result: 52 unit + 21 integration passed, all passed, no doctests.
```

Finding: the runner package-default command executes the deterministic runner contract tests and has no rustdoc doctest class.

Command:

```bash
cargo test -p terraphim_llm_runner --no-fail-fast
```

Evidence recorded from reproduced command output:

```text
terraphim_llm_runner package-default result: 1 unit passed; 30 non-ignored integration assertions passed; 1 live-Docker integration test ignored; 7 doc compile_fail passed; total 38 passed, 1 ignored.
```

Finding: the advisory companion package-default command adds the 7 rustdoc compile-fail contracts that `--tests` would miss. The missing tests are deterministic under package-default commands except for the explicitly ignored live Docker test.

## Rustdoc Contract Inventory

Anchor: `crates/terraphim_llm_runner/src/lib.rs:7-51`.

| Lines | Expected class | Contract | Protected boundary |
|---:|---|---|---|
| 7-10 | `E0599` | `ProbeResult::status(Probe::CargoMetadataNoDeps, 0, false, false)` must not compile. | Prevents companion callers from fabricating probe results through an exposed status constructor. |
| 12-22 | Unannotated compile failure | `sandbox.execute_probe(evidence, Probe::CargoMetadataNoDeps, ProbeExecutionLimits::default()).await` must not compile. | Prevents invoking a forbidden direct sandbox API that could bypass the validated provenance path. |
| 24-29 | `E0599` | `sandbox.cleanup().await` must not compile. | Prevents companion callers from depending on or invoking hidden Docker lifecycle/backend APIs. |
| 31-34 | `E0599` | `Probe::Shell` must not compile. | Prevents fabricating an arbitrary shell/read-only executable probe variant outside the closed probe set. |
| 36-39 | `E0599` | `Probe::from_command(["cargo", "test"])` must not compile. | Prevents fabricating probes from arbitrary commands and bypassing the approved native diagnostics API. |
| 41-45 | `E0277` | `ProbeResult` must not implement `serde::Deserialize`. | Prevents deserialising or fabricating probe result evidence through untrusted input. |
| 47-51 | `E0277` | `Diagnosis` must not implement `serde::Deserialize`. | Prevents deserialising or fabricating diagnosis/provenance output through untrusted input. |

These rustdoc contracts are part of the companion crate's public API boundary. A public API regression can make one or more compile-fail doctests fail red under `cargo test -p terraphim_llm_runner --no-fail-fast`; `--tests` would miss all 7.

## Ignore Policy and Fixture Visibility

Read-only fixture visibility checks should use exact tracked fixture paths, for example:

```bash
git check-ignore -v crates/terraphim_llm_runner/tests/ui/strict_sandbox_no_host_config.rs
git check-ignore -v crates/terraphim_llm_runner/tests/ui/strict_sandbox_no_host_config.stderr
git check-ignore -v crates/terraphim_llm_runner/tests/ui/strict_sandbox_no_public_profile.rs
git check-ignore -v crates/terraphim_llm_runner/tests/ui/strict_sandbox_no_public_profile.stderr
git check-ignore -v crates/terraphim_llm_runner/tests/ui/strict_sandbox_no_raw_constructor.rs
git check-ignore -v crates/terraphim_llm_runner/tests/ui/strict_sandbox_no_raw_constructor.stderr
cargo test -p terraphim_llm_runner --no-fail-fast -- --list --ignored
```

Evidence recorded from reproduced investigation:

- No `.gitignore` rule hides these fixtures.
- The live Docker ignore is explicit and environment-dependent.
- The Docker test was not executed for this research update.
- The live Docker ignore is not the root defect.

Finding: ignore policy is not suppressing the deterministic tests. The actionable gap is CI test scope.

## Tight Red-Capable Inventory Loop

This loop is research evidence only, not an implementation design:

1. Inventory the authoritative command:

   ```bash
   cargo test --workspace --lib --no-fail-fast -- --list
   ```

   Expected red-capable observation: the command exits 0 while missing the security-critical integration, trybuild, and rustdoc doctest contracts.

2. Inventory package-default runner tests:

   ```bash
   cargo test -p terraphim_gitea_runner --no-fail-fast -- --list
   ```

   Expected observation: runner workflow, protocol, poller, terminalization, and watchdog contract tests are listed. No doctests are listed for this crate.

3. Inventory package-default advisory companion tests:

   ```bash
   cargo test -p terraphim_llm_runner --no-fail-fast -- --list
   ```

   Expected observation: integration, trybuild UI policy assertions, deterministic non-Docker tests, and 7 rustdoc compile-fail contracts are listed.

4. Execute only package-default deterministic sets:

   ```bash
   cargo test -p terraphim_gitea_runner --no-fail-fast
   cargo test -p terraphim_llm_runner --no-fail-fast
   ```

   Expected observation: the package-default sets pass while the environment-dependent Docker test remains ignored.

This loop can turn red if a future policy or public API regression breaks the targeted contracts. `cargo test -p terraphim_llm_runner --tests --no-fail-fast` is insufficient because it would miss the 7 rustdoc compile-fail contracts.

## History

### 6b0017a3

Command:

```bash
git show --no-ext-diff --unified=8 --no-renames 6b0017a3 -- .gitea/workflows/native-ci.yml
```

Evidence:

```text
fix(ci): add integration-test step to native-ci.yml Refs #2396

native-ci.yml ran only --lib tests, which excludes the integration
tests in terraphim_gitea_runner (protocol_smoke.rs: 1 test,
poller_reliability.rs: 2 tests). These tests spin up a real axum stub
server and verify the full register/poll/execute/log/status cycle
without any external dependencies.

Add an explicit step: cargo test -p terraphim_gitea_runner --no-fail-fast
Keeps the existing workspace --lib step unchanged (no regression risk).
```

Finding: `6b0017a3` explicitly added package-default `gitea-runner` integration testing because `--lib` excluded it.

### ae065496

Command:

```bash
git show --no-ext-diff --unified=8 --no-renames ae065496 -- .gitea/workflows/native-ci.yml
```

Evidence:

```text
fix(runner): guarantee terminal native tasks Refs #3222
```

Relevant workflow evidence:

```text
+      # Deliberate scope: `--lib` only. Widening to `--all-targets` changes both
+      # the runtime budget and the failure semantics of the merge gate, and is
+      # tracked separately rather than folded into this recovery (Refs #3222).
       - run: cargo test --workspace --lib --no-fail-fast
```

Finding: `ae065496` reconstructed native CI and omitted the targeted package-default crate test step while explicitly deferring broad scope widening.

### 12e10fba

Command:

```bash
git show --no-ext-diff --unified=8 --no-renames 12e10fba -- .gitea/workflows/native-ci.yml
```

Evidence:

```text
ci(#2112): restore cargo test --lib to native-ci (W2b)
```

Relevant workflow evidence:

```text
-      # NOTE: cargo test deferred (W2b) -- some terraphim-ai --lib tests are
-      # flaky/env-dependent under the runner. Re-add once stabilised.
+      - run: cargo test --workspace --lib --no-fail-fast
```

Finding: `12e10fba` restored workspace `--lib` after earlier flaky/env-dependent tests.

## Ranked Hypotheses and Falsification

| Rank | Hypothesis | Evidence | Status |
|---:|---|---|---|
| 1 | Native CI omits deterministic contract tests because its authoritative test command is workspace `--lib` only. | Workflow lines 40-43 and `--lib -- --list` inventory omission. | Supported |
| 2 | The tests were deleted or no longer exist. | Package-default inventories contain the omitted tests. | Falsified |
| 3 | `.gitignore` hides fixtures so Cargo cannot discover them. | No `.gitignore` rule hides the fixtures; package-default inventories discover them. | Falsified |
| 4 | Clippy `--all-targets` already covers the gap. | Clippy compiles/lints targets but does not execute runtime, trybuild, or rustdoc assertions. | Falsified |
| 5 | The live Docker ignored test is the root defect. | The ignored Docker test is explicit and environment-dependent; deterministic non-Docker tests are also excluded by native CI `--lib`. | Falsified |
| 6 | `cargo test -p terraphim_llm_runner --tests --no-fail-fast` is sufficient for the companion crate. | Package-default result is 38 passed versus the `--tests` result of 31 passed; the delta is exactly 7 doctests from `src/lib.rs:7-51`. | Falsified |
| 7 | Workspace-wide `--all-targets` is the obvious fix. | History explicitly warns widening changes runtime budget and failure semantics; package-default crate commands already prove a narrower path exists. | Unproven and not recommended as first class |

## Provenance

Two independent read-only audits were recorded on 2026-08-14:

| Audit | Purpose | Key evidence |
|---|---|---|
| Repository/test inventory audit | Compared authoritative workspace `--lib` inventory with package-default crate inventories and results. | `terraphim_gitea_runner package-default result: 52 unit + 21 integration passed, all passed, no doctests.` `terraphim_llm_runner package-default result: 1 unit passed; 30 non-ignored integration assertions passed; 1 live-Docker integration test ignored; 7 doc compile_fail passed; total 38 passed, 1 ignored.` |
| CI/policy inspection audit | Inspected `.gitea/workflows/native-ci.yml`, commit history, authority ADR, and tracked fixture visibility. | `cargo test --workspace --lib --no-fail-fast` remains the authoritative workflow test command; `docs/decisions/0001-gitea-actions-authoritative-ci.md` makes Gitea Actions authoritative; no checked fixture path is hidden by `.gitignore`. |

Toolchain and host recorded for the reproduced evidence:

```text
rustc 1.96.1 commit 31fca3adb
cargo 1.96.1
Linux 6.0.12-76060012-generic x86_64
```

## Vital Three Constraints

| Constraint | Why It Is Vital | Evidence |
|---|---|---|
| Preserve workspace `--lib` | This step was restored after flaky/env-dependent concerns and remains the stable baseline. | `12e10fba` restored `cargo test --workspace --lib --no-fail-fast`. |
| Prefer package-default deterministic crate tests for investigation | The missing integration, trybuild, and rustdoc contracts are reachable and passing through crate-scoped package-default commands. | `terraphim_gitea_runner`: 52 unit + 21 integration, no doctests. `terraphim_llm_runner`: 38 passed, 1 ignored, including 7 rustdoc compile-fail contracts. |
| Do not convert Phase 1 into YAML design | The current phase is research-only and should not freeze exact workflow syntax. | User instruction and disciplined-research gate. |

## Risks and Unknowns

| Risk / Unknown | Likelihood | Impact | Research Note |
|---|---:|---:|---|
| Package-default crate test commands may still pick up future environment-dependent tests unless conventions are maintained. | Medium | Medium | Design should define deterministic selection boundaries or labels without encoding them here. |
| CI runtime budget impact on the native runner is not yet measured for the package-default crate commands. | Medium | Medium | Measure both targeted commands on the native runner during Phase 2; do not infer runner budget from local execution. |
| Trybuild and rustdoc output stability may depend on Rust version or diagnostics. | Medium | Medium | Existing compile-fail assertions are valuable but should be watched for toolchain drift. |
| Workflow contract tests may need to track future native CI policy syntax. | Medium | High | This is a feature of the tests, not a reason to omit them. |
| A future design could overcorrect with workspace-wide `--all-targets`. | Medium | High | History shows this changes failure semantics and may reintroduce flaky/env-dependent failures. |

## Multiple Interpretations

1. **Coverage interpretation**: the main problem is that authoritative CI does not execute the same deterministic contract tests developers already rely on locally.
2. **Policy interpretation**: the gap weakens enforcement of native runner command policy and advisory companion sandbox/secret/rustdoc API rules.
3. **Workflow-history interpretation**: earlier commits already recognized the `--lib` exclusion and solved it with a package-default crate step; later reconstruction accidentally regressed that targeted coverage while preserving the deliberate `--lib` baseline.
4. **Ignore-policy interpretation**: the live Docker ignored test is visible because the topic includes ignore policy, but the evidence points away from ignore rules as the root cause.

## Out of Scope

- Editing `.gitea/workflows/native-ci.yml`.
- Designing exact YAML, test matrix syntax, job names, or step order.
- Running network-dependent or Docker-dependent tests.
- Changing `.gitignore`, Cargo manifests, test fixtures, trybuild baselines, or rustdoc contracts.
- Committing, pushing, opening PRs, or updating remote issues.
- Replacing workspace `--lib` with workspace-wide `--all-targets`.

## Recommendation Class Only

Preserve the existing workspace `cargo test --workspace --lib --no-fail-fast` step as the stable baseline. In Phase 2, investigate adding package-default crate test steps for `terraphim_gitea_runner` and `terraphim_llm_runner`, based on the reproduced package-default inventories and passing test executions.

Do not start from workspace-wide `--all-targets`. The history and current workflow comments show that broad widening changes runtime budget and failure semantics. The narrower recommendation class is targeted deterministic contract execution through package-default crate commands, not a frozen YAML implementation.

## Research Gate Checklist

- [x] Status is DRAFT / NO IMPLEMENTATION AUTHORIZED.
- [x] Problem, impact, and success criteria documented.
- [x] Authoritative workflow lines 40-43 recorded with exact command and evidence.
- [x] Workspace `--lib` inventory omission recorded.
- [x] Package-default inventories recorded as containing omitted tests.
- [x] Package-default `terraphim_gitea_runner` test execution result recorded: 52 unit + 21 integration passed, no doctests.
- [x] Package-default `terraphim_llm_runner` test execution result recorded: 1 unit + 30 non-ignored integration assertions + 7 doc compile_fail passed, 1 live-Docker test ignored.
- [x] Rustdoc contract inventory recorded for `crates/terraphim_llm_runner/src/lib.rs:7-51`.
- [x] `.gitignore` and live Docker ignore findings recorded without executing Docker.
- [x] Git history for `6b0017a3`, `ae065496`, and `12e10fba` recorded.
- [x] Root cause stated precisely as CI policy omission of integration, trybuild, and rustdoc targets.
- [x] Ranked hypotheses and falsification included, including `--tests` sufficiency falsified by 38 versus 31 pass delta.
- [x] Provenance recorded for two independent read-only audits on 2026-08-14.
- [x] Vital three constraints included.
- [x] Risks, unknowns, multiple interpretations, and out-of-scope included.
- [x] Recommendation class only; no exact YAML or implementation frozen.
- [x] Human approval received; recorded in
      `docs/plans/decision-native-ci-ignore-policy-research-gate.md`.

## Post-Approval Verification Addendum

Phase 4 retained the package-default execution evidence that the original
research recorded only as prose. See
`docs/plans/verification-native-ci-ignore-policy-3222.md` for command output,
durations, mutation evidence, and traceability. On final local bytes:

- `terraphim_gitea_runner`: 53 library and 21 integration tests passed in 34s;
- `terraphim_llm_runner`: 38 tests passed, the live-Docker test remained
  ignored, and seven rustdoc compile-fail tests passed in 18s;
- the workspace library baseline, fmt, and clippy gates also passed.

The increase from 52 to 53 runner library tests is the new workflow-policy
guard itself. This addendum supplies retained provenance without changing the
Phase 1 root-cause conclusion or recommendation class.
