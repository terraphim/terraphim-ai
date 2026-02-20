//! Dynamic Ontology Usage Examples
//!
//! This example demonstrates how to use the Dynamic Ontology pipeline
//! for extracting entities from text and normalizing them to an ontology.

use terraphim_types::hgnc::HgncNormalizer;
use terraphim_types::{
    CoverageSignal, EntityType, ExtractedEntity, GroundingMetadata, RelationshipType, SchemaSignal,
};

fn main() {
    println!("=== Dynamic Ontology Usage Examples ===\n");

    // Example 1: HGNC Gene Normalization
    println!("1. HGNC Gene Normalization");
    println!("---------------------------");
    example_hgnc_normalization();

    // Example 2: Coverage Signal
    println!("\n2. Coverage Signal Calculation");
    println!("-------------------------------");
    example_coverage_signal();

    // Example 3: Schema Signal Creation
    println!("\n3. Schema Signal Creation");
    println!("-------------------------");
    example_schema_signal();

    // Example 4: Full Pipeline
    println!("\n4. Full Extraction Pipeline");
    println!("-----------------------------");
    example_full_pipeline();
}

fn example_hgnc_normalization() {
    // Create a new HGNC normalizer with oncology genes
    let normalizer = HgncNormalizer::new();

    // Test exact match
    let result = normalizer.normalize("EGFR");
    println!("  EGFR -> {:?}", result.map(|g| g.normalized_label));

    // Test alias (ERBB1 is an alias for EGFR)
    let result = normalizer.normalize("ERBB1");
    println!(
        "  ERBB1 (alias) -> {:?}",
        result.map(|g| g.normalized_label)
    );

    // Test alias (HER2 is an alias for ERBB2)
    let result = normalizer.normalize("HER2");
    println!("  HER2 (alias) -> {:?}", result.map(|g| g.normalized_label));

    // Test fuzzy variant (EGFRvIII is a variant of EGFR)
    let result = normalizer.normalize("EGFRvIII");
    println!(
        "  EGFRvIII (fuzzy) -> {:?}",
        result.map(|g| g.normalized_label)
    );

    // Test TP53
    let result = normalizer.normalize("TP53");
    println!("  TP53 -> {:?}", result.map(|g| g.normalized_label));

    // Test unknown gene
    let result = normalizer.normalize("XYZ123");
    println!("  XYZ123 (unknown) -> {:?}", result);
}

fn example_coverage_signal() {
    // Create entities with varying grounding
    let entities = vec![
        ExtractedEntity {
            entity_type: EntityType::CancerDiagnosis,
            raw_value: "non-small cell lung cancer".to_string(),
            normalized_value: Some("Non-Small Cell Lung Cancer".to_string()),
            grounding: Some(GroundingMetadata::new(
                "http://example.org/nsclc".to_string(),
                "Non-Small Cell Lung Cancer".to_string(),
                "NCIt".to_string(),
                0.95,
                terraphim_types::NormalizationMethod::Exact,
            )),
        },
        ExtractedEntity {
            entity_type: EntityType::Drug,
            raw_value: "Osimertinib".to_string(),
            normalized_value: Some("Osimertinib".to_string()),
            grounding: Some(GroundingMetadata::new(
                "http://example.org/osimertinib".to_string(),
                "Osimertinib".to_string(),
                "NCIt".to_string(),
                0.98,
                terraphim_types::NormalizationMethod::Exact,
            )),
        },
        ExtractedEntity {
            entity_type: EntityType::GenomicVariant,
            raw_value: "Unknown mutation".to_string(),
            normalized_value: None,
            grounding: None,
        },
    ];

    // Calculate categories
    let categories: Vec<String> = entities
        .iter()
        .map(|e| e.normalized_value.clone().unwrap_or(e.raw_value.clone()))
        .collect();

    // Count matched (entities with grounding)
    let matched = entities.iter().filter(|e| e.grounding.is_some()).count();

    // Compute coverage with 0.7 threshold
    let coverage = CoverageSignal::compute(&categories, matched, 0.7);

    println!("  Total categories: {}", coverage.total_categories);
    println!("  Matched categories: {}", coverage.matched_categories);
    println!("  Coverage ratio: {:.1}%", coverage.coverage_ratio * 100.0);
    println!("  Threshold: {:.0}%", coverage.threshold * 100.0);
    println!("  Needs review: {}", coverage.needs_review);
}

fn example_schema_signal() {
    // Create a schema signal from extracted oncology data
    let entities = vec![
        ExtractedEntity {
            entity_type: EntityType::CancerDiagnosis,
            raw_value: "lung carcinoma".to_string(),
            normalized_value: Some("Lung Carcinoma".to_string()),
            grounding: Some(GroundingMetadata::new(
                "http://example.org/lung_carcinoma".to_string(),
                "Lung Carcinoma".to_string(),
                "NCIt".to_string(),
                0.95,
                terraphim_types::NormalizationMethod::Exact,
            )),
        },
        ExtractedEntity {
            entity_type: EntityType::GenomicVariant,
            raw_value: "EGFR L858R".to_string(),
            normalized_value: Some("EGFR L858R".to_string()),
            grounding: None,
        },
    ];

    let relationships = vec![];

    let schema_signal = SchemaSignal {
        entities,
        relationships,
        confidence: 0.5,
    };

    println!("  Entities: {}", schema_signal.entities.len());
    println!("  Relationships: {}", schema_signal.relationships.len());
    println!("  Confidence: {:.0}%", schema_signal.confidence * 100.0);
}

fn example_full_pipeline() {
    println!("  Step 1: Extract entities from text");
    println!(
        "    Input: 'Patient with EGFR L858R mutation in lung carcinoma treated with Osimertinib'"
    );
    println!(
        "    -> Extract: EGFR L858R (GenomicVariant), lung carcinoma (CancerDiagnosis), Osimertinib (Drug)"
    );

    println!("\n  Step 2: Normalize entities to ontology");
    let normalizer = HgncNormalizer::new();

    // Normalize EGFR
    let egfr = normalizer.normalize("EGFR");
    println!(
        "    EGFR -> {}",
        egfr.as_ref()
            .map(|g| format!(
                "{} (score: {:.2})",
                g.normalized_label.as_ref().unwrap(),
                g.normalized_score.unwrap()
            ))
            .unwrap_or_else(|| "Not found".to_string())
    );

    println!("\n  Step 3: Check coverage");
    println!("    2/3 entities grounded = 66.7% coverage");
    println!("    Threshold: 70% -> needs review: true");

    println!("\n  Step 4: Review (if needed)");
    println!("    Review Agent suggests corrections for unmatched entities");

    println!("\n  Result: Grounded knowledge graph with coverage signal");
}
