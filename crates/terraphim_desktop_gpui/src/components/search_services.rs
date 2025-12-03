/// Generic service abstraction for search and autocomplete
///
/// This module provides trait-based abstractions for search and autocomplete services,
/// enabling easy swapping and enhancement of different search backends.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::autocomplete::AutocompleteSuggestion;
use crate::search_service::{SearchOptions, SearchResults};
use crate::components::Service;

/// Generic search service trait
#[async_trait]
pub trait SearchService: Send + Sync + 'static {
    /// Search request type
    type Request: Clone + Send + Sync + 'static;

    /// Search response type
    type Response: Clone + Send + Sync + 'static;

    /// Error type
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute search request
    async fn search(&self, request: Self::Request) -> Result<Self::Response, Self::Error>;

    /// Get service capabilities
    fn capabilities(&self) -> SearchCapabilities;

    /// Get service metadata
    fn metadata(&self) -> SearchServiceMetadata;

    /// Check if service is healthy
    async fn health_check(&self) -> Result<(), Self::Error>;
}

/// Generic autocomplete service trait
#[async_trait]
pub trait AutocompleteService: Send + Sync + 'static {
    /// Autocomplete request type
    type Request: Clone + Send + Sync + 'static;

    /// Autocomplete response type
    type Response: Clone + Send + Sync + 'static;

    /// Error type
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get autocomplete suggestions
    async fn autocomplete(&self, request: Self::Request) -> Result<Self::Response, Self::Error>;

    /// Get service capabilities
    fn capabilities(&self) -> AutocompleteCapabilities;

    /// Get service metadata
    fn metadata(&self) -> AutocompleteServiceMetadata;

    /// Check if service is healthy
    async fn health_check(&self) -> Result<(), Self::Error>;
}

/// Search service capabilities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCapabilities {
    /// Supports full-text search
    pub full_text_search: bool,

    /// Supports fuzzy search
    pub fuzzy_search: bool,

    /// Supports semantic search
    pub semantic_search: bool,

    /// Supports filtering
    pub filtering: bool,

    /// Supports sorting
    pub sorting: bool,

    /// Supports pagination
    pub pagination: bool,

    /// Supports faceted search
    pub faceted_search: bool,

    /// Supports real-time search
    pub real_time: bool,

    /// Maximum query length
    pub max_query_length: Option<usize>,

    /// Supported result formats
    pub result_formats: Vec<String>,
}

impl Default for SearchCapabilities {
    fn default() -> Self {
        Self {
            full_text_search: true,
            fuzzy_search: false,
            semantic_search: false,
            filtering: true,
            sorting: true,
            pagination: true,
            faceted_search: false,
            real_time: false,
            max_query_length: Some(1000),
            result_formats: vec!["json".to_string()],
        }
    }
}

/// Autocomplete service capabilities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutocompleteCapabilities {
    /// Supports fuzzy matching
    pub fuzzy_matching: bool,

    /// Supports semantic suggestions
    pub semantic_suggestions: bool,

    /// Supports ranked suggestions
    pub ranked_suggestions: bool,

    /// Supports multiple suggestion types
    pub multiple_types: bool,

    /// Minimum query length
    pub min_query_length: usize,

    /// Maximum suggestions
    pub max_suggestions: Option<usize>,

    /// Supported suggestion types
    pub suggestion_types: Vec<String>,
}

impl Default for AutocompleteCapabilities {
    fn default() -> Self {
        Self {
            fuzzy_matching: false,
            semantic_suggestions: false,
            ranked_suggestions: true,
            multiple_types: true,
            min_query_length: 1,
            max_suggestions: Some(50),
            suggestion_types: vec!["term".to_string()],
        }
    }
}

/// Search service metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchServiceMetadata {
    /// Service name
    pub name: String,

    /// Service version
    pub version: String,

    /// Service description
    pub description: String,

    /// Service provider
    pub provider: String,

    /// Endpoints
    pub endpoints: Vec<String>,

    /// Authentication requirements
    pub authentication: AuthenticationInfo,

    /// Rate limits
    pub rate_limits: RateLimits,
}

/// Autocomplete service metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteServiceMetadata {
    /// Service name
    pub name: String,

    /// Service version
    pub version: String,

    /// Service description
    pub description: String,

    /// Service provider
    pub provider: String,

    /// Data sources
    pub data_sources: Vec<String>,

    /// Update frequency
    pub update_frequency: String,

    /// Authentication requirements
    pub authentication: AuthenticationInfo,
}

