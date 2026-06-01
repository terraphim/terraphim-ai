# Implementation Plan: terraphim_grep v1.20.0 Release

**Status**: Draft (awaiting approval)
**Research Doc**: `crates/terraphim_grep/RELEASE_RESEARCH.md`
**Author**: Alex (via disciplined-design skill)
**Date**: 2026-05-24
**Estimated Effort**: 1-2 days (excluding any unexpected cross-compile issues)

## Overview

### Summary

Ship terraphim_grep as part of a workspace 1.20.0 release. All standard distribution
channels live from day one: crates.io, GitHub Release, Gitea Release mirror, Homebrew,
Debian package, signed/notarised macOS, Windows binary. Tag `terraphim_grep-v1.20.0`.

### Approach

Sequenced into six gated stages, each with a clear pass/fail signal. The Vital Few from
research:
1. Unblock crates.io (now trivial -- fff-search 0.8.2 is on crates.io as of 2026-05-24)
2. Cross-platform binaries on all 7 standard targets via existing matrix
3. Signed + notarised macOS via existing self-hosted runners

### Scope

**In Scope:**
- Bump workspace 1.19.3 -> 1.20.0
- Bump `fff-search` from git pin to crates.io `0.8.2`, verify API compatibility (0.5.1 -> 0.8.2 is non-trivial)
- Extend `release-comprehensive.yml` to build/sign/package terraphim_grep for all 7 targets
- Extend `publish-crates.sh` to publish terraphim_grep in dependency order
- Author `Formula/terraphim-grep.rb` for the homebrew-terraphim tap
- Add `[[package]]` entry to `.release-plz.toml`
- Write `RELEASE_NOTES_v1.20.0.md` (rename of v1.19.3 file)
- Cross-compile spike on local + bigbox
- `workflow_dispatch test_run: true` dry-run
- Merge PR #1825 to main
- Tag `terraphim_grep-v1.20.0` from main HEAD
- Per-channel rollback procedures, exercised at least once via dry-run

**Out of Scope:**
- `terraphim_server`, `terraphim_agent`, `terraphim-cli` release content (they ride the
  workspace bump but their release notes are separate)
