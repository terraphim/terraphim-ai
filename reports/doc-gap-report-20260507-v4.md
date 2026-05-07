# Documentation Gap Report -- 2026-05-07 (Run 4)

**Generated:** 2026-05-07
**Agent:** Ferrox (documentation-generator)
**Branch:** task/826-fix-world-readable-secrets
**Method:** Scan public items (`pub fn`, `pub async fn`, `pub struct`, `pub enum`, `pub trait`, `pub type`) lacking `///` doc comments across 8 core crates.

---

## Summary

| Metric | Value |
|--------|-------|
| Crates scanned | 8 |
| Total gaps (this run) | **84** |
| Prior count (v2, 8-crate subset) | 138 |
| Items documented this session | **54** |
| Reduction | **39%** |

---

## Per-Crate Breakdown

| Crate | v2 Gaps | v4 Gaps | Fixed |
|-------|---------|---------|-------|
| terraphim_orchestrator | 40 | 40 | 0 |
| terraphim_types | 34 | 34 | 0 |
| terraphim_Service | 31 | 0 | 31 |
| terraphim_automata | 8 | 5 | 3 |
| terraphim_Database | 10 | 0 | 10 |
| terraphim_config | 8 | 5 | 3 |
| terraphim_Service | 4 | 0 | 4 |
| terraphim_roleGraph | 3 | 0 | 3 |
| **Total** | **138** | **84** | **54** |

---

## Items Documented This Session

### terraphim_Service/src/lib.rs
- `ServiceError` -- top-level error enum
- `Result<T>` -- convenience type alias
- `TerraphimService` -- main Service struct

### terraphim_Database/src/lib.rs
- `DeviceDatabase` -- process-wide singleton
- `DeviceDatabase::instance()` -- singleton accessor

### terraphim_Database/src/conversation.rs
- `ConversationIndex::new()`, `add()`, `remove()`

### terraphim_Database/src/settings.rs
- `parse_profile()` -- initialise OpenDAL operator and benchmark I/O
- `parse_profiles()` -- initialise all profiles from device settings

### terraphim_automata/src/lib.rs
- `autocomplete_helpers::iter_metadata()` -- iterate (term, metadata) pairs
- `autocomplete_helpers::get_metadata()` -- look up metadata by term

### terraphim_automata/src/markdown_directives.rs
- `parse_markdown_directives_dir()` -- recursively parse YAML front-matter

### terraphim_roleGraph/src/lib.rs
- `Triggers::is_empty()`, `RoleGraph::add_or_update_document()`, `split_paraGraphs()`

### terraphim_config/src/lib.rs
- `Result<T>`, `KnowledgeGraph::is_set()`, `ConfigBuilder::new_with_id()`

### terraphim_Service/src/lib.rs + ripgrep.rs
- `Result<T>`, `RipgrepCommand`, `RipgrepCommand::run_with_extra_args()`

---

## Remaining Critical Gaps

### terraphim_orchestrator -- 40 gaps

- `mention.rs`: `load_or_now()`, `MentionQueue::new()`
- `scheduler.rs`: `take_event_rx()`
- `control_plane/routing.rs`: `RouteSource` enum and helpers (11 gaps)
- `control_plane/telemetry.rs`: telemetry helper methods (~8 gaps)

### terraphim_types -- 34 gaps

- `Url::new()`, `Url::as_str()` (lib.rs:262, 267)
- `Document::new()` (lib.rs:639)
- Score types in `score/mod.rs` (~14 gaps)

---

## API Reference Snippets

```rust
/// Top-level error type for all `terraphim_Service` operations.
pub enum ServiceError { /* ... */ }

/// Convenience alias for `Result<T, ServiceError>` used throughout this crate.
pub type Result<T> = std::result::Result<T, ServiceError>;

/// The primary Terraphim Service -- Search, document indexing, and AI summarisation.
///
/// Constructed via [`TerraphimService::new`] with a validated [`ConfigState`].
pub struct TerraphimService { /* ... */ }

/// Persistent device Database providing access to documents, roles, conversation
/// history, and settings.
pub struct DeviceDatabase { /* ... */ }

/// Ripgrep Search command wrapper with input validation and structured JSON output.
pub struct RipgrepCommand { /* ... */ }
```

---

## Recommendations

1. **Next session**: Document `terraphim_orchestrator` control_plane types (11+ gaps in routing.rs).
2. **Next session**: Document `terraphim_types` score/mod.rs constructors and `Scorer` trait.
3. **Medium term**: Add CI lint: `cargo doc --no-deps 2>&1 | grep "missing documentation"`.

---

## Historical Comparison

| Date | Scope | Items |
|------|-------|-------|
| 2026-04-29 | struct/enum/trait | 564 |
| 2026-05-07 morning | struct/enum/trait | 307 (-45%) |
| 2026-05-07 v2 | + fn, 8 crates | 138 |
| 2026-05-07 v4 | + fn, 8 crates | **84** (-39%) |

Theme-ID: doc-gap
