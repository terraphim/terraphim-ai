# Dynamic Ontology

Terraphim's Dynamic Ontology enables schema-first knowledge graph construction with coverage governance signals. This document explains the architecture, feature gates, and usage.

## Overview

Dynamic Ontology is a schema-first approach to knowledge graphs inspired by Akash Goyal's methodology. It provides:

- **Two-layer architecture**: Curated schema + ontology catalog
- **Extraction**: LLM-based entity extraction with schema signals
- **Normalization**: Aho-Corasick + fuzzy matching (no vector embeddings needed)
- **Coverage governance**: Quality signals to judge extraction quality
- **Grounding metadata**: Canonical URIs for interoperability

## Architecture

```
Text Input
    ↓
┌─────────────────────────────────────────────┐
│ Multi-Agent Orchestration                   │
├─────────────────────────────────────────────┤
│  1. Extraction Agent  → Schema signals       │
│  2. Normalization Agent → Graph + fuzzy     │
│  3. Coverage Agent  → Governance signal     │
│  4. Review Agent → QA on low-confidence    │
└─────────────────────────────────────────────┘
    ↓
Grounded Knowledge Graph
```

## Feature Gates

The Dynamic Ontology module uses Rust feature gates to support both generic and domain-specific use cases:

| Feature | Description |
|---------|-------------|
| `ontology` (default) | Core generic types (GroundingMetadata, CoverageSignal, SchemaSignal) |
| `medical` | Medical/oncology EntityType/RelationshipType enums |
| `hgnc` | HGNC gene normalization (EGFR, TP53, KRAS, etc.) |

### Feature Hierarchy

```
ontology (default)
    ↓
medical (implies ontology)
    ↓
hgnc (implies medical)
```

### Usage

```toml
# Cargo.toml
[dependencies]
terraphim_types = { version = "1.4", features = ["ontology"] }  # Generic only
terraphim_types = { version = "1.4", features = ["medical"] }  # Medical types
terraphim_types = { version = "1.4", features = ["hgnc"] }      # Full medical + HGNC
```

## Core Types

### GroundingMetadata

```rust
pub struct GroundingMetadata {
    pub normalized_uri: Option<String>,      // Canonical URI
    pub normalized_label: Option<String>,    // Canonical term
    pub normalized_prov: Option<String>,    // Ontology source (NCIt, HGNC, etc.)
    pub normalized_score: Option<f32>,      // Confidence score
    pub normalized_method: Option<NormalizationMethod>,
}
```

### NormalizationMethod

```rust
pub enum NormalizationMethod {
    Exact,      // Aho-Corasick exact match
    Fuzzy,      // Levenshtein/Jaro-Winkler
    GraphRank,  // Node rank from co-occurrence
}
```

### CoverageSignal

```rust
pub struct CoverageSignal {
    pub total_categories: usize,
    pub matched_categories: usize,
    pub coverage_ratio: f32,
    pub threshold: f32,
    pub needs_review: bool,
}
```

### SchemaSignal

```rust
pub struct SchemaSignal {
    pub entities: Vec<ExtractedEntity>,
    pub relationships: Vec<ExtractedRelationship>,
    pub confidence: f32,
}

pub struct ExtractedEntity {
    pub entity_type: String,           // Always available (String-based)
    pub raw_value: String,
    pub normalized_value: Option<String>,
    pub grounding: Option<GroundingMetadata>,
}
```

### Medical Entity Types (feature-gated)

When `medical` feature is enabled:

```rust
pub enum EntityType {
    CancerDiagnosis,
    Tumor,
    GenomicVariant,
    Biomarker,
    Drug,
    Treatment,
    SideEffect,
}
```

## Usage Examples

### Basic Entity Extraction

```rust
use terraphim_types::{ExtractedEntity, GroundingMetadata, CoverageSignal};

// Create extracted entity
let entity = ExtractedEntity {
    entity_type: "concept".to_string(),
    raw_value: "knowledge graph".to_string(),
    normalized_value: Some("Knowledge Graph".to_string()),
    grounding: Some(GroundingMetadata::new(
        "http://example.org/knowledge_graph".to_string(),
        "Knowledge Graph".to_string(),
        "custom".to_string(),
        0.95,
        terraphim_types::NormalizationMethod::Exact,
    )),
};

// Compute coverage
let categories = vec!["Knowledge Graph".to_string()];
let coverage = CoverageSignal::compute(&categories, 1, 0.7);

println!("Coverage: {:.1}%", coverage.coverage_ratio * 100.0);
```

### HGNC Gene Normalization (requires `hgnc` feature)

```rust
use terraphim_types::hgnc::HgncNormalizer;

let normalizer = HgncNormalizer::new();

// Exact match
let result = normalizer.normalize("EGFR");
assert!(result.is_some());

// Alias resolution (ERBB1 -> EGFR)
let result = normalizer.normalize("ERBB1");
assert_eq!(result.unwrap().normalized_label, Some("EGFR".to_string()));

// Fuzzy variant matching (EGFRvIII -> EGFR)
let result = normalizer.normalize("EGFRvIII");
assert_eq!(result.unwrap().normalized_label, Some("EGFR".to_string()));
```

### Knowledge Graph Normalization Example

The `kg_normalization` example demonstrates full pipeline:

```bash
cargo run --example kg_normalization -p terraphim_types
```

This example:
1. Loads markdown documents from a knowledge corpus
2. Builds ontology with extracted terms
3. Extracts entities from sample text
4. Computes coverage signals
5. Exports thesaurus for Terraphim automata

## Multi-Agent Workflow

The `terraphim_multi_agent` crate provides specialized agents:

```rust
use terraphim_multi_agent::agents::ontology_agents::{
    ExtractionAgent, NormalizationAgent, CoverageAgent, ReviewAgent,
};

// Create extraction agent
let extraction_agent = ExtractionAgent::new(config.clone())?;

// Extract schema signal
let schema_signal = extraction_agent.extract(text).await?;

// Create normalization agent with ontology terms
let normalizer = NormalizationAgent::new(config, ontology_terms)?;
let normalized = normalizer.normalize(schema_signal.entities).await?;

// Compute coverage
let coverage_agent = CoverageAgent::new(0.7);
let coverage = coverage_agent.compute_coverage(&normalized);

// Review low-confidence matches
let review_agent = ReviewAgent::new(config, ontology_terms, 0.5)?;
let reviewed = review_agent.review(&mut normalized.clone()).await?;
```

## Coverage Governance

Coverage signals provide quality governance:

| Coverage Ratio | Status | Action |
|----------------|--------|--------|
| >= 70% | Excellent | Proceed |
| 40-70% | Good | Minor review |
| 20-40% | Needs improvement | Expand ontology |
| < 20% | Poor | Different approach needed |

## Performance

- **No vector embeddings required** - Uses existing Aho-Corasick + fuzzy matching
- **Graph ranking** - Leverages existing rolegraph node rankings
- **Feature-gated** - Pay only for what you use

## References

- [Research Document](./research/dynamic_ontology_feature_gates.md)
- [Implementation Plan](./research/dynamic_ontology_feature_gates_impl.md)
