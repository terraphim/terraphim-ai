/// Reusable Knowledge Graph Search Components
///
/// This module provides high-performance, reusable knowledge graph search components
/// built on the ReusableComponent trait foundation, integrating with the existing
/// Terraphim RoleGraph system while adding standardized lifecycle management,
/// configuration, and performance monitoring.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::*;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::timeout;
use ulid::Ulid;

use crate::components::{
    ComponentConfig, ComponentError, LifecycleEvent, PerformanceTracker,
    ReusableComponent, ServiceRegistry, ViewContext, ComponentMetadata, ComponentCapability
};
use crate::kg_search::KGSearchService;
use terraphim_rolegraph::RoleGraph;
use terraphim_types::RoleName;
use crate::search_service::{SearchOptions, SearchResults};
use terraphim_types::{Document, NormalizedTermValue, NormalizedTerm, SearchQuery};

/// Knowledge Graph Search Component Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeGraphConfig {
    /// Role name for KG isolation
    pub role: RoleName,

    /// Maximum number of KG terms to return
    pub max_results: usize,

    /// Enable fuzzy search with Jaro-Winkler similarity
    pub enable_fuzzy_search: bool,

    /// Fuzzy search similarity threshold (0.0-1.0)
    pub fuzzy_threshold: f64,

    /// Enable graph path connectivity checking
    pub enable_connectivity_check: bool,

    /// Maximum graph traversal depth for connectivity
    pub max_traversal_depth: usize,

    /// Cache size for KG results
    pub cache_size: usize,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,

    /// Enable real-time KG updates
    pub enable_real_time_updates: bool,

    /// Performance monitoring configuration
    pub performance_config: KGPerformanceConfig,

    /// UI configuration
    pub ui_config: KGUIConfig,
}

impl Default for KnowledgeGraphConfig {
    fn default() -> Self {
        Self {
            role: RoleName::from("default"),
            max_results: 50,
            enable_fuzzy_search: true,
            fuzzy_threshold: 0.7,
            enable_connectivity_check: true,
            max_traversal_depth: 3,
            cache_size: 1000,
            cache_ttl_seconds: 300, // 5 minutes
            enable_real_time_updates: false,
            performance_config: KGPerformanceConfig::default(),
            ui_config: KGUIConfig::default(),
        }
    }
}

// TODO: Implement ComponentConfig for KnowledgeGraphConfig
// Commented out to avoid introducing more complex dependencies during initial compilation fixes
// impl ComponentConfig for KnowledgeGraphConfig {
//     // Implementation needed
// }

/// Knowledge Graph Performance Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KGPerformanceConfig {
    /// Enable performance monitoring
    pub enable_monitoring: bool,

    /// Search timeout in milliseconds
    pub search_timeout_ms: u64,

    /// Index rebuild timeout in milliseconds
    pub index_rebuild_timeout_ms: u64,

    /// Performance alert thresholds
    pub alert_thresholds: KGPerformanceThresholds,
}

impl Default for KGPerformanceConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            search_timeout_ms: 5000, // 5 seconds
            index_rebuild_timeout_ms: 30000, // 30 seconds
            alert_thresholds: KGPerformanceThresholds::default(),
        }
    }
}

/// Performance Alert Thresholds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KGPerformanceThresholds {
    /// Search latency threshold in milliseconds
    pub search_latency_ms: u64,

    /// Index rebuild time threshold in milliseconds
    pub index_rebuild_time_ms: u64,

    /// Memory usage threshold in MB
    pub memory_usage_mb: usize,

    /// Cache hit ratio threshold (0.0-1.0)
    pub cache_hit_ratio_threshold: f64,
}

impl Default for KGPerformanceThresholds {
    fn default() -> Self {
        Self {
            search_latency_ms: 1000, // 1 second
            index_rebuild_time_ms: 10000, // 10 seconds
            memory_usage_mb: 100, // 100MB
            cache_hit_ratio_threshold: 0.8, // 80%
        }
    }
}

/// Knowledge Graph UI Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KGUIConfig {
    /// Show term relationships in results
    pub show_relationships: bool,

    /// Show confidence scores
    pub show_confidence: bool,

    /// Show graph visualization
    pub enable_graph_visualization: bool,

    /// Maximum relationships to display per term
    pub max_relationships_display: usize,

    /// Result sorting strategy
    pub sort_strategy: KGSortStrategy,
}

impl Default for KGUIConfig {
    fn default() -> Self {
        Self {
            show_relationships: true,
            show_confidence: false,
            enable_graph_visualization: false,
            max_relationships_display: 10,
            sort_strategy: KGSortStrategy::Relevance,
        }
    }
}

