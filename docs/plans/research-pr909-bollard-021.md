# Research: bollard 0.20.2 -> 0.21.0

**Status**: Approved for merge after feature-gated build verification
**Canonical Path**: `docs/plans/research-pr909-bollard-021.md`
**Change Slug**: `pr909-bollard-021`
**Author**: OpenCode
**Date**: 2026-07-06

## Executive Summary

PR #909 bumps `bollard` from 0.20.2 to 0.21.0 in `terraphim_rlm` and `terraphim_validation`. `bollard` is the Docker API client used by the RLM docker executor and by validation's optional Docker feature. The RLM executor uses bollard types such as `ContainerCreateBody`, `HostConfig`, and exec/container APIs. A 0.21 bump may rename model fields or builder methods, so feature-gated builds are essential.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Keeps Docker API client current |
| Leverages strengths? | Yes | Feature-gated verification manageable |
| Meets real need? | Yes | Dependabot update |

**Proceed**: Yes, with feature-gated verification.

## Problem Statement

### Description
`bollard` 0.21.0 is the latest minor release. The Docker executor and validation Docker integration should use it.

### Impact
- `crates/terraphim_rlm/src/executor/docker.rs`
- `crates/terraphim_validation` when `docker` feature enabled.

### Success Criteria
- `cargo build -p terraphim_rlm --features docker-backend` succeeds.
- `cargo build -p terraphim_validation --features docker` succeeds.
- `cargo test -p terraphim_rlm --features docker-backend` passes (if Docker unavailable, at least compilation passes).

## Current State Analysis

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| RLM docker executor | `crates/terraphim_rlm/src/executor/docker.rs` | Container create/start/exec/remove |
| RLM manifest | `crates/terraphim_rlm/Cargo.toml:55` | `bollard = { version = "0.20", optional = true }` |
| Validation manifest | `crates/terraphim_validation/Cargo.toml:74` | `bollard = { version = "0.20", optional = true }` |

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `bollard` model field renames break `HostConfig` setup | Medium | High | Feature-gated build |
| Exec/container API signature changes | Low | High | Feature-gated build |
| Docker daemon unavailable for tests | High | Low | Compilation is the primary gate |

## Recommendations

### Proceed/No-Proceed
Proceed after feature-gated builds pass.

### Scope
- In scope: merge bump, verify `docker-backend` and `docker` feature builds.
- Out of scope: Running live Docker integration tests (environment-dependent).

## Next Steps
1. Merge PR #909.
2. Build `terraphim_rlm` with `docker-backend`.
3. Build `terraphim_validation` with `docker`.
4. Mirror to Gitea.