- Docker image for terraphim_grep specifically
- New features (the v1.20.0 release is the existing PR #1825 work, nothing more)

**Avoid At All Cost** (5/25 elimination list):
- Refactoring fff-search adoption to feature-gate per-platform (over-engineering;
  if it builds for one target it should build for all)
- Introducing a new versioning scheme (CalVer vs SemVer) -- stick with workspace SemVer
- Publishing additional terraphim crates we hadn't planned to (scope creep)
- Adding ARM Windows or BSD targets (out of matrix scope)
- Writing custom release-note generation (release-plz already does it)
- Auto-tweet integration (manual is fine for a single release)
- Adding new linters/checks to the release pipeline (zero changes to CI logic outside what's strictly required)

## Open Question Resolution

| Question | Answer | Source |
|---|---|---|
| Versioning | Workspace-aligned (bump workspace 1.19.3 -> 1.20.0) | User decision 2026-05-24 |
| Tag | `terraphim_grep-v1.20.0` (component-prefixed, workspace-aligned number) | User decision; note: the AskUserQuestion option labelled "terraphim_grep-v0.1.0" was a pattern example, not literal -- confirm interpretation |
| fff-search publish | **Already done** -- 0.8.2 stable on crates.io as of 2026-05-24 | crates.io API check |
| Release notes scope | Crate-focused | User decision 2026-05-24 |

## Architecture

### Release Pipeline Sequence

```
+---------------------+    +------------------------+    +--------------------+
| Stage 1: Prereqs    |    | Stage 2: Workflow      |    | Stage 3: Local     |
| - fff-search bump   |--> | extensions             |--> | cross-compile      |
| - API compat fix    |    | - matrix + Debian +    |    | - verify 7 targets |
| - Workspace 1.20.0  |    |   Homebrew + crates.io |    |   build clean      |
+---------------------+    +------------------------+    +--------------------+
                                                                  |
                                                                  v
+---------------------+    +------------------------+    +--------------------+
| Stage 6: Post-      |<-- | Stage 5: Tag + execute |<-- | Stage 4: Dry-run   |
| release verify      |    | - merge PR #1825       |    | workflow_dispatch  |
| - Each channel      |    | - push tag from main   |    | with test_run=true |
| - Update marketing  |    |                        |    |                    |
+---------------------+    +------------------------+    +--------------------+
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|---|---|---|
| Workspace 1.20.0 minor bump (not patch or major) | New feature (KG boost) but backward-compatible APIs | Patch (1.19.4) misrepresents the change; major (2.0.0) overstates it |
| Tag `terraphim_grep-v1.20.0` | Component-prefixed convention matches server/agent/desktop precedent; tag number matches workspace version for verify-versions step | Standalone `v1.20.0` would treat it as a workspace release with no clear grep emphasis; would also conflict if a separate workspace tag is pushed |
| Bump fff-search to crates.io 0.8.2 | The blocker is gone -- it's published. 0.5.1 git -> 0.8.2 crates.io may break our usage; that's a Stage 1 risk to surface early | Stay on git: blocks crates.io publish, holds whole release hostage |
| All 7 standard targets, not just the verified darwin-arm64 | Full coverage policy (memory `feedback_release_scope_full_coverage.md`); existing matrix already builds 7 for server/agent/cli; consistency matters | Subset: redo work at v1.20.1 with "now with Windows"; wastes infra investment |
| Crate-focused release notes (`RELEASE_NOTES_v1.20.0.md` under the crate dir) | Tag is component-scoped; notes scope matches tag scope | Workspace-context notes: dilutes the grep story with unrelated workspace changes |
| Merge-then-tag (PR #1825 lands on main first) | Tag points to merge commit, not feature branch; safer for verify-versions step which reads workspace state | Tag the feature branch: stale tag if branch gets squash-merged or rebased |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|---|---|---|
| Auto-generate `Formula/terraphim-grep.rb` from a template | Single formula, manual authoring is faster than building generation | Maintenance burden of a generator for one formula |
| Custom signing certificate for terraphim_grep | Reuse the existing Developer ID Application certificate from 1Password | Duplicate Apple developer account work for no benefit |
| Add terraphim_grep to the auto-update mechanism | terraphim_grep is invoked per-command, not a long-running service | Stale-update bugs we'd inherit |
| Multi-architecture Docker image | Out of scope per research | Image bloat, multi-stage build complexity |
| Pre-release / RC tag (`terraphim_grep-v1.20.0-rc.1`) | Pipeline doesn't differentiate prerelease behaviour beyond GitHub release flag; existing release-comprehensive workflow handles `rc` in tag name correctly | Extra ceremony for a release with already-verified L3 e2e |
| Manual sha256 generation per platform | The CI workflow generates checksums for tarballs/zips already | Drift between local and CI checksums |
| Posting follow-up X thread automatically | Marketing is one human decision per channel, not automated | Premature automation |

### Simplicity Check

**What if this could be easy?** The simplest path is:
1. Two-line Cargo.toml edits (workspace version + fff-search version)
2. Six-line workflow edit (add terraphim_grep-v* trigger, add to BINARIES verify, add three cargo build lines, add to Debian packaging, add to release notes body)
3. Six-line publish-crates.sh edit (add terraphim_grep to ordered list)
4. One new Homebrew formula file
5. Three-line release-plz.toml addition
6. One file rename + content update for release notes
7. Tag + push

If any of these turn into more than that, surface the complication rather than absorb it
silently.

**Senior Engineer Test**: Would a senior call this overcomplicated? The framework is
clear: reuse existing infrastructure. The only place complexity could leak in is the
fff-search 0.5.1 -> 0.8.2 API change, which Stage 1 verifies before anything else.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request (release-only PR)
- [x] No abstractions "in case we need them later" (no new release-mgmt code)
- [x] No flexibility "just in case" (specific channels, specific targets)
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose |
|---|---|
| `crates/terraphim_grep/RELEASE_NOTES_v1.20.0.md` | Crate-focused release notes (renamed from existing `RELEASE_NOTES_v1.19.3.md`) |
| `crates/terraphim_grep/RELEASE_RESEARCH.md` | Already exists (Phase 1 output) |
| `crates/terraphim_grep/RELEASE_DESIGN.md` | This file (Phase 2 output) |
| `Formula/terraphim-grep.rb` (in `terraphim/homebrew-terraphim` tap, separate repo) | Homebrew formula |

### Modified Files

| File | Changes |
|---|---|
| `Cargo.toml` (workspace) | Bump `version` 1.19.3 -> 1.20.0 |
| `crates/terraphim_grep/Cargo.toml` | Change `fff-search` from git source to `version = "0.8.2"`; remove the `branch = "feat/external-scorer"` reference. The crate version stays `version.workspace = true` (so it inherits 1.20.0). |
| `crates/terraphim_grep/src/hybrid_searcher.rs` | Possible API adjustments if `fff_search::{FilePicker, FilePickerOptions, FFFMode, GrepMode, GrepSearchOptions, ContentCacheBudget, grep_search, parse_grep_query}` signatures changed between 0.5.1 and 0.8.2. Investigation in Step 1.2. |
| `.github/workflows/release-comprehensive.yml` | (1) Add `terraphim_grep-v*` to the on.push.tags list. (2) Add `terraphim_grep` to the BINARIES array in `verify-versions`. (3) Add `cargo build --release --target ... -p terraphim_grep --bin terraphim-grep --features "code-search openrouter"` after the existing `terraphim-cli` build. (4) Add `terraphim-grep` to the binary-list in the artifact-preparation steps for Unix and Windows. (5) Add `terraphim-grep` to the Debian packaging job. (6) Update the GitHub release `body:` to mention `terraphim-grep-*` binaries. |
| `scripts/publish-crates.sh` | Add `terraphim_grep` to the `CRATES=()` array at the correct dependency-ordered position (after `terraphim_service` since grep depends on it transitively via the `llm` feature). |
| `.release-plz.toml` | Add a `[[package]] name = "terraphim_grep"` block with `changelog_path` set to the per-crate CHANGELOG if we choose to have one, or omitted to use the workspace CHANGELOG. |
| `crates/terraphim_grep/RELEASE_NOTES_v1.19.3.md` | Rename to `RELEASE_NOTES_v1.20.0.md`. Update version references, SHA-256s after CI build, link to PR #1825 merge commit. |

### Deleted Files

| File | Reason |
|---|---|
| (none) | The v1.19.3 release notes file gets renamed, not deleted |

## Tagged Asset Surface

What the GitHub release page will contain after the tag fires:

| Asset | Origin |
|---|---|
| `terraphim-grep-x86_64-unknown-linux-gnu.tar.gz` + `.sha256` | matrix |
| `terraphim-grep-x86_64-unknown-linux-musl.tar.gz` + `.sha256` | matrix (cross) |
| `terraphim-grep-aarch64-unknown-linux-musl.tar.gz` + `.sha256` | matrix (cross) |
| `terraphim-grep-armv7-unknown-linux-musleabihf.tar.gz` + `.sha256` | matrix (cross) |
| `terraphim-grep-x86_64-apple-darwin.tar.gz` + `.sha256` | self-hosted macOS X64 (signed + notarised) |
| `terraphim-grep-aarch64-apple-darwin.tar.gz` + `.sha256` | self-hosted macOS ARM64 (signed + notarised) |
| `terraphim-grep-universal-apple-darwin.tar.gz` + `.sha256` | lipo of the two darwin builds |
| `terraphim-grep-x86_64-pc-windows-msvc.zip` + `.sha256` | windows-latest |
| `terraphim-grep_1.20.0_amd64.deb` | Debian job |
| `terraphim-grep_1.20.0_arm64.deb` | Debian job |
| Source tarball | GitHub automatic |

Plus on the Homebrew tap:
- `Formula/terraphim-grep.rb` referencing the universal macOS binary

## API Design

No new public APIs in this release. The release surface is the *artefact set*, not the
crate's library API.

## Test Strategy

### Pre-tag Verification

| Verification | Tool/Command | Pass Criterion |
|---|---|---|
| fff-search 0.8.2 API compat | `cargo build -p terraphim_grep --features code-search` | No errors; existing 21 lib tests pass |
| Cross-compile linux-musl x86_64 | `cargo build --release --target x86_64-unknown-linux-musl -p terraphim_grep --features "code-search openrouter"` (via `cross`) | Binary produced |
| Cross-compile linux-musl aarch64 | same, target swapped | Binary produced |
| Cross-compile windows-msvc | `cross build --target x86_64-pc-windows-msvc ...` or windows runner test_run | Binary produced |
| Workflow dry-run | `gh workflow run release-comprehensive.yml -f test_run=true` | All matrix jobs pass; no GitHub release created |
| Publish dry-run | `cargo publish -p terraphim_grep --dry-run --features code-search` | Returns "Verifying" success not "all dependencies must have a version" |

### Post-tag Verification

| Verification | Tool/Command | Pass Criterion |
|---|---|---|
| Tag triggered workflow | `gh run list --workflow=release-comprehensive.yml --limit 1` | Workflow run started with the tag's SHA |
| All matrix jobs green | `gh run view <run-id>` | All `build-binaries` jobs `success` |
| Release page populated | `gh release view terraphim_grep-v1.20.0` | All expected assets listed |
| crates.io listing | `cargo search terraphim_grep` | Version 1.20.0 returned |
| Homebrew formula | `brew install terraphim/terraphim/terraphim-grep` on a clean Mac | Installs without error; `terraphim-grep --version` reports `1.20.0` |
| Debian install | `dpkg -i terraphim-grep_1.20.0_amd64.deb` on Ubuntu | Installs without error |
| End-to-end smoke per platform | Run the same `parse_grep_query` fixture command used in earlier L3 verification | Returns chunks; sufficiency=SearchOnly without LLM, RlmSynthesis with OPENROUTER_API_KEY set |

## Implementation Steps

### Stage 1: Prerequisites (estimated 2-4 hours)

**Step 1.1: Bump fff-search dependency**
- Files: `crates/terraphim_grep/Cargo.toml`
- Change: `fff-search = { git = "...", branch = "feat/external-scorer", optional = true }` to `fff-search = { version = "0.8.2", optional = true }`
- Test: `cargo build -p terraphim_grep --features code-search 2>&1`
- Expected outcome: either clean build (best case) or compile errors that indicate API drift between 0.5.1 and 0.8.2

**Step 1.2: Fix any 0.5.1 -> 0.8.2 API drift**
- Files: `crates/terraphim_grep/src/hybrid_searcher.rs` (the only file using `fff_search::*`)
- Approach: read the 0.8.2 docs on crates.io; update the eight imported symbols (`FilePicker, FilePickerOptions, FFFMode, GrepMode, GrepSearchOptions, ContentCacheBudget, grep_search, parse_grep_query`) to their new shapes
- Test: `cargo test -p terraphim_grep --features code-search` -- 24 unit + 3 router tests pass
- Bench: `cargo bench -p terraphim_grep --bench hybrid_search -- --test` -- all four groups still pass smoke
- If fff-search 0.8.2 has dropped or renamed any of those APIs, surface as a Step 1.2 blocker

**Step 1.3: Workspace version bump**
- Files: `Cargo.toml` (workspace `[workspace.package]` section)
- Change: `version = "1.19.3"` -> `version = "1.20.0"`
- Test: `cargo metadata --no-deps --format-version=1 | jq '.packages[] | select(.name == "terraphim_grep") | .version'` returns `1.20.0`
- Side effect: every workspace crate now reports 1.20.0; this is expected for a workspace-aligned bump

**Step 1.4: Verify publishability via dry-run**
- Command: `cargo publish -p terraphim_grep --dry-run --features code-search`
- Expected: success message ending with "Packaging" and "Verifying" but no upload

### Stage 2: Workflow Extensions (estimated 3-5 hours)

**Step 2.1: Add tag trigger**
- File: `.github/workflows/release-comprehensive.yml` (line 4-7, the `on.push.tags` list)
- Change: add `- 'terraphim_grep-v*'` after the existing patterns
- Test: cannot test until Stage 4 dry-run

**Step 2.2: Extend verify-versions BINARIES list**
- File: `.github/workflows/release-comprehensive.yml` (around line 76)
- Change: `BINARIES=("terraphim_server" "terraphim_agent" "terraphim-cli" "terraphim_grep")`
- Test: dry-run validates against workspace version

**Step 2.3: Add cargo build steps for all 7 targets**
- File: `.github/workflows/release-comprehensive.yml` (after the `terraphim-cli` build step, around line 226)
- Change: add (one per relevant build context)
  ```yaml
  - name: Build grep binary
    if: matrix.target != 'armv7-unknown-linux-musleabihf'  # skip armv7 to mirror current cli scope
    run: |
      cargo build --release \
        --target ${{ matrix.target }} \
        -p terraphim_grep \
        --bin terraphim-grep \
        --features "code-search openrouter"
  ```
- Test: dry-run produces binaries in expected paths

**Step 2.4: Add to artefact prep**
- File: `.github/workflows/release-comprehensive.yml` (the Unix prep step around line 266, Windows around line 290)
- Change: copy `target/${{ matrix.target }}/release/terraphim-grep[.exe]` into the artefact staging dir alongside the other three binaries
- Test: dry-run produces tarballs/zips containing the binary

**Step 2.5: Add to Debian packaging**
- File: `.github/workflows/release-comprehensive.yml` (the `build-debian-packages` job)
- Change: emit a `terraphim-grep_${VERSION}_${ARCH}.deb` alongside the existing server/agent packages. Uses `cargo-deb` (already in the workflow). Requires adding a `[package.metadata.deb]` block to `crates/terraphim_grep/Cargo.toml` with maintainer / depends fields modelled on `crates/terraphim_server/Cargo.toml`.
- Test: dry-run produces .deb file

**Step 2.6: Update GitHub release body**
- File: `.github/workflows/release-comprehensive.yml` (lines 663-700 area)
- Change: add a "### Grep Binaries" section mentioning `terraphim-grep-*` and pointing to `crates/terraphim_grep/RELEASE_NOTES_v1.20.0.md` for crate-focused details
- Test: dry-run produces expected release body text

### Stage 3: Publish & Tap Plumbing (estimated 2 hours)

**Step 3.1: Add to publish-crates.sh**
- File: `scripts/publish-crates.sh` (the `CRATES=()` array around line 50)
- Change: append `"terraphim_grep"` after the last entry. Ordering note: grep depends on terraphim_service, terraphim_router, terraphim_rolegraph, terraphim_automata, terraphim_types, terraphim_config -- all of which appear earlier in the list. Safe to append.
- Test: `./scripts/publish-crates.sh -v 1.20.0 -c terraphim_grep --dry-run`

**Step 3.2: Add to .release-plz.toml**
- File: `.release-plz.toml`
- Change: add
  ```toml
  [[package]]
  name = "terraphim_grep"
  publish = true
  semver_check = true
  ```
- Test: `release-plz update --dry-run` (if release-plz is locally available)

**Step 3.3: Author Homebrew formula**
- File: `Formula/terraphim-grep.rb` in the separate `terraphim/homebrew-terraphim` repo (not this workspace)
- Template (verify the SHA-256 placeholders are filled after Stage 5):
  ```ruby
  class TerraphimGrep < Formula
    desc "Intelligent hybrid grep with knowledge-graph boosting and LLM fallback"
    homepage "https://github.com/terraphim/terraphim-ai"
    version "1.20.0"
    license "MIT"

    on_macos do
      url "https://github.com/terraphim/terraphim-ai/releases/download/terraphim_grep-v1.20.0/terraphim-grep-universal-apple-darwin.tar.gz"
      sha256 "TBD_AFTER_CI_BUILD"
    end

    on_linux do
      if Hardware::CPU.intel?
        url "https://github.com/terraphim/terraphim-ai/releases/download/terraphim_grep-v1.20.0/terraphim-grep-x86_64-unknown-linux-musl.tar.gz"
        sha256 "TBD_AFTER_CI_BUILD"
      elsif Hardware::CPU.arm?
        url "https://github.com/terraphim/terraphim-ai/releases/download/terraphim_grep-v1.20.0/terraphim-grep-aarch64-unknown-linux-musl.tar.gz"
        sha256 "TBD_AFTER_CI_BUILD"
      end
    end

    def install
      bin.install "terraphim-grep"
    end

    test do
      system "#{bin}/terraphim-grep", "--version"
    end
  end
  ```
- Test: `brew install --build-from-source ./Formula/terraphim-grep.rb` locally on a Mac after Stage 5 lands SHA-256s
- Commit and push to the tap repo only after the GitHub release is live and SHA-256s are known. The existing release workflow has an "Update Homebrew tap" step (line 762 area) -- update that step to also handle the terraphim-grep formula by templating SHA-256s in.

### Stage 4: Cross-compile Spike (estimated 2 hours)

**Step 4.1: Local cross-compile via `cross`**
- Commands (run on darwin-arm64 dev box):
  ```bash
  cargo install cross --git https://github.com/cross-rs/cross  # if not present
  cross build --release --target x86_64-unknown-linux-musl -p terraphim_grep --features "code-search openrouter"
  cross build --release --target aarch64-unknown-linux-musl -p terraphim_grep --features "code-search openrouter"
  cross build --release --target armv7-unknown-linux-musleabihf -p terraphim_grep --features "code-search openrouter"
  ```
- Test: all three produce binaries; `file target/<target>/release/terraphim-grep` confirms architecture
- Risk: `git2` (transitively via fff-search?) may need `vendored-openssl` or similar feature for musl. If it fails, surface as a Stage 4 blocker.

**Step 4.2: Windows verification**
- Cannot cross-compile to Windows MSVC from Mac without significant setup. Instead: do this verification via the Stage 5 dry-run (windows-latest runner).
- If the Stage 5 dry-run fails on Windows, treat as a Stage 4 blocker -- do not proceed to the real tag.

**Step 4.3: macOS aarch64 (native)**
- Command: `cargo build --release -p terraphim_grep --features "code-search openrouter"` (we have already done this; it works)
- Test: already done -- verified in commit `fb261af30`

### Stage 5: Workflow Dry-Run (estimated 1-2 hours plus CI wait time)

**Step 5.1: Trigger workflow_dispatch**
- Command: `gh workflow run release-comprehensive.yml -f test_run=true --ref task/1743-terraphim-grep` (run from feature branch BEFORE merging, so we can iterate)
- Test: workflow run starts; all matrix jobs reach completion; no GitHub release is created (test_run=true blocks that)

**Step 5.2: Address any matrix failures**
- Most likely failure mode: Windows MSVC build of fff-search if git2 TLS isn't configured
- Mitigation: if fff-search needs platform-specific features for Windows, add them to terraphim_grep's Cargo.toml via target-specific dep configuration:
  ```toml
  [target.'cfg(windows)'.dependencies]
  fff-search = { version = "0.8.2", optional = true, features = ["vendored-openssl"] }
  ```

**Step 5.3: Verify artefact shapes**
- Download artefacts from the dry-run via `gh run download <run-id>`
- Confirm: every expected tarball/zip exists, contains the binary, runs `--version`

### Stage 6: Merge + Tag (estimated 30 minutes)

**Step 6.1: Merge PR #1825**
- Command: `gtr merge-pull --owner terraphim --repo terraphim-ai --index 1825` (or via Gitea web UI)
- Test: `git fetch origin main && git log origin/main --oneline -5` shows the merged commits

**Step 6.2: Pull main**
- Commands: `git checkout main && git pull origin main`

**Step 6.3: Tag from main HEAD**
- Commands:
  ```bash
  git tag -a terraphim_grep-v1.20.0 -m "terraphim_grep v1.20.0: hybrid search with KG boost and LLM fallback"
  git push origin terraphim_grep-v1.20.0
  ```
- Test: `gh run list --workflow=release-comprehensive.yml --limit 1` shows the tag-triggered run starting

**Step 6.4: Monitor the release run**
- Command: `gh run watch <run-id>` (or `tea` equivalent)
- Test: all jobs green; GitHub release page populated; crates.io publish step succeeded

### Stage 7: Post-Release (estimated 1 hour)

**Step 7.1: Capture SHA-256s and update Homebrew formula**
- Download each release asset; compute sha256; update `Formula/terraphim-grep.rb` in the tap repo with real values
- The workflow's existing "Update Homebrew tap" step should do this automatically -- verify it actually wrote the new formula. If manual, PR the formula to the tap repo.

**Step 7.2: Update release notes with final SHA-256s**
- File: `crates/terraphim_grep/RELEASE_NOTES_v1.20.0.md`
- Edit: replace placeholder SHA-256s with actual values from CI artefacts
- Commit on main as a documentation patch

**Step 7.3: Verify each channel from a clean machine**
- crates.io: `cargo install terraphim_grep --version 1.20.0 --features code-search` on a machine that hasn't seen this repo
- Homebrew: `brew install terraphim/terraphim/terraphim-grep` on a clean Mac
- Debian: `wget terraphim-grep_1.20.0_amd64.deb && sudo dpkg -i ...` on a clean Ubuntu VM (or bigbox)
- GitHub Release: download the darwin-arm64 tarball, extract, run `--version`
- Each smoke: run the parse_grep_query fixture command from earlier verification

**Step 7.4: Post-release marketing**
- Single follow-up tweet under the existing thread pointing to the GitHub release URL
- No new thread, no Reddit follow-up (the original drafts are still ready in `docs/src/blog/reddit-grep-announcement.md` -- post when ready, independently)
- Update the long-form blog post (`docs/blog/terraphim-grep-hybrid-search-with-llm-fallback.md`) with the "How to install" section pointing at the release URL

## Rollback Plan

For each channel, the rollback recipe + when to use it:

| Channel | Rollback Command | When to Use | Reversibility |
|---|---|---|---|
| crates.io | `cargo yank --vers 1.20.0 terraphim_grep` | Critical bug, security issue | Hard -- yank does not delete; everyone keeps their resolved versions, but new downloads are blocked |
| GitHub Release | `gh release delete terraphim_grep-v1.20.0 --yes` (keeps tag) or `gh release delete terraphim_grep-v1.20.0 --yes --cleanup-tag` (also deletes tag) | Mistake before downloads happen; quick recall | Soft -- can re-publish |
| Gitea Release | `tea release delete terraphim_grep-v1.20.0 --repo terraphim/terraphim-ai` | Same as GitHub | Soft |
| Homebrew tap | `cd homebrew-terraphim && git revert <commit-that-added-formula> && git push origin main` | Bad binary, broken formula | Soft -- users on the tap get the revert on next `brew update` |
| Debian | Supersede with `terraphim_grep_1.20.1_*.deb` carrying the fix; users get it on next `apt upgrade`. No way to "unpublish" a .deb already downloaded. | Bad binary | Forward-only; no real "remove" |
| Tag itself | `git push origin :refs/tags/terraphim_grep-v1.20.0` (delete) -- **only acceptable within the first hour and only if no users have cloned** | Catastrophic CI mistake | Hard -- destroys provenance |

**General principles**:
- Prefer a patch release (`1.20.1`) over a rollback. The release pipeline is fast enough that a fix-forward is usually cheaper than the support load of a partial rollback.
- Never force-push tags. The release workflow records SHA mappings; force-push breaks them.
- Each rollback step in the table above should be exercised at least once in dry-run mode (i.e., for crates.io, do a `cargo publish --dry-run` of 1.20.1 with the proposed fix; for Homebrew, do the revert locally without pushing).

## Migration (if applicable)

No database changes. No data migration. The "migration" is users moving from
"build-from-source" to one of the published channels -- documented in release notes.

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|---|---|---|
| `fff-search` | bumped from git 0.5.1 to crates.io 0.8.2 | Unblocks crates.io publish |

### Dependency Updates

| Crate | From | To | Reason |
|---|---|---|---|
| `fff-search` | git (0.5.1) | crates.io 0.8.2 | Unblocks crates.io publish for terraphim_grep |

Side effect: `cargo update` will resolve the new fff-search across the lockfile. Verify
no other crate in the workspace breaks (look at `cargo tree -i fff-search`).

## Performance Considerations

Release artefacts are static binaries; runtime performance is unchanged from PR #1825
(which already has benchmarks for hybrid search and KG boost overhead). No release-
specific benchmarks needed.

Build performance: cross-compilation via `cross` adds ~5-10 min per non-native target.
The existing matrix runs them in parallel, so total wall-clock for the release run is
dominated by the slowest job (typically macOS sign+notarise at ~15-20 min).

## Software Release Definition (SRD)

Not applicable -- this is an internal release without formal SRD requirements.

## Open Items (Resolved 2026-05-24)

| Item | Resolution |
|---|---|
| Tag number | `terraphim_grep-v1.20.0` -- workspace-aligned in both crate version and tag |
| `[package.metadata.deb]` block | Copy from `crates/terraphim_server/Cargo.toml` defaults, adjust description and binary name |
| Docker image for terraphim_grep | **Out of scope** -- terraphim_grep is a CLI invoked per-command, not a service |
| Workspace `v1.20.0` tag in addition to `terraphim_grep-v1.20.0` | **In scope** -- full workspace release as part of this work. Requires full workspace testing before tagging. |
| verify-versions for component-prefixed tag | Verified via Stage 5 dry-run |

## Expanded Scope: Full Workspace Release

Because v1.20.0 ships as both a workspace tag (`v1.20.0`) AND a component tag
(`terraphim_grep-v1.20.0`), the full workspace must be production-ready, not just grep.
This adds:

**Stage 0 (new): Full workspace test gate**
- `cargo test --workspace --features "code-search openrouter"` -- all crates pass
- `cargo build --workspace --release` -- entire workspace compiles
- `cargo clippy --workspace --tests --benches -- -D warnings` -- zero warnings
- `cargo fmt --all -- --check` -- clean
- Each binary (`terraphim_server`, `terraphim-agent`, `terraphim-cli`, `terraphim-grep`)
  runs `--version` and reports `1.20.0`
- Existing live integration tests (`-- --ignored`) pass at least for terraphim_service +
  terraphim_grep that have OpenRouter dependencies
- **Gate criterion**: cannot proceed to Stage 1 if any workspace test or build fails.
  If a non-grep crate has a real defect at 1.20.0, that's a separate PR before tagging.

**Stage 5 (expanded): Dry-run must succeed for ALL binaries**
- The existing `test_run=true` dispatch builds server + agent + cli; with my edits it
  also builds grep. All four binaries must pass for the run to be acceptable.

**Stage 7 (expanded): Post-release verification per binary**
- Smoke `--version` against each of the 4 published binaries on each of the platforms
  they ship for. Mostly automatable via a small shell script.

## Risk Register

| Risk | Likelihood | Impact | Mitigation | Owner |
|---|---|---|---|---|
| fff-search 0.5.1 -> 0.8.2 API drift requires substantive code changes | High | Medium | Stage 1.2 catches early; if drift is large, can revert the bump and ship without crates.io | Alex |
| Cross-compile fails on linux-musl or windows due to fff-search deps | Medium | Medium | Stage 4 spike catches before Stage 5; mitigation is target-specific features in Cargo.toml | Alex |
| Workflow edit breaks server/agent releases | Low | High | Stage 5 dry-run on the feature branch verifies; the edits are additive (new BINARIES entry, new build steps) not destructive | Alex |
| Homebrew tap formula has a typo that breaks `brew install` | Medium | Low | Stage 7.3 verification from a clean Mac catches it; fix-forward is 5 minutes | Alex |
| Marketing audience tries to install before the release lands | Medium | Low | Stage 6 (tag + workflow) takes ~30 min from merge; communicate ETA | Alex |
| 1Password Apple credentials expire or break | Low | High | Existing server/agent releases use the same credentials; if broken, blocks all macOS-signed releases; would surface as Stage 5 failure | (out of band) |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Risk register acknowledged
- [ ] Tag number resolved (1.20.0 vs 0.1.0)
- [ ] Open items addressed or explicitly deferred
- [ ] Human approval received

## Next Steps After Approval

1. Phase 2.5 (disciplined-specification) -- optional; this design is detailed enough that
   specification interview would mostly be confirmation rather than discovery
2. Phase 3 (disciplined-implementation) -- execute Stages 1-7 in order
3. Phase 4 (disciplined-verification) -- verify each channel from a clean machine
4. Phase 5 (disciplined-validation) -- check the release meets the success criteria from
   the research doc (install time <60s, cross-platform, reproducible, rollback documented)
