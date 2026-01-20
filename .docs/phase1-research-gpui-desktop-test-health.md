# Research Document: GPUI Desktop Test/Build Health

## 1. Problem Restatement and Scope
`cargo test -p terraphim_desktop_gpui` currently fails to compile and therefore cannot be used to confirm that the GPUI desktop crate is healthy.

Observed failure modes (from a local `cargo test -p terraphim_desktop_gpui` run):
- The test suite references public API items that do not exist (e.g., `terraphim_desktop_gpui::components`, `terraphim_desktop_gpui::rolegraph`, `views::search::SearchComponent`).
- There is at least one hard syntax error in a test file (`crates/terraphim_desktop_gpui/tests/ui_test_runner.rs`) that prevents compilation.
- Some tests import `ContextManager` from `terraphim_service::context`, but the service module appears to expose `TerraphimContextManager` (or a differently named type), causing unresolved import errors.

IN SCOPE
- Restoring a consistent contract between `crates/terraphim_desktop_gpui` public exports and the tests under `crates/terraphim_desktop_gpui/tests`.
- Fixing compile errors in tests and crate exports so `cargo test -p terraphim_desktop_gpui` compiles and runs.
- Determining which tests are “current” vs “legacy”, and adding clear gating (feature flags or module organization) so the test suite reflects the intended product state.

OUT OF SCOPE
- Implementing new GPUI features unrelated to test/build health.
- Large-scale refactors of GPUI UI/UX.
- Changing behavior of backend services beyond what’s required for API compatibility.

## 2. User & Business Outcomes
- Engineers can run a single command (`cargo test -p terraphim_desktop_gpui`) to verify GPUI desktop health.
- Reduced uncertainty: the test suite reflects the current architecture (GPUI views + state modules), not a previously drafted component framework.
- Faster iteration: fewer “paper tests” that fail due to stale imports and missing exports.

## 3. System Elements and Dependencies
Key elements involved in the failure:

- `crates/terraphim_desktop_gpui/src/lib.rs`
  - Defines the crate’s public module surface.
  - Currently has `pub mod components;` commented out, which makes `terraphim_desktop_gpui::components` missing at the crate root.

- `crates/terraphim_desktop_gpui/src/components/mod.rs`
  - Exists and defines a “reusable components” architecture.
  - Many submodules and re-exports are commented out (e.g., `testing`, `knowledge_graph`, `kg_search_modal`, etc.).
  - This suggests the components system is intentionally disabled or partially incomplete.

- `crates/terraphim_desktop_gpui/src/views/search/mod.rs`
  - Provides GPUI-native view types like `SearchView`, and exports `SearchInput`, `SearchResults`, `TermChips`.
  - Does NOT provide `SearchComponent`/`SearchComponentConfig` referenced by tests.

- `crates/terraphim_desktop_gpui/tests/*.rs`
  - Many tests reference the disabled “components” and other types that aren’t exported.
  - At least one file contains a syntax error (type parameter parsing error) that blocks compilation.

- `crates/terraphim_service/src/context.rs`
  - Provides a context manager type named `TerraphimContextManager`.
  - Some GPUI desktop tests expect `ContextManager` to exist in this module.

External dependencies impacting build/test:
- `gpui`, `gpui-component`, macOS platform integration crates.

## 4. Constraints and Their Implications
- Must compile and run on macOS (darwin).
  - Many GPUI dependencies are platform-specific; tests should avoid requiring a full UI runtime unless explicitly intended.

- “No mocks in tests” constraint.
  - Tests that define `MockServiceRegistry` (seen in `crates/terraphim_desktop_gpui/tests/ui_integration_tests.rs`) are likely non-compliant with repository policy.
  - We need to clarify whether this policy applies to this crate’s tests, or if these tests should be refactored to use real in-memory implementations.

- Keep scope disciplined.
  - The fastest path to “everything works correctly” is to ensure the crate exports and the test suite align with the current architecture, not to resurrect an entire legacy component system.

- Stability vs breadth.
  - Re-enabling `components` wholesale may balloon compile surface and introduce more failures.
  - Feature-gating legacy components/tests may reduce blast radius while still letting the main crate test suite pass.

## 5. Risks, Unknowns, and Assumptions
UNKNOWNS
- Which test files are authoritative for current GPUI desktop behavior vs experimental/legacy.
- Why `components` was disabled in `src/lib.rs` (performance? compile breakages? ongoing migration?).
- Whether the repository intends `ContextManager` to be a stable alias for `TerraphimContextManager`.
- Whether CI expects these tests to run, or if they are local-only.

ASSUMPTIONS
- Primary health check is `cargo test -p terraphim_desktop_gpui` on macOS.
- The “current” GPUI UI implementation is the `views/*` + `state/*` path, and the “reusable components” subsystem is not yet productized.

RISKS
- Re-enabling the components system could introduce cascading compile failures and slow builds.
- Disabling tests without agreement could reduce coverage and hide regressions.
- Renaming or aliasing service APIs (e.g., `ContextManager`) may affect other crates.

De-risking steps (information gathering, not implementation):
- Categorize tests into: (a) compile + logic tests for current code, (b) legacy components tests, (c) experimental visual/UI runner code.
- Decide “definition of done” for ‘everything works’ (tests pass, build passes, app launches).

## 6. Context Complexity vs. Simplicity Opportunities
Current complexity drivers:
- Two parallel architectures appear present:
  - GPUI-aligned `views/*` + `state/*`.
  - A legacy/experimental `components/*` system that is partially disabled.
- Tests seem written against both (or against an earlier shape), creating broken contracts.

Simplicity opportunities:
- Establish one explicit public API surface for the crate (current GPUI views + state), and gate legacy components behind a feature flag.
- Make tests mirror that same split:
  - Default tests validate current architecture.
  - Legacy tests run only under an opt-in feature.

## 7. Questions for Human Reviewer
1. What is the definition of “everything works correctly” here: `cargo test` only, or also `cargo run -p terraphim_desktop_gpui` (manual UI smoke test)?
2. Should the `components/*` subsystem be considered legacy/experimental (feature-gated), or should it be restored as a first-class API?
3. Is it acceptable to move/feature-gate tests that currently rely on `MockServiceRegistry` given the “never use mocks” rule?
4. Do we want to preserve the older `ContextManager` name as an alias in `terraphim_service::context`, or should tests be updated to the new type name?
5. Which subset of tests must pass to declare GPUI desktop “green” (unit/integration only, or also visual runners)?
