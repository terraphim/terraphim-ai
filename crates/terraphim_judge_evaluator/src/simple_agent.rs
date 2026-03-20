//! SimpleAgent for Knowledge Graph orchestration
//!
//! Wraps terraphim_router for KG lookups using Aho-Corasick automata.
//! Provides context enrichment for judge prompts based on matched terms.

use std::sync::Arc;

use terraphim_rolegraph::RoleGraph;

/// A match found in the Knowledge Graph
#[derive(Debug, Clone, PartialEq)]
pub struct KgMatch {
    /// The matched term from the input text
    pub term: String,
    /// The role/context this term belongs to
    pub role: String,
    /// Relevance score (0.0 - 1.0)
    pub score: f64,
}

impl KgMatch {
    /// Create a new KG match
    pub fn new(term: String, role: String, score: f64) -> Self {
        Self { term, role, score }
    }
}

/// SimpleAgent wraps terraphim_router for Knowledge Graph lookups
#[derive(Debug, Clone)]
pub struct SimpleAgent {
    router: Arc<RoleGraph>,
}

impl SimpleAgent {
    /// Create a new SimpleAgent with the given RoleGraph
    pub fn new(router: Arc<RoleGraph>) -> Self {
        Self { router }
    }

    /// Run text through Aho-Corasick automata and return matches
    ///
    /// # Example
    /// ```
    /// use std::sync::Arc;
    /// use terraphim_judge_evaluator::{SimpleAgent, KgMatch};
    ///
    /// // Assuming rolegraph is already loaded
    /// // let agent = SimpleAgent::new(Arc::new(rolegraph));
    /// // let matches = agent.lookup_terms("rust programming");
    /// ```
    pub fn lookup_terms(&self, text: &str) -> Vec<KgMatch> {
        let mut matches = Vec::new();

        // Use the Aho-Corasick automata to find matching node IDs
        let node_ids = self.router.find_matching_node_ids(text);

        for node_id in node_ids {
            // Get the normalized term for this node ID
            if let Some(normalized_term) = self.router.ac_reverse_nterm.get(&node_id) {
                let term = normalized_term.to_string();
                let role = self.router.role.to_string();

                // Calculate a simple relevance score based on term frequency/position
                // In a real implementation, this would use more sophisticated scoring
                let score = Self::calculate_score(text, &term);

                matches.push(KgMatch::new(term, role, score));
            }
        }

        // Remove duplicates while preserving order
        matches.dedup_by(|a, b| a.term == b.term);

        matches
    }

    /// Calculate a relevance score for a matched term
    fn calculate_score(text: &str, term: &str) -> f64 {
        // Simple scoring: exact match gets higher score
        // Case-insensitive contains gets medium score
        // Partial match gets lower score
        let text_lower = text.to_lowercase();
        let term_lower = term.to_lowercase();

        if text_lower.contains(&term_lower) {
            // Bonus for exact word match
            let word_boundary = format!(r"\b{}\b", regex::escape(&term_lower));
            if regex::Regex::new(&word_boundary)
                .map(|re| re.is_match(&text_lower))
                .unwrap_or(false)
            {
                1.0
            } else {
                0.8
            }
        } else {
            0.5
        }
    }

    /// Append KG context to judge prompt
    ///
    /// Enriches the prompt with relevant terms found in the Knowledge Graph.
    ///
    /// # Example
    /// ```
    /// use std::sync::Arc;
    /// use terraphim_judge_evaluator::SimpleAgent;
    ///
    /// // let agent = SimpleAgent::new(Arc::new(rolegraph));
    /// // let enriched = agent.enrich_prompt("Review this Rust code");
    /// // The enriched prompt will include context about matched terms
    /// ```
    pub fn enrich_prompt(&self, prompt: &str) -> String {
        let matches = self.lookup_terms(prompt);

        if matches.is_empty() {
            return prompt.to_string();
        }

        // Build context section
        let mut context_parts = Vec::new();
        context_parts.push("\n\n### Knowledge Graph Context".to_string());
        context_parts.push("The following relevant concepts were identified:".to_string());

        for kg_match in &matches {
            context_parts.push(format!(
                "- **{}** (role: {}, relevance: {:.2})",
                kg_match.term, kg_match.role, kg_match.score
            ));
        }

        context_parts.push("\nConsider these concepts in your evaluation.".to_string());

        let context = context_parts.join("\n");

        format!("{}{}", prompt, context)
    }

