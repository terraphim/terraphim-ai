use crate::indexer::IndexMiddleware;
use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use terraphim_config::Haystack;
use terraphim_persistence::Persistable;
use terraphim_types::{Document, Index};

/// Statistics for tracking URL fetch success/failure rates
#[derive(Default)]
struct FetchStats {
    successful: usize,
    failed: usize,
    skipped: usize,
}

impl FetchStats {
    fn new() -> Self {
        Self::default()
    }
}

/// Statistics for tracking persistence cache hits/misses
#[derive(Default)]
struct PersistenceStats {
    cache_hits: usize,
    cache_misses: usize,
    cache_saves: usize,
    document_cache_hits: usize,
    document_cache_misses: usize,
}

impl PersistenceStats {
    fn new() -> Self {
        Self::default()
    }
}

/// Middleware that uses query.rs as a haystack.
/// Supports comprehensive Rust documentation search including:
/// - Standard library docs (stable/nightly) via /suggest API
/// - Reddit posts via /posts/search JSON API
/// - Crates.io packages via /suggest API (when available)
/// - Docs.rs documentation via /suggest API (when available)
/// - Attributes, lints, books, caniuse, error codes via /suggest API
/// - Content scraping from found pages for full document content
#[derive(Debug, Clone)]
pub struct QueryRsHaystackIndexer {
    client: Client,
    /// Track fetched URLs to prevent duplicate fetches within an indexing session
    fetched_urls: Arc<Mutex<HashSet<String>>>,
}