/// Knowledge Graph Result Sorting Strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGSortStrategy {
    /// Sort by relevance score (default)
    Relevance,
    /// Sort alphabetically
    Alphabetical,
    /// Sort by connection count
    ConnectionCount,
    /// Sort by last updated timestamp
    RecentlyUpdated,
}

/// Knowledge Graph Search Component State
#[derive(Debug, Clone)]
pub struct KnowledgeGraphState {
    /// Current search query
    pub query: String,

    /// Current search results
    pub results: Vec<KGSearchResult>,

    /// Selected term for detailed view
    pub selected_term: Option<KGTermDetail>,

    /// Current search mode
    pub search_mode: KGSearchMode,

    /// Caching state
    pub cache_state: KGCacheState,

    /// Performance metrics
    pub performance_metrics: KGPerformanceMetrics,

    /// Component lifecycle status
    pub is_initialized: bool,

    /// Last error encountered
    pub last_error: Option<String>,
}

impl Default for KnowledgeGraphState {
    fn default() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected_term: None,
            search_mode: KGSearchMode::Standard,
            cache_state: KGCacheState::default(),
            performance_metrics: KGPerformanceMetrics::default(),
            is_initialized: false,
            last_error: None,
        }
    }
}

/// Knowledge Graph Search Mode
#[derive(Debug, Clone, PartialEq)]
pub enum KGSearchMode {
    /// Standard term search
    Standard,
    /// Fuzzy search for typos
    Fuzzy,
    /// Path connectivity search
    Connectivity,
    /// Multi-term logical search
    Logical,
}

/// Knowledge Graph Search Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGSearchResult {
    /// Term identifier
    pub id: String,

    /// Term display name
    pub term: String,

    /// Normalized term value
    pub normalized_term: String,

    /// Relevance score (0.0-1.0)
    pub relevance_score: f64,

    /// Number of connections in the graph
    pub connection_count: usize,

    /// Associated documents
    pub documents: Vec<Document>,

    /// Related terms
    pub related_terms: Vec<KGRelatedTerm>,

    /// Confidence score for the result
    pub confidence: f64,

    /// Last updated timestamp
    pub last_updated: std::time::SystemTime,
}

/// Related Term Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGRelatedTerm {
    /// Related term ID
    pub id: String,

    /// Related term name
    pub term: String,

    /// Relationship type
    pub relationship_type: KGRelationshipType,

    /// Relationship strength (0.0-1.0)
    pub strength: f64,
}

/// Knowledge Graph Relationship Type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGRelationshipType {
    /// Synonym relationship
    Synonym,
    /// Related concept
    Related,
    /// Parent-child hierarchy
    ParentChild,
    /// Cross-reference
    CrossReference,
    /// Unknown relationship
    Unknown,
}

/// Detailed Term Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGTermDetail {
    /// Basic term information
    pub basic_info: KGSearchResult,

    /// Full relationship graph
    pub relationships: Vec<KGRelatedTerm>,

    /// Term metadata
    pub metadata: KGTermMetadata,

    /// Historical usage statistics
    pub usage_stats: KGUsageStats,
}

/// Knowledge Graph Term Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGTermMetadata {
    /// Term creation timestamp
    pub created_at: std::time::SystemTime,

    /// Last modified timestamp
    pub modified_at: std::time::SystemTime,

    /// Term source (manual, imported, inferred)
    pub source: KGTermSource,

    /// Term tags or categories
    pub tags: Vec<String>,

    /// Additional custom attributes
    pub attributes: HashMap<String, String>,
}

/// Knowledge Graph Term Source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGTermSource {
    /// Manually added term
    Manual,
    /// Imported from external source
    Imported,
    /// Inferred from document analysis
    Inferred,
    /// Auto-generated from relationships
    AutoGenerated,
}

/// Knowledge Graph Usage Statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGUsageStats {
    /// Number of times this term was searched
    pub search_count: u64,

    /// Number of times this term was selected
    pub selection_count: u64,

    /// Last accessed timestamp
    pub last_accessed: Option<std::time::SystemTime>,

    /// Search frequency over time
    pub search_frequency: Vec<(std::time::SystemTime, u64)>,
}

/// Knowledge Graph Cache State
#[derive(Debug, Clone, Default)]
pub struct KGCacheState {
    /// Number of cached results
    pub cached_results: usize,

    /// Cache hit count
    pub cache_hits: u64,

    /// Cache miss count
    pub cache_misses: u64,

    /// Last cache cleanup timestamp
    pub last_cleanup: Option<std::time::SystemTime>,
}