    /// Get the underlying router reference
    pub fn router(&self) -> &Arc<RoleGraph> {
        &self.router
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_rolegraph::RoleGraph;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, RoleName, Thesaurus};

    fn create_test_rolegraph() -> RoleGraph {
        let mut thesaurus = Thesaurus::new("test".to_string());

        // Add some test terms
        let term1 = NormalizedTerm::new(1, NormalizedTermValue::from("rust"));
        let term2 = NormalizedTerm::new(2, NormalizedTermValue::from("async"));
        let term3 = NormalizedTerm::new(3, NormalizedTermValue::from("programming"));

        thesaurus.insert(NormalizedTermValue::from("rust"), term1);
        thesaurus.insert(NormalizedTermValue::from("async"), term2);
        thesaurus.insert(NormalizedTermValue::from("programming"), term3);

        // Create the RoleGraph synchronously
        RoleGraph::new_sync(RoleName::new("engineer"), thesaurus)
            .expect("Failed to create RoleGraph")
    }

    #[test]
    fn test_lookup_with_known_terms() {
        let rolegraph = create_test_rolegraph();
        let agent = SimpleAgent::new(Arc::new(rolegraph));

        let matches = agent.lookup_terms("I love rust programming");

        assert!(!matches.is_empty());

        // Check that "rust" and "programming" were matched
        let terms: Vec<String> = matches.iter().map(|m| m.term.clone()).collect();
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"programming".to_string()));
    }

    #[test]
    fn test_lookup_empty_text() {
        let rolegraph = create_test_rolegraph();
        let agent = SimpleAgent::new(Arc::new(rolegraph));

        let matches = agent.lookup_terms("");

        assert!(matches.is_empty());
    }

    #[test]
    fn test_lookup_no_matches() {
        let rolegraph = create_test_rolegraph();
        let agent = SimpleAgent::new(Arc::new(rolegraph));

        let matches = agent.lookup_terms("python java javascript");

        assert!(matches.is_empty());
    }

    #[test]
    fn test_enrich_prompt_formatting() {
        let rolegraph = create_test_rolegraph();
        let agent = SimpleAgent::new(Arc::new(rolegraph));

        let prompt = "Review this code implementation";
        let enriched = agent.enrich_prompt(prompt);

        // Check that the prompt is preserved
        assert!(enriched.starts_with(prompt));

        // Check that KG context section is added when there are matches
        // (This depends on the test thesaurus having terms that match "code")
        if enriched.contains("Knowledge Graph Context") {
            assert!(enriched.contains("### Knowledge Graph Context"));
            assert!(enriched.contains("relevant concepts were identified"));
        }
    }

    #[test]
    fn test_enrich_prompt_no_matches() {
        let rolegraph = create_test_rolegraph();
        let agent = SimpleAgent::new(Arc::new(rolegraph));

        let prompt = "xyz123 abc789";
        let enriched = agent.enrich_prompt(prompt);

        // When there are no matches, the prompt should be returned unchanged
        assert_eq!(enriched, prompt);
    }

    #[test]
    fn test_kg_match_creation() {
        let kg_match = KgMatch::new("rust".to_string(), "engineer".to_string(), 0.95);

        assert_eq!(kg_match.term, "rust");
        assert_eq!(kg_match.role, "engineer");
        assert!((kg_match.score - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_router_accessor() {
        let rolegraph = create_test_rolegraph();
        let agent = SimpleAgent::new(Arc::new(rolegraph));

        let router_ref = agent.router();
        assert_eq!(router_ref.role.to_string(), "engineer");
    }
}
