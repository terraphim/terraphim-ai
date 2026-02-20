# Implementation Plan: Dynamic Ontology Feature Gates

**Status**: Draft
**Research Doc**: `docs/research/dynamic_ontology_feature_gates.md`
**Author**: Claude
**Date**: 2026-02-20
**Estimated Effort**: 4 hours

## Overview

### Summary
Refactor Dynamic Ontology types to be domain-agnostic while feature-gating medical/oncology-specific components. Makes EntityType configurable, RelationshipType configurable, and HGNC normalizer optional.

### Approach
Use Rust feature gates to compile oncology-specific code only when needed:
- Default feature `ontology` includes generic types
- Feature `medical` adds oncology-specific EntityType/RelationshipType variants
- Feature `hgnc` includes gene normalization (implies `medical`)

### Scope
**In Scope:**
- Add feature gates to `terraphim_types/Cargo.toml`
- Make EntityType configurable via string or feature-gated enum
- Make RelationshipType configurable via string or feature-gated enum
- Feature-gate HgncNormalizer
- Keep generic types (GroundingMetadata, CoverageSignal, SchemaSignal) unchanged

**Out of Scope:**
- Runtime domain configuration
- Vector embeddings (not needed - using fuzzy matching)
- Full ontology import pipelines
- Multi-domain simultaneous serving

**Avoid At All Cost:**
- Runtime type registration (stick to compile-time features)
- Generic trait-based abstraction for simple use cases
- Breaking existing API without migration path

## Architecture

### Feature Gate Structure
```
terraphim_types
├── Default features: ontology
├── Feature: medical (implies ontology)
│   ├── EntityType (oncology variants)
│   └── RelationshipType (oncology variants)
└── Feature: hgnc (implies medical)
    └── HgncNormalizer
```

### Component Diagram
```
┌─────────────────────────────────────────────────────────┐
│                    terraphim_types                       │
├─────────────────────────────────────────────────────────┤
│  Generic (always available)                             │
│  ├── GroundingMetadata                                   │
│  ├── CoverageSignal                                     │
│  ├── SchemaSignal                                       │
│  ├── ExtractedEntity                                    │
│  └── NormalizationMethod                               │
├─────────────────────────────────────────────────────────┤
│  Feature: ontology (default)                            │
│  └── ExtractedEntity with String entity_type            │
├─────────────────────────────────────────────────────────┤
│  Feature: medical (opt-in)                              │
│  ├── EntityType enum (CancerDiagnosis, Tumor, etc.)    │
│  └── RelationshipType enum (HasTumor, etc.)           │
├─────────────────────────────────────────────────────────┤
│  Feature: hgnc (opt-in)                                │
│  └── HgncNormalizer                                    │
└─────────────────────────────────────────────────────────┘
```

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| String-based entity_type default | Maximum flexibility | Enum-only was too restrictive |
| Feature-gated enum for medical | Type safety when needed | Runtime dispatch overhead |
| HGNC implies medical | Logical dependency | Separate features causing confusion |
| Keep generic types unchanged | Minimize regression risk | Rewriting working code |

### Eliminated Options (Essentialism)
| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Runtime domain registration | Compile-time is simpler, faster | Complexity not needed yet |
| Trait-based EntityType | Over-abstraction for simple use case | YAGNI |
| Multiple ontology catalogs | Too complex for v1 | Can add later |

### Simplicity Check

> "Minimum code that solves the problem. Nothing speculative."

**Answer**: The simplest approach is:
1. Add `entity_type: String` to ExtractedEntity (always available)
2. Add `#[cfg(feature = "medical")]` EntityType enum (opt-in)
3. Add `#[cfg(feature = "hgnc")]` HgncNormalizer (opt-in)

This is a minimal change that:
- Keeps existing oncology users working (default feature)
- Allows new domains without medical dependency
- Adds no runtime overhead

**Senior Engineer Test**: Yes, this is simple enough. No over-engineering.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_types/Cargo.toml` | Add feature gates |
| `crates/terraphim_types/src/lib.rs` | Add conditional EntityType, RelationshipType |
| `crates/terraphim_types/src/hgnc.rs` | Add `#[cfg(feature = "hgnc")]` |
| `crates/terraphim_types/examples/ontology_usage.rs` | Update for string-based types |

### No New Files Required

## API Design

