/// Knowledge Graph-Enhanced Autocomplete Component
///
/// This module provides a specialized autocomplete component that integrates
/// with knowledge graph data sources, providing intelligent term suggestions
/// based on semantic relationships, usage patterns, and graph connectivity.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::*;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use ulid::Ulid;

use crate::components::{
    ComponentConfig, ComponentError, LifecycleEvent, PerformanceTracker,
    ReusableComponent, ServiceRegistry, ViewContext,
    search_services::{SearchService, AutocompleteService}
};
use crate::autocomplete::{AutocompleteEngine, AutocompleteSuggestion};
use terraphim_rolegraph::RoleGraph;
use terraphim_types::RoleName;
use crate::search_service::SearchService as MainSearchService;
use terraphim_automata::{build_autocomplete_index, fuzzy_autocomplete_search};
use terraphim_types::Thesaurus;

/// Knowledge Graph Autocomplete Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KGAutocompleteConfig {
    /// Role name for KG data isolation
    pub role: RoleName,

    /// Maximum number of suggestions to return
    pub max_suggestions: usize,

    /// Minimum characters to trigger autocomplete
    pub min_chars: usize,

    /// Enable fuzzy search for typos
    pub enable_fuzzy_search: bool,

    /// Fuzzy search similarity threshold (0.0-1.0)
    pub fuzzy_threshold: f64,

    /// Enable semantic suggestions based on graph relationships
    pub enable_semantic_suggestions: bool,

    /// Enable usage pattern-based suggestions
    pub enable_usage_pattern_suggestions: bool,

    /// Enable term popularity ranking
    pub enable_popularity_ranking: bool,

    /// Cache size for suggestions
    pub cache_size: usize,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,

    /// Debounce time for suggestions (milliseconds)
    pub debounce_ms: u64,

    /// Performance monitoring configuration
    pub performance_config: KGAutocompletePerformanceConfig,

    /// Suggestion ranking configuration
    pub ranking_config: KGAutocompleteRankingConfig,

    /// UI behavior configuration
    pub ui_config: KGAutocompleteUIConfig,
}

impl Default for KGAutocompleteConfig {
    fn default() -> Self {
        Self {
            role: RoleName::from("default"),
            max_suggestions: 8,
            min_chars: 2,
            enable_fuzzy_search: true,
            fuzzy_threshold: 0.7,
            enable_semantic_suggestions: true,
            enable_usage_pattern_suggestions: true,
            enable_popularity_ranking: true,
            cache_size: 500,
            cache_ttl_seconds: 600, // 10 minutes
            debounce_ms: 150,
            performance_config: KGAutocompletePerformanceConfig::default(),
            ranking_config: KGAutocompleteRankingConfig::default(),
            ui_config: KGAutocompleteUIConfig::default(),
        }
    }
}

/// Knowledge Graph Autocomplete Performance Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KGAutocompletePerformanceConfig {
    /// Enable performance monitoring
    pub enable_monitoring: bool,

    /// Suggestion timeout in milliseconds
    pub suggestion_timeout_ms: u64,

    /// Index rebuild timeout in milliseconds
    pub index_rebuild_timeout_ms: u64,

    /// Performance alert thresholds
    pub alert_thresholds: KGAutocompletePerformanceThresholds,
}

impl Default for KGAutocompletePerformanceConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            suggestion_timeout_ms: 1000, // 1 second
            index_rebuild_timeout_ms: 5000, // 5 seconds
            alert_thresholds: KGAutocompletePerformanceThresholds::default(),
        }
    }
}

/// Knowledge Graph Autocomplete Performance Thresholds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KGAutocompletePerformanceThresholds {
    /// Suggestion latency threshold in milliseconds
    pub suggestion_latency_ms: u64,

    /// Index rebuild time threshold in milliseconds
    pub index_rebuild_time_ms: u64,

    /// Memory usage threshold in MB
    pub memory_usage_mb: usize,

    /// Cache hit ratio threshold (0.0-1.0)
    pub cache_hit_ratio_threshold: f64,
}

impl Default for KGAutocompletePerformanceThresholds {
    fn default() -> Self {
        Self {
            suggestion_latency_ms: 200, // 200ms
            index_rebuild_time_ms: 2000, // 2 seconds
            memory_usage_mb: 50, // 50MB
            cache_hit_ratio_threshold: 0.7, // 70%
        }
    }
}

/// Knowledge Graph Autocomplete Ranking Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KGAutocompleteRankingConfig {
    /// Weight for exact matches (0.0-1.0)
    pub exact_match_weight: f64,

    /// Weight for fuzzy matches (0.0-1.0)
    pub fuzzy_match_weight: f64,

    /// Weight for semantic relationships (0.0-1.0)
    pub semantic_weight: f64,

    /// Weight for usage patterns (0.0-1.0)
    pub usage_pattern_weight: f64,

    /// Weight for term popularity (0.0-1.0)
    pub popularity_weight: f64,

    /// Weight for recent usage (0.0-1.0)
    pub recent_usage_weight: f64,

    /// Enable context-aware ranking
    pub enable_context_aware_ranking: bool,
}