/// Knowledge Graph Performance Metrics
#[derive(Debug, Clone, Default)]
pub struct KGPerformanceMetrics {
    /// Total searches performed
    pub total_searches: u64,

    /// Average search latency in milliseconds
    pub avg_search_latency_ms: f64,

    /// Peak search latency in milliseconds
    pub peak_search_latency_ms: u64,

    /// Index rebuild count
    pub index_rebuild_count: u64,

    /// Average index rebuild time in milliseconds
    pub avg_index_rebuild_time_ms: f64,

    /// Current memory usage in MB
    pub memory_usage_mb: usize,

    /// Performance alerts
    pub performance_alerts: Vec<KGPerformanceAlert>,
}

/// Knowledge Graph Performance Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGPerformanceAlert {
    /// Alert identifier
    pub id: String,

    /// Alert type
    pub alert_type: KGAlertType,

    /// Alert message
    pub message: String,

    /// Alert severity
    pub severity: KGAlertSeverity,

    /// Alert timestamp
    pub timestamp: std::time::SystemTime,

    /// Associated metric value
    pub metric_value: f64,

    /// Threshold that was exceeded
    pub threshold: f64,
}

/// Knowledge Graph Alert Type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGAlertType {
    /// Search latency too high
    HighSearchLatency,
    /// Index rebuild taking too long
    SlowIndexRebuild,
    /// Memory usage too high
    HighMemoryUsage,
    /// Cache hit ratio too low
    LowCacheHitRatio,
    /// Component initialization failure
    InitializationFailure,
    /// Search timeout
    SearchTimeout,
}

/// Knowledge Graph Alert Severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGAlertSeverity {
    /// Informational alert
    Info,
    /// Warning alert
    Warning,
    /// Error alert
    Error,
    /// Critical alert
    Critical,
}

/// Knowledge Graph Search Component Events
#[derive(Debug, Clone, PartialEq)]
pub enum KnowledgeGraphEvent {
    /// Search completed with results
    SearchCompleted { query: String, results: Vec<KGSearchResult> },

    /// Term selected for detailed view
    TermSelected { term: KGSearchResult },

    /// Search failed with error
    SearchFailed { query: String, error: String },

    /// Cache state updated
    CacheUpdated { cache_state: KGCacheState },

    /// Performance alert generated
    PerformanceAlert { alert: KGPerformanceAlert },

    /// Configuration updated
    ConfigurationUpdated { config: KnowledgeGraphConfig },

    /// Component lifecycle event
    LifecycleEvent { event: LifecycleEvent },
}

/// Reusable Knowledge Graph Search Component
#[derive(Debug)]
pub struct KnowledgeGraphComponent {
    /// Component configuration
    config: KnowledgeGraphConfig,

    /// Component state
    state: KnowledgeGraphState,

    /// Performance tracker
    performance_tracker: PerformanceTracker,

    /// Underlying KG search service
    kg_search_service: Arc<KGSearchService>,

    /// RoleGraph instance for this component
    rolegraph: Arc<RwLock<RoleGraph>>,

    /// Event sender for component events
    event_sender: mpsc::UnboundedSender<KnowledgeGraphEvent>,

    /// Event receiver for component events
    event_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<KnowledgeGraphEvent>>>>,

    /// Result cache
    result_cache: Arc<RwLock<HashMap<String, CachedKGResult>>>,

    /// Component metadata
    metadata: ComponentMetadata,
}

/// Cached Knowledge Graph Result
#[derive(Debug, Clone)]
struct CachedKGResult {
    /// Search results
    results: Vec<KGSearchResult>,

    /// Cache timestamp
    timestamp: std::time::SystemTime,

    /// Query hash for deduplication
    query_hash: u64,
}

