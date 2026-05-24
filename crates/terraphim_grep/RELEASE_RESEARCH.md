# Research Document: terraphim_grep First Release

**Status**: Draft (awaiting approval)
**Author**: Alex (via disciplined-research skill)
**Date**: 2026-05-24
**Reviewers**: Alex Mikhalev

## Executive Summary

terraphim_grep is a brand-new crate on `task/1743-terraphim-grep` (PR #1825) with no prior release. Releasing it cleanly requires choosing among five distribution paths -- two of them blocked by external dependencies, three viable today. The repo has a mature release pipeline (`release-comprehensive.yml`) that currently builds three binaries across seven targets but **does not include terraphim_grep**. Adding it is a small workflow edit; the bigger decisions are versioning (decouple from workspace SemVer or stay aligned), tag convention (extend `terraphim_grep-v*` or roll into next `v*`), and the crates.io publish path (blocked by `fff-search` git-source dependency).

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | New feature shipped end-to-end (D001/D005 fixed + KG boost + four-layer test pyramid + benches). Marketing posted. Natural moment to mark the artefact. |
| Leverages strengths? | Yes | Reuses existing release workflow, publish-crates.sh, Homebrew tap, 1Password credentials. Adding one crate to an established pipeline. |
| Meets real need? | Yes | The binary already works (verified live against free OpenRouter). Without a release, users have to clone + build with specific feature flags. A binary release lowers the adoption barrier from "1 hour" to "1 minute." |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

terraphim_grep needs a first release so users can install and run it without cloning the
workspace and figuring out the `--features "code-search openrouter"` incantation. Today
the only way to use it is to build from source.

### Impact

- New tool with no installation path = no adoption regardless of how strong the engineering is
- The marketing thread already posted (https://x.com/AlexMikhalev/status/2058521549406085579) sends people to the repo with no concrete binary link
- Without a tagged release the long-form blog can't link to a downloadable artefact

### Success Criteria

A user reading the blog post or tweet thread can install terraphim_grep on darwin, linux,
or windows in under 60 seconds, run it against a real codebase, and see the KG boost in
action. Rollback path is documented and exercised.

## Current State Analysis

### Existing Release Infrastructure

| Component | Location | Purpose |
|-----------|----------|---------|
| Main release workflow | `.github/workflows/release-comprehensive.yml` | Tag-triggered cross-platform builds + signed macOS + Debian packages + Homebrew tap + Docker |
| Crates.io publish workflow | `.github/workflows/publish-crates.yml` | Manual `workflow_dispatch` only; tag trigger disabled in favour of release-comprehensive |
| Publish script | `scripts/publish-crates.sh` | Dependency-ordered list of crates to publish; **does NOT include terraphim_grep** |
| release-plz config | `.release-plz.toml` | Automated version bumps + changelog; per-package entries exist for server/desktop/agent |
| Homebrew tap | `github.com/terraphim/homebrew-terraphim` | Live, updated by the workflow for server/agent. Old cli/repl formulas are deprecated |
| Release process docs | `docs/RELEASE_PROCESS.md`, `docs/src/release-process.md` | Documented |

### What the Release Workflow Builds Today

```yaml
cargo build --release -p terraphim_server --bin terraphim_server
cargo build --release -p terraphim_agent --bin terraphim-agent
cargo build --release -p terraphim-cli --bin terraphim-cli
```

terraphim_grep is **not in the matrix**. Adding it requires one line per build step.

### Build Matrix (7 targets)

| Target | Runner | Cross-compile |
|---|---|---|
| `x86_64-unknown-linux-gnu` | ubuntu-22.04 | native |
| `x86_64-unknown-linux-musl` | ubuntu-22.04 | cross |
| `aarch64-unknown-linux-musl` | ubuntu-22.04 | cross |
| `armv7-unknown-linux-musleabihf` | ubuntu-22.04 | cross |
| `x86_64-apple-darwin` | self-hosted macOS X64 | native |
| `aarch64-apple-darwin` | self-hosted macOS ARM64 | native |
| `x86_64-pc-windows-msvc` | windows-latest | native |

### Tag Conventions

| Pattern | Scope | Status |
|---|---|---|
| `v*` (SemVer) | Full workspace release | Active. Most recent: `v1.19.3` (2026-05-20) |
| `v*.*.*` (CalVer) | Same workflow, calendar-based | Active. Most recent: `v2026.05.19` |
| `terraphim_server-v*` | Server-only | Active |
| `terraphim-ai-desktop-v*` | Desktop-only | Active |
| `terraphim_agent-v*` | TUI-only | Active |
| `terraphim_grep-v*` | **Does not exist** | Would need workflow trigger added |

Important: SemVer `v*` and CalVer `v2026.*` *both match the same `v*` glob* and trigger
the full workspace release. The team is mid-transition; both styles work.

### fff-search Dependency Reality

`fff-search = { git = "https://github.com/AlexMikhalev/fff.nvim.git", branch = "feat/external-scorer", optional = true }`

The upstream is a Cargo workspace (`AlexMikhalev/fff.nvim`) with sub-crates:
`fff-c`, `fff-core`, `fff-mcp`, `fff-nvim`, `fff-query-parser`, `fff-grep`. The crate
we import as `fff-search` actually resolves to one of those (version 0.5.1 in lockfile).
**None of these crates are on crates.io.** The upstream owner is `AlexMikhalev` -- same
person as the project owner, so publishing them is in our control but requires upstream
work first.

This blocks `cargo publish -p terraphim_grep --features code-search`. Confirmed
yesterday:

```
$ cargo publish -p terraphim_grep --dry-run --features code-search
error: all dependencies must have a version requirement specified when publishing.
       dependency `fff-search` does not specify a version
```

Publishing terraphim_grep **without** the `code-search` feature is technically possible
but ships a degraded tool -- the code-search path is the whole point.

## Constraints

### Technical Constraints

- **fff-search git dep blocks crates.io for the useful feature set.** Either publish
  fff-search workspace to crates.io first, OR pick a non-crates.io distribution path.
- **Workspace version is shared.** `terraphim_grep/Cargo.toml` uses `version.workspace = true`,
  so it inherits `1.19.3`. A per-crate version requires changing this to a literal version
  string -- mechanically simple but creates a precedent.
- **No `terraphim_grep-v*` workflow trigger exists.** Adding it requires editing
  `release-comprehensive.yml` -- a workflow that's currently driving server/agent/desktop
  releases. Risk of breaking those if the edit is sloppy.
- **macOS signing/notarisation requires self-hosted runners.** If we want signed macOS
  binaries, we use them. Skipping signing means users see Gatekeeper warnings on first
  run. Acceptable for a developer tool, less so for a polished release.
- **fff-search platform support unknown.** It uses `git2` (often with platform-specific
  TLS features). Has been verified on darwin-arm64; linux and windows untested.

### Business Constraints

- **Marketing already posted.** The X thread and Reddit drafts reference the work but
  not a downloadable artefact yet. The longer the gap, the weaker the conversion.
- **No revenue impact.** This is a developer tool, not a billable deliverable.
- **Per-priority alignment.** Q2 Terraphim platform priority calls for "one PR merged
  per week"; a release marks this PR as done rather than just merged.

### Non-Functional Requirements

| Requirement | Target | Current |
|---|---|---|
| Install time (binary) | < 60s | N/A (no binary published) |
| Cross-platform | 3+ targets (darwin-arm64, linux-x86_64 at minimum) | 1 (darwin-arm64 local only) |
| Reproducibility | Anyone with the source + tag can rebuild | Yes (workspace is hermetic) |
| Rollback time | < 10 min from "broken" to "withdrawn" | TBD |

## Vital Few (Essentialism)

### Scope Decision (Revised)

The team policy is **full-coverage releases**: do not default-skip channels just because
it is a first release. The existing infrastructure (release-comprehensive.yml,
publish-crates.sh, homebrew-terraphim tap, self-hosted macOS runners) exists precisely
to make this cheap. Skipping channels wastes that investment and forces a "v0.2.0
with everything turned on" redo. Better to do it once, properly.

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|---|---|---|
| **Unblock crates.io path by publishing fff-search workspace to crates.io first** | crates.io is in scope; the git dep is the only blocker, and the upstream is in our control | `cargo publish --dry-run` failure documented above; `AlexMikhalev/fff.nvim` is same owner |
| Cross-platform binaries on all 7 standard targets (darwin x86+arm64, linux gnu+musl x86+arm64, armv7, windows) | The release workflow already builds these for server/agent/cli; consistency matters | `release-comprehensive.yml` matrix |
| Signed + notarised macOS binaries via existing self-hosted runners | The runners exist; the credentials are in 1Password; using them is one boolean flag in the workflow per binary | Existing server/agent releases already do this |

### What's In Scope (per team policy)

| Channel | Path | Prerequisite |
|---|---|---|
| crates.io | Add to `publish-crates.sh`, run via `release-comprehensive.yml` | **Publish fff-search workspace to crates.io first** |
| GitHub release | Add binary build to `release-comprehensive.yml` matrix | Workflow edit |
| Gitea release mirror | Push tag to `origin` (Gitea) auto-mirrors via release-comprehensive workflow that watches GH; for Gitea-native release, separate `gitea-release-action` step | Workflow edit |
| Homebrew | Add `Formula/terraphim-grep.rb` to `terraphim/homebrew-terraphim` tap via workflow's existing Homebrew-tap update step | Workflow edit + formula authoring |
| Debian package | Add `terraphim_grep` to the `build-debian-packages` job alongside server/agent/cli | Workflow edit |
| Signed/notarised macOS | Self-hosted ARM64 runner + existing 1Password Apple credentials | Workflow edit |
| Windows | windows-latest runner, x86_64-pc-windows-msvc target | **Verify `fff-search` builds on Windows** (git2 TLS feature tweak may be needed) |
| All 7 cross-compile targets | Already in the matrix | Workflow edit per target |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|---|---|
| Docker image for terraphim_grep specifically | The existing `ghcr.io/terraphim/terraphim-server` image is the umbrella; terraphim_grep is a CLI not a service. Add to the multi-stage image only if downloads warrant. |
| Per-crate CHANGELOG.md (in addition to workspace CHANGELOG) | The release-plz workflow generates workspace-level changelog; per-crate is extra maintenance with no consumer demand yet |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|---|---|---|
| `terraphim_service` | Required for LLM client + router | Low (stable) |
| `terraphim_router` | Dev-dep for L2 tests only | Low |
| `terraphim_rolegraph`, `terraphim_automata`, `terraphim_types` | Core libs | Low (stable workspace) |
| `terraphim_config` | Role parsing | Low |
| Workspace version | Shared across all crates | **Medium** -- per-crate decoupling has implications for future workspace releases |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|---|---|---|---|
| `fff-search` (git) | 0.5.1 from `feat/external-scorer` branch | **High** for crates.io path; **Low** for binary distribution (compiles fine from git) | Wait for fff-search crates.io publish, or vendor it |
| `criterion` | 0.8 (dev-dep) | Low | None needed |
| `tempfile` | workspace (dev-dep) | Low | None needed |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Workflow edit breaks server/agent releases | Medium | High | Test via `workflow_dispatch` with `test_run: true` before pushing a real tag |
| Cross-compile fails on linux-x86_64 or windows | Medium | Medium | Build and test linux-x86_64 locally on bigbox before tagging |
| fff-search has platform-specific git2 TLS issues | Low | Medium | Verify `cargo build --target x86_64-unknown-linux-musl` succeeds locally first |
| Rolled tag conflicts with future workspace tag | Low | Medium | Use `terraphim_grep-v*` namespace so it never collides with `v*` |
| Marketing leads to download attempts before release lands | Medium | Low | Land the release artefact within 24 hours of the X thread |

### Open Questions

1. **Versioning**: start terraphim_grep at `0.1.0` (decoupled, signals "early") or align with workspace `1.20.0` (signals "production-grade")?
   *Resolution needed before tagging.*
2. **Signing**: skip macOS signing/notarisation for v0.1.0 to avoid self-hosted runner dependency, accept Gatekeeper warnings?
   *Default: yes, skip.*
3. **Windows**: include in v0.1.0 or defer? terraphim_grep is pure Rust, fff-search uses git2 which may need TLS feature tweaks per target.
   *Default: defer.*
4. **CHANGELOG**: do we maintain a per-crate `CHANGELOG.md` for terraphim_grep, or rely on the workspace one?
   *Default: per-crate, since the workspace one is sparse.*

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|---|---|---|---|
| `release-comprehensive.yml` can be extended without breaking server/agent releases | Workflow uses `if: always()` patterns and matrix builds | Could break next server release | No -- requires `workflow_dispatch test_run` |
| Per-crate versioning is acceptable to the team | terraphim_server-v0.1.0 exists in tag history, suggesting prior precedent | Workspace coherence story breaks | No -- needs sign-off |
| Users will tolerate Gatekeeper warnings on macOS | Convention for unsigned developer tools | First-run abandonment | Indirect evidence from comparable tools |
| The blog/marketing audience can install from a tarball | Audience is Rust devs / engineers | Lower conversion if true | Reasonable for v0.1.0 |
| fff-search will compile on linux-musl x86_64 | Pure Rust crate with git2 dep | Linux binary missing from release | No -- needs cross-compile test |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|---|---|---|
| "Create a release" = full workspace `v1.20.0` release | Couples grep to next workspace bump; bigger release notes; longer test cycle | Rejected -- couples scope to unrelated changes |
| "Create a release" = `terraphim_grep-v0.1.0` per-crate | Isolated scope, faster cycle, can rollback without affecting other crates | **Chosen** -- aligns with terraphim_server-v0.1.0 precedent and Vital Few |
| "Create a release" = GitHub release without crates.io | Skip the fff-search blocker entirely | **Chosen** for v0.1.0 |
| "Create a release" = also publish to crates.io | Requires fff-search upstream work first | Deferred to v0.2.0 or later |

## Research Findings

### Key Insights

1. **The release workflow is target-of-many but mentions-terraphim_grep-zero-times.** Adding it
   is one line per cargo invocation per matrix entry plus one line in the tag trigger.
   That's the entire integration cost.
2. **Per-crate versioning is already a thing.** `terraphim_server-v0.1.0` exists as the
   oldest tag in history. Starting terraphim_grep at v0.1.0 is consistent.
3. **fff-search is in our control upstream.** Same GitHub owner. So crates.io is a
   "when, not if" question, but solving it now is wrong scope for v0.1.0.
4. **The Homebrew tap is live and automated** but adding terraphim-grep to it would
   require a new formula. Skip for v0.1.0; if downloads warrant it, add at v0.2.0.
5. **macOS signing requires self-hosted runners** which adds operational dependency. For
   a developer tool first release, acceptable to skip and document the `xattr -d` workaround.
6. **The existing release workflow is resilient** (uses `if: always()` so partial failures
   still produce a release) -- low risk of cascading damage from a partial workflow edit.

### Relevant Prior Art

- `docs/RELEASE_PROCESS.md` -- the canonical release procedure, well-documented
- `terraphim_server-v0.1.0` -- precedent for per-crate v0.x release within this workspace
- `scripts/publish-crates.sh` -- shows the dependency-ordered publish pattern, useful when we eventually want crates.io
- `.github/workflows/release-comprehensive.yml` lines 540-707 -- the GitHub release creation block, where assets are uploaded

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|---|---|---|
| Cross-compile `terraphim_grep` for `x86_64-unknown-linux-musl` locally via `cross` | Verify fff-search builds on linux | 30 min |
| `workflow_dispatch test_run` of release-comprehensive with terraphim_grep added | Verify workflow edits work without affecting other crates | 1 hour |
| Manual GitHub release upload of the locally-built darwin-arm64 tarball | Sanity-check the release UX before committing to a tag | 15 min |

## Recommendations

### Proceed/No-Proceed

**Proceed.** Three viable paths exist; one (per-crate GitHub release at v0.1.0) is the
clear best fit for the Vital Few. crates.io is deferred until fff-search lands upstream.

### Scope Recommendations (Full Coverage)

For v0.1.0 -- all channels live, all standard targets:

1. **Targets**: all 7 from existing matrix (darwin x86+arm64, linux gnu+musl x86+arm64,
   armv7-linux-musleabihf, windows-x86_64)
2. **Distribution**: GitHub release (primary), Gitea mirror, Homebrew formula, Debian
   package, crates.io publish
3. **Signing**: signed + notarised macOS via existing self-hosted runners
4. **Tag**: `terraphim_grep-v0.1.0`
5. **Versioning**: decouple from workspace -- set `version = "0.1.0"` literally in
   `crates/terraphim_grep/Cargo.toml`

### Hard Prerequisite: Publish fff-search to crates.io

The only true blocker for crates.io publish is the git dep on `fff-search`. The upstream
(`github.com/AlexMikhalev/fff.nvim`) is a Cargo workspace with crates not yet on
crates.io. Since the upstream owner is the same person, this is a sequenced piece of
work, not an external dependency:

1. Audit fff-search workspace crates for crates.io readiness (descriptions, license, no
   git deps of their own, semver compliance)
2. Publish in dependency order: `fff-query-parser` (no internal deps) -> `fff-core` ->
   `fff-grep` (the one we use as `fff-search`)
3. Update `crates/terraphim_grep/Cargo.toml` to use the published version instead of git
4. `cargo publish -p terraphim_grep --dry-run --features code-search` must succeed

This prerequisite is **in scope for the release plan** and should be Step 1 of the
Phase 3 implementation. If it slips, the rest of the release can still proceed --
crates.io publish becomes a follow-up patch release (`v0.1.1` or `v0.2.0`) but the
binaries, Homebrew, and Debian package can all ship at v0.1.0.

### Risk Mitigation Recommendations

- **Cross-compile spike**: verify `cargo build --target x86_64-unknown-linux-musl` and
  `aarch64-unknown-linux-musl` and `x86_64-pc-windows-msvc` succeed locally before
  pushing the workflow edits
- **fff-search Windows**: explicitly verify git2 TLS works on Windows MSVC (may need
  `features = ["vendored-openssl"]` or similar)
- **Workflow edit**: test via `workflow_dispatch test_run: true` before pushing the
  real tag; partial workflow break would interrupt server/agent releases
- **Homebrew formula**: write and test the formula against a local install before
  committing to the tap repo
- **Rollback**: document concrete procedure for each channel (crates.io yank, GitHub
  release delete, Homebrew tap revert, Debian package supersede)
- **Tag from main after merge**: never tag a feature branch; merge PR #1825 to main,
  bump version on main, tag from main HEAD

## Next Steps

If approved:
1. Phase 2 (disciplined-design) produces the detailed implementation plan: which files to
   edit, what version strings, what release notes content, what rollback recipe
2. Phase 3 implements the release per the design

## Appendix

### Reference Materials

- `docs/RELEASE_PROCESS.md` -- canonical release procedure
- `.github/workflows/release-comprehensive.yml` -- workflow that needs editing
- `scripts/publish-crates.sh` -- crates.io publish script (not used in v0.1.0)
- `crates/terraphim_grep/RELEASE_NOTES_v1.19.3.md` -- the v1.19.3-aligned release notes already in the repo; will be retitled/superseded by v0.1.0 notes
- PR #1825: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1825

### Code Locations

| Component | Location | Action Needed in Phase 3 |
|---|---|---|
| Crate version | `crates/terraphim_grep/Cargo.toml:3` (`version.workspace = true`) | Change to literal `version = "0.1.0"` |
| Release workflow | `.github/workflows/release-comprehensive.yml` | Add `terraphim_grep-v*` trigger + binary build steps |
| Release notes | `crates/terraphim_grep/RELEASE_NOTES_v1.19.3.md` | Rename to `RELEASE_NOTES_v0.1.0.md`, update SHA-256 |
| release-plz config | `.release-plz.toml` | Add `[[package]] name = "terraphim_grep"` entry |
