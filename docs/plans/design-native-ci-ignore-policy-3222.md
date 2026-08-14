# Implementation Plan: Native CI Ignore-Policy Coverage Correction

**Status**: APPROVED / PHASE 3 IMPLEMENTED — PHASE 4 VERIFICATION IN PROGRESS
**Research Doc**: `docs/plans/research-native-ci-ignore-policy-3222.md`
**Research Approval SHA-256**: `3dd5f2b069e34fd5a361f204d520c11405b734513c84fe98a230a21e21b9429b`
**Current Research SHA-256**: `d44a4a29c93a9591d47deacb5ba424bab05188f43ab5f5fe7ae5a74579beacf6`
**Research Gate**: `docs/plans/decision-native-ci-ignore-policy-research-gate.md`
**Date**: 2026-08-14
**Issue**: `#3222`
**Estimated Effort**: 30-45 minutes plus native-runner PR timing

## Overview

Restore deterministic crate-level CI coverage that the authoritative native CI
lost when it retained only the workspace library baseline: preserve
`cargo test --workspace --lib --no-fail-fast`; add exactly one package-default
targeted test command each for `terraphim_gitea_runner` and
`terraphim_llm_runner`; add one RED-capable `#[cfg(test)]` library unit test in
`crates/terraphim_gitea_runner/src/lib.rs`.

No source APIs, dependencies, workflow contexts, branch-protection settings,
Docker/network execution, ignored-test execution, commits, or pushes are
authorized by this design.

## Scope

**In Scope:** exactly two implementation files: `.gitea/workflows/native-ci.yml`
and `crates/terraphim_gitea_runner/src/lib.rs`.

**Out of Scope:** integration-test-file edits, source APIs, production
dependencies, Cargo manifests, any implementation file beyond the two above,
`--tests`, `--all-targets`, `--ignored`, Docker, network,
branch-protection/status context mutation, commit, push, or deployment.

**Avoid At All Cost:** replacing the workspace `--lib` baseline,
workspace-wide `cargo test --workspace --all-targets`, `cargo test -p
terraphim_llm_runner --tests`, ignored-test execution, a new ignore scanner,
and new workflow/job/status contexts.

## Architecture

The workflow keeps the workspace lib baseline, then runs package-default
`gitea_runner` and `llm_runner` commands. The policy guard is one library unit
test compiled into `terraphim_gitea_runner`, so the existing baseline
`cargo test --workspace --lib --no-fail-fast` executes it immediately in the
same PR before the targeted steps run.

Key decisions: use `crates/terraphim_gitea_runner/src/lib.rs`, not an
integration test file; parse the authoritative workflow with the existing
`terraphim_github_runner::parse_single_workflow_yaml`; load YAML with
`include_str!("../../../.gitea/workflows/native-ci.yml")`; assert exact counts
for the baseline and both targeted commands; reject forbidden cargo-test flags
across the whole step list; add no production APIs, dependencies, status
contexts, or scanners. Simplicity check: the correction is a small YAML
amendment plus one local library unit test.

## File Changes

Modify only `.gitea/workflows/native-ci.yml` (amend stale test-scope comment
and add two `run:` lines) and `crates/terraphim_gitea_runner/src/lib.rs` (add
one `#[cfg(test)]` unit test, approximately 30-45 lines). This design phase
creates no implementation changes.

## Exact Test Design

Add exactly this bare crate-root test in `crates/terraphim_gitea_runner/src/lib.rs`.
It must not be placed in an enclosing test module, so its exact libtest name stays
`native_ci_workflow_declares_targeted_package_default_policy_tests`:

```rust
#[cfg(test)]
#[test]
fn native_ci_workflow_declares_targeted_package_default_policy_tests() {
    let workflow_yaml = include_str!("../../../.gitea/workflows/native-ci.yml");
    let wf = terraphim_github_runner::parse_single_workflow_yaml(workflow_yaml)
        .expect("native-ci.yml must parse");
    let commands: Vec<&str> = wf.steps.iter().map(|s| s.command.as_str()).collect();

    let count = |needle: &str| commands.iter().copied().filter(|c| *c == needle).count();

    assert_eq!(count("cargo test --workspace --lib --no-fail-fast"), 1);
    assert_eq!(count("cargo test -p terraphim_gitea_runner --no-fail-fast"), 1);
    assert_eq!(count("cargo test -p terraphim_llm_runner --no-fail-fast"), 1);

    for command in &commands {
        for line in command.lines() {
            let line = crate::policy::strip_env_assignments(line);
            let mut tokens = line.split_whitespace();
            if tokens.next() == Some("cargo") && tokens.next() == Some("test") {
                let remaining: Vec<&str> = tokens.collect();
                for forbidden in ["--tests", "--all-targets", "--ignored"] {
                    assert!(
                        !remaining.contains(&forbidden),
                        "native CI cargo-test step must not include {forbidden}: {line}"
                    );
                }
            }
        }
    }
}
```

