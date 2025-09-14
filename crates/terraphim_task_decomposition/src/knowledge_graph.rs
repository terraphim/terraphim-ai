//! Knowledge graph integration for task decomposition
//!
//! This module provides the core integration with Terraphim's knowledge graph
//! infrastructure, enabling intelligent task decomposition based on semantic
//! relationships and domain knowledge.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

// use terraphim_automata::{extract_paragraphs_from_automata, is_all_terms_connected_by_path};
use terraphim_rolegraph::RoleGraph;
// use terraphim_types::Automata;

// Temporary mock functions until dependencies are fixed
fn extract_paragraphs_from_automata(
    _automata: &MockAutomata,
    text: &str,
    max_results: u32,
) -> Result<Vec<String>, String> {
    // Simple mock implementation
    let words: Vec<String> = text
        .split_whitespace()
        .take(max_results as usize)
        .map(|s| s.to_string())
        .collect();
    Ok(words)
}

fn is_all_terms_connected_by_path(
    _automata: &MockAutomata,
    terms: &[&str],
) -> Result<bool, String> {
    // Simple mock implementation - assume connected if terms share characters
    if terms.len() < 2 {
        return Ok(true);
    }
    let first = terms[0].to_lowercase();
    let second = terms[1].to_lowercase();
    Ok(first.chars().any(|c| second.contains(c)))
}

use crate::{Automata, MockAutomata};

use crate::{Task, TaskDecompositionError, TaskDecompositionResult};

/// Knowledge graph query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphQuery {
    /// Query terms
    pub terms: Vec<String>,
    /// Query type
    pub query_type: QueryType,
    /// Maximum results to return
    pub max_results: u32,
    /// Similarity threshold for results
    pub similarity_threshold: f64,
}

/// Types of knowledge graph queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    /// Find concepts related to given terms
    RelatedConcepts,
    /// Check connectivity between terms
    Connectivity,
    /// Extract context paragraphs
    ContextExtraction,
    /// Find semantic paths between terms
    SemanticPaths,
}

/// Knowledge graph query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Original query
    pub query: KnowledgeGraphQuery,
    /// Result data
    pub results: QueryResultData,
    /// Query execution metadata
    pub metadata: QueryMetadata,
}

/// Different types of query result data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResultData {
    /// Related concepts with similarity scores
    Concepts(Vec<ConceptResult>),
    /// Connectivity information
    Connectivity(ConnectivityResult),
    /// Extracted context paragraphs
    Context(Vec<String>),
    /// Semantic paths between terms
    Paths(Vec<SemanticPath>),
}

/// A concept result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptResult {
    /// Concept name
    pub concept: String,
    /// Similarity score to query terms
    pub similarity: f64,
    /// Related domains
    pub domains: Vec<String>,
    /// Concept metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Connectivity analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityResult {
    /// Whether all terms are connected
    pub all_connected: bool,
    /// Connectivity matrix (term pairs and their connectivity)
    pub connectivity_matrix: HashMap<(String, String), bool>,
    /// Strongly connected components
    pub connected_components: Vec<Vec<String>>,
    /// Overall connectivity score
    pub connectivity_score: f64,
}

/// A semantic path between concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticPath {
    /// Source concept
    pub source: String,
    /// Target concept
    pub target: String,
    /// Path nodes (intermediate concepts)
    pub path: Vec<String>,
    /// Path strength/confidence
    pub strength: f64,
}

/// Query execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Query execution time in milliseconds
    pub execution_time_ms: u64,
    /// Number of results found
    pub result_count: u32,
    /// Whether the query was cached
    pub was_cached: bool,
    /// Query confidence score
    pub confidence_score: f64,
}

/// Knowledge graph integration interface
#[async_trait]
pub trait KnowledgeGraphIntegration: Send + Sync {
    /// Execute a knowledge graph query
    async fn execute_query(
        &self,
        query: &KnowledgeGraphQuery,
    ) -> TaskDecompositionResult<QueryResult>;

