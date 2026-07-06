# Research: rand 0.9.4 -> 0.10.1

**Status**: Approved for merge after build verification
**Canonical Path**: `docs/plans/research-pr915-rand-0101.md`
**Change Slug**: `pr915-rand-0101`
**Author**: OpenCode
**Date**: 2026-07-06

## Executive Summary

PR #915 bumps `rand` from 0.9.4 to 0.10.1 in `terraphim_server`. `rand` is used for ID/nonce generation in the server crate. The workspace also has historical security-fix guidance to replace `rand` with `fastrand` in `terraphim_multi_agent` and `terraphim_kg_agents`, but those migrations are separate and out of scope for this Dependabot bump.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Keeps server randomness crate current |
| Leverages strengths? | Yes | Narrow dependency maintenance |
| Meets real need? | Yes | Dependabot update |

**Proceed**: Yes.

## Problem Statement

### Description
`rand` 0.10.1 is the latest semver-compatible release for the server crate. The lockfile should reflect it.

### Impact
Only `terraphim_server` and its callers are affected.

### Success Criteria
- `cargo build -p terraphim_server` succeeds.
- `cargo test -p terraphim_server` succeeds.
- `Cargo.lock` diff is limited to `rand` graph.

## Current State Analysis

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Server manifest | `terraphim_server/Cargo.toml:48` | `rand = "0.9"` |
| Random usage | `terraphim_server/src/` | ID/nonce generation |

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| API change in `rand` 0.10 breaks server code | Low | High | Build and test server crate |

## Recommendations

### Proceed/No-Proceed
Proceed after server build/test verification.

### Scope
- In scope: merge the Dependabot bump.
- Out of scope: migrating other crates from `rand` to `fastrand`.

## Next Steps
1. Merge PR #915.
2. Build and test `terraphim_server`.
3. Mirror to Gitea.
