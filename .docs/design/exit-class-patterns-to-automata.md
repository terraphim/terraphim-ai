# Implementation Plan: Exit Class Patterns to terraphim-automata Migration

**Status**: Draft
**Research Doc**: [`.docs/research/exit-class-patterns-to-automata.md`](./research/exit-class-patterns-to-automata.md)
**Author**: OpenCode Agent
**Date**: 2026-05-21
**Estimated Effort**: 4-6 hours

## Overview

### Summary
Remove the hard-coded `EXIT_CLASS_PATTERNS` static array from `agent_run_record.rs` and replace it with a build-time generated JSON thesaurus derived from `docs/src/kg/exit_classes.md` using the existing `terraphim_automata::Logseq` builder. The `ExitClassifier` will load its thesaurus from the embedded JSON, making the knowledge graph the single source of truth for exit classification patterns.

### Approach
1. **Split** the single `exit_classes.md` into one file per concept (the `Logseq` builder derives concepts from file stems).
2. **Build script**: Add `build.rs` to `terraphim_orchestrator` that invokes `Logseq` builder at compile time to produce `exit_classes.json` in `OUT_DIR`.
3. **Embed**: Use `include_str!` to embed the JSON into the binary.
4. **Load**: `ExitClassifier::new()` deserialises the embedded JSON into a `Thesaurus` via `terraphim_automata::load_thesaurus_from_json`.
5. **Remove**: Delete the `EXIT_CLASS_PATTERNS` static array and `PatternDef` struct.
6. **Verify**: All existing tests pass without modification.

### Scope

**In Scope:**
- Split `docs/src/kg/exit_classes.md` into 9 per-concept markdown files
- Add `build.rs` to `terraphim_orchestrator` for build-time thesaurus generation
- Modify `ExitClassifier` to load from embedded JSON
- Remove `EXIT_CLASS_PATTERNS` and `PatternDef`
- Preserve all existing tests
- Add parity test comparing old vs new thesaurus (temporary, removed before merge)

**Out of Scope:**
- Runtime hot-reload of patterns
- Config-file overrides
- Builder enhancements for multi-concept single files
- Machine-learning classification

**Avoid At All Cost** (from 5/25 analysis):
- Extending `Logseq` builder with single-file multi-concept parsing (one-off complexity in reusable code)
- Hand-maintaining a JSON thesaurus alongside markdown (duplicates source of truth)
- Runtime `ripgrep` invocation on every `ExitClassifier::new()` (startup latency, external dependency)

## Architecture

### Component Diagram
```
+----------------------------------+
| docs/src/kg/exit_classes/        |
|   timeout.md                     |
|   ratelimit.md                   |
|   compilationerror.md            |
|   ... (9 files)                  |
+-------------+--------------------+
              |
              v
+----------------------------------+
| build.rs (terraphim_orchestrator)|
|   Logseq::default().build(...)   |
|   -> Thesaurus                   |
|   serde_json::to_string()        |
|   -> $OUT_DIR/exit_classes.json  |
+-------------+--------------------+
              |
              v
+----------------------------------+
| agent_run_record.rs              |
|   include_str!(concat!(...))     |
|   load_thesaurus_from_json()     |
|   -> ExitClassifier.thesaurus    |
+----------------------------------+
```

