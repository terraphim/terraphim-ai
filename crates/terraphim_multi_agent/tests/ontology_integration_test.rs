//! Dynamic Ontology Integration Tests
//!
//! Integration tests for the Dynamic Ontology multi-agent pipeline.
//! These tests validate the end-to-end flow from text extraction to normalization.

#[cfg(test)]
mod tests {
    use terraphim_types::hgnc::HgncNormalizer;
    use terraphim_types::{
        CoverageSignal, EntityType, ExtractedEntity, GroundingMetadata, SchemaSignal,
    };

    /// Test the HGNC gene normalizer with known oncology genes
    #[test]
    fn test_hgnc_normalizer_egfr() {
        let normalizer = HgncNormalizer::new();

        // Test exact match
        let result = normalizer.normalize("EGFR");
        assert!(result.is_some());

        let meta = result.unwrap();
        assert_eq!(meta.normalized_label, Some("EGFR".to_string()));
        assert!(meta.normalized_score.unwrap() > 0.9);
    }

    /// Test HGNC with alias
    #[test]
    fn test_hgnc_normalizer_erbb1_alias() {
        let normalizer = HgncNormalizer::new();

        // ERBB1 is an alias for EGFR
        let result = normalizer.normalize("ERBB1");
        assert!(result.is_some());

        let meta = result.unwrap();
        assert_eq!(meta.normalized_label, Some("EGFR".to_string()));
    }

    /// Test HGNC fuzzy matching
    #[test]
    fn test_hgnc_normalizer_fuzzy_variant() {
        let normalizer = HgncNormalizer::new();

        // EGFRvIII should fuzzy match to EGFR
        let result = normalizer.normalize("EGFRvIII");
        assert!(result.is_some());

        let meta = result.unwrap();
        assert_eq!(meta.normalized_label, Some("EGFR".to_string()));
    }

    /// Test coverage signal calculation
    #[test]
    fn test_coverage_signal_calculation() {
        let entities = vec![
            ExtractedEntity {
                entity_type: EntityType::CancerDiagnosis,
                raw_value: "lung carcinoma".to_string(),
                normalized_value: Some("lung carcinoma".to_string()),
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
                raw_value: "EGFR".to_string(),
                normalized_value: None,
                grounding: None,
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
        ];

        let coverage = CoverageSignal::compute(
            &entities
                .iter()
                .map(|e| e.normalized_value.clone().unwrap_or(e.raw_value.clone()))
                .collect::<Vec<_>>(),
            2, // 2 matched out of 3
            0.7,
        );

        assert_eq!(coverage.total_categories, 3);
        assert_eq!(coverage.matched_categories, 2);
        assert!((coverage.coverage_ratio - 0.667).abs() < 0.01);
        assert!(coverage.needs_review); // 0.667 < 0.7 threshold
    }

    /// Test coverage above threshold
    #[test]
    fn test_coverage_above_threshold() {
        let entities = vec![
            ExtractedEntity {
                entity_type: EntityType::CancerDiagnosis,
                raw_value: "lung carcinoma".to_string(),
                normalized_value: Some("lung carcinoma".to_string()),
                grounding: Some(GroundingMetadata::new(
                    "http://example.org/lung_carcinoma".to_string(),
                    "Lung Carcinoma".to_string(),
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
        ];

        let coverage = CoverageSignal::compute(
            &entities
                .iter()
                .map(|e| e.normalized_value.clone().unwrap_or(e.raw_value.clone()))
                .collect::<Vec<_>>(),
            2, // 2 matched out of 2
            0.5,
        );

        assert_eq!(coverage.total_categories, 2);
        assert_eq!(coverage.matched_categories, 2);
        assert!(!coverage.needs_review); // 1.0 > 0.5 threshold
    }

    /// Test schema signal creation
    #[test]
    fn test_schema_signal_creation() {
        let entities = vec![
            ExtractedEntity {
                entity_type: EntityType::CancerDiagnosis,
                raw_value: "non-small cell lung carcinoma".to_string(),
                normalized_value: Some("Non-Small Cell Lung Carcinoma".to_string()),
                grounding: Some(GroundingMetadata::new(
                    "http://example.org/nsclc".to_string(),
                    "Non-Small Cell Lung Carcinoma".to_string(),
                    "NCIt".to_string(),
                    0.92,
                    terraphim_types::NormalizationMethod::Exact,
                )),
            },
            ExtractedEntity {
                entity_type: EntityType::GenomicVariant,
                raw_value: "EGFR L858R".to_string(),
                normalized_value: None,
                grounding: None,
            },
        ];

        let schema_signal = SchemaSignal {
            entities,
            relationships: vec![],
            confidence: 0.5,
        };

        assert_eq!(schema_signal.entities.len(), 2);
        assert_eq!(schema_signal.relationships.len(), 0);
        assert_eq!(schema_signal.confidence, 0.5);
    }

    /// Test grounding metadata creation
    #[test]
    fn test_grounding_metadata_creation() {
        let grounding = GroundingMetadata::new(
            "http://example.org/egfr".to_string(),
            "EGFR".to_string(),
            "HGNC".to_string(),
            1.0,
            terraphim_types::NormalizationMethod::Exact,
        );

        assert_eq!(grounding.normalized_uri, Some("http://example.org/egfr".to_string()));
        assert_eq!(grounding.normalized_label, Some("EGFR".to_string()));
        assert_eq!(grounding.normalized_prov, Some("HGNC".to_string()));
        assert_eq!(grounding.normalized_score, Some(1.0));
        assert_eq!(
            grounding.normalized_method,
            Some(terraphim_types::NormalizationMethod::Exact)
        );
    }

    /// Test multiple HGNC genes
    #[test]
    fn test_hgnc_multiple_genes() {
        let normalizer = HgncNormalizer::new();

        let genes = ["EGFR", "TP53", "KRAS", "BRAF", "ALK", "ROS1", "MET"];

        for gene in genes {
            let result = normalizer.normalize(gene);
            assert!(
                result.is_some(),
                "Expected to find {} in HGNC database",
                gene
            );
        }
    }
}
