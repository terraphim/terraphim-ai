# Research Document: Make Offline Mode Default for terraphim-agent

**Date**: 2026-03-30
**Author**: AI Agent (Phase 1 Research)
**Status**: Draft

## 1. Problem Restatement and Scope

### Problem

When a user runs `terraphim-agent` with no arguments, the default behaviour is to launch the fullscreen ratatui TUI, which **unconditionally requires a running `terraphim_server` at `http://localhost:8000`**. If the server is not running, the user gets an opaque error:

```
Fullscreen TUI requires a running Terraphim server at http://localhost:8000.
Start terraphim_server or use offline mode with `terraphim-agent repl`.
Connection error: health check failed
```

This is the **first experience** most users have with the tool. The error message is technically accurate but user-hostile: it forces the user to either (a) start a separate long-running server process first, or (b) know to use `terraphim-agent repl` instead.

The fundamental problem: **the default invocation of terraphim-agent does not work without infrastructure the user may not have started**.

### Scope

**IN scope:**
- Making `terraphim-agent` (no args) work without a running server
- Determining the appropriate default mode when no server is available
- Refactoring the fullscreen TUI to use the existing `TuiService` (offline path) instead of requiring `ApiClient` (online path)
- Providing graceful fallback or auto-detection when the server IS available

**OUT of scope:**
- Removing server mode entirely (server-backed TUI still useful for remote/shared scenarios)
- Changing `terraphim_server` itself
- Modifying the REPL or subcommand paths (they already support offline mode)
- Adding new features beyond making existing offline capabilities the default
- Changing the `terraphim-cli` device settings fallback

### Distinguishing Problem from Solution Guesses

**Problem statements:**
- "Running `terraphim-agent` without arguments fails when no server is running"
- "The fullscreen TUI depends on a server that most users don't have running"
- "TuiService provides all the same capabilities locally but is not wired into the TUI"

**Solution guesses (flagged, not pursued yet):**
- "Make the TUI use TuiService by default" -- likely direction but needs design validation
- "Auto-detect server and fall back to offline" -- option to evaluate
- "Change default to REPL" -- simpler but loses the fullscreen experience

## 2. User and Business Outcomes

### User-Visible Changes

| Current Behaviour | Desired Behaviour |
|---|---|
| `terraphim-agent` fails with server error | `terraphim-agent` works immediately, no server needed |
| User must start `terraphim_server` first | User can use the tool immediately after installation |
| Fullscreen TUI requires server | Fullscreen TUI works standalone |
| Error message says "use repl" as workaround | No workaround needed; default mode just works |

### Business Value

- **First-run experience**: New users succeed immediately, no setup friction
- **Single-binary simplicity**: `terraphim-agent` becomes self-contained for local use
- **Power-user option**: `--server` flag preserves the server-backed mode for those who need it
- **Competitive positioning**: Most search/KG tools work standalone; requiring a server is an unnecessary barrier

## 3. System Elements and Dependencies

### 3.1 Key Components

| Component | Location | Role | Dependencies |
|---|---|---|---|
| `Cli` struct | `crates/terraphim_agent/src/main.rs:537-564` | Clap CLI argument parsing | clap |
| `Command` enum | `crates/terraphim_agent/src/main.rs:566-758` | Subcommand dispatch | clap |
| `run_tui()` | `crates/terraphim_agent/src/main.rs:2881-2940` | Fullscreen TUI terminal setup | ratatui, crossterm |
| `ui_loop()` | `crates/terraphim_agent/src/main.rs:2942-3238` | TUI event loop -- **entirely ApiClient-based** | ApiClient |
| `ApiClient` | `crates/terraphim_agent/src/client.rs` | HTTP client for server API | reqwest |
| `TuiService` | `crates/terraphim_agent/src/service.rs:11-15` | Local service (no network) | TerraphimService, ConfigState |
| `TerraphimService` | `crates/terraphim_service/src/` | Core search/KG/chat engine | All domain crates |
| `ReplHandler` | `crates/terraphim_agent/src/repl/handler.rs:25-58` | REPL with offline/server modes | TuiService or ApiClient |
| `terraphim_server` | `terraphim_server/src/` | Axum HTTP API server | TerraphimService |
| `ensure_tui_server_reachable()` | `crates/terraphim_agent/src/main.rs:96-104` | Health check gate | ApiClient |
| `tui_server_requirement_error()` | `crates/terraphim_agent/src/main.rs:86-94` | Error message builder | -- |

### 3.2 Data Flow: Current (Server-Required TUI)

```
terraphim-agent (no args)
  -> run_tui_offline_mode()       [misleading name!]
    -> run_tui(None, transparent)
      -> ui_loop(terminal, None, transparent)
        -> resolve_tui_server_url(None) -> "http://localhost:8000"
        -> ApiClient::new("http://localhost:8000")
        -> ensure_tui_server_reachable()  <-- FAILS HERE if no server
        -> [TUI event loop using api.search(), api.rolegraph(), etc.]
```

