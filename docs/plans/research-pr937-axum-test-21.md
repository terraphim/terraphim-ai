# Research: axum-test 19.1.1 -> 21.0.0

**Status**: Approved for merge after CI sign-off
**Canonical Path**: `docs/plans/research-pr937-axum-test-21.md`
**Change Slug**: `pr937-axum-test-21`
**Author**: OpenCode
**Date**: 2026-07-06

## Executive Summary

PR #937 bumps the HTTP endpoint testing helper `axum-test` from 19.1.1 to 21.0.0. The crate is used only in test/dev contexts: one integration-test harness in `terraphim_validation` and three integration-test suites in `terraphim_server`. The intended change is a narrow dependency bump with no source-file modifications. We previously cleaned unrelated `Cargo.lock` drift, so the net diff is now limited to `axum-test`, `educe`, and `enum-ordinalize` graph changes plus two manifest version updates.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Keeps test infrastructure current with upstream `axum` 0.8 ecosystem |
| Leverages strengths? | Yes | Trivial infrastructure maintenance with clear blast radius |
| Meets real need? | Yes | Dependabot flagged the update; staying current reduces future security/compat debt |

**Proceed**: Yes.

## Problem Statement

### Description
`axum-test` 19.x depends on older internal crates and lags behind `axum` 0.8.x used elsewhere. 21.x brings the test helper in line with the current `axum` minor.

### Impact
Only test authors and CI consumers of the server/validation integration suites are affected.

### Success Criteria
- `cargo test -p terraphim_server` and `cargo test -p terraphim_validation` compile and pass.
- No production code changes.
- `Cargo.lock` diff is limited to the intended dependency graph.

## Current State Analysis

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Server API test harness | `crates/terraphim_validation/src/testing/server_api/harness.rs` | Wraps `axum_test::TestServer` for validation tests |
| Ollama API tests | `terraphim_server/tests/ollama_api_test.rs` | Uses `axum_test::TestServer` directly |
| Agent web flow tests | `terraphim_server/tests/agent_web_flows_test.rs` | Uses `axum_test::TestServer` directly |
| Workflow E2E tests | `terraphim_server/tests/workflow_e2e_tests.rs` | Uses `axum_test::TestServer` directly |
| Validation manifest | `crates/terraphim_validation/Cargo.toml:76` | `axum-test = "19.1.1"` |
| Server manifest | `terraphim_server/Cargo.toml:94` | `axum-test = "19"` |

### Data Flow
Tests build an `axum` router and wrap it in `TestServer`, then issue HTTP requests via `.get()`, `.post()`, `.put()`, `.delete()` and assert on `TestResponse` status/body.

## Constraints

- `axum-test` is a dev/test dependency only.
- Source API used here is the stable `TestServer`/`TestResponse` surface.
- `axum` 0.8.x is already the production server version.

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking API change in `TestServer::new` or request builders | Low | Medium (fails compilation) | Run focused `cargo test -p terraphim_server -p terraphim_validation` after merge |
| WebSocket-related comment becomes stale | Very low | Low | Comment already states `axum-test` doesn't support WebSockets; still true in 21.x |
| New transitive crates `educe`/`enum-ordinalize` cause audit noise | Low | Low | They are standard proc-macro helper crates pulled by `axum-test` |

## Recommendations

### Proceed/No-Proceed
Proceed.

### Scope
- In scope: merge the cleaned PR after CI passes.
- Out of scope: refactoring tests, changing production code, additional dependency bumps.

## Next Steps
1. Merge PR #937.
2. Monitor `cargo test -p terraphim_server -p terraphim_validation` on main.
3. If any test API breaks, file a follow-up bug fix.
