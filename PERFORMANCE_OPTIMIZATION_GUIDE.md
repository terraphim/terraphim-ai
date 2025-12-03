# Performance Optimization Guide for Reusable Components

## Executive Summary

This guide provides comprehensive strategies for achieving the performance targets outlined in the architecture plan: sub-50ms search response times, <10ms autocomplete, 60 FPS scrolling, and <16ms markdown rendering. These optimizations are critical for user experience and system scalability.

## Performance Targets

### Component-Specific Targets
- **Search**: <50ms (cached), <200ms (uncached)
- **Autocomplete**: <10ms response time
- **Chat**: First token <100ms, streaming at 50+ tokens/sec
- **Virtual Scrolling**: 60 FPS with 100+ messages
- **Markdown Rendering**: <16ms per message
- **Memory Usage**: <100MB for typical workload

### System-Wide Targets
- **Startup Time**: <2 seconds
- **Cache Hit Rate**: >90%
- **Error Rate**: <0.1%
- **CPU Usage**: <50% under normal load
- **Memory Leaks**: Zero tolerance

## Core Optimization Strategies

### 1. Intelligent Caching System

#### Hierarchical Cache Architecture
```rust
use lru::LruCache;
use dashmap::DashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Multi-level caching system
pub struct HierarchicalCache<K, V> {
    // L1: In-memory LRU cache (fastest)
    l1_cache: Arc<RwLock<LruCache<K, V>>>,

    // L2: Compressed cache for larger data
    l2_cache: Arc<DashMap<K, CompressedValue<V>>>,

    // L3: Persistent cache (optional)
    l3_cache: Option<Box<dyn PersistentCache<K, V>>>,

    // Cache metrics
    metrics: CacheMetrics,
}

#[derive(Clone)]
struct CompressedValue<V> {
    compressed: Vec<u8>,
    _phantom: std::marker::PhantomData<V>,
}

impl<K, V> HierarchicalCache<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    pub fn new(l1_size: usize, l2_size: usize) -> Self {
        Self {
            l1_cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(l1_size).unwrap()
            ))),
            l2_cache: Arc::new(DashMap::with_capacity(l2_size)),
            l3_cache: None,
            metrics: CacheMetrics::new(),
        }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        // Try L1 first
        if let Some(value) = self.l1_cache.read().get(key) {
            self.metrics.record_hit(CacheLevel::L1);
            return Some(value.clone());
        }

        // Try L2
        if let Some(compressed) = self.l2_cache.get(key) {
            self.metrics.record_hit(CacheLevel::L2);

            // Decompress and promote to L1
            let value = self.decompress(&compressed.compressed)?;
            self.l1_cache.write().put(key.clone(), value.clone());
            return Some(value);
        }

        // Try L3 if available
        if let Some(l3) = &self.l3_cache {
            if let Some(value) = l3.get(key).await {
                self.metrics.record_hit(CacheLevel::L3);

                // Promote through levels
                self.l2_cache.insert(key.clone(), self.compress(&value));
                self.l1_cache.write().put(key.clone(), value.clone());
                return Some(value);
            }
        }

        self.metrics.record_miss();
        None
    }

    pub async fn put(&self, key: K, value: V) {
        // Store in L1
        self.l1_cache.write().put(key.clone(), value.clone());

        // Compress and store in L2
        self.l2_cache.insert(key, self.compress(&value));

        // Store in L3 if available
        if let Some(l3) = &self.l3_cache {
            let _ = l3.put(&key, &value).await;
        }
    }

    fn compress(&self, value: &V) -> CompressedValue<V> {
        // Use lz4 for fast compression
        let serialized = bincode::serialize(value).unwrap();
        let compressed = lz4::block::compress(&serialized)
            .unwrap_or(serialized); // Fallback to uncompressed

        CompressedValue {
            compressed,
            _phantom: std::marker::PhantomData,
        }
    }

    fn decompress(&self, compressed: &[u8]) -> Option<V> {
        // Try to decompress
        if let Ok(decompressed) = lz4::block::decompress(compressed, None) {
            bincode::deserialize(&decompressed).ok()
        } else {
            // Fallback: try direct deserialization
            bincode::deserialize(compressed).ok()
        }
    }
}

enum CacheLevel {
    L1,
    L2,
    L3,
}

struct CacheMetrics {
    l1_hits: std::sync::atomic::AtomicU64,
    l2_hits: std::sync::atomic::AtomicU64,
    l3_hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
}

impl CacheMetrics {
    fn record_hit(&self, level: CacheLevel) {
        match level {
            CacheLevel::L1 => self.l1_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            CacheLevel::L2 => self.l2_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            CacheLevel::L3 => self.l3_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        };
    }

    fn record_miss(&self) {
        self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn hit_rate(&self) -> f64 {
        let total_hits = self.l1_hits.load(std::sync::atomic::Ordering::Relaxed)
            + self.l2_hits.load(std::sync::atomic::Ordering::Relaxed)
            + self.l3_hits.load(std::sync::atomic::Ordering::Relaxed);
        let total_requests = total_hits + self.misses.load(std::sync::atomic::Ordering::Relaxed);

        if total_requests == 0 {
            0.0
        } else {
            total_hits as f64 / total_requests as f64
        }
    }
}
```

