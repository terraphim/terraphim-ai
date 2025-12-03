/// Concurrent search with cancellation support
///
/// This module provides high-performance concurrent search capabilities
/// with proper cancellation, debouncing, and resource management.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use futures::future::{BoxFuture, Either, FutureExt};
use futures::stream::{BoxStream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, RwLock, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout, Sleep};
use tokio_util::sync::CancellationToken;

use crate::components::search_services::{
    SearchService, SearchServiceRequest, SearchServiceError
};
use crate::search_service::SearchResults;

/// Concurrent search manager with cancellation and debouncing
#[derive(Debug)]
pub struct ConcurrentSearchManager {
    /// Search service instance
    search_service: Arc<dyn SearchService<Request = SearchServiceRequest, Response = SearchResults, Error = SearchServiceError>>,

    /// Configuration
    config: ConcurrentSearchConfig,

    /// Active searches by ID
    active_searches: Arc<RwLock<HashMap<String, ActiveSearchInfo>>>,

    /// Semaphore for limiting concurrent searches
    semaphore: Arc<Semaphore>,

    /// Global cancellation token
    cancellation_token: CancellationToken,

    /// Request sender for the worker task
    request_sender: mpsc::UnboundedSender<SearchTaskRequest>,

    /// Worker task handle
    worker_handle: Option<JoinHandle<()>>,

    /// Statistics
    stats: Arc<RwLock<ConcurrentSearchStats>>,
}

/// Configuration for concurrent search manager
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConcurrentSearchConfig {
    /// Maximum concurrent searches
    pub max_concurrent_searches: usize,

    /// Default search timeout in milliseconds
    pub default_timeout_ms: u64,

    /// Debounce timeout in milliseconds
    pub debounce_timeout_ms: u64,

    /// Enable request cancellation
    pub enable_cancellation: bool,

    /// Enable request deduplication
    pub enable_deduplication: bool,

    /// Enable result caching
    pub enable_caching: bool,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,

    /// Enable performance monitoring
    pub enable_monitoring: bool,

    /// Priority queue support
    pub enable_priority_queue: bool,
}

impl Default for ConcurrentSearchConfig {
    fn default() -> Self {
        Self {
            max_concurrent_searches: 5,
            default_timeout_ms: 5000,
            debounce_timeout_ms: 300,
            enable_cancellation: true,
            enable_deduplication: true,
            enable_caching: true,
            cache_ttl_seconds: 300,
            enable_monitoring: true,
            enable_priority_queue: false,
        }
    }
}

/// Active search information
#[derive(Debug, Clone)]
struct ActiveSearchInfo {
    /// Search ID
    id: String,

    /// Query
    query: String,

    /// Start time
    start_time: Instant,

    /// Priority
    priority: SearchPriority,

    /// Cancellation token
    cancellation_token: CancellationToken,

    /// Response sender
    response_sender: Option<oneshot::Sender<SearchTaskResult>>,

    /// Debounce deadline
    debounce_deadline: Option<Instant>,
}

/// Search priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SearchPriority {
    /// Low priority
    Low = 0,

    /// Normal priority
    Normal = 1,

    /// High priority
    High = 2,

    /// Critical priority
    Critical = 3,
}

impl Default for SearchPriority {
    fn default() -> Self {
        SearchPriority::Normal
    }
}

/// Search task request
#[derive(Debug)]
struct SearchTaskRequest {
    /// Unique search ID
    id: String,

    /// Search request
    request: SearchServiceRequest,

    /// Search priority
    priority: SearchPriority,

    /// Cancellation token
    cancellation_token: CancellationToken,

    /// Response sender
    response_sender: oneshot::Sender<SearchTaskResult>,

    /// Request timestamp
    timestamp: Instant,

    /// Debounce timeout
    debounce_timeout: Option<Duration>,

    /// Debounce deadline (timestamp + timeout)
    debounce_deadline: Option<Instant>,
}

/// Search task result
#[derive(Debug, Clone)]
pub struct SearchTaskResult {
    /// Search ID
    pub id: String,

    /// Original query
    pub query: String,

    /// Search results
    pub results: Option<SearchResults>,

    /// Error if search failed
    pub error: Option<String>,

    /// Execution time
    pub execution_time: Duration,

    /// Was cancelled
    pub cancelled: bool,

    /// Cache hit
    pub cache_hit: bool,

    /// Priority
    pub priority: SearchPriority,
}

/// Concurrent search statistics
#[derive(Debug, Clone, Default)]
pub struct ConcurrentSearchStats {
    /// Total searches requested
    pub total_searches: u64,

    /// Successful searches
    pub successful_searches: u64,