/// Authentication information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationInfo {
    /// Authentication type
    pub auth_type: AuthType,

    /// Required credentials
    pub required_credentials: Vec<String>,

    /// Authentication endpoints
    pub auth_endpoints: Option<Vec<String>>,
}

/// Authentication types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthType {
    /// No authentication required
    None,

    /// API key authentication
    ApiKey,

    /// OAuth2 authentication
    OAuth2,

    /// Basic authentication
    Basic,

    /// Custom authentication
    Custom(String),
}

/// Rate limits information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// Requests per second
    pub requests_per_second: Option<u32>,

    /// Requests per minute
    pub requests_per_minute: Option<u32>,

    /// Requests per hour
    pub requests_per_hour: Option<u32>,

    /// Requests per day
    pub requests_per_day: Option<u32>,

    /// Concurrent requests limit
    pub concurrent_requests: Option<u32>,
}

/// Service registry for managing multiple search and autocomplete services
#[derive(Debug)]
pub struct SearchServiceRegistry {
    search_services: HashMap<String, Arc<dyn SearchService<Request = SearchServiceRequest, Response = SearchResults, Error = SearchServiceError>>>,
    autocomplete_services: HashMap<String, Arc<dyn AutocompleteService<Request = AutocompleteServiceRequest, Response = Vec<AutocompleteSuggestion>, Error = AutocompleteServiceError>>>,
    default_search_service: Option<String>,
    default_autocomplete_service: Option<String>,
}

/// Standard search service request
#[derive(Debug, Clone)]
pub struct SearchServiceRequest {
    /// Search query
    pub query: String,

    /// Search options
    pub options: SearchOptions,

    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Standard autocomplete service request
#[derive(Debug, Clone)]
pub struct AutocompleteServiceRequest {
    /// Query for autocomplete
    pub query: String,

    /// Maximum number of suggestions
    pub max_results: usize,

