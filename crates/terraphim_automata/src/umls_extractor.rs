//! UMLS entity extraction using Aho-Corasick automaton

use aho_corasick::AhoCorasick;
use std::collections::HashMap;

use crate::umls::{UmlsConcept, UmlsDataset};

/// Matched UMLS entity
#[derive(Debug, Clone)]
pub struct UmlsMatch {
    /// Concept Unique Identifier (e.g., "C0004238")
    pub cui: String,
    /// Matched text term from input
    pub matched_term: String,
    /// Canonical/preferred term for this concept
    pub canonical_term: String,
    /// Character span in original text (start, end)
    pub span: (usize, usize),
    /// Match confidence (0.0 to 1.0)
    pub confidence: f32,
}

/// UMLS entity extractor using Aho-Corasick automaton
///
/// Builds an Aho-Corasick automaton from all UMLS terms for
/// O(n + m + z) extraction where n = text length, m = total pattern length,
/// z = number of matches.
pub struct UmlsExtractor {
    /// Concept lookup by CUI
    concept_index: HashMap<String, UmlsConcept>,
    /// Aho-Corasick automaton for multi-pattern matching
    automaton: AhoCorasick,
    /// Pattern index -> CUI mapping
    pattern_cuis: Vec<String>,
    /// Pattern index -> term mapping
    pattern_terms: Vec<String>,
    /// Total patterns in automaton
    pattern_count: usize,
}

impl UmlsExtractor {
    /// Build a UMLS extractor from a loaded dataset
    ///
    /// # Arguments
    /// * `dataset` - Loaded UMLS dataset with terms and CUIs
    ///
    /// # Returns
    /// * `Ok(UmlsExtractor)` on success
    /// * `Err` if automaton construction fails
    pub fn from_dataset(dataset: &UmlsDataset) -> anyhow::Result<Self> {
        let start = std::time::Instant::now();

        let mut concept_index: HashMap<String, UmlsConcept> = HashMap::new();
        let mut pattern_terms: Vec<String> = Vec::new();
        let mut pattern_cuis: Vec<String> = Vec::new();

        // Collect all terms from the dataset
        for (cui, concept) in &dataset.concepts {
            for term in &concept.terms {
                let term_lower = term.to_lowercase();
                if !term_lower.is_empty() {
                    pattern_terms.push(term_lower);
                    pattern_cuis.push(cui.clone());
                }
            }

            // Store concept for lookup
            concept_index.insert(cui.clone(), concept.clone());
        }

        let pattern_count = pattern_terms.len();
        log::info!(
            "Building Aho-Corasick automaton with {} patterns...",
            pattern_count
        );

        // Build Aho-Corasick automaton
        // Using case-insensitive matching via lowercase patterns
        let automaton = AhoCorasick::new(&pattern_terms)?;

        let build_time = start.elapsed();
        log::info!("Automaton built in {}ms", build_time.as_millis());

        Ok(Self {
            concept_index,
            automaton,
            pattern_cuis,
            pattern_terms,
            pattern_count,
        })
    }

    /// Extract UMLS entities from text
    ///
    /// # Arguments
    /// * `text` - Input text to analyze
    ///
    /// # Returns
    /// * `Vec<UmlsMatch>` - All matched entities sorted by position
    pub fn extract(&self, text: &str) -> Vec<UmlsMatch> {
        let text_lower = text.to_lowercase();
        let mut matches: Vec<UmlsMatch> = Vec::new();

        // Use Aho-Corasick automaton for O(n + m + z) matching
        for mat in self.automaton.find_iter(&text_lower) {
            let pattern_index = mat.pattern().as_usize();
            let cui = &self.pattern_cuis[pattern_index];
            let term = &self.pattern_terms[pattern_index];

            // Get concept details
            let (canonical, confidence) = if let Some(concept) = self.concept_index.get(cui) {
                // Higher confidence for exact matches of preferred term
                let conf = if concept.preferred_term.to_lowercase() == *term {
                    1.0
                } else {
                    0.9 // Slightly lower for synonyms
                };
                (concept.preferred_term.clone(), conf)
            } else {
                (term.clone(), 0.8)
            };

            let start = mat.start();
            let end = mat.end();

            // Check for overlapping matches - prefer longer matches
            let overlaps = matches.iter().any(|m| start < m.span.1 && end > m.span.0);

            if !overlaps {
                // Extract original case text from input
                let matched_original = &text[start..end];

                matches.push(UmlsMatch {
                    cui: cui.clone(),
                    matched_term: matched_original.to_string(),
                    canonical_term: canonical,
                    span: (start, end),
                    confidence,
                });
            }
        }

        // Sort by position in text
        matches.sort_by_key(|m| m.span.0);

        matches
    }