impl Default for KGAutocompleteRankingConfig {
    fn default() -> Self {
        Self {
            exact_match_weight: 1.0,
            fuzzy_match_weight: 0.8,
            semantic_weight: 0.6,
            usage_pattern_weight: 0.4,
            popularity_weight: 0.3,
            recent_usage_weight: 0.5,
            enable_context_aware_ranking: true,
        }
    }
}

/// Knowledge Graph Autocomplete UI Configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KGAutocompleteUIConfig {
    /// Show suggestion categories
    pub show_categories: bool,

    /// Show suggestion confidence scores
    pub show_confidence: bool,

    /// Show relationship indicators
    pub show_relationships: bool,

    /// Show usage statistics
    pub show_usage_stats: bool,

    /// Enable keyboard shortcuts
    pub enable_keyboard_shortcuts: bool,

    /// Highlight matching parts in suggestions
    pub highlight_matches: bool,

    /// Group suggestions by type
    pub group_by_type: bool,

    /// Maximum suggestions per category
    pub max_per_category: usize,
}

impl Default for KGAutocompleteUIConfig {
    fn default() -> Self {
        Self {
            show_categories: true,
            show_confidence: false,
            show_relationships: true,
            show_usage_stats: false,
            enable_keyboard_shortcuts: true,
            highlight_matches: true,
            group_by_type: true,
            max_per_category: 5,
        }
    }
}

/// Knowledge Graph Autocomplete State
#[derive(Debug, Clone)]
pub struct KGAutocompleteState {
    /// Current query
    pub query: String,

    /// Current suggestions
    pub suggestions: Vec<KGAutocompleteSuggestion>,

    /// Selected suggestion index
    pub selected_index: Option<usize>,

    /// Autocomplete mode
    pub mode: KGAutocompleteMode,

    /// Loading state
    pub is_loading: bool,

    /// Error state
    pub last_error: Option<String>,

    /// Performance metrics
    pub performance_metrics: KGAutocompletePerformanceMetrics,

    /// Cache state
    pub cache_state: KGAutocompleteCacheState,

    /// Component lifecycle status
    pub is_initialized: bool,

    /// Last query timestamp
    pub last_query_time: Option<Instant>,
}

impl Default for KGAutocompleteState {
    fn default() -> Self {
        Self {
            query: String::new(),
            suggestions: Vec::new(),
            selected_index: None,
            mode: KGAutocompleteMode::Standard,
            is_loading: false,
            last_error: None,
            performance_metrics: KGAutocompletePerformanceMetrics::default(),
            cache_state: KGAutocompleteCacheState::default(),
            is_initialized: false,
            last_query_time: None,
        }
    }
}

/// Knowledge Graph Autocomplete Mode
#[derive(Debug, Clone, PartialEq)]
pub enum KGAutocompleteMode {
    /// Standard prefix-based autocomplete
    Standard,
    /// Fuzzy search for typos
    Fuzzy,
    /// Semantic suggestions based on context
    Semantic,
    /// Hybrid approach combining multiple methods
    Hybrid,
}

/// Enhanced Autocomplete Suggestion with KG Integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGAutocompleteSuggestion {
    /// Basic suggestion information
    pub base_suggestion: AutocompleteSuggestion,

    /// Suggestion category/type
    pub category: KGAutocompleteCategory,

    /// Confidence score (0.0-1.0)
    pub confidence: f64,

    /// Relationship to query
    pub relationship: Option<KGSuggestionRelationship>,

    /// Usage statistics
    pub usage_stats: KGSuggestionUsageStats,

    /// Semantic context
    pub semantic_context: Option<KGSemanticContext>,

    /// Ranking score
    pub ranking_score: f64,

    /// Suggestion metadata
    pub metadata: KGSuggestionMetadata,
}

/// Knowledge Graph Autocomplete Suggestion Category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGAutocompleteCategory {
    /// Exact match from thesaurus
    ExactMatch,
    /// Fuzzy match for typos
    FuzzyMatch,
    /// Semantically related term
    SemanticRelated,
    /// Popular term based on usage
    Popular,
    /// Recently used term
    Recent,
    /// Domain-specific term
    DomainSpecific,
    /// Cross-reference from another domain
    CrossReference,
    /// Unknown category
    Unknown,
}

/// Knowledge Graph Suggestion Relationship to Query
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGSuggestionRelationship {
    /// Direct prefix match
    Prefix,
    /// Contains query as substring
    Substring,
    /// Semantic similarity
    SemanticSimilarity,
    /// Graph connectivity
    GraphConnected,
    /// Usage pattern correlation
    UsageCorrelation,
    /// No clear relationship
    None,
}

