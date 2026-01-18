use crate::{indexer::IndexMiddleware, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use terraphim_config::Haystack;
use terraphim_persistence::Persistable;
use terraphim_types::{Document, Index};

/// Request payload for Perplexity API
#[derive(Debug, Serialize)]
struct PerplexityRequest {
    model: String,
    messages: Vec<PerplexityMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_domain_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_related_questions: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_recency_filter: Option<String>, // "month", "week", "day", "hour"
}

#[derive(Debug, Serialize)]
struct PerplexityMessage {
    role: String,
    content: String,
}

/// Response from Perplexity API
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PerplexityResponse {
    id: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    created: u64,
    #[serde(default)]
    usage: Option<PerplexityUsage>,
    #[serde(default)]
    choices: Vec<PerplexityChoice>,
    #[serde(default)]
    citations: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct PerplexityUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PerplexityChoice {
    index: usize,
    finish_reason: String,
    message: PerplexityResponseMessage,
    #[serde(default)]
    delta: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PerplexityResponseMessage {
    role: String,
    content: String,
}

/// Statistics for tracking API usage and performance
#[derive(Default, Debug)]
struct PerplexityStats {
    cache_hits: usize,
    cache_misses: usize,
    api_calls: usize,
    total_tokens: usize,
    avg_response_time_ms: u64,
}

/// Middleware that uses Perplexity API for AI-powered web search and research.
///
/// Features:
/// - Real-time web search with AI-powered summaries
/// - Citation tracking and source verification
/// - Configurable search domains and recency filters
/// - Response caching to reduce API costs
/// - Multiple model support (sonar-small-online, sonar-medium-online, sonar-large-online)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PerplexityHaystackIndexer {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    cache_ttl_hours: u64,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    search_domain_filter: Option<Vec<String>>,
    search_recency_filter: Option<String>,
}

impl PerplexityHaystackIndexer {
    /// Create a new PerplexityHaystackIndexer with configuration
    pub fn new(
        api_key: String,
        model: Option<String>,
        cache_ttl_hours: Option<u64>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        search_domain_filter: Option<Vec<String>>,
        search_recency_filter: Option<String>,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60)) // Generous timeout for AI responses
            .user_agent("Terraphim/1.0 (https://terraphim.ai)")
            .build()
            .map_err(crate::Error::Http)?;

        Ok(Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| "sonar-medium-online".to_string()),
            base_url: "https://api.perplexity.ai/chat/completions".to_string(),
            cache_ttl_hours: cache_ttl_hours.unwrap_or(1),
            max_tokens,
            temperature,
            search_domain_filter,
            search_recency_filter,
        })
    }

    /// Create indexer from haystack configuration
    pub fn from_haystack_config(haystack: &Haystack) -> Result<Self> {
        let extra_params = haystack.get_extra_parameters();

        // API key is required - check environment variable or extra parameters
        let api_key = extra_params
            .get("api_key")
            .cloned()
            .or_else(|| std::env::var("PERPLEXITY_API_KEY").ok())
            .ok_or_else(|| {
                crate::Error::Config(
                    terraphim_config::TerraphimConfigError::Config(
                        "Perplexity API key not found. Set PERPLEXITY_API_KEY environment variable or add 'api_key' to extra_parameters".to_string()
                    )
                )
            })?;

        let model = extra_params.get("model").cloned();
        let cache_ttl_hours = extra_params
            .get("cache_ttl_hours")
            .and_then(|s| s.parse().ok());
        let max_tokens = extra_params.get("max_tokens").and_then(|s| s.parse().ok());
        let temperature = extra_params.get("temperature").and_then(|s| s.parse().ok());

        let search_domain_filter = extra_params.get("search_domains").map(|domains| {
            domains
                .split(',')
                .map(|d| d.trim().to_string())
                .collect::<Vec<_>>()
        });

        let search_recency_filter = extra_params.get("search_recency").cloned();

        Self::new(
            api_key,
            model,
            cache_ttl_hours,
            max_tokens,
            temperature,
            search_domain_filter,
            search_recency_filter,
        )
    }

    /// Make a request to Perplexity API
    async fn make_perplexity_request(&self, query: &str) -> Result<PerplexityResponse> {
        let request = PerplexityRequest {
            model: self.model.clone(),
            messages: vec![PerplexityMessage {
                role: "user".to_string(),
                content: query.to_string(),
            }],
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: None, // Could be configurable in the future
            search_domain_filter: self.search_domain_filter.clone(),
            return_images: Some(false), // Focus on text content
            return_related_questions: Some(true),
            search_recency_filter: self.search_recency_filter.clone(),
        };

        log::debug!("Making Perplexity API request with model: {}", self.model);
        let start_time = std::time::Instant::now();

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(crate::Error::Http)?;

        let response_time = start_time.elapsed();
        log::info!(
            "Perplexity API response time: {}ms",
            response_time.as_millis()
        );

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let error_msg = format!("Perplexity API error {}: {}", status, error_text);
            log::error!("{}", error_msg);
            return Err(crate::Error::Validation(error_msg));
        }

        let perplexity_response: PerplexityResponse =
            response.json().await.map_err(crate::Error::Http)?;

        Ok(perplexity_response)
    }

    /// Convert Perplexity API response to Terraphim Documents
    fn convert_response_to_documents(
        &self,
        query: &str,
        response: PerplexityResponse,
    ) -> Vec<Document> {
        let mut documents = Vec::new();

        // Create a document from the main response content
        if let Some(choice) = response.choices.first() {
            let content = &choice.message.content;

            // Extract citations if available (clone to avoid move)
            let citations = response
                .citations
                .as_ref()
                .map(|c| c.join("\n\nSources:\n"))
                .unwrap_or_default();

            let full_content = if !citations.is_empty() {
                format!("{}\n\n{}", content, citations)
            } else {
                content.clone()
            };

            let doc_id = format!("perplexity_{}", self.normalize_query_for_id(query));

            let document = Document {
                id: doc_id.clone(),
                url: format!("perplexity://search/{}", urlencoding::encode(query)),
                title: format!("[Perplexity] {}", self.generate_title_from_query(query)),
                body: full_content,
                description: Some(format!(
                    "AI-powered web search results from Perplexity for: {}",
                    query
                )),
                summarization: Some(content.clone()),
                stub: Some(self.extract_stub(content)),
                tags: Some(vec![
                    "perplexity".to_string(),
                    "ai-search".to_string(),
                    "web-search".to_string(),
                    "real-time".to_string(),
                ]),
                rank: Some(1000), // High rank for AI-curated results
                source_haystack: None,
            };

            documents.push(document);
        }

        // If there are citations, create separate documents for sources (optional)
        if let Some(citations) = response.citations {
            for (i, citation) in citations.iter().enumerate() {
                if let Ok(url) = url::Url::parse(citation) {
                    let source_doc = Document {
                        id: format!(
                            "perplexity_source_{}_{}",
                            self.normalize_query_for_id(query),
                            i
                        ),
                        url: citation.clone(),
                        title: format!("[Source] {}", url.host_str().unwrap_or("Unknown")),
                        body: format!("Source reference for Perplexity search: {}", query),
                        description: Some("Source citation from Perplexity search".to_string()),
                        summarization: None,
                        stub: Some(format!("Source: {}", url.host_str().unwrap_or("Unknown"))),
                        tags: Some(vec![
                            "perplexity".to_string(),
                            "source".to_string(),
                            "citation".to_string(),
                        ]),
                        rank: Some(500), // Lower rank than main result
                        source_haystack: None,
                    };
                    documents.push(source_doc);
                }
            }
        }

        documents
    }

    /// Generate a meaningful title from the search query
    fn generate_title_from_query(&self, query: &str) -> String {
        let words: Vec<&str> = query.split_whitespace().collect();
        if words.len() > 8 {
            format!("{}...", words[..8].join(" "))
        } else {
            query.to_string()
        }
    }

    /// Extract a short stub from the content
    fn extract_stub(&self, content: &str) -> String {
        let words: Vec<&str> = content.split_whitespace().collect();
        let stub = if words.len() > 30 {
            format!("{}...", words[..30].join(" "))
        } else {
            content.to_string()
        };
        stub.chars().take(200).collect::<String>()
    }

    /// Normalize query for use as document ID
    fn normalize_query_for_id(&self, query: &str) -> String {
        let mut result = query
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();

        // Replace multiple consecutive underscores with single underscore
        while result.contains("__") {
            result = result.replace("__", "_");
        }

        // Remove leading/trailing underscores
        result = result.trim_matches('_').to_string();

        // Limit length
        result.chars().take(50).collect()
    }

    /// Generate cache key for the query
    fn get_cache_key(&self, query: &str) -> String {
        format!("perplexity_search_{}", self.normalize_query_for_id(query))
    }

    /// Check if cached response is still fresh
    fn is_cache_fresh(&self, cached_doc: &Document) -> bool {
        // For now, we'll consider cache fresh if the document exists
        // TODO: Add timestamp metadata to documents for proper cache expiration
        !cached_doc.body.is_empty()
    }

    /// Load cached results if available and fresh
    async fn load_cached_results(&self, query: &str) -> Option<Vec<Document>> {
        let cache_key = self.get_cache_key(query);
        let mut cache_placeholder = Document {
            id: cache_key,
            ..Default::default()
        };

        match cache_placeholder.load().await {
            Ok(cached_doc) => {
                if self.is_cache_fresh(&cached_doc) {
                    log::info!("Using cached Perplexity results for query: '{}'", query);
                    match serde_json::from_str::<Vec<Document>>(&cached_doc.body) {
                        Ok(cached_documents) => Some(cached_documents),
                        Err(e) => {
                            log::warn!(
                                "Failed to deserialize cached Perplexity results for '{}': {}",
                                query,
                                e
                            );
                            None
                        }
                    }
                } else {
                    log::debug!(
                        "Cached Perplexity results for '{}' are stale, fetching fresh results",
                        query
                    );
                    None
                }
            }
            Err(_) => {
                log::debug!("No cached Perplexity results found for query: '{}'", query);
                None
            }
        }
    }

    /// Save results to cache for future queries
    async fn save_to_cache(&self, query: &str, documents: &[Document]) -> Result<()> {
        let cache_key = self.get_cache_key(query);

        match serde_json::to_string(documents) {
            Ok(serialized_docs) => {
                let cache_doc = Document {
                    id: cache_key,
                    title: format!("Perplexity search results for '{}'", query),
                    body: serialized_docs,
                    url: format!("cache://perplexity/{}", query),
                    description: Some(format!(
                        "Cached search results from Perplexity API for query: {}",
                        query
                    )),
                    summarization: None,
                    stub: None,
                    tags: Some(vec!["perplexity".to_string(), "cache".to_string()]),
                    rank: None,
                    source_haystack: None,
                };

                if let Err(e) = cache_doc.save().await {
                    log::warn!("Failed to cache Perplexity results for '{}': {}", query, e);
                } else {
                    log::debug!(
                        "Cached {} Perplexity results for query: '{}'",
                        documents.len(),
                        query
                    );
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to serialize Perplexity results for caching '{}': {}",
                    query,
                    e
                );
            }
        }
        Ok(())
    }
}