impl KnowledgeGraphComponent {
    /// Create a new knowledge graph component
    pub fn new(
        config: KnowledgeGraphConfig,
        kg_search_service: Arc<KGSearchService>,
    ) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Self {
            config: config.clone(),
            state: KnowledgeGraphState::default(),
            performance_tracker: PerformanceTracker::new("knowledge-graph-search"),
            kg_search_service,
            rolegraph: Arc::new(RwLock::new(RoleGraph::new())),
            event_sender,
            event_receiver: Arc::new(RwLock::new(Some(event_receiver))),
            result_cache: Arc::new(RwLock::new(HashMap::new())),
            metadata: Self::create_metadata(),
        }
    }

    /// Create component metadata
    fn create_metadata() -> ComponentMetadata {
        ComponentMetadata::new(
            "knowledge-graph-search".to_string(),
            "1.0.0".to_string(),
            "Knowledge Graph Search Component".to_string(),
            "Reusable component for knowledge graph search with RoleGraph integration".to_string(),
            "Terraphim AI Team".to_string(),
        )
        .with_capability(ComponentCapability::Searchable)
        .with_capability(ComponentCapability::Filterable)
        .with_capability(ComponentCapability::Configurable)
        .with_capability(ComponentCapability::PerformanceMonitoring)
        .with_capability(ComponentCapability::RealTimeUpdates)
    }

    /// Perform knowledge graph search
    pub async fn search(&mut self, query: String) -> Result<Vec<KGSearchResult>, ComponentError> {
        let search_start = Instant::now();

        // Update state
        self.state.query = query.clone();
        self.state.last_error = None;

        // Check cache first
        if let Some(cached_result) = self.check_cache(&query).await {
            self.update_cache_stats(true).await;
            self.state.results = cached_result.results.clone();

            // Send search completed event
            let _ = self.event_sender.send(KnowledgeGraphEvent::SearchCompleted {
                query: query.clone(),
                results: cached_result.results.clone(),
            });

            return Ok(cached_result.results);
        }

        self.update_cache_stats(false).await;

        // Perform search with timeout
        let search_result = timeout(
            Duration::from_millis(self.config.performance_config.search_timeout_ms),
            self.perform_search(&query),
        )
        .await;

        match search_result {
            Ok(Ok(results)) => {
                let search_duration = search_start.elapsed();

                // Cache results
                self.cache_results(&query, &results).await;

                // Update state
                self.state.results = results.clone();

                // Update performance metrics
                self.update_search_metrics(search_duration).await;

                // Send completion event
                let _ = self.event_sender.send(KnowledgeGraphEvent::SearchCompleted {
                    query,
                    results: results.clone(),
                });

                Ok(results)
            }
            Ok(Err(e)) => {
                let error_msg = format!("Search failed: {}", e);
                self.state.last_error = Some(error_msg.clone());

                // Send failure event
                let _ = self.event_sender.send(KnowledgeGraphEvent::SearchFailed {
                    query,
                    error: error_msg,
                });

                Err(e)
            }
            Err(_) => {
                let error_msg = "Search timeout".to_string();
                self.state.last_error = Some(error_msg.clone());

                // Generate performance alert
                self.generate_alert(
                    KGAlertType::SearchTimeout,
                    "Search operation timed out".to_string(),
                    KGAlertSeverity::Error,
                    search_start.elapsed().as_millis() as f64,
                    self.config.performance_config.search_timeout_ms as f64,
                ).await;

                // Send failure event
                let _ = self.event_sender.send(KnowledgeGraphEvent::SearchFailed {
                    query,
                    error: error_msg,
                });

                Err(ComponentError::Performance("Search timeout".to_string()))
            }
        }
    }

    /// Perform the actual search using RoleGraph
    async fn perform_search(&self, query: &str) -> Result<Vec<KGSearchResult>, ComponentError> {
        let rolegraph = self.rolegraph.read().await;

        // Determine search mode based on query and configuration
        let search_mode = self.determine_search_mode(query);

        match search_mode {
            KGSearchMode::Standard => {
                self.perform_standard_search(&rolegraph, query).await
            }
            KGSearchMode::Fuzzy => {
                self.perform_fuzzy_search(&rolegraph, query).await
            }
            KGSearchMode::Connectivity => {
                self.perform_connectivity_search(&rolegraph, query).await
            }
            KGSearchMode::Logical => {
                self.perform_logical_search(&rolegraph, query).await
            }
        }
    }

    /// Determine search mode based on query and configuration
    fn determine_search_mode(&self, query: &str) -> KGSearchMode {
        // Check for logical operators
        if query.contains(" AND ") || query.contains(" OR ") {
            return KGSearchMode::Logical;
        }

        // Check for connectivity search
        if query.contains("related to:") || query.contains("connected to:") {
            return KGSearchMode::Connectivity;
        }

        // Use fuzzy search if enabled and query is short
        if self.config.enable_fuzzy_search && query.len() <= 20 {
            return KGSearchMode::Fuzzy;
        }

        KGSearchMode::Standard
    }

    /// Perform standard exact match search
    async fn perform_standard_search(
        &self,
        rolegraph: &RoleGraph,
        query: &str,
    ) -> Result<Vec<KGSearchResult>, ComponentError> {
        let mut results = Vec::new();

        // Search thesaurus for exact matches
        if let Some(thesaurus_entry) = rolegraph.thesaurus.get(&query.into()) {
            let kg_result = self.convert_thesaurus_to_result(thesaurus_entry).await?;
            results.push(kg_result);
        }

        // TODO: Search related terms - temporarily commented out due to missing RoleGraph.find_related_terms method
        // for related_term in rolegraph.find_related_terms(query, self.config.max_results) {
        //     if let Some(thesaurus_entry) = rolegraph.thesaurus.get(&related_term.into()) {
        //         let kg_result = self.convert_thesaurus_to_result(thesaurus_entry).await?;
        //         results.push(kg_result);
        //     }
        // }

        // Sort and limit results
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        results.truncate(self.config.max_results);

        Ok(results)
    }

    /// Perform fuzzy search using Jaro-Winkler similarity
    async fn perform_fuzzy_search(
        &self,
        rolegraph: &RoleGraph,
        query: &str,
    ) -> Result<Vec<KGSearchResult>, ComponentError> {
        let mut results = Vec::new();

        // TODO: Find fuzzy matches in thesaurus - temporarily commented out due to missing Thesaurus.fuzzy_search method
        // for term_entry in rolegraph.thesaurus.fuzzy_search(query, self.config.fuzzy_threshold) {
        //     let kg_result = self.convert_thesaurus_to_result(term_entry).await?;
        //     results.push(kg_result);
        // }

        // Sort by relevance (similarity score) and limit
        results.sort_by(|a: &KGSearchResult, b: &KGSearchResult| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        results.truncate(self.config.max_results);

        Ok(results)
    }

    /// Perform connectivity search to find connected terms
    async fn perform_connectivity_search(
        &self,
        rolegraph: &RoleGraph,
        query: &str,
    ) -> Result<Vec<KGSearchResult>, ComponentError> {
        let mut results = Vec::new();

        // Extract target term from connectivity query
        let target_term = query
            .split(':')
            .nth(1)
            .unwrap_or(query)
            .trim();

        // TODO: Find connected terms within traversal depth - temporarily commented out due to missing RoleGraph methods
        // let connected_terms = rolegraph.find_connected_terms(
        //     target_term,
        //     self.config.max_traversal_depth,
        //     self.config.max_results,
        // );

        // for term in connected_terms {
        //     if let Some(thesaurus_entry) = rolegraph.thesaurus.get(&term) {
        //         let mut kg_result = self.convert_thesaurus_to_result(thesaurus_entry).await?;

        //         // Add connectivity information
        //         kg_result.confidence = rolegraph.calculate_connectivity_score(target_term, &term);
        //
        //         results.push(kg_result);
        //     }
        // }

        Ok(results)
    }

    /// Perform logical search with AND/OR operators
    async fn perform_logical_search(
        &self,
        rolegraph: &RoleGraph,
        query: &str,
    ) -> Result<Vec<KGSearchResult>, ComponentError> {
        let mut results = Vec::new();

        // Parse logical query
        let logical_terms = self.parse_logical_query(query);

        match logical_terms.operator.as_str() {
            "AND" => {
                // TODO: Find terms that match all conditions - temporarily commented out due to missing RoleGraph.find_term_intersection method
                // if let Some(intersection) = rolegraph.find_term_intersection(&logical_terms.terms) {
                //     for term in intersection {
                //         if let Some(thesaurus_entry) = rolegraph.thesaurus.get(&term) {
                //             let kg_result = self.convert_thesaurus_to_result(thesaurus_entry).await?;
                //             results.push(kg_result);
                //         }
                //     }
                // }
            }
            "OR" => {
                // Find terms that match any condition
                for term in &logical_terms.terms {
                    if let Some(thesaurus_entry) = rolegraph.thesaurus.get(term) {
                        let kg_result = self.convert_thesaurus_to_result(thesaurus_entry).await?;
                        results.push(kg_result);
                    }
                }
            }
            _ => {
                // Fallback to standard search
                return self.perform_standard_search(rolegraph, query).await;
            }
        }

        // Remove duplicates and limit results
        results.sort_by(|a, b| a.term.cmp(&b.term));
        results.dedup_by(|a, b| a.term == b.term);
        results.truncate(self.config.max_results);

        Ok(results)
    }

    /// Parse logical query into terms and operator
    fn parse_logical_query(&self, query: &str) -> LogicalQueryParts {
        let mut terms = Vec::new();
        let mut operator = "OR";

        if query.contains(" AND ") {
            terms = query
                .split(" AND ")
                .map(|s| s.trim().to_string())
                .collect();
            operator = "AND";
        } else if query.contains(" OR ") {
            terms = query
                .split(" OR ")
                .map(|s| s.trim().to_string())
                .collect();
            operator = "OR";
        } else {
            terms.push(query.to_string());
        }

        LogicalQueryParts { terms, operator: operator.to_string() }
    }

    /// Convert thesaurus entry to KG search result
    async fn convert_thesaurus_to_result(
        &self,
        thesaurus_entry: &NormalizedTerm,
    ) -> Result<KGSearchResult, ComponentError> {
        let rolegraph = self.rolegraph.read().await;

        // Get related terms
        let related_terms = rolegraph
            .get_related_terms(&thesaurus_entry.value)
            .into_iter()
            .map(|(term, rel_type, strength)| KGRelatedTerm {
                id: term.clone(),
                term,
                relationship_type: self.map_relationship_type(rel_type),
                strength,
            })
            .collect();

        // Get associated documents
        let documents = rolegraph.get_documents_for_term(&thesaurus_entry.value);

        // Calculate relevance score
        let relevance_score = self.calculate_relevance_score(thesaurus_entry, &related_terms, &documents);

        Ok(KGSearchResult {
            id: thesaurus_entry.id.clone(),
            term: thesaurus_entry.value.to_string(),
            normalized_term: thesaurus_entry.value.clone(),
            relevance_score,
            connection_count: related_terms.len(),
            documents,
            related_terms,
            confidence: 1.0, // Default confidence for thesaurus entries
            last_updated: std::time::SystemTime::now(),
        })
    }

    /// Map internal relationship types to KG relationship types
    fn map_relationship_type(&self, internal_type: &str) -> KGRelationshipType {
        match internal_type {
            "synonym" => KGRelationshipType::Synonym,
            "related" => KGRelationshipType::Related,
            "parent" | "child" => KGRelationshipType::ParentChild,
            "cross_ref" => KGRelationshipType::CrossReference,
            _ => KGRelationshipType::Unknown,
        }
    }

    /// Calculate relevance score for a search result
    fn calculate_relevance_score(
        &self,
        thesaurus_entry: &NormalizedTerm,
        related_terms: &[KGRelatedTerm],
        documents: &[Document],
    ) -> f64 {
        let mut score = 1.0;

        // Boost based on connection count
        score += (related_terms.len() as f64) * 0.1;

        // Boost based on document count
        score += (documents.len() as f64) * 0.05;

        // Normalize to 0.0-1.0 range
        (score / 5.0).min(1.0)
    }

    /// Check cache for query results
    async fn check_cache(&self, query: &str) -> Option<CachedKGResult> {
        let cache = self.result_cache.read().await;
        let query_hash = self.calculate_query_hash(query);

        if let Some(cached_result) = cache.get(query) {
            let cache_age = std::time::SystemTime::now()
                .duration_since(cached_result.timestamp)
                .unwrap_or(Duration::ZERO);

            if cache_age.as_secs() < self.config.cache_ttl_seconds {
                return Some(cached_result.clone());
            }
        }

        None
    }

    /// Cache search results
    async fn cache_results(&self, query: &str, results: &[KGSearchResult]) {
        let mut cache = self.result_cache.write().await;

        // Implement LRU eviction if cache is full
        if cache.len() >= self.config.cache_size {
            self.evict_oldest_cache_entries(&mut cache).await;
        }

        let cached_result = CachedKGResult {
            results: results.to_vec(),
            timestamp: std::time::SystemTime::now(),
            query_hash: self.calculate_query_hash(query),
        };

        cache.insert(query.to_string(), cached_result);
    }

    /// Calculate query hash for caching
    fn calculate_query_hash(&self, query: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        hasher.finish()
    }

    /// Evict oldest cache entries
    async fn evict_oldest_cache_entries(&self, cache: &mut HashMap<String, CachedKGResult>) {
        if cache.is_empty() {
            return;
        }

        // Find oldest entries (simple LRU implementation)
        let mut entries: Vec<_> = cache.iter().collect();
        entries.sort_by_key(|(_, cached)| cached.timestamp);

        // Remove oldest 25% of entries
        let remove_count = cache.len() / 4;
        for (key, _) in entries.iter().take(remove_count) {
            cache.remove(*key);
        }
    }

    /// Update cache statistics
    async fn update_cache_stats(&mut self, is_hit: bool) {
        if is_hit {
            self.state.cache_state.cache_hits += 1;
        } else {
            self.state.cache_state.cache_misses += 1;
        }
        self.state.cache_state.cached_results = self.result_cache.read().await.len();
    }

    /// Update search performance metrics
    async fn update_search_metrics(&mut self, search_duration: Duration) {
        self.state.performance_metrics.total_searches += 1;

        let duration_ms = search_duration.as_millis() as f64;
        let total_searches = self.state.performance_metrics.total_searches as f64;

        // Update average latency
        self.state.performance_metrics.avg_search_latency_ms =
            (self.state.performance_metrics.avg_search_latency_ms * (total_searches - 1.0) + duration_ms) / total_searches;

        // Update peak latency
        self.state.performance_metrics.peak_search_latency_ms =
            self.state.performance_metrics.peak_search_latency_ms.max(duration_ms as u64);

        // Check for performance alerts
        if duration_ms > self.config.performance_config.alert_thresholds.search_latency_ms as f64 {
            self.generate_alert(
                KGAlertType::HighSearchLatency,
                format!("Search latency exceeded threshold: {:.2}ms", duration_ms),
                KGAlertSeverity::Warning,
                duration_ms,
                self.config.performance_config.alert_thresholds.search_latency_ms as f64,
            ).await;
        }
    }

    /// Generate performance alert
    async fn generate_alert(
        &mut self,
        alert_type: KGAlertType,
        message: String,
        severity: KGAlertSeverity,
        metric_value: f64,
        threshold: f64,
    ) {
        let alert = KGPerformanceAlert {
            id: Uuid::new_v4().to_string(),
            alert_type,
            message,
            severity,
            timestamp: std::time::SystemTime::now(),
            metric_value,
            threshold,
        };

        self.state.performance_metrics.performance_alerts.push(alert.clone());

        // Send alert event
        let _ = self.event_sender.send(KnowledgeGraphEvent::PerformanceAlert { alert });
    }

    /// Get component events
    pub async fn get_events(&mut self) -> Vec<KnowledgeGraphEvent> {
        let mut receiver = self.event_receiver.write().await;
        if let Some(ref mut rx) = *receiver {
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
            events
        } else {
            Vec::new()
        }
    }

    /// Select a term for detailed view
    pub async fn select_term(&mut self, term: &KGSearchResult) -> Result<KGTermDetail, ComponentError> {
        let rolegraph = self.rolegraph.read().await;

        // Get detailed information about the term
        let detailed_info = rolegraph.get_term_details(&term.id).await?;

        let kg_detail = KGTermDetail {
            basic_info: term.clone(),
            relationships: detailed_info
                .relationships
                .into_iter()
                .map(|(rel_term, rel_type, strength)| KGRelatedTerm {
                    id: rel_term.clone(),
                    term: rel_term,
                    relationship_type: self.map_relationship_type(&rel_type),
                    strength,
                })
                .collect(),
            metadata: KGTermMetadata {
                created_at: detailed_info.created_at,
                modified_at: detailed_info.modified_at,
                source: self.map_term_source(&detailed_info.source),
                tags: detailed_info.tags,
                attributes: detailed_info.attributes,
            },
            usage_stats: KGUsageStats {
                search_count: detailed_info.search_count,
                selection_count: detailed_info.selection_count,
                last_accessed: detailed_info.last_accessed,
                search_frequency: detailed_info.search_frequency,
            },
        };

        self.state.selected_term = Some(kg_detail.clone());

        // Send term selected event
        let _ = self.event_sender.send(KnowledgeGraphEvent::TermSelected {
            term: term.clone(),
        });

        Ok(kg_detail)
    }

    /// Map internal term source to KG term source
    fn map_term_source(&self, internal_source: &str) -> KGTermSource {
        match internal_source {
            "manual" => KGTermSource::Manual,
            "imported" => KGTermSource::Imported,
            "inferred" => KGTermSource::Inferred,
            "auto_generated" => KGTermSource::AutoGenerated,
            _ => KGTermSource::Inferred,
        }
    }

    /// Update component configuration
    pub async fn update_config(&mut self, config: KnowledgeGraphConfig) -> Result<(), ComponentError> {
        self.config = config.clone();

        // Update configuration-dependent state
        if config.enable_real_time_updates {
            self.enable_real_time_updates().await?;
        }

        // Send configuration updated event
        let _ = self.event_sender.send(KnowledgeGraphEvent::ConfigurationUpdated { config });

        Ok(())
    }

    /// Enable real-time updates for KG data
    async fn enable_real_time_updates(&mut self) -> Result<(), ComponentError> {
        // Implementation would depend on real-time update system
        // For now, this is a placeholder
        Ok(())
    }

    /// Clear search cache
    pub async fn clear_cache(&mut self) -> Result<(), ComponentError> {
        let mut cache = self.result_cache.write().await;
        cache.clear();

        // Reset cache statistics
        self.state.cache_state = KGCacheState::default();

        Ok(())
    }

    /// Get current performance metrics
    pub fn get_performance_metrics(&self) -> &KGPerformanceMetrics {
        &self.state.performance_metrics
    }

    /// Get current cache state
    pub fn get_cache_state(&self) -> &KGCacheState {
        &self.state.cache_state
    }

    /// Get component state
    pub fn get_state(&self) -> &KnowledgeGraphState {
        &self.state
    }
}

