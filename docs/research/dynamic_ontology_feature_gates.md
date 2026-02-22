# Research Document: Dynamic Ontology Feature Gates

**Status**: Draft
**Author**: Claude
**Date**: 2026-02-20
**Reviewers**: TBD

## Executive Summary

The Dynamic Ontology implementation currently has oncology-specific types hardcoded (EntityType, RelationshipType) and a gene-specific normalizer (HgncNormalizer). This research evaluates how to refactor for cross-domain use while maintaining optional medical/oncology features via feature gates. The recommendation is to make EntityType/RelationshipType generic/configurable and feature-gate the HGNC normalizer.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Core infrastructure should be domain-agnostic |
| Leverages strengths? | Yes | Uses existing fuzzy matching and graph rank |
| Meets real need? | Yes | Users want ontology for multiple domains (legal, tech, etc.) |

**Proceed**: Yes - at least 2/3 YES

## Problem Statement

### Description
Current Dynamic Ontology implementation couples domain logic with generic pipeline:
- `EntityType` enum has oncology values (CancerDiagnosis, Tumor, GenomicVariant)
- `RelationshipType` enum has oncology values (HasTumor, HasVariant)
- `HgncNormalizer` is hardcoded with oncology genes
- Cannot use for other domains (legal, software, finance)

### Impact
- Users cannot use Dynamic Ontology for non-oncology domains
- Cannot serve multiple domains simultaneously
- HGNC adds dependency weight for users who don't need it

### Success Criteria
- Generic types work without oncology dependencies
- Feature gate enables oncology-specific features
- HGNC normalizer optional via feature flag
- Existing oncology use cases still work

## Current State Analysis

### Existing Implementation
The Dynamic Ontology pipeline consists of:

| Component | Location | Status |
|-----------|----------|--------|
| EntityType enum | `lib.rs:2138` | Oncology-specific |
| RelationshipType enum | `lib.rs:2151` | Oncology-specific |
| GroundingMetadata | `lib.rs:2050` | Generic |
| CoverageSignal | `lib.rs:2100` | Generic |
| SchemaSignal | `lib.rs:2170` | Generic |
| ExtractedEntity | `lib.rs:2159` | Generic |
| HgncNormalizer | `hgnc.rs` | Oncology-specific |
| OntologyWorkflow | `ontology_workflow.rs` | Generic |

### Generic Components (No Change Needed)
```rust
// These work for any domain - no changes needed
pub struct GroundingMetadata {
    pub normalized_uri: Option<String>,
    pub normalized_label: Option<String>,
    pub normalized_prov: Option<String>,  // e.g., "HGNC", "NCIt", or custom
    pub normalized_score: Option<f32>,
    pub normalized_method: Option<NormalizationMethod>,
}

pub struct CoverageSignal {
    pub total_categories: usize,
    pub matched_categories: usize,
    pub coverage_ratio: f32,
    pub threshold: f32,
    pub needs_review: bool,
}
```

### Domain-Specific Components (Need Refactoring)
```rust
// Current: Hardcoded oncology types
pub enum EntityType {
    CancerDiagnosis,  // oncology-specific
    Tumor,           // oncology-specific
    GenomicVariant,  // oncology-specific
    // ...
}

// Current: Hardcoded oncology relationships
pub enum RelationshipType {
    HasTumor,       // oncology-specific
    HasVariant,     // oncology-specific
    // ...
}
```

## Constraints

### Technical Constraints
- Must maintain backward compatibility with existing oncology use
- Feature gates must not break existing functionality
- Generic solution should not require runtime configuration for simple cases

### Non-Functional Requirements
| Requirement | Target |
|-------------|--------|
| Binary size (no oncology) | < 500KB overhead |
| Binary size (with oncology) | < 1MB overhead |
| Latency | < 10ms for coverage calculation |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Make EntityType configurable | Core abstraction | Cannot serve multiple domains |
| Feature gate HGNC | Reduce dependencies | Users don't always need genes |
| Keep generic pipeline intact | Existing functionality | Already tested, works |

### Eliminated from Scope
| Item | Why Eliminated |
|------|----------------|
| Vector embeddings | Not needed - using fuzzy matching |
| Full ontology import | Too complex for v1 |
| Runtime domain switching | Compile-time is simpler |

## Recommendations

### Proceed: Yes

The refactoring is essential for cross-domain use. Current implementation blocks:
1. Legal document ontology
2. Software code ontology
3. Finance terminology ontology

### Recommended Approach

**Option A: Generic Enum + Feature Gate (Recommended)**
```rust
// Generic base - works for any domain
#[cfg(feature = "ontology")]
pub mod medical {
    pub enum EntityType {
        // oncology types
    }
}

// Usage without oncology feature:
let entity_type = "Person"; // String-based for generic
```

**Option B: String-Based Type System**
```rust
// Fully generic - any string as entity type
pub struct ExtractedEntity {
    pub entity_type: String,  // "Person", "Organization", "CancerDiagnosis"
    pub raw_value: String,
    // ...
}
```

### Scope Recommendations
1. Keep GroundingMetadata, CoverageSignal, SchemaSignal as-is (generic)
2. Make EntityType configurable via `#[cfg(feature = "medical")]` or string
3. Feature-gate HgncNormalizer with `#[cfg(feature = "hgnc")]`
4. Keep existing oncology types behind feature flag

### Risk Mitigation
- Risk: Breaking existing oncology users
- Mitigation: Keep oncology as default feature
- Risk: Performance overhead from strings
- Mitigation: Use String interning/caching

## Next Steps

1. Approve this research document
2. Proceed to Phase 2 (Disciplined Design) for implementation plan
3. Define exact API changes for generic EntityType
4. Create feature gate structure in Cargo.toml

## Appendix

### Feature Gate Proposal
```toml
[features]
default = ["ontology"]
ontology = []        # Core generic ontology types (EntityType, RelationshipType without variants)
medical = ["ontology"]  # Medical/oncology extensions
hgnc = ["medical"]    # HGNC gene normalization
```

### Alternative: String-Based Approach
Could replace enum with String for maximum flexibility:
```rust
pub struct ExtractedEntity {
    pub entity_type: String,  // User-defined
    pub raw_value: String,
    // ...
}

// Validation can be added via configuration
let validator = EntityValidator::new()
    .allow("Person", "Organization")
    .allow_if_feature("CancerDiagnosis", feature = "medical");
```
