//! SNOMED CT data structures
//!
//! Types for representing SNOMED CT concepts, semantic types, and matches.
//! Used by the medical entity extraction pipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Semantic type categories in SNOMED CT
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SemanticType {
    Disease,
    Finding,
    Procedure,
    Substance,
    Pharmaceutical,
    Gene,
    Variant,
    Anatomical,
    Observable,
    Specimen,
    BodyStructure,
    Event,
    Environment,
    SocialConcept,
    Stated,
    Group,
    #[serde(other)]
    #[default]
    Unknown,
}

/// A SNOMED CT concept
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnomedConcept {
    /// SNOMED CT Concept ID
    pub id: u64,
    /// Preferred term
    pub term: String,
    /// Semantic type
    #[serde(alias = "semantic")]
    pub semantic_type: SemanticType,
    /// Alternative terms/synonyms
    #[serde(default)]
    pub synonyms: Vec<String>,
    /// Parent concept IDs (IS-A relationships)
    #[serde(default)]
    pub parents: Vec<u64>,
}

impl SnomedConcept {
    pub fn new(id: u64, term: String, semantic_type: SemanticType) -> Self {
        Self {
            id,
            term,
            semantic_type,
            synonyms: Vec::new(),
            parents: Vec::new(),
        }
    }
}

/// A matched entity from SNOMED extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnomedMatch {
    /// Matched SNOMED concept ID
    pub concept_id: u64,
    /// Matched text term
    pub term: String,
    /// Canonical (preferred) term
    pub canonical: String,
    /// Semantic type
    pub semantic_type: SemanticType,
    /// Character span in original text
    pub span: (usize, usize),
    /// Match confidence (0.0 to 1.0)
    pub confidence: f32,
}

/// SNOMED subset data for loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnomedSubset {
    pub concepts: Vec<SnomedConcept>,
}

impl SnomedSubset {
    /// Parse from JSON -- accepts either a raw array of concepts or `{"concepts": [...]}`
    pub fn from_json(data: &[u8]) -> anyhow::Result<Self> {
        // Try parsing as a raw array first (most common in tests/data files)
        if let Ok(concepts) = serde_json::from_slice::<Vec<SnomedConcept>>(data) {
            return Ok(Self { concepts });
        }
        // Fall back to wrapped object format
        Ok(serde_json::from_slice(data)?)
    }
}

/// Create an in-memory index for quick lookup
pub fn create_concept_index(concepts: &[SnomedConcept]) -> HashMap<u64, SnomedConcept> {
    concepts.iter().map(|c| (c.id, c.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_type_parse() {
        let json = r#"{"semantic": "Disease"}"#;
        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        let st = serde_json::from_value::<SemanticType>(parsed.get("semantic").unwrap().clone());
        assert!(st.is_ok());
        assert_eq!(st.unwrap(), SemanticType::Disease);
    }

    #[test]
    fn test_snomed_subset_parsing() {
        let data = r#"[
            {"id": 254637007, "term": "NSCLC", "semantic": "Disease"},
            {"id": 363358000, "term": "EGFR", "semantic": "Gene"}
        ]"#;
        let subset = SnomedSubset::from_json(data.as_bytes());
        assert!(subset.is_ok());
        assert_eq!(subset.unwrap().concepts.len(), 2);
    }
}
