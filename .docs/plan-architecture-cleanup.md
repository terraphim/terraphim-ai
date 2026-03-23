# Implementation Plan: Architecture Cleanup (Duplicates + Consistency)

Status: Draft (Needs Approval)
Research Doc: `.docs/research-architecture-cleanup.md`
Author: OpenCode
Date: 2026-01-22
Branch: refactor/architecture-cleanup

## Overview

This plan reduces duplication and inconsistency without large-scale rewrites by introducing small shared modules/crates, migrating call sites incrementally, and preserving existing public APIs until a follow-up deprecation pass.

Primary product decision: `terraphim_agent` (`terraphim-agent`) is the primary user-facing CLI/TUI surface. `terraphim-cli` and `terraphim-repl` are treated as secondary/frontends that should become thin wrappers around shared core logic (or be deprecated once the primary CLI covers their needs).

User decisions (2026-01-22):
- `terraphim-repl`: remove crate/binary once `terraphim-agent repl` has parity.
- AI assistant session sources that must be stable: Claude Code + OpenCode + Aider.
- Tool-calling extraction: ignore for now (no structured tool invocation model required in this refactor).
- Aider indexing: include the full transcript + file diffs/patches as searchable artifacts (commands can remain as plain text, not structured events).

## Scope

In scope:
- Consolidate duplicated service facade logic used by CLI/TUI crates, centered around `terraphim_agent` as the primary consumer.
- Consolidate duplicated session connector abstractions.
- Remove duplicated path expansion logic.
- Replace mock-framework-based tests for haystack crates with in-process “real server” tests.
- Align crate metadata (edition/version/dependency declarations) with workspace conventions where feasible.

Out of scope (for this refactor pass):
- Major redesign of `terraphim_service` or multi-agent workflows.
- Large-scale changes to persistence backends.
- Changes requiring secrets/credentials to run tests (keep those gated).
- Building a full structured “tool invocation” event model (can be added later once ingestion is stable).

## Key Design Decisions

1) Shared abstractions should live in libraries, not binaries.
2) Prefer additive changes first (new shared crate/module) and migrate call sites before deleting old code.
3) Avoid breaking published APIs in the same release; use deprecation windows.
4) Tests should use real HTTP servers (in-process) rather than mock frameworks.

5) `terraphim_agent` owns the UX and command semantics; secondary binaries should either:
- delegate to the primary implementation (for shared semantics), or
- intentionally diverge for automation needs (e.g., JSON output) but still reuse core initialization + domain logic.

## File Changes

### New Files (proposed)

| File | Purpose |
|------|---------|
| `crates/terraphim_app_core/src/lib.rs` | Shared service facade + initialization, extracted from `terraphim_agent` and reused by other binaries |
| `crates/terraphim_session_connectors/src/lib.rs` | Shared session connector trait(s), models, registry |

### Modified Files (proposed)

| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/service.rs` | Replace local facade with wrapper around `terraphim_app_core` |
| `crates/terraphim_repl/src/service.rs` | Replace local facade with wrapper around `terraphim_app_core` |
| `crates/terraphim_cli/src/service.rs` | Replace local facade with wrapper around `terraphim_app_core` |
| `crates/terraphim_sessions/src/connector/mod.rs` | Re-export connector abstractions from `terraphim_session_connectors` |
| `crates/terraphim-session-analyzer/src/connectors/mod.rs` | Reuse `terraphim_session_connectors` instead of local duplicates |
| `crates/terraphim_middleware/src/haystack/ai_assistant.rs` | Use canonical path expansion helper |
| `crates/haystack_*/*.toml` | Remove `wiremock`/`mockito`, align edition/version declarations |

### Deleted Files (possible, after migration)

| File | Reason |
|------|--------|
| duplicate connector traits/registries | superseded by shared crate |
| duplicated service facade code | superseded by shared crate |

## Detailed Design

## terraphim-repl Parity Checklist (Gate For Removal)

Definition of “parity” for removing `crates/terraphim_repl`:

User-facing behaviors that must be available via `terraphim-agent repl`:
- Offline startup works without requiring a running server.
- Configuration and thesaurus are created/loaded on first run without manual file setup.
- Core commands available:
  - search
  - roles list/select
  - config show (and any supported set operations)
  - find / replace (including synonym mode if used)
  - thesaurus inspection
  - (optional but recommended) chat if configured

Ergonomic behaviors that must be at least “not worse”:
- Clear error messages when configuration is missing/invalid.
- REPL command help / discoverability.
- No surprising filesystem writes outside the documented config dir.

Non-functional:
- `cargo test -p terraphim_agent --features repl-full` passes.
- `cargo test --workspace` passes with `terraphim_repl` still present.

Removal gate:
- We only delete `crates/terraphim_repl` after parity is confirmed in a short, explicit checklist review.

### A) Consolidate App Service Facades

Goal: keep `terraphim_agent` authoritative and minimize duplicated initialization/role/search logic in `terraphim-cli` and `terraphim-repl`.

Proposed API (in `terraphim_app_core`):

- `pub struct AppService` (name tbd) containing:
  - `ConfigState`
  - `Arc<Mutex<TerraphimService>>`
- `impl AppService`:
  - `async fn new_embedded() -> Result<Self>` (shared init path)
  - common operations: `get_config`, `get_selected_role`, `update_selected_role`, `search_with_query`, `get_thesaurus`, `find_matches`, `replace_matches`, etc.

Migration:
- extract `AppService` by moving code out of `crates/terraphim_agent/src/service.rs` into `terraphim_app_core`.
- update `crates/terraphim_agent/src/service.rs` to become a thin wrapper around `terraphim_app_core`.
- update `crates/terraphim_cli/src/service.rs` to reuse the same `AppService` init/role selection primitives.
- update `crates/terraphim_repl/src/service.rs` similarly, and decide whether `crates/terraphim_repl/src/main.rs` should stop writing its own embedded config assets (prefer using the same persistence flow as `terraphim_agent`).

Deprecation/removal plan (`terraphim-repl`):
- Phase 3 should first ensure `terraphim-agent repl` covers the workflows that `terraphim-repl` users rely on.
- After parity is verified, remove `crates/terraphim_repl` from the workspace and delete the crate/binary.

### B) Consolidate Session Connector Abstractions

Goal: one connector interface and one normalization model.

Recommendation:
- Create `terraphim_session_connectors` as the canonical abstraction crate.
- Move (or duplicate-then-delete) the shared pieces:
  - `ConnectorStatus`
  - `ImportOptions`
  - connector trait
  - registry
  - normalized session/message model

Then:
- `terraphim_sessions` becomes a higher-level library that:
  - depends on `terraphim_session_connectors`
  - provides caching/search/enrichment layers (as it does today)
  - converts from normalized sessions into its `Session` model if needed
- `terraphim-session-analyzer` depends on `terraphim_session_connectors` for connector enumeration/import, and keeps its analysis pipeline.

Notes:
- This approach avoids forcing TSA to adopt async immediately.
- If async is desired long term, introduce `AsyncSessionConnector` in the shared crate and provide a sync adapter for TSA.

Connector priority (stable):
- Claude Code
- OpenCode
- Aider

Tool calling handling:
- Do not build a structured tool invocation schema in this pass.
- If tool commands appear in session sources (especially Aider/OpenCode), keep them as plain text content inside the normalized session/messages so they remain searchable.

## Aider Artifact Model (Transcript + Diffs/Patches)

Goal: make Aider sessions searchable by both conversational intent and concrete code changes.

Constraints:
- No structured “tool invocation” model in this pass.
- Keep model minimal and compatible with existing `NormalizedSession` / `NormalizedMessage` style ingestion.

Proposed indexing contract for Aider:

- Canonical session unit: one `NormalizedSession` per “# aider chat started at ...” segment.
- Canonical searchable artifacts produced from a session:
  - Transcript documents: message-level documents (existing behavior).
  - Patch documents: derived documents extracted from message content when it contains patch/diff material.

Patch/diff identification heuristics:
- Treat fenced code blocks that look like diffs as patches:
  - code fence language indicates diff/patch (e.g., "diff"), OR
  - content starts with common diff markers (file headers and hunk markers).
- Treat "apply_patch" style blocks (if present in logs) as patches.
- Keep all other blocks as regular transcript content.

Patch document structure (indexed fields):
- Title: derived from project name + session id + message index + (if discoverable) file name.
- Body: the diff/patch content verbatim.
- Metadata:
  - source = aider
  - artifact_type = patch
  - session_external_id
  - message_idx
  - source_path
  - project_path (if available)

Transcript document structure (indexed fields):
- Title: derived from project name + session id + message index.
- Body: message content verbatim.
- Metadata:
  - source = aider
  - artifact_type = transcript
  - session_external_id
  - message_idx

Acceptance criteria:
- A query that matches only within a diff still returns results.
- A query that matches conversational text still returns results.
- Patch results are distinguishable from transcript results via metadata.

### C) Canonical Path Expansion

Goal: all “path-like” config values resolve the same way.

Recommendation:
- Expose a single helper as a public API (either in `terraphim_config` or a new small `terraphim_path` crate).
- Replace middleware’s local helper with that canonical implementation.

### D) Remove Mock Frameworks From Haystack Tests

Goal: comply with project guidance and improve fidelity.

Approach:
- Replace `wiremock`/`mockito` usage with an in-process `axum` (or `hyper`) test server that:
  - binds on `127.0.0.1:0`
  - serves deterministic JSON fixture responses
  - exercises the real HTTP client and response parsing logic

This keeps tests hermetic without using mocking frameworks.

### E) Align Crate Metadata and Dependency Declarations

Goal: reduce edition/version skew inside the workspace.

Approach:
- Prefer `edition.workspace = true` and `version.workspace = true` for workspace members.
- Prefer `dep = { workspace = true }` where appropriate.
- For crates that must remain edition 2021 for external reasons, add explicit documentation explaining why.

## Test Strategy

Unit tests:
- Add focused tests for the new shared crates:
  - `terraphim_app_core`: initialization path, role selection, query execution (mock-free; can use existing in-memory persistence patterns).
  - `terraphim_session_connectors`: connector detection and import for at least one connector using temp directories.

Integration tests:
- Replace haystack HTTP mocking tests with in-process server tests.

Verification commands (expected during Phase 3):
- `cargo test -p terraphim_sessions`
- `cargo test -p terraphim-session-analyzer`
- `cargo test -p terraphim_agent`
- `cargo test -p terraphim_cli`
- `cargo test -p terraphim_repl`
- `cargo test --workspace`

## Implementation Steps (Proposed Sequence)

1) Add `crates/terraphim_app_core` with shared initialization and core methods extracted from `terraphim_agent`.
2) Migrate `terraphim_agent` to use `terraphim_app_core` (no behavior changes, just move).
3) Migrate `terraphim_cli` to use `terraphim_app_core` (preserve JSON output, reuse logic).
4) Confirm `terraphim-agent repl` parity against the checklist, then remove `crates/terraphim_repl`.
5) Add `crates/terraphim_session_connectors` and port connector abstractions.
6) Migrate `terraphim_sessions` to re-export/use shared connectors.
7) Migrate `terraphim-session-analyzer` to use shared connectors.
8) Remove duplicated connector code from both crates (after successful migration).
9) Replace middleware’s `expand_path` with canonical helper.
10) Replace haystack tests (remove `wiremock`/`mockito`), ensure they remain hermetic.
11) Align crate metadata (edition/version) and ensure `cargo test --workspace` passes.

Addendum (removal):
- Once `terraphim-agent repl` parity is verified, remove `crates/terraphim_repl` and any docs pointing users at it.

## Rollback Plan

- Each step is designed to be revertible:
  - introduce shared crates first,
  - migrate one consumer at a time,
  - only delete old implementations at the end.
