# Research Document: Shared Learning Store Markdown Migration

**Status**: Approved
**Author**: Terraphim AI Agent
**Date**: 2026-04-19
**Reviewers**: Human approval required

## Executive Summary

`SharedLearningStore` currently presents a durable-storage API but is backed by `DeviceStorage::arc_memory_only()` and a no-op `load_all()`, so shared learnings disappear between process runs. A markdown backend already exists, but it is not yet a safe drop-in replacement because it serialises only part of `SharedLearning`, uses a different path strategy than the approved cross-platform recommendation, and can mix canonical learnings with shared copies.

The approved Step 6 direction is a minimal migration: preserve the existing `SharedLearningStore` API, keep its in-memory index and BM25 logic, and replace only the fake persistence layer with `MarkdownLearningStore` after extending markdown round-tripping to cover the full `SharedLearning` state. Startup loading should read both canonical and shared markdown files, then de-duplicate by `learning.id`.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | This is the missing durability step in the approved Learning and KG plan, and it unblocks making the existing CLI commands genuinely useful across runs. |
| Leverages strengths? | Yes | The codebase already has a markdown-first learning model, wiki sync, and a dedicated markdown store implementation. |
| Meets real need? | Yes | `learn shared` commands currently imply persistent state, but `SharedLearningStore::open()` loads an in-memory backend and `load_all()` does nothing. |

**Proceed**: Yes

## Problem Statement

### Description

Step 6 needs to make `SharedLearningStore` durable without rewriting its higher-level behaviour. The current store API is used by the CLI and tests, but persistence is effectively stubbed.

### Impact

- Users can import, list, promote, and query shared learnings during one run, but those learnings are lost on restart.
- Cross-agent work now has two persistence concepts: `SharedLearningStore` and `MarkdownLearningStore`, with no single canonical path.
- The new injector and future sharing features risk diverging from the store used by the CLI.

### Success Criteria

- `SharedLearningStore` data survives process restart on the same machine.
- Existing `SharedLearningStore` callers do not need signature changes.
- Full `SharedLearning` fidelity is preserved when saving and reloading.
- Canonical learnings and shared-directory copies do not appear as duplicate records in the store index.
- Storage paths follow the approved `ProjectDirs::from("com", "aks", "terraphim")` direction, while still permitting explicit overrides for tests and power users.

## Current State Analysis

### Existing Implementation

`crates/terraphim_agent/src/shared_learning/store.rs` contains the main store API used by the CLI. It holds an in-memory `RwLock<HashMap<String, SharedLearning>>`, implements deduplication and promotion logic, and calls `persist()` after mutations. However:

- `open()` initialises `DeviceStorage::arc_memory_only()`.
- `load_all()` only logs and returns `Ok(())`.
- The `storage` field is not used for reads.
- Durability is therefore implied by the API but not delivered by the implementation.

`crates/terraphim_agent/src/shared_learning/markdown_store.rs` already provides filesystem-backed markdown save/load/list logic. It is close to the right backend, but it currently loses part of the domain model on round-trip:

- `quality`
- `applicable_agents`
- `keywords`
- `verify_pattern`
- `updated_at`
- `promoted_at`

It also defaults to `dirs::data_local_dir()/terraphim/learnings`, while the approved cross-platform recommendation in `.docs/research-cross-platform-data-dirs.md` is `ProjectDirs::from("com", "aks", "terraphim").data_local_dir()`.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Shared learning API | `crates/terraphim_agent/src/shared_learning/store.rs` | Public store, BM25 deduplication, trust promotion, CLI backing |
| Markdown backend | `crates/terraphim_agent/src/shared_learning/markdown_store.rs` | Save/load/list learnings as markdown with YAML frontmatter |
| Domain model | `crates/terraphim_agent/src/shared_learning/types.rs` | `SharedLearning`, `TrustLevel`, `QualityMetrics`, source metadata |
| CLI integration | `crates/terraphim_agent/src/main.rs` | `learn shared` subcommands call `SharedLearningStore` |
| CLI integration tests | `crates/terraphim_agent/tests/shared_learning_cli_tests.rs` | Current behavioural coverage for list/import/promote/stats |
| Prior plan | `.docs/design-learning-kg-2026-04-17.md` | Step 6 explicitly calls for replacing in-memory storage with markdown backend |