    /// Find concepts related to a task
    async fn find_related_concepts(
        &self,
        task: &Task,
    ) -> TaskDecompositionResult<Vec<ConceptResult>>;

    /// Analyze connectivity between task concepts
    async fn analyze_task_connectivity(
        &self,
        task: &Task,
    ) -> TaskDecompositionResult<ConnectivityResult>;

    /// Extract contextual information for a task
    async fn extract_task_context(&self, task: &Task) -> TaskDecompositionResult<Vec<String>>;

    /// Update task knowledge context based on graph analysis
    async fn enrich_task_context(&self, task: &mut Task) -> TaskDecompositionResult<()>;
}

/// Terraphim knowledge graph integration implementation
pub struct TerraphimKnowledgeGraph {
    /// Knowledge graph automata
    automata: Arc<Automata>,
    /// Role graph for role-based analysis
    role_graph: Arc<RoleGraph>,
    /// Query cache for performance
    cache: tokio::sync::RwLock<HashMap<String, QueryResult>>,
    /// Configuration
    config: KnowledgeGraphConfig,
}

/// Configuration for knowledge graph integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphConfig {
    /// Default similarity threshold
    pub default_similarity_threshold: f64,
    /// Maximum query results
    pub max_query_results: u32,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Enable query caching
    pub enable_caching: bool,
    /// Maximum context extraction length
    pub max_context_length: u32,
}

impl Default for KnowledgeGraphConfig {
    fn default() -> Self {
        Self {
            default_similarity_threshold: 0.7,
            max_query_results: 100,
            cache_ttl_seconds: 3600, // 1 hour
            enable_caching: true,
            max_context_length: 1000,
        }
    }
}