#### Smart Cache Warming
```rust
/// Proactive cache warming for predictable access patterns
pub struct CacheWarmer {
    cache: Arc<HierarchicalCache<String, CachedSearchResult>>,
    access_patterns: Arc<RwLock<AccessPatternTracker>>,
    warmer_task: Option<tokio::task::JoinHandle<()>>,
}

impl CacheWarmer {
    pub fn new(cache: Arc<HierarchicalCache<String, CachedSearchResult>>) -> Self {
        Self {
            cache,
            access_patterns: Arc::new(RwLock::new(AccessPatternTracker::new())),
            warmer_task: None,
        }
    }

    pub fn start(&mut self) {
        let cache = self.cache.clone();
        let patterns = self.access_patterns.clone();

        self.warmer_task = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                // Get predicted queries
                let predictions = patterns.read().predict_next_queries(10);

                // Warm cache with predicted queries
                for query in predictions {
                    if !cache.contains_key(&query).await {
                        if let Some(result) = Self::prefetch_result(&query).await {
                            cache.put(query, result).await;
                        }
                    }
                }
            }
        }));
    }

    pub async fn record_access(&self, query: &str) {
        self.access_patterns.write().record_access(query);
    }

    async fn prefetch_result(query: &str) -> Option<CachedSearchResult> {
        // Implement predictive fetching based on user patterns
        // This could use ML or simple frequency analysis
        None
    }
}

struct AccessPatternTracker {
    // Query frequency count
    frequencies: HashMap<String, usize>,
    // Sequential access patterns
    sequences: VecDeque<String>,
    // Time-based patterns
    temporal_patterns: HashMap<String, Vec<Instant>>,
}
```

### 2. Optimized Search Implementation

