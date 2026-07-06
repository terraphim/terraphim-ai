# Research: napi 3.9.4 -> 3.10.3

**Status**: Approved for merge after CI sign-off
**Canonical Path**: `docs/plans/research-pr935-napi-3103.md`
**Change Slug**: `pr935-napi-3103`
**Author**: OpenCode
**Date**: 2026-07-06

## Executive Summary

PR #935 bumps the Node-API Rust binding crate `napi` from 3.9.4 to 3.10.3 in `Cargo.lock`. The manifest in `terraphim_ai_nodejs` pins `napi = "3.8.3"`, so the lockfile change pulls a compatible minor version. Upstream release notes include fixes for off-thread JS error cloning and custom-GC routing, which improve correctness of Node bindings. No source changes are required.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Correctness fixes for Node bindings |
| Leverages strengths? | Yes | Low-risk infrastructure update |
| Meets real need? | Yes | Dependabot minor update with upstream bug fixes |

**Proceed**: Yes.

## Problem Statement

### Description
`napi` 3.10.3 contains correctness fixes for `Error` object handling across threads and custom GC paths. The lockfile should use the latest compatible minor version.

### Impact
Only `terraphim_ai_nodejs` runtime behaviour is affected.

### Success Criteria
- `cargo build -p terraphim_ai_nodejs` succeeds.
- Node.js smoke tests still pass.
- `Cargo.lock` diff is limited to `napi` 3.9.4 -> 3.10.3.

## Current State Analysis

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Node.js bindings crate | `terraphim_ai_nodejs/` | Exports Terraphim functions to Node |
| napi dependency | `terraphim_ai_nodejs/Cargo.toml:20` | `napi = { version = "3.8.3", ... }` |
| Macro usage | `terraphim_ai_nodejs/src/lib.rs` | `#[napi]` macros |

## Constraints

- `napi` is a direct production dependency of the Node bindings crate.
- Semver-compatible minor bump should not require source changes.

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Minor API change breaks build | Very low | High | Build crate after merge |
| Changed error handling affects JS consumers | Low | Medium | Run JS smoke tests |

## Recommendations

### Proceed/No-Proceed
Proceed.

### Scope
- In scope: merge the cleaned PR after CI passes.
- Out of scope: modifying Node binding source or public JS API.

## Next Steps
1. Merge PR #935.
2. Mirror to Gitea.
3. Build and run JS smoke tests for `terraphim_ai_nodejs`.