The test intentionally avoids tautological exact-string-derived checks. Its
load-bearing assertions are exact command counts plus a whole-step-list,
line-wise scan for forbidden cargo-test tokens. Reusing
`crate::policy::strip_env_assignments` mirrors production classification for
env-prefixed commands; scanning `command.lines()` also covers YAML block scalars.

Negative mutations that must fail: removing either targeted command, mutating
either targeted command away from package-default form, duplicating the baseline
or either targeted command, or adding `--tests`, `--all-targets`, or `--ignored`
to any `cargo test` step.

## Exact Workflow Amendment

Keep existing format/build order unchanged. Replace the stale YAML comment above
the workspace test step with this wording and include the two new run lines:

```yaml
      # Deliberate scope: the workspace `--lib` command remains the stable baseline.
      # Targeted package-default runner/companion commands close known deterministic contract gaps.
      # Broad workspace `--all-targets` remains excluded from this merge gate (Refs #3222).
      - run: cargo test --workspace --lib --no-fail-fast
      - run: cargo test -p terraphim_gitea_runner --no-fail-fast
      - run: cargo test -p terraphim_llm_runner --no-fail-fast
```

The package-default steps stay after the baseline so the previous known-stable
workspace signal remains visible and unchanged.

## Step Order

1. Add the library unit test to
   `crates/terraphim_gitea_runner/src/lib.rs`.
2. Run focused RED:

   ```bash
   cargo test -p terraphim_gitea_runner --lib --no-fail-fast native_ci_workflow_declares_targeted_package_default_policy_tests -- --exact
   ```

   Expected on current `origin/main`: fail because the workflow has no targeted
   package-default commands. This argument ordering was directly verified against
   an existing exact lib unit test on 2026-08-14.
3. Edit `.gitea/workflows/native-ci.yml` to replace the stale comment and append
   the two targeted commands after the existing workspace `--lib` step.
4. Re-run focused GREEN:

   ```bash
   cargo test -p terraphim_gitea_runner --lib --no-fail-fast native_ci_workflow_declares_targeted_package_default_policy_tests -- --exact
   ```

   Expected after YAML change: pass. Verification must confirm the output says
   `running 1 test`, `1 passed; 0 failed`, and a non-zero filtered-out count
   (52 against the current 53-test lib inventory after adding this guard); exit
   code zero alone is insufficient because libtest `--exact` also succeeds when
   no test matches.
5. Run package-default verification:

   ```bash
   cargo test -p terraphim_gitea_runner --no-fail-fast
   cargo test -p terraphim_llm_runner --no-fail-fast
   ```

   Expected: runner package-default executes 53 lib + 21 integration tests, plus
   a zero-test bin and no doctests; llm executes 38 pass + 1 ignored including
   seven doctests. The ignored Docker test remains ignored.

