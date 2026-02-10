# Research Document: Terraphim Agent & Terraphim CLI Status Check

**Status**: Draft
**Author**: pi
**Date**: 2026-02-10
**Reviewers**: TBD

## Executive Summary

This research reviews the current state of the `terraphim_agent` and `terraphim_cli` crates. Both are CLI front-ends to the shared `terraphim_service` stack, but they diverge in user experience: `terraphim_agent` focuses on interactive TUI/REPL workflows, onboarding, and AI-agent integration, while `terraphim_cli` is a non-interactive, JSON-first tool for automation. The codebase shows feature-rich capabilities (hooks, guard patterns, replace/validate/suggest, update mechanisms) with a few notable constraints and risks: TUI mode currently expects a running server; `terraphim_cli` requires device settings to load (no embedded fallback); and feature-gated REPL functionality introduces build-time variability.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Requested by user for project status review. |
| Leverages strengths? | Yes | We can inspect and document existing system details quickly. |
| Meets real need? | Yes | Status insight supports prioritization and next-step planning. |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
Establish the current status of the Terraphim CLI interfaces (starting with `terraphim_agent` and `terraphim_cli`) to inform subsequent planning, risk mitigation, and design decisions.

### Impact
Stakeholders need a clear picture of capabilities, constraints, and gaps in the CLI front-ends to plan improvements and ensure reliability for users and automation.

### Success Criteria
- Clear inventory of current functionality and entry points.
- Identified constraints, risks, and open questions.
- Documented dependencies and code locations.

## Current State Analysis

### Existing Implementation
- `terraphim_agent` provides a TUI/TUI-based interactive CLI, REPL (feature-gated), onboarding wizard, hooks integration, guard checks for destructive commands, and AI/robot-friendly output modes.
- `terraphim_cli` provides JSON-first commands for automation: search, config/roles, graph, replace/link/synonym, find matches, thesaurus listing, and update/rollback capabilities.
- Both use embedded configuration via `terraphim_config` and `terraphim_service`, backed by persistence and settings.

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Agent entrypoint | `crates/terraphim_agent/src/main.rs` | TUI/CLI command dispatch, hooks, replace/validate/suggest, updates |
| Agent service wrapper | `crates/terraphim_agent/src/service.rs` | Config state + service facade for TUI/CLI commands |
| Agent API client | `crates/terraphim_agent/src/client.rs` | Server-mode HTTP API calls |
| Agent onboarding | `crates/terraphim_agent/src/onboarding/` | Wizard and templates for initial setup |
| Agent guard | `crates/terraphim_agent/src/guard_patterns.rs` | Command safety checks |
| Agent robot mode | `crates/terraphim_agent/src/robot/` | Structured JSON output for AI agents |
| CLI entrypoint | `crates/terraphim_cli/src/main.rs` | JSON-first commands and formatting |
| CLI service wrapper | `crates/terraphim_cli/src/service.rs` | Config state + service facade |

### Data Flow
- CLI input → clap parsing → service wrapper (`TuiService`/`CliService`) → `terraphim_service::TerraphimService` → automata/thesaurus/rolegraph operations → output formatting.
- `terraphim_agent` supports server mode for queries via `ApiClient` and offline mode using embedded config.

### Integration Points
- Shared libraries: `terraphim_service`, `terraphim_config`, `terraphim_types`, `terraphim_persistence`, `terraphim_settings`, `terraphim_automata`, `terraphim_rolegraph`.
- Update mechanism: `terraphim_update` for update checks and binary updates.
- TUI dependencies: `ratatui`, `crossterm`, `atty`.

## Constraints

### Technical Constraints
- TUI mode comments indicate a server dependency even in “offline” mode (`terraphim_agent`), reducing true offline capability.
- `terraphim_cli` strictly loads device settings without a fallback; missing settings will error early.
- REPL and related commands are feature-gated and not always built.

