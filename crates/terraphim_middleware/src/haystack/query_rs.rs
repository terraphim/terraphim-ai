use crate::indexer::IndexMiddleware;
use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use terraphim_config::Haystack;
use terraphim_types::{Document, Index};
use terraphim_persistence::Persistable;

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
}

impl Default for QueryRsHaystackIndexer {
    fn default() -> Self {
        // Create optimized client for API calls with shorter timeout
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Terraphim/1.0 (https://terraphim.ai)")
            .build()
            .unwrap_or_else(|_| Client::new());
            
        Self { client }
    }
}

#[async_trait]
impl IndexMiddleware for QueryRsHaystackIndexer {
    fn index(
        &self,
        needle: &str,
        _haystack: &Haystack,
    ) -> impl std::future::Future<Output = Result<Index>> + Send {
        async move {
            let mut documents = Vec::new();

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

            // Fetch and scrape content for documents that have URLs
            let mut enhanced_documents = Vec::new();
            for doc in documents {
                if !doc.url.is_empty() && doc.url.starts_with("http") {
                    match self.fetch_and_scrape_content(&doc).await {
                        Ok(enhanced_doc) => enhanced_documents.push(enhanced_doc),
                        Err(e) => {
                            log::warn!("Failed to fetch content for {}: {}", doc.url, e);
                            enhanced_documents.push(doc);
                        }
                    }
                } else {
                    enhanced_documents.push(doc);
                }
            }

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
    fn normalize_document_id(&self, original_id: &str) -> String {
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
        };
        dummy_doc.normalize_key(original_id)
    }

    /// Fetch and scrape content from a document's URL
    async fn fetch_and_scrape_content(&self, doc: &Document) -> Result<Document> {
        let mut enhanced_doc = doc.clone();

        log::info!("Fetching content from: {}", doc.url);

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
                        }
                        Err(e) => {
                            log::warn!("Failed to get text content from {}: {}", doc.url, e);
                        }
                    }
                } else {
                    log::warn!(
                        "Failed to fetch {} with status: {}",
                        doc.url,
                        response.status()
                    );
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch {}: {}", doc.url, e);
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
    fn parse_reddit_json(&self, posts: Vec<Value>) -> Result<Vec<Document>> {
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
                    let original_id = format!("reddit-{}", url);
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
                    });
                }
            }
        }

        Ok(documents)
    }

    /// Parse suggest API JSON results (OpenSearch Suggestions format)
    fn parse_suggest_json(&self, suggest_data: Value, query: &str) -> Result<Vec<Document>> {
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
                                    let tags = self.generate_tags_for_search_type(&search_type);

                                    let original_id = format!("{}-{}", search_type, url);
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
    fn parse_crates_io_json(&self, crates_data: Value, _query: &str) -> Result<Vec<Document>> {
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
                    let url = format!("https://crates.io/crates/{}", name);

                    if !name.is_empty() {
                        let original_id = format!("crate-{}", name);
                        let normalized_id = self.normalize_document_id(&original_id);
                        documents.push(Document {
                            id: normalized_id,
                            url: url.clone(),
                            title: format!("[CRATE] {} {}", name, version),
                            description: Some(format!("{} ({} downloads)", description, downloads)),
                            summarization: None,
                            body: format!("Crate: {} - {}", name, description),
                            stub: None,
                            tags: Some(vec![
                                "rust".to_string(),
                                "crate".to_string(),
                                "package".to_string(),
                            ]),
                            rank: Some(downloads),
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
}