/// Knowledge Graph Suggestion Usage Statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGSuggestionUsageStats {
    /// Number of times this suggestion was selected
    pub selection_count: u64,

    /// Number of times this suggestion was shown
    pub impression_count: u64,

    /// Click-through rate
    pub click_through_rate: f64,

    /// Last selected timestamp
    pub last_selected: Option<std::time::SystemTime>,

    /// Selection frequency over time
    pub selection_frequency: Vec<(std::time::SystemTime, u64)>,
}

impl Default for KGSuggestionUsageStats {
    fn default() -> Self {
        Self {
            selection_count: 0,
            impression_count: 0,
            click_through_rate: 0.0,
            last_selected: None,
            selection_frequency: Vec::new(),
        }
    }
}

/// Knowledge Graph Semantic Context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGSemanticContext {
    /// Contextual domain
    pub domain: Option<String>,

    /// Related concepts
    pub related_concepts: Vec<String>,

    /// Context confidence
    pub confidence: f64,

    /// Context source
    pub source: KGSemanticContextSource,
}

/// Knowledge Graph Semantic Context Source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGSemanticContextSource {
    /// Derived from document analysis
    DocumentAnalysis,
    /// Inferred from usage patterns
    UsageInference,
    /// Based on graph relationships
    GraphRelationships,
    /// Manual annotation
    ManualAnnotation,
}

/// Knowledge Graph Suggestion Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGSuggestionMetadata {
    /// Suggestion creation timestamp
    pub created_at: std::time::SystemTime,

    /// Last updated timestamp
    pub updated_at: std::time::SystemTime,

    /// Suggestion source
    pub source: KGSuggestionSource,

    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

/// Knowledge Graph Suggestion Source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGSuggestionSource {
    /// From thesaurus
    Thesaurus,
    /// Derived from usage patterns
    UsagePattern,
    /// Inferred from graph relationships
    GraphInference,
    /// Manual addition
    Manual,
    /// External data source
    External,
}

/// Knowledge Graph Autocomplete Performance Metrics
#[derive(Debug, Clone, Default)]
pub struct KGAutocompletePerformanceMetrics {
    /// Total queries processed
    pub total_queries: u64,

    /// Average suggestion latency in milliseconds
    pub avg_suggestion_latency_ms: f64,

    /// Peak suggestion latency in milliseconds
    pub peak_suggestion_latency_ms: u64,

    /// Cache hit ratio
    pub cache_hit_ratio: f64,

    /// Number of cache hits
    pub cache_hits: u64,

    /// Number of cache misses
    pub cache_misses: u64,

    /// Average suggestions per query
    pub avg_suggestions_per_query: f64,

    /// Performance alerts
    pub performance_alerts: Vec<KGAutocompletePerformanceAlert>,
}

/// Knowledge Graph Autocomplete Performance Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KGAutocompletePerformanceAlert {
    /// Alert identifier
    pub id: String,

    /// Alert type
    pub alert_type: KGAutocompleteAlertType,

    /// Alert message
    pub message: String,

    /// Alert severity
    pub severity: crate::components::knowledge_graph::KGAlertSeverity,

    /// Alert timestamp
    pub timestamp: std::time::SystemTime,

    /// Associated metric value
    pub metric_value: f64,
}

/// Knowledge Graph Autocomplete Alert Type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KGAutocompleteAlertType {
    /// Suggestion latency too high
    HighSuggestionLatency,
    /// Index rebuild taking too long
    SlowIndexRebuild,
    /// Memory usage too high
    HighMemoryUsage,
    /// Cache hit ratio too low
    LowCacheHitRatio,
    /// Component initialization failure
    InitializationFailure,
    /// Suggestion timeout
    SuggestionTimeout,
}

/// Knowledge Graph Autocomplete Cache State
#[derive(Debug, Clone, Default)]
pub struct KGAutocompleteCacheState {
    /// Number of cached queries
    pub cached_queries: usize,

    /// Cache hit count
    pub cache_hits: u64,

    /// Cache miss count
    pub cache_misses: u64,

    /// Last cache cleanup timestamp
    pub last_cleanup: Option<std::time::SystemTime>,
}

/// Knowledge Graph Autocomplete Events
#[derive(Debug, Clone, PartialEq)]
pub enum KGAutocompleteEvent {
    /// Suggestions generated
    SuggestionsGenerated { query: String, suggestions: Vec<KGAutocompleteSuggestion> },

    /// Suggestion selected
    SuggestionSelected { suggestion: KGAutocompleteSuggestion, index: usize },

    /// Query changed
    QueryChanged { query: String },

    /// Cache state updated
    CacheUpdated { cache_state: KGAutocompleteCacheState },

    /// Performance alert
    PerformanceAlert { alert: KGAutocompletePerformanceAlert },

    /// Mode changed
    ModeChanged { mode: KGAutocompleteMode },

