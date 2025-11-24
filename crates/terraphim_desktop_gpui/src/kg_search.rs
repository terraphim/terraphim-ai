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
    pub id: usize,
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
    pub fn get_document(&self, role_name: &str, document_id: &str) -> Result<Option<IndexedDocument>> {
        let graph = self
            .role_graphs
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role graph not found for role: {}", role_name))?;

        Ok(graph.get_document(document_id).cloned())
    }

    /// Get KG term details
    pub fn get_kg_term(&self, role_name: &str, term: &str) -> Result<Option<KGTerm>> {
        let graph = self
            .role_graphs
            .get(role_name)
            .ok_or_else(|| anyhow::anyhow!("Role graph not found"))?;

        // Search for term in graph
        let thesaurus_entry = graph
            .thesaurus
            .iter()
            .find(|t| t.nterm.eq_ignore_ascii_case(term) || t.term.eq_ignore_ascii_case(term));

        if let Some(entry) = thesaurus_entry {
            let kg_term = KGTerm {
                term: entry.term.clone(),
                normalized_term: entry.nterm.clone(),
                id: entry.id,
                definition: entry.definition.clone(),
                synonyms: vec![], // Would need to be extracted from thesaurus
                related_terms: vec![], // Would need graph traversal
                url: entry.url.clone(),
                metadata: HashMap::new(),
            };

            Ok(Some(kg_term))
        } else {
            Ok(None)
        }
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

        let terms: Vec<String> = graph
            .thesaurus
            .iter()
            .map(|t| t.nterm.clone())
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
}

impl Default for KGSearchService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::Thesaurus;

    fn create_test_thesaurus() -> Thesaurus {
        vec![
            IndexedDocument {
                id: 1,
                nterm: "rust".to_string(),
                term: "Rust".to_string(),
                url: "https://rust-lang.org".to_string(),
                definition: Some("A systems programming language".to_string()),
            },
            IndexedDocument {
                id: 2,
                nterm: "tokio".to_string(),
                term: "Tokio".to_string(),
                url: "https://tokio.rs".to_string(),
                definition: Some("An async runtime for Rust".to_string()),
            },
            IndexedDocument {
                id: 3,
                nterm: "async".to_string(),
                term: "Async".to_string(),
                url: "https://rust-lang.org/async".to_string(),
                definition: Some("Asynchronous programming in Rust".to_string()),
            },
        ]
    }

    fn create_test_role_graph() -> RoleGraph {
        let thesaurus = create_test_thesaurus();
        RoleGraph {
            thesaurus,
            edges: ahash::AHashMap::new(),
            nodes: ahash::AHashMap::new(),
            documents: ahash::AHashMap::new(),
        }
    }

    #[test]
    fn test_kg_search_service_creation() {
        let service = KGSearchService::new();
        assert!(service.list_kg_terms("test_role").is_err());
    }

    #[test]
    fn test_load_role_graph() {
        let mut service = KGSearchService::new();
        let graph = create_test_role_graph();

        service.load_role_graph("engineer", graph);

        // Should now be able to list terms
        let terms = service.list_kg_terms("engineer").unwrap();
        assert_eq!(terms.len(), 3);
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"tokio".to_string()));
        assert!(terms.contains(&"async".to_string()));
    }

    #[test]
    fn test_get_kg_term() {
        let mut service = KGSearchService::new();
        let graph = create_test_role_graph();
        service.load_role_graph("engineer", graph);

        // Get term by exact match
        let term = service.get_kg_term("engineer", "rust").unwrap();
        assert!(term.is_some());

        let kg_term = term.unwrap();
        assert_eq!(kg_term.term, "Rust");
        assert_eq!(kg_term.normalized_term, "rust");
        assert_eq!(kg_term.id, 1);
        assert_eq!(kg_term.definition, Some("A systems programming language".to_string()));
        assert_eq!(kg_term.url, "https://rust-lang.org");
    }

    #[test]
    fn test_get_kg_term_case_insensitive() {
        let mut service = KGSearchService::new();
        let graph = create_test_role_graph();
        service.load_role_graph("engineer", graph);

        // Should match case-insensitively
        let term1 = service.get_kg_term("engineer", "RUST").unwrap();
        let term2 = service.get_kg_term("engineer", "Rust").unwrap();
        let term3 = service.get_kg_term("engineer", "rust").unwrap();

        assert!(term1.is_some());
        assert!(term2.is_some());
        assert!(term3.is_some());

        assert_eq!(term1.as_ref().unwrap().id, term2.as_ref().unwrap().id);
        assert_eq!(term2.as_ref().unwrap().id, term3.as_ref().unwrap().id);
    }

    #[test]
    fn test_list_kg_terms() {
        let mut service = KGSearchService::new();
        let graph = create_test_role_graph();
        service.load_role_graph("engineer", graph);

        let terms = service.list_kg_terms("engineer").unwrap();
        assert_eq!(terms.len(), 3);

        // All normalized terms should be present
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"tokio".to_string()));
        assert!(terms.contains(&"async".to_string()));
    }
}
