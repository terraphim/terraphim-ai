# Documentation Gap Report -- 2026-05-07 (Afternoon Run)

**Generated:** 2026-05-07 06:45 CEST
**Agent:** Ferrox (documentation-generator)
**Method:** Scan public items (`pub fn`, `pub async fn`, `pub struct`, `pub enum`, `pub trait`, `pub type`) lacking `///` doc comments

---

## Summary

| Metric | Value |
|--------|-------|
| Crates scanned | 9 |
| Rust files changed since v2 (morning) | 0 |
| Total missing documentation items | **395** (unchanged from v2) |
| New gaps introduced today | 0 |

No Rust source changes since the v2 scan at 05:43 CEST. All counts carry forward unchanged.

---

## Per-Crate Breakdown (unchanged from v2)

| Crate | Missing Items | Top File |
|-------|---------------|----------|
| terraphim_agent | 189 | client.rs (59 gaps) |
| terraphim_orchestrator | 61 | control_plane/routing.rs (11 gaps) |
| terraphim_types | 48 | score/mod.rs (14 gaps) |
| terraphim_service | 39 | error.rs (13 gaps) |
| terraphim_automata | 23 | builder.rs (14 gaps) |
| terraphim_persistence | 12 | conversation.rs (6 gaps) |
| terraphim_config | 11 | lib.rs (11 gaps) |
| terraphim_middleware | 8 | command/ripgrep.rs (3 gaps) |
| terraphim_rolegraph | 4 | lib.rs (4 gaps) |

---

## Spec Validation Cross-Reference

The spec validation report (`reports/spec-validation-20260507.md`) confirms:
- **FAIL** status: 2 persistent gaps remain unresolved
- No new spec violations introduced today
- `meta_coordinator.rs` (741 lines, added in build-runner commit) has no corresponding spec document -- flagged as a documentation gap risk

---

## API Reference Snippets (carried from v2)

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

/// Routes agent tasks to the appropriate executor based on budget and availability.
///
/// Integrates with [`BudgetPressure`] for back-pressure-aware dispatching.
pub enum RouteSource { /* ... */ }
```

---

## Recommendations

### Priority 1 -- Immediate

1. `terraphim_orchestrator` meta_coordinator.rs: New 741-line file with zero doc comments. Requires crate-level doc (`//!`) and docs on at least the public structs and entry-point methods.
2. `terraphim_service` lib.rs: Document `TerraphimService`, `Result`, `ServiceError` (3 lines each).
3. `terraphim_persistence` lib.rs: Document `DeviceStorage::instance()`.

### Priority 2 -- High

4. `terraphim_agent` client.rs: Document `ApiClient` and its 10 public methods.
5. `terraphim_types` score/mod.rs: Document `Scorer` trait and `sort_documents`.

### Priority 3 -- Ongoing

6. CI lint gate: `cargo doc --no-deps 2>&1 | grep "missing documentation"` as a soft-warning step.

---

## Comparison to Prior Reports

| Date | Run | Scope | Items |
|------|-----|-------|-------|
| 2026-04-29 | morning | struct/enum/trait | 564 |
| 2026-05-07 | v1 morning | struct/enum/trait | 307 (-45%) |
| 2026-05-07 | v2 morning | + fn/async fn | 395 |
| 2026-05-07 | v3 afternoon | + fn/async fn | 395 (unchanged) |

Theme-ID: doc-gap
