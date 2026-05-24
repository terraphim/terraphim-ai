use std::path::PathBuf;
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

pub struct HybridSearcher {
    role_graph: Arc<tokio::sync::RwLock<terraphim_rolegraph::RoleGraph>>,
    search_path: PathBuf,
}

impl HybridSearcher {
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
            search_path: PathBuf::from("."),
        })
    }

    pub fn with_search_path(mut self, path: PathBuf) -> Self {
        self.search_path = path;
        self
    }

    pub async fn search(
        &self,
        query: &str,
        options: &GrepOptions,
    ) -> Result<HybridResults, String> {
        let max_results = options.max_results;
        let search_path = self.search_path.clone();
        let role_graph = self.role_graph.clone();
        let query_owned = query.to_string();

        let (kg_concepts, code_results) = match options.haystack {
            Haystack::All | Haystack::Code => {
                let kg_handle = tokio::spawn({
                    let query = query_owned.clone();
                    let graph = role_graph.clone();
                    async move { Self::search_kg(&query, max_results, graph).await }
                });

                let code_handle = tokio::spawn({
                    let query = query_owned.clone();
                    let path = search_path.clone();
                    async move { Self::search_code(&query, max_results, path).await }
                });

                let kg_concepts = kg_handle
                    .await
                    .map_err(|e| format!("KG search join error: {}", e))??;
                let code_results = code_handle
                    .await
                    .map_err(|e| format!("Code search join error: {}", e))??;
                (kg_concepts, code_results)
            }
            Haystack::Docs => {
                let kg_concepts =
                    Self::search_kg(&query_owned, max_results, role_graph.clone()).await?;
                (kg_concepts, vec![])
            }
        };

        Ok(HybridResults {
            code_results,
            doc_results: vec![],
            kg_concepts,
        })
    }

    async fn search_kg(
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

    async fn search_code(
        query: &str,
        limit: usize,
        search_path: PathBuf,
    ) -> Result<Vec<RetrievedChunk>, String> {
        #[cfg(feature = "code-search")]
        {
            use fff_search::{
                ContentCacheBudget, FFFMode, FilePicker, FilePickerOptions, GrepMode,
                GrepSearchOptions, grep_search, parse_grep_query,
            };

            let mut picker = FilePicker::new(FilePickerOptions {
                base_path: search_path.to_string_lossy().to_string(),
                mode: FFFMode::Ai,
                watch: false,
                warmup_mmap_cache: false,
                cache_budget: None,
            })
            .map_err(|e| format!("FilePicker init failed: {}", e))?;

            picker
                .collect_files()
                .map_err(|e| format!("File scan failed: {}", e))?;

            let files = picker.get_files().to_vec();
            if files.is_empty() {
                return Ok(vec![]);
            }

            let fff_query = parse_grep_query(query);
            let budget = ContentCacheBudget::default();
            let options = GrepSearchOptions {
                max_file_size: 10 * 1024 * 1024,
                max_matches_per_file: 200,
                smart_case: true,
                file_offset: 0,
                page_limit: limit,
                mode: GrepMode::PlainText,
                time_budget_ms: 0,
                before_context: 0,
                after_context: 0,
                classify_definitions: false,
            };

            let result = grep_search(&files, &fff_query, &options, &budget, None, None, None);

            let chunks = result
                .matches
                .into_iter()
                .take(limit)
                .filter_map(|m| {
                    let file = result.files.get(m.file_index)?;
                    Some(RetrievedChunk {
                        content: m.line_content,
                        source: file.relative_path.clone(),
                        line_start: Some(m.line_number as usize),
                        line_end: Some(m.line_number as usize),
                        relevance_score: 1.0,
                        haystack: "code",
                    })
                })
                .collect();

            Ok(chunks)
        }

        #[cfg(not(feature = "code-search"))]
        {
            let _ = (query, limit, search_path);
            Ok(vec![])
        }
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