    /// Extract entities with filtering by CUI prefix or other criteria
    pub fn extract_filtered<F>(&self, text: &str, filter: F) -> Vec<UmlsMatch>
    where
        F: Fn(&UmlsMatch) -> bool,
    {
        self.extract(text).into_iter().filter(filter).collect()
    }

    /// Get a concept by CUI
    pub fn get_concept(&self, cui: &str) -> Option<&UmlsConcept> {
        self.concept_index.get(cui)
    }

    /// Get total number of patterns in the automaton
    pub fn pattern_count(&self) -> usize {
        self.pattern_count
    }

    /// Get number of unique concepts
    pub fn concept_count(&self) -> usize {
        self.concept_index.len()
    }
}

/// Statistics about the UMLS extractor
#[derive(Debug, Clone)]
pub struct UmlsExtractorStats {
    pub concept_count: usize,
    pub pattern_count: usize,
    pub memory_estimate_mb: f64,
}

impl UmlsExtractorStats {
    /// Estimate statistics from the extractor
    pub fn from_extractor(extractor: &UmlsExtractor) -> Self {
        let concept_count = extractor.concept_count();
        let pattern_count = extractor.pattern_count();

        // Rough memory estimate:
        // ~50 bytes per pattern + string data
        // ~100 bytes per concept
        let memory_estimate_mb = (pattern_count as f64 * 0.00005) + (concept_count as f64 * 0.0001);

        Self {
            concept_count,
            pattern_count,
            memory_estimate_mb,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::umls::UmlsDataset;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_dataset() -> UmlsDataset {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "non-small cell lung carcinoma\tC0000001").unwrap();
        writeln!(file, "nsclc\tC0000001").unwrap();
        writeln!(file, "lung cancer\tC0000001").unwrap();
        writeln!(file, "egfr\tC0000002").unwrap();
        writeln!(file, "epidermal growth factor receptor\tC0000002").unwrap();
        writeln!(file, "gefitinib\tC0000003").unwrap();

        UmlsDataset::from_tsv(file.path()).unwrap()
    }

    #[test]
    fn test_extractor_from_dataset() {
        let dataset = create_test_dataset();
        let extractor = UmlsExtractor::from_dataset(&dataset).unwrap();

        assert_eq!(extractor.concept_count(), 3);
        assert_eq!(extractor.pattern_count(), 6);
    }

    #[test]
    fn test_extract_single_entity() {
        let dataset = create_test_dataset();
        let extractor = UmlsExtractor::from_dataset(&dataset).unwrap();

        let results = extractor.extract("Patient has lung cancer");

        assert!(!results.is_empty());
        assert_eq!(results[0].cui, "C0000001");
        assert!(results[0].confidence > 0.0);
    }

    #[test]
    fn test_extract_multiple_entities() {
        let dataset = create_test_dataset();
        let extractor = UmlsExtractor::from_dataset(&dataset).unwrap();

        let results = extractor.extract("EGFR mutation in NSCLC patient");

        assert!(results.len() >= 2);
        // Check that we got both EGFR and NSCLC
        let cuis: Vec<&str> = results.iter().map(|r| r.cui.as_str()).collect();
        assert!(cuis.contains(&"C0000001")); // NSCLC
        assert!(cuis.contains(&"C0000002")); // EGFR
    }

    #[test]
    fn test_case_insensitive_matching() {
        let dataset = create_test_dataset();
        let extractor = UmlsExtractor::from_dataset(&dataset).unwrap();

        let results_lower = extractor.extract("patient has lung cancer");
        let results_upper = extractor.extract("Patient has LUNG CANCER");
        let results_mixed = extractor.extract("Patient has Lung Cancer");

        assert!(!results_lower.is_empty());
        assert!(!results_upper.is_empty());
        assert!(!results_mixed.is_empty());
    }

    #[test]
    fn test_confidence_scoring() {
        let dataset = create_test_dataset();
        let extractor = UmlsExtractor::from_dataset(&dataset).unwrap();

        // Preferred term (shortest) should have higher confidence
        // "nsclc" is the shortest term, so it becomes preferred
        let results_preferred = extractor.extract("nsclc");
        let results_synonym = extractor.extract("lung cancer");

        assert!(!results_preferred.is_empty());
        assert!(!results_synonym.is_empty());
        assert_eq!(results_preferred[0].confidence, 1.0);
        assert_eq!(results_synonym[0].confidence, 0.9);
    }

    #[test]
    fn test_get_concept() {
        let dataset = create_test_dataset();
        let extractor = UmlsExtractor::from_dataset(&dataset).unwrap();

        let concept = extractor.get_concept("C0000001").unwrap();
        assert_eq!(concept.cui, "C0000001");
        assert_eq!(concept.terms.len(), 3);
    }

    #[test]
    fn test_extract_no_match() {
        let dataset = create_test_dataset();
        let extractor = UmlsExtractor::from_dataset(&dataset).unwrap();

        let results = extractor.extract("Patient is feeling well today");
        assert!(results.is_empty());
    }
}