### Data Flow
```
Agent output (stdout + stderr)
    |
    v
ExitClassifier::classify()
    |-- embedded JSON -> Thesaurus (deserialised once at new())
    |-- find_matches(combined_text, thesaurus)
    |-- count matches per concept
    |-- pick dominant concept -> ExitClass
    v
ExitClassification { exit_class, matched_patterns, confidence }
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| **Split markdown into 9 files** | `Logseq` builder derives concepts from file stems; one concept per file is idiomatic | Extending builder for H2 parsing (one-off complexity) |
| **Build-time JSON generation** | Avoids runtime `ripgrep` dependency; startup is instant; binary is self-contained | Runtime parsing (adds latency, external dependency) |
| **Embed JSON via `include_str!`** | Zero runtime file I/O; works in containers without KG source present | `std::fs::read` at runtime (fragile, needs file present) |
| **Keep `ExitClassifier` API unchanged** | Zero breaking changes for callers; all existing tests compile | Refactoring classify signature (unnecessary churn) |
| **Use `load_thesaurus_from_json` (sync)** | Build script and embedded JSON are synchronous contexts | Async loader (not needed, adds complexity) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Runtime `Logseq` builder in `new()` | Requires `tokio` runtime and `ripgrep` at agent startup | Startup failure in minimal containers |
| Hand-maintained JSON committed to repo | Duplicates KG markdown; guaranteed drift | Maintenance burden, stale patterns |
| `#[cfg(test)]` static array fallback | Defeats purpose — tests would test different code than production | False confidence, hidden divergence |
| Concept ID remapping | Current IDs are auto-generated from concept names; preserving exact IDs is unnecessary | Over-engineering; matcher uses concept name for grouping |

### Simplicity Check

> "What if this could be easy?"

The simplest design is: split markdown, build script generates JSON, embed JSON, load JSON. No new crates, no new traits, no runtime dependencies. The only "new" code is a ~20-line `build.rs` and a 3-line change in `ExitClassifier::new()`.

**Senior Engineer Test**: A senior engineer would recognise this as the obvious path. No abstractions, no frameworks, just wiring existing pieces together.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur (build script failure fails compilation, which is correct)
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `docs/src/kg/exit_classes/timeout.md` | Timeout concept with synonyms |
| `docs/src/kg/exit_classes/ratelimit.md` | RateLimit concept with synonyms |
| `docs/src/kg/exit_classes/compilationerror.md` | CompilationError concept with synonyms |
| `docs/src/kg/exit_classes/testfailure.md` | TestFailure concept with synonyms |
| `docs/src/kg/exit_classes/modelerror.md` | ModelError concept with synonyms |
| `docs/src/kg/exit_classes/networkerror.md` | NetworkError concept with synonyms |
| `docs/src/kg/exit_classes/resourceexhaustion.md` | ResourceExhaustion concept with synonyms |
| `docs/src/kg/exit_classes/permissiondenied.md` | PermissionDenied concept with synonyms |
| `docs/src/kg/exit_classes/crash.md` | Crash concept with synonyms |
| `crates/terraphim_orchestrator/build.rs` | Build script: Logseq builder -> JSON |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/agent_run_record.rs` | Remove `EXIT_CLASS_PATTERNS` and `PatternDef`; rewrite `build_thesaurus()` to load from embedded JSON |
| `crates/terraphim_orchestrator/Cargo.toml` | Add `build-dependencies` for `terraphim_automata` and `tokio` (for build script) |
| `docs/src/kg/exit_classes.md` | Convert to index/overview file or remove after split |

### Deleted Files

| File | Reason |
|------|--------|
| `docs/src/kg/exit_classes.md` | Replaced by per-concept files in `exit_classes/` directory |

## API Design

### Public Types (No Changes)

```rust
/// No changes to public API
pub struct ExitClassifier { ... }
pub enum ExitClass { ... }
pub struct ExitClassification { ... }
pub struct AgentRunRecord { ... }
```

### Internal Functions

```rust
impl ExitClassifier {
    /// Create a new ExitClassifier with the built-in exit class thesaurus.
    /// Loads thesaurus from build-time generated JSON embedded in the binary.
    pub fn new() -> Self {
        Self {
            thesaurus: Self::load_thesaurus(),
        }
    }