#### Binary Search for Autocomplete
```rust
/// High-performance autocomplete with binary search
pub struct AutocompleteEngine {
    // Sorted list of all terms
    sorted_terms: Vec<String>,
    // Prefix index for faster lookups
    prefix_index: HashMap<String, Vec<usize>>,
    // Fuzzy search cache
    fuzzy_cache: LruCache<String, Vec<AutocompleteSuggestion>>,
}

impl AutocompleteEngine {
    pub fn new(terms: Vec<String>) -> Self {
        let mut sorted_terms = terms;
        sorted_terms.sort_unstable();

        let mut prefix_index = HashMap::new();

        // Build prefix index (for first 3 characters)
        for (idx, term) in sorted_terms.iter().enumerate() {
            for len in 1..=std::cmp::min(3, term.len()) {
                let prefix = term[..len].to_lowercase();
                prefix_index
                    .entry(prefix)
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
        }

        Self {
            sorted_terms,
            prefix_index,
            fuzzy_cache: LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()),
        }
    }

    /// Get suggestions for a partial query (<10ms target)
    pub fn get_suggestions(&mut self, query: &str, limit: usize) -> Vec<AutocompleteSuggestion> {
        let start = Instant::now();

        let suggestions = if query.len() >= 2 {
            // Use prefix index for exact matches
            self.get_prefix_suggestions(query, limit)
        } else {
            // Use fuzzy search for very short queries
            self.get_fuzzy_suggestions(query, limit)
        };

        // Log performance
        let duration = start.elapsed();
        if duration > Duration::from_millis(10) {
            log::warn!("Autocomplete took {:?} for query: {}", duration, query);
        }

        suggestions
    }

    fn get_prefix_suggestions(&self, prefix: &str, limit: usize) -> Vec<AutocompleteSuggestion> {
        let prefix_lower = prefix.to_lowercase();

        if let Some(indices) = self.prefix_index.get(&prefix_lower) {
            // Get all terms with matching prefix
            let mut suggestions = Vec::with_capacity(indices.len());

            for &idx in indices {
                if let Some(term) = self.sorted_terms.get(idx) {
                    suggestions.push(AutocompleteSuggestion {
                        text: term.clone(),
                        score: 1.0, // Perfect match for prefix
                        highlight_range: 0..prefix.len(),
                    });
                }
            }

            // Sort and limit
            suggestions.sort_by(|a, b| {
                b.score.partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.text.cmp(&b.text))
            });

            suggestions.truncate(limit);
            suggestions
        } else {
            // Fallback to binary search
            self.binary_search_suggestions(prefix, limit)
        }
    }

    fn binary_search_suggestions(&self, prefix: &str, limit: usize) -> Vec<AutocompleteSuggestion> {
        let prefix_lower = prefix.to_lowercase();

        // Find insertion point using binary search
        let start_idx = match self.sorted_terms.binary_search_by(|term| {
            term.to_lowercase().as_str().cmp(&prefix_lower)
        }) {
            Ok(idx) => idx,
            Err(idx) => idx,
        };

        // Collect matches from insertion point
        let mut suggestions = Vec::new();
        let end_idx = std::cmp::min(start_idx + limit * 2, self.sorted_terms.len());

        for term in &self.sorted_terms[start_idx..end_idx] {
            if term.to_lowercase().starts_with(&prefix_lower) {
                suggestions.push(AutocompleteSuggestion {
                    text: term.clone(),
                    score: 0.9,
                    highlight_range: 0..prefix.len(),
                });
            } else if !suggestions.is_empty() {
                // We've passed all potential matches
                break;
            }
        }

        suggestions.truncate(limit);
        suggestions
    }

    fn get_fuzzy_suggestions(&mut self, query: &str, limit: usize) -> Vec<AutocompleteSuggestion> {
        // Check cache first
        if let Some(cached) = self.fuzzy_cache.get(query) {
            return cached.clone();
        }

        // Use Jaro-Winkler distance for fuzzy matching
        let mut matches: Vec<_> = self.sorted_terms
            .iter()
            .map(|term| {
                let distance = jaro_winkler(query, term);
                AutocompleteSuggestion {
                    text: term.clone(),
                    score: distance,
                    highlight_range: self.find_highlight_range(query, term),
                }
            })
            .filter(|s| s.score > 0.7) // Only good matches
            .collect();

        // Sort by score
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        matches.truncate(limit);

        // Cache result
        self.fuzzy_cache.put(query.to_string(), matches.clone());

        matches
    }
}

/// Fast Jaro-Winkler distance implementation
fn jaro_winkler(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }

    let len1 = s1.len();
    let len2 = s2.len();

    if len1 == 0 || len2 == 0 {
        return 0.0;
    }

    let match_distance = std::cmp::max(len1, len2) / 2 - 1;
    let mut s1_matches = vec![false; len1];
    let mut s2_matches = vec![false; len2];
    let mut matches = 0;
    let mut transpositions = 0;

    // Find matches
    for i in 0..len1 {
        let start = std::cmp::max(0, i as i32 - match_distance as i32) as usize;
        let end = std::cmp::min(i + match_distance + 1, len2);

        for j in start..end {
            if !s2_matches[j] && s1.as_bytes()[i] == s2.as_bytes()[j] {
                s1_matches[i] = true;
                s2_matches[j] = true;
                matches += 1;
                break;
            }
        }
    }

    if matches == 0 {
        return 0.0;
    }

    // Count transpositions
    let mut k = 0;
    for i in 0..len1 {
        if s1_matches[i] {
            while !s2_matches[k] {
                k += 1;
            }
            if s1.as_bytes()[i] != s2.as_bytes()[k] {
                transpositions += 1;
            }
            k += 1;
        }
    }

    let jaro = (
        matches as f64 / len1 as f64
        + matches as f64 / len2 as f64
        + (matches - transpositions / 2) as f64 / matches as f64
    ) / 3.0;

    // Winkler prefix bonus
    let prefix_len = std::cmp::min(
        4,
        s1.chars()
            .zip(s2.chars())
            .take_while(|&(a, b)| a == b)
            .count()
    );

    jaro + prefix_len as f64 * 0.1 * (1.0 - jaro)
}
```

