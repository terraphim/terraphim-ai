# Documentation Gap Report

**Generated:** 2026-05-07
**Agent:** Ferrox (documentation-generator)
**Scope:** Nine core crates scanned for missing `///` doc comments on public items (`pub fn`, `pub struct`, `pub enum`, `pub trait`, `pub type`)

---

## Executive Summary

| Metric | Count |
|--------|-------|
| Crates scanned | 9 |
| Total missing documentation items | **307** |
| Crates with zero missing top-level docs | 4 |
| Worst offender | `terraphim_agent` (139 items) |

Improvement from prior report (2026-04-29: 564 items): **-257 items** (-45%).

---

## Per-Crate Breakdown

| Crate | Missing Items | Top Category |
|-------|---------------|--------------|
| terraphim_agent | 139 | struct / fn (listener, robot, service) |
| terraphim_types | 76 | struct field, type alias, newtype |
| terraphim_orchestrator | 54 | method (mention, scheduler, pr_poller) |
| terraphim_automata | 23 | struct (markdown_directives module) |
| terraphim_config | 15 | type alias, enum, struct |
| terraphim_middleware | 0 | -- |
| terraphim_rolegraph | 0 | -- |
| terraphim_service | 0 | -- |
| terraphim_persistence | 0 | -- |

---

## Critical Gaps (User-Facing Public API)

### terraphim_agent (139 gaps)

**Missing struct documentation**
- `src/listener.rs:10` -- `AgentIdentity` (core public type)
- `src/listener.rs:19` -- `AgentIdentity::new()` constructor

**Missing module documentation**
- `src/lib.rs` -- crate root has no `//!` overview

**Missing trait documentation**
- Multiple robot-mode traits lack doc comments explaining contracts

### terraphim_types (76 gaps)

**Missing newtype documentation**
- `src/lib.rs:168` -- `RoleName`
- `src/lib.rs:259` -- `NormalizedTermValue`
- `src/lib.rs:262` -- `NormalizedTermValue::new()`

**Pattern:** Newtype wrappers consistently lack documentation. These are public API surface.

### terraphim_orchestrator (54 gaps)

**Missing method documentation**
- `src/mention.rs:545` -- `new()` on mention type
- `src/scheduler.rs:80` -- `take_event_rx()` (important lifecycle method)
- `src/pr_poller.rs:124` -- `new()` constructor

**Note:** Recent commit `484a8e71b` (Refs #1275) wired `meta_coordinator` into `lib.rs` -- that module's public items are now reachable but undocumented.

### terraphim_automata (23 gaps)

**Missing struct documentation**
- `src/markdown_directives.rs:9` -- `MarkdownDirectiveWarning`
- `src/markdown_directives.rs:16` -- `MarkdownDirectivesParseResult`
- `src/markdown_directives.rs:21` -- `parse_markdown_directives_dir()`

**Note:** The core automata types are well-documented; gaps are concentrated in the `markdown_directives` module added recently.

### terraphim_config (15 gaps)

**Missing documentation**
- `src/lib.rs:31` -- `Result<T>` type alias
- `src/lib.rs:38` -- `TerraphimConfigError` enum
- `src/lib.rs:201` -- `Role` struct

---

## Qualitative Gaps (found by manual inspection)

These items have doc comments but they are minimal or misleading:

| Crate | Item | Issue |
|-------|------|-------|
| `terraphim_service` | `LlmClient` trait | Trait undocumented; only methods have docs |
| `terraphim_service` | `SummarizeOptions`, `ChatOptions` | Struct purpose not explained |
| `terraphim_persistence` | `Persistable` trait | Method relationships unclear |
| `terraphim_orchestrator` | `PreCheckResult`, `AgentStatus` | Minimal -- no examples |
| `terraphim_middleware` | `Error` enum | Variants lack context |

---

## Recommendations

### Priority 1 -- Highest user impact

1. **`terraphim_types`**: Add one-line doc to every newtype wrapper. These are the shared currency of the whole workspace.
2. **`terraphim_agent`**: Add crate-level `//!` overview and document `AgentIdentity`.
3. **`terraphim_orchestrator`**: Document `meta_coordinator` public API now that the module is wired in (#1275).

### Priority 2 -- Developer ergonomics

4. **`terraphim_config`**: Document `Result`, `TerraphimConfigError`, and `Role` -- the three most-imported items.
5. **`terraphim_automata`**: Document `markdown_directives` module.

### Priority 3 -- Qualitative improvement

6. Expand minimal docs on `LlmClient`, `Persistable`, `PreCheckResult` with examples.

---

## Methodology

Items counted as "missing" if:
- A `pub fn`, `pub struct`, `pub enum`, `pub trait`, or `pub type` declaration
- Has no `///` doc comment on the three preceding lines
- Attribute lines (`#[...]`) are skipped when looking back

Struct *fields* are not counted in this report (would add ~500 additional items).

---

## Comparison to Prior Reports

| Date | Items Missing | Delta |
|------|---------------|-------|
| 2026-04-29 | 564 | baseline |
| 2026-05-07 | 307 | **-257 (-45%)** |

The reduction reflects substantial documentation work on `terraphim_middleware`, `terraphim_service`, `terraphim_rolegraph`, and `terraphim_persistence` between the two snapshots.
