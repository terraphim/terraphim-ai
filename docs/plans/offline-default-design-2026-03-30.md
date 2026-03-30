# Implementation Plan: Make Offline Mode Default for terraphim-agent TUI

**Status**: Draft
**Research Doc**: `docs/plans/offline-default-research-2026-03-30.md`
**Supersedes**: `docs/plans/terraphim-agent-tui-offline-server-design-2026-02-13.md` (that plan chose to keep TUI server-backed; this plan makes offline the default)
**Author**: AI Agent
**Date**: 2026-03-30
**Estimated Effort**: 1-2 days

## Overview

### Summary

Make `terraphim-agent` (no arguments) work without a running `terraphim_server` by refactoring the fullscreen TUI to use `TuiService` (local, offline) by default. The `--server` flag opts into the existing server-backed behaviour.

### Approach

1. Define a `TuiBackend` enum that wraps either `TuiService` or `ApiClient`
2. Replace all direct `ApiClient` calls in `ui_loop()` with calls through `TuiBackend`
3. Change the default dispatch to create `TuiService`; use `ApiClient` only when `--server` is specified
4. Remove the health check gate for the default (offline) path

### Scope

**In Scope:**
- Refactor `ui_loop()` to work with either `TuiService` or `ApiClient`
- Change default (no-arg) behaviour from server-required to offline-first
- Add `TuiBackend` abstraction with unified method signatures
- Fix misleading function names (`run_tui_offline_mode`)
- Add loading indicator during `TuiService` initialisation

**Out of Scope:**
- Removing server mode (`--server` remains fully functional)
- Changing the REPL or subcommand paths
- Modifying `terraphim_server`
- Adding new features beyond offline TUI
- Changing `terraphim-cli` device settings

**Avoid At All Cost** (from 5/25 analysis):
- Async TUI rewrite (ratatui is synchronous; accept the `block_on` bridge)
- Trait-based abstraction for every possible API call (YAGNI; use an enum)
- Adding WebSocket/workflow features to offline TUI
- Feature flags for offline/online modes (unconditional; both paths always compiled)
- Changing CLI subcommand default behaviour (already offline; leave alone)

## Architecture

### Component Diagram

```
CLI Entry (main.rs)
    |
    +-- no args / interactive (default) --> TuiService (offline)
    |       |
    |       +-- --server flag -----------> ApiClient (online)
    |
    +-- repl (default) ------------------> TuiService (offline)
    |       |
    |       +-- --server flag -----------> ApiClient (online)
    |
    +-- subcommands (default) -----------> TuiService (offline)
            |
            +-- --server flag -----------> ApiClient (online)
```

### Data Flow: New Default (Offline TUI)

```
terraphim-agent (no args)
  -> run_tui_with_backend(backend=Offline)
    -> TuiService::new(None)           [loads config, builds rolegraphs]
    -> run_tui(TuiBackend::Local(service), transparent)
      -> ui_loop(terminal, backend, transparent)
        -> backend.search()             [local TerraphimService]
        -> backend.autocomplete()       [local automata]
        -> backend.rolegraph()          [local RoleGraph]
```

### Data Flow: Server Mode (unchanged, opt-in)