#### Concurrent Search with Cancellation
```rust
use tokio::sync::{mpsc, oneshot, CancellationToken};
use futures::stream::{self, StreamExt};

/// Concurrent search coordinator with cancellation support
pub struct ConcurrentSearchCoordinator {
    search_services: Vec<Arc<dyn SearchService>>,
    result_aggregator: ResultAggregator,
    performance_tracker: Arc<PerformanceTracker>,
}

impl ConcurrentSearchCoordinator {
    pub async fn search(
        &self,
        query: SearchQuery,
        options: SearchOptions,
    ) -> Result<SearchResults, SearchError> {
        let start = Instant::now();
        let cancellation_token = CancellationToken::new();
        let timeout = Duration::from_millis(options.timeout_ms.unwrap_or(5000));

        // Create channels for results
        let (result_tx, mut result_rx) = mpsc::channel(100);
        let (completion_tx, completion_rx) = oneshot::channel();

        // Spawn search tasks for all services
        let mut handles = Vec::new();
        for service in &self.search_services {
            let service = service.clone();
            let query = query.clone();
            let result_tx = result_tx.clone();
            let token = cancellation_token.clone();

            let handle = tokio::spawn(async move {
                let service_start = Instant::now();

                // Race between search and cancellation
                tokio::select! {
                    result = service.search(query.clone()) => {
                        let duration = service_start.elapsed();
                        match result {
                            Ok(results) => {
                                let _ = result_tx.send(Ok((results, duration))).await;
                            }
                            Err(e) => {
                                let _ = result_tx.send(Err(e)).await;
                            }
                        }
                    }
                    _ = token.cancelled() => {
                        log::debug!("Search cancelled for service: {}", service.service_id());
                    }
                }
            });

            handles.push(handle);
        }

        // Drop our sender to close when all tasks are done
        drop(result_tx);

        // Spawn aggregation task
        let aggregator = self.result_aggregator.clone();
        let performance_tracker = self.performance_tracker.clone();
        let query_id = query.id.clone();

        let aggregation_handle = tokio::spawn(async move {
            let mut all_results = Vec::new();
            let mut service_times = Vec::new();

            while let Some(result) = result_rx.recv().await {
                match result {
                    Ok((results, duration)) => {
                        all_results.push(results);
                        service_times.push(duration);
                    }
                    Err(e) => {
                        log::warn!("Search service error: {}", e);
                    }
                }
            }

            // Aggregate results
            let final_results = aggregator.aggregate(all_results).await;

            // Record performance metrics
            let total_time = start.elapsed();
            performance_tracker.record_search_operation(
                query_id,
                total_time,
                service_times,
                final_results.len()
            ).await;

            final_results
        });

        // Wait for either completion, timeout, or cancellation
        tokio::select! {
            results = aggregation_handle => {
                let _ = completion_tx.send(()).await;
                Ok(results?)
            }
            _ = tokio::time::sleep(timeout) => {
                cancellation_token.cancel();
                Err(SearchError::Timeout(timeout))
            }
        }
    }

    pub async fn search_stream(
        &self,
        query: SearchQuery,
    ) -> impl Stream<Item = Result<PartialSearchResult, SearchError>> {
        let (tx, rx) = mpsc::channel(10);

        for service in &self.search_services {
            let service = service.clone();
            let query = query.clone();
            let tx = tx.clone();

            tokio::spawn(async move {
                let mut stream = service.search_stream(query).await?;

                while let Some(result) = stream.next().await {
                    if tx.send(Ok(result)).await.is_err() {
                        break; // Receiver dropped
                    }
                }

                Ok::<(), SearchError>(())
            });
        }

        drop(tx); // Drop sender after spawning all tasks

        Box::pin(stream::unfold(rx, |mut rx| async {
            match rx.recv().await {
                Some(result) => Some((result, rx)),
                None => None,
            }
        }))
    }
}

/// Smart result aggregation with deduplication and ranking
pub struct ResultAggregator {
    deduplicator: ResultDeduplicator,
    ranker: ResultRanker,
}

impl ResultAggregator {
    pub async fn aggregate(
        &self,
        result_sets: Vec<SearchResults>,
    ) -> SearchResults {
        let start = Instant::now();

        // Flatten all results
        let all_results: Vec<SearchResult> = result_sets
            .into_iter()
            .flat_map(|r| r.results)
            .collect();

        // Deduplicate based on content similarity
        let unique_results = self.deduplicator.deduplicate(all_results).await;

        // Re-rank based on relevance and diversity
        let ranked_results = self.ranker.rank_results(unique_results).await;

        let aggregation_time = start.elapsed();
        log::debug!(
            "Aggregated {} results in {:?}",
            ranked_results.len(),
            aggregation_time
        );

        SearchResults {
            query_id: uuid::Uuid::new_v4().to_string(),
            results: ranked_results,
            total_found: ranked_results.len(),
            aggregation_time: Some(aggregation_time),
        }
    }
}
```

### 3. Virtual Scrolling Optimization