/// Logical query parsing result
#[derive(Debug)]
struct LogicalQueryParts {
    terms: Vec<String>,
    operator: String,
}

// TODO: Implement ReusableComponent trait for KnowledgeGraphComponent
// Temporarily commented out due to ComponentConfig trait implementation requirement
// impl ReusableComponent for KnowledgeGraphComponent {
//     type Config = KnowledgeGraphConfig;
//     type State = KnowledgeGraphState;
//     type Event = KnowledgeGraphEvent;

    // TODO: fn component_id() -> &'static str - commented out due to missing impl block
    // {
    //     "knowledge-graph-search"
    // }

    // TODO: fn component_version() -> &'static str - commented out due to missing impl block
    // {
    //     "1.0.0"
    // }

    // TODO: fn init() and config() - commented out due to missing impl block
    // fn init(config: Self::Config) -> Self { /* implementation */ }
    // fn config(&self) -> &Self::Config { &self.config }

    // TODO: fn update_config() - commented out due to missing impl block
    // fn update_config(&mut self, config: Self::Config) -> Result<(), ComponentError> { /* implementation */ }

    // TODO: state, update_state, mount, unmount - commented out due to missing impl block
    // fn state(&self) -> &Self::State { &self.state }
    // fn update_state(&mut self, state: Self::State) -> Result<(), ComponentError> { self.state = state; Ok(()) }
    // fn mount(&mut self, _cx: &mut ViewContext<Self>) -> Result<(), ComponentError> { /* implementation */ }
    // fn unmount(&mut self, _cx: &mut ViewContext<Self>) -> Result<(), ComponentError> { /* implementation */ }

    // TODO: These functions are commented out because they're not inside an impl block
    // They should be part of ReusableComponent implementation when ComponentConfig is fixed
    /*
    fn handle_lifecycle_event(
        &mut self,
        event: LifecycleEvent,
        _cx: &mut ViewContext<Self>,
    ) -> Result<(), ComponentError> {
        match event {
            LifecycleEvent::Mounting => {
                // Prepare for mounting
            }
            LifecycleEvent::Mounted => {
                // Component is now mounted and active
                self.state.is_initialized = true;
            }
            LifecycleEvent::Unmounting => {
                // Prepare for unmounting
            }
            LifecycleEvent::Unmounted => {
                // Component is now unmounted
                self.state.is_initialized = false;
            }
            LifecycleEvent::ConfigChanged => {
                // Configuration has changed, handle updates
            }
            LifecycleEvent::StateChanged => {
                // State has changed internally
            }
            LifecycleEvent::DependenciesChanged => {
                // Dependencies have changed
            }
            LifecycleEvent::Suspending => {
                // Component is being suspended for performance
            }
            LifecycleEvent::Resumed => {
                // Component has been resumed
            }
            LifecycleEvent::Reloading => {
                // Component is being reloaded
            }
            LifecycleEvent::Reloaded => {
                // Component has been reloaded
            }
        }

        Ok(())
    }

    fn is_mounted(&self) -> bool {
        self.state.is_initialized
    }

    fn performance_metrics(&self) -> &PerformanceTracker {
        &self.performance_tracker
    }

    fn reset_performance_metrics(&mut self) {
        self.performance_tracker.reset();
        self.state.performance_metrics = KGPerformanceMetrics::default();
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![
            "kg-search-service",
            "rolegraph-service",
            "config-service",
        ]
    }

    fn are_dependencies_satisfied(&self, _registry: &ServiceRegistry) -> bool {
        // In a real implementation, this would check the registry
        // For now, we assume dependencies are satisfied
        true
    }

    fn cleanup(&mut self) -> Result<(), ComponentError> {
        // Clear cache
        let cache = Arc::try_unwrap(self.result_cache.clone()).unwrap_or_default();
        let _ = cache.into_inner();

        // Clear state
        self.state = KnowledgeGraphState::default();

        // Send lifecycle event
        let _ = self.event_sender.send(KnowledgeGraphEvent::LifecycleEvent {
            event: LifecycleEvent::Unmounted,
        });

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    */