impl Default for QueryRsHaystackIndexer {
    fn default() -> Self {
        // Create optimized client for API calls with reasonable timeout
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Terraphim/1.0 (https://terraphim.ai)")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            fetched_urls: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

impl QueryRsHaystackIndexer {
    /// Check if URL should be fetched. Returns true if URL hasn't been fetched yet.
    /// If URL is new, adds it to the cache and returns true.
    /// Thread-safe for concurrent access.
    pub(crate) fn should_fetch_url(&self, url: &str) -> bool {
        let mut cache = self.fetched_urls.lock().unwrap();
        if cache.contains(url) {
            log::debug!("⏭️  Skipping already fetched URL: {}", url);
            false
        } else {
            cache.insert(url.to_string());
            log::trace!("📝 Registered new URL for fetching: {}", url);
            true
        }
    }

    /// Clear the URL cache. Call this at the start of a new indexing session.
    pub(crate) fn clear_url_cache(&self) {
        let mut cache = self.fetched_urls.lock().unwrap();
        let count = cache.len();
        cache.clear();
        if count > 0 {
            log::debug!("🗑️  Cleared {} URLs from fetch cache", count);
        }
    }

    /// Get the number of unique URLs that have been fetched
    pub(crate) fn get_fetched_count(&self) -> usize {
        self.fetched_urls.lock().unwrap().len()
    }
}

#[async_trait]
impl IndexMiddleware for QueryRsHaystackIndexer {
    #[allow(clippy::manual_async_fn)]
    fn index(
        &self,
        needle: &str,
        haystack: &Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send {
        async move {
            let fetch_content = haystack.fetch_content;
            log::warn!(
                "QueryRs: Starting index for search term: '{}' (fetch_content: {})",
                needle,
                fetch_content
            );

            // Clear URL cache at start of new indexing session
            self.clear_url_cache();

            let mut documents = Vec::new();
            let mut persistence_stats = PersistenceStats::new();

            // First, try to load cached search results from persistence
            let cache_key = format!("queryrs_search_{}", self.normalize_search_query(needle));
            let mut cache_placeholder = Document {
                id: cache_key.clone(),
                ..Default::default()
            };

            let use_cached_results = match cache_placeholder.load().await {
                Ok(cached_doc) => {
                    // Check if cache is fresh (less than 1 hour old)
                    if self.is_cache_fresh(&cached_doc) {
                        log::info!(
                            "Using cached QueryRs search results for query: '{}'",
                            needle
                        );
                        match serde_json::from_str::<Vec<Document>>(&cached_doc.body) {
                            Ok(cached_documents) => {
                                documents = cached_documents;
                                persistence_stats.cache_hits += 1;
                                true
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to deserialize cached results for '{}': {}",
                                    needle,
                                    e
                                );
                                persistence_stats.cache_misses += 1;
                                false
                            }
                        }
                    } else {
                        log::debug!(
                            "Cached results for '{}' are stale, fetching fresh results",
                            needle
                        );
                        persistence_stats.cache_misses += 1;
                        false
                    }
                }
                Err(_) => {
                    log::debug!("No cached results found for query: '{}'", needle);
                    persistence_stats.cache_misses += 1;
                    false
                }
            };

            if !use_cached_results {
                log::warn!(
                    "QueryRs: No cached results found, executing fresh search for '{}'",
                    needle
                );
                // Search across all query.rs endpoints concurrently
                let (reddit_results, suggest_results, crates_results, docs_results) = tokio::join!(
                    self.search_reddit_posts(needle),
                    self.search_suggest_api(needle),
                    self.search_crates_io(needle),
                    self.search_docs_rs(needle),
                );

                // Collect results from all searches
                if let Ok(docs) = reddit_results {
                    documents.extend(docs);
                }
                if let Ok(docs) = suggest_results {
                    documents.extend(docs);
                }
                if let Ok(docs) = crates_results {
                    documents.extend(docs);
                }
                if let Ok(docs) = docs_results {
                    documents.extend(docs);
                }

                // Cache the search results for future queries
                if !documents.is_empty() {
                    match serde_json::to_string(&documents) {
                        Ok(serialized_docs) => {
                            let cache_doc = Document {
                                id: cache_key,
                                title: format!("QueryRs search results for '{}'", needle),
                                body: serialized_docs,
                                url: format!("cache://queryrs/{}", needle),
                                description: Some(format!(
                                    "Cached search results from query.rs API for query: {}",
                                    needle
                                )),
                                summarization: None,
                                stub: None,
                                tags: Some(vec!["queryrs".to_string(), "cache".to_string()]),
                                rank: None,
                                source_haystack: None,
                            };
                            if let Err(e) = cache_doc.save().await {
                                log::warn!(
                                    "Failed to cache search results for '{}': {}",
                                    needle,
                                    e
                                );
                            } else {
                                log::debug!(
                                    "Cached {} search results for query: '{}'",
                                    documents.len(),
                                    needle
                                );
                                persistence_stats.cache_saves += 1;
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to serialize search results for caching '{}': {}",
                                needle,
                                e
                            );
                        }
                    }
                }
            }

            // Process documents: check persistence first, then fetch content if needed
            let mut enhanced_documents = Vec::new();
            let mut fetch_stats = FetchStats::new();

            log::warn!(
                "QueryRs: Processing {} documents from search results",
                documents.len()
            );

            for doc in documents {
                log::warn!(
                    "QueryRs: Processing document '{}' - title: '{}'",
                    doc.id,
                    doc.title
                );

                // First, check if we have a cached version of this document with enhanced content
                let mut doc_placeholder = Document {
                    id: doc.id.clone(),
                    ..Default::default()
                };

                let enhanced_doc = match doc_placeholder.load().await {
                    Ok(cached_doc) => {
                        log::debug!("Found cached document '{}' in persistence", doc.title);
                        persistence_stats.document_cache_hits += 1;
                        // Use cached version if it has more content than the API result
                        if cached_doc.body.len() > doc.body.len() + 100 {
                            log::debug!(
                                "Using cached content for '{}' (cached: {} chars vs API: {} chars)",
                                doc.title,
                                cached_doc.body.len(),
                                doc.body.len()
                            );
                            // Clear any existing summaries to ensure fresh AI summarization
                            let mut fresh_doc = cached_doc;
                            fresh_doc.summarization = None;
                            fresh_doc.description = None;
                            log::debug!("Cleared existing summaries from cached document '{}' for fresh AI summarization", fresh_doc.id);
                            fresh_doc
                        } else {
                            doc
                        }
                    }
                    Err(_) => {
                        persistence_stats.document_cache_misses += 1;
                        doc
                    }
                };

                // Check if content enhancement is disabled via configuration
                let disable_content_enhancement = haystack
                    .extra_parameters
                    .get("disable_content_enhancement")
                    .map(|v| v == "true")
                    .unwrap_or(true); // Default to disabled for performance

                log::warn!(
                    "QueryRs: disable_content_enhancement = {} for document '{}'",
                    disable_content_enhancement,
                    enhanced_doc.id
                );

                if disable_content_enhancement {
                    // Skip aggressive content fetching to improve performance
                    // The QueryRs API already provides good summaries and metadata
                    fetch_stats.skipped += 1;

                    log::warn!(
                        "QueryRs: Processing document '{}' for persistence saving",
                        enhanced_doc.id
                    );

                    // Still save documents to persistence for summarization to work
                    log::warn!(
                        "QueryRs: Attempting to save document '{}' to persistence",
                        enhanced_doc.id
                    );
                    match enhanced_doc.save().await {
                        Ok(_) => {
                            log::warn!(
                            "QueryRs: Successfully saved document '{}' to persistence for summarization",
                            enhanced_doc.id
                        );
                        }
                        Err(e) => {
                            log::error!(
                                "QueryRs: Failed to save document '{}' to persistence: {}",
                                enhanced_doc.id,
                                e
                            );
                            // Continue processing even if save fails
                        }
                    }

                    enhanced_documents.push(enhanced_doc);
                } else {
                    // Legacy content fetching (disabled by default)
                    // Only fetch content if enabled and URL hasn't been fetched yet
                    if fetch_content
                        && !enhanced_doc.url.is_empty()
                        && enhanced_doc.url.starts_with("http")
                        && !enhanced_doc.url.contains("crates.io/crates/")
                        && enhanced_doc.body.len() < 200
                        && self.should_fetch_url(&enhanced_doc.url)
                    {
                        match self.fetch_and_scrape_content(&enhanced_doc).await {
                            Ok(fetched_doc) => {
                                fetch_stats.successful += 1;
                                // Save the enhanced document to persistence for future use
                                if let Err(e) = fetched_doc.save().await {
                                    log::warn!(
                                        "Failed to save enhanced document '{}': {}",
                                        fetched_doc.title,
                                        e
                                    );
                                } else {
                                    log::debug!(
                                        "Saved enhanced document '{}' to persistence",
                                        fetched_doc.title
                                    );
                                }
                                enhanced_documents.push(fetched_doc);
                            }
                            Err(e) => {
                                fetch_stats.failed += 1;
                                if self.is_critical_url(&enhanced_doc.url) {
                                    log::warn!(
                                        "Failed to fetch critical URL {}: {}",
                                        enhanced_doc.url,
                                        e
                                    );
                                } else {
                                    log::debug!(
                                        "Failed to fetch non-critical URL {}: {}",
                                        enhanced_doc.url,
                                        e
                                    );
                                }
                                enhanced_documents.push(enhanced_doc);
                            }
                        }
                    } else {
                        fetch_stats.skipped += 1;
                        if !fetch_content {
                            log::trace!(
                                "Skipping content fetch (fetch_content=false): {}",
                                enhanced_doc.url
                            );
                        }
                        enhanced_documents.push(enhanced_doc);
                    }
                }
            }

            // Log comprehensive summary statistics
            let unique_urls_fetched = self.get_fetched_count();
            log::info!(
                "QueryRs processing complete: {} documents, {} unique URLs fetched (fetch: {} successful, {} failed, {} skipped) | (cache: {} search hits, {} search misses, {} doc hits, {} doc misses)",
                enhanced_documents.len(),
                unique_urls_fetched,
                fetch_stats.successful, fetch_stats.failed, fetch_stats.skipped,
                persistence_stats.cache_hits, persistence_stats.cache_misses,
                persistence_stats.document_cache_hits, persistence_stats.document_cache_misses
            );

            // Convert to Index format
            let mut index = Index::new();
            for doc in enhanced_documents {
                index.insert(doc.id.clone(), doc);
            }

            Ok(index)
        }
    }
}

impl QueryRsHaystackIndexer {
    /// Normalize document ID to match persistence layer expectations
    pub fn normalize_document_id(&self, original_id: &str) -> String {
        // Create a dummy document to access the normalize_key method
        let dummy_doc = Document {
            id: "dummy".to_string(),
            title: "dummy".to_string(),
            body: "dummy".to_string(),
            url: "dummy".to_string(),
            description: None,
            summarization: None,
            stub: None,
            tags: None,
            rank: None,
            source_haystack: None,
        };
        let normalized = dummy_doc.normalize_key(original_id);

        // Validate the normalized ID to ensure it follows expected patterns
        self.validate_document_id(&normalized, original_id)
    }

    /// Validate and potentially fix document IDs to prevent malformed entries
    fn validate_document_id(&self, normalized_id: &str, original_id: &str) -> String {
        // Check for common malformed patterns that cause OpenDAL warnings
        let is_malformed = normalized_id.contains("crategravitydb")
            || normalized_id.contains("crategqlite000")
            || normalized_id.ends_with("md")
            || normalized_id.len() > 50  // Reduced threshold for testing
            || original_id.ends_with(".md")  // Check original too
            || original_id.is_empty();

        if is_malformed {
            log::warn!("Detected potentially malformed document ID: '{}' (from '{}'). Applying additional normalization.",
                      normalized_id, original_id);

            // Apply additional cleaning for legacy compatibility
            return self.apply_legacy_id_cleanup(original_id);
        }

        normalized_id.to_string()
    }

    /// Apply cleanup for legacy malformed IDs
    fn apply_legacy_id_cleanup(&self, original_id: &str) -> String {
        let mut cleaned = original_id.to_lowercase();

        // Remove common problematic patterns
        cleaned = cleaned.replace(".md", "");
        if cleaned.ends_with("md") {
            cleaned = cleaned.strip_suffix("md").unwrap_or(&cleaned).to_string();
        }
        cleaned = cleaned.replace("-", "_");
        cleaned = cleaned.replace(".", "_");
        cleaned = cleaned.replace("/", "_");

        // Replace multiple underscores with single ones
        while cleaned.contains("__") {
            cleaned = cleaned.replace("__", "_");
        }

        // Trim underscores from start/end
        cleaned = cleaned.trim_matches('_').to_string();

        // Limit length to prevent very long IDs
        if cleaned.len() > 50 {
            cleaned = cleaned.chars().take(50).collect::<String>();
            cleaned = cleaned.trim_matches('_').to_string();
        }

        // Ensure we have a valid ID
        if cleaned.is_empty() {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            original_id.hash(&mut hasher);
            cleaned = format!("legacy_{:x}", hasher.finish());
        }

        log::info!("Legacy ID cleanup: '{}' → '{}'", original_id, cleaned);
        cleaned
    }

    /// Extract Reddit post ID from URL
    /// Example: https://www.reddit.com/r/rust/comments/abc123/title/ -> abc123
    pub fn extract_reddit_post_id(&self, url: &str) -> Option<String> {
        // Look for the pattern /comments/{post_id}/
        if let Some(comments_pos) = url.find("/comments/") {
            let after_comments = &url[comments_pos + 10..]; // "/comments/" is 10 chars
            if let Some(slash_pos) = after_comments.find('/') {
                let post_id = &after_comments[..slash_pos];
                if !post_id.is_empty() && post_id.chars().all(|c| c.is_alphanumeric()) {
                    return Some(post_id.to_string());
                }
            }
        }
        None
    }

    /// Extract a clean identifier from a documentation URL
    /// Example: https://doc.rust-lang.org/std/iter/trait.Iterator.html -> std_iter_Iterator
    pub fn extract_doc_identifier(&self, url: &str) -> String {
        if let Some(path) = url.strip_prefix("https://doc.rust-lang.org/") {
            // Remove .html extension and replace slashes and dots with underscores
            let clean_path = path.trim_end_matches(".html").replace(['/', '.'], "_");
            return clean_path;
        }

        // For other URLs, extract domain and path
        if let Ok(parsed_url) = url::Url::parse(url) {
            let host = parsed_url.host_str().unwrap_or("");
            let path = parsed_url
                .path()
                .trim_start_matches('/')
                .trim_end_matches('/');
            let clean_host = host.replace('.', "_");
            // Clean the path more thoroughly
            let clean_path = path
                .replace(['/', '.', '@', '#', '?', '&', '=', '-'], "_")
                .trim_end_matches("_html")
                .to_string();
            if clean_path.is_empty() {
                return clean_host;
            }
            return format!("{}_{}", clean_host, clean_path);
        }

        // Fallback: use a hash of the URL
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        format!("url_{:x}", hasher.finish())
    }

    /// Fetch and scrape content from a document's URL with retry logic for critical URLs
    async fn fetch_and_scrape_content(&self, doc: &Document) -> Result<Document> {
        let mut enhanced_doc = doc.clone();

        // For Reddit posts, try API first before scraping
        if doc.url.contains("reddit.com") {
            if let Some(api_content) = self.fetch_reddit_api_content(&doc.url).await {
                enhanced_doc.body = api_content;
                log::info!("✅ Fetched Reddit content via API: {}", doc.url);
                return Ok(enhanced_doc);
            }
            log::debug!("Reddit API fetch failed, falling back to scraping");
        }

        let is_critical = self.is_critical_url(&doc.url);
        let max_retries = if is_critical { 2 } else { 0 };

        log::info!("Fetching content from: {}", doc.url);

        for attempt in 0..=max_retries {
            if attempt > 0 {
                let delay = std::time::Duration::from_millis(500 * attempt as u64);
                tokio::time::sleep(delay).await;
                log::debug!(
                    "Retrying fetch for {} (attempt {}/{})",
                    doc.url,
                    attempt + 1,
                    max_retries + 1
                );
            }

            match self
                .client
                .get(&doc.url)
                .header("User-Agent", "Terraphim/1.0")
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text().await {
                            Ok(html_content) => {
                                let scraped_content = self.scrape_content(&html_content, &doc.url);
                                enhanced_doc.body = scraped_content;
                                log::info!("Successfully scraped content from: {}", doc.url);
                                return Ok(enhanced_doc);
                            }
                            Err(e) => {
                                if attempt == max_retries {
                                    log::warn!(
                                        "Failed to get text content from {}: {}",
                                        doc.url,
                                        e
                                    );
                                }
                                continue;
                            }
                        }
                    } else {
                        if attempt == max_retries {
                            log::warn!(
                                "Failed to fetch {} with status: {}",
                                doc.url,
                                response.status()
                            );
                        }
                        continue;
                    }
                }
                Err(e) => {
                    if attempt == max_retries {
                        return Err(crate::Error::Http(e));
                    }
                    continue;
                }
            }
        }

        Ok(enhanced_doc)
    }

