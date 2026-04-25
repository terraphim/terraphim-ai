# Design & Implementation Plan: Add Action Metadata to NormalizedTerm

**Issue**: #735 — `feat(types): add action metadata to NormalizedTerm beyond url field`
**Author**: Agent
**Date**: 2026-04-22
**Phase**: 2 — Design

---

## 1. Summary of Target Behavior

After implementation, `NormalizedTerm` carries four additional optional metadata fields:

| Field | Type | Description |
|-------|------|-------------|
| `action` | `Option<String>` | CLI action template with `{{ model }}` and `{{ prompt }}` placeholders |
| `priority` | `Option<u8>` | Routing tiebreaking priority (higher = preferred) |
| `trigger` | `Option<String>` | Pattern or alias that activates this term |
| `pinned` | `bool` | Whether the term is pinned (default `false`) |

`AutocompleteMetadata` (in `terraphim_automata`) receives the same four fields in the same PR to stay in sync.

All new fields are `Option<T>` (or `bool` with `#[serde(default)]`), ensuring backward-compatible JSON deserialization: existing serialised `NormalizedTerm` data remains valid.

---

## 2. Key Invariants and Acceptance Criteria

### Invariants
- `NormalizedTerm` JSON can be deserialised regardless of which optional fields are present
- Adding fields does not change the meaning of existing `id`, `value`, `display_value`, or `url` fields
- `AutocompleteMetadata` and `NormalizedTerm` remain structurally aligned for their shared fields

### Acceptance Criteria

| # | Criterion | Test Type | Location |
|---|-----------|-----------|----------|
| AC1 | `NormalizedTerm::new()`, `with_auto_id()`, `with_display_value()`, `with_url()` still compile and produce terms without the new fields set | Unit | New test in `terraphim_types` |
| AC2 | `NormalizedTerm` deserialises from JSON missing all new fields | Unit | New test in `terraphim_types` |
| AC3 | `NormalizedTerm` serialises with all four new fields present | Unit | New test in `terraphim_types` |
| AC4 | `NormalizedTerm::action()` / `::priority()` / `::trigger()` / `::pinned()` builder methods work | Unit | New test in `terraphim_types` |
| AC5 | `AutocompleteMetadata` gains the same four fields | Unit | New test in `terraphim_automata` |
| AC6 | `cargo build --workspace` succeeds with no new warnings | Build | CI |
| AC7 | All existing tests pass (`cargo test --workspace`) | Integration | CI |

---

## 3. High-Level Design and Boundaries

### Components

```
crates/terraphim_types/src/lib.rs
  └── NormalizedTerm struct (MODIFY)
        + action: Option<String>
        + priority: Option<u8>
        + trigger: Option<String>
        + pinned: bool
      + builder methods: with_action(), with_priority(), with_trigger(), with_pinned()
      + getter methods: action(), priority(), trigger(), pinned()

crates/terraphim_automata/src/autocomplete.rs
  └── AutocompleteMetadata struct (MODIFY)
        + action: Option<String>
        + priority: Option<u8>
        + trigger: Option<String>
        + pinned: bool
```

### Boundaries
- **Inside**: Adding fields to `NormalizedTerm` and `AutocompleteMetadata`, including serde attributes and builder/getter methods
- **Outside**: No changes to `MarkdownDirectives`, `RouteDirective`, `RoutingRule`, `Thesaurus`, or any storage layer
- **New dependencies**: None — only standard library + existing serde/ahash imports

### Anti-Patterns Avoided
- No wrapper struct (per user decision)
- No changes to JSON schema for existing persisted data (backward-compatible `Option` fields)
- No migration of existing data