```
terraphim-agent --server (no args)
  -> run_tui_with_backend(backend=Server, url)
    -> ApiClient::new(url)
    -> ensure_tui_server_reachable()   [health check]
    -> run_tui(TuiBackend::Remote(api), transparent)
      -> ui_loop(terminal, backend, transparent)
        -> backend.search()             [HTTP to server]
        -> backend.autocomplete()       [HTTP to server]
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use `enum TuiBackend` instead of trait | Simpler, no dynamic dispatch, both variants always compiled | Trait object (`Box<dyn TuiBackend>`) -- over-engineered for 2 variants |
| Move `TuiService::new()` before `run_tui()` | Avoids raw-mode terminal during potentially slow init | Loading screen in raw mode -- more complex, worse UX |
| Keep `--server` flag semantics | Already understood by users, consistent across TUI/REPL/subcommands | New `--offline` flag -- adds cognitive load for the default case |
| Map return types in `TuiBackend` methods | `TuiService` and `ApiClient` return different types; adapter in the enum | Change TuiService return types -- would break REPL and subcommands |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Trait object abstraction | Only 2 implementations; enum is simpler and zero-cost | Unnecessary indirection, harder to debug |
| Async TUI framework | ratatui works; `block_on` bridge is proven in existing code | Complete rewrite, massive regression risk |
| Auto-detect server fallback | Adds network call on every startup; complexity for edge case | Slow startup, confusing error paths |
| Separate `--offline` flag | Default should just work; no flag needed for the happy path | Flag proliferation, user confusion |

### Simplicity Check

**What if this could be easy?** The simplest change: create a `TuiBackend` enum with two variants. Each method matches on the variant and calls the appropriate inner type. The `ui_loop` function takes `TuiBackend` instead of `ApiClient`. Done.

**Senior Engineer Test**: Would a senior engineer call this overcomplicated? No. An enum with 2 variants matching on methods is the minimal abstraction.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_agent/src/tui_backend.rs` | `TuiBackend` enum bridging TuiService and ApiClient |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/main.rs` | Refactor `ui_loop()` to use `TuiBackend`; fix dispatch; rename misleading functions |
| `crates/terraphim_agent/src/lib.rs` (or `mod.rs`) | Add `mod tui_backend;` |

### Deleted Files

| File | Reason |
|------|--------|
| None | N/A |

## API Design

### TuiBackend Enum

```rust
use anyhow::Result;
use terraphim_types::{Document, SearchQuery, RoleName};

pub enum TuiBackend {
    Local(crate::service::TuiService),
    Remote(crate::client::ApiClient),
}

impl TuiBackend {
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>> {
        match self {
            Self::Local(svc) => svc.search_with_query(query).await,
            Self::Remote(api) => {
                let resp = api.search(query).await?;
                Ok(resp.results)
            }
        }
    }

    pub async fn get_config(&self) -> Result<terraphim_config::Config> {
        match self {
            Self::Local(svc) => Ok(svc.get_config().await),
            Self::Remote(api) => {
                let resp = api.get_config().await?;
                Ok(resp.config)
            }
        }
    }