## Regression Gates

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib --no-fail-fast
cargo test -p terraphim_gitea_runner --no-fail-fast
cargo test -p terraphim_llm_runner --no-fail-fast
```

Do not run:

```bash
cargo test -p terraphim_llm_runner --tests --no-fail-fast
cargo test -p terraphim_llm_runner -- --ignored
cargo test --workspace --all-targets
docker ...
```

## RED Evidence

The focused RED command checks the workflow through the authoritative lib guard.
On current `origin/main`, it must fail with missing targeted package-default
commands. After the YAML change, the same command must pass. The preserved
workspace baseline also runs the guard immediately in the same PR, so removal,
mutation, or duplication of either targeted command, or addition of forbidden
cargo-test flags, fails before the targeted steps are reached.

Residual bootstrap limit: outright deletion of the workspace `--lib` step also
deletes this in-repo guard's executor, so the guard cannot detect that deletion.
It does detect mutation of the baseline while lib tests still execute. This is
an irreducible limit of an in-repository guard; branch protection and review must
continue to protect outright removal of the authoritative baseline job step.

## Native PR Acceptance

Record total job duration, `cargo test --workspace --lib --no-fail-fast` step
duration, each targeted package step duration, and confirmation that no network,
Docker, or ignored Docker test executed for both existing observations:
`native-ci / build (push)` and `native-ci / build (pull_request)`.

Each targeted package step must finish in <=10 minutes and remain green on both
push and pull_request observations. Abort and do not merge if either targeted
step exceeds 10 minutes, flakes, reaches network/Docker, or executes the ignored
Docker test. The existing 90-minute job timeout is only a hard ceiling, not the
acceptance target.

## Branch-Protection Invariant

Do not rename workflow `native-ci`, job `build`, triggers, or status contexts;
do not mutate branch protection. The current native CI contexts remain
merge-authoritative: `native-ci / build (push)`, `native-ci / build
(pull_request)`, and `native-ci / build (workflow_dispatch)`.

## Failure Semantics

- Workspace `--lib` failures continue to fail the build as before.
- The lib guard is authoritative under the existing workspace baseline before
  targeted steps execute.
- `terraphim_gitea_runner` package-default failures fail the same native CI job.
- `terraphim_llm_runner` package-default failures fail the same native CI job.
- Rustdoc `compile_fail` regressions fail under package-default llm testing.
- The explicit live-Docker ignored test remains skipped unless a future,
  separate design intentionally opts into ignored tests.

## Determinism Boundary

Package-default testing is an open set. Future external or
environment-dependent tests in these crates must use explicit
`#[ignore = "requires ..."]` or move to a separately approved lane; they must
never silently enter the merge gate.

Current known state: the llm Docker test conforms by being ignored. After the
guard is added, the gitea runner has 53 lib + 21 integration tests, zero
`#[ignore]`, and
integration suites that include socket/timing behavior. This change adds no
automated ignore scanner.

## Trybuild and Toolchain Drift

Exact `.stderr` fixtures are intentionally merge-blocking. Any rustc/toolchain
update that changes diagnostics requires explicit fixture review and fixture
updates in that update PR. CI must never auto-bless with `TRYBUILD=overwrite`.
Rustdoc `compile_fail` contracts remain less diagnostic-sensitive but still run
under package-default llm testing.

## Diff Budget

| File | Expected Diff |
|---|---:|
| `.gitea/workflows/native-ci.yml` | Amend 3 comment lines and add 2 `run:` lines. |
| `crates/terraphim_gitea_runner/src/lib.rs` | One `#[cfg(test)]` unit test, approximately 30-45 lines. |

No other implementation files may change.

## Eliminated

Rejected: integration-test-file guard (bootstrap gap), workspace
`--all-targets` (broader merge gate), `--tests` for llm runner (omits rustdoc
contracts), ignored-test execution (Docker/environment dependency), new ignore
scanner (extra moving part), new workflow/job/status context (branch-protection
churn), and new parser/dependency (existing parser is sufficient).

## Rollback

If the added package-default steps destabilize native CI unexpectedly, revert
only the workflow amendment in `.gitea/workflows/native-ci.yml` and the lib test
in `crates/terraphim_gitea_runner/src/lib.rs`; keep the workspace `--lib`
baseline intact; file a follow-up issue with native-runner logs and package
command timing; do not change branch protection or status contexts.

## Human Approval Checklist

- [x] Approve design status change from DRAFT to implementation-authorized.
- [x] Confirm exact new test name:
      `native_ci_workflow_declares_targeted_package_default_policy_tests`.
- [x] Confirm only implementation files are `.gitea/workflows/native-ci.yml`
      and `crates/terraphim_gitea_runner/src/lib.rs`.
- [x] Confirm no integration-test-file edit, no `--tests`, no `--all-targets`,
      no `--ignored`, no Docker, no network, and no branch-protection/status
      context mutation.
- [x] Confirm the workflow keeps the existing workspace `--lib` baseline and
      adds both package-default targeted commands.
- [x] Confirm human approval remains required before implementation.

## Specification Interview Findings

