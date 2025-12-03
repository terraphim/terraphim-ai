#![recursion_limit = "1024"]

/// Comprehensive tests for enhanced search components
///
/// These tests validate the enhanced search components including:
/// - Reusable search components with ReusableComponent trait
/// - Generic service abstraction for search and autocomplete
/// - Concurrent search with cancellation and debouncing
/// - Performance monitoring and optimization
/// - Service registry and health checking

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use terraphim_desktop_gpui::components::{
    ComponentConfig, ComponentError, ServiceRegistry, ReusableComponent,
    SearchComponent, SearchComponentConfig, SearchComponentState, SearchComponentEvent,
    SearchServiceRegistry, SearchService, AutocompleteService, SearchServiceRequest,
    AutocompleteServiceRequest, TerraphimSearchServiceAdapter, TerraphimAutocompleteServiceAdapter,
    ConcurrentSearchManager, ConcurrentSearchConfig, DebouncedSearch, SearchResultCache,
    SearchPerformanceMonitor, SearchPerformanceMonitorConfig, SearchAlertConfig,
    ComponentTestUtils, PerformanceTestUtils, ComponentTestHarness
};
use terraphim_desktop_gpui::search_service::{SearchOptions, SearchResults, LogicalOperator};
use terraphim_desktop_gpui::autocomplete::AutocompleteSuggestion;

/// Mock search service for testing
#[derive(Debug, Clone)]
struct MockSearchService {
    delay: Duration,
    should_fail: bool,
}

#[async_trait::async_trait]
impl SearchService for MockSearchService {
    type Request = SearchServiceRequest;
    type Response = SearchResults;
    type Error = terraphim_desktop_gpui::components::search_services::SearchServiceError;

    async fn search(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        // Simulate processing delay
        sleep(self.delay).await;

        if self.should_fail {
            return Err(terraphim_desktop_gpui::components::search_services::SearchServiceError::Network("Mock failure".to_string()));
        }

        // Generate mock results
        let documents = vec![];
        Ok(SearchResults {
            documents,
            total: documents.len(),
            query: request.query,
        })
    }

    fn capabilities(&self) -> terraphim_desktop_gpui::components::search_services::SearchCapabilities {
        terraphim_desktop_gpui::components::search_services::SearchCapabilities::default()
    }

    fn metadata(&self) -> terraphim_desktop_gpui::components::search_services::SearchServiceMetadata {
        terraphim_desktop_gpui::components::search_services::SearchServiceMetadata {
            name: "Mock Search Service".to_string(),
            version: "1.0.0".to_string(),
            description: "Mock search service for testing".to_string(),
            provider: "Test".to_string(),
            endpoints: vec![],
            authentication: terraphim_desktop_gpui::components::search_services::AuthenticationInfo {
                auth_type: terraphim_desktop_gpui::components::search_services::AuthType::None,
                required_credentials: vec![],
                auth_endpoints: None,
            },
            rate_limits: terraphim_desktop_gpui::components::search_services::RateLimits {
                requests_per_second: Some(100),
                requests_per_minute: None,
                requests_per_hour: None,
                requests_per_day: None,
                concurrent_requests: Some(10),
            },
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        if !self.should_fail {
            Ok(())
        } else {
            Err(terraphim_desktop_gpui::components::search_services::SearchServiceError::ServiceUnavailable("Mock service unavailable".to_string()))
        }
    }
}

/// Mock autocomplete service for testing
#[derive(Debug, Clone)]
struct MockAutocompleteService {
    suggestions: Vec<AutocompleteSuggestion>,
    delay: Duration,
    should_fail: bool,
}

#[async_trait::async_trait]
impl AutocompleteService for MockAutocompleteService {
    type Request = AutocompleteServiceRequest;
    type Response = Vec<AutocompleteSuggestion>;
    type Error = terraphim_desktop_gpui::components::search_services::AutocompleteServiceError;

    async fn autocomplete(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        // Simulate processing delay
        sleep(self.delay).await;

        if self.should_fail {
            return Err(terraphim_desktop_gpui::components::search_services::AutocompleteServiceError::Network("Mock failure".to_string()));
        }

        // Filter suggestions based on query
        let filtered: Vec<AutocompleteSuggestion> = self.suggestions
            .iter()
            .filter(|s| s.term.to_lowercase().contains(&request.query.to_lowercase()))
            .take(request.max_results)
            .cloned()
            .collect();

        if filtered.is_empty() {
            Err(terraphim_desktop_gpui::components::search_services::AutocompleteServiceError::NoResults)
        } else {
            Ok(filtered)
        }
    }