---

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/terraphim_types/src/lib.rs` | Modify | `NormalizedTerm` with 4 fields | `NormalizedTerm` with 8 fields; 4 new builder methods; 4 getter methods | None new |
| `crates/terraphim_types/src/lib.rs` | Modify | `AutocompleteMetadata` with 4 fields | `AutocompleteMetadata` with 8 fields | None new |
| `crates/terraphim_automata/src/autocomplete.rs` | Modify | `AutocompleteMetadata` with 4 fields | `AutocompleteMetadata` with 8 fields; serde attributes updated | `terraphim_types` re-export |

---

## 5. Step-by-Step Implementation Sequence

### Step 1: Add fields to `NormalizedTerm` in `terraphim_types/src/lib.rs`
- Add `action: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`
- Add `priority: Option<u8>` with same serde attributes
- Add `trigger: Option<String>` with same serde attributes
- Add `pinned: bool` with `#[serde(default)]`
- Add builder methods: `with_action(self, String)`, `with_priority(self, u8)`, `with_trigger(self, String)`, `with_pinned(self, bool)`
- Add getter methods: `action(&self) -> Option<&String>`, `priority(&self) -> Option<&u8>`, `trigger(&self) -> Option<&String>`, `pinned(&self) -> bool`
- Update `new()` to initialise all new fields to default values
- Update `with_auto_id()` similarly
- Update existing `with_url()` return type to `Self` (already correct for builder pattern)

**Deployable state**: Yes — compiles, all old code path-compatible

### Step 2: Add tests for `NormalizedTerm` in `terraphim_types`
- Test deserialisation from JSON missing all new fields
- Test serialisation with all fields set
- Test builder chain with all new methods
- Test `display()` still works with new fields

**Deployable state**: Yes — tests only

### Step 3: Update `AutocompleteMetadata` in `terraphim_automata/src/autocomplete.rs`
- Add the same four fields with identical serde attributes
- Add identical builder and getter methods

**Deployable state**: Yes — compiles, structurally aligned with NormalizedTerm

### Step 4: Add tests for `AutocompleteMetadata`
- Test serialisation and deserialisation round-trip
- Test builder methods

**Deployable state**: Yes — tests only

### Step 5: Run full workspace build and test
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy -- -D warnings`

**Deployable state**: Yes — verified green

---

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| AC1: Constructors produce unset new fields | Unit | `crates/terraphim_types/src/lib.rs` (inline tests or `tests/` dir) |
| AC2: JSON backward compat (missing fields) | Unit | Same |
| AC3: JSON serialisation with all fields | Unit | Same |
| AC4: Builder/getter methods | Unit | Same |
| AC5: AutocompleteMetadata alignment | Unit | `crates/terraphim_automata/src/autocomplete.rs` |
| AC6: No new warnings | Build | CI (`cargo clippy`) |
| AC7: All tests pass | Integration | CI (`cargo test --workspace`) |

---

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|--------------|
| Forgetting to update a construction site causes compiler warning | Run `cargo build --workspace` which will warn on missing fields; all test crates updated in same PR | Low |
| JSON output change breaks external consumers | New fields are optional; consumers that ignore unknown fields are unaffected | Low |
| `AutocompleteMetadata` and `NormalizedTerm` drift out of sync in future | Document that they must be updated together; consider a shared test | Low |

---

## 8. Open Questions / Decisions for Human Review

1. **Field defaults for `priority`**: `Option<u8>` means `None` means no priority set. Should `priority: Option<u8>` be replaced with `priority: u8` with `#[serde(default = "default_priority")]` where `default_priority()` returns `50` as a sensible neutral priority? (The current plan uses `Option<u8>` — confirm this is preferred or choose the default-value approach.)

2. **Should `trigger` be a `Vec<String>` instead of `Option<String>`** to support multiple trigger patterns per term? (Current plan uses `Option<String>` — single trigger per term. Multi-trigger would require `Option<Vec<String>>`.)

3. **`pinned: bool` with `#[serde(default)]`** — should pinned terms default to `false` (current plan) or should the serde default be a specific behaviour (e.g., pinned = false is never serialised)?

**Note**: All three questions above are clarifications on default values and representation — the four-field structure is fixed per the approved research. The default-value approach for `priority` and the single-string `trigger` are recommended for simplicity.

---

## Appendix: Target Struct Shapes

### `NormalizedTerm` (after)
```rust
pub struct NormalizedTerm {
    pub id: u64,
    pub value: NormalizedTermValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}
```

### `AutocompleteMetadata` (after)
```rust
pub struct AutocompleteMetadata {
    pub id: u64,
    pub normalized_term: NormalizedTermValue,
    pub original_term: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}
```
