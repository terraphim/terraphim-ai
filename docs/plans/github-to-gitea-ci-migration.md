# Plan: GitHub Actions -> Gitea Actions CI migration

Execution plan for [ADR-0001](../decisions/0001-gitea-actions-authoritative-ci.md):
Gitea Actions (native `terraphim_gitea_runner`) is the authoritative CI; GitHub
Actions is retained only for release publishing.

## Current state (2026-06-03)

- All 6 polyrepos cut over to native-ci (`native-ci / build (push)` is the required
  gate; interim ADF lanes disabled).
- `terraphim-ai` (server-monorepo) still runs CI on **both** the flaky ADF lane
  (`adf/build` + `adf/pr-reviewer`, build-runner-llm + rch + `--profile ci`) **and**
  ~11 GitHub Actions CI workflows on push/PR.
- `cargo check --workspace` is clean on the native-runner branch; the recurring
  `adf/build` "4/4 failed" is the old lane's environment, not the code.

## Workstreams

### W1 -- Disable duplicate GitHub CI workflows (this PR)
Scope the following to `workflow_dispatch` (manual) only, removing push/pull_request/
tags so they no longer duplicate the Gitea native-ci lane:
`ci-pr`, `ci-main`, `ci-native`, `ci-firecracker`, `test-firecracker-runner`,
`test-matrix`, `vm-execution-tests`, `performance-benchmarking`, `sentrux-quality-gate`.
Untouched (release/publish): `publish-*`, `release-comprehensive`, `release-sign`,
`docker-multiarch`, `deploy-docs`, `deploy`, `publish-benchmarks-to-site`,
`python-bindings`. Left as-is (not push/PR CI): `claude`, `adf-agent-validation`,
`cleanup-target`, `compliance-scan`.

### W2 -- Cut `terraphim-ai` over to native-ci
Add `terraphim-ai` to the native runner `RUNNER_ACTIVE_REPOS`, add
`.gitea/workflows/native-ci.yml` (fmt/clippy/build/test), verify green, then swap
branch protection `adf/build`+`adf/pr-reviewer` -> `native-ci / build (push)` and
retire the `terraphim.toml` interim lane. Unblocks merging the native-runner crate
PR (#2023) and the W1 PR through a working gate.

### W3 -- Port `ci-firecracker` + VM tests to Gitea Actions
The native runner is host-only in M1; Firecracker execution is M2 (#2076). Once the
Firecracker route lands, add a Gitea workflow that runs the Firecracker VM-lifecycle
and VM-execution tests on the native runner, then retire the GitHub equivalents
(`ci-firecracker`, `vm-execution-tests`, `test-firecracker-runner`) entirely.

### W4 -- Port `performance-benchmarking` to Gitea Actions
Run Criterion benchmarks + regression gate on the native runner (sccache-backed),
publishing baselines to the site as today. Retire the GitHub `performance-benchmarking`
push/PR triggers (the site-publish step may stay on GitHub if it needs GitHub Pages).

## Sequence / dependencies

W2 first (gives a working authoritative gate) -> then W1 merges through it -> W3 after
the runner Firecracker route (#2076) -> W4 after W2. Each retirement of a GitHub
workflow happens only after its Gitea equivalent is proven green.

<!-- verdict-agents e2e probe -->
