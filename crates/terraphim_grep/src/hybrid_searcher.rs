use std::sync::Arc;

use serde::{Deserialize, Serialize};
use terraphim_types::Document;

#[derive(Debug, Clone)]
pub struct GrepOptions {
    pub haystack: Haystack,
    pub context_lines: usize,
    pub max_results: usize,
    pub force_rlm: bool,
    pub include_answer: bool,
}

impl Default for GrepOptions {
    fn default() -> Self {
        Self {
            haystack: Haystack::All,
            context_lines: 0,
            max_results: 50,
            force_rlm: false,
            include_answer: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Haystack {
    #[default]
    Code,
    Docs,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub content: String,
    pub source: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub relevance_score: f64,
    pub haystack: &'static str,
}

impl From<Document> for RetrievedChunk {
    fn from(doc: Document) -> Self {
        Self {
            content: doc.body,
            source: doc.url,
            line_start: None,
            line_end: None,
            relevance_score: doc.rank.unwrap_or(0) as f64,
            haystack: "code",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgConcept {
    pub id: u64,
    pub name: String,
    pub display_value: Option<String>,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct HybridResults {
    pub code_results: Vec<RetrievedChunk>,
    pub doc_results: Vec<RetrievedChunk>,
    pub kg_concepts: Vec<KgConcept>,
}

impl HybridResults {
    pub fn to_chunks(&self) -> Vec<RetrievedChunk> {
        let mut chunks = Vec::with_capacity(self.code_results.len() + self.doc_results.len());
        chunks.extend(self.code_results.clone());
        chunks.extend(self.doc_results.clone());
        chunks
    }

    pub fn total_results(&self) -> usize {
        self.code_results.len() + self.doc_results.len() + self.kg_concepts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.code_results.is_empty() && self.doc_results.is_empty() && self.kg_concepts.is_empty()
    }
}

/// Hybrid searcher that combines KG, code, and doc search.
pub struct HybridSearcher {
    role_graph: Arc<tokio::sync::RwLock<terraphim_rolegraph::RoleGraph>>,
    #[cfg(feature = "code-search")]
    code_searcher: Option<CodeSearcher>,
    #[cfg(not(feature = "code-search"))]
    _code_searcher: (),
    #[cfg(feature = "doc-search")]
    doc_searcher: Option<DocSearcher>,
    #[cfg(not(feature = "doc-search"))]
    _doc_searcher: (),
}

#[cfg(feature = "code-search")]
pub struct CodeSearcher {
    // Future: will use fff-search or ripgrep for code search
}

#[cfg(not(feature = "code-search"))]
pub struct CodeSearcher(pub ());

#[cfg(feature = "doc-search")]
pub struct DocSearcher {
    // Future: will use haystack_core HaystackProvider for doc search
}

#[cfg(not(feature = "doc-search"))]
pub struct DocSearcher(pub ());

impl HybridSearcher {
    /// Create a new HybridSearcher with the given role name and thesaurus.
    pub fn new(
        role_name: String,
        thesaurus: terraphim_types::Thesaurus,
    ) -> Result<Self, terraphim_rolegraph::Error> {
        let rolegraph = terraphim_rolegraph::RoleGraph::new_sync(
            terraphim_types::RoleName::new(&role_name),
            thesaurus,
        )?;

        Ok(Self {
            role_graph: Arc::new(tokio::sync::RwLock::new(rolegraph)),
            #[cfg(feature = "code-search")]
            code_searcher: None,
            #[cfg(not(feature = "code-search"))]
            _code_searcher: (),
            #[cfg(feature = "doc-search")]
            doc_searcher: None,
            #[cfg(not(feature = "doc-search"))]
            _doc_searcher: (),
        })
    }

    /// Create a HybridSearcher with an existing RoleGraph.
    pub fn with_role_graph(role_graph: terraphim_rolegraph::RoleGraph) -> Self {
        Self {
            role_graph: Arc::new(tokio::sync::RwLock::new(role_graph)),
            #[cfg(feature = "code-search")]
            code_searcher: None,
            #[cfg(not(feature = "code-search"))]
            _code_searcher: (),
            #[cfg(feature = "doc-search")]
            doc_searcher: None,
            #[cfg(not(feature = "doc-search"))]
            _doc_searcher: (),
        }
    }

    #[cfg(feature = "code-search")]
    pub fn with_code_searcher(mut self, searcher: CodeSearcher) -> Self {
        self.code_searcher = Some(searcher);
        self
    }

    #[cfg(not(feature = "code-search"))]
    pub fn with_code_searcher(self, _searcher: CodeSearcher) -> Self {
        self
    }

    #[cfg(feature = "doc-search")]
    pub fn with_doc_searcher(mut self, searcher: DocSearcher) -> Self {
        self.doc_searcher = Some(searcher);
        self
    }

    #[cfg(not(feature = "doc-search"))]
    pub fn with_doc_searcher(self, _searcher: DocSearcher) -> Self {
        self
    }

    pub async fn search(
        &self,
        query: &str,
        options: &GrepOptions,
    ) -> Result<HybridResults, String> {
        let max_results = options.max_results;

        // Run searches in parallel based on haystack configuration
        // We need to clone data before spawning tasks to avoid lifetime issues
        let role_graph = self.role_graph.clone();
        let query_owned = query.to_string();
        let max_results_owned = max_results;

        let (kg_concepts, code_results, doc_results) = match options.haystack {
            Haystack::All => {
                let kg_handle = tokio::spawn({
                    let query = query_owned.clone();
                    let graph = role_graph.clone();
                    async move { Self::search_kg_static(&query, max_results_owned, graph).await }
                });

                let code_handle = tokio::spawn({
                    let query = query_owned.clone();
                    async move { Self::search_code_static(&query, max_results_owned).await }
                });

                let doc_handle = tokio::spawn({
                    let query = query_owned.clone();
                    async move { Self::search_docs_static(&query, max_results_owned).await }
                });

                let kg_concepts = kg_handle
                    .await
                    .map_err(|e| format!("KG search join error: {}", e))??;
                let code_results = code_handle
                    .await
                    .map_err(|e| format!("Code search join error: {}", e))??;
                let doc_results = doc_handle
                    .await
                    .map_err(|e| format!("Doc search join error: {}", e))??;

                (kg_concepts, code_results, doc_results)
            }
            Haystack::Code => {
                let kg_concepts =
                    Self::search_kg_static(&query_owned, max_results_owned, role_graph.clone())
                        .await?;
                let code_results =
                    Self::search_code_static(&query_owned, max_results_owned).await?;
                (kg_concepts, code_results, vec![])
            }
            Haystack::Docs => {
                let kg_concepts =
                    Self::search_kg_static(&query_owned, max_results_owned, role_graph.clone())
                        .await?;
                let doc_results = Self::search_docs_static(&query_owned, max_results_owned).await?;
                (kg_concepts, vec![], doc_results)
            }
        };

        Ok(HybridResults {
            code_results,
            doc_results,
            kg_concepts,
        })
    }

    async fn search_kg_static(
        query: &str,
        limit: usize,
        graph: Arc<tokio::sync::RwLock<terraphim_rolegraph::RoleGraph>>,
    ) -> Result<Vec<KgConcept>, String> {
        let graph_guard = graph.read().await;

        let matches = graph_guard
            .query_graph_with_trigger_fallback(query, None, Some(limit), false)
            .map_err(|e| e.to_string())?;

        let concepts = matches
            .into_iter()
            .map(|(doc_id, indexed_doc)| KgConcept {
                id: 0,
                name: doc_id,
                display_value: None,
                score: indexed_doc.rank as f64,
            })
            .collect();

        Ok(concepts)
    }

    async fn search_code_static(
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<RetrievedChunk>, String> {
        // Code search placeholder - would use fff-search or ripgrep
        Ok(vec![])
    }

    async fn search_docs_static(
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<RetrievedChunk>, String> {
        // Doc search placeholder - would use haystack_core HaystackProvider
        Ok(vec![])
    }

    pub fn fuse_and_rank(&self, mut results: Vec<RetrievedChunk>) -> Vec<RetrievedChunk> {
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hybrid_results_empty() {
        let results = HybridResults {
            code_results: vec![],
            doc_results: vec![],
            kg_concepts: vec![],
        };
        assert!(results.is_empty());
        assert_eq!(results.total_results(), 0);
    }

    #[tokio::test]
    async fn test_hybrid_results_to_chunks() {
        let results = HybridResults {
            code_results: vec![RetrievedChunk {
                content: "test".to_string(),
                source: "file1.rs".to_string(),
                line_start: Some(1),
                line_end: Some(1),
                relevance_score: 0.9,
                haystack: "code",
            }],
            doc_results: vec![RetrievedChunk {
                content: "test doc".to_string(),
                source: "file2.md".to_string(),
                line_start: Some(5),
                line_end: Some(5),
                relevance_score: 0.8,
                haystack: "docs",
            }],
            kg_concepts: vec![],
        };

        let chunks = results.to_chunks();
        assert_eq!(chunks.len(), 2);
        assert_eq!(results.total_results(), 2);
    }

    #[test]
    fn test_grep_options_default() {
        let options = GrepOptions::default();
        assert_eq!(options.haystack, Haystack::All);
        assert_eq!(options.context_lines, 0);
        assert_eq!(options.max_results, 50);
        assert!(!options.force_rlm);
        assert!(!options.include_answer);
    }
}