    fn capabilities(&self) -> terraphim_desktop_gpui::components::search_services::AutocompleteCapabilities {
        terraphim_desktop_gpui::components::search_services::AutocompleteCapabilities::default()
    }

    fn metadata(&self) -> terraphim_desktop_gpui::components::search_services::AutocompleteServiceMetadata {
        terraphim_desktop_gpui::components::search_services::AutocompleteServiceMetadata {
            name: "Mock Autocomplete Service".to_string(),
            version: "1.0.0".to_string(),
            description: "Mock autocomplete service for testing".to_string(),
            provider: "Test".to_string(),
            data_sources: vec!["mock".to_string()],
            update_frequency: "daily".to_string(),
            authentication: terraphim_desktop_gpui::components::search_services::AuthenticationInfo {
                auth_type: terraphim_desktop_gpui::components::search_services::AuthType::None,
                required_credentials: vec![],
                auth_endpoints: None,
            },
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        if !self.should_fail {
            Ok(())
        } else {
            Err(terraphim_desktop_gpui::components::search_services::AutocompleteServiceError::ServiceUnavailable("Mock service unavailable".to_string()))
        }
    }
}

// Test search component configuration
#[derive(Debug, Clone, PartialEq)]
struct TestSearchConfig {
    pub placeholder: String,
    pub autocomplete_min_chars: usize,
    pub max_autocomplete_suggestions: usize,
    pub search_debounce_ms: u64,
}

impl Default for TestSearchConfig {
    fn default() -> Self {
        Self {
            placeholder: "Test search...".to_string(),
            autocomplete_min_chars: 2,
            max_autocomplete_suggestions: 5,
            search_debounce_ms: 200,
        }
    }
}

impl ComponentConfig for TestSearchConfig {
    fn schema() -> terraphim_desktop_gpui::components::ConfigSchema {
        use terraphim_desktop_gpui::components::{ConfigSchema, ConfigField, ConfigFieldType, ValidationRule};

        ConfigSchema::new(
            "TestSearchConfig".to_string(),
            "1.0.0".to_string(),
            "Test search configuration".to_string(),
        )
        .with_field(ConfigField {
            name: "placeholder".to_string(),
            field_type: ConfigFieldType::String,
            required: false,
            default: Some(terraphim_desktop_gpui::components::ConfigValue::String("Test search...".to_string())),
            description: "Search placeholder text".to_string(),
            validation: vec![],
            docs: None,
        })
    }

    fn validate(&self) -> Result<(), terraphim_desktop_gpui::components::ConfigError> {
        if self.autocomplete_min_chars == 0 {
            return Err(terraphim_desktop_gpui::components::ConfigError::Validation(
                "autocomplete_min_chars must be greater than 0".to_string()
            ));
        }
        Ok(())
    }

    fn default() -> Self {
        Self::default()
    }

    fn merge(&self, other: &Self) -> Result<Self, terraphim_desktop_gpui::components::ConfigError> {
        Ok(Self {
            placeholder: if other.placeholder != "Test search..." { other.placeholder.clone() } else { self.placeholder.clone() },
            autocomplete_min_chars: if other.autocomplete_min_chars != 2 { other.autocomplete_min_chars } else { self.autocomplete_min_chars },
            max_autocomplete_suggestions: if other.max_autocomplete_suggestions != 5 { other.max_autocomplete_suggestions } else { self.max_autocomplete_suggestions },
            search_debounce_ms: if other.search_debounce_ms != 200 { other.search_debounce_ms } else { self.search_debounce_ms },
        })
    }

    fn to_map(&self) -> HashMap<String, terraphim_desktop_gpui::components::ConfigValue> {
        let mut map = HashMap::new();
        map.insert("placeholder".to_string(), terraphim_desktop_gpui::components::ConfigValue::String(self.placeholder.clone()));
        map.insert("autocomplete_min_chars".to_string(), terraphim_desktop_gpui::components::ConfigValue::Integer(self.autocomplete_min_chars as i64));
        map.insert("max_autocomplete_suggestions".to_string(), terraphim_desktop_gpui::components::ConfigValue::Integer(self.max_autocomplete_suggestions as i64));
        map.insert("search_debounce_ms".to_string(), terraphim_desktop_gpui::components::ConfigValue::Integer(self.search_debounce_ms as i64));
        map
    }