### Business Constraints
- No explicit constraints captured in code; assume CLI stability and automation compatibility are priorities.

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Automation-friendly output | JSON by default | Implemented in `terraphim_cli` |
| Interactive usability | TUI/REPL | Implemented in `terraphim_agent` |
| Safety guard | Block destructive commands | Implemented in `guard_patterns` |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Stable CLI interfaces | Automation integrations depend on deterministic outputs | JSON output paths in `terraphim_cli` |
| Config availability | Both CLIs depend on embedded config and device settings | `ConfigBuilder` + `DeviceSettings` loading |
| Server availability for TUI | TUI “offline” path still needs server | Comment in `run_tui_offline_mode` |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|----------------|----------------|
| Performance benchmarking | Not requested for status check |
| UI/UX redesign | Not requested; focus is current status |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_service` | Core search/rolegraph operations | High (single point of logic) |
| `terraphim_config` | Config load/save and embedded defaults | Medium |
| `terraphim_settings` | Device settings load | Medium (runtime failure if missing) |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `ratatui` | 0.30 | Low | n/a |
| `crossterm` | 0.29 | Low | n/a |
| `clap` | 4.x | Low | n/a |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| TUI “offline” mode still expects server | Medium | Medium | Document requirement or adjust fallback behavior |
| CLI fails if device settings missing | Medium | Medium | Consider embedded fallback like agent |
| Feature-gated REPL availability creates inconsistent builds | Low | Medium | Clarify build profiles and packaging |

### Open Questions
1. Is the expectation that `terraphim_agent` TUI should operate fully offline, or is server dependency acceptable?
2. Should `terraphim_cli` mirror the agent’s fallback to embedded device settings on first run?
3. What feature parity is required between `terraphim_agent` commands and `terraphim_cli` automation?

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Embedded config is the default for both tools | Config builders use `ConfigId::Embedded` | CLI usability could break | No |
| Server mode endpoints are available and stable | `ApiClient` uses fixed paths | TUI features may fail | No |
| Update mechanism is expected for both CLIs | `terraphim_update` usage | Update failures impact UX | No |

### Multiple Interpretations Considered
| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| “Status check” means runtime health tests | Would require running binaries | Deferred: no runtime execution requested |
| “Status check” means code inventory and risks | Focused on code evidence and docs | Chosen for this report |

## Research Findings

### Key Insights
1. `terraphim_agent` is feature-rich (TUI/REPL/hooks/guard/replace/validate/suggest) with both offline and server flows, but true offline TUI is constrained by current assumptions.
2. `terraphim_cli` is lean, JSON-first, and automation-focused, with some feature overlap (replace, graph, roles) but without guard/hook or TUI features.
3. Both tools rely heavily on shared service/config layers; stability hinges on those shared crates.

### Relevant Prior Art
- Existing CLI onboarding and design docs in `.docs/` (e.g., `research-cli-onboarding-wizard.md`, `design-cli-onboarding-wizard.md`).

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Run CLI smoke tests | Validate actual runtime behavior | 1-2 hours |
| Compare feature parity | Identify missing automation commands | 2-4 hours |

## Recommendations

### Proceed/No-Proceed
Proceed with a structured status/health assessment (test runs + doc updates) once scope is approved.

### Scope Recommendations
- Limit to `terraphim_agent` and `terraphim_cli` in this phase.
- Document server dependencies and config behaviors explicitly.

### Risk Mitigation Recommendations
- Clarify device settings fallback for CLI.
- Document or enforce the TUI server requirement.

## Next Steps

If approved:
1. Produce a design plan for status assessment and documentation updates.
2. Run targeted smoke tests for both CLIs.
3. Open issues for identified gaps and risks.

## Appendix

### Reference Materials
- `crates/terraphim_agent/src/main.rs`
- `crates/terraphim_agent/src/service.rs`
- `crates/terraphim_agent/src/client.rs`
- `crates/terraphim_cli/src/main.rs`
- `crates/terraphim_cli/src/service.rs`

### Code Snippets
See referenced files above for command parsing and service wrappers.
