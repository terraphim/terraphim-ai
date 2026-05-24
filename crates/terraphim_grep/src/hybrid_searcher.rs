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

/// Default weight applied to KG matches when boosting a chunk's relevance score.
/// A weight of 1.0 means a chunk whose path and content fully match the top KG concept
/// can roughly double its rank vs an unmatched chunk with the same base score.
pub const DEFAULT_KG_BOOST_WEIGHT: f64 = 1.0;

/// Compute the KG boost for a single chunk against a set of matched concepts.
///
/// For each concept whose lowercased `name` (or `display_value`, if set) appears in the
/// chunk's lowercased source path or content, the concept's normalised score contributes
/// to the boost. The result is in `[0.0, weight]`; callers add it to the chunk's
/// `relevance_score`.
///
/// Why path-and-content: matching only paths misses content-defined concepts (a struct
/// `RetryPolicy` declared in `src/network.rs`); matching only content over-rewards files
/// that mention a concept in passing. Combining the two is a sensible default.
pub fn score_kg_boost(chunk: &RetrievedChunk, concepts: &[KgConcept], weight: f64) -> f64 {
    if concepts.is_empty() || weight <= 0.0 {
        return 0.0;
    }
    let max_concept_score: f64 = concepts.iter().map(|c| c.score).fold(0.0, f64::max);
    if max_concept_score <= 0.0 {
        return 0.0;
    }
    let source_lower = chunk.source.to_lowercase();
    let content_lower = chunk.content.to_lowercase();

    let mut boost = 0.0;
    for c in concepts {
        let needle = c
            .display_value
            .as_deref()
            .unwrap_or(c.name.as_str())
            .to_lowercase();
        if needle.is_empty() {
            continue;
        }
        if source_lower.contains(&needle) || content_lower.contains(&needle) {
            boost += c.score / max_concept_score;
        }
    }
    (boost * weight).min(weight * concepts.len() as f64)
}

/// Apply KG boost to a batch of chunks and sort by boosted score (descending).
/// Mutates `relevance_score` in place so downstream consumers can see the boost reflected
/// in the JSON output -- otherwise the ordering would be inexplicable.
pub fn boost_chunks_with_kg(
    mut chunks: Vec<RetrievedChunk>,
    concepts: &[KgConcept],
) -> Vec<RetrievedChunk> {
    for chunk in chunks.iter_mut() {
        let boost = score_kg_boost(chunk, concepts, DEFAULT_KG_BOOST_WEIGHT);
        chunk.relevance_score += boost;
    }
    chunks.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    chunks
}

pub struct HybridSearcher {
    role_graph: Arc<tokio::sync::RwLock<terraphim_rolegraph::RoleGraph>>,
    /// Kept alongside the rolegraph so KG-style boosting still works when no documents
    /// have been indexed into the graph. The rolegraph requires indexed documents to
    /// return meaningful query results; the raw thesaurus is enough to identify which
    /// of the user's known concepts touch the query.
    thesaurus: terraphim_types::Thesaurus,
    search_path: PathBuf,
}