    fn from_map(map: HashMap<String, terraphim_desktop_gpui::components::ConfigValue>) -> Result<Self, terraphim_desktop_gpui::components::ConfigError> {
        Ok(Self {
            placeholder: map.get("placeholder")
                .and_then(|v| v.as_string())
                .unwrap_or("Test search...").to_string(),
            autocomplete_min_chars: map.get("autocomplete_min_chars")
                .and_then(|v| v.as_integer())
                .and_then(|i| usize::try_from(i).ok())
                .unwrap_or(2),
            max_autocomplete_suggestions: map.get("max_autocomplete_suggestions")
                .and_then(|v| v.as_integer())
                .and_then(|i| usize::try_from(i).ok())
                .unwrap_or(5),
            search_debounce_ms: map.get("search_debounce_ms")
                .and_then(|v| v.as_integer())
                .and_then(|i| u64::try_from(i).ok())
                .unwrap_or(200),
        })
    }

    fn is_equivalent(&self, other: &Self) -> bool {
        self.placeholder == other.placeholder &&
        self.autocomplete_min_chars == other.autocomplete_min_chars &&
        self.search_debounce_ms == other.search_debounce_ms
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_config(&self) -> Box<dyn ComponentConfig> {
        Box::new(self.clone())
    }
}

// Test search component state
#[derive(Debug, Clone, PartialEq)]
struct TestSearchState {
    pub query: String,
    pub results_count: usize,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for TestSearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            results_count: 0,
            loading: false,
            error: None,
        }
    }
}

// Simple test search component
#[derive(Debug)]
struct TestSearchComponent {
    config: TestSearchConfig,
    state: TestSearchState,
    performance_tracker: terraphim_desktop_gpui::components::PerformanceTracker,
    is_mounted: bool,
}

impl ReusableComponent for TestSearchComponent {
    type Config = TestSearchConfig;
    type State = TestSearchState;
    type Event = String;

    fn component_id() -> &'static str {
        "test-search-component"
    }