### 3.3 Data Flow: REPL Offline (Already Works)

```
terraphim-agent repl
  -> run_repl_offline_mode()
    -> TuiService::new(None)      [loads config, builds rolegraphs locally]
    -> ReplHandler::new_offline(service)
    -> [REPL event loop using service.search_with_role(), etc.]
```

### 3.4 TuiService Capability Surface

The `TuiService` at `service.rs` already provides ALL the capabilities the TUI needs:

| Capability | TuiService Method | ApiClient Method (current TUI) |
|---|---|---|
| Search | `search_with_role()` | `api.search()` |
| Get config | `get_config()` | `api.get_config()` |
| List roles | `list_roles_with_info()` | (from config) |
| Get rolegraph | `get_role_graph_top_k()` | `api.rolegraph()` |
| Autocomplete | `autocomplete()` | `api.get_autocomplete()` |
| Summarize | `summarize()` | `api.summarize_document()` |
| Chat | `chat()` | (via REPL only) |

The `TuiService` is already used by:
- `run_offline_command()` for all CLI subcommands
- `ReplHandler::new_offline()` for the REPL

**It is NOT used by the fullscreen TUI (`ui_loop`).** This is the core gap.

### 3.5 Shared State and Cross-Cutting Concerns

- **Config loading**: Same priority chain (CLI flag -> settings.toml -> persistence -> embedded defaults)
- **Tokio runtime**: TUI creates its own runtime; TuiService is async-capable
- **Thread safety**: `TuiService` uses `Arc<Mutex<TerraphimService>>` -- safe to use from TUI event loop via `rt.block_on()`
- **Logging**: Both paths use `terraphim_service::logging::init_logging()`

## 4. Constraints and Their Implications

### 4.1 UX Constraints

| Constraint | Why It Matters | Good vs Bad Solution |
|---|---|---|
| First-run must work | User retention depends on immediate value | Good: works without server. Bad: requires setup guide |
| No breaking change for server users | Existing workflows depend on `--server` | Good: `--server` still works. Bad: removing server mode |
| TUI must remain responsive | Terminal UI should not block on disk I/O | Good: async service calls. Bad: synchronous disk reads in render loop |
| Backward-compatible CLI flags | Scripts may use current flags | Good: keep `--server` semantics. Bad: repurpose flags |

### 4.2 Technical Constraints

| Constraint | Why It Matters | Good vs Bad Solution |
|---|---|---|
| TuiService is async | TUI event loop is synchronous (`block_on` bridge needed) | Good: reuse existing `rt.block_on()` pattern. Bad: rewrite TUI as async |
| ratatui is synchronous | Cannot use `.await` in render closures | Good: pre-fetch data before render. Bad: async in render |
| Search may be slow on first load | Building rolegraphs and indexing takes time | Good: show loading state. Bad: blank screen |
| Memory usage | Loading all rolegraphs into memory is heavier than HTTP client | Good: acceptable for local use. Bad: unnecessary for server users |

### 4.3 Performance Constraints

| Constraint | Why It Matters | Implication |
|---|---|---|
| TUI startup time | Users expect instant feedback | May need splash/loading screen during TuiService init |
| Search latency | TUI renders results synchronously | Local search should be fast (in-memory), but first query builds automata |
| Memory footprint | Server mode shares indexed data; offline loads per-process | Acceptable for single-user local tool |

### 4.4 Unclear/Conflicting Constraints

- **"offline" naming**: The function `run_tui_offline_mode` does NOT actually run offline. This naming must be fixed to avoid confusion.
- **Default behaviour change**: Changing the default from server-required to offline-first is a behaviour change, but since the current default FAILS for most users, this is arguably a fix rather than a break.

## 5. Risks, Unknowns, and Assumptions

### 5.1 Unknowns

| Unknown | Impact | How to Resolve |
|---|---|---|
| How long does TuiService::new() take on a typical config? | If slow (>2s), needs loading UI | Benchmark with real config |
| Does the TUI autocomplete path work identically via TuiService vs ApiClient? | UX consistency | Compare REPL autocomplete (TuiService) with TUI autocomplete (ApiClient) |
| Are there server-only features the TUI relies on that TuiService lacks? | Feature parity gaps | Audit all `api.*` calls in ui_loop against TuiService methods |
| What happens with WebSocket/workflow features in offline TUI? | Some features may be server-only | Check if TUI uses any WebSocket routes |

### 5.2 Assumptions

