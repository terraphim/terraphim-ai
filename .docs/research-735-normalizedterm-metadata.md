# Research Document: Add Action Metadata to NormalizedTerm

**Issue**: #735 — `feat(types): add action metadata to NormalizedTerm beyond url field`
**Author**: Agent
**Date**: 2026-04-22
**Phase**: 1 — Research

---

## 1. Problem Restatement and Scope

### Problem in My Own Words

The `NormalizedTerm` type currently provides only four fields: `id`, `value` (the normalised text), `display_value` (original case for output), and `url` (a link reference). For many use cases — routing CLI actions, autocomplete suggestions, KG entries — the single `url` string is insufficient. The system needs to attach richer metadata such as CLI action templates with variable substitution (`{{ model }}`, `{{ prompt }}`), priority levels, trigger patterns, and pinning status directly to a term, rather than relying on separate lookup tables or storing everything inside a URL string.

### IN Scope
- Extending `NormalizedTerm` with structured action metadata
- Ensuring backward compatibility with existing `url` field usage
- Updating all call sites that construct `NormalizedTerm`
- Updating tests and serialization/deserialization

### OUT of Scope
- Changes to `MarkdownDirectives` or `RouteDirective` (these are separate structures)
- Changes to the `Thesaurus` or `RoutingRule` types
- Changes to the knowledge graph storage format (those already have rich metadata)
- Migration of existing data

---

## 2. User & Business Outcomes

### Visible Changes
- Autocomplete responses can include the full CLI action template (e.g., `opencode -m {{ model }} -p "{{ prompt }}"`) alongside the matched term, enabling one-step invocation
- Routing decisions made purely from a `NormalizedTerm` can extract the action without a separate directive lookup
- Terms used in command registries can carry their action template inline, simplifying dispatch

### Business Outcomes
- Reduced latency in routing hot paths (fewer lookups)
- Cleaner separation of concerns: the term carries its own intent
- Enables richer autocomplete UX (showing action previews before execution)

---

## 3. System Elements and Dependencies

| Component | Role | Location |
|-----------|------|----------|
| `NormalizedTerm` struct | Core term type being extended | `crates/terraphim_types/src/lib.rs:297-355` |
| `NormalizedTermValue` | Newtype wrapper for the normalised string value | `crates/terraphim_types/src/lib.rs:250-288` |
| `AutocompleteMetadata` | Existing autocomplete struct that mirrors NormalizedTerm | `crates/terraphim_automata/src/autocomplete.rs:22-28` |
| `MarkdownDirectives` | Existing rich metadata container (`route`, `routes`, `priority`, `trigger`, `pinned`) | `crates/terraphim_types/src/lib.rs:417-439` |
| `RouteDirective` | Has `provider`, `model`, and `action: Option<String>` with CLI template placeholders | `crates/terraphim_types/src/lib.rs:408-415` |
| `KGTermDefinition` | KG entry type with `metadata: AHashMap<String, String>` for extensibility | `crates/terraphim_types/src/lib.rs:1455-1480` |
| `terraphim_automata::autocomplete` | Uses `AutocompleteMetadata` for search results | `crates/terraphim_automata/src/autocomplete.rs:490` |
| `terraphim_agent::commands::registry` | Constructs `NormalizedTerm` with `command:{name}` URLs | `crates/terraphim_agent/src/commands/registry.rs:552-586` |
| `terraphim_orchestrator::kg_router` | Builds `NormalizedTerm` without url for routing lookups | `crates/terraphim_orchestrator/src/kg_router.rs:134-139` |
| `terraphim_service::lib` | Sets URLs in `kg:{concept}` format | `terraphim_service/src/lib.rs:779` |
| `terraphim_sessions::enrichment::enricher` | Constructs `NormalizedTerm` for session enrichment | `crates/terraphim-session-analyzer/src/enrichment/enricher.rs:315` |
| All test files | Construct `NormalizedTerm` directly | Multiple locations across `*/tests/*.rs` and `*/benches/*.rs` |

### Key Dependency Observation
`NormalizedTerm` is the lowest-common-denominator term type used across **all** crates (types, automata, agent, orchestrator, service, sessions). Adding fields is a breaking change for every construction site, but the type already uses `#[serde(default, skip_serializing_if = "Option::is_none")]` on optional fields, which provides backward compatibility for JSON deserialization when new optional fields are added.

---

## 4. Constraints and Their Implications

### Backward Compatibility (Critical)
- Existing JSON data will deserialise correctly if new fields are `Option<T>` with defaults
- Serialisation output will change (new fields appear) — consumers must handle absent optional fields gracefully
- **Implication**: All new fields must be `Option<T>` with `#[serde(default)]`

### Cross-Crate Impact (High)
- `NormalizedTerm` is used in 10+ crates; any change propagates everywhere
- Serialisation format is shared with potential external consumers (CLI, LSP, IPC)
- **Implication**: Change must be minimal and additive; consider a separate `TermMetadata` attachant if fields would otherwise proliferate

### No Data Migration (Constraint)
- No migration scripts or dual-format support are in scope
- Existing stored thesauruses should not break
- **Implication**: The `url` field remains as-is; new fields are purely additive

### Action Template Format (Known Pattern)
- `RouteDirective.action` already uses `{{ model }}` and `{{ prompt }}` placeholders
- Actions are CLI command strings, not structured objects
- **Implication**: Store action as `Option<String>`, mirroring `RouteDirective.action`

---

## 5. Risks, Unknowns, and Assumptions

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Forgetting to update a construction site causes a partial-update compiler warning (not error) | Low — Rust warns on missing struct fields | Audit all call sites; use `cargo build --workspace` to catch |
| JSON schema change breaks external consumers | Medium — external systems reading NormalizedTerm JSON | Document new schema; new fields are optional |
| New fields bloat autocomplete payloads | Low — all new fields are `Option<String>` or `Option<u8>`, small |