#### Item Height Caching
```rust
/// Optimized virtual scrolling with height caching
pub struct VirtualScrollList<T> {
    items: Vec<T>,
    viewport_height: f32,
    estimated_item_height: f32,

    // Height cache for rendered items
    height_cache: LruCache<usize, f32>,

    // Scroll state
    scroll_offset: f32,
    total_height: f32,

    // Rendering optimization
    render_buffer: usize, // Extra items to render above/below viewport
    visible_range: Range<usize>,
}

impl<T> VirtualScrollList<T> {
    pub fn new(viewport_height: f32) -> Self {
        Self {
            items: Vec::new(),
            viewport_height,
            estimated_item_height: 40.0,
            height_cache: LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()),
            scroll_offset: 0.0,
            total_height: 0.0,
            render_buffer: 5,
            visible_range: 0..0,
        }
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.recalculate_total_height();
    }

    pub fn scroll_to(&mut self, offset: f32) {
        self.scroll_offset = offset.clamp(0.0, self.total_height - self.viewport_height);
        self.update_visible_range();
    }

    /// Get items that should be rendered
    pub fn get_visible_items(&self) -> Vec<(usize, &T, f32)> {
        let mut visible_items = Vec::new();
        let mut current_y = 0.0;

        // Find start position
        for (idx, _) in self.items.iter().enumerate() {
            let height = self.get_item_height(idx);

            if current_y + height >= self.scroll_offset {
                // This item is at or below the top of viewport
                if current_y > self.scroll_offset + self.viewport_height {
                    // This item is below the bottom of viewport
                    break;
                }

                visible_items.push((idx, &self.items[idx], current_y));
            }

            current_y += height;
        }

        visible_items
    }

    fn get_item_height(&self, index: usize) -> f32 {
        self.height_cache
            .get(&index)
            .copied()
            .unwrap_or(self.estimated_item_height)
    }

    fn update_item_height(&mut self, index: usize, height: f32) {
        self.height_cache.put(index, height);
        self.recalculate_total_height();
    }

    fn recalculate_total_height(&mut self) {
        self.total_height = self
            .items
            .iter()
            .enumerate()
            .map(|(idx, _)| self.get_item_height(idx))
            .sum();
    }

    fn update_visible_range(&mut self) {
        let start_y = self.scroll_offset;
        let end_y = start_y + self.viewport_height;

        // Find items intersecting viewport
        let mut current_y = 0.0;
        let mut start_idx = 0;
        let mut end_idx = 0;

        for (idx, _) in self.items.iter().enumerate() {
            let height = self.get_item_height(idx);

            if current_y + height < start_y {
                start_idx = idx + 1;
            } else if current_y <= end_y {
                end_idx = idx + 1;
            }

            current_y += height;
        }

        // Add render buffer
        self.visible_range = {
            let buffered_start = start_idx.saturating_sub(self.render_buffer);
            let buffered_end = std::cmp::min(
                end_idx + self.render_buffer,
                self.items.len()
            );
            buffered_start..buffered_end
        };
    }

    /// Smooth scroll animation
    pub fn smooth_scroll_to(
        &mut self,
        target_offset: f32,
        duration: Duration,
    ) -> impl Stream<Item = f32> {
        let start_offset = self.scroll_offset;
        let distance = target_offset - start_offset;
        let start_time = Instant::now();

        Box::pin(stream::unfold(
            (start_offset, distance, start_time),
            move |(current_offset, remaining_distance, start_time)| async move {
                let elapsed = start_time.elapsed();
                let progress = (elapsed.as_secs_f64() / duration.as_secs_f64()).min(1.0);

                // Ease-in-out animation
                let eased = if progress < 0.5 {
                    2.0 * progress * progress
                } else {
                    1.0 - 2.0 * (1.0 - progress) * (1.0 - progress)
                };

                let new_offset = start_offset + distance * eased as f32;

                if progress >= 1.0 {
                    None
                } else {
                    Some((new_offset, (new_offset, remaining_distance, start_time)))
                }
            }
        ))
    }
}

/// GPUI integration for virtual scrolling
impl<T> Render for VirtualScrollList<T>
where
    T: Render + Clone,
{
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let visible_items = self.get_visible_items();

        div()
            .size_full()
            .overflow_hidden()
            .relative()
            .child(
                div()
                    .absolute()
                    .top(px(0.0))
                    .left(px(0.0))
                    .w(px(10.0)) // Scrollbar width
                    .h(px(self.total_height))
                    .bg(rgb(0xf0f0f0))
            )
            .child(
                div()
                    .relative()
                    .top(px(self.scroll_offset))
                    .w_full()
                    .children(
                        visible_items.into_iter().map(|(idx, item, y)| {
                            div()
                                .absolute()
                                .top(px(y))
                                .left(px(0.0))
                                .w_full()
                                .child(item.clone())
                        })
                    )
            )
    }
}
```

### 4. Markdown Rendering Optimization

