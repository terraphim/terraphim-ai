# Research: sha2 0.10.9 -> 0.11.0

**Status**: Approved for merge after build verification
**Canonical Path**: `docs/plans/research-pr911-sha2-011.md`
**Change Slug**: `pr911-sha2-011`
**Author**: OpenCode
**Date**: 2026-07-06

## Executive Summary

PR #911 bumps `sha2` from 0.10.9 to 0.11.0 across crates that pin it: `terraphim_update`, `terraphim_validation`, `terraphim_orchestrator`, and `terraphim_github_runner_server`. The highest-impact usage is in `terraphim_orchestrator/src/webhook.rs`, where `sha2::Sha256` is used with `hmac` to verify GitHub/Gitea webhook signatures. `sha2` 0.11 is a major release that may change trait implementations used by `hmac`; verification builds and tests are required.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Cryptographic dependency update for security/compat |
| Leverages strengths? | Yes | Cross-crate build verification |
| Meets real need? | Yes | Dependabot major update |

**Proceed**: Yes, with build verification.

## Problem Statement

### Description
`sha2` 0.11.0 is a new major version. The workspace should consolidate on it.

### Impact
All crates using SHA-256 hashing, especially webhook HMAC verification.

### Success Criteria
- `cargo build --workspace` succeeds.
- `cargo test -p terraphim_orchestrator webhook` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Current State Analysis

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Orchestrator webhook | `crates/terraphim_orchestrator/src/webhook.rs` | HMAC-SHA256 signature verification |
| Validation manifest | `crates/terraphim_validation/Cargo.toml:46` | `sha2 = "0.10"` |
| Update manifest | `crates/terraphim_update/Cargo.toml:31` | `sha2 = "0.10"` |
| GitHub runner server | `crates/terraphim_github_runner_server/Cargo.toml:28` | `sha2 = "0.10"` |
| Orchestrator manifest | `crates/terraphim_orchestrator/Cargo.toml:70` | `sha2 = "0.10"` |

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `hmac` crate needs matching `sha2` traits | Medium | High | Build workspace and run webhook tests |
| Output type or trait method changes | Low | Medium | Compile check |
| Multiple `sha2` versions remain in lockfile | Low | Low | Confirm single `sha2` entry after merge |

## Recommendations

### Proceed/No-Proceed
Proceed after workspace build and webhook tests pass.

### Scope
- In scope: merge the bump, verify builds/tests.
- Out of scope: Changing hashing logic or adding new HMAC algorithms.

## Next Steps
1. Merge PR #911.
2. Build workspace.
3. Run orchestrator webhook tests.
4. Mirror to Gitea.
