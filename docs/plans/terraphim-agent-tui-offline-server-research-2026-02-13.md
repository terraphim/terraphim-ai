# Research Document: Clarify `terraphim-agent` TUI Offline/Server Requirement (`terraphim-ai-cbm`)

**Status**: Draft
**Author**: Codex
**Date**: 2026-02-13
**Reviewers**: AlexMikhalev

## Executive Summary

`terraphim-agent` currently has mixed runtime contracts: CLI subcommands and REPL offline mode run through embedded local services, while fullscreen interactive TUI uses HTTP API calls to a server endpoint. Documentation and help text mix these models, which creates user confusion about whether "TUI" is offline-capable.
Recommendation: keep current architecture (server-backed fullscreen TUI, offline-capable REPL/subcommands), but make the contract explicit in code-facing UX and docs, then add tests that lock the behavior.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | This removes recurring operator confusion and support churn in a core UX path. |
| Leverages strengths? | Yes | The codebase already has both code paths (`ApiClient` and `TuiService`), so clarification is high leverage and low-risk. |
| Meets real need? | Yes | Ready issue `terraphim-ai-cbm` explicitly requests requirement clarification and behavior adjustment if needed. |

**Proceed**: Yes (3/3 Yes)

## Problem Statement

### Description

Clarify whether `terraphim-agent` interactive usage requires a running server, supports fully offline operation, or both depending on mode.

### Impact

- Users running `terraphim-agent` with no args can assume offline support from top-level messaging, but interactive path is server-backed.
- Documentation currently contains contradictory framing ("comprehensive REPL" vs "requires server"), increasing setup friction.
- Inconsistent expectations reduce trust in automation docs and operator workflows.

### Success Criteria

1. Runtime contract is explicit and consistent across code help text and docs.
2. Users can quickly pick correct mode:
   - Fullscreen interactive TUI (server-backed)
   - REPL and CLI subcommands (offline-capable default)
3. Tests enforce the contract to prevent drift.

## Current State Analysis

### Existing Implementation

- `main` routes no-arg/`interactive` invocation to `run_tui_*` path (`crates/terraphim_agent/src/main.rs:587`).
- `run_tui_offline_mode` comment states TUI requires a running server and points users to REPL for offline (`crates/terraphim_agent/src/main.rs:629`).
- `ui_loop` always builds `ApiClient` from URL/environment default (`crates/terraphim_agent/src/main.rs:1898`).
- Offline CLI subcommands instantiate `TuiService::new()` and use embedded config/settings path (`crates/terraphim_agent/src/main.rs:640`, `crates/terraphim_agent/src/service.rs:17`).
- REPL has explicit offline/server constructors and mode banner (`crates/terraphim_agent/src/repl/handler.rs:198`, `crates/terraphim_agent/src/repl/handler.rs:2367`).

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| CLI dispatch and interactive entry | `crates/terraphim_agent/src/main.rs` | Selects interactive TUI vs offline/server command paths |
| Local embedded service | `crates/terraphim_agent/src/service.rs` | Offline-capable config/search/role behavior |
| REPL runtime mode split | `crates/terraphim_agent/src/repl/handler.rs` | Distinct offline and server REPL modes |
| User-facing TUI guide | `docs/tui-usage.md` | States server requirements for TUI |
| Doc-site TUI page | `docs/src/tui.md` | Currently describes `terraphim-agent` as REPL-centric and server endpoint usage |

### Data Flow

1. **No args / `interactive`**: `main` -> `run_tui` -> `ui_loop` -> `ApiClient` HTTP calls (`/config`, `/search`, `/rolegraph`, etc.).
2. **Offline subcommands**: `main` -> `run_offline_command` -> `TuiService::new` -> local config/service path.
3. **REPL offline**: `repl::run_repl_offline_mode` -> `TuiService`.
4. **REPL server**: `repl::run_repl_server_mode` -> `ApiClient`.

### Integration Points

- Server endpoints used by fullscreen TUI and server-mode commands: `/config`, `/documents/search`, `/rolegraph`, `/chat`, `/documents/summarize`.
- Local/offline path uses embedded config + local persistence via `TuiService`.

## Constraints

### Technical Constraints

- Fullscreen TUI event loop is currently tightly coupled to `ApiClient` operations.
- Existing offline functionality is already implemented for subcommands and REPL; duplicating full TUI logic onto `TuiService` would be non-trivial.
- Feature flags (`repl`, `repl-interactive`, `repl-full`) gate portions of behavior (`crates/terraphim_agent/Cargo.toml`).

### Business Constraints