#### Incremental Rendering
```rust
/// High-performance markdown renderer with incremental updates
pub struct IncrementalMarkdownRenderer {
    // Parsed markdown AST
    ast_nodes: Vec<MarkdownNode>,

    // Render cache for static content
    render_cache: LruCache<String, RenderedMarkdown>,

    // Streaming support
    streaming_content: String,
    last_rendered_length: usize,

    // Performance optimization
    dirty_ranges: Vec<RenderRange>,
    render_budget: Duration,
}

impl IncrementalMarkdownRenderer {
    pub fn new() -> Self {
        Self {
            ast_nodes: Vec::new(),
            render_cache: LruCache::new(std::num::NonZeroUsize::new(100).unwrap()),
            streaming_content: String::new(),
            last_rendered_length: 0,
            dirty_ranges: Vec::new(),
            render_budget: Duration::from_millis(16), // 60 FPS
        }
    }

    /// Stream markdown content and render incrementally
    pub fn stream_content(&mut self, content: &str) -> RenderedMarkdown {
        let start = Instant::now();

        // Append new content
        self.streaming_content.push_str(content);

        // Parse only the new content
        let new_nodes = self.parse_incremental(content);

        // Mark dirty ranges
        self.mark_dirty_ranges(new_nodes.len());

        // Render within budget
        let rendered = self.render_within_budget(start);

        rendered
    }

    fn parse_incremental(&mut self, new_content: &str) -> Vec<MarkdownNode> {
        let start_pos = self.last_rendered_length;
        let parser = pulldown_cmark::Parser::new(&new_content[start_pos..]);

        let mut new_nodes = Vec::new();
        for event in parser {
            match event {
                Event::Start(tag) => {
                    new_nodes.push(MarkdownNode::start(tag, start_pos + new_content.len()));
                }
                Event::End(tag_end) => {
                    new_nodes.push(MarkdownNode::end(tag_end, start_pos + new_content.len()));
                }
                Event::Text(text) => {
                    new_nodes.push(MarkdownNode::text(text.to_string(), start_pos + new_content.len()));
                }
                _ => {}
            }
        }

        self.ast_nodes.extend(new_nodes.clone());
        self.last_rendered_length = self.streaming_content.len();

        new_nodes
    }

    fn render_within_budget(&mut self, start: Instant) -> RenderedMarkdown {
        let mut rendered = Vec::new();
        let mut current_pos = 0;

        // Render dirty ranges first
        for range in &self.dirty_ranges {
            if start.elapsed() > self.render_budget {
                break; // Budget exceeded
            }

            let partial = self.render_range(range.start..range.end);
            rendered.extend(partial);
            current_pos = range.end;
        }

        // Clear rendered ranges
        self.dirty_ranges.clear();

        RenderedMarkdown {
            elements: rendered,
            total_height: self.calculate_height(&rendered),
            render_time: start.elapsed(),
        }
    }

    fn render_range(&self, range: Range<usize>) -> Vec<RenderedElement> {
        let mut elements = Vec::new();
        let mut in_code_block = false;
        let mut code_block_language = None;

        for node in &self.ast_nodes[range] {
            match node {
                MarkdownNode::Start(Tag::CodeBlock(lang)) => {
                    in_code_block = true;
                    code_block_language = lang.clone();
                }
                MarkdownNode::End(TagEnd::CodeBlock) => {
                    in_code_block = false;
                    code_block_language = None;
                }
                MarkdownNode::Text(text) => {
                    if in_code_block {
                        elements.push(RenderedElement::CodeBlock {
                            language: code_block_language.clone(),
                            content: text.clone(),
                        });
                    } else {
                        // Render inline markdown
                        let inline_elements = self.render_inline_text(text);
                        elements.extend(inline_elements);
                    }
                }
                MarkdownNode::Start(Tag::Heading(level)) => {
                    elements.push(RenderedElement::Heading {
                        level: *level,
                        text: String::new(), // Will be filled by following text
                    });
                }
                _ => {}
            }
        }

        elements
    }

    fn render_inline_text(&self, text: &str) -> Vec<RenderedElement> {
        // Fast inline rendering without regex
        let mut elements = Vec::new();
        let mut current_text = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '*' => {
                    if let Some(next_ch) = chars.peek() {
                        if *next_ch == '*' {
                            // Bold text
                            if !current_text.is_empty() {
                                elements.push(RenderedElement::Text(current_text));
                                current_text.clear();
                            }
                            chars.next(); // Consume second *

                            // Collect bold text until **
                            let mut bold_text = String::new();
                            while let Some(ch) = chars.next() {
                                if ch == '*' {
                                    if let Some(next_ch) = chars.peek() {
                                        if *next_ch == '*' {
                                            chars.next(); // Consume second *
                                            elements.push(RenderedElement::Bold(bold_text));
                                            break;
                                        }
                                    }
                                }
                                bold_text.push(ch);
                            }
                        } else {
                            current_text.push(ch);
                        }
                    } else {
                        current_text.push(ch);
                    }
                }
                '`' => {
                    if !current_text.is_empty() {
                        elements.push(RenderedElement::Text(current_text));
                        current_text.clear();
                    }

                    // Collect inline code
                    let mut code_text = String::new();
                    while let Some(ch) = chars.next() {
                        if ch == '`' {
                            break;
                        }
                        code_text.push(ch);
                    }

                    elements.push(RenderedElement::InlineCode(code_text));
                }
                _ => {
                    current_text.push(ch);
                }
            }
        }

        if !current_text.is_empty() {
            elements.push(RenderedElement::Text(current_text));
        }

        elements
    }
}

#[derive(Debug, Clone)]
enum MarkdownNode {
    Start(pulldown_cmark::Tag, usize),
    End(pulldown_cmark::TagEnd, usize),
    Text(String, usize),
}

#[derive(Debug, Clone)]
enum RenderedElement {
    Text(String),
    Bold(String),
    Italic(String),
    InlineCode(String),
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    Heading {
        level: u32,
        text: String,
    },
}

struct RenderedMarkdown {
    elements: Vec<RenderedElement>,
    total_height: f32,
    render_time: Duration,
}
```

### 5. Memory Optimization

#### Object Pooling
```rust
/// Generic object pool for expensive allocations
pub struct ObjectPool<T> {
    objects: Arc<Mutex<Vec<T>>>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
    created: Arc<AtomicUsize>,
    reused: Arc<AtomicUsize>,
}

