# Research Document: Release CI in the Gitea-Authoritative / GitHub-Publish Model

**Status**: Draft — awaiting human approval before implementation  
**Author**: Terraphim AI (Grok)  
**Date**: 2026-06-14  
**Reviewers**: Alex  
**Related**: ADR-0001, ADR-0002, Gitea #1910, #2080, #2706, `.docs/research-release-blockers.md`

## Executive Summary

Release publishing is **broken for v1.20.4**: GitHub `release-comprehensive.yml` failed on all binary, Debian, and crates.io jobs, yet an **empty** GitHub release (no assets) was published. Root cause is a **polyrepo split (#1910) that outpaced the monorepo release workflow** — the tag still expects in-tree `terraphim_agent` / `terraphim_cli` / `terraphim_grep` crates that now live in Gitea polyrepos and are consumed via the private `terraphim` Cargo registry on `main`.

The correct architecture is already documented in ADR-0001 (Gitea CI gates merges) and ADR-0002 (polyrepo publish pipeline). What is missing is an **integrated release orchestration plan** that sequences Gitea validation → polyrepo publish → GitHub tag → GitHub release artefacts.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | v1.20.4 shipped with zero binaries; auto-update and Homebrew are broken |
| Leverages strengths? | Yes | `polyrepo-publish.sh`, `terraphim_gitea_runner`, and ADRs already exist |
| Meets real need? | Yes | Public users consume GitHub releases; internal work happens on Gitea |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

Terraphim AI uses a **dual-remote model**:

- **Gitea** (`git.terraphim.cloud`) — authoritative source, native CI (`native-ci / build (push)`), private Cargo registry, task management (PageRank workflow).
- **GitHub** (`github.com/terraphim/terraphim-ai`) — public mirror and **release publishing surface** (GitHub Releases, crates.io dispatch, Homebrew, Docker GHCR, npm/PyPI/WASM).

After #1910, core crates were extracted to six Gitea polyrepos. The monorepo `terraphim-ai` now builds `terraphim_server` against **registry dependencies**, but `release-comprehensive.yml` still tries to build **workspace-local** `terraphim_agent`, `terraphim-cli`, and `terraphim_grep` binaries.

### Impact

| Stakeholder | Impact |
|-------------|--------|
| End users | Cannot download v1.20.4 binaries; auto-update fails |
| Homebrew | Formula update job skipped; tap stale |
| crates.io | Monorepo publish job references moved crates |
| Maintainers | Manual release attempts create empty GitHub releases |
| ADF / agents | No single documented release runbook tying Gitea + GitHub |

### Success Criteria

1. Tag push produces a GitHub release with **all expected platform binaries** and `checksums.txt`.
2. **Gitea `native-ci` is green** on the tagged commit before any publish step runs.
3. Polyrepo crates required by the monorepo are on **crates.io** (or release builds use published versions consistently).
4. Release workflow scope matches ADR-0001: GitHub Actions **tag/release only**, not merge gating.
5. Tagging and pushing follow the **dual-remote sync protocol** (origin first, then gitea, convergence verified).

## Current State Analysis

### CI authority split (ADR-0001, accepted 2026-06-03)

| System | Role | terraphim-ai trigger |
|--------|------|----------------------|
| Gitea Actions + `terraphim_gitea_runner` | **Authoritative** merge gate | `.gitea/workflows/native-ci.yml` on `push` |
| GitHub Actions | **Release/publish only** | Tag push → `release-comprehensive.yml` |

Most GitHub CI workflows (`ci-native`, `ci-pr`, `ci-main`) are already `workflow_dispatch`-only. Exceptions still on `push`: `deploy-docs.yml`, `publish-benchmarks-to-site.yml`, `python-bindings.yml`.

### Polyrepo publish pipeline (ADR-0002, proposed 2026-06-05)

Script: `scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh`  
Flow: `scripts/adf-setup/polyrepo-publish/polyrepo-publish-flow.toml`  
Runbook: `scripts/adf-setup/polyrepo-publish/RUNBOOK.md`

Per-repo cycle (topological order):

```
clone → scrub → Gitea CI gate → strip registry refs → GitHub push → GitHub CI gate
→ merge back to Gitea → dispatch publish-crates.yml
```

Repos: `terraphim-core` → `terraphim-config-persistence` → `terraphim-service` → `terraphim-agents` → `terraphim-kg-agents` → `terraphim-clients`.

Client binaries (`terraphim-agent`, `terraphim-cli`, `terraphim-grep`) live in **`terraphim-clients`** after publish.

### GitHub release workflow inventory

| Workflow | Trigger | Purpose | Post-#1910 status |
|----------|---------|---------|-------------------|
| `release-comprehensive.yml` | `v*` tags | Binaries, Docker, Debian, GH release, Homebrew, crates | **Broken** — builds extracted crates |
| `release-sign.yml` | `release: created` | Ed25519 zipsign on tarballs | Never ran (no assets) |
| `publish-crates.yml` | `workflow_dispatch` | crates.io (monorepo script) | **Stale** — crate list assumes monorepo paths |
| `publish-npm.yml` | `v*` tags | Node WASM package | May need registry/auth audit |
| `publish-pypi.yml` | `v*` tags | Python bindings | Same |
| `publish-wasm.yml` | `v*` tags | WASM automata | Crate may be in terraphim-core polyrepo |
| `docker-multiarch.yml` | Called by comprehensive | GHCR images | Failed at v1.20.4 tag |
| `publish-tauri.yml` | Deprecated | Desktop | Moved to `terraphim-ai-desktop` |

### v1.20.4 release post-mortem (live evidence, 2026-06-14)

**GitHub Actions run** `27495290642` — `Comprehensive Release` — **conclusion: failure**

| Job | Result | Root cause |
|-----|--------|------------|
| Verify version consistency | failure | Tag `v1.20.4`, workspace `1.20.3` in tag's `Cargo.toml` |
| Build binaries (all targets) | failure | `crates/terraphim_agent/Cargo.toml` missing; still matched by `members = ["crates/*"]` at tag |
| Build Debian packages | failure | `cargo-deb` install step |
| Publish Rust crates | failure | `publish-crates.sh` targets monorepo paths |
| Create GitHub release | skipped | Asset verification failed |
| Update Homebrew | skipped | — |

**GitHub release** `v1.20.4`: published 2026-06-14, **zero assets** (`gh release view v1.20.4` → `assets: []`).

**Tag `v1.20.4` Cargo.toml** (differs from current `main`):

- `members` still includes `crates/*` without excluding extracted crates.
- `workspace.package.version = "1.20.3"`.
- Explicit `crates/terraphim_grep` member still present.

**Current `main` Cargo.toml`** (post-#2260 fixes):

- Excludes `terraphim_agent`, `terraphim_cli`, `terraphim_service`, etc.
- Documents registry consumption from Gitea polyrepos.
- Workspace version still `1.20.3` (not bumped for v1.20.4 tag).

### Remote sync state

As of 2026-06-14: `git diff origin/main gitea/main --stat` is **empty** (converged).

Remotes: `origin` → GitHub, `gitea` → Gitea (token URL).

### Gitea native CI

`.gitea/workflows/native-ci.yml`:

```yaml
runs-on: terraphim-native
steps:
  - cargo fmt / clippy / build / test (workspace)
  - cargo test -p terraphim_gitea_runner
```

Prior infra blockers documented in `.docs/research-release-blockers.md` (#2462 rustup wrapper, #2610). Status on bigbox **not re-verified in this session** — treat as open until `native-ci` green on a test push.

### publish-crates workflow mismatch

`polyrepo-publish.sh` dispatches:

```bash
gh workflow run publish-crates.yml -f crate_list="$publishable" -f dry_run="false"
```

But `publish-crates.yml` only defines input `crate` (singular), not `crate_list`. Polyrepo crate publishing on injected per-repo workflows may work; **monorepo** `publish-crates.yml` does not match the ADF script contract.

## Constraints

### Technical

| Constraint | Source |
|------------|--------|
| Extracted crates consumed via `registry = "terraphim"` on Gitea | `Cargo.toml`, `.cargo/config.toml` |
| GitHub release builds cannot use private Gitea registry | ADR-0002 Gate 2 |
| `self_update` expects GitHub asset naming `{bin}-{target}` | `lessons-learned.md` |
| macOS signing requires 1Password + self-hosted macOS runner | `release-comprehensive.yml` |
| crates.io publish is irreversible (yank only) | ADR-0002 |
| Edition 2024 workspace | `Cargo.toml` |

### Business / process

| Constraint | Source |
|------------|--------|
| Gitea is SSOT for tasks and merges | CLAUDE.md, AGENTS.md |
| Tags must sync to both remotes | Remote Sync Protocol |
| Never force-push either remote | AGENTS.md |

### Non-functional

| Requirement | Target | Current v1.20.4 |
|-------------|--------|-----------------|
| Release completeness | All platform binaries + checksums | 0 assets |
| Gitea CI before release | Green `native-ci` on tagged SHA | Not enforced in workflow |
| Time to release | < 2h monorepo binaries | Failed at ~14–21 min |

## Vital Few (Essentialism)

| Constraint | Why vital |
|------------|-------------|
| Align release workflow with polyrepo topology | Without this, every tag fails at `cargo build` |
| Gitea green before GitHub tag push | Prevents publishing broken SHAs to the public mirror |
| crates.io availability for public builds | GitHub runners have no `terraphim` registry token |

### Eliminated from scope (this phase)

| Item | Why |
|------|-----|
| Migrating GitHub Releases to Gitea packages | ADR-0001 explicitly keeps publish on GitHub |
| Replacing self-hosted runners with GitHub-hosted for macOS | Cost + notarization tooling |
| Full ADF release automation in one sprint | Defer until manual runbook proven |
| Re-publishing all historical crate versions | Forward-fix from next release |

## Dependencies

### Internal

| Dependency | Impact | Risk |
|------------|--------|------|
| `polyrepo-publish.sh` | Must run before monorepo release if client crates changed | Medium — 4h full cycle |
| `terraphim_gitea_runner` | Gates Gitea merges | High if #2462 unfixed |
| `scripts/publish-crates.sh` | Monorepo crate list stale | High |
| `terraphim-ai-desktop` | Separate desktop release | Low — already triggered remotely |

### External

| Dependency | Risk |
|------------|------|
| crates.io indexing delay | Medium — 30s sleep may be insufficient |
| Apple notarization | Medium — credential expiry |
| GitHub Actions self-hosted labels (`bigbox`, `macOS`) | Medium — runner hygiene |

## Risks and Unknowns

### Known risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Empty release published again | High | High | Gate `create-release` on asset verification; delete draft releases |
| Version tag without Cargo bump | High | Medium | Mandatory pre-tag check in Gitea workflow or script |
| Polyrepo not published before monorepo tag | Medium | High | Document ordering; optional ADF release flow |
| `publish-crates.yml` / script contract drift | High | Medium | Add `crate_list` input; single publish entrypoint |

### Open questions

1. Should client binaries (`terraphim-agent`, `terraphim-cli`, `terraphim-grep`) be released from **`terraphim-clients` GitHub repo** instead of `terraphim-ai`? (Recommended: yes, per ADR-0002.)
2. Is `native-ci` currently green on bigbox after #2462 fix? (Verify before next tag.)
3. Should v1.20.4 be **re-run** (delete tag/release) or superseded by v1.20.5? (Recommend v1.20.5 — tag already public with no assets.)
4. Does `publish-crates.yml` need to live only on polyrepo GitHub mirrors, not monorepo? (Likely split responsibilities.)

### Assumptions

| Assumption | Basis | Risk if wrong |
|------------|-------|---------------|
| Tag pushed to GitHub triggers release workflow | Observed v1.20.4 runs | Gitea-only tags would not publish |
| `origin` and `gitea` main are identical before release | Verified 2026-06-14 | Divergence causes wrong SHA on GitHub |
| Registry versions on main resolve for release build | `terraphim_server` deps at 1.19.2–1.20.2 | Version skew breaks compile |

## Research Findings

### Key insights

1. **Two release surfaces** are required post-#1910: polyrepo pipeline (libraries + client binaries) and monorepo pipeline (server, docker, nodejs bindings).
2. **v1.20.4 is a partial release** — metadata only, no artefacts; worse than no release for auto-update.
3. ADR-0001 follow-up ("scope GitHub workflows to release triggers only") is **mostly done** for CI, but docs/deploy workflows still push-gate.
4. The polyrepo publish script and monorepo `publish-crates.yml` have **API drift** (`crate_list` vs `crate`).
5. Tag creation did not include workspace version bump — `continue-on-error: true` on verify-versions allowed downstream jobs to attempt builds anyway.

### Relevant prior art

- `.docs/research-release-blockers.md` — infra blockers (#2462, #2610)
- `.docs/design-release-plan.md` — older monorepo-centric plan (pre-#1910)
- `docs/architecture/adr/0002-polyrepo-github-publish-pipeline.md`
- `lessons-learned.md` — Windows zip, asset naming, publish script sed bugs

## Additional finding: `terraphim_service` Gitea registry bug (2026-06-14)

**Confirmed:** `terraphim_service@1.20.2` on the Gitea `terraphim` registry is **self-inconsistent** when built with `--features openrouter`.

- **Published** (`~/.cargo/registry/.../terraphim_service-1.20.2/src/summary.rs`): `async fn enhance_descriptions_with_ai` (private)
- **Gitea source** (`terraphim-service` `main`): `pub(crate) async fn enhance_descriptions_with_ai` (fixed in `554d202`, merged `0d89247`)
- **Call sites:** `search.rs` lines 397–1166 call the method from a sibling module → `E0624` (9 errors with `openrouter` enabled)

Gitea `main` compiles cleanly (`cargo check -p terraphim_service --features openrouter` passes). The fix was never republished; registry still serves `1.20.2`.

**Release blocker:** Republish `terraphim_service@1.20.5` (or next aligned version) from `terraphim-service` **before** tagging `terraphim-ai` v1.20.5. Downstream crates (`terraphim_server`, `terraphim_rlm`, `terraphim_github_runner*`, `terraphim_ai_nodejs`) must bump their `terraphim_service` registry pin.

**Client binary decision (locked):** Build from `terraphim-clients` polyrepo; attach artefacts to **`terraphim/terraphim-ai` GitHub release** for auto-update URL stability.

## Recommendations

### Proceed

Yes — implement a **Gitea-first release orchestration** that composes existing ADR decisions rather than inventing a third CI path.

### Scope recommendations

**Phase 1 (unblock next release)**

1. Fix `release-comprehensive.yml` for monorepo-only artefacts (`terraphim_server`, docker, deb).
2. Add client-binary jobs that checkout/build from published `terraphim-clients` OR delegate to that repo's release workflow.
3. Enforce version gate (fail workflow, do not `continue-on-error`).
4. Repair v1.20.4: publish v1.20.5 with full assets; yank/annotate v1.20.4 as incomplete.

**Phase 2 (integrate Gitea)**

1. Document release runbook: Gitea `native-ci` → polyrepo publish (if needed) → version bump on Gitea → tag → push origin + gitea → GitHub release.
2. Optional ADF `release-publish` flow wrapping the runbook.
3. Align `publish-crates.yml` with polyrepo script (`crate_list` input).

**Phase 3 (hardening)**

1. Gitea-side `release-preflight` workflow on tag push (mirror to GitHub only after green).
2. Workflow audit closing ADR-0001 follow-up for non-release push triggers.

## Next Steps

If approved:

1. Review this research + design doc (`.docs/design-release-ci-gitea-github.md`).
2. File/update Gitea issue #2706 with findings.
3. Implement Phase 1 via `task/2706-release-ci` branch.

## Appendix: Code locations

| Component | Location |
|-----------|----------|
| Monorepo release workflow | `.github/workflows/release-comprehensive.yml` |
| Gitea native CI | `.gitea/workflows/native-ci.yml` |
| Polyrepo publish | `scripts/adf-setup/polyrepo-publish/` |
| Workspace exclusions | `Cargo.toml` `[workspace.exclude]` |
| Private registry config | `.cargo/config.toml` `[registries.terraphim]` |
| Crate publish script | `scripts/publish-crates.sh` |
| ADR CI authority | `docs/decisions/0001-gitea-actions-authoritative-ci.md` |
| ADR polyrepo publish | `docs/architecture/adr/0002-polyrepo-github-publish-pipeline.md` |