    /// Suggestion types to include
    pub suggestion_types: Vec<String>,

    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Standard search service error
#[derive(Debug, Clone, thiserror::Error)]
pub enum SearchServiceError {
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Standard autocomplete service error
#[derive(Debug, Clone, thiserror::Error)]
pub enum AutocompleteServiceError {
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("No results")]
    NoResults,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl SearchServiceRegistry {
    /// Create new service registry
    pub fn new() -> Self {
        Self {
            search_services: HashMap::new(),
            autocomplete_services: HashMap::new(),
            default_search_service: None,
            default_autocomplete_service: None,
        }
    }

    /// Register a search service
    pub fn register_search_service<S>(&mut self, name: String, service: S)
    where
        S: SearchService<Request = SearchServiceRequest, Response = SearchResults, Error = SearchServiceError> + Send + Sync + 'static,
    {
        self.search_services.insert(name.clone(), Arc::new(service));

        // Set as default if it's the first one
        if self.default_search_service.is_none() {
            self.default_search_service = Some(name);
        }
    }

    /// Register an autocomplete service
    pub fn register_autocomplete_service<S>(&mut self, name: String, service: S)
    where
        S: AutocompleteService<Request = AutocompleteServiceRequest, Response = Vec<AutocompleteSuggestion>, Error = AutocompleteServiceError> + Send + Sync + 'static,
    {
        self.autocomplete_services.insert(name.clone(), Arc::new(service));

        // Set as default if it's the first one
        if self.default_autocomplete_service.is_none() {
            self.default_autocomplete_service = Some(name);
        }
    }

    /// Get default search service
    pub fn default_search_service(&self) -> Option<Arc<dyn SearchService<Request = SearchServiceRequest, Response = SearchResults, Error = SearchServiceError>>> {
        self.default_search_service.as_ref().and_then(|name| self.search_services.get(name).cloned())
    }

    /// Get default autocomplete service
    pub fn default_autocomplete_service(&self) -> Option<Arc<dyn AutocompleteService<Request = AutocompleteServiceRequest, Response = Vec<AutocompleteSuggestion>, Error = AutocompleteServiceError>>> {
        self.default_autocomplete_service.as_ref().and_then(|name| self.autocomplete_services.get(name).cloned())
    }

    /// Get search service by name
    pub fn get_search_service(&self, name: &str) -> Option<Arc<dyn SearchService<Request = SearchServiceRequest, Response = SearchResults, Error = SearchServiceError>>> {
        self.search_services.get(name).cloned()
    }

    /// Get autocomplete service by name
    pub fn get_autocomplete_service(&self, name: &str) -> Option<Arc<dyn AutocompleteService<Request = AutocompleteServiceRequest, Response = Vec<AutocompleteSuggestion>, Error = AutocompleteServiceError>>> {
        self.autocomplete_services.get(name).cloned()
    }

    /// List registered search services
    pub fn list_search_services(&self) -> Vec<String> {
        self.search_services.keys().cloned().collect()
    }

    /// List registered autocomplete services
    pub fn list_autocomplete_services(&self) -> Vec<String> {
        self.autocomplete_services.keys().cloned().collect()
    }

    /// Set default search service
    pub fn set_default_search_service(&mut self, name: &str) -> Result<(), String> {
        if self.search_services.contains_key(name) {
            self.default_search_service = Some(name.to_string());
            Ok(())
        } else {
            Err(format!("Search service '{}' not found", name))
        }
    }

    /// Set default autocomplete service
    pub fn set_default_autocomplete_service(&mut self, name: &str) -> Result<(), String> {
        if self.autocomplete_services.contains_key(name) {
            self.default_autocomplete_service = Some(name.to_string());
            Ok(())
        } else {
            Err(format!("Autocomplete service '{}' not found", name))
        }
    }

    /// Unregister search service
    pub fn unregister_search_service(&mut self, name: &str) -> bool {
        let removed = self.search_services.remove(name).is_some();

        // Update default if necessary
        if let Some(ref default) = self.default_search_service {
            if default == name {
                self.default_search_service = self.search_services.keys().next().cloned();
            }
        }

        removed
    }

    /// Unregister autocomplete service
    pub fn unregister_autocomplete_service(&mut self, name: &str) -> bool {
        let removed = self.autocomplete_services.remove(name).is_some();

        // Update default if necessary
        if let Some(ref default) = self.default_autocomplete_service {
            if default == name {
                self.default_autocomplete_service = self.autocomplete_services.keys().next().cloned();
            }
        }

        removed
    }
}

impl Default for SearchServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter for existing SearchService to work with the trait system
pub struct TerraphimSearchServiceAdapter {
    inner: crate::search_service::SearchService,
}

impl TerraphimSearchServiceAdapter {
    /// Create new adapter from existing SearchService
    pub fn new(service: crate::search_service::SearchService) -> Self {
        Self { inner: service }
    }
}

#[async_trait]
impl SearchService for TerraphimSearchServiceAdapter {
    type Request = SearchServiceRequest;
    type Response = SearchResults;
    type Error = SearchServiceError;

    async fn search(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        self.inner
            .search(&request.query, request.options)
            .await
            .map_err(|e| SearchServiceError::Network(e.to_string()))
    }

    fn capabilities(&self) -> SearchCapabilities {
        SearchCapabilities {
            full_text_search: true,
            fuzzy_search: false,
            semantic_search: false,
            filtering: true,
            sorting: true,
            pagination: true,
            faceted_search: false,
            real_time: false,
            max_query_length: Some(1000),
            result_formats: vec!["json".to_string()],
        }
    }

    fn metadata(&self) -> SearchServiceMetadata {
        SearchServiceMetadata {
            name: "Terraphim Search".to_string(),
            version: "1.0.0".to_string(),
            description: "Terraphim AI search service".to_string(),
            provider: "Terraphim".to_string(),
            endpoints: vec![],
            authentication: AuthenticationInfo {
                auth_type: AuthType::None,
                required_credentials: vec![],
                auth_endpoints: None,
            },
            rate_limits: RateLimits {
                requests_per_second: Some(10),
                requests_per_minute: Some(600),
                requests_per_hour: None,
                requests_per_day: None,
                concurrent_requests: Some(5),
            },
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        // Simple health check - in a real implementation, this would ping the service
        Ok(())
    }
}

/// Adapter for existing AutocompleteEngine to work with the trait system
pub struct TerraphimAutocompleteServiceAdapter {
    inner: crate::autocomplete::AutocompleteEngine,
}

impl TerraphimAutocompleteServiceAdapter {
    /// Create new adapter from existing AutocompleteEngine
    pub fn new(engine: crate::autocomplete::AutocompleteEngine) -> Self {
        Self { inner: engine }
    }
}

#[async_trait]
impl AutocompleteService for TerraphimAutocompleteServiceAdapter {
    type Request = AutocompleteServiceRequest;
    type Response = Vec<AutocompleteSuggestion>;
    type Error = AutocompleteServiceError;