impl Default for PerplexityHaystackIndexer {
    fn default() -> Self {
        Self {
            client: Client::new(),
            api_key: std::env::var("PERPLEXITY_API_KEY").unwrap_or_default(),
            model: "sonar-medium-online".to_string(),
            base_url: "https://api.perplexity.ai/chat/completions".to_string(),
            cache_ttl_hours: 1,
            max_tokens: None,
            temperature: None,
            search_domain_filter: None,
            search_recency_filter: None,
        }
    }
}

#[async_trait]
impl IndexMiddleware for PerplexityHaystackIndexer {
    #[allow(clippy::manual_async_fn)]
    fn index(
        &self,
        needle: &str,
        _haystack: &Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send {
        async move {
            // Validate that we have an API key
            if self.api_key.is_empty() {
                return Err(crate::Error::Config(
                    terraphim_config::TerraphimConfigError::Config(
                        "Perplexity API key not configured".to_string(),
                    ),
                ));
            }

            let mut stats = PerplexityStats::default();

            // First, check for cached results
            let documents = if let Some(cached_docs) = self.load_cached_results(needle).await {
                stats.cache_hits += 1;
                cached_docs
            } else {
                stats.cache_misses += 1;

                // Make API request to Perplexity
                let start_time = std::time::Instant::now();

                match self.make_perplexity_request(needle).await {
                    Ok(response) => {
                        let response_time = start_time.elapsed();
                        stats.api_calls += 1;
                        stats.avg_response_time_ms = response_time.as_millis() as u64;

                        // Track token usage if available
                        if let Some(usage) = &response.usage {
                            stats.total_tokens += usage.total_tokens as usize;
                            log::info!(
                                "Perplexity API usage - Prompt: {}, Completion: {}, Total: {}",
                                usage.prompt_tokens,
                                usage.completion_tokens,
                                usage.total_tokens
                            );
                        }

                        let documents = self.convert_response_to_documents(needle, response);

                        // Cache the results
                        if let Err(e) = self.save_to_cache(needle, &documents).await {
                            log::warn!("Failed to cache Perplexity results: {}", e);
                        }

                        documents
                    }
                    Err(e) => {
                        log::error!("Perplexity API request failed: {}", e);
                        // Return empty results rather than propagating the error
                        // This allows the system to gracefully degrade
                        Vec::new()
                    }
                }
            };

            // Log comprehensive statistics
            log::info!(
                "Perplexity processing complete: {} documents (cache: {} hits, {} misses, {} API calls, {} total tokens, {}ms avg response)",
                documents.len(),
                stats.cache_hits,
                stats.cache_misses,
                stats.api_calls,
                stats.total_tokens,
                stats.avg_response_time_ms
            );

            // Convert to Index format
            let mut index = Index::new();
            for doc in documents {
                index.insert(doc.id.clone(), doc);
            }

            Ok(index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_query_for_id() {
        let indexer = PerplexityHaystackIndexer::default();

        assert_eq!(
            indexer.normalize_query_for_id("What is Rust programming?"),
            "what_is_rust_programming"
        );

        assert_eq!(
            indexer.normalize_query_for_id("AI & Machine Learning 2024"),
            "ai_machine_learning_2024"
        );

        // Test length limiting
        let long_query = "a".repeat(100);
        assert!(indexer.normalize_query_for_id(&long_query).len() <= 50);
    }

    #[test]
    fn test_generate_title_from_query() {
        let indexer = PerplexityHaystackIndexer::default();

        assert_eq!(
            indexer.generate_title_from_query("Rust programming"),
            "Rust programming"
        );

        // Test truncation for long queries
        let long_query = "one two three four five six seven eight nine ten eleven twelve";
        let title = indexer.generate_title_from_query(long_query);
        assert!(title.ends_with("..."));
        assert!(title.split_whitespace().count() <= 9); // 8 words + "..."
    }

    #[test]
    fn test_extract_stub() {
        let indexer = PerplexityHaystackIndexer::default();

        let short_content = "This is a short response.";
        assert_eq!(indexer.extract_stub(short_content), short_content);

        let long_content = "word ".repeat(50);
        let stub = indexer.extract_stub(&long_content);
        assert!(stub.ends_with("..."));
        assert!(stub.len() <= 200);
    }

    #[tokio::test]
    async fn test_perplexity_config_parsing() {
        let mut extra_params = std::collections::HashMap::new();
        extra_params.insert("api_key".to_string(), "test_key".to_string());
        extra_params.insert("model".to_string(), "sonar-large-online".to_string());
        extra_params.insert("max_tokens".to_string(), "1000".to_string());
        extra_params.insert("cache_ttl_hours".to_string(), "2".to_string());
        extra_params.insert(
            "search_domains".to_string(),
            "example.com,test.org".to_string(),
        );

        let haystack = Haystack {
            location: "https://api.perplexity.ai".to_string(),
            service: terraphim_config::ServiceType::Perplexity,
            read_only: true,
            atomic_server_secret: None,
            extra_parameters: extra_params,
            fetch_content: false,
        };

        let indexer = PerplexityHaystackIndexer::from_haystack_config(&haystack).unwrap();
        assert_eq!(indexer.api_key, "test_key");
        assert_eq!(indexer.model, "sonar-large-online");
        assert_eq!(indexer.max_tokens, Some(1000));
        assert_eq!(indexer.cache_ttl_hours, 2);
        assert_eq!(
            indexer.search_domain_filter,
            Some(vec!["example.com".to_string(), "test.org".to_string()])
        );
    }

    #[test]
    fn test_cache_key_generation() {
        let indexer = PerplexityHaystackIndexer::default();
        let key = indexer.get_cache_key("What is Rust?");
        assert!(key.starts_with("perplexity_search_"));
        assert!(key.contains("what_is_rust"));
    }
}