    /// Load thesaurus from embedded JSON.
    fn load_thesaurus() -> Thesaurus {
        const JSON: &str = include_str!(concat!(
            env!("OUT_DIR"),
            "/exit_classes.json"
        ));
        terraphim_automata::load_thesaurus_from_json(JSON)
            .expect("build-time generated exit_classes.json must be valid")
    }
}
```

### Removed Types

```rust
// DELETED
struct PatternDef { ... }
const EXIT_CLASS_PATTERNS: &[PatternDef] = &[...];
```

## Test Strategy

### Unit Tests (Existing — Must Pass Unchanged)

| Test | Location | Purpose |
|------|----------|---------|
| `classify_success_with_output` | `agent_run_record.rs` | Happy path |
| `classify_empty_success` | `agent_run_record.rs` | Empty output detection |
| `classify_timeout` | `agent_run_record.rs` | Timeout classification |
| `classify_rate_limit` | `agent_run_record.rs` | Rate limit classification |
| `classify_compilation_error` | `agent_run_record.rs` | Compilation error |
| `classify_test_failure` | `agent_run_record.rs` | Test failure |
| `classify_model_error` | `agent_run_record.rs` | Model error |
| `classify_network_error` | `agent_run_record.rs` | Network error |
| `classify_resource_exhaustion` | `agent_run_record.rs` | OOM/disk full |
| `classify_permission_denied` | `agent_run_record.rs` | Permission denied |
| `classify_crash` | `agent_run_record.rs` | Crash detection |
| `classify_unknown_exit` | `agent_run_record.rs` | Unknown fallback |
| `classify_mixed_patterns_picks_dominant` | `agent_run_record.rs` | Dominant class wins |
| `exit_code_zero_with_*` | `agent_run_record.rs` | False-positive prevention |
| `classify_quota_*` | `agent_run_record.rs` | Quota/rate-limit variants |

### New Test (Parity Verification)

```rust
#[test]
fn embedded_thesaurus_matches_legacy_patterns() {
    // Load the new thesaurus from embedded JSON
    let new_classifier = ExitClassifier::new();
    
    // Build old thesaurus from static array (before deletion, temporarily)
    let old_thesaurus = build_thesaurus_legacy();
    
    // Verify every legacy pattern exists in the new thesaurus
    for def in EXIT_CLASS_PATTERNS {
        let concept = Concept::from(def.concept_name.to_string());
        for pattern in def.patterns {
            let key = NormalizedTermValue::new(pattern.to_string());
            let matched = new_classifier.thesaurus.get(&key);
            assert!(
                matched.is_some(),
                "Pattern '{}' for concept '{}' missing from embedded thesaurus",
                pattern, def.concept_name
            );
            assert_eq!(
                matched.unwrap().value, concept.value,
                "Pattern '{}' maps to wrong concept",
                pattern
            );
        }
    }
}
```

**Note**: This test is temporary during transition. It verifies parity before `EXIT_CLASS_PATTERNS` is deleted.

### Integration Tests
- Run full `cargo test -p terraphim_orchestrator` suite — all 566 tests must pass.

## Implementation Steps

### Step 1: Split Knowledge Graph Markdown
**Files**: `docs/src/kg/exit_classes/*.md` (9 new files), `docs/src/kg/exit_classes.md` (deleted)
**Description**: Convert single-file KG into per-concept files for `Logseq` builder compatibility.
**Tests**: None (documentation refactor)
**Estimated**: 30 minutes

Example `timeout.md`:
```markdown
# Timeout

synonyms:: timed out, deadline exceeded, wall-clock kill, context deadline exceeded, operation timed out, execution expired
```

### Step 2: Build Script
**Files**: `crates/terraphim_orchestrator/build.rs`, `Cargo.toml`
**Description**: Add build-time thesaurus generation.
**Dependencies**: Step 1
**Estimated**: 1 hour

```rust
// build.rs
use std::path::Path;

fn main() {
    let kg_dir = Path::new("../../docs/src/kg/exit_classes");
    println!("cargo::rerun-if-changed={}", kg_dir.display());
    
    // Runtime tokio needed for Logseq builder
    let rt = tokio::runtime::Runtime::new().unwrap();
    let thesaurus = rt.block_on(async {
        let logseq = terraphim_automata::builder::Logseq::default();
        logseq.build("exit_classes".into(), kg_dir).await
    }).expect("failed to build exit classes thesaurus");
    
    let json = serde_json::to_string_pretty(&thesaurus)
        .expect("failed to serialise thesaurus");
    
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(
        Path::new(&out_dir).join("exit_classes.json"),
        json
    ).expect("failed to write exit_classes.json");
}
```

Cargo.toml additions:
```toml
[build-dependencies]
terraphim_automata = { path = "../terraphim_automata", features = ["tokio-runtime"] }
terraphim_types = { path = "../terraphim_types" }
tokio = { version = "1", features = ["rt-multi-thread"] }
serde_json = "1"
```

### Step 3: Modify ExitClassifier
**Files**: `crates/terraphim_orchestrator/src/agent_run_record.rs`
**Description**: Remove static array; load from embedded JSON.
**Dependencies**: Step 2
**Estimated**: 1 hour

Changes:
1. Delete `PatternDef` struct.
2. Delete `EXIT_CLASS_PATTERNS` constant.
3. Replace `build_thesaurus()` with `load_thesaurus()` using `include_str!`.
4. Keep `ExitClass::from_concept_name()` unchanged.

### Step 4: Parity Test
**Files**: `crates/terraphim_orchestrator/src/agent_run_record.rs` (temporary test)
**Description**: Add test verifying embedded thesaurus contains all legacy patterns.
**Dependencies**: Step 3
**Estimated**: 30 minutes

### Step 5: Run Full Test Suite
**Command**: `cargo test -p terraphim_orchestrator`
**Expected**: All tests pass.
**Estimated**: 10 minutes

### Step 6: Remove Temporary Code
**Files**: `agent_run_record.rs`
**Description**: Delete parity test and any `#[cfg(test)]` legacy fallback.
**Estimated**: 15 minutes

### Step 7: Documentation Update
**Files**: `.docs/summary-crates-terraphim_orchestrator-src-agent_run_record.rs.md`
**Description**: Update file summary to reflect KG-driven loading.
**Estimated**: 15 minutes

## Rollback Plan

If issues discovered:
1. Revert `agent_run_record.rs` to restore `EXIT_CLASS_PATTERNS` and `build_thesaurus()`.
2. Delete `build.rs` and revert `Cargo.toml`.
3. Delete `docs/src/kg/exit_classes/` directory; restore `docs/src/kg/exit_classes.md`.
4. All changes are contained to one crate and docs directory — rollback is a single git revert.

## Dependencies

### New Build Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| `terraphim_automata` | workspace | Logseq builder for build-time thesaurus generation |
| `tokio` | 1.x | Runtime for async Logseq builder in build script |
| `serde_json` | 1.x | Thesaurus serialisation in build script |

### No New Runtime Dependencies
All required crates (`terraphim_automata`, `terraphim_types`) are already runtime dependencies of `terraphim_orchestrator`.

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| Build time increase | < +2s | `cargo build -p terraphim_orchestrator` |
| Binary size increase | < +10KB | Embedded JSON vs static strings |
| Runtime classification latency | No regression | Existing benchmarks |
| Startup (ExitClassifier::new) | < +50us | JSON parse vs static array iteration |

### Benchmarks
No new benchmarks needed; existing test suite covers classification correctness. If desired, add a micro-benchmark for `ExitClassifier::new()` to ensure JSON parse is negligible.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify Logseq builder output concept names match `ExitClass::from_concept_name` expectations | Pending | Implementer (Step 4 parity test) |
| Decide whether to preserve `docs/src/kg/exit_classes.md` as an index | Pending | Implementer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

---

## Post-Approval Next Steps

After human approval:
1. Execute Step 1 (split markdown)
2. Execute Step 2 (build script)
3. Execute Step 3 (ExitClassifier refactor)
4. Execute Step 4 (parity test)
5. Execute Step 5 (full test suite)
6. Execute Step 6 (cleanup)
7. Execute Step 7 (docs)
8. Create PR referencing this plan
