# 1. Gitea Actions is the authoritative CI; GitHub Actions is for release publishing only

- Status: accepted
- Date: 2026-06-03
- Deciders: Alex Mikhalev (CTO)
- Refs: #1910, #2080, terraphim_gitea_runner crate, scripts/adf-setup/polyrepo-ci/

## Context

The #1910 split produced polyrepos (`terraphim-core`, `-config-persistence`,
`-service`, `-agents`, `-kg-agents`, `-clients`) hosted on the self-hosted Gitea
at `git.terraphim.cloud`, alongside the `terraphim-ai` server-monorepo which is
dual-hosted (Gitea `origin` + GitHub mirror).

CI has been served by two mechanisms during the migration:

- An **interim ADF lane** (the `adf-orchestrator` dispatching a deterministic
  `build-runner` agent via webhooks), posting the `adf/build` commit status.
- A **native Gitea Actions runner** (`terraphim_gitea_runner`, host execution
  with sccache, repo checkout, per-job auth), posting `native-ci / build (push)`.

As of 2026-06-03 all six polyrepos have been cut over to the native runner and
their branch protection requires `native-ci / build (push)`; the interim lane is
retired. The native runner is the proven, single CI path for the polyrepos.

The open question was where the **authority** for merge gating lives, given that
`terraphim-ai` is also on GitHub and historically used GitHub Actions on
self-hosted `[self-hosted, bigbox]` runners.

## Decision

**Gitea Actions (served by the native `terraphim_gitea_runner`) is the
authoritative CI for all repositories -- it gates branch protection and merges.**

**GitHub Actions is retained solely for publishing releases** (crates.io,
Homebrew, signed macOS, Debian, Windows artefacts). It does not gate merges and
is not the source of truth for build/test status.

Concretely:

- Branch protection on every repo (polyrepos and `terraphim-ai`) requires the
  Gitea-side native context (`native-ci / build (push)` for the polyrepos).
- Merges are performed on Gitea (`gtr` / Gitea API). A Gitea PR being mergeable
  with its native check green is sufficient authority to merge.
- GitHub workflows are scoped to release/publish triggers (tags, release events),
  not push/PR build gating.
- This mirrors the existing principle that Gitea is the single source of truth
  for task management (see CLAUDE.md / Gitea PageRank workflow).

## Consequences

Positive:

- One authoritative CI lane; no ambiguity about which check gates a merge.
- The native runner stack is lightweight (rch + sccache -> SeaweedFS) and fully
  under our control on `bigbox`, avoiding GitHub-hosted-runner cost and the
  GitHub<->Gitea status-sync problem.
- Release machinery stays on GitHub where the publishing integrations already live.

Negative / risks:

- Merging on Gitea does not consult GitHub Actions; contributors must not treat a
  green GitHub check as a merge gate (it may not run on push/PR at all).
- The native runner is currently a single instance (M1); capacity scaling is
  tracked in #2079. A runner outage blocks merges until restored.
- The native-runner crate and its host environment must be maintained on `bigbox`
  (systemd `--user` service, sccache credentials, repo-checkout token).

## Follow-ups

- Promote `terraphim_gitea_runner` from `task/native-gitea-runner` to `main`.
- Scope/confirm GitHub workflows to release triggers only.
- Native runner M2/M3 (Firecracker route, artifacts, broad `uses:` emulation,
  capacity scaling): #2076-#2079.