### Public Types (Generic - Always Available)
```rust
/// Generic extracted entity - works for any domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity type as string for maximum flexibility
    pub entity_type: String,  // Changed from enum
    pub raw_value: String,
    pub normalized_value: Option<String>,
    pub grounding: Option<GroundingMetadata>,
}

/// Grounding metadata - unchanged, generic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingMetadata {
    pub normalized_uri: Option<String>,
    pub normalized_label: Option<String>,
    pub normalized_prov: Option<String>,
    pub normalized_score: Option<f32>,
    pub normalized_method: Option<NormalizationMethod>,
}
```

### Feature-Gated Types (Medical)
```rust
#[cfg(feature = "medical")]
pub mod medical {
    /// Oncology-specific entity types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum EntityType {
        CancerDiagnosis,
        Tumor,
        GenomicVariant,
        Biomarker,
        Drug,
        Treatment,
        SideEffect,
    }

    /// Oncology-specific relationship types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum RelationshipType {
        HasTumor,
        HasVariant,
        HasBiomarker,
        TreatedWith,
        Causes,
        HasDiagnosis,
    }
}
```

### Feature-Gated Types (HGNC)
```rust
#[cfg(feature = "hgnc")]
pub mod hgnc {
    pub struct HgncNormalizer { /* ... */ }
    pub struct HgncGene { /* ... */ }

    impl HgncNormalizer {
        pub fn new() -> Self;
        pub fn normalize(&self, symbol: &str) -> Option<GroundingMetadata>;
    }
}
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_string_entity_type` | `lib.rs` | Verify string-based entity_type works |
| `test_grounding_metadata_generic` | `lib.rs` | Verify generic grounding works |
| `test_coverage_signal_generic` | `lib.rs` | Verify coverage calculation unchanged |
| `#[cfg(feature = "medical")] test_medical_entity_type` | `lib.rs` | Verify medical enum available |
| `#[cfg(feature = "hgnc")] test_hgnc_normalizer` | `hgnc.rs` | Verify gene normalizer works |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_ontology_without_medical` | `tests/ontology.rs` | Verify works without medical feature |
| `test_ontology_with_medical` | `tests/ontology.rs` | Verify medical feature works |

### Build Tests
```bash
# Without medical features
cargo build -p terraphim_types --no-default-features

# With medical features
cargo build -p terraphim_types --features medical

# With all features
cargo build -p terraphim_types --features "medical,hgnc"
```

## Implementation Steps

### Step 1: Update Cargo.toml with Feature Gates
**Files:** `crates/terraphim_types/Cargo.toml`
**Description:** Add feature gates
**Estimated:** 30 minutes

```toml
[features]
default = ["ontology"]
ontology = []       # Core generic ontology (string-based types)
medical = ["ontology"]  # Medical/oncology extensions
hgnc = ["medical"] # HGNC gene normalization
```

### Step 2: Refactor ExtractedEntity to String-based EntityType
**Files:** `crates/terraphim_types/src/lib.rs`
**Description:** Change entity_type from enum to String
**Tests:** Unit tests for string entity type
**Estimated:** 1 hour

### Step 3: Add Feature-Gated Medical Types
**Files:** `crates/terraphim_types/src/lib.rs`
**Description:** Add `#[cfg(feature = "medical")]` module with EntityType/RelationshipType enums
**Tests:** Feature-gated unit tests
**Estimated:** 1 hour

### Step 4: Feature-Gate HGNC Normalizer
**Files:** `crates/terraphim_types/src/hgnc.rs`
**Description:** Add `#[cfg(feature = "hgnc")]` to module
**Tests:** Feature-gated unit tests
**Estimated:** 30 minutes

### Step 5: Update Example and Verify
**Files:** `crates/terraphim_types/examples/ontology_usage.rs`
**Description:** Update example to use string-based types
**Tests:** Run example
**Estimated:** 30 minutes

### Step 6: Run Full Test Suite
**Files:** All modified
**Description:** Verify no regressions
**Tests:** All ontology tests
**Estimated:** 30 minutes

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| None | - | Using existing dependencies |

### No Dependency Updates Required

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| Binary size (no features) | < 100KB | Measure with `cargo size` |
| Binary size (all features) | < 500KB | Measure with `cargo size` |
| Coverage calc latency | < 1ms | Benchmark |

### No Benchmarks Required
The changes are compile-time feature gates, no runtime performance impact.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Approve research document | Pending | - |
| Approve this implementation plan | Pending | - |
| Implement Step 1 | Pending | - |
| Implement Step 2 | Pending | - |
| Implement Step 3 | Pending | - |
| Implement Step 4 | Pending | - |
| Implement Step 5 | Pending | - |
| Verify all tests pass | Pending | - |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

## Next Steps

After approval:
1. Begin implementation with Step 1
2. After each step, run relevant tests
3. Final verification with all feature combinations