Convergence Status: Complete. Multiple independent research/design KLS rounds
already converged, and Phase 2 was approved at SHA
`632acff377eff7053a397b11ce589c03b43b9241ab7880c10f374ab8dfbb83ca` with
"perfect, proceed"; see
`docs/plans/decision-native-ci-ignore-policy-design-gate.md`. No new
implementation files or requirements are introduced beyond the approved design.

The only follow-up interview question asked whether forbidden cargo-test flags
should be globally banned or restricted. It timed out, so the approved design
default remains authoritative: all native-CI cargo test steps are constrained;
future exceptions require an explicit reviewed guard update.

Precise decisions:

- Concurrency/race: workflow steps are sequential; the guard reads
  compile-time embedded YAML with `include_str!`; there is no mutable state and
  no race surface.
- Failure/recovery: any guard failure or targeted package-default test failure
  blocks the same job. Recovery is the two-file rollback already specified. Do
  not add retries that mask flakes.
- Edge boundaries: env-prefixed commands and YAML block scalars are covered by
  line-wise scanning plus `strip_env_assignments`. Exact, duplicate, and
  missing baseline/targeted commands are covered. Residual deletion of the
  baseline executor remains an external review/branch-protection concern.
  Forbidden-flag detection covers directly invoked `cargo test` lines only;
  interpreter-wrapped invocations such as `bash -c 'cargo test ...'` remain an
  explicit review/branch-protection concern.
- Evolution/compatibility: the global forbidden-flag policy stands. Future
  exceptions require an explicit reviewed guard update. Package-default remains
  an open set; environment-dependent tests require an explicit ignored reason
  or a separately approved lane.
- Security/privacy: the change uses no secrets, network, or Docker. The ignored
  Docker test remains skipped. `TRYBUILD` stderr is never auto-blessed.
- Scale/performance: each targeted step must complete in <=10 minutes on push
  and PR observations; abort otherwise. The existing 90-minute job timeout is
  only the ceiling.
- Integration/migration: status contexts, jobs, triggers, data shape, APIs, and
  migrations remain unchanged.
- Operations: capture per-step durations and ignored-test count; require green
  push and PR observations; use the rollback criteria already documented.
  Post-guard package-default runner evidence must report 53 lib + 21
  integration tests; the executable verification step above now uses this count.
- Accessibility/i18n/user mental model: not applicable as no UI or user-facing
  text changes. The developer mental model is the authoritative CI policy
  contract.

Traceability:

| Finding | Evidence |
|---|---|
| Sequential guard, no race | `native-ci.yml` step order; embedded YAML guard test |
| Exact/missing/duplicate commands | Guard asserts exact counts for baseline and both targeted commands |
| Direct Cargo-test steps globally constrained | Guard scans directly invoked native-CI cargo test lines for `--tests`, `--all-targets`, `--ignored`; interpreter-wrapped commands remain a documented review boundary |
| Env prefixes/block scalars covered | Guard uses `command.lines()` and `crate::policy::strip_env_assignments` |
| Same-job failure semantics | Workspace lib guard runs before targeted steps in `native-ci / build` |
| Ignored Docker remains skipped | Package-default llm evidence must report ignored count without `--ignored` |
| Performance limit | Push and PR evidence must record targeted step duration <=10m |
| Status context stability | PR evidence must preserve `native-ci / build (push)` and `(pull_request)` |
| Rollback path | Two-file revert: workflow amendment plus gitea-runner lib guard |

### Phase 4 Defect Loop-Back Clarifications

Independent structural review Round 1 identified two under-enforced parts of
the already-approved policy. These are specification/test-strategy corrections,
not implementation-scope expansion:

| ID | Finding | Origin | Required Resolution |
|---|---|---|---|
| D001 | `--include-ignored` can execute ignored tests but was absent from the forbidden-token list. | Phase 2.5 security/operations boundary | Reject `--include-ignored` alongside `--ignored`; prove the negative path with a temporary workflow mutation. |
| D002 | Exact-count assertions did not reject an extra direct Cargo-test step or enforce the approved baseline → runner → companion order. | Phase 2 test strategy | Collect every directly invoked Cargo-test line after env stripping and assert that the ordered list equals exactly the three approved commands. |

Interpreter-wrapped Cargo commands and outright deletion of the baseline
executor remain the documented review/branch-protection boundaries. No new
files, dependencies, workflow contexts, scanners, or runtime behavior are added.
