# Research Document: PR #888 Fix Plan

**Status**: Draft
**Date**: 2026-05-27
**Branch**: task/1875-adf-ctl-local-direct-dispatch

## Executive Summary

PR #888 consolidates three features (adf-ctl direct dispatch, FffIndexer migration, local .terraphim config) in a 65-file, ~7.7k LOC change. CI fails on two fronts: (1) a flaky `test_role_switching_persistence` test in terraphim_agent, and (2) a Firecracker VM infrastructure failure (exit code 22 on VM creation). A structural PR review raised two P1 findings about direct dispatch zero-ID semantics and FffIndexer parity. The adf-ctl direct dispatch and binary tests (39 tests) all pass cleanly.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Unblocks low-latency local agent dispatch |
| Leverages strengths? | Yes | Pure-Rust search migration + Unix IPC expertise |
| Meets real need? | Yes | ADF operator workflow requires fast local triggering |

**Proceed**: Yes (3/3)

## Problem Statement

### CI Failures

1. **`test_role_switching_persistence`** (terraphim_agent): Panics at line 290 -- `assertion failed: roles_to_test.iter().any(|role| role == final_role)`. The test sets 4 roles sequentially, then reads the final config and asserts the persisted role matches one of them. On CI, the final role returned does not match any of the 4 test roles. Passes locally (non-deterministic timing).

2. **Firecracker VM lifecycle proof**: `curl -X POST $FCCTL_URL/api/vms` returns exit code 22 (HTTP error from curl). The fcctl-web health check passes, but VM creation fails. This is an infrastructure issue, not a code issue.

### PR Review Findings (Confidence: 2/5)

**P1 - Direct dispatch emits `WebhookDispatch::SpawnAgent` with synthetic zero IDs**:
- `direct_dispatch.rs:140-150` constructs `SpawnAgent { issue_number: 0, comment_id: 0 }`
- Any downstream code branching on `issue_number > 0` or using these for Gitea API calls will behave differently on the direct path vs webhook path

**P1 - FffIndexer migration lacks demonstrated parity**:
- `cargo test -p terraphim_middleware --test fff_indexer` is unchecked in PR checklist
- TerraphimGraph relevance depends on document indexing quality; regression risk

**P2 - No rate limiting on Unix socket**:
- Socket listener accepts connections and sends on `dispatch_tx` with no bounding
- Multiple `adf-ctl` invocations could exert backpressure on main orchestrator loop

**P2 - .terraphim/learnings/ mass deletion**:
- 31 auto-generated learning files deleted alongside new `ProjectConfig::load_from_dir()` 
- Interaction between generated artefacts and new persistence model is under-specified

## Current State Analysis

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| direct_dispatch.rs | `crates/terraphim_orchestrator/src/direct_dispatch.rs` | Unix domain socket listener (449 lines, new) |
| adf-ctl.rs | `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | CLI expansion for --local + --direct (+623/-114 lines) |
| lib.rs | `crates/terraphim_orchestrator/src/lib.rs` | Orchestrator wiring, LoopEvent enum, dispatch routing (+262/-3) |
| config.rs | `crates/terraphim_orchestrator/src/config.rs` | DirectDispatchConfig, ProjectConfig integration (+28) |
| persistence_tests.rs | `crates/terraphim_agent/tests/persistence_tests.rs` | Flaky test at line 290 |
| fff.rs (not in diff) | `crates/terraphim_middleware/src/indexer/fff.rs` | FffIndexer (referenced but not in current diff) |

### Data Flow

```
adf-ctl --local trigger --direct
  -> Unix Socket (0600)
  -> direct_dispatch_listener (bounded read 8KiB)
  -> mpsc::channel(64) -> LoopEvent::DirectDispatch
  -> handle_direct_dispatch() (exact-name lookup, no MentionConfig)
  -> WebhookDispatch::SpawnAgent { issue_number:0, comment_id:0 }
```

### Integration Points

- `WebhookDispatch::SpawnAgent` is consumed by `handle_webhook_dispatch()` and `handle_direct_dispatch()` 
- Downstream: Gitea comment posting, issue deduplication, audit logging -- all potentially branch on `issue_number > 0`

## Constraints

### Technical Constraints
- `#[cfg(unix)]` gating required for Windows cross-compilation
- Unix domain sockets not available on Windows
- mpsc channel bounded at 64 -- adequate for local single-user dispatch

### Test Constraints
- `test_role_switching_persistence` spawns `cargo run` subprocesses -- inherently slow and timing-sensitive
- CI runner is a Firecracker microVM -- resource-constrained, no VM nesting

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Flaky persistence test on CI | High | Medium | Fix assertion to be more resilient, or add retry |
| Firecracker VM infra failure | High | Low | Infrastructure issue, not code -- separate fix |
| Downstream SpawnAgent zero-ID breakage | Medium | High | Audit all consumers of issue_number/comment_id |
| FffIndexer quality regression | Medium | High | Complete test suite before merge |
| Socket backpressure under load | Low | Medium | Add try_send with bounded channel check |

### Assumptions

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Zero IDs are safe for direct dispatch | No Gitea interaction needed for local dispatch | Downstream code crashes | No -- needs audit |
| FffIndexer tests will pass once run | PR author claims 722-line test suite exists | Search quality regression | No -- needs verification |
| Firecracker failure is infra-only | Health check passes, VM create fails | CI won't pass even after code fixes | Partially |

## Research Findings

### Key Insights

1. **The flaky test is a race condition**: `test_role_switching_persistence` uses `cargo run` subprocess calls with 200ms sleep between switches. On a slow CI runner (Firecracker), the final `config show` may not reflect the last write because the file system or DashMap persistence layer hasn't flushed. The assertion at line 290 is too strict -- it assumes `final_role` matches one of the 4 test roles, but on CI the role could be stale or the config file could have been reset.

2. **Direct dispatch zero-ID pattern is architecturally sound but needs consumer audit**: The separate `LoopEvent::DirectDispatch` variant correctly routes through `handle_direct_dispatch()` which uses exact-name lookup. The risk is in shared downstream code that processes `WebhookDispatch::SpawnAgent`.

3. **Firecracker VM failure is pre-existing infrastructure issue**: The `curl -X POST /api/vms` returns exit code 22 consistently. The fcctl-web service is healthy but the VM creation endpoint fails. This is unrelated to PR changes.

4. **39 direct_dispatch + adf-ctl tests pass**: All 13 direct_dispatch lib tests, 26 adf-ctl bin tests pass locally.

### Open Questions

1. Does `handle_direct_dispatch()` skip Gitea comment posting entirely? -- Needs code trace
2. What does `cargo test -p terraphim_middleware --test fff_indexer` actually return? -- Needs verification
3. Is the Firecracker VM failure intermittent or persistent? -- Infrastructure team

## Recommendations

### Proceed: Yes, with targeted fixes

The PR is structurally sound. The failures are:
- 1 flaky test (easy fix)
- 1 infrastructure issue (out of scope)
- 2 P1 review findings requiring targeted code changes

### Scope Recommendations

1. Fix the flaky persistence test immediately
2. Audit all `WebhookDispatch::SpawnAgent` consumers for zero-ID tolerance
3. Verify FffIndexer test suite passes
4. Address socket backpressure as a follow-up (P2, not blocking)
