//! Knowledge graph integration for smart routing
//!
//! This module integrates with Terraphim's knowledge graph to enable
//! context-aware routing based on role graphs and concept relationships.

use std::collections::HashMap;

use terraphim_types::{
    Concept, NormalizedTermValue, RoleName, Thesaurus,
};

/// Knowledge graph aware router
#[derive(Debug, Clone)]
pub struct KnowledgeGraphRouter {
    /// Role-specific thesauri for concept expansion
    thesauri: HashMap<RoleName, Thesaurus>,
    /// Default role if none specified
    default_role: Option<RoleName>,
}

impl KnowledgeGraphRouter {
    /// Create a new knowledge graph router
    pub fn new() -> Self {
        Self {
            thesauri: HashMap::new(),
            default_role: None,
        }
    }

    /// Set default role
    pub fn with_default_role(mut self, role: RoleName) -> Self {
        self.default_role = Some(role);
        self
    }

    /// Add a thesaurus for a role
    pub fn add_thesaurus(
        &mut self,
        role: RoleName,
        thesaurus: Thesaurus,
    ) {
        self.thesauri.insert(role, thesaurus);
    }

    /// Expand search terms using knowledge graph
    pub fn expand_terms(
        &self,
        terms: &[NormalizedTermValue],
        role: Option<&RoleName>,
    ) -> Vec<NormalizedTermValue> {
        let role = match role.or(self.default_role.as_ref()) {
            Some(r) => r,
            None => return terms.to_vec(),
        };
        let thesaurus = match self.thesauri.get(role) {
            Some(t) => t,
            None => return terms.to_vec(),
        };

        let mut expanded = terms.to_vec();

        for term in terms {
            // Look up related terms in thesaurus
            if let Some(normalized) = thesaurus.get(term) {
                // Add the normalized value as an expanded term
                let val = normalized.value.clone();
                if !expanded.contains(&val) {
                    expanded.push(val);
                }
            }
        }

        expanded
    }

    /// Score a provider's relevance to a concept
    pub fn score_provider_relevance(
        &self,
        _provider_id: &str,
        _concept: &Concept,
    ) -> f64 {
        // In a real implementation, this would:
        // 1. Look up provider in knowledge graph
        // 2. Check relationships to the concept
        // 3. Return a relevance score (0.0 - 1.0)

        // For now, return a default score
        0.5
    }

    /// Find related concepts for a query
    pub fn find_related_concepts(
        &self,
        query: &str,
        _role: Option<&RoleName>,
    ) -> Vec<Concept> {
        // In a real implementation, this would:
        // 1. Parse the query
        // 2. Find matching concepts in KG
        // 3. Return related concepts

        // For now, return empty
        vec![]
    }
}

impl Default for KnowledgeGraphRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, Thesaurus};

    #[test]
    fn test_kg_router_creation() {
        let router = KnowledgeGraphRouter::new()
            .with_default_role(RoleName::new("engineer"));

        assert_eq!(
            router.default_role,
            Some(RoleName::new("engineer"))
        );
    }

    #[test]
    fn test_term_expansion() {
        let mut router = KnowledgeGraphRouter::new();

        // Create a thesaurus with synonyms
        let mut thesaurus = Thesaurus::new("programming".to_string());
        let term = NormalizedTerm::new(
            1,
            NormalizedTermValue::from("rust"),
        );
        thesaurus.insert(
            NormalizedTermValue::from("rust"),
            term,
        );

        router.add_thesaurus(RoleName::new("engineer"), thesaurus);

        // Expand terms
        let terms = vec![NormalizedTermValue::from("rust")];
        let expanded = router.expand_terms(
            &terms,
            Some(&RoleName::new("engineer")),
        );

        assert!(!expanded.is_empty());
        assert!(expanded.contains(&NormalizedTermValue::from("rust")));
    }
}