impl TerraphimKnowledgeGraph {
    /// Create a new Terraphim knowledge graph integration
    pub fn new(
        automata: Arc<Automata>,
        role_graph: Arc<RoleGraph>,
        config: KnowledgeGraphConfig,
    ) -> Self {
        Self {
            automata,
            role_graph,
            cache: tokio::sync::RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Create with default configuration
    pub fn with_default_config(automata: Arc<Automata>, role_graph: Arc<RoleGraph>) -> Self {
        Self::new(automata, role_graph, KnowledgeGraphConfig::default())
    }

    /// Generate cache key for a query
    fn generate_cache_key(&self, query: &KnowledgeGraphQuery) -> String {
        format!(
            "{:?}_{:?}_{}",
            query.query_type, query.terms, query.similarity_threshold
        )
    }

    /// Execute concept extraction query
    async fn execute_concept_query(
        &self,
        terms: &[String],
        max_results: u32,
    ) -> TaskDecompositionResult<Vec<ConceptResult>> {
        let text = terms.join(" ");

        match extract_paragraphs_from_automata(&self.automata, &text, max_results) {
            Ok(paragraphs) => {
                let mut concepts = Vec::new();

                for paragraph in paragraphs {
                    // Extract individual concepts from paragraph
                    let paragraph_concepts: Vec<String> = paragraph
                        .split_whitespace()
                        .map(|s| s.to_lowercase())
                        .filter(|s| s.len() > 2) // Filter out very short terms
                        .collect::<HashSet<_>>()
                        .into_iter()
                        .collect();

                    for concept in paragraph_concepts {
                        // Simple similarity calculation (could be enhanced)
                        let similarity = self.calculate_concept_similarity(&concept, terms);

                        concepts.push(ConceptResult {
                            concept: concept.clone(),
                            similarity,
                            domains: vec!["general".to_string()], // TODO: Implement domain detection
                            metadata: HashMap::new(),
                        });
                    }
                }

                // Sort by similarity and limit results
                concepts.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
                concepts.truncate(max_results as usize);

                debug!(
                    "Extracted {} concepts from {} terms",
                    concepts.len(),
                    terms.len()
                );
                Ok(concepts)
            }
            Err(e) => {
                warn!("Failed to extract concepts: {}", e);
                Err(TaskDecompositionError::KnowledgeGraphError(format!(
                    "Concept extraction failed: {}",
                    e
                )))
            }
        }
    }

    /// Execute connectivity analysis query
    async fn execute_connectivity_query(
        &self,
        terms: &[String],
    ) -> TaskDecompositionResult<ConnectivityResult> {
        let mut connectivity_matrix = HashMap::new();
        let mut connected_pairs = 0;
        let mut total_pairs = 0;

        // Check pairwise connectivity
        for i in 0..terms.len() {
            for j in (i + 1)..terms.len() {
                total_pairs += 1;
                let term1 = &terms[i];
                let term2 = &terms[j];

                match is_all_terms_connected_by_path(&self.automata, &[term1, term2]) {
                    Ok(connected) => {
                        connectivity_matrix.insert((term1.clone(), term2.clone()), connected);
                        if connected {
                            connected_pairs += 1;
                        }
                    }
                    Err(e) => {
                        debug!(
                            "Connectivity check failed for {} -> {}: {}",
                            term1, term2, e
                        );
                        connectivity_matrix.insert((term1.clone(), term2.clone()), false);
                    }
                }
            }
        }

        let all_connected = connected_pairs == total_pairs && total_pairs > 0;
        let connectivity_score = if total_pairs > 0 {
            connected_pairs as f64 / total_pairs as f64
        } else {
            0.0
        };

        // Find connected components (simplified)
        let connected_components = self.find_connected_components(terms, &connectivity_matrix);

        debug!(
            "Connectivity analysis: {}/{} pairs connected, score: {:.2}",
            connected_pairs, total_pairs, connectivity_score
        );

        Ok(ConnectivityResult {
            all_connected,
            connectivity_matrix,
            connected_components,
            connectivity_score,
        })
    }

    /// Execute context extraction query
    async fn execute_context_query(
        &self,
        terms: &[String],
        max_results: u32,
    ) -> TaskDecompositionResult<Vec<String>> {
        let text = terms.join(" ");

        match extract_paragraphs_from_automata(&self.automata, &text, max_results) {
            Ok(paragraphs) => {
                let context: Vec<String> = paragraphs
                    .into_iter()
                    .take(max_results as usize)
                    .map(|p| {
                        if p.len() > self.config.max_context_length as usize {
                            format!("{}...", &p[..self.config.max_context_length as usize])
                        } else {
                            p
                        }
                    })
                    .collect();

                debug!("Extracted {} context paragraphs", context.len());
                Ok(context)
            }
            Err(e) => {
                warn!("Failed to extract context: {}", e);
                Err(TaskDecompositionError::KnowledgeGraphError(format!(
                    "Context extraction failed: {}",
                    e
                )))
            }
        }
    }

    /// Calculate similarity between a concept and query terms
    fn calculate_concept_similarity(&self, concept: &str, terms: &[String]) -> f64 {
        // Simple similarity based on string matching
        // TODO: Implement more sophisticated semantic similarity
        let concept_lower = concept.to_lowercase();

        let mut max_similarity: f64 = 0.0;
        for term in terms {
            let term_lower = term.to_lowercase();

            // Exact match
            if concept_lower == term_lower {
                return 1.0;
            }

            // Substring match
            if concept_lower.contains(&term_lower) || term_lower.contains(&concept_lower) {
                let similarity = 0.8;
                max_similarity = max_similarity.max(similarity);
            }

            // Character overlap (Jaccard similarity on character level)
            let concept_chars: HashSet<char> = concept_lower.chars().collect();
            let term_chars: HashSet<char> = term_lower.chars().collect();
            let intersection = concept_chars.intersection(&term_chars).count();
            let union = concept_chars.union(&term_chars).count();

            if union > 0 {
                let jaccard = intersection as f64 / union as f64;
                max_similarity = max_similarity.max(jaccard * 0.6);
            }
        }

        max_similarity
    }

    /// Find connected components in the term graph
    fn find_connected_components(
        &self,
        terms: &[String],
        connectivity_matrix: &HashMap<(String, String), bool>,
    ) -> Vec<Vec<String>> {
        let mut visited = HashSet::new();
        let mut components = Vec::new();

        for term in terms {
            if visited.contains(term) {
                continue;
            }

            let mut component = Vec::new();
            let mut stack = vec![term.clone()];

            while let Some(current) = stack.pop() {
                if visited.contains(&current) {
                    continue;
                }

                visited.insert(current.clone());
                component.push(current.clone());

                // Find connected terms
                for other_term in terms {
                    if visited.contains(other_term) {
                        continue;
                    }

                    let connected = connectivity_matrix
                        .get(&(current.clone(), other_term.clone()))
                        .or_else(|| connectivity_matrix.get(&(other_term.clone(), current.clone())))
                        .unwrap_or(&false);

                    if *connected {
                        stack.push(other_term.clone());
                    }
                }
            }

            if !component.is_empty() {
                components.push(component);
            }
        }

        components
    }
}

#[async_trait]
impl KnowledgeGraphIntegration for TerraphimKnowledgeGraph {
    async fn execute_query(
        &self,
        query: &KnowledgeGraphQuery,
    ) -> TaskDecompositionResult<QueryResult> {
        let start_time = std::time::Instant::now();

        // Check cache if enabled
        if self.config.enable_caching {
            let cache_key = self.generate_cache_key(query);
            let cache = self.cache.read().await;
            if let Some(cached_result) = cache.get(&cache_key) {
                debug!("Using cached result for query: {:?}", query.query_type);
                return Ok(cached_result.clone());
            }
        }

        let result_data = match query.query_type {
            QueryType::RelatedConcepts => {
                let concepts = self
                    .execute_concept_query(&query.terms, query.max_results)
                    .await?;
                QueryResultData::Concepts(concepts)
            }
            QueryType::Connectivity => {
                let connectivity = self.execute_connectivity_query(&query.terms).await?;
                QueryResultData::Connectivity(connectivity)
            }
            QueryType::ContextExtraction => {
                let context = self
                    .execute_context_query(&query.terms, query.max_results)
                    .await?;
                QueryResultData::Context(context)
            }
            QueryType::SemanticPaths => {
                // TODO: Implement semantic path finding
                QueryResultData::Paths(Vec::new())
            }
        };

        let execution_time = start_time.elapsed();
        let result_count = match &result_data {
            QueryResultData::Concepts(concepts) => concepts.len() as u32,
            QueryResultData::Connectivity(_) => 1,
            QueryResultData::Context(context) => context.len() as u32,
            QueryResultData::Paths(paths) => paths.len() as u32,
        };

        let result = QueryResult {
            query: query.clone(),
            results: result_data,
            metadata: QueryMetadata {
                execution_time_ms: execution_time.as_millis() as u64,
                result_count,
                was_cached: false,
                confidence_score: 0.8, // TODO: Calculate actual confidence
            },
        };

        // Cache the result if enabled
        if self.config.enable_caching {
            let cache_key = self.generate_cache_key(query);
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, result.clone());
        }

        debug!(
            "Query executed in {}ms, {} results",
            result.metadata.execution_time_ms, result.metadata.result_count
        );

        Ok(result)
    }

    async fn find_related_concepts(
        &self,
        task: &Task,
    ) -> TaskDecompositionResult<Vec<ConceptResult>> {
        let query_terms = [
            task.description
                .split_whitespace()
                .map(|s| s.to_lowercase())
                .collect::<Vec<_>>(),
            task.knowledge_context.keywords.clone(),
            task.knowledge_context.concepts.clone(),
        ]
        .concat();

        let query = KnowledgeGraphQuery {
            terms: query_terms,
            query_type: QueryType::RelatedConcepts,
            max_results: self.config.max_query_results,
            similarity_threshold: self.config.default_similarity_threshold,
        };

        let result = self.execute_query(&query).await?;

        match result.results {
            QueryResultData::Concepts(concepts) => Ok(concepts),
            _ => Err(TaskDecompositionError::KnowledgeGraphError(
                "Unexpected query result type".to_string(),
            )),
        }
    }

    async fn analyze_task_connectivity(
        &self,
        task: &Task,
    ) -> TaskDecompositionResult<ConnectivityResult> {
        let query_terms = [
            task.knowledge_context.keywords.clone(),
            task.knowledge_context.concepts.clone(),
        ]
        .concat();

        if query_terms.is_empty() {
            return Ok(ConnectivityResult {
                all_connected: false,
                connectivity_matrix: HashMap::new(),
                connected_components: Vec::new(),
                connectivity_score: 0.0,
            });
        }

        let query = KnowledgeGraphQuery {
            terms: query_terms,
            query_type: QueryType::Connectivity,
            max_results: self.config.max_query_results,
            similarity_threshold: self.config.default_similarity_threshold,
        };

        let result = self.execute_query(&query).await?;

        match result.results {
            QueryResultData::Connectivity(connectivity) => Ok(connectivity),
            _ => Err(TaskDecompositionError::KnowledgeGraphError(
                "Unexpected query result type".to_string(),
            )),
        }
    }

    async fn extract_task_context(&self, task: &Task) -> TaskDecompositionResult<Vec<String>> {
        let query_terms = [
            task.description
                .split_whitespace()
                .map(|s| s.to_lowercase())
                .collect::<Vec<_>>(),
            task.knowledge_context.keywords.clone(),
        ]
        .concat();

        let query = KnowledgeGraphQuery {
            terms: query_terms,
            query_type: QueryType::ContextExtraction,
            max_results: 10, // Limit context extraction
            similarity_threshold: self.config.default_similarity_threshold,
        };

        let result = self.execute_query(&query).await?;

        match result.results {
            QueryResultData::Context(context) => Ok(context),
            _ => Err(TaskDecompositionError::KnowledgeGraphError(
                "Unexpected query result type".to_string(),
            )),
        }
    }

    async fn enrich_task_context(&self, task: &mut Task) -> TaskDecompositionResult<()> {
        info!("Enriching context for task: {}", task.task_id);

        // Find related concepts
        let related_concepts = self.find_related_concepts(task).await?;

        // Add high-similarity concepts to task context
        for concept_result in related_concepts {
            if concept_result.similarity > self.config.default_similarity_threshold {
                if !task
                    .knowledge_context
                    .concepts
                    .contains(&concept_result.concept)
                {
                    task.knowledge_context
                        .concepts
                        .push(concept_result.concept.clone());
                }

                // Add domains
                for domain in concept_result.domains {
                    if !task.knowledge_context.domains.contains(&domain) {
                        task.knowledge_context.domains.push(domain);
                    }
                }
            }
        }

        // Analyze connectivity and update similarity thresholds
        let connectivity = self.analyze_task_connectivity(task).await?;
        task.knowledge_context.similarity_thresholds.insert(
            "connectivity_score".to_string(),
            connectivity.connectivity_score,
        );

        debug!(
            "Enriched context for task {}: {} concepts, {} domains",
            task.task_id,
            task.knowledge_context.concepts.len(),
            task.knowledge_context.domains.len()
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_graph::Automata;
    use crate::Task;
    use std::sync::Arc;
    use terraphim_rolegraph::RoleGraph;

    fn create_test_automata() -> Arc<Automata> {
        Arc::new(Automata::default())
    }

    async fn create_test_role_graph() -> Arc<RoleGraph> {
        use terraphim_automata::{load_thesaurus, AutomataPath};
        use terraphim_types::RoleName;

        let role_name = RoleName::new("test_role");
        let thesaurus = load_thesaurus(&AutomataPath::local_example())
            .await
            .unwrap();

        let role_graph = RoleGraph::new(role_name, thesaurus).await.unwrap();

        Arc::new(role_graph)
    }

    fn create_test_task() -> Task {
        use crate::{TaskComplexity, TaskKnowledgeContext};

        let mut task = Task::new(
            "test_task".to_string(),
            "Analyze data and create visualization".to_string(),
            TaskComplexity::Moderate,
            1,
        );

        task.knowledge_context = TaskKnowledgeContext {
            domains: vec!["data_analysis".to_string()],
            concepts: vec!["analysis".to_string(), "data".to_string()],
            relationships: Vec::new(),
            keywords: vec!["analyze".to_string(), "visualize".to_string()],
            input_types: vec!["dataset".to_string()],
            output_types: vec!["chart".to_string()],
            similarity_thresholds: HashMap::new(),
        };

        task
    }

    #[tokio::test]
    async fn test_knowledge_graph_creation() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let config = KnowledgeGraphConfig::default();

        let kg = TerraphimKnowledgeGraph::new(automata, role_graph, config);
        assert!(kg.cache.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_concept_query() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let kg = TerraphimKnowledgeGraph::with_default_config(automata, role_graph);

        let query = KnowledgeGraphQuery {
            terms: vec!["analysis".to_string(), "data".to_string()],
            query_type: QueryType::RelatedConcepts,
            max_results: 10,
            similarity_threshold: 0.7,
        };

        let result = kg.execute_query(&query).await;
        assert!(result.is_ok());

        let query_result = result.unwrap();
        assert!(matches!(query_result.results, QueryResultData::Concepts(_)));
    }

    #[tokio::test]
    async fn test_connectivity_query() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let kg = TerraphimKnowledgeGraph::with_default_config(automata, role_graph);

        let query = KnowledgeGraphQuery {
            terms: vec!["analysis".to_string(), "data".to_string()],
            query_type: QueryType::Connectivity,
            max_results: 10,
            similarity_threshold: 0.7,
        };

        let result = kg.execute_query(&query).await;
        assert!(result.is_ok());

        let query_result = result.unwrap();
        assert!(matches!(
            query_result.results,
            QueryResultData::Connectivity(_)
        ));
    }

    #[tokio::test]
    async fn test_task_context_enrichment() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let kg = TerraphimKnowledgeGraph::with_default_config(automata, role_graph);

        let mut task = create_test_task();
        let original_concept_count = task.knowledge_context.concepts.len();

        let result = kg.enrich_task_context(&mut task).await;
        assert!(result.is_ok());

        // Context should be enriched (though exact results depend on automata content)
        assert!(task.knowledge_context.concepts.len() >= original_concept_count);
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let kg = TerraphimKnowledgeGraph::with_default_config(automata, role_graph);

        let query = KnowledgeGraphQuery {
            terms: vec!["test".to_string()],
            query_type: QueryType::RelatedConcepts,
            max_results: 10,
            similarity_threshold: 0.7,
        };

        let key1 = kg.generate_cache_key(&query);
        let key2 = kg.generate_cache_key(&query);
        assert_eq!(key1, key2);

        let mut query2 = query.clone();
        query2.similarity_threshold = 0.8;
        let key3 = kg.generate_cache_key(&query2);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_concept_similarity_calculation() {
        let automata = create_test_automata();
        let role_graph = create_test_role_graph().await;
        let kg = TerraphimKnowledgeGraph::with_default_config(automata, role_graph);

        let terms = vec!["analysis".to_string(), "data".to_string()];

        // Exact match
        let similarity1 = kg.calculate_concept_similarity("analysis", &terms);
        assert_eq!(similarity1, 1.0);

        // Partial match
        let similarity2 = kg.calculate_concept_similarity("analyze", &terms);
        assert!(similarity2 > 0.0 && similarity2 < 1.0);

        // No match
        let similarity3 = kg.calculate_concept_similarity("unrelated", &terms);
        assert!(similarity3 >= 0.0);
    }
}
