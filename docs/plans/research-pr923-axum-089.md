# Research: axum 0.7.9 -> 0.8.9 in terraphim_validation

**Status**: Approved for merge after broader build verification
**Canonical Path**: `docs/plans/research-pr923-axum-089.md`
**Change Slug**: `pr923-axum-089`
**Author**: OpenCode
**Date**: 2026-07-06

## Executive Summary

PR #923 bumps `axum` in `terraphim_validation` from `0.7` to `0.8`, matching the version already used by `terraphim_server`, `terraphim_orchestrator`, `terraphim_gitea_runner`, and `terraphim_symphony`. The change consolidates the workspace onto a single `axum`/`axum-core`/`matchit` graph, removing duplicate 0.7/0.8 entries from `Cargo.lock`. Because `terraphim_validation` is a production crate (release validation system), this bump has wider blast radius than the test-only `axum-test` bump.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Resolves workspace axum version split |
| Leverages strengths? | Yes | Aligns validation crate with rest of stack |
| Meets real need? | Yes | Dependabot update, also reduces lockfile duplication |

**Proceed**: Yes, with build verification.

## Problem Statement

### Description
`terraphim_validation` is the only crate still on `axum 0.7`, causing `Cargo.lock` to carry two `axum`/`axum-core`/`matchit` versions. Upgrading it to `0.8` removes that duplication.

### Impact
- Build graph for `terraphim_validation` and consumers.
- Any runtime code using `axum` extractors, routing, or response types in `terraphim_validation`.

### Success Criteria
- `cargo build -p terraphim_validation` succeeds.
- `cargo test -p terraphim_validation` succeeds.
- `cargo clippy -p terraphim_validation --all-targets -- -D warnings` passes.
- Workspace no longer resolves two `axum` versions.

## Current State Analysis

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Validation crate manifest | `crates/terraphim_validation/Cargo.toml:41` | Currently `axum = "0.7"` |
| Validation axum usage | `crates/terraphim_validation/src/` | Server API harness, HTTP testing utilities |
| Server crate | `terraphim_server/Cargo.toml:29` | Already `axum = { version = "0.8.7", ... }` |
| Orchestrator crate | `crates/terraphim_orchestrator/Cargo.toml:68` | Already `axum = "0.8"` |

### Data Flow
`terraphim_validation` builds routers and uses `tower`/`tower-http` layers for release validation and server API tests.

## Constraints

- `axum 0.8` is already the workspace standard.
- `tower 0.5` and `tower-http 0.5` are kept unchanged.
- `async-trait` is no longer required by `axum 0.8` handlers, but the crate may still use it elsewhere.

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Handler trait changes break validation routers | Low | High | Full build + clippy + test of `terraphim_validation` |
| `axum::extract` API changes | Low | High | Compile check covers this |
| `matchit` 0.8 routing differences | Low | Medium | Integration tests exercise routing |
| Duplicate dependency now removed exposes version conflict elsewhere | Low | Medium | Build full workspace after merge |

## Recommendations

### Proceed/No-Proceed
Proceed after explicit build verification.

### Scope
- In scope: merge PR #923, verify `terraphim_validation`, verify workspace builds.
- Out of scope: Refactoring handlers to remove `async-trait` or adopting new `axum 0.8` features.

## Next Steps
1. Merge PR #923.
2. Run `cargo build --workspace`.
3. Run `cargo test -p terraphim_validation`.
4. Run `cargo clippy -p terraphim_validation --all-targets -- -D warnings`.
5. Mirror to Gitea.
