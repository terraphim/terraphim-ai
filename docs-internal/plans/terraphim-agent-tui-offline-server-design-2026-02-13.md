# Implementation Plan: Clarify `terraphim-agent` TUI Offline/Server Requirement (`terraphim-ai-cbm`)

**Status**: Draft
**Research Doc**: `docs/plans/terraphim-agent-tui-offline-server-research-2026-02-13.md`
**Author**: Codex
**Date**: 2026-02-13
**Estimated Effort**: 0.5-1.0 day

## Overview

### Summary

This plan makes runtime behavior explicit and predictable without changing core architecture: fullscreen interactive TUI remains server-backed, while REPL/offline subcommands remain offline-capable. The implementation focuses on clarity in user messaging, docs alignment, and tests that prevent behavior drift.

### Approach

1. Define a single mode contract and reflect it consistently in CLI help and documentation.
2. Add explicit, actionable failure messaging when fullscreen TUI cannot reach server.
3. Backstop with tests for mode-specific behavior and messaging.

### Scope

**In Scope:**
- Clarify mode contract in help text and docs.
- Improve fullscreen TUI startup behavior/messaging for missing server.
- Add tests that validate the contract and avoid regressions.

**Out of Scope:**
- Full offline rewrite of fullscreen ratatui TUI.
- `terraphim-cli` device settings fallback (issue `terraphim-ai-2sz`).
- Feature-parity redesign between `terraphim-agent` and `terraphim-cli` (issue `terraphim-ai-tcw`).

**Avoid At All Cost**:
- Ambiguous terms where "TUI" and "REPL" are interchangeable.
- Broad architecture refactors hidden inside this clarification task.
- Breaking default automation/subcommand behavior.

## Architecture

### Component Diagram
```
CLI Entry (main.rs)
    |
    +-- no args / interactive --> Fullscreen TUI (ratatui) --> ApiClient --> Server endpoints
    |
    +-- repl ------------------> ReplHandler::new_offline/new_server
    |                               |                 |
    |                               |                 +--> ApiClient
    |                               +--> TuiService (embedded/local)
    |
    +-- subcommands -----------> run_offline_command (TuiService) OR run_server_command (ApiClient)
```

### Data Flow
```
User input -> mode routing -> [ApiClient path OR TuiService path] -> output/messaging
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Keep fullscreen interactive TUI server-backed | Matches current implementation and minimizes risk | Full offline fullscreen rewrite |
| Keep REPL/subcommands offline-capable | Existing behavior and docs promise local-first operation | Force everything through server |
| Add explicit startup guidance for server-unreachable fullscreen TUI | Faster operator diagnosis, less confusion | Silent/implicit failure behavior |
| Normalize wording across help/docs | Root-cause fix for ambiguity | Patch one doc only |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Implement offline data provider in fullscreen TUI now | Too large for issue scope | Schedule slip, regression risk |
| Change no-arg behavior to REPL by default in same patch | Potentially breaking UX contract without sign-off | User-facing behavior break |
| Introduce new abstraction layer for all modes | Speculative complexity | Harder maintenance |

### Simplicity Check

The simplest change that solves the problem is contract clarity, not runtime redesign.

**Senior Engineer Test**: Passes.
**Nothing Speculative Checklist**:
- [x] No features user did not request
- [x] No "future-proof" abstractions
- [x] No speculative flexibility layers
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `docs/plans/terraphim-agent-tui-offline-server-design-2026-02-13.md` | Phase 2 design plan |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/main.rs` | Clarify no-arg interactive mode wording; add actionable server-requirement messaging for fullscreen TUI path |
| `docs/tui-usage.md` | Add explicit mode matrix (fullscreen TUI vs REPL vs subcommands) |
| `docs/src/tui.md` | Correct "Interactive REPL Mode" wording and align usage examples with real routing |
| `README.md` | Ensure usage snippets do not imply fullscreen TUI is offline-capable |
| `crates/terraphim_agent/tests/*` | Add/update tests for mode-specific behavior and messaging |

### Deleted Files

| File | Reason |
|------|--------|
| None | N/A |