### Unknowns

| Unknown | Why It Matters |
|---------|----------------|
| What specific metadata fields are actually needed beyond `action`? | The issue description is vague; only `action` is confirmed from context |
| Should `AutocompleteMetadata` also be extended, or does it mirror `NormalizedTerm` separately? | If they diverge, autocomplete consumers need separate changes |
| Are there persisted `NormalizedTerm` JSON files (thesauruses) that would need updating? | Would determine if a migration path is needed despite "no migration" constraint |

### Assumptions

- **ASSUMPTION 1**: The primary motivation is to support inline action templates on terms used in routing/autocomplete, not a general-purpose extensible metadata map (which already exists in `KGTermDefinition.metadata`)
- **ASSUMPTION 2**: The `AutocompleteMetadata` struct in `terraphim_automata` is intended to stay in sync with `NormalizedTerm` and should be updated in lockstep
- **ASSUMPTION 3**: `NormalizedTerm` should gain fields that mirror the subset of `MarkdownDirectives` most relevant to a single term: `action` (template string), `priority` (u8), and optionally `trigger` (string pattern)

---

## 6. Complexity vs. Simplicity Opportunities

### Complexity Sources

1. **Proliferation of optional fields**: Adding `action`, `priority`, `trigger`, and `pinned` would create 4 new `Option` fields on a type already criticised for having only a URL. This is at odds with the "few fields, clear semantics" design of `NormalizedTerm`.

2. **Separate `AutocompleteMetadata` type**: `AutocompleteMetadata` in `terraphim_automata` is nearly identical to `NormalizedTerm` but without `display_value`. These two types may need to converge or at least both receive the new fields.

3. **Existing workaround pattern**: The codebase already solves this problem by storing directives separately from terms (in `MarkdownDirectives` on the routing rule). Adding action metadata directly to `NormalizedTerm` would change this established pattern.

### Simplicity Opportunities

1. **Introduce a `TermMetadata` struct** that wraps `action`, `priority`, `trigger`, and `pinned`, and add a single `metadata: Option<TermMetadata>` field to `NormalizedTerm`. This keeps the struct flat but encapsulated.

2. **Mirror the `AutocompleteMetadata` update** in the same PR to keep the two types consistent.

3. **Reuse existing serde patterns**: All new fields use `#[serde(default, skip_serializing_if = "Option::is_none")]` — the same pattern already used for `display_value` and `url`.

---

## 7. Questions for Human Reviewer

1. **What specific metadata fields are needed?** The issue says "action metadata" — is this only an `action: Option<String>` CLI template field, or also `priority`, `trigger`, and `pinned` like `MarkdownDirectives`?
2. **Should `AutocompleteMetadata`** (in `terraphim_automata`) be updated in the same PR to stay in sync with `NormalizedTerm`, or is it a separate concern?
3. **Is a single `metadata: Option<TermMetadata>` wrapper struct preferred**, or should each field be added directly to `NormalizedTerm` as top-level optional fields?
4. **Should the `url` field be deprecated** in favour of the new action metadata, or is `url` still needed for terms that are simple links without actions?
5. **Are there existing persisted JSON files** (thesauruses, autocomplete indexes) that contain `NormalizedTerm` arrays which would be affected by a schema change?
6. **Should `NormalizedTerm` also gain a `routes: Vec<RouteDirective>` field** (like `MarkdownDirectives`) for multi-route support, or is a single `action` string sufficient?
7. **What is the backward-compatibility story?** New fields are optional, but should existing serialised data be re-generated or is read-only backward compatibility sufficient?
8. **Should the `display_value` field be folded into the metadata struct**, or does it serve a distinct purpose that warrants keeping it top-level?
9. **Is there a reason `KGTermDefinition.metadata` (extensible `AH ahashmap`) was not used** — is the intent to have structured fields rather than a generic map?
10. **Are there any performance concerns** with adding optional fields to `NormalizedTerm` given it appears in hot autocomplete and routing paths?

---

## Appendix: Existing Relevant Types

### `NormalizedTerm` (current)
```rust
pub struct NormalizedTerm {
    pub id: u64,
    pub value: NormalizedTermValue,
    pub display_value: Option<String>,  // serde: nterm
    pub url: Option<String>,
}
```

### `AutocompleteMetadata` (current)
```rust
pub struct AutocompleteMetadata {
    pub id: u64,
    pub normalized_term: NormalizedTermValue,
    pub url: Option<String>,
    pub original_term: String,
}
```

### `MarkdownDirectives` (reference for richness)
```rust
pub struct MarkdownDirectives {
    pub doc_type: DocumentType,
    pub synonyms: Vec<String>,
    pub route: Option<RouteDirective>,      // provider + model + action
    pub routes: Vec<RouteDirective>,
    pub priority: Option<u8>,
    pub trigger: Option<String>,
    pub pinned: bool,
    pub heading: Option<String>,
}
```

### `RouteDirective` (action template reference)
```rust
pub struct RouteDirective {
    pub provider: String,
    pub model: String,
    pub action: Option<String>,  // CLI template with {{ model }}, {{ prompt }}
}
```

### `KGTermDefinition` (extensible metadata reference)
```rust
pub struct KGTermDefinition {
    pub term: String,
    pub normalized_term: NormalizedTermValue,
    pub id: u64,
    pub definition: Option<String>,
    pub synonyms: Vec<String>,
    pub related_terms: Vec<String>,
    pub usage_examples: Vec<String>,
    pub url: Option<String>,
    pub metadata: AHashMap<String, String>,  // <-- extensible
    pub relevance_score: Option<f64>,
}
```