    /// Scrape relevant content from HTML based on the URL type
    fn scrape_content(&self, html_content: &str, url: &str) -> String {
        let document = Html::parse_document(html_content);

        if url.contains("crates.io") {
            self.scrape_crates_io_content(&document)
        } else if url.contains("docs.rs") {
            self.scrape_docs_rs_content(&document)
        } else if url.contains("doc.rust-lang.org") {
            self.scrape_rust_doc_content(&document)
        } else if url.contains("reddit.com") {
            self.scrape_reddit_content(&document)
        } else {
            self.scrape_generic_content(&document)
        }
    }

    /// Scrape content from crates.io pages
    fn scrape_crates_io_content(&self, document: &Html) -> String {
        let mut content = String::new();

        // Try to get the crate description
        if let Ok(desc_selector) = Selector::parse(".crate-description") {
            if let Some(desc_elem) = document.select(&desc_selector).next() {
                content.push_str(&format!(
                    "Description: {}\n\n",
                    desc_elem.text().collect::<Vec<_>>().join(" ")
                ));
            }
        }

        // Try to get README content
        if let Ok(readme_selector) = Selector::parse("#readme") {
            if let Some(readme_elem) = document.select(&readme_selector).next() {
                content.push_str(&format!(
                    "README: {}\n\n",
                    readme_elem.text().collect::<Vec<_>>().join(" ")
                ));
            }
        }

        // Try to get dependencies
        if let Ok(deps_selector) = Selector::parse(".dependencies") {
            if let Some(deps_elem) = document.select(&deps_selector).next() {
                content.push_str(&format!(
                    "Dependencies: {}\n\n",
                    deps_elem.text().collect::<Vec<_>>().join(" ")
                ));
            }
        }

        if content.is_empty() {
            // Fallback: get all text content
            content = document.root_element().text().collect::<Vec<_>>().join(" ");
        }

        content
    }