### Data Flow

Current:

```text
CLI -> SharedLearningStore::open()
    -> DeviceStorage::arc_memory_only()
    -> load_all() no-op
    -> in-memory HashMap only

Mutations -> persist() -> SharedLearningRecord.save()
          -> no durable reads on next run
```

Desired:

```text
CLI -> SharedLearningStore::open()
    -> MarkdownLearningStore backend
    -> load persisted canonical learnings into in-memory index

Mutations -> persist canonical markdown file
Reads     -> serve from in-memory index after startup load
```

### Integration Points

- `run_shared_learning_command()` in `main.rs` depends on `SharedLearningStore::open()` plus current list/get/promote/import APIs.
- `LearningInjector` reads markdown files directly from the shared directory and should not be broken by canonical-store migration.
- `wiki_sync` is related to shared learning publication but is not currently wired through `SharedLearningStore`.

## Constraints

### Technical Constraints

- The public `SharedLearningStore` API is already used by the CLI and tests, so Step 6 should preserve those signatures where possible.
- `SharedLearning` includes quality and metadata fields that the markdown backend does not yet persist.
- `MarkdownLearningStore::list_all()` walks every subdirectory, including the shared directory, which can create duplicates if shared copies are treated as canonical records.
- The repository guidance for this cluster explicitly prefers `ProjectDirs::from("com", "aks", "terraphim")` over ad hoc `dirs::data_local_dir()` usage.
- Tests must not use mocks.

### Business Constraints

- This is Step 6 of an already approved sequence, so the safest option is to preserve behaviour and limit scope to store durability.
- Human approval is required before implementation.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Durability across restart | Required | Not met |
| CLI compatibility | Preserve | Currently present |
| Startup cost | Acceptable for local file scan | Not applicable yet |
| Human readability | Preserve markdown files | Already present in markdown backend |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Preserve full `SharedLearning` fidelity | Losing quality/promotion metadata would silently break trust promotion and ranking behaviour | `types.rs` has richer fields than `markdown_store.rs` frontmatter |
| Preserve current `SharedLearningStore` API | CLI and tests already depend on it | `main.rs` and `shared_learning_cli_tests.rs` call `open`, `insert`, `list_all`, `promote_*`, `get` |
| Keep de-duplicated canonical state in memory even when loading shared publication artefacts | Shared copies may be present alongside canonical records, but the store index must still present one logical learning per ID | `MarkdownLearningStore::list_all()` currently traverses `shared/` as well as agent directories |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Rewriting BM25 deduplication or ranking | Already works inside `SharedLearningStore`; durability does not require changing it |
| Reworking injector logic | Step 6 is about store migration, not injection behaviour |
| Introducing SQLite or another database | Conflicts with the markdown-first direction already approved |
| Auto-publishing L2/L3 learnings to shared storage during this step | Changes behaviour and risks duplicate indexing; better handled separately |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `shared_learning/types.rs` | Defines the full payload that must round-trip | High |
| `shared_learning/markdown_store.rs` | Primary backend candidate | High |
| `main.rs` shared-learning CLI | Must keep working unchanged | Medium |
| `.docs/research-cross-platform-data-dirs.md` | Defines the approved path direction | Medium |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `tokio::fs` | Existing | Low | None needed |
| `serde_yaml` | Existing | Low | None needed |
| `directories` crate or shared helper using it | Already used elsewhere in repo | Low | Keep `dirs`, but that conflicts with the approved path direction |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Partial frontmatter causes silent metadata loss | High | High | Expand frontmatter to cover all persisted `SharedLearning` fields and add restart round-trip tests |
| Shared-directory files are indexed as duplicates | Medium | High | Load all markdown files, then de-duplicate by `learning.id`, preferring the non-shared path when both copies exist |
| Path migration changes where files are stored | Medium | Medium | Keep `TERRAPHIM_LEARNINGS_DIR` override and support sparse old frontmatter when reading existing files |
| Existing tests assume in-memory semantics | Medium | Medium | Move tests to temp-directory-backed durable stores and add restart coverage |

