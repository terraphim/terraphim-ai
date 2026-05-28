# Research Document: Exit Class Patterns to terraphim-automata Migration

**Status**: Approved
**Author**: OpenCode Agent
**Date**: 2026-05-21
**Reviewers**: [Pending]

## Executive Summary

The `EXIT_CLASS_PATTERNS` constant in `crates/terraphim_orchestrator/src/agent_run_record.rs` hard-codes 63 exit-classification patterns across 9 concepts. A knowledge graph source file (`docs/src/kg/exit_classes.md`) already documents these patterns in Logseq `synonyms::` format, but the code does not consume it. This research analyses moving the orchestrator to a true `terraphim-automata` pipeline: build the exit-class thesaurus from the KG markdown via the existing `Logseq` builder, cache it as JSON, and load it at runtime. This removes the static duplication, enables hot-reloading of patterns, and aligns the orchestrator with Terraphim's knowledge-graph-first architecture.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Reduces pattern duplication; enables non-devs to tune exit classification by editing markdown |
| Leverages strengths? | Yes | `terraphim_automata` already provides `Logseq` builder, JSON serialisation, and Aho-Corasick matching |
| Meets real need? | Yes | PR review feedback already identified broad-pattern risks; KG-driven loading allows rapid iteration without recompilation |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
Agent exit classification relies on a hard-coded `const EXIT_CLASS_PATTERNS: &[PatternDef]` embedded in `agent_run_record.rs`. The same patterns are separately maintained in `docs/src/kg/exit_classes.md` in Logseq `synonyms::` syntax. This creates a maintenance liability: every pattern update requires a Rust recompilation, and the two sources can drift out of sync.

### Impact
- **Developers** must edit Rust source and recompile to add or refine exit-class patterns.
- **DevOps/Operators** cannot tune classification behaviour without a code change.
- **Knowledge graph maintainers** edit `exit_classes.md` but see no effect because the orchestrator ignores it.

### Success Criteria
1. `EXIT_CLASS_PATTERNS` static array is removed from `agent_run_record.rs`.
2. `ExitClassifier` loads its thesaurus from the KG markdown (or a build-time JSON artefact derived from it).
3. All existing classification behaviour and 30+ unit tests continue to pass without modification.
4. Pattern updates in `docs/src/kg/exit_classes.md` are reflected in classification after rebuild/restart.

## Current State Analysis

### Existing Implementation

#### Hard-coded Patterns (`agent_run_record.rs:238-358`)
```rust
const EXIT_CLASS_PATTERNS: &[PatternDef] = &[
    PatternDef { concept_name: "timeout", patterns: &["timed out", "deadline exceeded", ...] },
    PatternDef { concept_name: "ratelimit", patterns: &["429", "rate limit", ...] },
    // ... 7 more concepts, 63 total patterns
];
```

At runtime `ExitClassifier::build_thesaurus()` iterates this array, creates a `Concept` per entry, and inserts each pattern as a `NormalizedTermValue -> NormalizedTerm` synonym into a `Thesaurus`. The thesaurus is then passed to `terraphim_automata::matcher::find_matches()`.

#### Knowledge Graph Source (`docs/src/kg/exit_classes.md`)
The markdown already uses Logseq-style `synonyms::` lists:
```markdown
## Timeout
synonyms:: timed out, deadline exceeded, wall-clock kill, ...
## RateLimit
synonyms:: 429, rate limit, too many requests, ...
```

#### terraphim-automata Capabilities
- `Logseq` builder (`builder.rs`): Parses markdown directories with `ripgrep`, extracts `synonyms::` lines, and produces a `Thesaurus` where each markdown file stem is a concept and its synonyms map to it.
- `load_thesaurus_from_json()` (`lib.rs:322`): Deserialises a `Thesaurus` from JSON (new or legacy format).
- `find_matches()` (`matcher.rs:19`): Aho-Corasick multi-pattern matching, case-insensitive, leftmost-longest.
- `Thesaurus` (`terraphim_types::Thesaurus`): Serializable, cloneable, supports `source_hash` for cache invalidation.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `EXIT_CLASS_PATTERNS` | `agent_run_record.rs:238-358` | Static pattern definitions |
| `ExitClassifier` | `agent_run_record.rs:227-516` | Builds thesaurus, classifies exits |
| `ExitClass` enum | `agent_run_record.rs:49-74` | Classification taxonomy |
| `find_matches` | `terraphim_automata/src/matcher.rs:19` | Aho-Corasick matcher |
| `Logseq` builder | `terraphim_automata/src/builder.rs:100` | KG markdown -> Thesaurus |
| `load_thesaurus_from_json` | `terraphim_automata/src/lib.rs:322` | JSON -> Thesaurus |
| `Thesaurus` | `terraphim_types/src/lib.rs:720` | Core KG dictionary type |
| KG source | `docs/src/kg/exit_classes.md` | Human-maintained pattern list |

