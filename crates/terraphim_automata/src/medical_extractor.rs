//! Entity extraction using Aho-Corasick automaton for SNOMED CT concepts
//!
//! Provides fast multi-pattern matching for clinical text using the
//! Aho-Corasick algorithm with SNOMED CT terminology.

use aho_corasick::AhoCorasick;
use std::collections::HashMap;

use crate::snomed::{SemanticType, SnomedConcept, SnomedMatch, SnomedSubset};

/// Entity extractor using Aho-Corasick multi-pattern automaton
///
/// Builds an Aho-Corasick automaton from all concept terms and synonyms for
/// O(n + m + z) extraction where n = text length, m = total pattern length,
/// z = number of matches. This replaces the naive O(n*p) substring scan.
pub struct EntityExtractor {
    /// Concept lookup by ID
    concept_index: HashMap<u64, SnomedConcept>,
    /// Parent relationships for subsumption checking
    parent_map: HashMap<u64, Vec<u64>>,
    /// Aho-Corasick automaton for multi-pattern matching
    automaton: AhoCorasick,
    /// Pattern index -> concept_id mapping (parallel to automaton patterns)
    pattern_concept_ids: Vec<u64>,
    /// Pattern index -> original term (parallel to automaton patterns)
    pattern_terms: Vec<String>,
}

impl EntityExtractor {
    /// Create a new entity extractor from SNOMED JSON data
    pub fn new(snomed_data: &[u8]) -> anyhow::Result<Self> {
        let subset = SnomedSubset::from_json(snomed_data)?;

        let mut concept_index: HashMap<u64, SnomedConcept> = HashMap::new();
        let mut parent_map: HashMap<u64, Vec<u64>> = HashMap::new();
        let mut pattern_terms: Vec<String> = Vec::new();
        let mut pattern_concept_ids: Vec<u64> = Vec::new();

        for concept in &subset.concepts {
            // Add preferred term
            let term_lower = concept.term.to_lowercase();
            if !term_lower.is_empty() {
                pattern_terms.push(term_lower);
                pattern_concept_ids.push(concept.id);
            }

            // Add synonyms
            for syn in &concept.synonyms {
                let syn_lower = syn.to_lowercase();
                if !syn_lower.is_empty() {
                    pattern_terms.push(syn_lower);
                    pattern_concept_ids.push(concept.id);
                }
            }

            // Store concept
            concept_index.insert(concept.id, concept.clone());

            // Store parent relationships
            if !concept.parents.is_empty() {
                parent_map.insert(concept.id, concept.parents.clone());
            }
        }

        // Build Aho-Corasick automaton from patterns
        let automaton = AhoCorasick::new(&pattern_terms)?;

        Ok(Self {
            concept_index,
            parent_map,
            automaton,
            pattern_concept_ids,
            pattern_terms,
        })
    }

    /// Create a simple extractor from predefined terms
    pub fn from_terms(terms: Vec<(&str, u64)>) -> Self {
        let mut pattern_terms: Vec<String> = Vec::new();
        let mut pattern_concept_ids: Vec<u64> = Vec::new();

        for (term, id) in terms {
            let term_lower = term.to_lowercase();
            pattern_terms.push(term_lower);
            pattern_concept_ids.push(id);
        }

        // Build Aho-Corasick automaton from patterns
        let automaton =
            AhoCorasick::new(&pattern_terms).expect("Failed to build Aho-Corasick automaton");

        Self {
            concept_index: HashMap::new(),
            parent_map: HashMap::new(),
            automaton,
            pattern_concept_ids,
            pattern_terms,
        }
    }

    /// Extract SNOMED entities from clinical text
    ///
    /// Returns entities sorted by position in text
    pub fn extract(&self, text: &str) -> Vec<SnomedMatch> {
        self.extract_with_confidence(text)
            .into_iter()
            .map(|m| SnomedMatch {
                concept_id: m.concept_id,
                term: m.term,
                canonical: m.canonical,
                semantic_type: m.semantic_type,
                span: m.span,
                confidence: m.confidence,
            })
            .collect()
    }

