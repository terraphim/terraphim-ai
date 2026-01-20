# Design & Implementation Plan: Restore GPUI Desktop Test/Build Health

## 1. Summary of Target Behavior
After this change:
- `cargo test -p terraphim_desktop_gpui` compiles and runs successfully on macOS.
- The default test suite validates the *current* GPUI architecture (`views/*` + `state/*` + `search_service`, etc.).
- Legacy/experimental tests and benchmark code that target the disabled `components/*` subsystem do not block default test runs.

## 2. Key Invariants and Acceptance Criteria
Invariants
- No changes to end-user behavior are required to reach “green tests”; focus is build/test correctness.
- Default test command must be stable: no optional features required, no manual steps.
- Avoid introducing or expanding mock-based testing; tests should use real in-memory types where feasible.

Acceptance Criteria
- `cargo test -p terraphim_desktop_gpui` exits 0.
- `cargo test -p terraphim_desktop_gpui --tests` exits 0.
- `cargo build -p terraphim_desktop_gpui` exits 0.

## 3. High-Level Design and Boundaries
Boundary decision: split the crate/test surface into two explicit layers.

- “Current GPUI App Surface” (default)
  - Crate exports: `app`, `views`, `state`, `search_service`, `slash_command`, `markdown`, etc.
  - Tests: should import these modules and exercise real behavior.

- “Legacy Reusable Components” (opt-in)
  - Crate exports: `components` (and its dependent submodules) behind a feature flag, OR kept internal.
  - Tests: any tests that reference `terraphim_desktop_gpui::components::*` are moved behind the same feature, or are rewritten to use the current surface.

We do NOT attempt to fully revive all commented-out submodules in `crates/terraphim_desktop_gpui/src/components/mod.rs` unless explicitly required.

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|---|---|---|---|---|
| `crates/terraphim_desktop_gpui/src/lib.rs` | Modify | `components` disabled at crate root | Add feature-gated export: `#[cfg(feature = "legacy-components")] pub mod components;` (or keep disabled and gate tests) | Cargo features |
| `crates/terraphim_desktop_gpui/Cargo.toml` | Modify | No explicit test/legacy feature split | Add `legacy-components` feature; optionally add `legacy-benches` feature | Cargo features |
| `crates/terraphim_desktop_gpui/tests/ui_test_runner.rs` | Fix | Syntax error prevents compilation | Fix the generics/where-clause syntax so tests compile | Rust syntax |
| `crates/terraphim_desktop_gpui/tests/*components*_tests.rs` | Modify | Imports `terraphim_desktop_gpui::components` (missing) | Either (A) gate entire file with `#![cfg(feature = "legacy-components")]` or (B) rewrite tests to current surface | Feature gating decision |
| `crates/terraphim_desktop_gpui/tests/*journey*test*.rs` | Modify | Imports `ContextManager` from `terraphim_service::context` (missing) | Update imports to `TerraphimContextManager` OR add a compatibility `pub type ContextManager = TerraphimContextManager;` in `terraphim_service::context` | Cross-crate API |
| `crates/terraphim_desktop_gpui/benches/*` | Modify | Bench files contain compile errors; benches might be built by some workflows | Gate benches behind feature OR fix them; ensure default `cargo test` not impacted | Criterion, types |

## 5. Step-by-Step Implementation Sequence
1. Establish intended “default surface” and “legacy surface” in code via Cargo features.
2. Fix the hard syntax error in `crates/terraphim_desktop_gpui/tests/ui_test_runner.rs` so the test crate can compile.
3. Resolve the `ContextManager` naming mismatch:
   - Preferred: add a backward-compatible alias in `crates/terraphim_service/src/context.rs`.
   - Alternative: update all tests to the new type name.
4. Make `components` tests non-blocking:
   - Option A (fastest, smallest blast radius): gate those tests behind `legacy-components`.
   - Option B (more coverage now): re-export `components` from crate root and ensure minimal required submodules compile; gate the rest.
5. Resolve `views::search::SearchComponent` mismatches:
   - Either gate tests that reference `SearchComponent`, or update them to use `SearchView` (current API).
6. Run verification commands:
   - `cargo build -p terraphim_desktop_gpui`
   - `cargo test -p terraphim_desktop_gpui`
   - If legacy tests are kept: `cargo test -p terraphim_desktop_gpui --features legacy-components` (optional).

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---|---|---|
| `cargo test -p terraphim_desktop_gpui` passes | Build + integration | `crates/terraphim_desktop_gpui/tests/*.rs` (default set) |
| Current search view compiles and links | Build | `crates/terraphim_desktop_gpui/src/views/search/mod.rs` |
| Context flow tests use correct service type | Integration | `crates/terraphim_desktop_gpui/tests/complete_user_journey_test.rs`, `crates/terraphim_desktop_gpui/tests/end_to_end_flow_test.rs` |
| Legacy components tests do not block default run | Build gate | `#![cfg(feature = "legacy-components")]` on legacy files |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|---|---|---|
| Feature-gating hides tests | Keep an explicit opt-in `legacy-components` test run documented; add CI later if desired | Some drift possible |
| Re-exporting `components` increases compile surface | Prefer gating tests first; only re-export when needed | Potential new compile errors |
| “No mocks in tests” conflict | Refactor mock registry tests to use real in-memory registry implementations OR gate them as legacy | May require more work to keep coverage |
| Cross-crate alias impacts others | Use a type alias only; avoid behavior change | Low |

## 8. Open Questions / Decisions for Human Review
1. Should `components` be restored as a supported API now, or treated as `legacy-components` feature-only?
2. Are we allowed to add `pub type ContextManager = TerraphimContextManager;` for backwards compatibility in `crates/terraphim_service/src/context.rs`?
3. For “everything works”: is passing `cargo test -p terraphim_desktop_gpui` sufficient, or do you also want a manual `cargo run -p terraphim_desktop_gpui` smoke test?