    /// Scrape content from docs.rs pages
    fn scrape_docs_rs_content(&self, document: &Html) -> String {
        let mut content = String::new();

        // Try to get the main documentation content
        if let Ok(main_selector) = Selector::parse("main") {
            if let Some(main_elem) = document.select(&main_selector).next() {
                content = main_elem.text().collect::<Vec<_>>().join(" ");
            }
        }

        // If no main content, try to get documentation sections
        if content.is_empty() {
            if let Ok(doc_selector) = Selector::parse(".docblock") {
                for elem in document.select(&doc_selector) {
                    content.push_str(&elem.text().collect::<Vec<_>>().join(" "));
                    content.push_str("\n\n");
                }
            }
        }

        if content.is_empty() {
            // Fallback: get all text content
            content = document.root_element().text().collect::<Vec<_>>().join(" ");
        }

        content
    }

    /// Scrape content from Rust documentation pages
    fn scrape_rust_doc_content(&self, document: &Html) -> String {
        let mut content = String::new();

        // Try to get the main documentation content
        if let Ok(main_selector) = Selector::parse("main") {
            if let Some(main_elem) = document.select(&main_selector).next() {
                content = main_elem.text().collect::<Vec<_>>().join(" ");
            }
        }

        // Try to get documentation sections
        if content.is_empty() {
            if let Ok(doc_selector) = Selector::parse(".docblock") {
                for elem in document.select(&doc_selector) {
                    content.push_str(&elem.text().collect::<Vec<_>>().join(" "));
                    content.push_str("\n\n");
                }
            }
        }

        if content.is_empty() {
            // Fallback: get all text content
            content = document.root_element().text().collect::<Vec<_>>().join(" ");
        }

        content
    }