    /// Configuration updated
    ConfigurationUpdated,

    /// Component lifecycle event
    LifecycleEvent { event: LifecycleEvent },
}

/// Knowledge Graph-Enhanced Autocomplete Component
#[derive(Debug)]
pub struct KGAutocompleteComponent {
    /// Component configuration
    config: KGAutocompleteConfig,

    /// Component state
    state: KGAutocompleteState,

    /// Performance tracker
    performance_tracker: PerformanceTracker,

    /// RoleGraph instance for KG data
    rolegraph: Arc<RwLock<RoleGraph>>,

    /// Autocomplete engine for thesaurus integration
    autocomplete_engine: Arc<AutocompleteEngine>,

    /// Suggestion cache
    suggestion_cache: Arc<RwLock<HashMap<String, CachedKGSuggestions>>>,

    /// Event sender for component events
    event_sender: mpsc::UnboundedSender<KGAutocompleteEvent>,

    /// Event receiver for component events
    event_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<KGAutocompleteEvent>>>>,

    /// Usage statistics tracker
    usage_tracker: Arc<RwLock<KGUsageTracker>>,

    /// Component metadata
    metadata: crate::components::ComponentMetadata,
}

/// Cached Knowledge Graph Suggestions
#[derive(Debug, Clone)]
struct CachedKGSuggestions {
    /// Cached suggestions
    suggestions: Vec<KGAutocompleteSuggestion>,

    /// Cache timestamp
    timestamp: std::time::SystemTime,

    /// Query hash for deduplication
    query_hash: u64,
}

/// Knowledge Graph Usage Tracker
#[derive(Debug, Default)]
struct KGUsageTracker {
    /// Selection statistics per suggestion
    selection_stats: HashMap<String, KGSuggestionUsageStats>,

    /// Query patterns
    query_patterns: HashMap<String, u64>,

    /// Last cleanup timestamp
    pub last_cleanup: Option<std::time::SystemTime>,
}