### Open Questions

1. Should Step 6 include a small helper for consistent Terraphim data paths, or is adding `directories` directly to `terraphim_agent` acceptable?
2. Resolved during approval: read everything and de-duplicate by `learning.id`, preferring a non-shared path when both canonical and shared copies exist.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Existing Step 1 markdown files may already exist with sparse frontmatter | `markdown_store.rs` already shipped with a reduced frontmatter schema | Old files could fail to load after migration if parsing becomes strict | Partially |
| `SharedLearningStore` should remain the public API | CLI and integration tests already use it directly | Wider refactor than Step 6 if wrong | Yes |
| When the same learning exists in both canonical and shared locations, the non-shared copy should win during load | Shared copies are distribution artefacts, while canonical files are the editable source of truth | Stale shared copy could override fresher canonical metadata if wrong | No |
| We do not need a `terraphim_persistence` abstraction for Step 6 | The filesystem backend already exists and works; current persistence field is effectively dead code | Extra abstraction work if other code requires it later | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Replace `SharedLearningStore` entirely with `MarkdownLearningStore` | Simpler type graph, but forces CLI and algorithm rewrites | Rejected; too broad for Step 6 |
| Keep `SharedLearningStore` and swap only the persistence backend | Preserves caller API and existing ranking logic | Chosen |
| Persist canonical learnings and also auto-write to `shared/` on promotion | Useful later, but changes behaviour and risks duplicate reads | Rejected for this step |

## Research Findings

### Key Insights

1. The real Step 6 problem is not "build a storage abstraction"; it is "remove fake durability without disturbing the higher-level store behaviour".
2. The current markdown backend is close, but its frontmatter is incomplete relative to `SharedLearning`.
3. The cleanest migration path is to delete the unused `DeviceStorage` dependency from `store.rs`, not add more abstraction on top of it.
4. Canonical-store loading and shared-directory publication need separate concepts, even if both locations are loaded and then collapsed into one in-memory record per `learning.id`.

### Relevant Prior Art

- `crates/terraphim_orchestrator/src/learning.rs`: separates high-level store behaviour from persistence implementation; useful as a structural reference, even though the agent store can stay more minimal.
- `.docs/research-cross-platform-data-dirs.md`: already recommends `ProjectDirs` for cross-platform local data.

### Technical Spikes Needed

No separate spike is required before implementation. The remaining work is design-level, not exploratory.

## Recommendations

### Proceed/No-Proceed

Proceed.

### Scope Recommendations

- Keep `SharedLearningStore` as the public API.
- Replace `DeviceStorage`/`SharedLearningRecord` usage in `store.rs` with `MarkdownLearningStore`.
- Extend markdown frontmatter to persist the full `SharedLearning` state.
- Load all markdown learning locations and de-duplicate by `learning.id` during startup hydration.
- Align default paths with the `ProjectDirs` recommendation while preserving explicit environment override.

### Risk Mitigation Recommendations

- Add round-trip tests that verify `quality`, `keywords`, `applicable_agents`, `verify_pattern`, `promoted_at`, and `updated_at` survive restart.
- Add a restart test for `SharedLearningStore` using a temp directory.
- Keep backwards-compatible parsing for old sparse markdown files by making newly added frontmatter fields optional with safe defaults.

## Next Steps

If approved:

1. Write the implementation plan for Step 6.
2. Implement the markdown round-trip expansion and canonical loading changes.
3. Run focused unit and integration tests for `shared-learning`.

## Appendix

### Reference Materials

- `crates/terraphim_agent/src/shared_learning/store.rs`
- `crates/terraphim_agent/src/shared_learning/markdown_store.rs`
- `crates/terraphim_agent/src/shared_learning/types.rs`
- `crates/terraphim_agent/tests/shared_learning_cli_tests.rs`
- `.docs/design-learning-kg-2026-04-17.md`
- `.docs/research-cross-platform-data-dirs.md`