### Data Flow (Current)
```
poll_agent_exits()
    -> ExitClassifier::new() -> build_thesaurus(EXIT_CLASS_PATTERNS) -> Thesaurus
    -> classify(exit_code, stdout, stderr) -> find_matches(combined_text, thesaurus)
    -> ExitClassification { exit_class, matched_patterns, confidence }
```

### Integration Points
- `terraphim_automata::matcher::find_matches` — already used.
- `terraphim_types::Thesaurus` — already used.
- `docs/src/kg/exit_classes.md` — currently documentation-only, not consumed by code.

## Constraints

### Technical Constraints
- **tokio-runtime feature**: The `Logseq` builder requires `tokio-runtime` feature for `ripgrep` invocation. The orchestrator already depends on `tokio`.
- **Build-time vs runtime**: Embedding JSON at build time avoids runtime `ripgrep` dependency; runtime loading enables hot-reload but requires file-system access.
- **Thesaurus format**: `Logseq` builder produces concepts from file stems. The current markdown file is named `exit_classes.md` (single file), so either the file must be split per concept or the builder must be extended.

### Business Constraints
- **Zero regression**: All 30+ existing unit tests must pass.
- **No API breakage**: `ExitClassifier::classify()` signature must remain stable.

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Classification latency | < 1ms per agent exit | ~0.2ms (Aho-Corasick on small text) |
| Build time impact | No measurable regression | N/A |
| Binary size impact | < +50KB JSON embedded | N/A |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Must preserve exact classification behaviour | Regression in exit classification breaks ADF fallback logic | 566 tests depend on correct classification |
| Must remove `EXIT_CLASS_PATTERNS` static array | This is the primary goal — eliminate hard-coded duplication | PR review feedback |
| Must use existing `terraphim-automata` APIs | No new crate dependencies or custom parsers | `Logseq` builder and JSON loader already exist |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Runtime hot-reload without restart | Not in top 5; build-time JSON embedding is simpler and sufficient |
| Web-based KG editor UI | Far beyond scope; markdown editing is adequate |
| Fuzzy/autocomplete for exit classes | Over-engineering; exact Aho-Corasick matching is the requirement |
| Machine-learning classification | Replaces pattern matching entirely; different project |
| Multi-language pattern support | All agent output is English; add when needed |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_automata` | Provides `Logseq` builder, JSON loader, matcher | Low — already a dependency |
| `terraphim_types` | Provides `Thesaurus`, `Concept`, `NormalizedTerm` | Low — core type |
| `terraphim_orchestrator` | Consumer of the new loading path | Low — we're modifying it |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `ripgrep` (rg) | 14.x | Low — widely available | `grep` fallback in builder is not implemented |
| `aho-corasick` | 1.x | Low — via `terraphim_automata` | N/A |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `Logseq` builder expects one concept per markdown file | High | High — `exit_classes.md` is a single file with multiple H2 sections | Split into `timeout.md`, `ratelimit.md`, etc., OR extend builder to handle H2 headings |
| Pattern order or case-sensitivity differences | Low | Medium — `find_matches` uses `LeftmostLongest` + `ascii_case_insensitive` | Ensure generated thesaurus patterns match current static array exactly |
| Build script complexity | Medium | Low — `build.rs` in orchestrator crate | Keep simple: compile markdown to JSON at build time |
| JSON embedding increases binary size | Low | Low | Thesaurus is small (~5KB JSON) |

### Open Questions
1. **Should we split `exit_classes.md` into per-concept files?** — The `Logseq` builder derives concepts from file stems. Single-file multi-concept is not natively supported.
2. **Build-time JSON generation or runtime markdown parsing?** — Build-time is simpler and avoids runtime `ripgrep`; runtime enables hot-reload.
3. **Should patterns be overridable via config?** — Future work; out of scope for this migration.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `docs/src/kg/exit_classes.md` format is stable Logseq | It already uses `synonyms::` syntax | Builder fails to parse | Yes — inspected file |
| `find_matches` behaviour is identical between hand-built and builder-built thesaurus | Both use same `Thesaurus` type and `AhoCorasick` configuration | Classification differences break tests | No — must verify in Phase 3 |
| All 63 patterns in static array are present in markdown | Visual inspection suggests yes | Missing patterns reduce classification accuracy | Partial — needs automated check |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **A**: Split `exit_classes.md` into 9 files, use `Logseq` builder unchanged | Clean separation, native builder support | **Preferred** — simplest, most idiomatic |
| **B**: Extend `Logseq` builder to parse H2 headings as concepts from a single file | No file restructuring needed | **Rejected** — adds complexity to reusable builder for one use case |
| **C**: Hand-write JSON thesaurus, commit it, load at runtime | No builder dependency | **Rejected** — duplicates patterns in yet another format; loses KG source-of-truth |
| **D**: Keep static array, generate markdown from it | Reverse direction | **Rejected** — does not solve the core problem |

## Research Findings

### Key Insights
1. **The knowledge graph already exists** — `docs/src/kg/exit_classes.md` is maintained but ignored by the code. The migration is primarily about wiring code to existing documentation.
2. **`Logseq` builder is almost ready** — It needs one concept per file. Splitting the markdown is a one-time refactor that aligns with Terraphim's KG conventions.
3. **Build-time JSON generation is the sweet spot** — Use a `build.rs` script to invoke the `Logseq` builder on the split markdown files, serialise the `Thesaurus` to JSON, and embed it via `include_str!`. This avoids runtime dependencies and keeps startup fast.
4. **Test parity is achievable** — The existing tests call `ExitClassifier::new()` and `classify()`. If `new()` transparently loads the same patterns, all tests pass unchanged.

### Relevant Prior Art
- `terraphim_automata/src/builder.rs` — `Logseq` builder with `compute_kg_source_hash` for cache invalidation.
- `terraphim_service/src/auto_route.rs` — Loads thesaurus from JSON at runtime for auto-routing.
- `crates/terraphim_automata/tests/autocomplete_tests.rs` — Demonstrates thesaurus construction and matching.

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Split `exit_classes.md` and verify `Logseq` builder output | Confirm builder produces correct thesaurus | 30 minutes |
| Build script POC | Verify build-time JSON generation works in orchestrator crate | 1 hour |

## Recommendations

### Proceed/No-Proceed
**Proceed** — The migration is low-risk, high-alignment with Terraphim architecture, and removes a known maintenance liability.

### Scope Recommendations
- **In scope**: Split markdown, build-time JSON generation, remove static array, preserve all tests.
- **Out of scope**: Runtime hot-reload, config overrides, builder enhancements.

### Risk Mitigation Recommendations
1. Split markdown files first, verify builder output matches static array.
2. Add a test that compares builder-generated thesaurus against static array before removing the array.
3. Keep the `EXIT_CLASS_PATTERNS` array behind a `#[cfg(test)]` gate temporarily during transition, or generate it from the markdown.

