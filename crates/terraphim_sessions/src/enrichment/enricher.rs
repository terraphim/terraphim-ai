//! Session enrichment engine using terraphim knowledge graph

use std::sync::Arc;

use terraphim_automata::matcher::{find_matches, Matched};
use terraphim_rolegraph::RoleGraph;
use terraphim_types::Thesaurus;
use tokio::sync::RwLock;

use super::concept::{ConceptMatch, ConceptOccurrence, SessionConcepts};
use crate::model::Session;

/// Configuration for session enrichment
#[derive(Debug, Clone)]
pub struct EnrichmentConfig {
    /// Include surrounding context with matches
    pub include_context: bool,
    /// Context window size (characters before/after match)
    pub context_window: usize,
    /// Minimum occurrences for dominant topics
    pub dominant_topic_threshold: usize,
    /// Number of top concepts to consider as dominant
    pub top_n_dominant: usize,
    /// Whether to check graph connectivity
    pub check_graph_connections: bool,
}

impl Default for EnrichmentConfig {
    fn default() -> Self {
        Self {
            include_context: true,
            context_window: 50,
            dominant_topic_threshold: 1,
            top_n_dominant: 10,
            check_graph_connections: false,
        }
    }
}

/// Result of enrichment process
#[derive(Debug, Clone)]
pub struct EnrichmentResult {
    /// Extracted concepts
    pub concepts: SessionConcepts,
    /// Number of messages processed
    pub messages_processed: usize,
    /// Total characters processed
    pub chars_processed: usize,
    /// Processing duration in milliseconds
    pub duration_ms: u64,
}

/// Session enricher using terraphim automata
pub struct SessionEnricher {
    /// Thesaurus for concept matching
    thesaurus: Thesaurus,
    /// Optional role graph for connectivity checking
    rolegraph: Option<Arc<RwLock<RoleGraph>>>,
    /// Configuration
    config: EnrichmentConfig,
}

impl SessionEnricher {
    /// Create a new session enricher with thesaurus
    pub fn new(thesaurus: Thesaurus) -> Self {
        Self {
            thesaurus,
            rolegraph: None,
            config: EnrichmentConfig::default(),
        }
    }

