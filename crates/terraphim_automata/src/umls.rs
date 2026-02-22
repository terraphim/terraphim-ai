//! UMLS (Unified Medical Language System) data structures and loader
//!
//! Handles loading of UMLS Concept Unique Identifiers (CUIs) and their associated terms
//! from TSV format for fast entity extraction using Aho-Corasick automaton.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// UMLS Concept with CUI and associated terms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmlsConcept {
    /// Concept Unique Identifier (e.g., "C0004238")
    pub cui: String,
    /// All terms associated with this concept
    pub terms: Vec<String>,
    /// Preferred term (first term encountered or shortest term)
    pub preferred_term: String,
}

impl UmlsConcept {
    /// Create a new UMLS concept with initial term
    pub fn new(cui: String, term: String) -> Self {
        Self {
            preferred_term: term.clone(),
            terms: vec![term],
            cui,
        }
    }

    /// Add a term to this concept
    pub fn add_term(&mut self, term: String) {
        if !self.terms.contains(&term) {
            // Update preferred term if this one is shorter
            if term.len() < self.preferred_term.len() {
                self.preferred_term = term.clone();
            }
            self.terms.push(term);
        }
    }
}

/// UMLS dataset containing all concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmlsDataset {
    /// Concepts indexed by CUI
    pub concepts: HashMap<String, UmlsConcept>,
    /// Total number of terms (including duplicates across concepts)
    pub term_count: usize,
}

impl UmlsDataset {
    /// Create an empty UMLS dataset
    pub fn new() -> Self {
        Self {
            concepts: HashMap::new(),
            term_count: 0,
        }
    }

    /// Load UMLS data from TSV file
    ///
    /// Format: term<TAB>cui (one mapping per line)
    ///
    /// # Arguments
    /// * `path` - Path to the TSV file
    ///
    /// # Returns
    /// * `Ok(UmlsDataset)` on success
    /// * `Err` if file cannot be read or parsed
    pub fn from_tsv<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut dataset = Self::new();

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse TSV: term<TAB>cui
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let term = parts[0].trim().to_string();
                let cui = parts[1].trim().to_string();

                if !term.is_empty() && !cui.is_empty() {
                    dataset.add_term(term, cui);
                }
            }
        }

        Ok(dataset)
    }

    /// Add a term-CUI mapping to the dataset
    pub fn add_term(&mut self, term: String, cui: String) {
        self.term_count += 1;

        if let Some(concept) = self.concepts.get_mut(&cui) {
            concept.add_term(term);
        } else {
            let concept = UmlsConcept::new(cui.clone(), term);
            self.concepts.insert(cui, concept);
        }
    }

    /// Get total number of unique concepts
    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    /// Get all terms as a flat list for automaton building
    pub fn get_all_terms(&self) -> Vec<(String, String)> {
        let mut terms = Vec::with_capacity(self.term_count);
        for (cui, concept) in &self.concepts {
            for term in &concept.terms {
                terms.push((term.clone(), cui.clone()));
            }
        }
        terms
    }

    /// Get concept by CUI
    pub fn get_concept(&self, cui: &str) -> Option<&UmlsConcept> {
        self.concepts.get(cui)
    }
}

impl Default for UmlsDataset {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about loaded UMLS data
#[derive(Debug, Clone, Serialize)]
pub struct UmlsStats {
    pub total_concepts: usize,
    pub total_term_mappings: usize,
    pub avg_terms_per_concept: f64,
    pub loading_time_ms: u64,
}

impl UmlsStats {
    /// Calculate statistics from a loaded dataset
    pub fn from_dataset(dataset: &UmlsDataset, loading_time_ms: u64) -> Self {
        let total_concepts = dataset.concept_count();
        let total_term_mappings = dataset.term_count;
        let avg_terms_per_concept = if total_concepts > 0 {
            total_term_mappings as f64 / total_concepts as f64
        } else {
            0.0
        };

        Self {
            total_concepts,
            total_term_mappings,
            avg_terms_per_concept,
            loading_time_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_tsv() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "non-small cell lung carcinoma\tC0000001").unwrap();
        writeln!(file, "nsclc\tC0000001").unwrap();
        writeln!(file, "lung cancer\tC0000001").unwrap();
        writeln!(file, "egfr\tC0000002").unwrap();
        writeln!(file, "epidermal growth factor receptor\tC0000002").unwrap();
        writeln!(file, "gefitinib\tC0000003").unwrap();
        file
    }

    #[test]
    fn test_load_from_tsv() {
        let file = create_test_tsv();
        let dataset = UmlsDataset::from_tsv(file.path()).unwrap();

        assert_eq!(dataset.concept_count(), 3);
        assert_eq!(dataset.term_count, 6);
    }

    #[test]
    fn test_concept_terms() {
        let file = create_test_tsv();
        let dataset = UmlsDataset::from_tsv(file.path()).unwrap();

        let concept = dataset.get_concept("C0000001").unwrap();
        assert_eq!(concept.terms.len(), 3);
        assert!(
            concept
                .terms
                .contains(&"non-small cell lung carcinoma".to_string())
        );
        assert!(concept.terms.contains(&"nsclc".to_string()));
    }

    #[test]
    fn test_get_all_terms() {
        let file = create_test_tsv();
        let dataset = UmlsDataset::from_tsv(file.path()).unwrap();
        let terms = dataset.get_all_terms();

        assert_eq!(terms.len(), 6);
    }

    #[test]
    fn test_umls_concept_add_term() {
        let mut concept = UmlsConcept::new("C0000001".to_string(), "lung cancer".to_string());
        concept.add_term("lung carcinoma".to_string());
        concept.add_term("lung cancer".to_string()); // Duplicate, should be ignored

        assert_eq!(concept.terms.len(), 2);
        assert_eq!(concept.preferred_term, "lung cancer");
    }

    #[test]
    fn test_stats_calculation() {
        let file = create_test_tsv();
        let dataset = UmlsDataset::from_tsv(file.path()).unwrap();
        let stats = UmlsStats::from_dataset(&dataset, 1500);

        assert_eq!(stats.total_concepts, 3);
        assert_eq!(stats.total_term_mappings, 6);
        assert_eq!(stats.avg_terms_per_concept, 2.0);
        assert_eq!(stats.loading_time_ms, 1500);
    }
}