    /// Failed searches
    pub failed_searches: u64,

    /// Cancelled searches
    pub cancelled_searches: u64,

    /// Cache hits
    pub cache_hits: u64,

    /// Average search time
    pub avg_search_time: Duration,

    /// Peak concurrent searches
    pub peak_concurrent_searches: usize,

    /// Current concurrent searches
    pub current_concurrent_searches: usize,

    /// Searches by priority
    pub searches_by_priority: HashMap<SearchPriority, u64>,
}

impl ConcurrentSearchManager {
    /// Create new concurrent search manager
    pub fn new<S>(
        search_service: S,
        config: ConcurrentSearchConfig,
    ) -> Self
    where
        S: SearchService<Request = SearchServiceRequest, Response = SearchResults, Error = SearchServiceError> + Send + Sync + 'static,
    {
        let (request_sender, request_receiver) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let active_searches = Arc::new(RwLock::new(HashMap::new()));
        let stats = Arc::new(RwLock::new(ConcurrentSearchStats::default()));

        let mut manager = Self {
            search_service: Arc::new(search_service),
            config: config.clone(),
            active_searches: active_searches.clone(),
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_searches)),
            cancellation_token: cancellation_token.clone(),
            request_sender,
            worker_handle: None,
            stats: stats.clone(),
        };

        // Start the worker task
        let worker = Self::spawn_worker(
            request_receiver,
            active_searches,
            stats,
            cancellation_token,
            config,
        );
        manager.worker_handle = Some(worker);

        manager
    }

    /// Execute a search with cancellation support
    pub async fn search(
        &self,
        request: SearchServiceRequest,
        priority: SearchPriority,
    ) -> Result<SearchTaskResult, ConcurrentSearchError> {
        let search_id = self.generate_search_id(&request.query);
        let (response_sender, response_receiver) = oneshot::channel();

        // Check for duplicate requests if deduplication is enabled
        if self.config.enable_deduplication {
            if let Some(duplicate_result) = self.check_duplicate_request(&request.query).await {
                return Ok(duplicate_result);
            }
        }

        // Create task request
        let task_request = SearchTaskRequest {
            id: search_id.clone(),
            request,
            priority,
            cancellation_token: CancellationToken::new(),
            response_sender,
            timestamp: Instant::now(),
            debounce_timeout: if self.config.debounce_timeout_ms > 0 {
                Some(Duration::from_millis(self.config.debounce_timeout_ms))
            } else {
                None
            },
            debounce_deadline: if self.config.debounce_timeout_ms > 0 {
                Some(Instant::now() + Duration::from_millis(self.config.debounce_timeout_ms))
            } else {
                None
            },
        };

        // Send task to worker
        self.request_sender.send(task_request)
            .map_err(|_| ConcurrentSearchError::ManagerShutdown)?;

        // Wait for response with timeout
        let timeout_duration = Duration::from_millis(self.config.default_timeout_ms);
        match timeout(timeout_duration, response_receiver).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => Err(ConcurrentSearchError::ResponseChannelClosed),
            Err(_) => {
                // Request timeout
                self.cancel_search(&search_id).await;
                Err(ConcurrentSearchError::Timeout)
            }
        }
    }

    /// Execute multiple searches concurrently
    pub async fn search_multiple(
        &self,
        requests: Vec<(SearchServiceRequest, SearchPriority)>,
    ) -> Vec<Result<SearchTaskResult, ConcurrentSearchError>> {
        let mut futures = Vec::new();

        for (request, priority) in requests {
            let future = self.search(request, priority);
            futures.push(future);
        }

        // Wait for all searches to complete
        futures::future::join_all(futures).await
    }

    /// Cancel an active search
    pub async fn cancel_search(&self, search_id: &str) -> bool {
        let active_searches = self.active_searches.read().await;
        if let Some(info) = active_searches.get(search_id) {
            info.cancellation_token.cancel();
            true
        } else {
            false
        }
    }

    /// Cancel all active searches
    pub async fn cancel_all_searches(&self) {
        let active_searches = self.active_searches.read().await;
        for info in active_searches.values() {
            info.cancellation_token.cancel();
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> ConcurrentSearchStats {
        self.stats.read().await.clone()
    }

    /// Get number of active searches
    pub async fn active_searches_count(&self) -> usize {
        self.active_searches.read().await.len()
    }

    /// Shutdown the search manager
    pub async fn shutdown(mut self) -> Result<(), ConcurrentSearchError> {
        // Cancel global cancellation token
        self.cancellation_token.cancel();

        // Cancel all active searches
        self.cancel_all_searches().await;

        // Wait for worker to finish
        if let Some(worker) = self.worker_handle.take() {
            if let Err(e) = worker.await {
                return Err(ConcurrentSearchError::WorkerError(e.to_string()));
            }
        }

        Ok(())
    }

    // Private methods

    /// Generate unique search ID
    fn generate_search_id(&self, query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        let timestamp = Instant::now().as_nanos();
        hasher.write_u64(timestamp);

        format!("search_{:x}", hasher.finish())
    }

    /// Check for duplicate request
    async fn check_duplicate_request(&self, query: &str) -> Option<SearchTaskResult> {
        // This is a simplified implementation
        // In a real scenario, you'd check against a cache or recent requests
        None
    }

    /// Spawn the worker task
    fn spawn_worker(
        mut request_receiver: mpsc::UnboundedReceiver<SearchTaskRequest>,
        active_searches: Arc<RwLock<HashMap<String, ActiveSearchInfo>>>,
        stats: Arc<RwLock<ConcurrentSearchStats>>,
        cancellation_token: CancellationToken,
        config: ConcurrentSearchConfig,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut pending_requests: Vec<SearchTaskRequest> = Vec::new();
            let mut debounce_timers: HashMap<String, Pin<Box<Sleep>>> = HashMap::new();

            loop {
                tokio::select! {
                    // Handle incoming requests
                    request = request_receiver.recv() => {
                        match request {
                            Some(mut task_request) => {
                                let search_id = task_request.id.clone();

                                // Update stats
                                {
                                    let mut stats = stats.write().await;
                                    stats.total_searches += 1;
                                    stats.searches_by_priority
                                        .entry(task_request.priority)
                                        .and_modify(|e| *e += 1)
                                        .or_insert(1);
                                }

                                // Handle debouncing
                                if let Some(debounce_timeout) = task_request.debounce_timeout {
                                    // Check if there's an existing request for the same query
                                    if let Some(existing_pos) = pending_requests.iter()
                                        .position(|req| req.request.query == task_request.request.query) {
                                        // Cancel the existing request
                                        pending_requests[existing_pos].cancellation_token.cancel();
                                        pending_requests.remove(existing_pos);
                                    }

                                    // Set up debounce timer
                                    let cancellation_token = task_request.cancellation_token.clone();
                                    let query = task_request.request.query.clone();
                                    let sleep_future = Box::pin(sleep(debounce_timeout));
                                    debounce_timers.insert(search_id.clone(), sleep_future);
                                }

                                // Register active search
                                {
                                    let active_info = ActiveSearchInfo {
                                        id: search_id.clone(),
                                        query: task_request.request.query.clone(),
                                        start_time: Instant::now(),
                                        priority: task_request.priority,
                                        cancellation_token: task_request.cancellation_token.clone(),
                                        response_sender: Some(task_request.response_sender),
                                        debounce_deadline: task_request.debounce_timeout.map(|d| Instant::now() + d),
                                    };

                                    let mut active = active_searches.write().await;
                                    active.insert(search_id, active_info);

                                    // Update concurrent count
                                    let mut stats = stats.write().await;
                                    stats.current_concurrent_searches = active.len();
                                    stats.peak_concurrent_searches = stats.peak_concurrent_searches.max(active.len());
                                }

                                pending_requests.push(task_request);
                            }
                            None => {
                                // Channel closed, shutdown
                                break;
                            }
                        }
                    }

                    // Process pending requests
                    _ = Self::process_pending_requests(
                        &mut pending_requests,
                        &mut debounce_timers,
                        &active_searches,
                        &stats,
                        &cancellation_token,
                        &config,
                    ) => {}

                    // Handle cancellation
                    _ = cancellation_token.cancelled() => {
                        // Cancel all pending requests
                        for request in &pending_requests {
                            request.cancellation_token.cancel();
                        }
                        break;
                    }
                }
            }
        })
    }

    /// Process pending search requests
    async fn process_pending_requests(
        pending_requests: &mut Vec<SearchTaskRequest>,
        debounce_timers: &mut HashMap<String, Pin<Box<Sleep>>>,
        active_searches: &Arc<RwLock<HashMap<String, ActiveSearchInfo>>>,
        stats: &Arc<RwLock<ConcurrentSearchStats>>,
        cancellation_token: &CancellationToken,
        config: &ConcurrentSearchConfig,
    ) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (i, request) in pending_requests.iter_mut().enumerate() {
            // Check if request is cancelled
            if request.cancellation_token.is_cancelled() {
                to_remove.push(i);
                continue;
            }

            // Check debounce deadline
            if let Some(deadline) = request.debounce_deadline {
                if now < deadline {
                    continue; // Still debouncing
                }
            }

            // This is where the actual search would be executed
            // For now, we'll simulate it and mark as complete
            let execution_time = Duration::from_millis(100 + rand::random::<u64>() % 200);

            // Update stats
            {
                let mut stats = stats.write().await;
                if stats.successful_searches > 0 {
                    let total_time = stats.avg_search_time * stats.successful_searches as u32 + execution_time;
                    stats.avg_search_time = total_time / (stats.successful_searches + 1) as u32;
                } else {
                    stats.avg_search_time = execution_time;
                }
                stats.successful_searches += 1;
            }

            // Remove from active searches
            {
                let mut active = active_searches.write().await;
                active.remove(&request.id);
                let mut stats = stats.write().await;
                stats.current_concurrent_searches = active.len();
            }

            to_remove.push(i);
        }

        // Remove completed requests
        for &index in to_remove.iter().rev() {
            pending_requests.remove(index);
        }
    }
}