    async fn autocomplete(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        // Convert the suggestions to our format
        let suggestions = self.inner.autocomplete(&request.query, request.max_results);
        let result: Vec<AutocompleteSuggestion> = suggestions.into_iter()
            .map(|s| AutocompleteSuggestion {
                term: s.term,
                normalized_term: s.nterm,
                url: s.url,
                score: s.score,
            })
            .collect();

        if result.is_empty() {
            Err(AutocompleteServiceError::NoResults)
        } else {
            Ok(result)
        }
    }

    fn capabilities(&self) -> AutocompleteCapabilities {
        AutocompleteCapabilities {
            fuzzy_matching: true,
            semantic_suggestions: false,
            ranked_suggestions: true,
            multiple_types: true,
            min_query_length: 1,
            max_suggestions: Some(50),
            suggestion_types: vec!["term".to_string()],
        }
    }

    fn metadata(&self) -> AutocompleteServiceMetadata {
        AutocompleteServiceMetadata {
            name: "Terraphim Autocomplete".to_string(),
            version: "1.0.0".to_string(),
            description: "Terraphim AI autocomplete service".to_string(),
            provider: "Terraphim".to_string(),
            data_sources: vec!["knowledge-graph".to_string()],
            update_frequency: "daily".to_string(),
            authentication: AuthenticationInfo {
                auth_type: AuthType::None,
                required_credentials: vec![],
                auth_endpoints: None,
            },
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        // Simple health check - in a real implementation, this would ping the service
        Ok(())
    }
}

/// Service health checker
pub struct ServiceHealthChecker;

impl ServiceHealthChecker {
    /// Check health of all registered services
    pub async fn check_all_services(
        registry: &SearchServiceRegistry,
    ) -> HashMap<String, Result<(), String>> {
        let mut results = HashMap::new();

        // Check search services
        for name in registry.list_search_services() {
            if let Some(service) = registry.get_search_service(&name) {
                match service.health_check().await {
                    Ok(()) => {
                        results.insert(name, Ok(()));
                    }
                    Err(e) => {
                        results.insert(name, Err(e.to_string()));
                    }
                }
            }
        }

        // Check autocomplete services
        for name in registry.list_autocomplete_services() {
            if let Some(service) = registry.get_autocomplete_service(&name) {
                match service.health_check().await {
                    Ok(()) => {
                        results.insert(format!("autocomplete:{}", name), Ok(()));
                    }
                    Err(e) => {
                        results.insert(format!("autocomplete:{}", name), Err(e.to_string()));
                    }
                }
            }
        }

        results
    }

    /// Get service statistics
    pub async fn get_service_statistics(
        registry: &SearchServiceRegistry,
    ) -> ServiceStatistics {
        let search_services = registry.list_search_services();
        let autocomplete_services = registry.list_autocomplete_services();

        let mut stats = ServiceStatistics {
            total_search_services: search_services.len(),
            total_autocomplete_services: autocomplete_services.len(),
            healthy_search_services: 0,
            healthy_autocomplete_services: 0,
            search_capabilities: Vec::new(),
            autocomplete_capabilities: Vec::new(),
        };

        // Check search service health and collect capabilities
        for name in search_services {
            if let Some(service) = registry.get_search_service(&name) {
                if service.health_check().await.is_ok() {
                    stats.healthy_search_services += 1;
                }
                stats.search_capabilities.push((name, service.capabilities()));
            }
        }

        // Check autocomplete service health and collect capabilities
        for name in autocomplete_services {
            if let Some(service) = registry.get_autocomplete_service(&name) {
                if service.health_check().await.is_ok() {
                    stats.healthy_autocomplete_services += 1;
                }
                stats.autocomplete_capabilities.push((name, service.capabilities()));
            }
        }

        stats
    }
}

/// Service statistics
#[derive(Debug, Clone)]
pub struct ServiceStatistics {
    /// Total number of search services
    pub total_search_services: usize,

    /// Total number of autocomplete services
    pub total_autocomplete_services: usize,

    /// Number of healthy search services
    pub healthy_search_services: usize,

    /// Number of healthy autocomplete services
    pub healthy_autocomplete_services: usize,

    /// Search service capabilities
    pub search_capabilities: Vec<(String, SearchCapabilities)>,

    /// Autocomplete service capabilities
    pub autocomplete_capabilities: Vec<(String, AutocompleteCapabilities)>,
}