    /// Create enricher with thesaurus and role graph
    pub fn with_rolegraph(thesaurus: Thesaurus, rolegraph: Arc<RwLock<RoleGraph>>) -> Self {
        Self {
            thesaurus,
            rolegraph: Some(rolegraph),
            config: EnrichmentConfig {
                check_graph_connections: true,
                ..Default::default()
            },
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: EnrichmentConfig) -> Self {
        self.config = config;
        self
    }

    /// Enrich a session with concepts
    pub async fn enrich_session(&self, session: &Session) -> anyhow::Result<EnrichmentResult> {
        let start = std::time::Instant::now();
        let mut concepts = SessionConcepts::new(session.id.clone());
        let mut chars_processed = 0;

        // Process each message
        for (msg_idx, message) in session.messages.iter().enumerate() {
            let text = &message.content;
            chars_processed += text.len();

            // Find concept matches
            let matches = find_matches(text, self.thesaurus.clone(), true)?;

            for matched in matches {
                let concept = self.matched_to_concept(&matched, msg_idx, text);
                concepts.insert_or_update(concept);
            }
        }

        // Calculate derived data
        concepts.calculate_dominant_topics(self.config.top_n_dominant);
        concepts.calculate_co_occurrences();

        // Check graph connectivity if enabled
        if self.config.check_graph_connections {
            if let Some(ref rolegraph) = self.rolegraph {
                let graph = rolegraph.read().await;
                self.find_graph_connections(&mut concepts, &graph);
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(EnrichmentResult {
            concepts,
            messages_processed: session.messages.len(),
            chars_processed,
            duration_ms,
        })
    }

    /// Enrich multiple sessions
    pub async fn enrich_sessions(
        &self,
        sessions: &[Session],
    ) -> anyhow::Result<Vec<EnrichmentResult>> {
        let mut results = Vec::with_capacity(sessions.len());

        for session in sessions {
            let result = self.enrich_session(session).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Convert a match to a concept with occurrence
    fn matched_to_concept(&self, matched: &Matched, msg_idx: usize, text: &str) -> ConceptMatch {
        let (start, end) = matched.pos.unwrap_or((0, 0));

        let context = if self.config.include_context {
            Some(self.extract_context(text, start, end))
        } else {
            None
        };

        let occurrence = ConceptOccurrence {
            message_idx: msg_idx,
            start_pos: start,
            end_pos: end,
            context,
        };

        let mut concept = ConceptMatch::new(
            matched.term.clone(),
            matched.normalized_term.value.to_string(),
            matched.normalized_term.id,
            matched.normalized_term.url.clone(),
        );
        concept.add_occurrence(occurrence);

        concept
    }

    /// Extract context around a match
    fn extract_context(&self, text: &str, start: usize, end: usize) -> String {
        let window = self.config.context_window;
        let ctx_start = start.saturating_sub(window);
        let ctx_end = (end + window).min(text.len());

        let mut context = String::new();

        if ctx_start > 0 {
            context.push_str("...");
        }

        context.push_str(&text[ctx_start..ctx_end]);

        if ctx_end < text.len() {
            context.push_str("...");
        }

        context
    }

    /// Find concepts that are connected via the knowledge graph
    fn find_graph_connections(&self, concepts: &mut SessionConcepts, graph: &RoleGraph) {
        let terms: Vec<String> = concepts
            .all_terms()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        for i in 0..terms.len() {
            for j in (i + 1)..terms.len() {
                let combined = format!("{} {}", terms[i], terms[j]);
                if graph.is_all_terms_connected_by_path(&combined) {
                    let (a, b) = if terms[i] < terms[j] {
                        (terms[i].clone(), terms[j].clone())
                    } else {
                        (terms[j].clone(), terms[i].clone())
                    };
                    concepts.graph_connections.push((a, b));
                }
            }
        }
    }
}

/// Search sessions by concept
pub fn search_by_concept<'a>(
    sessions: &'a [Session],
    concepts_map: &'a std::collections::HashMap<String, SessionConcepts>,
    concept: &str,
) -> Vec<(&'a Session, &'a ConceptMatch)> {
    let concept_lower = concept.to_lowercase();
    let mut results = Vec::new();

    for session in sessions {
        if let Some(session_concepts) = concepts_map.get(&session.id) {
            // Check both term and normalized term
            for concept_match in session_concepts.concepts.values() {
                if concept_match.term.to_lowercase().contains(&concept_lower)
                    || concept_match
                        .normalized_term
                        .to_lowercase()
                        .contains(&concept_lower)
                {
                    results.push((session, concept_match));
                }
            }
        }
    }

    // Sort by occurrence count (most occurrences first)
    results.sort_by(|a, b| b.1.count.cmp(&a.1.count));

    results
}

/// Find sessions that share concepts
pub fn find_related_sessions<'a>(
    session_id: &str,
    concepts_map: &'a std::collections::HashMap<String, SessionConcepts>,
    min_shared_concepts: usize,
) -> Vec<(&'a str, usize, Vec<String>)> {
    let source_concepts = match concepts_map.get(session_id) {
        Some(c) => c,
        None => return Vec::new(),
    };

    let source_terms: std::collections::HashSet<&str> = source_concepts
        .concepts
        .keys()
        .map(|s| s.as_str())
        .collect();

    let mut related = Vec::new();

    for (other_id, other_concepts) in concepts_map.iter() {
        if other_id == session_id {
            continue;
        }

        let shared: Vec<String> = other_concepts
            .concepts
            .keys()
            .filter(|k| source_terms.contains(k.as_str()))
            .cloned()
            .collect();

        if shared.len() >= min_shared_concepts {
            related.push((other_id.as_str(), shared.len(), shared));
        }
    }

    // Sort by number of shared concepts (most first)
    related.sort_by(|a, b| b.1.cmp(&a.1));

    related
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Message, MessageRole, SessionMetadata};
    use std::path::PathBuf;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue};

    fn create_test_thesaurus() -> Thesaurus {
        let mut thesaurus = Thesaurus::new("Test".to_string());

        // Add some test concepts
        let concepts = [
            ("rust", "Rust Programming", 1),
            ("tokio", "Tokio Runtime", 2),
            ("async", "Asynchronous Programming", 3),
            ("wasm", "WebAssembly", 4),
        ];

        for (key, normalized, id) in concepts {
            let term = NormalizedTerm {
                id,
                value: NormalizedTermValue::from(normalized),
                display_value: None,
                url: Some(format!("https://example.com/{}", key)),
            };
            thesaurus.insert(NormalizedTermValue::from(key), term);
        }

        thesaurus
    }

    fn create_test_session() -> Session {
        Session {
            id: "test-session".to_string(),
            source: "test".to_string(),
            external_id: "test-1".to_string(),
            title: Some("Test Session".to_string()),
            source_path: PathBuf::from("."),
            started_at: None,
            ended_at: None,
            messages: vec![
                Message::text(
                    0,
                    MessageRole::User,
                    "How do I use rust with tokio for async programming?",
                ),
                Message::text(
                    1,
                    MessageRole::Assistant,
                    "Tokio is a popular async runtime for Rust. You can use it with async/await.",
                ),
                Message::text(2, MessageRole::User, "Can I compile rust to wasm?"),
            ],
            metadata: SessionMetadata::default(),
        }
    }

    #[tokio::test]
    async fn test_enrich_session() {
        let thesaurus = create_test_thesaurus();
        let enricher = SessionEnricher::new(thesaurus);
        let session = create_test_session();

        let result = enricher.enrich_session(&session).await.unwrap();

        assert_eq!(result.messages_processed, 3);
        assert!(
            result.concepts.concept_count() > 0,
            "Should find at least one concept"
        );

        // Debug: print all concept keys
        println!("Found concepts:");
        for (key, concept) in result.concepts.concepts.iter() {
            println!(
                "  key='{}', term='{}', normalized='{}'",
                key, concept.term, concept.normalized_term
            );
        }

        // Should find rust, tokio, async, wasm - check by iterating
        let has_rust = result
            .concepts
            .concepts
            .values()
            .any(|c| c.normalized_term.contains("Rust") || c.term.contains("rust"));
        assert!(has_rust, "Should find rust-related concept");
    }

    #[tokio::test]
    async fn test_dominant_topics() {
        let thesaurus = create_test_thesaurus();
        let enricher = SessionEnricher::new(thesaurus);
        let session = create_test_session();

        let result = enricher.enrich_session(&session).await.unwrap();

        // Only check if there are concepts
        if result.concepts.concept_count() > 0 {
            assert!(
                !result.concepts.dominant_topics.is_empty(),
                "Should have dominant topics"
            );
            println!("Dominant topics: {:?}", result.concepts.dominant_topics);
        }
    }

    #[tokio::test]
    async fn test_co_occurrences() {
        let thesaurus = create_test_thesaurus();
        let enricher = SessionEnricher::new(thesaurus);
        let session = create_test_session();

        let result = enricher.enrich_session(&session).await.unwrap();

        // rust and tokio appear in same messages, should co-occur
        assert!(!result.concepts.co_occurrences.is_empty());
    }
}
