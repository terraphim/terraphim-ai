# Research Document: Architecture Cleanup (Duplicate Functionality + Architectural Issues)

Status: Draft
Author: OpenCode
Date: 2026-01-22
Branch: refactor/architecture-cleanup

## Executive Summary

The repository contains several clusters of duplicated functionality and inconsistent crate-level architecture decisions (edition/version skew, duplicated connector abstractions, duplicated app-service wrappers, duplicated path expansion helpers, and the use of HTTP mocking libraries despite a stated “no mocks in tests” policy). These issues increase maintenance cost, make behavior inconsistent across binaries, and raise the risk of subtle bugs (e.g., different path expansion semantics across components).

This document maps the main duplication points, identifies the likely root causes, and proposes consolidation directions to be turned into a Phase 2 implementation plan.

Decision (2026-01-22): treat `terraphim_agent` (`terraphim-agent` binary) as the primary user-facing CLI/TUI surface. Other CLIs (`terraphim-cli`, `terraphim-repl`) should either become thin wrappers around shared core logic or be candidates for deprecation once feature parity and stability are proven.

User decisions (2026-01-22):
- `terraphim-repl` can be removed once `terraphim-agent repl` has parity.
- AI assistant session sources that must be stable: Claude Code + OpenCode + Aider.
- Tool calling extraction/structuring: ignore for now (keep commands as plain text where they appear).
- For Aider: index the full transcript plus file diffs/patches as primary searchable artifacts.

## Problem Statement

### Description
We want to:
1) identify duplicate implementations that should be shared or removed, and
2) identify “bad” architectural decisions that increase complexity or violate project rules.

### Impact
- Higher maintenance cost: bug fixes must be applied in multiple places.
- Inconsistent runtime behavior: two call paths may implement similar features differently.
- Increased cognitive load for contributors: multiple ways to do the same thing.
- Policy drift: tests include mock frameworks while the repo guidance says “never use mocks”.

### Success Criteria
- For each duplication cluster: a single canonical implementation exists (shared crate/module), and other call sites depend on it.
- Crate metadata and dependency versioning becomes consistent with workspace conventions (edition 2024, version.workspace, and consistent dependency declarations).
- Tests comply with project guidelines (replace mocks with real implementations / local ephemeral servers where reasonable).

## Current State Analysis

### Duplication/Issue Map (Initial)

| Area | Symptom | Primary Locations |
|------|---------|-------------------|
| Session connectors | Two distinct `SessionConnector` traits + registries | `crates/terraphim_sessions/src/connector/mod.rs`, `crates/terraphim-session-analyzer/src/connectors/mod.rs` |
| Session model normalization | Two different session models (`Session` vs `NormalizedSession`) with overlapping intent | `crates/terraphim_sessions/src/model.rs`, `crates/terraphim-session-analyzer/src/connectors/mod.rs` |
| App service wrappers | Near-copy implementations for CLI/TUI service facades | `crates/terraphim_agent/src/service.rs`, `crates/terraphim_repl/src/service.rs`, `crates/terraphim_cli/src/service.rs` |
| App surface overlap | Multiple binaries expose overlapping commands (search/roles/config/replace) with diverging behavior and output formats | `crates/terraphim_agent/src/main.rs`, `crates/terraphim_cli/src/main.rs`, `crates/terraphim_repl/src/main.rs` |
| Path expansion | Multiple `expand_path()` variants with different semantics | `crates/terraphim_config/src/lib.rs` (robust), `crates/terraphim_middleware/src/haystack/ai_assistant.rs` (minimal) |
| HTTP client creation | Many direct `reqwest::Client::new()` call sites | multiple crates/tests; central helper exists in `crates/terraphim_service` but not used everywhere |
| Testing policy drift | Mock HTTP frameworks in dev-deps | `crates/haystack_atlassian/Cargo.toml` (mockito), `crates/haystack_grepapp/Cargo.toml` + `crates/haystack_discourse/Cargo.toml` (wiremock) |
| Crate metadata skew | Mixed Rust editions and inconsistent version pinning | e.g. `crates/terraphim-session-analyzer/Cargo.toml` (edition 2021), `crates/haystack_*` (edition 2021), various `version = "1.0.0"` pins |

### Observations

1) The codebase already has a pattern for optional integration to avoid duplication (e.g. `terraphim_sessions` feature-gates `terraphim-session-analyzer`). However, the connector abstraction is still duplicated instead of being shared.

2) The CLI/TUI ecosystem appears to have evolved organically. Multiple crates wrap the same core service (`terraphim_service::TerraphimService`) with near-identical initialization and common operations.

2b) The repo documentation already positions `terraphim_agent` as the main CLI/TUI entrypoint:
- `README.md` repeatedly recommends `cargo install terraphim_agent` and documents `terraphim-agent` usage.
- `crates/terraphim_agent/Cargo.toml` describes "Complete CLI and TUI interface" and includes features such as REPL, hooks integration, robot mode, and sessions.

3) The config crate contains a robust `expand_path()` implementation (supports `$VAR`, `${VAR}`, `${VAR:-default}`, and `~`). The middleware AI assistant haystack contains a simpler `expand_path()` that only handles `~` and `$HOME`, creating inconsistent behavior.

4) The workspace is declared as edition 2024 and version 1.6.0, but multiple crates are still edition 2021 and/or pin internal dependency versions to 1.0.0. This may be intentional for crates.io compatibility but creates confusion inside a single workspace.