impl KGAutocompleteComponent {
    /// Create a new knowledge graph-enhanced autocomplete component
    pub fn new(
        config: KGAutocompleteConfig,
        rolegraph: Arc<RwLock<RoleGraph>>,
        autocomplete_engine: Arc<AutocompleteEngine>,
    ) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Self {
            config: config.clone(),
            state: KGAutocompleteState::default(),
            performance_tracker: PerformanceTracker::new("kg-autocomplete"),
            rolegraph,
            autocomplete_engine,
            suggestion_cache: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            event_receiver: Arc::new(RwLock::new(Some(event_receiver))),
            usage_tracker: Arc::new(RwLock::new(KGUsageTracker::default())),
            metadata: Self::create_metadata(),
        }
    }

    /// Create component metadata
    fn create_metadata() -> crate::components::ComponentMetadata {
        crate::components::ComponentMetadata::new(
            "kg-autocomplete".to_string(),
            "1.0.0".to_string(),
            "Knowledge Graph Autocomplete Component".to_string(),
            "Autocomplete component with knowledge graph integration and semantic suggestions".to_string(),
            "Terraphim AI Team".to_string(),
        )
        .with_capability(crate::components::ComponentCapability::Searchable)
        .with_capability(crate::components::ComponentCapability::Filterable)
        .with_capability(crate::components::ComponentCapability::KeyboardNavigable)
        .with_capability(crate::components::ComponentCapability::PerformanceMonitoring)
    }

    /// Generate suggestions for the given query
    pub async fn generate_suggestions(&mut self, query: String) -> Result<Vec<KGAutocompleteSuggestion>, ComponentError> {
        let suggestion_start = Instant::now();

        // Update state
        self.state.query = query.clone();
        self.state.last_query_time = Some(suggestion_start);
        self.state.last_error = None;

        // Check if query meets minimum length
        if query.len() < self.config.min_chars {
            self.state.suggestions.clear();
            self.state.selected_index = None;
            return Ok(Vec::new());
        }

        // Check cache first
        if let Some(cached_suggestions) = self.check_cache(&query).await {
            self.update_cache_stats(true).await;
            self.state.suggestions = cached_suggestions.suggestions.clone();

            // Send suggestions generated event
            let _ = self.event_sender.send(KGAutocompleteEvent::SuggestionsGenerated {
                query,
                suggestions: cached_suggestions.suggestions.clone(),
            });

            return Ok(cached_suggestions.suggestions);
        }

        self.update_cache_stats(false).await;

        // Set loading state
        self.state.is_loading = true;

        // Perform suggestion generation with timeout
        let suggestion_result = tokio::time::timeout(
            Duration::from_millis(self.config.performance_config.suggestion_timeout_ms),
            self.perform_suggestion_generation(&query).await,
        )
        .await;

        match suggestion_result {
            Ok(Ok(mut suggestions)) => {
                let suggestion_duration = suggestion_start.elapsed();

                // Update usage statistics and rankings
                self.update_suggestion_rankings(&mut suggestions).await;

                // Cache suggestions
                self.cache_suggestions(&query, &suggestions).await;

                // Update state
                self.state.suggestions = suggestions.clone();
                self.state.is_loading = false;

                // Update performance metrics
                self.update_performance_metrics(suggestion_duration, suggestions.len()).await;

                // Send completion event
                let _ = self.event_sender.send(KGAutocompleteEvent::SuggestionsGenerated {
                    query,
                    suggestions: suggestions.clone(),
                });

                Ok(suggestions)
            }
            Ok(Err(e)) => {
                let error_msg = format!("Suggestion generation failed: {}", e);
                self.state.last_error = Some(error_msg.clone());
                self.state.is_loading = false;

                // Generate performance alert
                self.generate_alert(
                    KGAutocompleteAlertType::InitializationFailure,
                    error_msg.clone(),
                    crate::components::knowledge_graph::KGAlertSeverity::Error,
                    suggestion_start.elapsed().as_millis() as f64,
                    self.config.performance_config.suggestion_timeout_ms as f64,
                ).await;

                Err(e)
            }
            Err(_) => {
                let error_msg = "Suggestion generation timeout".to_string();
                self.state.last_error = Some(error_msg.clone());
                self.state.is_loading = false;

                // Generate performance alert
                self.generate_alert(
                    KGAutocompleteAlertType::SuggestionTimeout,
                    error_msg.clone(),
                    crate::components::knowledge_graph::KGAlertSeverity::Error,
                    suggestion_start.elapsed().as_millis() as f64,
                    self.config.performance_config.suggestion_timeout_ms as f64,
                ).await;

                Err(ComponentError::Performance("Suggestion generation timeout".to_string()))
            }
        }
    }

    /// Perform the actual suggestion generation
    async fn perform_suggestion_generation(&self, query: &str) -> Result<Vec<KGAutocompleteSuggestion>, ComponentError> {
        let mut suggestions = Vec::new();

        // Determine suggestion mode based on query and configuration
        let mode = self.determine_suggestion_mode(query);

        match mode {
            KGAutocompleteMode::Standard => {
                suggestions.extend(self.generate_standard_suggestions(query).await?);
            }
            KGAutocompleteMode::Fuzzy => {
                suggestions.extend(self.generate_fuzzy_suggestions(query).await?);
            }
            KGAutocompleteMode::Semantic => {
                suggestions.extend(self.generate_semantic_suggestions(query).await?);
            }
            KGAutocompleteMode::Hybrid => {
                suggestions.extend(self.generate_hybrid_suggestions(query).await?);
            }
        }

        // Sort and limit suggestions
        suggestions.sort_by(|a, b| b.ranking_score.partial_cmp(&a.ranking_score).unwrap());
        suggestions.truncate(self.config.max_suggestions);

        Ok(suggestions)
    }

    /// Determine suggestion mode based on query and configuration
    fn determine_suggestion_mode(&self, query: &str) -> KGAutocompleteMode {
        // For short queries, use fuzzy search
        if query.len() <= 5 && self.config.enable_fuzzy_search {
            return KGAutocompleteMode::Fuzzy;
        }

        // If semantic suggestions are enabled and query is complex, use semantic mode
        if self.config.enable_semantic_suggestions && query.len() > 10 {
            return KGAutocompleteMode::Semantic;
        }

        // Use hybrid mode for comprehensive suggestions
        if self.config.enable_semantic_suggestions && self.config.enable_fuzzy_search {
            return KGAutocompleteMode::Hybrid;
        }

        KGAutocompleteMode::Standard
    }

    /// Generate standard prefix-based suggestions
    async fn generate_standard_suggestions(&self, query: &str) -> Result<Vec<KGAutocompleteSuggestion>, ComponentError> {
        let mut suggestions = Vec::new();

        // Get suggestions from autocomplete engine
        let autocomplete_suggestions = self.autocomplete_engine.suggest(query).await?;

        for suggestion in autocomplete_suggestions {
            let kg_suggestion = self.convert_to_kg_suggestion(
                suggestion,
                KGAutocompleteCategory::ExactMatch,
                KGSuggestionRelationship::Prefix,
            ).await?;
            suggestions.push(kg_suggestion);
        }

        Ok(suggestions)
    }

    /// Generate fuzzy search suggestions
    async fn generate_fuzzy_suggestions(&self, query: &str) -> Result<Vec<KGAutocompleteSuggestion>, ComponentError> {
        let mut suggestions = Vec::new();

        let rolegraph = self.rolegraph.read().await;

        // Get fuzzy matches from thesaurus
        for term_entry in rolegraph.thesaurus.fuzzy_search(query, self.config.fuzzy_threshold) {
            let base_suggestion = AutocompleteSuggestion {
                term: term_entry.term.clone(),
                score: term_entry.relevance.unwrap_or(1.0),
                source: "thesaurus".to_string(),
            };

            let kg_suggestion = self.convert_to_kg_suggestion(
                base_suggestion,
                KGAutocompleteCategory::FuzzyMatch,
                KGSuggestionRelationship::SemanticSimilarity,
            ).await?;
            suggestions.push(kg_suggestion);
        }

        Ok(suggestions)
    }

    /// Generate semantic suggestions based on graph relationships
    async fn generate_semantic_suggestions(&self, query: &str) -> Result<Vec<KGAutocompleteSuggestion>, ComponentError> {
        let mut suggestions = Vec::new();

        let rolegraph = self.rolegraph.read().await;

        // Find semantically related terms
        if let Some(thesaurus_entry) = rolegraph.thesaurus.find_term(query) {
            let related_terms = rolegraph.get_related_terms(&thesaurus_entry.term);

            for (related_term, _relationship_type, _strength) in related_terms {
                let base_suggestion = AutocompleteSuggestion {
                    term: related_term.clone(),
                    score: 0.8, // Default score for semantic suggestions
                    source: "graph-relationship".to_string(),
                };

                let kg_suggestion = self.convert_to_kg_suggestion(
                    base_suggestion,
                    KGAutocompleteCategory::SemanticRelated,
                    KGSuggestionRelationship::GraphConnected,
                ).await?;
                suggestions.push(kg_suggestion);
            }
        }

        Ok(suggestions)
    }

    /// Generate hybrid suggestions combining multiple methods
    async fn generate_hybrid_suggestions(&self, query: &str) -> Result<Vec<KGAutocompleteSuggestion>, ComponentError> {
        let mut all_suggestions = Vec::new();

        // Combine suggestions from all methods
        if let Ok(standard) = self.generate_standard_suggestions(query).await {
            all_suggestions.extend(standard);
        }

        if self.config.enable_fuzzy_search {
            if let Ok(fuzzy) = self.generate_fuzzy_suggestions(query).await {
                all_suggestions.extend(fuzzy);
            }
        }

        if self.config.enable_semantic_suggestions {
            if let Ok(semantic) = self.generate_semantic_suggestions(query).await {
                all_suggestions.extend(semantic);
            }
        }

        // Add usage pattern suggestions
        if self.config.enable_usage_pattern_suggestions {
            if let Ok(usage) = self.generate_usage_pattern_suggestions(query).await {
                all_suggestions.extend(usage);
            }
        }

        // Add popular suggestions
        if self.config.enable_popularity_ranking {
            if let Ok(popular) = self.generate_popular_suggestions(query).await {
                all_suggestions.extend(popular);
            }
        }

        Ok(all_suggestions)
    }

    /// Generate usage pattern-based suggestions
    async fn generate_usage_pattern_suggestions(&self, query: &str) -> Result<Vec<KGAutocompleteSuggestion>, ComponentError> {
        let mut suggestions = Vec::new();

        let usage_tracker = self.usage_tracker.read().await;

        // Find frequently used terms that start with the query
        for (term, usage_stats) in &usage_tracker.selection_stats {
            if term.starts_with(query) && usage_stats.selection_count > 0 {
                let base_suggestion = AutocompleteSuggestion {
                    term: term.clone(),
                    score: (usage_stats.click_through_rate * 0.8) as f64, // Scale based on CTR
                    source: "usage-pattern".to_string(),
                };

                let mut kg_suggestion = self.convert_to_kg_suggestion(
                    base_suggestion,
                    KGAutocompleteCategory::Recent,
                    KGSuggestionRelationship::UsageCorrelation,
                ).await?;
                kg_suggestion.usage_stats = usage_stats.clone();
                suggestions.push(kg_suggestion);
            }
        }

        Ok(suggestions)
    }

    /// Generate popular suggestions
    async fn generate_popular_suggestions(&self, _query: &str) -> Result<Vec<KGAutocompleteSuggestion>, ComponentError> {
        let mut suggestions = Vec::new();

        let usage_tracker = self.usage_tracker.read().await;

        // Find most frequently selected terms overall
        let mut popular_terms: Vec<_> = usage_tracker.selection_stats
            .iter()
            .map(|(term, stats)| (term.clone(), stats.selection_count))
            .collect();

        popular_terms.sort_by(|a, b| b.1.cmp(&a.1));
        popular_terms.truncate(5); // Top 5 popular terms

        for (term, selection_count) in popular_terms {
            let base_suggestion = AutocompleteSuggestion {
                term,
                score: (selection_count as f64 / 100.0).min(1.0), // Normalize score
                source: "popularity".to_string(),
            };

            let kg_suggestion = self.convert_to_kg_suggestion(
                base_suggestion,
                KGAutocompleteCategory::Popular,
                KGSuggestionRelationship::None,
            ).await?;
            suggestions.push(kg_suggestion);
        }

        Ok(suggestions)
    }

    /// Convert basic AutocompleteSuggestion to KG-enhanced version
    async fn convert_to_kg_suggestion(
        &self,
        base_suggestion: AutocompleteSuggestion,
        category: KGAutocompleteCategory,
        relationship: KGSuggestionRelationship,
    ) -> Result<KGAutocompleteSuggestion, ComponentError> {
        let usage_stats = self.get_usage_stats(&base_suggestion.term).await;

        Ok(KGAutocompleteSuggestion {
            base_suggestion,
            category,
            confidence: 0.8, // Default confidence
            relationship: Some(relationship),
            usage_stats,
            semantic_context: None, // Would be populated by semantic analysis
            ranking_score: 0.8, // Will be recalculated during ranking
            metadata: KGSuggestionMetadata {
                created_at: std::time::SystemTime::now(),
                updated_at: std::time::SystemTime::now(),
                source: KGSuggestionSource::Thesaurus,
                attributes: HashMap::new(),
            },
        })
    }

    /// Get usage statistics for a term
    async fn get_usage_stats(&self, term: &str) -> KGSuggestionUsageStats {
        let usage_tracker = self.usage_tracker.read().await;
        usage_tracker.selection_stats
            .get(term)
            .cloned()
            .unwrap_or_default()
    }

    /// Update suggestion rankings based on configuration
    async fn update_suggestion_rankings(&mut self, suggestions: &mut [KGAutocompleteSuggestion]) {
        let ranking_config = &self.config.ranking_config;

        for suggestion in suggestions {
            let mut score = 0.0;

            // Base suggestion score
            score += suggestion.base_suggestion.score * ranking_config.exact_match_weight;

            // Category-specific scoring
            match suggestion.category {
                KGAutocompleteCategory::ExactMatch => {
                    score += ranking_config.exact_match_weight;
                }
                KGAutocompleteCategory::FuzzyMatch => {
                    score += ranking_config.fuzzy_match_weight * suggestion.base_suggestion.score;
                }
                KGAutocompleteCategory::SemanticRelated => {
                    score += ranking_config.semantic_weight;
                }
                KGAutocompleteCategory::Recent => {
                    score += ranking_config.recent_usage_weight;
                }
                KGAutocompleteCategory::Popular => {
                    score += ranking_config.popularity_weight;
                }
                _ => {}
            }

            // Usage pattern scoring
            if suggestion.usage_stats.selection_count > 0 {
                score += ranking_config.usage_pattern_weight *
                    (suggestion.usage_stats.click_through_rate * 2.0).min(1.0);
            }

            // Relationship scoring
            if let Some(relationship) = &suggestion.relationship {
                match relationship {
                    KGSuggestionRelationship::Prefix => score += 1.0,
                    KGSuggestionRelationship::Substring => score += 0.8,
                    KGSuggestionRelationship::SemanticSimilarity => score += 0.6,
                    KGSuggestionRelationship::GraphConnected => score += 0.7,
                    KGSuggestionRelationship::UsageCorrelation => score += 0.5,
                    KGSuggestionRelationship::None => {}
                }
            }

            suggestion.ranking_score = score;
        }
    }

    /// Select a suggestion
    pub async fn select_suggestion(&mut self, index: usize) -> Result<&KGAutocompleteSuggestion, ComponentError> {
        if index >= self.state.suggestions.len() {
            return Err(ComponentError::Configuration("Invalid suggestion index".to_string()));
        }

        let suggestion = &self.state.suggestions[index];

        // Update usage statistics
        self.update_usage_statistics(&suggestion.base_suggestion.term).await;

        // Update selected index
        self.state.selected_index = Some(index);

        // Send selection event
        let _ = self.event_sender.send(KGAutocompleteEvent::SuggestionSelected {
            suggestion: suggestion.clone(),
            index,
        });

        Ok(suggestion)
    }

    /// Update usage statistics for a selected suggestion
    async fn update_usage_statistics(&mut self, term: &str) {
        let mut usage_tracker = self.usage_tracker.write().await;
        let stats = usage_tracker.selection_stats.entry(term.to_string()).or_default();

        stats.selection_count += 1;
        stats.impression_count += 1;
        stats.last_selected = Some(std::time::SystemTime::now());

        // Update click-through rate
        if stats.impression_count > 0 {
            stats.click_through_rate = stats.selection_count as f64 / stats.impression_count as f64;
        }

        // Update selection frequency
        let now = std::time::SystemTime::now();
        stats.selection_frequency.push((now, 1));

        // Clean old frequency data (keep last 100 entries)
        if stats.selection_frequency.len() > 100 {
            stats.selection_frequency.drain(0..stats.selection_frequency.len() - 100);
        }
    }

    /// Check cache for suggestions
    async fn check_cache(&self, query: &str) -> Option<CachedKGSuggestions> {
        let cache = self.suggestion_cache.read().await;
        let query_hash = self.calculate_query_hash(query);

        if let Some(cached_suggestions) = cache.get(query) {
            let cache_age = std::time::SystemTime::now()
                .duration_since(cached_suggestions.timestamp)
                .unwrap_or(Duration::ZERO);

            if cache_age.as_secs() < self.config.cache_ttl_seconds {
                return Some(cached_suggestions.clone());
            }
        }

        None
    }

    /// Cache suggestions
    async fn cache_suggestions(&self, query: &str, suggestions: &[KGAutocompleteSuggestion]) {
        let mut cache = self.suggestion_cache.write().await;

        // Implement LRU eviction if cache is full
        if cache.len() >= self.config.cache_size {
            self.evict_oldest_cache_entries(&mut cache).await;
        }

        let cached_suggestions = CachedKGSuggestions {
            suggestions: suggestions.to_vec(),
            timestamp: std::time::SystemTime::now(),
            query_hash: self.calculate_query_hash(query),
        };

        cache.insert(query.to_string(), cached_suggestions);
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
    async fn evict_oldest_cache_entries(&self, cache: &mut HashMap<String, CachedKGSuggestions>) {
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
        self.state.cache_state.cached_queries = self.suggestion_cache.read().await.len();

        // Update performance metrics
        let total_requests = self.state.cache_state.cache_hits + self.state.cache_state.cache_misses;
        if total_requests > 0 {
            self.state.performance_metrics.cache_hit_ratio =
                self.state.cache_state.cache_hits as f64 / total_requests as f64;
        }
    }

    /// Update performance metrics
    async fn update_performance_metrics(&mut self, duration: Duration, suggestion_count: usize) {
        self.state.performance_metrics.total_queries += 1;

        let duration_ms = duration.as_millis() as f64;
        let total_queries = self.state.performance_metrics.total_queries as f64;

        // Update average latency
        self.state.performance_metrics.avg_suggestion_latency_ms =
            (self.state.performance_metrics.avg_suggestion_latency_ms * (total_queries - 1.0) + duration_ms) / total_queries;

        // Update peak latency
        self.state.performance_metrics.peak_suggestion_latency_ms =
            self.state.performance_metrics.peak_suggestion_latency_ms.max(duration_ms as u64);

        // Update average suggestions per query
        self.state.performance_metrics.avg_suggestions_per_query =
            (self.state.performance_metrics.avg_suggestions_per_query * (total_queries - 1.0) + suggestion_count as f64) / total_queries;

        // Check for performance alerts
        if duration_ms > self.config.performance_config.alert_thresholds.suggestion_latency_ms as f64 {
            self.generate_alert(
                KGAutocompleteAlertType::HighSuggestionLatency,
                format!("Suggestion latency exceeded threshold: {:.2}ms", duration_ms),
                crate::components::knowledge_graph::KGAlertSeverity::Warning,
                duration_ms,
                self.config.performance_config.alert_thresholds.suggestion_latency_ms as f64,
            ).await;
        }
    }

    /// Generate performance alert
    async fn generate_alert(
        &mut self,
        alert_type: KGAutocompleteAlertType,
        message: String,
        severity: crate::components::knowledge_graph::KGAlertSeverity,
        metric_value: f64,
        threshold: f64,
    ) {
        let alert = KGAutocompletePerformanceAlert {
            id: Uuid::new_v4().to_string(),
            alert_type,
            message,
            severity,
            timestamp: std::time::SystemTime::now(),
            metric_value,
        };

        self.state.performance_metrics.performance_alerts.push(alert.clone());

        // Send alert event
        let _ = self.event_sender.send(KGAutocompleteEvent::PerformanceAlert { alert });
    }

    /// Get component events
    pub async fn get_events(&mut self) -> Vec<KGAutocompleteEvent> {
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

    /// Clear suggestion cache
    pub async fn clear_cache(&mut self) -> Result<(), ComponentError> {
        let mut cache = self.suggestion_cache.write().await;
        cache.clear();

        // Reset cache statistics
        self.state.cache_state = KGAutocompleteCacheState::default();

        Ok(())
    }

    /// Get current configuration
    pub fn config(&self) -> &KGAutocompleteConfig {
        &self.config
    }

    /// Update component configuration
    pub async fn update_config(&mut self, config: KGAutocompleteConfig) -> Result<(), ComponentError> {
        self.config = config.clone();

        // Clear cache as configuration has changed
        self.clear_cache().await?;

        // Send configuration updated event
        let _ = self.event_sender.send(KGAutocompleteEvent::ConfigurationUpdated);

        Ok(())
    }

    /// Get current state
    pub fn state(&self) -> &KGAutocompleteState {
        &self.state
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> &KGAutocompletePerformanceMetrics {
        &self.state.performance_metrics
    }

    /// Get cache state
    pub fn get_cache_state(&self) -> &KGAutocompleteCacheState {
        &self.state.cache_state
    }
}