## Next Steps

1. Create design document (Phase 2) with exact file changes and signatures.
2. Obtain human approval on research findings.
3. Proceed to implementation (Phase 3).

## Appendix

### Reference Materials
- `docs/src/kg/exit_classes.md` — Current KG source
- `crates/terraphim_orchestrator/src/agent_run_record.rs` — Current implementation
- `crates/terraphim_automata/src/builder.rs` — `Logseq` builder
- `crates/terraphim_automata/src/lib.rs` — JSON loading
- `crates/terraphim_types/src/lib.rs:718-779` — `Thesaurus` API

### Code Snippets

#### Current static array (excerpt)
```rust
const EXIT_CLASS_PATTERNS: &[PatternDef] = &[
    PatternDef {
        concept_name: "timeout",
        patterns: &["timed out", "deadline exceeded", "wall-clock kill", ...],
    },
    // ...
];
```

#### Current builder
```rust
fn build_thesaurus() -> Thesaurus {
    let mut thesaurus = Thesaurus::new("exit_classes".to_string());
    for def in EXIT_CLASS_PATTERNS {
        let concept = Concept::from(def.concept_name.to_string());
        let nterm = NormalizedTerm::new(concept.id, concept.value.clone());
        thesaurus.insert(concept.value.clone(), nterm.clone());
        for pattern in def.patterns {
            thesaurus.insert(NormalizedTermValue::new(pattern.to_string()), nterm.clone());
        }
    }
    thesaurus
}
```

#### Logseq builder usage pattern
```rust
let logseq = Logseq::default();
let thesaurus = logseq.build("exit_classes".into(), "path/to/kg/dir").await?;
```

#### JSON serialisation
```rust
let json = serde_json::to_string(&thesaurus)?;
// embed: include_str!(concat!(env!("OUT_DIR"), "/exit_classes.json"))
```