4b) `terraphim_agent` itself is still `edition = "2021"` while `terraphim-cli` and `terraphim-repl` are edition 2024. This increases the chance of feature/idiom drift and makes cross-crate refactors more complex.

## Constraints

### Technical Constraints
- Large Rust workspace with many crates; changes must be staged to avoid widespread breakage.
- Some crates are likely published independently to crates.io; API changes require care (semver).
- Feature gates are used heavily; refactors must preserve feature combinations.

### Process Constraints
- No implementation in Phase 1: only identify and document.
- Tests should avoid mocks (project guidance), suggesting using:
  - local ephemeral servers when feasible,
  - record/replay fixtures, or
  - integration tests against real services with env gating.

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking public APIs while consolidating | Medium | High | Introduce shared crates behind existing APIs first; deprecate later |
| Feature-gated code drift | Medium | Medium | Add compile-only tests for feature matrices; keep small steps |
| Crate edition upgrades cause subtle behavior changes | Low-Med | Medium | Upgrade one crate at a time; run `cargo test --workspace` frequently |

### Open Questions
1) Resolved: `terraphim_agent` is the long-term user-facing primary surface.
2) Is `terraphim-session-analyzer` intended to remain a standalone CLI (with its own connector ecosystem), or should it become a library-first crate powering `terraphim_sessions`?
3) Are haystack crates meant to be publishable independently (hence edition/version skew), or are they internal to this monorepo?

4) (Deferred) Should we introduce a structured tool invocation event model for session ingestion, or keep tool commands as plain text only?

## Research Findings (Detailed)

### Finding A: Duplicate Session Connector Abstractions

- `terraphim_sessions` defines:
  - async `SessionConnector` trait returning `Vec<Session>`
  - `ConnectorRegistry` that can incorporate TSA-based connectors via feature gates
  - intended as a library API for session management
- `terraphim-session-analyzer` defines:
  - sync `SessionConnector` trait returning `Vec<NormalizedSession>`
  - its own registry and multiple connector implementations behind a feature

Why this is a problem:
- Two “source of truth” abstractions drift independently.
- Implementing a new connector likely requires changes in two crates.

Likely root cause:
- TSA started as a standalone CLI and later `terraphim_sessions` was introduced as a reusable library, but the connector layer wasn’t consolidated.

### Finding B: Duplicated App Service Facades

- `crates/terraphim_agent/src/service.rs` (TUI service)
- `crates/terraphim_repl/src/service.rs` (TUI service)
- `crates/terraphim_cli/src/service.rs` (CLI service)

These files repeat:
- logging init
- loading device settings
- load-or-create embedded config
- construct `ConfigState`
- wrap `TerraphimService` in `Arc<Mutex<...>>`
- many shared helper methods (search, thesaurus, match/replace, etc.)

Additional nuance (surface divergence):
- `terraphim-agent` has both interactive (TUI/REPL) and non-interactive modes, plus Claude Code hook handling.
- `terraphim-cli` is automation-first and defaults to JSON output.
- `terraphim-repl` has its own asset embedding + first-run config/thesaurus writing, which diverges from the config/persistence flow used elsewhere.

Why this is a problem:
- Bug fixes and feature additions must be replicated.
- It’s unclear which surface is authoritative.

### Finding C: Duplicated Path Expansion Helpers

- Robust expansion exists in `crates/terraphim_config/src/lib.rs` (supports multiple syntaxes).
- A minimal expansion exists in `crates/terraphim_middleware/src/haystack/ai_assistant.rs`.

Why this is a problem:
- Same config values can resolve differently depending on which path uses them.
- The minimal helper doesn’t support `${VAR}` or `${VAR:-default}` used elsewhere.

### Finding D: Test Framework Drift (Mocks)

Despite the repository guidance “never use mocks in tests”, the following exist:
- `mockito` in `crates/haystack_atlassian/Cargo.toml`
- `wiremock` in `crates/haystack_grepapp/Cargo.toml` and `crates/haystack_discourse/Cargo.toml`

Why this is a problem:
- Inconsistent testing philosophy and maintenance expectations.
- Mock-based tests can mask integration issues.

### Finding E: Workspace Consistency Issues

- Workspace declares edition 2024 and version 1.6.0.
- Some crates are edition 2021 (e.g. `terraphim-session-analyzer`, `atlassian_haystack`, `discourse_haystack`).
- Some crates pin sibling crates to `version = "1.0.0"` even though they are in the same workspace.

Why this is a problem:
- Confusing dependency story inside the monorepo.
- Increased chance of version skew and publish-time breakage.

## Recommendations (Phase 1 Output)

1) Establish a single canonical connector abstraction for session sources.
2) Extract common app-service facade logic into a shared crate/module that is owned by (and optimized for) `terraphim_agent` as the primary consumer; other binaries should call into the shared module rather than re-implementing init/search/role selection.
3) Centralize path expansion into one shared utility and remove duplicates.
4) Replace mock-based haystack tests with integration-style tests (local ephemeral server or recorded fixtures).
5) Align crate editions and dependency version declarations with workspace conventions, or explicitly document why certain crates intentionally diverge.

## Next Steps

If this research direction is approved, proceed to Phase 2 (disciplined design) by drafting an implementation plan with:
- a migration strategy that avoids breaking public APIs,
- a stepwise refactor sequence,
- a test strategy for each step,
- and a rollback plan.
