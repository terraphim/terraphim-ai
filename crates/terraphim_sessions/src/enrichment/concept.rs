//! Concept data structures for session enrichment

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::model::SessionId;

/// A single occurrence of a concept in text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptOccurrence {
    /// Message index where concept was found
    pub message_idx: usize,
    /// Start position in text
    pub start_pos: usize,
    /// End position in text
    pub end_pos: usize,
    /// Surrounding context (snippet)
    pub context: Option<String>,
}

/// A matched concept with all its occurrences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptMatch {
    /// The matched term (as found in text)
    pub term: String,
    /// Normalized term from thesaurus
    pub normalized_term: String,
    /// URL associated with concept (if any)
    pub url: Option<String>,
    /// Concept ID from thesaurus
    pub concept_id: u64,
    /// All occurrences in the session
    pub occurrences: Vec<ConceptOccurrence>,
    /// Total occurrence count
    pub count: usize,
    /// Confidence score (based on match quality)
    pub confidence: f64,
}

impl ConceptMatch {
    /// Create a new concept match
    pub fn new(
        term: String,
        normalized_term: String,
        concept_id: u64,
        url: Option<String>,
    ) -> Self {
        Self {
            term,
            normalized_term,
            concept_id,
            url,
            occurrences: Vec::new(),
            count: 0,
            confidence: 1.0,
        }
    }

    /// Add an occurrence
    pub fn add_occurrence(&mut self, occurrence: ConceptOccurrence) {
        self.occurrences.push(occurrence);
        self.count += 1;
    }

    /// Message indices where this concept appears
    pub fn message_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.occurrences.iter().map(|o| o.message_idx).collect();
        indices.sort_unstable();
        indices.dedup();
        indices
    }
}

/// Collection of concepts extracted from a session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionConcepts {
    /// Session ID
    pub session_id: SessionId,
    /// All matched concepts (keyed by normalized term)
    pub concepts: HashMap<String, ConceptMatch>,
    /// Concept pairs that co-occur in messages
    pub co_occurrences: Vec<(String, String)>,
    /// Dominant topics (most frequent concepts)
    pub dominant_topics: Vec<String>,
    /// Concepts that are connected via knowledge graph paths
    pub graph_connections: Vec<(String, String)>,
}

impl SessionConcepts {
    /// Create new session concepts
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            concepts: HashMap::new(),
            co_occurrences: Vec::new(),
            dominant_topics: Vec::new(),
            graph_connections: Vec::new(),
        }
    }

    /// Get concept count
    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    /// Get total occurrence count
    pub fn total_occurrences(&self) -> usize {
        self.concepts.values().map(|c| c.count).sum()
    }

    /// Get concept by normalized term
    pub fn get(&self, normalized_term: &str) -> Option<&ConceptMatch> {
        self.concepts.get(normalized_term)
    }

    /// Insert or update a concept
    pub fn insert_or_update(&mut self, concept: ConceptMatch) {
        let key = concept.normalized_term.clone();
        if let Some(existing) = self.concepts.get_mut(&key) {
            // Merge occurrences
            for occ in concept.occurrences {
                existing.add_occurrence(occ);
            }
        } else {
            self.concepts.insert(key, concept);
        }
    }

    /// Get concepts sorted by frequency
    pub fn by_frequency(&self) -> Vec<&ConceptMatch> {
        let mut sorted: Vec<_> = self.concepts.values().collect();
        sorted.sort_by(|a, b| b.count.cmp(&a.count));
        sorted
    }

    /// Get concepts for a specific message
    pub fn concepts_in_message(&self, message_idx: usize) -> Vec<&ConceptMatch> {
        self.concepts
            .values()
            .filter(|c| c.message_indices().contains(&message_idx))
            .collect()
    }

    /// Calculate dominant topics (top N by frequency)
    pub fn calculate_dominant_topics(&mut self, top_n: usize) {
        self.dominant_topics = self
            .by_frequency()
            .into_iter()
            .take(top_n)
            .map(|c| c.normalized_term.clone())
            .collect();
    }

    /// Find co-occurring concepts (concepts that appear in the same message)
    pub fn calculate_co_occurrences(&mut self) {
        let mut pairs: Vec<(String, String)> = Vec::new();

        // Group concepts by message
        let mut message_concepts: HashMap<usize, Vec<&str>> = HashMap::new();
        for concept in self.concepts.values() {
            for idx in concept.message_indices() {
                message_concepts
                    .entry(idx)
                    .or_default()
                    .push(&concept.normalized_term);
            }
        }

        // Find pairs in each message
        for concepts in message_concepts.values() {
            for i in 0..concepts.len() {
                for j in (i + 1)..concepts.len() {
                    let (a, b) = if concepts[i] < concepts[j] {
                        (concepts[i].to_string(), concepts[j].to_string())
                    } else {
                        (concepts[j].to_string(), concepts[i].to_string())
                    };
                    if !pairs.contains(&(a.clone(), b.clone())) {
                        pairs.push((a, b));
                    }
                }
            }
        }

        self.co_occurrences = pairs;
    }

    /// Get all concept terms
    pub fn all_terms(&self) -> Vec<&str> {
        self.concepts.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concept_match() {
        let mut concept = ConceptMatch::new(
            "rust".to_string(),
            "Rust Programming".to_string(),
            1,
            Some("https://rust-lang.org".to_string()),
        );

        concept.add_occurrence(ConceptOccurrence {
            message_idx: 0,
            start_pos: 10,
            end_pos: 14,
            context: Some("learning rust programming".to_string()),
        });

        concept.add_occurrence(ConceptOccurrence {
            message_idx: 2,
            start_pos: 5,
            end_pos: 9,
            context: Some("more rust".to_string()),
        });

        assert_eq!(concept.count, 2);
        assert_eq!(concept.message_indices(), vec![0, 2]);
    }

    #[test]
    fn test_session_concepts() {
        let mut concepts = SessionConcepts::new("test-session".to_string());

        let mut rust = ConceptMatch::new("rust".to_string(), "Rust".to_string(), 1, None);
        rust.add_occurrence(ConceptOccurrence {
            message_idx: 0,
            start_pos: 0,
            end_pos: 4,
            context: None,
        });
        rust.add_occurrence(ConceptOccurrence {
            message_idx: 1,
            start_pos: 0,
            end_pos: 4,
            context: None,
        });

        let mut tokio = ConceptMatch::new("tokio".to_string(), "Tokio".to_string(), 2, None);
        tokio.add_occurrence(ConceptOccurrence {
            message_idx: 0,
            start_pos: 10,
            end_pos: 15,
            context: None,
        });

        concepts.insert_or_update(rust);
        concepts.insert_or_update(tokio);

        assert_eq!(concepts.concept_count(), 2);
        assert_eq!(concepts.total_occurrences(), 3);

        let by_freq = concepts.by_frequency();
        assert_eq!(by_freq[0].normalized_term, "Rust");
        assert_eq!(by_freq[1].normalized_term, "Tokio");

        concepts.calculate_co_occurrences();
        assert_eq!(concepts.co_occurrences.len(), 1);
        assert!(concepts
            .co_occurrences
            .contains(&("Rust".to_string(), "Tokio".to_string())));
    }
}
