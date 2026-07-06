# Research: napi-derive 3.5.7 -> 3.5.9

**Status**: Approved for merge after CI sign-off
**Canonical Path**: `docs/plans/research-pr936-napi-derive-359.md`
**Change Slug**: `pr936-napi-derive-359`
**Author**: OpenCode
**Date**: 2026-07-06

## Executive Summary

PR #936 bumps `napi-derive` from 3.5.7 to 3.5.9 in `Cargo.lock`. The crate is consumed only by `terraphim_ai_nodejs`, which uses `napi-derive` macros to generate Node-API bindings. The manifest still pins `napi-derive = "3.5.2"`, so the lockfile change captures the transitive patch update within the semver-compatible range. The upstream release notes mention only an internal backend update (`napi-derive-backend`), making this a low-risk patch bump.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Keeps Node.js binding generator current |
| Leverages strengths? | Yes | Trivial infrastructure maintenance |
| Meets real need? | Yes | Dependabot patch update, low risk |

**Proceed**: Yes.

## Problem Statement

### Description
`napi-derive` 3.5.9 updates the derive backend. The lockfile should reflect the latest patch version compatible with the manifest's `3.5.2` requirement.

### Impact
Only `terraphim_ai_nodejs` build output is affected.

### Success Criteria
- `cargo build -p terraphim_ai_nodejs` succeeds.
- Generated bindings are unchanged (no `#[napi]` macro errors).
- `Cargo.lock` diff is limited to `napi-derive` and `napi-derive-backend`.

## Current State Analysis

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Node.js bindings crate | `terraphim_ai_nodejs/` | Exports Terraphim functions to Node |
| Derive macro dependency | `terraphim_ai_nodejs/Cargo.toml:21` | `napi-derive = "3.5.2"` |
| Macro usage | `terraphim_ai_nodejs/src/lib.rs` | `#[napi]` macros |

## Constraints

- `napi-derive` is a build-time macro dependency for Node bindings.
- `napi` itself remains at 3.8.3 in the manifest.

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Backend update changes generated binding layout | Very low | Medium | Build `terraphim_ai_nodejs` and run its JS smoke tests |
| Breaks `napi` 3.8.3 compatibility | Very low | High | Both crates are from the same `napi-rs` ecosystem and tested together |

## Recommendations

### Proceed/No-Proceed
Proceed.

### Scope
- In scope: merge the cleaned PR after CI passes.
- Out of scope: bumping `napi` itself, modifying bindings source.

## Next Steps
1. Merge PR #936.
2. Mirror to Gitea.
3. Build `terraphim_ai_nodejs` on main to confirm macro output is stable.