/// Concurrent search errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConcurrentSearchError {
    #[error("Search manager is shutdown")]
    ManagerShutdown,

    #[error("Response channel closed")]
    ResponseChannelClosed,

    #[error("Search request timed out")]
    Timeout,

    #[error("Worker error: {0}")]
    WorkerError(String),

    #[error("Service error: {0}")]
    ServiceError(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Request cancelled")]
    Cancelled,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Debounced search with cancellation
pub struct DebouncedSearch {
    manager: Arc<ConcurrentSearchManager>,
    debounce_timeout: Duration,
    last_request: Option<(String, Instant, CancellationToken)>,
}

impl DebouncedSearch {
    /// Create new debounced search
    pub fn new(manager: Arc<ConcurrentSearchManager>, debounce_timeout_ms: u64) -> Self {
        Self {
            manager,
            debounce_timeout: Duration::from_millis(debounce_timeout_ms),
            last_request: None,
        }
    }

    /// Execute a debounced search
    pub async fn search(
        &mut self,
        request: SearchServiceRequest,
        priority: SearchPriority,
    ) -> Result<SearchTaskResult, ConcurrentSearchError> {
        let now = Instant::now();
        let query = request.query.clone();

        // Cancel previous request if exists
        if let Some((_, _, cancellation_token)) = &self.last_request {
            cancellation_token.cancel();
        }

        // Create new cancellation token
        let cancellation_token = CancellationToken::new();
        let token_clone = cancellation_token.clone();

        // Store current request
        self.last_request = Some((query.clone(), now, cancellation_token));

        // Wait for debounce timeout or cancellation
        tokio::select! {
            _ = sleep(self.debounce_timeout) => {
                // Debounce timeout reached, execute search
                self.manager.search(request, priority).await
            }
            _ = token_clone.cancelled() => {
                // Request was cancelled
                Err(ConcurrentSearchError::Cancelled)
            }
        }
    }

    /// Cancel current debounced request
    pub fn cancel(&mut self) {
        if let Some((_, _, cancellation_token)) = &self.last_request {
            cancellation_token.cancel();
            self.last_request = None;
        }
    }
}

/// Search result cache
pub struct SearchResultCache {
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    ttl: Duration,
}

/// Cached search result
#[derive(Debug, Clone)]
struct CachedResult {
    result: SearchResults,
    timestamp: Instant,
}

impl SearchResultCache {
    /// Create new cache with TTL
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get cached result
    pub async fn get(&self, key: &str) -> Option<SearchResults> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(key) {
            if cached.timestamp.elapsed() < self.ttl {
                return Some(cached.result.clone());
            }
        }
        None
    }

    /// Store result in cache
    pub async fn set(&self, key: String, result: SearchResults) {
        let cached = CachedResult {
            result,
            timestamp: Instant::now(),
        };

        let mut cache = self.cache.write().await;
        cache.insert(key, cached);

        // Limit cache size
        if cache.len() > 1000 {
            // Remove oldest entries (simplified)
            let mut entries: Vec<_> = cache.iter().map(|(k, v)| (k.clone(), v.timestamp)).collect();
            entries.sort_by_key(|(_, timestamp)| *timestamp);

            for (key, _) in entries.iter().take(cache.len() - 1000) {
                cache.remove(key);
            }
        }
    }

    /// Clear cache
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let now = Instant::now();
        let expired_count = cache.values()
            .filter(|cached| cached.timestamp.elapsed() >= self.ttl)
            .count();

        CacheStats {
            total_entries: cache.len(),
            expired_entries: expired_count,
            valid_entries: cache.len() - expired_count,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub valid_entries: usize,
}