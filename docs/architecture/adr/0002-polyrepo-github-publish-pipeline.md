# ADR-0002: Polyrepo GitHub Publish Pipeline

**Status**: Proposed
**Date**: 2026-06-05
**Decider**: Alex
**Refs**: #1910 (polyrepo split), ADR-0001 (Gitea CI authority)

## Context

The terraphim-ai monorepo has been split into 6 polyrepos on Gitea (#1910). All
split repos are private. The public GitHub `terraphim/terraphim-ai` repo (52 stars)
is 108 commits behind Gitea main and has no split-repo structure.

Two CI runner systems are available:

| Runner | Platform | Label | Environment |
|--------|----------|-------|-------------|
| Gitea native | Gitea Actions | `terraphim-native` | bigbox (sccache, rch, full deps) |
| GitHub hosted | GitHub Actions | `ubuntu-latest` | Clean Ubuntu, no Terraphim deps |

We need an automated pipeline that:
1. Publishes each split repo to GitHub as a public repository
2. Scrubs internal secrets, infrastructure URLs, and private configuration
3. Validates on BOTH CI runners before promoting
4. Publishes selected crates to crates.io
5. Keeps Gitea as the authoritative source

## Decision

Build an ADF flow pipeline (`polyrepo-publish`) that uses both runners as
authoritative quality gates. The Gitea runner validates the internal build;
the GitHub runner proves the code works in a clean public environment and owns
crates.io publishing after public CI passes.

The pipeline is developed and rehearsed locally first using `adf-ctl --local
flow polyrepo-publish-local`. After the local dry-run passes, the same script
and production flow are installed on bigbox and run with the remote orchestrator.

`adf-ctl --local trigger ... --direct` remains useful for direct-dispatch smoke
tests of ADF agents and webhook/event plumbing, but current `adf-ctl flow` is
already local/direct execution of a flow file and does not require webhook or
HMAC dispatch.

The flow MUST run a complete publish cycle per repository before starting the
next repository. A stage-wise matrix (clone all, scrub all, push all, then wait
all) is explicitly rejected because it can push downstream repos before
`terraphim-core` has passed public CI and published the crates downstream repos
need.

### Dual-runner architecture

```
                 ADF orchestrator (bigbox)
                        |
          +-------------+-------------+
          |                           |
    Gitea runner                GitHub runner
  terraphim-native             ubuntu-latest
  (sccache + rch)             (clean build)
          |                           |
    Gate 1: internal             Gate 2: public
    build passes?                build passes?
          |                           |
          +------- BOTH PASS --------+
                      |
                GitHub publish workflow
                publishes to crates.io
```

The pipeline pushes a `publish/github-mirror` branch to Gitea first.
The Gitea runner builds it (Gate 1). Only after Gitea CI passes does the
pipeline create the GitHub repo and push. GitHub Actions then builds in a
clean Ubuntu environment (Gate 2). Both must pass before crates.io publish.

### Pipeline flow (per repo)

```
Gitea split repo (private)
        |
  [1] clone to staging dir
        |
  [2] scrub secrets (trufflehog + regex)
        |
  [3] rewrite Cargo.toml (strip Gitea registry refs)
        |
  [4] push publish/github-mirror branch to Gitea
        |
  [5] GATE 1: wait for Gitea native-ci on publish branch
        |       (terraphim-native runner: fmt + clippy + build + test)
        |
  [6] create GitHub repo + add .github/workflows/ci.yml
        |
  [7] push publish branch to GitHub as main
        |   (triggers GitHub Actions automatically)
        |
  [8] GATE 2: wait for GitHub Actions CI
        |       (ubuntu-latest: fmt + clippy + build + test)
        |
  [9] merge publish branch back to Gitea main
        |
 [10] dispatch GitHub Publish Crates workflow
        |       (ubuntu-latest publishes from the public repo)
```

### Execution lanes

| Lane | Command | Purpose | Pushes? |
|------|---------|---------|---------|
| Local flow dry-run | `POLYREPO_DRY_RUN=1 adf-ctl --local flow polyrepo-publish-local` | Validate orchestration, clone, scrub, and Cargo rewrite from the developer workstation | No |
| Local direct smoke | `adf-ctl --local trigger <agent> --direct --event push --sha <sha> --ref-name refs/heads/main` | Validate direct ADF dispatch/event handling for agents, not flow execution | No |
| Bigbox dry-run | `POLYREPO_DRY_RUN=1 adf-ctl trigger polyrepo-publish --wait` | Validate production paths, tokens, and bigbox staging without external mutation | No |
| Bigbox production | `POLYREPO_DRY_RUN=0 POLYREPO_PUBLISH_MODE=dependency adf-ctl trigger polyrepo-publish --wait` | Publish repos and crates using Gitea and GitHub runners | Yes |

Local dry-run uses `.terraphim/flows/polyrepo-publish-local.toml` and the same
`scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh` script. Production uses
`scripts/adf-setup/polyrepo-publish/polyrepo-publish-flow.toml` after it is
installed into `/opt/ai-dark-factory/orchestrator.toml` and the script is copied

### Promotion sequence

1. Run `bash -n scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh` locally.
2. Run `POLYREPO_DRY_RUN=1 adf-ctl --local flow polyrepo-publish-local`.
3. If local direct-dispatch changes were made, run `adf-ctl --local trigger ... --direct` against a harmless agent/event.
4. Copy the script to bigbox under `/opt/ai-dark-factory/scripts/polyrepo-publish/`.
5. Append or include `polyrepo-publish-flow.toml` in bigbox `orchestrator.toml`.
6. Run `/usr/local/bin/adf --check /opt/ai-dark-factory/orchestrator.toml` on bigbox.
7. Restart `adf-orchestrator` only after the config check passes.
8. Run `POLYREPO_DRY_RUN=1 adf-ctl trigger polyrepo-publish --wait`.
9. Run production with `POLYREPO_DRY_RUN=0` only after the bigbox dry-run passes.

### Per-repo steps

| # | Step | Runner | Description |
|---|------|--------|-------------|
| 1 | `clone` | ADF | Clone Gitea split repo to staging dir |
| 2 | `scrub` | ADF | trufflehog + regex secret scan |
| 3 | `rewrite-cargo` | ADF | Strip `registry = "terraphim"` from all Cargo.toml |
| 4 | `prepare-gitea-branch` | ADF | Commit rewrites, push `publish/github-mirror` to Gitea |
| 5 | `wait-gitea-ci` | **Gitea native** | Poll commit status until `native-ci` passes (30 min max) |
| 6 | `create-github` | ADF | `gh repo create` + inject `.github/workflows/ci.yml` |
| 7 | `push-github` | ADF | Push to GitHub, tag `publish/v{version}` |
| 8 | `wait-github-ci` | **GitHub Actions** | Poll `gh run list` until green (30 min max) |
| 9 | `merge-back` | ADF | Merge `publish/github-mirror` into Gitea main |
| 10 | `crates-publish` | GitHub Actions | ADF dispatches `publish-crates.yml`; GitHub runner publishes downstream-needed crates |

### GitHub Actions workflows (injected per repo)

The pipeline injects two workflows into each public repo:

1. `ci.yml` validates public buildability.
2. `publish-crates.yml` publishes selected crates by workflow dispatch after CI succeeds.

#### `ci.yml`

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo build --workspace
      - run: cargo test --workspace --lib --no-fail-fast
```

This proves the code builds in a vanilla Ubuntu environment with no
Terraphim-specific tooling (no sccache, no rch, no Gitea registry).

#### `publish-crates.yml`

The publish workflow accepts a space-separated `crate_list` and publishes crates
in that order. It skips crate versions that already exist on crates.io and uses
the repository or organisation `CARGO_REGISTRY_TOKEN` secret. ADF only dispatches
and waits for this workflow; it does not run `cargo publish` locally.

This is safer than local shell publishing because the artefact source is the
public GitHub repository that has just passed public CI, not an ADF staging
checkout.

### Dependency order

Repos are processed as complete units in strict sequence:

```
E1: terraphim-core          (types, automata, rolegraph, markdown-parser)
E2: terraphim-config-persistence (config, persistence, settings)
E3: terraphim-service       (service, middleware, router, haystack)
E4a: terraphim-agents       (multi_agent, orchestrator, evolution, messaging)
E4b: terraphim-kg-agents    (kg_agents, kg_linter, codebase_eval)
E5: terraphim-clients       (cli, mcp_server, grep, sessions, agent)
```

Each repository runs `clone -> scrub -> rewrite -> Gitea CI gate -> GitHub CI
because stripping `registry = "terraphim"` makes downstream public builds depend
on crates.io availability of upstream packages.

### Cargo.toml translation

```toml
# Before (Gitea internal)
terraphim_types = { version = "1.20", registry = "terraphim" }

# After (public, no registry)
terraphim_types = { version = "1.20" }
```

The rewrite also removes `[registries.terraphim]` from `.cargo/config.toml`.

### crates.io publishing

There are two publish modes:

| Mode | Purpose |
|------|---------|
| `dependency` | Default. Publish all crates needed by downstream layers after each repo passes both CI gates. Required for the full public GitHub build to work without the private Gitea registry. Publishing is performed by GitHub Actions. |
| `approved` | Publish only selected community-ready crates. Useful for a limited launch, but downstream repos may need Git dependencies or the Gitea registry until more crates are published. |

Approved community crates:

| Crate | Repo | Publish? | Rationale |
|-------|------|----------|-----------|
| terraphim_types | core | Yes | Foundation type library |
| terraphim_automata | core | Yes | Aho-Corasick engine, WASM-ready |
| terraphim_rolegraph | core | Yes | Knowledge graph with symbolic embeddings |
| terraphim-markdown-parser | core | Yes | Markdown parsing |
| terraphim_persistence | config | Yes | Embedded redb storage |
| terraphim_router | service | Yes | Multi-strategy LLM routing |
| terraphim_service | service | No | Too coupled, internal |
| terraphim_orchestrator | agents | No | ADF-internal |
| terraphim_mcp_server | clients | Later | After API stabilisation |

### Safety mechanisms

1. **Dual CI gates**: Both runners must pass. Neither can be bypassed.
2. **Dry-run mode**: `POLYREPO_DRY_RUN=1` skips all push/publish steps.
3. **Secret scrub**: trufflehog + custom regex for Gitea tokens, internal IPs,
   webhook secrets, 1Password refs. Blocks on any match.
4. **Branch isolation**: Rewrites go to `publish/github-mirror` branch, not main.
   Only merged back after both CI gates pass.
5. **Idempotent**: Re-running creates no duplicates. GitHub repos that exist are
   skipped. Pushes to already-current branches are no-ops.
6. **Rollback**: Each publish is tagged `publish/v{version}` on GitHub. Rollback
   is `git push --force origin {previous-tag}`.
7. **Topological ordering**: E1 must pass both gates and publish dependency crates before E2 starts.
8. **No stage-wise matrixing**: Each repo is processed as a complete unit to prevent partial public exposure of downstream repos.

## Consequences

### Positive

- One command publishes everything: `adf-ctl trigger polyrepo-publish`
- Dual CI gates catch both internal breakage AND public-consumer breakage
- GitHub runner proves the code works without Terraphim-specific infrastructure
- Gitea runner provides fast feedback (sccache, warm cache)
- Fully reproducible for subsequent releases

### Negative

- ~30-40 min per repo (Gitea CI wait + GitHub CI wait)
- 6 repos x 40 min = ~4 hours total
- crates.io publishing is irreversible (yank only)
- Requires `gh auth` on bigbox and `CARGO_REGISTRY_TOKEN` configured as a GitHub repository or organisation secret
- `dependency` publish mode may publish crates that are not yet strongly branded
  for standalone community use, but are necessary for public downstream builds

### Risks

| Risk | Mitigation |
|------|------------|
| Secret in git history | trufflehog + regex scan blocks before any push |
| Gitea CI passes but GitHub CI fails | Good: catches portability issues early |
| GitHub repo name collision | `gh repo create` is idempotent (no-op if exists) |
| crates.io name squatting | Reserve `terraphim-*` namespace in initial run |
| Cross-repo dependency break | Topological publish order + per-layer CI gates |
| GitHub runner down | Pipeline waits 30 min then fails; re-trigger later |
| Cargo rewrite deletes dependencies | Rewrite strips only the `registry = "terraphim"` field, never whole dependency lines |
| Downstream GitHub CI cannot resolve upstream deps | Publish upstream dependency crates before starting the downstream repo |
| ADF staging checkout differs from GitHub source | crates.io publish runs on GitHub Actions from the public repo SHA, not from local ADF staging |
