# Documentation Gap Report -- 2026-05-07 (Run 2)

**Generated:** 2026-05-07 05:43 CEST
**Agent:** Ferrox (documentation-generator)
**Method:** Scan public items (`pub fn`, `pub async fn`, `pub struct`, `pub enum`, `pub trait`, `pub type`) lacking `///` doc comments

---

## Summary

| Metric | Value |
|--------|-------|
| Crates scanned | 9 |
| Total missing documentation items | **395** |
| Prior count (morning run, struct/enum only) | 307 |
| Methodology difference | +88 (fn-level items now included) |
| Rust files changed since morning run | 0 |

No Rust code changed between the two scans; count difference is purely methodology (this run includes `pub fn`/`pub async fn`).

---

## Per-Crate Breakdown

| Crate | Missing Items | Top File |
|-------|---------------|----------|
| terraphim_agent | 189 | client.rs (59 gaps) |
| terraphim_types | 48 | score/mod.rs (14 gaps) |
| terraphim_orchestrator | 61 | control_plane/routing.rs (11 gaps) |
| terraphim_automata | 23 | builder.rs (14 gaps) |
| terraphim_config | 11 | lib.rs (11 gaps) |
| terraphim_service | 39 | error.rs (13 gaps) |
| terraphim_persistence | 12 | conversation.rs (6 gaps) |
| terraphim_middleware | 8 | command/ripgrep.rs (3 gaps) |
| terraphim_rolegraph | 4 | lib.rs (4 gaps) |

---

## Critical Gaps (Public API Entry Points)

### terraphim_service -- 39 gaps

Crate-level public API is undocumented:

- `lib.rs:68` -- `ServiceError` (root error type)
- `lib.rs:122` -- `Result<T>` type alias
- `lib.rs:124` -- `TerraphimService` (main service struct)
- `llm.rs:21` -- `SummarizeOptions`
- `llm.rs:27` -- `ChatOptions`
- `llm.rs:33` -- `LlmClient` trait (no trait-level doc)
- `error.rs:116-133` -- error constructor methods have no doc

### terraphim_persistence -- 12 gaps

- `lib.rs:41` -- `DeviceStorage` struct
- `lib.rs:47` -- `DeviceStorage::instance()`
- `conversation.rs:42-56` -- conversation mutation methods
- `error.rs:4` -- `Error` enum
- `settings.rs:164,349` -- `parse_profile`, `parse_profiles`

### terraphim_orchestrator -- 61 gaps (new: control_plane)

New `control_plane/` sub-module (routing, telemetry, policy) added recently:

- `control_plane/routing.rs:17` -- `RouteSource` enum (11 total gaps in file)
- `control_plane/routing.rs:38` -- `BudgetPressure`
- `control_plane/telemetry.rs:70-95` -- telemetry helper methods
- `flow/config.rs` -- 4 flow configuration types undocumented

### terraphim_agent -- 189 gaps

`client.rs` is the worst single file (59 gaps). `ApiClient` struct and all its methods are undocumented.

---

## API Reference Snippets

Recommended rustdoc stubs for the highest-priority items:

```rust
/// The primary Terraphim service, providing search, indexing, and AI summarisation.
///
/// Constructed via [`TerraphimService::new`] with a validated configuration.
/// All search operations are async and cancellable.
pub struct TerraphimService { /* ... */ }

/// Persistent device storage, providing access to documents, roles, and settings.
///
/// Implemented as a singleton per device profile. Use [`DeviceStorage::instance`]
/// to obtain the shared handle.
pub struct DeviceStorage { /* ... */ }

/// Trait for language-model backend implementations.
///
/// Implement this to wire in a new LLM provider. The two required methods are
/// [`LlmClient::summarize`] and [`LlmClient::chat`].
pub trait LlmClient { /* ... */ }
```

---

## Recommendations

### Priority 1 -- Immediate

1. `terraphim_service` lib.rs: Add three-line crate doc (`//!`) and doc comments on `TerraphimService`, `Result`, `ServiceError`.
2. `terraphim_persistence` lib.rs: Document `DeviceStorage` and `instance()`.
3. `terraphim_orchestrator` control_plane: Batch-document the routing and telemetry types (recently added, no docs yet).

### Priority 2 -- High

4. `terraphim_agent` client.rs: Document `ApiClient` and its 10 public methods.
5. `terraphim_types` score/mod.rs: Document `Scorer` trait and `sort_documents`.

### Priority 3 -- Ongoing

6. Introduce CI lint: `cargo doc --no-deps 2>&1 | grep "missing documentation"` as a soft warning gate.

---

## Comparison to Prior Reports

| Date | Scope | Items |
|------|-------|-------|
| 2026-04-29 | struct/enum/trait | 564 |
| 2026-05-07 morning | struct/enum/trait | 307 (-45%) |
| 2026-05-07 afternoon | + fn/async fn | 395 |

Theme-ID: doc-gap