1. **ASSUMPTION**: TuiService provides all capabilities needed by the fullscreen TUI. The capability table in section 3.4 supports this, but the exact return types need verification.
2. **ASSUMPTION**: The `rt.block_on()` pattern used for ApiClient calls in `ui_loop` will work identically for TuiService async methods. Both return `Result` types.
3. **ASSUMPTION**: Users who currently run both `terraphim_server` and `terraphim-agent` will accept using `--server` to maintain their workflow.
4. **ASSUMPTION**: The `summarize()` method in TuiService (which uses chat/LLM) will work in the TUI context the same as the server's summarize endpoint.

### 5.3 Risks

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| TuiService init is slow, causing blank TUI on startup | Medium | Medium | Add loading screen or init before entering raw mode |
| Return types differ between ApiClient and TuiService methods | Medium | Low | Create adapter/wrapper with unified interface |
| Memory usage significantly higher than server mode | Low | Low | Profile and compare; acceptable for local tool |
| Users rely on current "no args = server" behaviour | Low | Very Low | Current behaviour fails without server; improvement is net positive |
| Autocomplete UX differs between local and server | Medium | Medium | TuiService.autocomplete() returns different type than ApiClient.get_autocomplete() -- needs adapter |

## 6. Context Complexity vs. Simplicity Opportunities

### 6.1 Sources of Complexity

1. **Dual service abstraction**: The codebase has two parallel service paths (`ApiClient` and `TuiService`) that do the same things but with different interfaces. The TUI only uses one; the REPL uses both.

2. **Misleading naming**: `run_tui_offline_mode` does not run offline. `run_tui_server_mode` is the only one that actually uses the server URL explicitly. Both call `run_tui()` which always uses `ApiClient`.

3. **Copy-paste patterns**: The TUI event loop in `ui_loop()` has ~280 lines of `rt.block_on(api.method())` calls that would need to be changed to `rt.block_on(service.method())`. This is a mechanical but large change.

4. **Feature-gated code**: The REPL is behind `#[cfg(feature = "repl")]`. Offline-first might need similar feature gating or should be unconditional.

### 6.2 Simplification Opportunities

1. **Unified TUI service trait**: Define a trait `TuiServiceProvider` with methods for search, autocomplete, rolegraph, config, summarize. Implement for both `TuiService` and `ApiClient`. The TUI event loop works against the trait. This eliminates the `if offline { ... } else { ... }` branching.

2. **Reuse ReplHandler's pattern**: The `ReplHandler` already solved this exact problem with `new_offline(service)` and `new_server(api_client)`. Apply the same pattern to the TUI.

3. **Simplest possible change**: Replace `ApiClient` with `TuiService` in `ui_loop()`, changing method call signatures. No trait needed if we accept that offline is the default and `--server` is the exception.

4. **Eliminate the middle function**: `run_tui_offline_mode` -> `run_tui(None, transparent)` -> `ui_loop()`. If offline is default, the dispatch can be simplified to: always create TuiService; if `--server`, create ApiClient instead.

## 7. Questions for Human Reviewer

1. **Default mode preference**: Should the default (no-arg) invocation be (a) always offline TuiService, (b) auto-detect server and fall back to offline, or (c) offline with `--server` opt-in?
   - *Why it matters*: Determines whether we need server detection logic or a clean cut.

2. **Startup loading experience**: TuiService::new() builds rolegraphs and indexes. If this takes >1s, should we show a loading screen in the TUI, or initialise before entering raw mode?
   - *Why it matters*: Affects TUI startup flow and user experience.

3. **`--server` flag semantics**: Currently `--server` means "use ApiClient for subcommands". Should it also control the fullscreen TUI mode, or should there be a separate mechanism?
   - *Why it matters*: Flag reuse vs. new flag.

4. **Feature parity tolerance**: The `summarize` feature in TuiService uses the chat/LLM path (requires API key). The server's summarize may differ. Is partial feature parity acceptable for offline TUI?
   - *Why it matters*: Defines "done" for this feature.

5. **Existing design plan**: The previous design plan at `docs/plans/terraphim-agent-tui-offline-server-design-2026-02-13.md` explicitly chose to keep the TUI server-backed and only improve error messaging. Should this new plan supersede that decision?
   - *Why it matters*: Alignment with prior architectural decisions.

6. **Scope of change**: Should this be a minimal change (just make TUI use TuiService) or should it include refactoring the dual-path pattern (trait abstraction)?
   - *Why it matters*: Risk vs. long-term maintainability.

7. **Migration path for server users**: Any documentation, scripts, or workflows that assume `terraphim-agent` (no args) uses the server will break. What communication/changelog approach is needed?
   - *Why it matters*: User communication planning.

---

## Quality Checklist

- [x] Problem clearly distinguished from solutions
- [x] All affected system elements identified with file:line references
- [x] Constraints have clear implications (good vs bad solutions)
- [x] Every assumption marked as such (4 assumptions)
- [x] Risks have de-risking suggestions (5 risks with mitigations)
- [x] Questions are specific and actionable (7 questions with rationale)