## API Design

### Internal Functions (proposed)

```rust
/// Resolve server URL for fullscreen TUI mode.
fn resolve_tui_server_url(explicit: Option<&str>) -> String;

/// Preflight server availability for fullscreen TUI and return actionable error.
async fn ensure_tui_server_reachable(api: &ApiClient) -> anyhow::Result<()>;

/// Build user-facing guidance when fullscreen TUI cannot reach server.
fn tui_server_requirement_error(url: &str, cause: &anyhow::Error) -> anyhow::Error;
```

### Error States

```text
TUI_SERVER_UNREACHABLE:
  Fullscreen TUI cannot reach server URL.
  Message includes:
  - URL attempted
  - suggestion: start server OR use `terraphim-agent repl` for offline mode
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `resolve_tui_server_url_uses_explicit_then_env_then_default` | `crates/terraphim_agent/src/main.rs` (or extracted module) | Validate deterministic URL selection |
| `tui_server_requirement_error_mentions_repl_fallback` | `crates/terraphim_agent/src/main.rs` (or extracted module) | Ensure actionable operator guidance |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `offline_subcommand_config_show_still_works_without_server` | `crates/terraphim_agent/tests/offline_mode_tests.rs` | Preserve offline command contract |
| `server_mode_config_show_fails_cleanly_when_server_down` | `crates/terraphim_agent/tests/server_mode_tests.rs` (or new targeted test) | Validate failure path messaging |
| `help_text_describes_mode_contract` | `crates/terraphim_agent/tests/integration_tests.rs` (or new) | Prevent future ambiguity |

### Manual Acceptance Checks

1. `terraphim-agent config show` works with no server running.
2. `terraphim-agent --server config show` fails with clear server connection error.
3. `terraphim-agent` no-arg usage/help text clearly distinguishes fullscreen TUI vs `repl`.
4. Docs contain a single consistent mode contract.

## Implementation Steps

### Step 1: Contract Wording Baseline
**Files:** `crates/terraphim_agent/src/main.rs`, `README.md`, `docs/tui-usage.md`, `docs/src/tui.md`
**Description:** Define and apply one authoritative mode matrix and terminology (`Fullscreen TUI`, `REPL`, `CLI subcommands`).
**Tests:** Help output checks.
**Estimated:** 1.5 hours.

### Step 2: Fullscreen TUI Startup Messaging
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Add preflight/failure guidance so missing server is explicit and points to `repl` fallback.
**Tests:** Unit tests for URL resolution and error message composition.
**Dependencies:** Step 1.
**Estimated:** 2 hours.

### Step 3: Regression Tests
**Files:** `crates/terraphim_agent/tests/offline_mode_tests.rs`, `crates/terraphim_agent/tests/server_mode_tests.rs`, optional new test file
**Description:** Lock expected behavior for offline subcommands and server-backed paths.
**Tests:** `cargo test -p terraphim_agent offline_mode_tests server_mode_tests`.
**Dependencies:** Step 2.
**Estimated:** 2 hours.

### Step 4: Final Documentation Sync
**Files:** `README.md`, `docs/tui-usage.md`, `docs/src/tui.md`
**Description:** Ensure examples and requirements reflect final runtime behavior and fallback guidance.
**Tests:** Markdown lint/docs build if available.
**Dependencies:** Step 3.
**Estimated:** 1 hour.

## Rollback Plan

If issues are found:
1. Revert messaging/preflight additions in `main.rs`.
2. Keep documentation-only clarifications while restoring prior runtime behavior.
3. Reopen follow-up issue for architectural decisions beyond clarification scope.

## Dependencies

### New Dependencies

None planned.

### Dependency Updates

None planned.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Offline subcommand startup | No regression | Existing test execution time comparison |
| Fullscreen TUI startup | Negligible added overhead | Optional preflight (single config call) |

### Benchmarks to Add

None required for this change class.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Decide whether no-arg should remain fullscreen TUI or switch to REPL default in future | Pending approval | Maintainers |
| Confirm whether fullscreen TUI server dependency is long-term product direction | Pending | Maintainers |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
