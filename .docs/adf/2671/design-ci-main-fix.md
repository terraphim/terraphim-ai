# Design Document: Fix CI Main Branch After Merge + Gitea-First Validation

**Status**: Draft
**Research**: `.docs/adf/2671/research-ci-main-broken.md`
**Author**: opencode
**Date**: 2026-06-14

---

## Overview

### Summary

Three actions to restore green CI on `main` after the merge from `task/2668-terraphim-lsp-foundation`:

1. **Fix workspace `Cargo.toml`**: Add `terraphim_lsp` to `members` list so `cargo --workspace` resolves it
2. **Verify `terraphim_lsp` compiles**: Run `cargo check -p terraphim_lsp` locally
3. **Validate on Gitea first**: Push to Gitea, let ADF agents (`adf/build`, `adf/pr-reviewer`) validate, then push to GitHub
4. **Manually trigger GitHub CI**: `gh workflow run ci-main.yml --ref main`

### Architecture: Gitea primary, GitHub public

```
┌──────────────────────────────────────┐
│           GITEA (primary)             │
│  git.terraphim.cloud/terraphim-ai    │
│                                       │
│  CI: ADF agents (adf/build,           │
│      adf/pr-reviewer) via webhook     │
│  Issues: PageRank-based prioritisation│
│  PRs: #2706 (this work)              │
└──────────────┬───────────────────────┘
               │ manual push via
               │ git push gitea main
               ▼
┌──────────────────────────────────────┐
│         GITHUB (public mirror)        │
│  github.com/terraphim/terraphim-ai   │
│                                       │
│  CI: GitHub Actions (workflow_dispatch)│
│  Releases: tags + release notes       │
│  Public consumption: crates.io, docs  │
└──────────────────────────────────────┘
```

### Approach

1. Fix code locally on `main` branch
2. Build and test locally (`cargo check --workspace`, `cargo test -p terraphim_lsp`)
3. Push to Gitea first → ADF agents validate
4. Push to GitHub → manually trigger `ci-main.yml`
5. Verify both sides green

## File changes

| File | Change | Rationale |
|------|--------|-----------|
| `Cargo.toml` | Add `"crates/terraphim_lsp"` to `members` list | `crates/*` glob doesn't include it because directory may not exist; explicit member ensures cargo resolves it |
| `crates/terraphim_lsp/` | Verify directory exists with Cargo.toml | Merge added it but it may not be on disk |
| `.docs/adf/2671/research-ci-main-broken.md` | New: research document | Documents findings |
| `.docs/adf/2671/design-ci-main-fix.md` | New: this document | Design for fix |

## What we're NOT doing

| Avoided | Why |
|---------|-----|
| Re-enabling push triggers on ci-main.yml | ADF agents control CI orchestration; workflow_dispatch is by design |
| Fixing ci-pr.yml `needs` bug | Separate issue, not related to merge CI failure |
| Fixing cargo audit flag issue | Pre-existing workflow YAML bug, not code |
| Fixing WASM build separately | Root cause (workspace resolution) fixes all three failures |
| Adding Gitea Actions workflows | Gitea CI is ADF-agent-based, not Actions-based |

## Validation plan

### Pre-merge (local)
```
cargo check -p terraphim_lsp          # LSP crate compiles
cargo check --workspace               # all workspace members resolve
cargo test -p terraphim_lsp --lib     # LSP tests pass
cargo clippy -p terraphim_lsp         # clean
cargo fmt -- --check                  # formatting correct
```

### Gitea (primary CI)
```
git push gitea main
→ ADF webhook → adf/build agent picks up commit
→ Build + test + lint on Gitea runner
→ Status posted to PR #2706
```

### GitHub (public CI)
```
git push origin main
gh workflow run ci-main.yml --ref main
→ Manual dispatch → bigbox runner
→ Format/lint, security scan, WASM build, Rust build, Docker, integration tests
```