    pub async fn get_rolegraph_terms(&self, role: &str) -> Result<Vec<String>> {
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role);
                svc.get_role_graph_top_k(&role_name, 50).await
            }
            Self::Remote(api) => {
                let resp = api.rolegraph(Some(role)).await?;
                Ok(resp.nodes.into_iter().map(|n| n.label).collect())
            }
        }
    }

    pub async fn autocomplete(
        &self,
        role: &str,
        query: &str,
    ) -> Result<Vec<String>> {
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role);
                let results = svc.autocomplete(&role_name, query, Some(5)).await?;
                Ok(results.into_iter().map(|r| r.term).collect())
            }
            Self::Remote(api) => {
                let resp = api.get_autocomplete(role, query).await?;
                Ok(resp.suggestions.into_iter().map(|s| s.text).collect())
            }
        }
    }

    pub async fn summarize(
        &self,
        document: &Document,
        role: Option<&str>,
    ) -> Result<Option<String>> {
        match self {
            Self::Local(svc) => {
                let role_name = RoleName::new(role.unwrap_or("Terraphim Engineer"));
                let summary = svc.summarize(&role_name, &document.body).await?;
                Ok(Some(summary))
            }
            Self::Remote(api) => {
                let resp = api.summarize_document(document, role).await?;
                Ok(resp.summary)
            }
        }
    }

    pub async fn switch_role(
        &self,
        role: &str,
    ) -> Result<terraphim_config::Config> {
        match self {
            Self::Local(svc) => {
                svc.update_selected_role(RoleName::new(role)).await
            }
            Self::Remote(api) => {
                let resp = api.update_selected_role(role).await?;
                Ok(resp.config)
            }
        }
    }
}
```

### Error Types

No new error types needed. All methods return `anyhow::Result`. Existing errors from `TuiService` and `ApiClient` propagate naturally.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_tui_backend_local_search` | `tui_backend.rs` | Offline search returns documents |
| `test_tui_backend_local_autocomplete` | `tui_backend.rs` | Offline autocomplete returns strings |
| `test_tui_backend_local_rolegraph` | `tui_backend.rs` | Offline rolegraph returns term labels |
| `test_tui_backend_local_get_config` | `tui_backend.rs` | Offline config returns Config struct |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_no_args_offline_no_server` | `tests/tui_offline_tests.rs` | `terraphim-agent` works without server |
| `test_server_flag_requires_server` | `tests/tui_offline_tests.rs` | `--server` still requires running server |
| `test_offline_search_returns_results` | `tests/tui_offline_tests.rs` | Full search flow works offline |

### Manual Acceptance Checks

1. `terraphim-agent` (no args) opens fullscreen TUI without server running
2. Search, autocomplete, role switching, summarize all work in offline TUI
3. `terraphim-agent --server` still requires server and shows clear error if missing
4. `terraphim-agent repl` unchanged behaviour
5. `terraphim-agent search "test"` unchanged behaviour (already offline)

## Implementation Steps

### Step 1: Create TuiBackend Enum
**Files:** `crates/terraphim_agent/src/tui_backend.rs`
**Description:** Implement the `TuiBackend` enum with all methods needed by `ui_loop()`
**Tests:** Unit tests for all methods using `TuiService` variant
**Estimated:** 3 hours

### Step 2: Refactor ui_loop() to Use TuiBackend
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Replace `ApiClient` parameter with `TuiBackend` in `ui_loop()`. Update all `api.*` calls to `backend.*` calls.
**Tests:** Compile check; manual TUI test
**Dependencies:** Step 1
**Estimated:** 2 hours

### Step 3: Fix Main Dispatch
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Change dispatch at lines 921-967 to create `TuiService` by default. Move `TuiService::new()` before `run_tui()` to avoid raw-mode loading. Rename `run_tui_offline_mode` to `run_tui_local_mode`. Remove health check for offline path.
**Tests:** Integration test: no-arg invocation succeeds without server
**Dependencies:** Step 2
**Estimated:** 2 hours

### Step 4: Add Loading Indicator
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Print "Loading..." to stdout before entering raw mode. Clear after `TuiService` init completes. This handles the case where rolegraph building takes >500ms.
**Tests:** Manual visual check
**Dependencies:** Step 3
**Estimated:** 1 hour

### Step 5: Tests and Cleanup
**Files:** `crates/terraphim_agent/tests/tui_offline_tests.rs`
**Description:** Integration tests for offline TUI. Remove dead code. Fix any clippy warnings.
**Tests:** `cargo test -p terraphim_agent`, `cargo clippy -p terraphim_agent`
**Dependencies:** Step 4
**Estimated:** 2 hours

## Rollback Plan

If issues are found:
1. Revert `ui_loop()` changes -- restore `ApiClient`-based TUI
2. Keep `TuiBackend` enum (dead code, no runtime impact)
3. Default behaviour returns to server-required

Feature flag approach: not needed. The `--server` flag IS the feature flag.

## Dependencies

### New Dependencies

None. All types (`TuiService`, `ApiClient`, domain types) already exist.

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| TUI startup (offline) | < 2s | Wall clock from invocation to interactive |
| TUI startup (server) | No regression | Same as current |
| Search latency (offline) | < 100ms (in-memory) | Compared to server HTTP round-trip |
| Memory (offline) | < 100MB | Acceptable for local tool |

### Key Performance Notes

- `TuiService::new()` builds rolegraphs and loads automata. This happens before raw mode, so no TUI jank.
- Offline search is in-memory (no HTTP overhead) -- should be faster than server mode.
- `autocomplete()` in TuiService builds an index each call. If slow, cache the index per role (out of scope for this plan).

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify TuiService::autocomplete return type maps cleanly to Vec<String> | Pending (needs AutocompleteResult struct check) | Implementer |
| Confirm TuiService init time with real config | Pending | Implementer |
| Update docs/plans/terraphim-agent-tui-offline-server-design-2026-02-13.md status to "Superseded" | Pending | Implementer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