impl HybridSearcher {
    pub fn new(
        role_name: String,
        thesaurus: terraphim_types::Thesaurus,
    ) -> Result<Self, terraphim_rolegraph::Error> {
        let rolegraph = terraphim_rolegraph::RoleGraph::new_sync(
            terraphim_types::RoleName::new(&role_name),
            thesaurus.clone(),
        )?;

        Ok(Self {
            role_graph: Arc::new(tokio::sync::RwLock::new(rolegraph)),
            thesaurus,
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

        let thesaurus = self.thesaurus.clone();

        let (kg_concepts, code_results) = match options.haystack {
            Haystack::All | Haystack::Code => {
                let kg_handle = tokio::spawn({
                    let query = query_owned.clone();
                    let graph = role_graph.clone();
                    let thes = thesaurus.clone();
                    async move { Self::search_kg(&query, max_results, graph, &thes).await }
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
                    Self::search_kg(&query_owned, max_results, role_graph.clone(), &thesaurus)
                        .await?;
                (kg_concepts, vec![])
            }
        };

        // KG boost: re-rank code_results so chunks whose source path or content matches
        // a thesaurus concept rank above generic matches. The base relevance from fff is
        // currently uniform (1.0 per match), so without this step the user's knowledge
        // does not influence ordering at all. Boost in place; the boosted score is what
        // the JSON output reports so downstream tools see why a chunk ranked where it did.
        let code_results = boost_chunks_with_kg(code_results, &kg_concepts);

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
        thesaurus: &terraphim_types::Thesaurus,
    ) -> Result<Vec<KgConcept>, String> {
        let graph_guard = graph.read().await;

        let matches = graph_guard
            .query_graph_with_trigger_fallback(query, None, Some(limit), false)
            .map_err(|e| e.to_string())?;

        if !matches.is_empty() {
            let concepts = matches
                .into_iter()
                .map(|(doc_id, indexed_doc)| KgConcept {
                    id: 0,
                    name: doc_id,
                    display_value: None,
                    score: indexed_doc.rank as f64,
                })
                .collect();
            return Ok(concepts);
        }

        // Fallback: rolegraph returned nothing (graph has no indexed documents yet, or no
        // node matched the query). Fall back to thesaurus-only matching so KG boost still
        // fires. Match the rolegraph's Aho-Corasick semantics by lowercasing both sides
        // and scanning each thesaurus key for substring presence in the query.
        let query_lower = query.to_lowercase();
        let mut concepts: Vec<KgConcept> = thesaurus
            .keys()
            .filter_map(|key| {
                let key_str = key.as_str();
                let key_lower = key_str.to_lowercase();
                if query_lower.contains(&key_lower) || key_lower.contains(&query_lower) {
                    Some(KgConcept {
                        id: 0,
                        name: key_str.to_string(),
                        display_value: None,
                        score: 1.0,
                    })
                } else {
                    None
                }
            })
            .take(limit)
            .collect();
        // Stable ordering for deterministic boost output across runs.
        concepts.sort_by(|a, b| a.name.cmp(&b.name));
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

    fn chunk(source: &str, content: &str, score: f64) -> RetrievedChunk {
        RetrievedChunk {
            content: content.to_string(),
            source: source.to_string(),
            line_start: Some(1),
            line_end: Some(1),
            relevance_score: score,
            haystack: "code",
        }
    }

    fn concept(name: &str, score: f64) -> KgConcept {
        KgConcept {
            id: 0,
            name: name.to_string(),
            display_value: None,
            score,
        }
    }

    #[test]
    fn kg_boost_promotes_matching_chunks_to_top() {
        // Two chunks with identical base scores. Only one mentions the KG concept in its
        // path or content. After boost, the matching chunk must rank first -- this is the
        // "your knowledge tops the results" guarantee.
        let chunks = vec![
            chunk("src/parse_csv.rs", "fn parse_csv() {}", 1.0),
            chunk("src/retry_policy.rs", "pub struct RetryPolicy {}", 1.0),
        ];
        let concepts = vec![concept("retry_policy", 0.9)];

        let ranked = boost_chunks_with_kg(chunks, &concepts);
        assert_eq!(ranked[0].source, "src/retry_policy.rs");
        assert!(
            ranked[0].relevance_score > ranked[1].relevance_score,
            "KG-matched chunk must outscore the unmatched chunk: {:?} vs {:?}",
            ranked[0].relevance_score,
            ranked[1].relevance_score,
        );
    }

    #[test]
    fn kg_boost_no_concepts_is_a_noop() {
        // No KG concepts -> no boost -> ordering by base score only. Confirms the boost
        // path stays neutral when there's nothing to learn from the KG.
        let chunks = vec![chunk("a.rs", "alpha", 0.5), chunk("b.rs", "beta", 0.9)];
        let ranked = boost_chunks_with_kg(chunks, &[]);
        assert_eq!(ranked[0].source, "b.rs");
        assert!((ranked[0].relevance_score - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn kg_boost_stacks_when_multiple_concepts_match() {
        // A chunk that matches *two* concepts gets a larger boost than one matching only
        // one. Pins down the additive behaviour of score_kg_boost.
        let one_match = chunk("src/retry.rs", "fn retry() {}", 1.0);
        let two_matches = chunk("src/retry.rs", "fn retry() -> backoff::Result<()>", 1.0);
        let concepts = vec![concept("retry", 1.0), concept("backoff", 1.0)];

        let b1 = score_kg_boost(&one_match, &concepts, 1.0);
        let b2 = score_kg_boost(&two_matches, &concepts, 1.0);
        assert!(
            b2 > b1,
            "two-concept match must score higher than one-concept match: {b2} vs {b1}"
        );
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