- Need fast clarification and low regression risk (issue is a P2 task).
- Must preserve existing user workflows and scripts.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Contract clarity | Single consistent mode statement in help/docs | Inconsistent wording across docs/help |
| Backward compatibility | No breaking change to core subcommand behavior | Mostly stable today |
| Operator guidance | Fast diagnosis when server is missing | Weak for no-arg interactive expectations |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Preserve offline subcommand/REPL behavior | Automation and local workflows depend on it | `run_offline_command` and REPL offline path are active today |
| Avoid fullscreen TUI rewrite in this issue | Large scope, high risk, low immediacy | Current loop depends on API client calls throughout |
| Remove ambiguity from docs/help/runtime messages | Root cause of reported confusion | Mixed statements in `show_usage_info`, `docs/tui-usage.md`, `docs/src/tui.md` |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Re-architect fullscreen TUI to run fully offline | Not required to clarify contract; high complexity |
| Feature-parity redesign between `terraphim-agent` and `terraphim-cli` | Separate ready issue (`terraphim-ai-tcw`) |
| Device settings fallback work for `terraphim-cli` | Separate ready issue (`terraphim-ai-2sz`) |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_agent::main` dispatch logic | Defines user mode behavior | High |
| `TuiService` embedded defaults path | Enables offline operation | Medium |
| Docs (`docs/tui-usage.md`, `docs/src/tui.md`, README snippets) | Controls user expectation | High |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Terraphim server endpoint availability | N/A (runtime service) | Medium | Offline REPL/subcommands |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Users interpret "TUI" differently (fullscreen UI vs REPL umbrella) | High | High | Publish explicit mode matrix and examples |
| Contract changes introduce accidental behavior regressions | Medium | Medium | Add focused tests for mode routing and messaging |
| Documentation drift reappears | Medium | Medium | Link docs to tests/checklist in implementation plan |

### Open Questions

1. Should no-arg `terraphim-agent` remain fullscreen TUI, or default to offline REPL?
2. Is fullscreen TUI intentionally server-only long-term, or temporary architecture?
3. Should missing-server detection happen pre-loop with explicit actionable error?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Fullscreen interactive TUI is intended to be server-backed | `ui_loop` uses `ApiClient` only; comment in `run_tui_offline_mode` | Plan may reinforce a temporary behavior | Partially |
| Offline support requirement applies to REPL/subcommands, not fullscreen TUI | Existing code already supports this split | Could conflict with product direction | No |
| Fast clarification is preferred over architectural rewrite for this issue | P2 issue scope and wording ("document requirement and adjust behavior if needed") | Might under-deliver if stakeholders expect full offline TUI | Partially |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| A: "TUI" means fullscreen ratatui interface only | Server-backed requirement can remain; focus on clarity | Chosen as default interpretation based on current code path |
| B: "TUI" includes REPL and all interactive CLI | Product can claim offline capability if modes are distinguished clearly | Also valid; should be reflected in docs as mode matrix |
| C: Fullscreen TUI must be fully offline by default | Requires significant refactor and broader test surface | Rejected for this issue scope; candidate follow-up issue |

## Research Findings

### Key Insights

1. Behavior is not uniformly server-dependent or offline; it is mode-dependent.
2. Current naming (`run_tui_offline_mode`) and help/docs text are the main confusion source.
3. Lowest-risk resolution is contract clarity + targeted UX messaging + regression tests.

### Relevant Prior Art

- Existing offline tests (`crates/terraphim_agent/tests/offline_mode_tests.rs`) already validate local command operation without server.
- Existing server-mode tests (`crates/terraphim_agent/tests/server_mode_tests.rs`) validate API-backed paths.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| None mandatory | Existing code paths are sufficient for this issue | 0 |

## Recommendations

### Proceed/No-Proceed

Proceed.

### Scope Recommendations

1. Define and document a clear mode contract:
   - Fullscreen interactive TUI: server-backed.
   - REPL (`terraphim-agent repl`) and offline subcommands: offline-capable.
2. Replace ambiguous/help text that suggests no-arg interactive may be either REPL or TUI.
3. Add actionable error/messaging when server is unavailable for fullscreen TUI path.

### Risk Mitigation Recommendations

1. Add tests that verify mode-specific messaging and behavior.
2. Keep behavior changes minimal; do not redesign runtime architecture in this issue.
3. Capture deferred architectural direction (full offline fullscreen TUI) as a follow-up if desired.

## Next Steps

If approved:
1. Produce Phase 2 implementation plan with concrete file-level changes.
2. Resolve open questions around default no-arg behavior before Phase 3.
3. Run disciplined quality evaluation on research/design docs before implementation.

## Appendix

### Reference Materials

- `crates/terraphim_agent/src/main.rs:587`
- `crates/terraphim_agent/src/main.rs:629`
- `crates/terraphim_agent/src/main.rs:1898`
- `crates/terraphim_agent/src/service.rs:17`
- `crates/terraphim_agent/src/repl/handler.rs:198`
- `docs/tui-usage.md:409`
- `docs/src/tui.md:37`