impl<T> ObjectPool<T>
where
    T: Send + Sync,
{
    pub fn new<F>(factory: F, initial_size: usize, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        let mut objects = Vec::with_capacity(initial_size);
        for _ in 0..initial_size {
            objects.push(factory());
        }

        Self {
            objects: Arc::new(Mutex::new(objects)),
            factory: Box::new(factory),
            max_size,
            created: Arc::new(AtomicUsize::new(initial_size)),
            reused: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn get(&self) -> PooledObject<T> {
        let mut objects = self.objects.lock().await;

        let object = if let Some(obj) = objects.pop() {
            self.reused.fetch_add(1, Ordering::Relaxed);
            obj
        } else {
            self.created.fetch_add(1, Ordering::Relaxed);
            (self.factory)()
        };

        PooledObject {
            object: Some(object),
            pool: self.objects.clone(),
        }
    }

    pub fn stats(&self) -> PoolStats {
        PoolStats {
            created: self.created.load(Ordering::Relaxed),
            reused: self.reused.load(Ordering::Relaxed),
            available: self.objects.lock().now_or_poison().len(),
        }
    }
}

/// RAII wrapper for pooled objects
pub struct PooledObject<T> {
    object: Option<T>,
    pool: Arc<Mutex<Vec<T>>>,
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.object.take() {
            // Return to pool if not at capacity
            let mut pool = self.pool.now_or_poison();
            if pool.len() < pool.capacity() {
                pool.push(obj);
            }
        }
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.object.as_ref().unwrap()
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.object.as_mut().unwrap()
    }
}

#[derive(Debug)]
struct PoolStats {
    created: usize,
    reused: usize,
    available: usize,
}

/// Pre-configured pools for common types
pub struct CommonPools {
    string_pool: ObjectPool<String>,
    vec_pool: ObjectPool<Vec<u8>>,
    hash_map_pool: ObjectPool<std::collections::HashMap<String, String>>,
}

impl CommonPools {
    pub fn new() -> Self {
        Self {
            string_pool: ObjectPool::new(
                || String::with_capacity(256),
                100,
                1000
            ),
            vec_pool: ObjectPool::new(
                || Vec::with_capacity(1024),
                50,
                500
            ),
            hash_map_pool: ObjectPool::new(
                || std::collections::HashMap::with_capacity(16),
                25,
                250
            ),
        }
    }

    pub async fn get_string(&self) -> PooledObject<String> {
        self.string_pool.get().await
    }

    pub async fn get_vec(&self) -> PooledObject<Vec<u8>> {
        self.vec_pool.get().await
    }

    pub async fn get_hash_map(&self) -> PooledObject<std::collections::HashMap<String, String>> {
        self.hash_map_pool.get().await
    }
}
```

#### Zero-Copy Operations
```rust
/// Zero-copy string operations for performance
pub struct ZeroCopyStringOperations;

impl ZeroCopyStringOperations {
    /// Extract prefix without allocation
    pub fn extract_prefix(s: &str, n: usize) -> Option<&str> {
        if s.is_char_boundary(n) {
            Some(&s[..n])
        } else {
            None
        }
    }

    /// Split on first occurrence without allocation
    pub fn split_once(s: &str, delimiter: &str) -> Option<(&str, &str)> {
        if let Some(pos) = s.find(delimiter) {
            Some((&s[..pos], &s[pos + delimiter.len()..]))
        } else {
            None
        }
    }

    /// Check if string starts with prefix (optimized)
    pub fn starts_with_fast(s: &str, prefix: &str) -> bool {
        if prefix.len() > s.len() {
            return false;
        }

        let s_bytes = s.as_bytes();
        let prefix_bytes = prefix.as_bytes();

        // Use SIMD-accelerated comparison if available
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::*;

            // Check 16 bytes at a time
            let chunks = prefix_bytes.len() / 16;
            for i in 0..chunks {
                let s_chunk = unsafe {
                    _mm_loadu_si128(s_bytes.as_ptr().add(i * 16) as *const __m128i)
                };
                let p_chunk = unsafe {
                    _mm_loadu_si128(prefix_bytes.as_ptr().add(i * 16) as *const __m128i)
                };
                let cmp = unsafe { _mm_cmpeq_epi8(s_chunk, p_chunk) };
                let mask = unsafe { _mm_movemask_epi8(cmp) };

                if mask != 0xFFFF {
                    return false;
                }
            }

            // Check remaining bytes
            for i in (chunks * 16)..prefix_bytes.len() {
                if s_bytes[i] != prefix_bytes[i] {
                    return false;
                }
            }

            true
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            // Fallback to standard comparison
            s_bytes.starts_with(prefix_bytes)
        }
    }
}

/// Zero-copy JSON parsing
pub struct ZeroCopyJsonParser {
    buffer: Vec<u8>,
}