    /// Extract entities with confidence scores
    pub fn extract_with_confidence(&self, text: &str) -> Vec<ExtractedEntity> {
        let text_lower = text.to_lowercase();
        let mut matches: Vec<ExtractedEntity> = Vec::new();

        // Use Aho-Corasick automaton for O(n + m + z) matching
        for mat in self.automaton.find_iter(&text_lower) {
            let pattern_index = mat.pattern().as_usize();
            let concept_id = self.pattern_concept_ids[pattern_index];
            let term = &self.pattern_terms[pattern_index];

            // Get concept details if available
            let (canonical, semantic_type) =
                if let Some(concept) = self.concept_index.get(&concept_id) {
                    (concept.term.clone(), concept.semantic_type)
                } else {
                    (term.clone(), SemanticType::Unknown)
                };

            // Check for overlapping matches
            let start = mat.start();
            let end = mat.end();

            // Skip if overlapping with existing match
            let overlaps = matches.iter().any(|m| {
                (start >= m.span.0 && start < m.span.1) || (end > m.span.0 && end <= m.span.1)
            });

            if !overlaps {
                matches.push(ExtractedEntity {
                    concept_id,
                    term: text[start..end].to_string(),
                    canonical,
                    semantic_type,
                    span: (start, end),
                    confidence: 1.0,
                });
            }
        }

        // Sort by position in text
        matches.sort_by_key(|m| m.span.0);

        matches
    }

    /// Get a SNOMED concept by ID
    pub fn get_concept(&self, concept_id: u64) -> Option<&SnomedConcept> {
        self.concept_index.get(&concept_id)
    }

    /// Check if concept A is a descendant of concept B in the hierarchy
    pub fn is_descendant(&self, child: u64, parent: u64) -> bool {
        self.get_ancestors(child).contains(&parent)
    }

    /// Get all ancestors of a concept
    pub fn get_ancestors(&self, concept: u64) -> Vec<u64> {
        let mut ancestors = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![concept];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(parents) = self.parent_map.get(&current) {
                for &parent in parents {
                    if !ancestors.contains(&parent) {
                        ancestors.push(parent);
                        stack.push(parent);
                    }
                }
            }
        }

        ancestors
    }
}

/// Extended extracted entity with additional fields
#[derive(Debug, Clone)]
pub struct ExtractedEntity {
    pub concept_id: u64,
    pub term: String,
    pub canonical: String,
    pub semantic_type: SemanticType,
    pub span: (usize, usize),
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_extractor() -> EntityExtractor {
        let data = r#"[
            {"id": 254637007, "term": "Non-small cell lung carcinoma", "semantic": "Disease", "parents": [363358000]},
            {"id": 363358000, "term": "Lung carcinoma", "semantic": "Disease", "parents": [63250001]},
            {"id": 63250001, "term": "Lung cancer", "semantic": "Disease", "parents": []},
            {"id": 363358001, "term": "EGFR", "semantic": "Gene", "synonyms": ["Epidermal growth factor receptor"]},
            {"id": 86249004, "term": "Gefitinib", "semantic": "Pharmaceutical"}
        ]"#;
        EntityExtractor::new(data.as_bytes()).unwrap()
    }

    #[test]
    fn test_extract_single_entity() {
        let extractor = create_test_extractor();
        let result = extractor.extract("Non-small cell lung carcinoma");

        assert!(!result.is_empty());
        assert_eq!(result[0].semantic_type, SemanticType::Disease);
    }

    #[test]
    fn test_extract_multiple_entities() {
        let extractor = create_test_extractor();
        let result = extractor.extract("Patient with EGFR mutation and NSCLC");

        // Should extract EGFR
        assert!(!result.is_empty());
    }

    #[test]
    fn test_extract_no_match() {
        let extractor = create_test_extractor();
        let result = extractor.extract("Patient feeling well");

        assert!(result.is_empty());
    }

    #[test]
    fn test_confidence_scoring() {
        let extractor = create_test_extractor();
        let result = extractor.extract_with_confidence("Patient has lung cancer");

        for entity in &result {
            assert!(entity.confidence >= 0.0 && entity.confidence <= 1.0);
        }
    }

    #[test]
    fn test_get_concept() {
        let extractor = create_test_extractor();
        let concept = extractor.get_concept(254637007);

        assert!(concept.is_some());
        assert_eq!(concept.unwrap().term, "Non-small cell lung carcinoma");
    }

    #[test]
    fn test_ancestor_query() {
        let extractor = create_test_extractor();
        let ancestors = extractor.get_ancestors(254637007);

        // Should have ancestors
        assert!(ancestors.contains(&363358000) || ancestors.contains(&63250001));
    }
}