    /// Scrape content from Reddit pages
    /// Fetch Reddit post content via JSON API
    /// This provides structured data instead of scraping HTML
    async fn fetch_reddit_api_content(&self, url: &str) -> Option<String> {
        // Extract post ID from Reddit URL
        // URLs like: https://www.reddit.com/r/rust/comments/abc123/title/
        let post_id = url.split("/comments/").nth(1)?.split('/').next()?;

        let json_url = format!("https://www.reddit.com/comments/{}.json", post_id);

        log::debug!("Fetching Reddit API for post: {}", post_id);

        match self
            .client
            .get(&json_url)
            .header("User-Agent", "Terraphim/1.0 (https://terraphim.ai)")
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                match response.json::<Value>().await {
                    Ok(json_data) => {
                        // Parse Reddit JSON structure: array with post and comments
                        if let Some(post_data) = json_data
                            .get(0)
                            .and_then(|v| v.get("data"))
                            .and_then(|v| v.get("children"))
                            .and_then(|v| v.get(0))
                            .and_then(|v| v.get("data"))
                        {
                            let title = post_data
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let selftext = post_data
                                .get("selftext")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let author = post_data
                                .get("author")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let score =
                                post_data.get("score").and_then(|v| v.as_i64()).unwrap_or(0);
                            let num_comments = post_data
                                .get("num_comments")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            let content = format!(
                                "Title: {}\n\nAuthor: u/{}\nScore: {} | Comments: {}\n\n{}",
                                title, author, score, num_comments, selftext
                            );

                            log::debug!("✅ Fetched Reddit API content ({} chars)", content.len());
                            return Some(content);
                        }
                    }
                    Err(e) => log::debug!("Failed to parse Reddit JSON: {}", e),
                }
            }
            Err(e) => log::debug!("Failed to fetch Reddit API: {}", e),
            _ => {}
        }

        None
    }

    fn scrape_reddit_content(&self, document: &Html) -> String {
        let mut content = String::new();

        // Try to get the post content
        if let Ok(post_selector) = Selector::parse("[data-testid='post-content']") {
            if let Some(post_elem) = document.select(&post_selector).next() {
                content = post_elem.text().collect::<Vec<_>>().join(" ");
            }
        }

        // Try alternative selectors for Reddit content
        if content.is_empty() {
            if let Ok(content_selector) = Selector::parse(".RichTextJSON-root") {
                if let Some(content_elem) = document.select(&content_selector).next() {
                    content = content_elem.text().collect::<Vec<_>>().join(" ");
                }
            }
        }

        if content.is_empty() {
            // Fallback: get all text content
            content = document.root_element().text().collect::<Vec<_>>().join(" ");
        }

        content
    }

    /// Scrape generic content from any HTML page
    fn scrape_generic_content(&self, document: &Html) -> String {
        let mut content = String::new();

        // Try to get main content
        if let Ok(main_selector) = Selector::parse("main") {
            if let Some(main_elem) = document.select(&main_selector).next() {
                content = main_elem.text().collect::<Vec<_>>().join(" ");
            }
        }

        // Try to get article content
        if content.is_empty() {
            if let Ok(article_selector) = Selector::parse("article") {
                if let Some(article_elem) = document.select(&article_selector).next() {
                    content = article_elem.text().collect::<Vec<_>>().join(" ");
                }
            }
        }

        // Try to get content from common content divs
        if content.is_empty() {
            for selector_str in [".content", ".main", ".post-content", ".entry-content"] {
                if let Ok(selector) = Selector::parse(selector_str) {
                    if let Some(elem) = document.select(&selector).next() {
                        content = elem.text().collect::<Vec<_>>().join(" ");
                        break;
                    }
                }
            }
        }

        if content.is_empty() {
            // Fallback: get all text content
            content = document.root_element().text().collect::<Vec<_>>().join(" ");
        }

        content
    }

    /// Search Reddit posts using the JSON API
    async fn search_reddit_posts(&self, query: &str) -> Result<Vec<Document>> {
        let url = format!("https://query.rs/posts/search?q={}", query);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Vec<Value>>().await {
                        Ok(posts) => self.parse_reddit_json(posts),
                        Err(e) => {
                            log::warn!("Failed to parse Reddit JSON response: {}", e);
                            Ok(Vec::new())
                        }
                    }
                } else {
                    log::warn!("Reddit search failed with status: {}", response.status());
                    Ok(Vec::new())
                }
            }
            Err(e) => {
                log::warn!("Failed to search Reddit: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Search using the suggest API for std docs, attributes, etc.
    async fn search_suggest_api(&self, query: &str) -> Result<Vec<Document>> {
        let url = format!("https://query.rs/suggest/{}", query);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Value>().await {
                        Ok(suggest_data) => self.parse_suggest_json(suggest_data, query),
                        Err(e) => {
                            log::warn!("Failed to parse suggest JSON response: {}", e);
                            Ok(Vec::new())
                        }
                    }
                } else {
                    log::warn!("Suggest search failed with status: {}", response.status());
                    Ok(Vec::new())
                }
            }
            Err(e) => {
                log::warn!("Failed to search suggest API: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Search crates.io for packages
    async fn search_crates_io(&self, query: &str) -> Result<Vec<Document>> {
        // Try to search crates.io directly via their API
        let url = format!("https://crates.io/api/v1/crates?q={}&per_page=10", query);

        log::info!("Searching crates.io for: {}", query);

        match self
            .client
            .get(&url)
            .header("User-Agent", "Terraphim/1.0")
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Value>().await {
                        Ok(crates_data) => {
                            let docs = self.parse_crates_io_json(crates_data, query);
                            log::info!(
                                "Found {} crates.io results for: {}",
                                docs.as_ref().map(|d| d.len()).unwrap_or(0),
                                query
                            );
                            docs
                        }
                        Err(e) => {
                            log::warn!("Failed to parse crates.io JSON response: {}", e);
                            Ok(Vec::new())
                        }
                    }
                } else {
                    log::warn!("Crates.io search failed with status: {}", response.status());
                    Ok(Vec::new())
                }
            }
            Err(e) => {
                log::warn!("Failed to search crates.io: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Search docs.rs for documentation
    async fn search_docs_rs(&self, query: &str) -> Result<Vec<Document>> {
        // Try to search docs.rs directly
        let url = format!("https://docs.rs/releases/search?query={}", query);

        log::info!("Searching docs.rs for: {}", query);

        match self
            .client
            .get(&url)
            .header("User-Agent", "Terraphim/1.0")
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    // Parse HTML response to extract crate information
                    match response.text().await {
                        Ok(html_content) => {
                            let docs = self.parse_docs_rs_html(&html_content, query);
                            log::info!(
                                "Found {} docs.rs results for: {}",
                                docs.as_ref().map(|d| d.len()).unwrap_or(0),
                                query
                            );
                            docs
                        }
                        Err(e) => {
                            log::warn!("Failed to get docs.rs HTML response: {}", e);
                            Ok(Vec::new())
                        }
                    }
                } else {
                    log::warn!("Docs.rs search failed with status: {}", response.status());
                    Ok(Vec::new())
                }
            }
            Err(e) => {
                log::warn!("Failed to search docs.rs: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Parse Reddit JSON results
    pub fn parse_reddit_json(&self, posts: Vec<Value>) -> Result<Vec<Document>> {
        let mut documents = Vec::new();

        for post in posts {
            if let Some(obj) = post.as_object() {
                let title = obj
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let url = obj.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let author = obj.get("author").and_then(|v| v.as_str()).unwrap_or("");
                let score = obj.get("score").and_then(|v| v.as_u64()).unwrap_or(0);
                let body = obj.get("selftext").and_then(|v| v.as_str()).unwrap_or("");

                if !title.is_empty() && !url.is_empty() {
                    // Extract clean post ID from Reddit URL, fallback to URL hash if extraction fails
                    let post_identifier = if let Some(post_id) = self.extract_reddit_post_id(url) {
                        post_id
                    } else {
                        // Fallback: use hash of URL for non-standard Reddit URLs
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
                        let mut hasher = DefaultHasher::new();
                        url.hash(&mut hasher);
                        format!("{:x}", hasher.finish())
                    };
                    let original_id = format!("reddit-{}", post_identifier);
                    let normalized_id = self.normalize_document_id(&original_id);
                    documents.push(Document {
                        id: normalized_id,
                        url: url.to_string(),
                        title: format!("[Reddit] {}", title),
                        description: Some(format!("by {} (score: {})", author, score)),
                        summarization: None,
                        body: body.to_string(),
                        stub: None,
                        tags: Some(vec![
                            "rust".to_string(),
                            "reddit".to_string(),
                            "community".to_string(),
                        ]),
                        rank: Some(score),
                        source_haystack: None,
                    });
                }
            }
        }

        Ok(documents)
    }

    /// Parse suggest API JSON results (OpenSearch Suggestions format)
    pub fn parse_suggest_json(&self, suggest_data: Value, query: &str) -> Result<Vec<Document>> {
        let mut documents = Vec::new();

        // OpenSearch Suggestions format: [query, [completions], [descriptions], [urls]]
        if let Some(suggestions) = suggest_data.as_array() {
            if suggestions.len() >= 2 {
                if let Some(completions) = suggestions[1].as_array() {
                    for completion in completions {
                        if let Some(completion_str) = completion.as_str() {
                            // Parse completion format: "std::iter::Iterator - https://doc.rust-lang.org/std/iter/trait.Iterator.html"
                            if let Some((title_part, url_part)) = completion_str.split_once(" - ") {
                                let title = title_part.trim();
                                let url = url_part.trim();

                                if !title.is_empty() && !url.is_empty() {
                                    // Determine search type based on URL or title
                                    let search_type = self.determine_search_type(title, url);
                                    let tags = self.generate_tags_for_search_type(search_type);

                                    // Use a clean identifier based on the title and search type instead of the full URL
                                    let doc_identifier = self.extract_doc_identifier(url);
                                    let original_id = format!("{}-{}", search_type, doc_identifier);
                                    let normalized_id = self.normalize_document_id(&original_id);
                                    documents.push(Document {
                                        id: normalized_id,
                                        url: url.to_string(),
                                        title: format!(
                                            "[{}] {}",
                                            search_type.to_uppercase(),
                                            title
                                        ),
                                        description: Some(format!(
                                            "Rust {} documentation",
                                            search_type
                                        )),
                                        summarization: None,
                                        body: format!("Search result for '{}': {}", query, title),
                                        stub: None,
                                        tags: Some(tags),
                                        rank: None,
                                        source_haystack: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(documents)
    }

    /// Parse crates.io JSON results
    pub fn parse_crates_io_json(&self, crates_data: Value, _query: &str) -> Result<Vec<Document>> {
        let mut documents = Vec::new();

        if let Some(crates) = crates_data.get("crates").and_then(|v| v.as_array()) {
            for crate_info in crates {
                if let Some(obj) = crate_info.as_object() {
                    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let description = obj
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let version = obj
                        .get("max_version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let downloads = obj.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0);
                    let homepage = obj.get("homepage").and_then(|v| v.as_str()).unwrap_or("");
                    let documentation = obj
                        .get("documentation")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let repository = obj.get("repository").and_then(|v| v.as_str()).unwrap_or("");
                    let keywords = obj
                        .get("keywords")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|k| k.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    let url = format!("https://crates.io/crates/{}", name);

                    if !name.is_empty() {
                        // Build a comprehensive body with all available API data
                        let mut body = format!("Crate: {} v{}\n\n", name, version);
                        body.push_str(&format!("Description: {}\n\n", description));
                        body.push_str(&format!("Downloads: {}\n\n", downloads));

                        if !keywords.is_empty() {
                            body.push_str(&format!("Keywords: {}\n\n", keywords));
                        }

                        if !homepage.is_empty() {
                            body.push_str(&format!("Homepage: {}\n\n", homepage));
                        }

                        if !documentation.is_empty() {
                            body.push_str(&format!("Documentation: {}\n\n", documentation));
                        }

                        if !repository.is_empty() {
                            body.push_str(&format!("Repository: {}\n\n", repository));
                        }

                        let original_id = format!("crate-{}", name);
                        let normalized_id = self.normalize_document_id(&original_id);
                        documents.push(Document {
                            id: normalized_id,
                            url: url.clone(),
                            title: format!("[CRATE] {} {}", name, version),
                            description: Some(format!("{} ({} downloads)", description, downloads)),
                            summarization: None,
                            body,
                            stub: None,
                            tags: Some(vec![
                                "rust".to_string(),
                                "crate".to_string(),
                                "package".to_string(),
                            ]),
                            rank: Some(downloads),
                            source_haystack: None,
                        });
                    }
                }
            }
        }

        Ok(documents)
    }

    /// Parse docs.rs HTML results
    fn parse_docs_rs_html(&self, html_content: &str, query: &str) -> Result<Vec<Document>> {
        let mut documents = Vec::new();

        // Simple HTML parsing to extract crate information
        // This is a basic implementation - in production you might want to use a proper HTML parser
        let lines: Vec<&str> = html_content.lines().collect();

        for line in lines {
            if line.contains("crate-name") && line.contains(query) {
                // Extract crate name from the HTML
                if let Some(start) = line.find("crate-name") {
                    if let Some(end) = line[start..].find('"') {
                        let crate_name = &line[start + 11..start + end];
                        if !crate_name.is_empty() {
                            let url = format!("https://docs.rs/{}", crate_name);
                            let original_id = format!("docs-{}", crate_name);
                            let normalized_id = self.normalize_document_id(&original_id);
                            documents.push(Document {
                                id: normalized_id,
                                url: url.clone(),
                                title: format!("[DOCS] {}", crate_name),
                                description: Some(format!("Documentation for {}", crate_name)),
                                summarization: None,
                                body: format!("Documentation for crate: {}", crate_name),
                                stub: None,
                                tags: Some(vec![
                                    "rust".to_string(),
                                    "docs".to_string(),
                                    "documentation".to_string(),
                                ]),
                                rank: None,
                                source_haystack: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(documents)
    }

    /// Determine search type based on title and URL
    fn determine_search_type(&self, title: &str, url: &str) -> &'static str {
        if url.contains("doc.rust-lang.org") {
            if title.contains("::") {
                // std library items
                if title.contains("attr.") {
                    "attribute"
                } else if title.contains("trait.") {
                    "trait"
                } else if title.contains("struct.") {
                    "struct"
                } else if title.contains("enum.") {
                    "enum"
                } else if title.contains("fn.") {
                    "function"
                } else if title.contains("mod.") {
                    "module"
                } else {
                    "std"
                }
            } else {
                "std"
            }
        } else if url.contains("crates.io") {
            "crate"
        } else if url.contains("docs.rs") {
            "docs"
        } else if url.contains("rust-lang.github.io") {
            "book"
        } else if url.contains("rust-lang.github.io/rust-clippy") {
            "lint"
        } else {
            "docs"
        }
    }

    /// Generate appropriate tags for search type
    fn generate_tags_for_search_type(&self, search_type: &str) -> Vec<String> {
        let mut tags = vec!["rust".to_string()];

        match search_type {
            "std" | "trait" | "struct" | "enum" | "function" | "module" => {
                tags.extend(vec!["std".to_string(), "documentation".to_string()]);
            }
            "attribute" => {
                tags.extend(vec!["attribute".to_string(), "macro".to_string()]);
            }
            "crate" => {
                tags.extend(vec!["crate".to_string(), "package".to_string()]);
            }
            "docs" => {
                tags.extend(vec!["docs".to_string(), "documentation".to_string()]);
            }
            "book" => {
                tags.extend(vec!["book".to_string(), "guide".to_string()]);
            }
            "lint" => {
                tags.extend(vec!["lint".to_string(), "clippy".to_string()]);
            }
            _ => {
                tags.push("documentation".to_string());
            }
        }

        tags
    }

    /// Determine if a URL is critical (Rust documentation) and should have higher priority/warnings
    fn is_critical_url(&self, url: &str) -> bool {
        url.contains("doc.rust-lang.org")
            || url.contains("docs.rs")
            || url.contains("rust-lang.github.io")
    }

    /// Normalize search query for use as cache key
    fn normalize_search_query(&self, query: &str) -> String {
        query
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .chars()
            .take(50) // Limit cache key length
            .collect()
    }

    /// Check if cached document is fresh (less than 1 hour old)
    fn is_cache_fresh(&self, cached_doc: &Document) -> bool {
        // For now, we'll consider cache fresh if the document exists
        // In the future, we could add timestamp metadata to documents
        // to implement proper cache expiration
        !cached_doc.body.is_empty()
    }
}