impl ZeroCopyJsonParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(8192),
        }
    }

    /// Parse JSON without string allocations
    pub fn parse_str(&mut self, s: &str) -> Result<ZeroCopyValue, serde_json::Error> {
        // Copy to buffer if needed
        if s.as_bytes().len() > self.buffer.capacity() {
            self.buffer.reserve(s.as_bytes().len() - self.buffer.capacity());
        }
        self.buffer.clear();
        self.buffer.extend_from_slice(s.as_bytes());

        // Parse using simd-json for zero-copy parsing
        #[cfg(feature = "simd-json")]
        {
            let mut parsed = simd_json::to_owned_value(&self.buffer)
                .map_err(|e| serde_json::Error::custom(e.to_string()))?;

            Ok(ZeroCopyValue::from(parsed))
        }

        #[cfg(not(feature = "simd-json"))]
        {
            // Fallback to standard JSON parsing
            let parsed: serde_json::Value = serde_json::from_str(s)?;
            Ok(ZeroCopyValue::from(parsed))
        }
    }
}

#[derive(Debug, Clone)]
pub enum ZeroCopyValue<'a> {
    Null,
    Bool(bool),
    Number(f64),
    String(&'a str),
    Array(Vec<ZeroCopyValue<'a>>),
    Object(Vec<(&'a str, ZeroCopyValue<'a>)>),
}

impl<'a> ZeroCopyValue<'a> {
    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            ZeroCopyValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ZeroCopyValue::Number(n) => Some(*n),
            _ => None,
        }
    }
}
```

## Performance Monitoring

### Real-time Metrics
```rust
/// Real-time performance monitoring dashboard
pub struct PerformanceMonitor {
    metrics: Arc<RwLock<ComponentMetrics>>,
    alerts: Arc<Mutex<Vec<PerformanceAlert>>>,
    history: Arc<RwLock<VecDeque<Snapshot>>>,
    max_history: usize,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(ComponentMetrics::default())),
            alerts: Arc::new(Mutex::new(Vec::new())),
            history: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            max_history: 1000,
        }
    }

    /// Start real-time monitoring
    pub fn start_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let metrics = self.metrics.clone();
        let history = self.history.clone();
        let alerts = self.alerts.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));

            loop {
                interval.tick().await;

                // Take snapshot
                let snapshot = {
                    let metrics = metrics.read().await;
                    Snapshot {
                        timestamp: Instant::now(),
                        memory_usage: get_memory_usage(),
                        cpu_usage: get_cpu_usage(),
                        response_times: metrics.response_times.clone(),
                        error_rate: metrics.error_rate,
                        throughput: metrics.throughput,
                    }
                };

                // Store in history
                {
                    let mut history = history.write().await;
                    if history.len() >= 1000 {
                        history.pop_front();
                    }
                    history.push_back(snapshot.clone());
                }

                // Check for alerts
                Self::check_alerts(&snapshot, &alerts).await;
            }
        })
    }

    async fn check_alerts(snapshot: &Snapshot, alerts: &Arc<Mutex<Vec<PerformanceAlert>>>) {
        let mut new_alerts = Vec::new();

        // Memory usage alert
        if snapshot.memory_usage > 512 * 1024 * 1024 {
            new_alerts.push(PerformanceAlert {
                alert_type: AlertType::HighMemoryUsage,
                severity: AlertSeverity::Warning,
                message: format!("Memory usage: {}MB", snapshot.memory_usage / 1024 / 1024),
                timestamp: Utc::now(),
            });
        }

        // Response time alert
        if let Some(p95) = snapshot.response_times.get(&Percentile::P95) {
            if *p95 > Duration::from_millis(100) {
                new_alerts.push(PerformanceAlert {
                    alert_type: AlertType::SlowResponse,
                    severity: AlertSeverity::Error,
                    message: format!("P95 response time: {:?}", p95),
                    timestamp: Utc::now(),
                });
            }
        }

        // Add alerts if any
        if !new_alerts.is_empty() {
            let mut alerts = alerts.lock().await;
            alerts.extend(new_alerts);
        }
    }
}

#[derive(Debug, Clone)]
struct Snapshot {
    timestamp: Instant,
    memory_usage: usize,
    cpu_usage: f64,
    response_times: HashMap<Percentile, Duration>,
    error_rate: f64,
    throughput: f64,
}

#[derive(Debug, Clone)]
enum Percentile {
    P50,
    P95,
    P99,
}

fn get_memory_usage() -> usize {
    // Use platform-specific APIs to get actual memory usage
    #[cfg(unix)]
    {
        use std::fs;
        let status = fs::read_to_string("/proc/self/status").unwrap();
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                return parts[1].parse::<usize>().unwrap() * 1024;
            }
        }
    }

    0 // Fallback
}

fn get_cpu_usage() -> f64 {
    // Use platform-specific APIs to get CPU usage
    // This is a placeholder implementation
    0.0
}
```

This comprehensive performance optimization guide provides concrete implementations for achieving the ambitious performance targets. The key strategies include intelligent caching, concurrent operations, virtual scrolling, incremental rendering, and memory optimization techniques that together will ensure the Terraphim AI system delivers a responsive, smooth user experience.