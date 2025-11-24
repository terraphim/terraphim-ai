use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use terraphim_rolegraph::RoleGraph;
use terraphim_types::{Document, IndexedDocument};

/// Knowledge graph search integration
pub struct KGSearchService {
    role_graphs: HashMap<String, RoleGraph>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KGTerm {
    pub term: String,
    pub normalized_term: String,
    pub id: u64,
    pub definition: Option<String>,
    pub synonyms: Vec<String>,
    pub related_terms: Vec<String>,
    pub url: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KGSearchResult {
    pub term: KGTerm,
    pub documents: Vec<Document>,
    pub related_terms: Vec<KGTerm>,
}

impl KGSearchService {
    pub fn new() -> Self {
        Self {
            role_graphs: HashMap::new(),
        }
    }

    /// Load role graph for a specific role
    pub fn load_role_graph(&mut self, role_name: &str, graph: RoleGraph) {
        log::info!("Loaded role graph for role: {}", role_name);
        self.role_graphs.insert(role_name.to_string(), graph);
    }

    /// Search for documents related to a KG term
    pub fn search_kg_term_ids(&self, role_name: &str, term: &str) -> Result<Vec<String>> {
        let graph = self
            .role_graphs
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role graph not found for role: {}", role_name))?;

        log::info!("Searching KG term '{}' in role '{}'", term, role_name);

        let document_ids = graph.find_document_ids_for_term(term);

        log::info!("Found {} documents for term '{}'", document_ids.len(), term);

        Ok(document_ids)
    }

    /// Get document from graph
    pub fn get_document(
        &self,
        role_name: &str,
        document_id: &str,
    ) -> Result<Option<IndexedDocument>> {
        let graph = self
            .role_graphs
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role graph not found for role: {}", role_name))?;

        Ok(graph.get_document(document_id).cloned())
    }

    /// Get KG term details from thesaurus
    pub fn get_kg_term_from_thesaurus(&self, role_name: &str, term: &str) -> Result<Option<KGTerm>> {
        let graph = self
            .role_graphs
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role graph not found"))?;

        // Search for term in thesaurus
        let thesaurus = &graph.thesaurus;

        // Try to find the term in the thesaurus
        for (normalized_value, normalized_term) in thesaurus {
            if normalized_value.as_str().eq_ignore_ascii_case(term) {
                return Ok(Some(KGTerm {
                    term: normalized_value.as_str().to_string(),
                    normalized_term: normalized_value.as_str().to_string(),
                    id: normalized_term.id,
                    definition: None,
                    synonyms: vec![],
                    related_terms: vec![],
                    url: normalized_term.url.clone().unwrap_or_default(),
                    metadata: HashMap::new(),
                }));
            }
        }

        Ok(None)
    }

    /// Check if all terms are connected by a single path
    pub fn are_terms_connected(&self, role_name: &str, terms: &[String]) -> Result<bool> {
        let graph = self
            .role_graphs
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role graph not found"))?;

        if terms.len() < 2 {
            return Ok(true);
        }

        // Build query string from terms
        let query = terms.join(" ");
        let connected = graph.is_all_terms_connected_by_path(&query);

        log::info!(
            "Terms {} are {} connected",
            terms.join(", "),
            if connected { "" } else { "NOT" }
        );

        Ok(connected)
    }

    /// Get all KG terms for a role
    pub fn list_kg_terms(&self, role_name: &str) -> Result<Vec<String>> {
        let graph = self
            .role_graphs
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role graph not found"))?;

        let terms: Vec<String> = (&graph.thesaurus)
            .into_iter()
            .map(|(k, _)| k.as_str().to_string())
            .collect();

        Ok(terms)
    }

    /// Get all documents from a role graph
    pub fn list_documents(&self, role_name: &str) -> Result<Vec<(&String, &IndexedDocument)>> {
        let graph = self
            .role_graphs
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role graph not found"))?;

        Ok(graph.get_all_documents().collect())
    }

    /// Get graph statistics
    pub fn get_stats(&self, role_name: &str) -> Result<terraphim_rolegraph::GraphStats> {
        let graph = self
            .role_graphs
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role graph not found"))?;

        Ok(graph.get_graph_stats())
    }

    /// Check if a role graph is loaded
    pub fn has_role(&self, role_name: &str) -> bool {
        self.role_graphs.contains_key(role_name)
    }

    /// Get list of loaded roles
    pub fn list_roles(&self) -> Vec<String> {
        self.role_graphs.keys().cloned().collect()
    }
}

impl Default for KGSearchService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kg_search_service_creation() {
        let service = KGSearchService::new();
        assert!(service.list_kg_terms("test_role").is_err());
        assert_eq!(service.list_roles().len(), 0);
    }

    #[test]
    fn test_has_role() {
        let service = KGSearchService::new();
        assert!(!service.has_role("engineer"));
        assert!(!service.has_role("nonexistent"));
    }

    #[test]
    fn test_list_roles_empty() {
        let service = KGSearchService::new();
        let roles = service.list_roles();
        assert_eq!(roles.len(), 0);
    }

    #[test]
    fn test_kg_term_structure() {
        let term = KGTerm {
            term: "rust".to_string(),
            normalized_term: "rust".to_string(),
            id: 1,
            definition: Some("A programming language".to_string()),
            synonyms: vec!["rust-lang".to_string()],
            related_terms: vec!["tokio".to_string()],
            url: "https://rust-lang.org".to_string(),
            metadata: HashMap::new(),
        };

        assert_eq!(term.term, "rust");
        assert_eq!(term.id, 1);
        assert!(term.definition.is_some());
        assert_eq!(term.synonyms.len(), 1);
    }

    #[test]
    fn test_kg_search_result_structure() {
        let term = KGTerm {
            term: "rust".to_string(),
            normalized_term: "rust".to_string(),
            id: 1,
            definition: None,
            synonyms: vec![],
            related_terms: vec![],
            url: "https://rust-lang.org".to_string(),
            metadata: HashMap::new(),
        };

        let result = KGSearchResult {
            term,
            documents: vec![],
            related_terms: vec![],
        };

        assert_eq!(result.term.term, "rust");
        assert_eq!(result.documents.len(), 0);
        assert_eq!(result.related_terms.len(), 0);
    }

    #[test]
    fn test_are_terms_connected_empty() {
        let service = KGSearchService::new();
        // Should error because no role graph loaded
        assert!(service.are_terms_connected("test", &[]).is_err());
    }

    #[test]
    fn test_search_kg_term_ids_no_role() {
        let service = KGSearchService::new();
        let result = service.search_kg_term_ids("nonexistent", "rust");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Role graph not found"));
    }

    #[test]
    fn test_get_document_no_role() {
        let service = KGSearchService::new();
        let result = service.get_document("nonexistent", "doc1");
        assert!(result.is_err());
    }

    #[test]
    fn test_default_implementation() {
        let service = KGSearchService::default();
        assert_eq!(service.list_roles().len(), 0);
    }
}