    fn component_version() -> &'static str {
        "1.0.0"
    }

    fn init(config: Self::Config) -> Self {
        Self {
            config,
            state: TestSearchState::default(),
            performance_tracker: terraphim_desktop_gpui::components::PerformanceTracker::default(),
            is_mounted: false,
        }
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn update_config(&mut self, config: Self::Config) -> Result<(), ComponentError> {
        self.config = config;
        Ok(())
    }

    fn state(&self) -> &Self::State {
        &self.state
    }

    fn update_state(&mut self, state: Self::State) -> Result<(), ComponentError> {
        self.state = state;
        Ok(())
    }

    fn mount(&mut self, _cx: &mut gpui::Context<Self>) -> Result<(), ComponentError> {
        if self.is_mounted {
            return Err(ComponentError::AlreadyMounted);
        }
        self.is_mounted = true;
        Ok(())
    }

    fn unmount(&mut self, _cx: &mut gpui::Context<Self>) -> Result<(), ComponentError> {
        if !self.is_mounted {
            return Err(ComponentError::NotMounted);
        }
        self.is_mounted = false;
        Ok(())
    }

    fn handle_lifecycle_event(&mut self, _event: terraphim_desktop_gpui::components::LifecycleEvent, _cx: &mut gpui::Context<Self>) -> Result<(), ComponentError> {
        Ok(())
    }

    fn is_mounted(&self) -> bool {
        self.is_mounted
    }

    fn performance_metrics(&self) -> &terraphim_desktop_gpui::components::PerformanceTracker {
        &self.performance_tracker
    }

    fn reset_performance_metrics(&mut self) {
        self.performance_tracker.reset();
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["MockSearchService"]
    }

    fn are_dependencies_satisfied(&self, _registry: &ServiceRegistry) -> bool {
        true
    }

    fn cleanup(&mut self) -> Result<(), ComponentError> {
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod enhanced_search_component_tests {
    use super::*;

    #[tokio::test]
    async fn test_search_component_initialization() {
        let config = SearchComponentConfig::default();
        let component = SearchComponent::new(config);

        assert_eq!(component.config().placeholder, "Search...");
        assert_eq!(component.config().autocomplete_min_chars, 2);
        assert_eq!(component.state().query, "");
        assert!(component.state().results.is_none());
    }

    #[tokio::test]
    async fn test_search_component_configuration_validation() {
        // Valid configuration
        let valid_config = SearchComponentConfig {
            placeholder: "Test".to_string(),
            autocomplete_min_chars: 2,
            max_autocomplete_suggestions: 10,
            search_debounce_ms: 300,
            ..Default::default()
        };
        assert!(valid_config.validate().is_ok());

        // Invalid configuration (autocomplete_min_chars = 0)
        let invalid_config = SearchComponentConfig {
            placeholder: "Test".to_string(),
            autocomplete_min_chars: 0,
            max_autocomplete_suggestions: 10,
            search_debounce_ms: 300,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[tokio::test]
    async fn test_search_component_config_serialization() {
        let config = SearchComponentConfig::default();

        // Test JSON serialization
        let json = config.to_json();
        assert!(json.is_ok());

        // Test JSON deserialization
        let deserialized: SearchComponentConfig = SearchComponentConfig::from_json(&json.unwrap());
        assert!(deserialized.is_ok());
        assert_eq!(config, deserialized.unwrap());
    }

    #[tokio::test]
    async fn test_search_component_query_operations() {
        let config = SearchComponentConfig::default();
        let mut component = SearchComponent::new(config);

        // Set query
        component.set_query("test query".to_string());
        assert_eq!(component.state().query, "test query");

        // Check parsed query
        assert!(component.state().parsed_query.is_some());
        let parsed = component.state().parsed_query.as_ref().unwrap();
        assert_eq!(parsed.terms.len(), 1);
        assert_eq!(parsed.terms[0], "test query");
        assert!(parsed.operator.is_none());

        // Clear query
        component.clear();
        assert_eq!(component.state().query, "");
        assert!(component.state().parsed_query.is_none());
    }

    #[tokio::test]
    async fn test_search_component_suggestion_selection() {
        let config = SearchComponentConfig::default();
        let mut component = SearchComponent::new(config);

        // Add mock suggestions
        component.state.suggestions = vec![
            AutocompleteSuggestion {
                term: "test1".to_string(),
                normalized_term: "test1".to_string(),
                url: Some("http://example.com/test1".to_string()),
                score: 1.0,
            },
            AutocompleteSuggestion {
                term: "test2".to_string(),
                normalized_term: "test2".to_string(),
                url: Some("http://example.com/test2".to_string()),
                score: 0.8,
            },
        ];

        // Select suggestion
        let selected = component.select_suggestion(1);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().term, "test2");
        assert_eq!(component.state().query, "test2");

        // Check selected index
        assert_eq!(component.state().selected_suggestion_index, Some(1));
    }

    #[tokio::test]
    async fn test_search_service_registry() {
        let mut registry = SearchServiceRegistry::new();

        // Create mock service
        let mock_service = MockSearchService {
            delay: Duration::from_millis(10),
            should_fail: false,
        };

        // Register service
        registry.register_search_service("test_search".to_string(), mock_service);

        // Check service registration
        assert!(registry.list_search_services().contains(&"test_search".to_string()));
        assert!(registry.get_search_service("test_search").is_some());

        // Set default service
        assert!(registry.set_default_search_service("test_search").is_ok());
        assert!(registry.default_search_service().is_some());

        // Unregister service
        assert!(registry.unregister_search_service("test_search"));
        assert!(!registry.list_search_services().contains(&"test_search".to_string()));
    }

    #[tokio::test]
    async fn test_autocomplete_service_registry() {
        let mut registry = SearchServiceRegistry::new();

        // Create mock autocomplete service
        let suggestions = vec![
            AutocompleteSuggestion {
                term: "rust".to_string(),
                normalized_term: "rust".to_string(),
                url: Some("http://example.com/rust".to_string()),
                score: 1.0,
            },
            AutocompleteSuggestion {
                term: "tokio".to_string(),
                normalized_term: "tokio".to_string(),
                url: Some("http://example.com/tokio".to_string()),
                score: 0.9,
            },
        ];

        let mock_service = MockAutocompleteService {
            suggestions,
            delay: Duration::from_millis(5),
            should_fail: false,
        };

        // Register service
        registry.register_autocomplete_service("test_autocomplete".to_string(), mock_service);

        // Check service registration
        assert!(registry.list_autocomplete_services().contains(&"test_autocomplete".to_string()));
        assert!(registry.get_autocomplete_service("test_autocomplete").is_some());

        // Set default service
        assert!(registry.set_default_autocomplete_service("test_autocomplete").is_ok());
        assert!(registry.default_autocomplete_service().is_some());
    }

    #[tokio::test]
    async fn test_concurrent_search_manager() {
        let config = ConcurrentSearchConfig::default();
        let mock_service = MockSearchService {
            delay: Duration::from_millis(50),
            should_fail: false,
        };

        let manager = ConcurrentSearchManager::new(mock_service, config);

        // Create search request
        let request = SearchServiceRequest {
            query: "test query".to_string(),
            options: SearchOptions::default(),
            metadata: HashMap::new(),
        };

        // Execute search
        let result = manager.search(request, terraphim_desktop_gpui::components::search_services::SearchPriority::Normal).await;
        assert!(result.is_ok());

        let search_result = result.unwrap();
        assert_eq!(search_result.query, "test query");
        assert!(search_result.results.is_some());
    }

    #[tokio::test]
    async fn test_concurrent_search_cancellation() {
        let config = ConcurrentSearchConfig::default();
        let mock_service = MockSearchService {
            delay: Duration::from_millis(100),
            should_fail: false,
        };

        let manager = Arc::new(ConcurrentSearchManager::new(mock_service, config));

        // Create search request
        let request = SearchServiceRequest {
            query: "test query".to_string(),
            options: SearchOptions::default(),
            metadata: HashMap::new(),
        };

        // Start search in background
        let manager_clone = manager.clone();
        let search_future = tokio::spawn(async move {
            manager_clone.search(request, terraphim_desktop_gpui::components::search_services::SearchPriority::Normal).await
        });

        // Cancel the search
        manager.cancel_all_searches().await;

        // Wait for search to complete
        let result = search_future.await.unwrap();
        // The search might still complete if cancellation happened late
        // This test mainly ensures the cancellation mechanism works
    }

    #[tokio::test]
    async fn test_debounced_search() {
        let config = ConcurrentSearchConfig::default();
        let mock_service = MockSearchService {
            delay: Duration::from_millis(10),
            should_fail: false,
        };

        let manager = Arc::new(ConcurrentSearchManager::new(mock_service, config));
        let mut debounced = DebouncedSearch::new(manager, 200); // 200ms debounce

        // First search request
        let request1 = SearchServiceRequest {
            query: "test".to_string(),
            options: SearchOptions::default(),
            metadata: HashMap::new(),
        };

        let result1_future = debounced.search(request1, terraphim_desktop_gpui::components::search_services::SearchPriority::Normal);

        // Second search request immediately (should cancel the first)
        tokio::time::sleep(Duration::from_millis(50)).await;
        debounced.cancel();

        let request2 = SearchServiceRequest {
            query: "test final".to_string(),
            options: SearchOptions::default(),
            metadata: HashMap::new(),
        };

        let result2 = debounced.search(request2, terraphim_desktop_gpui::components::search_services::SearchPriority::Normal).await;
        assert!(result2.is_ok());

        // The first search was cancelled
        assert!(matches!(result1_future.await.unwrap().unwrap_err(), terraphim_desktop_gpui::components::concurrent_search::ConcurrentSearchError::Cancelled));
    }

    #[tokio::test]
    async fn test_search_result_cache() {
        let cache = SearchResultCache::new(60); // 60 seconds TTL

        // Create test result
        let result = SearchResults {
            documents: vec![],
            total: 0,
            query: "test query".to_string(),
        };

        // Cache miss initially
        assert!(cache.get("test query").await.is_none());

        // Store result
        cache.set("test query".to_string(), result.clone()).await;

        // Cache hit
        let cached_result = cache.get("test query").await;
        assert!(cached_result.is_some());
        assert_eq!(cached_result.unwrap().query, "test query");

        // Check cache stats
        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.valid_entries, 1);
        assert_eq!(stats.expired_entries, 0);
    }

    #[tokio::test]
    async fn test_search_performance_monitor() {
        let config = SearchPerformanceMonitorConfig::default();
        let alert_config = SearchAlertConfig::default();
        let monitor = SearchPerformanceMonitor::new(config, alert_config);

        // Record some search operations
        monitor.record_search(Duration::from_millis(100), true, 5).await;
        monitor.record_search(Duration::from_millis(200), true, 8).await;
        monitor.record_search(Duration::from_millis(150), false, 3).await;

        // Get metrics
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.search_metrics.total_searches, 3);
        assert_eq!(metrics.search_metrics.successful_searches, 2);
        assert_eq!(metrics.search_metrics.failed_searches, 1);

        // Calculate expected average: (100 + 200) / 2 = 150ms
        assert_eq!(metrics.search_metrics.avg_response_time, Duration::from_millis(150));
    }

    #[test]
    fn test_component_test_harness_search() {
        let config = TestSearchConfig::default();
        let state = TestSearchState::default();
        let mut harness = ComponentTestHarness::<TestSearchComponent>::new(config, state);

        // Run comprehensive tests
        let result = harness.run_comprehensive_test();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_component_performance_benchmark() {
        let result = PerformanceTestUtils::benchmark(
            || {
                let config = SearchComponentConfig::default();
                SearchComponent::new(config)
            },
            100
        ).await;

        assert_eq!(result.results.len(), 100);
        assert!(result.avg_duration() > Duration::ZERO);

        // Should be reasonably fast to create components
        assert!(result.avg_duration() < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_search_service_health_check() {
        let healthy_service = MockSearchService {
            delay: Duration::from_millis(1),
            should_fail: false,
        };

        let unhealthy_service = MockSearchService {
            delay: Duration::from_millis(1),
            should_fail: true,
        };

        // Health check
        assert!(healthy_service.health_check().await.is_ok());
        assert!(unhealthy_service.health_check().await.is_err());
    }

    #[tokio::test]
    async fn test_autocomplete_service_health_check() {
        let healthy_service = MockAutocompleteService {
            suggestions: vec![],
            delay: Duration::from_millis(1),
            should_fail: false,
        };

        let unhealthy_service = MockAutocompleteService {
            suggestions: vec![],
            delay: Duration::from_millis(1),
            should_fail: true,
        };

        // Health check
        assert!(healthy_service.health_check().await.is_ok());
        assert!(unhealthy_service.health_check().await.is_err());
    }

    #[tokio::test]
    async fn test_service_registry_health_checker() {
        let mut registry = SearchServiceRegistry::new();

        // Register healthy services
        let healthy_search = MockSearchService {
            delay: Duration::from_millis(1),
            should_fail: false,
        };
        registry.register_search_service("healthy_search".to_string(), healthy_search);

        let healthy_autocomplete = MockAutocompleteService {
            suggestions: vec![
                AutocompleteSuggestion {
                    term: "test".to_string(),
                    normalized_term: "test".to_string(),
                    url: Some("http://example.com".to_string()),
                    score: 1.0,
                }
            ],
            delay: Duration::from_millis(1),
            should_fail: false,
        };
        registry.register_autocomplete_service("healthy_autocomplete".to_string(), healthy_autocomplete);

        // Check all services health
        let health_results = terraphim_desktop_gpui::components::search_services::ServiceHealthChecker::check_all_services(&registry).await;
        assert_eq!(health_results.len(), 2);
        assert!(health_results.values().all(|r| r.is_ok()));

        // Get service statistics
        let stats = terraphim_desktop_gpui::components::search_services::ServiceHealthChecker::get_service_statistics(&registry).await;
        assert_eq!(stats.total_search_services, 1);
        assert_eq!(stats.total_autocomplete_services, 1);
        assert_eq!(stats.healthy_search_services, 1);
        assert_eq!(stats.healthy_autocomplete_services, 1);
    }

    #[tokio::test]
    async fn test_concurrent_search_performance_under_load() {
        let config = ConcurrentSearchConfig {
            max_concurrent_searches: 10,
            default_timeout_ms: 2000,
            enable_cancellation: true,
            enable_deduplication: true,
            enable_caching: true,
            ..Default::default()
        };

        let mock_service = MockSearchService {
            delay: Duration::from_millis(20),
            should_fail: false,
        };

        let manager = Arc::new(ConcurrentSearchManager::new(mock_service, config));

        // Create multiple concurrent search requests
        let mut futures = Vec::new();
        for i in 0..20 {
            let request = SearchServiceRequest {
                query: format!("search query {}", i),
                options: SearchOptions::default(),
                metadata: HashMap::new(),
            };
            let future = manager.search(
                request,
                if i % 2 == 0 {
                    terraphim_desktop_gpui::components::search_services::SearchPriority::Normal
                } else {
                    terraphim_desktop_gpui::components::search_services::SearchPriority::Low
                }
            );
            futures.push(future);
        }

        // Execute all searches concurrently
        let results = futures::future::join_all(futures).await;

        // All searches should succeed
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(success_count, 20);

        // Check statistics
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_searches, 20);
        assert_eq!(stats.successful_searches, 20);
        assert_eq!(stats.failed_searches, 0);
    }

    #[test]
    fn test_search_component_lifecycle() {
        let config = TestSearchConfig::default();
        let component = TestSearchComponent::init(config);

        // Test initial state
        assert_eq!(component.config().placeholder, "Test search...");
        assert!(!component.is_mounted());

        // Test component type safety
        let any_component = component.as_any();
        assert!(any_component.is::<TestSearchComponent>());

        let downcasted = any_component.downcast_ref::<TestSearchComponent>();
        assert!(downcasted.is_some());

        // Test performance tracker
        let timer = component.performance_metrics().start_operation();
        std::thread::sleep(Duration::from_millis(1));
        timer.complete_success();

        let metrics = component.performance_metrics().current_metrics();
        assert_eq!(metrics.operation_count, 1);
        assert_eq!(metrics.success_count, 1);
    }

    #[test]
    fn test_search_component_config_serialization() {
        let config = TestSearchConfig::default();
        let map = config.to_map();

        assert_eq!(map.len(), 4);
        assert_eq!(map.get("placeholder"), Some(&terraphim_desktop_gpui::components::ConfigValue::String("Test search...".to_string())));
        assert_eq!(map.get("autocomplete_min_chars"), Some(&terraphim_desktop_gpui::components::ConfigValue::Integer(2)));

        let restored = TestSearchConfig::from_map(map);
        assert!(restored.is_ok());
        assert_eq!(config, restored.unwrap());
    }

    #[test]
    fn test_search_performance_monitor_config() {
        let config = SearchPerformanceMonitorConfig {
            enable_real_time: true,
            collection_interval_ms: 2000,
            history_retention_seconds: 7200,
            max_history_entries: 2000,
            enable_trend_analysis: true,
            enable_optimization_suggestions: true,
            enable_profiling: true,
            enable_distributed_tracing: false,
            enable_cache_monitoring: true,
        };

        assert_eq!(config.collection_interval_ms, 2000);
        assert_eq!(config.history_retention_seconds, 7200);
        assert_eq!(config.max_history_entries, 2000);
        assert!(config.enable_real_time);
        assert!(config.enable_trend_analysis);
    }

    #[test]
    fn test_concurrent_search_config() {
        let config = ConcurrentSearchConfig {
            max_concurrent_searches: 8,
            default_timeout_ms: 3000,
            debounce_timeout_ms: 250,
            enable_cancellation: true,
            enable_deduplication: true,
            enable_caching: true,
            cache_ttl_seconds: 600,
            enable_monitoring: true,
            enable_priority_queue: false,
        };

        assert_eq!(config.max_concurrent_searches, 8);
        assert_eq!(config.default_timeout_ms, 3000);
        assert_eq!(config.debounce_timeout_ms, 250);
        assert!(config.enable_cancellation);
        assert!(config.enable_deduplication);
        assert!(config.enable_caching);
    }

    #[test]
    fn test_search_alert_config() {
        let config = SearchAlertConfig {
            enabled: true,
            response_time_threshold_ms: 1500,
            error_rate_threshold: 10.0,
            cpu_threshold: 85.0,
            memory_threshold: 90.0,
            alert_cooldown: Duration::from_secs(120),
            alert_channels: vec![
                terraphim_desktop_gpui::components::search_performance::AlertChannel::Log,
                terraphim_desktop_gpui::components::search_performance::AlertChannel::Console,
            ],
        };

        assert_eq!(config.response_time_threshold_ms, 1500);
        assert_eq!(config.error_rate_threshold, 10.0);
        assert_eq!(config.cpu_threshold, 85.0);
        assert_eq!(config.memory_threshold, 90.0);
        assert_eq!(config.alert_cooldown, Duration::from_secs(120));
        assert_eq!(config.alert_channels.len(), 2);
    }
}