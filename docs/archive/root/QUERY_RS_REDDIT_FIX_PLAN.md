# Query.rs and Reddit Content Fetching Fix Plan

## Problem Statement

The `test_query_rs_crates_search` test is failing because:
1. Query.rs search returns mixed results (actual crates + Reddit posts about crates)
2. Reddit posts don't have crates.io API data fields (Description, Downloads, etc.)
3. Content is potentially being fetched multiple times for the same URL
4. No configurable option to control whether to fetch full content

## Solution Architecture

### 1. Add Fetch Content Configuration Parameter

**Location**: `terraphim_types::Haystack`

```rust
pub struct Haystack {
    pub location: String,
    pub service: ServiceType,
    pub read_only: Option<bool>,
    pub atomic_server_secret: Option<String>,
    pub extra_parameters: Option<Map<String, String>>,
    pub fetch_content: Option<bool>,  // NEW: Control content fetching
}
```

**Default Behavior**: `fetch_content = false` (lightweight, metadata only)

### 2. Implement URL Deduplication with HashMap

**Location**: `terraphim_middleware/src/haystack/query_rs.rs`

Add a URL tracking cache to the indexer:

```rust
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct QueryRsHaystackIndexer {
    client: reqwest::Client,
    fetched_urls: Arc<Mutex<HashSet<String>>>,  // Track fetched URLs
}

impl QueryRsHaystackIndexer {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            fetched_urls: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn should_fetch(&self, url: &str) -> bool {
        let mut cache = self.fetched_urls.lock().unwrap();
        if cache.contains(url) {
            log::debug!("⏭️ Skipping already fetched URL: {}", url);
            false
        } else {
            cache.insert(url.to_string());
            true
        }
    }
}
```

### 3. Differentiate Between Crates.io and Reddit Results

**Strategy**: Detect result type and apply appropriate fetching logic

```rust
enum QueryRsResultType {
    CratesIo { crate_name: String },
    Reddit { post_id: String },
    Other { url: String },
}

impl QueryRsHaystackIndexer {
    fn classify_result(&self, url: &str, title: &str) -> QueryRsResultType {
        if url.contains("crates.io/crates/") {
            let crate_name = extract_crate_name(url);
            QueryRsResultType::CratesIo { crate_name }
        } else if url.contains("reddit.com") || title.starts_with("[Reddit]") {
            let post_id = extract_reddit_post_id(url);
            QueryRsResultType::Reddit { post_id }
        } else {
            QueryRsResultType::Other { url: url.to_string() }
        }
    }
}
```

### 4. Conditional Content Fetching Logic

**Implementation Flow**:

```rust
async fn fetch_document_content(
    &self,
    url: &str,
    title: &str,
    result_type: QueryRsResultType,
    fetch_content: bool,
) -> Result<Document> {

    // Check if already fetched
    if !self.should_fetch(url) {
        return Ok(create_cached_placeholder(url, title));
    }

    let body = match result_type {
        QueryRsResultType::CratesIo { crate_name } => {
            if fetch_content {
                // Fetch from crates.io API
                self.fetch_crate_api_data(&crate_name).await
                    .unwrap_or_else(|e| {
                        log::warn!("Failed to fetch crate API for {}: {}", crate_name, e);
                        format!("Crate: {} (API unavailable)", crate_name)
                    })
            } else {
                format!("Crate: {} - {}", crate_name, url)
            }
        }

        QueryRsResultType::Reddit { post_id } => {
            if fetch_content {
                // Fetch Reddit post content via API
                self.fetch_reddit_post(&post_id).await
                    .unwrap_or_else(|e| {
                        log::warn!("Failed to fetch Reddit post {}: {}", post_id, e);
                        format!("Reddit discussion: {}", title)
                    })
            } else {
                format!("Reddit: {}", title)
            }
        }

        QueryRsResultType::Other { url } => {
            if fetch_content {
                self.fetch_generic_content(&url).await
                    .unwrap_or_else(|_| title.to_string())
            } else {
                title.to_string()
            }
        }
    };

    Ok(Document {
        id: generate_document_id(&url),
        url: url.to_string(),
        title: title.to_string(),
        body,
        description: None,
        stub: None,
        tags: None,
        rank: None,
    })
}
```

### 5. Update Configuration Handling

**Location**: `terraphim_middleware/src/haystack/query_rs.rs`

```rust
async fn index(&self, haystack: &Haystack) -> Result<BTreeMap<String, Document>> {
    let fetch_content = haystack.fetch_content.unwrap_or(false);

    log::info!(
        "🔍 Indexing query.rs haystack (fetch_content: {})",
        fetch_content
    );

    // Clear URL cache for fresh indexing
    {
        let mut cache = self.fetched_urls.lock().unwrap();
        cache.clear();
    }

    let query = extract_query_from_location(&haystack.location);
    let search_results = self.search_query_rs(&query).await?;

    let mut documents = BTreeMap::new();

    for result in search_results {
        let result_type = self.classify_result(&result.url, &result.title);

        let doc = self.fetch_document_content(
            &result.url,
            &result.title,
            result_type,
            fetch_content,
        ).await?;

        documents.insert(doc.id.clone(), doc);
    }

    log::info!("✅ Indexed {} documents from query.rs", documents.len());
    Ok(documents)
}
```

### 6. Reddit-Specific API Integration

**New Module**: `terraphim_middleware/src/haystack/reddit.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct RedditPostData {
    title: String,
    selftext: String,
    author: String,
    score: i32,
    num_comments: i32,
    created_utc: f64,
}

impl QueryRsHaystackIndexer {
    async fn fetch_reddit_post(&self, post_id: &str) -> Result<String> {
        let url = format!("https://www.reddit.com/comments/{}.json", post_id);

        let response: Value = self.client
            .get(&url)
            .header("User-Agent", "terraphim-indexer/0.2.0")
            .send()
            .await?
            .json()
            .await?;

        // Parse Reddit JSON structure
        let post_data = parse_reddit_response(response)?;

        Ok(format!(
            "Title: {}\n\nAuthor: u/{}\nScore: {} | Comments: {}\n\n{}",
            post_data.title,
            post_data.author,
            post_data.score,
            post_data.num_comments,
            post_data.selftext
        ))
    }
}
```

### 7. Update Test to Use Configuration

**Location**: `crates/terraphim_middleware/src/tests/query_rs_haystack_test.rs`

```rust
#[tokio::test]
async fn test_query_rs_crates_search() {
    let indexer = QueryRsHaystackIndexer::default();

    let haystack = Haystack {
        location: "https://query.rs/graph".to_string(),
        service: ServiceType::QueryRs,
        read_only: Some(true),
        atomic_server_secret: None,
        extra_parameters: None,
        fetch_content: Some(true),  // Enable content fetching for this test
    };

    // ... rest of test
}

#[tokio::test]
async fn test_query_rs_lightweight_mode() {
    let indexer = QueryRsHaystackIndexer::default();

    let haystack = Haystack {
        location: "https://query.rs/graph".to_string(),
        service: ServiceType::QueryRs,
        read_only: Some(true),
        atomic_server_secret: None,
        extra_parameters: None,
        fetch_content: Some(false),  // Metadata only, no content fetching
    };

    match indexer.index(&haystack).await {
        Ok(index) => {
            println!("Found {} documents (lightweight mode)", index.len());
            for (_id, doc) in index.iter() {
                // In lightweight mode, body should be minimal
                assert!(
                    doc.body.len() < 200,
                    "Lightweight mode should have minimal body content"
                );
            }
        }
        Err(e) => panic!("Failed to index: {:?}", e),
    }
}
```

### 8. Crates.io API Enhancement

**Existing Enhancement**: Already has crates.io API fetching

```rust
async fn fetch_crate_api_data(&self, crate_name: &str) -> Result<String> {
    let api_url = format!("https://crates.io/api/v1/crates/{}", crate_name);

    let response: Value = self.client
        .get(&api_url)
        .header("User-Agent", "terraphim-indexer/0.2.0")
        .send()
        .await?
        .json()
        .await?;

    // Extract relevant fields
    let crate_data = response["crate"].as_object()
        .ok_or_else(|| anyhow::anyhow!("Invalid crate data"))?;

    let description = crate_data["description"]
        .as_str()
        .unwrap_or("No description");
    let downloads = crate_data["downloads"]
        .as_i64()
        .unwrap_or(0);
    let version = crate_data["max_version"]
        .as_str()
        .unwrap_or("unknown");

    Ok(format!(
        "Description: {}\nVersion: {}\nDownloads: {}\n",
        description, version, downloads
    ))
}
```

## Implementation Checklist

### Phase 1: Core Infrastructure ✅
- [ ] Add `fetch_content` field to `Haystack` struct in `terraphim_types`
- [ ] Add URL deduplication cache to `QueryRsHaystackIndexer`
- [ ] Implement `should_fetch()` method with HashMap tracking
- [ ] Update TypeScript bindings if needed

### Phase 2: Result Classification 🔄
- [ ] Create `QueryRsResultType` enum
- [ ] Implement `classify_result()` method
- [ ] Add `extract_reddit_post_id()` helper (if not exists)
- [ ] Add `extract_crate_name()` helper (if not exists)

### Phase 3: Conditional Fetching Logic 📝
- [ ] Refactor `fetch_document_content()` to use `fetch_content` parameter
- [ ] Implement Reddit API fetching in separate module
- [ ] Add error handling with graceful degradation
- [ ] Add logging for fetch decisions

### Phase 4: Testing & Validation ✅
- [ ] Update `test_query_rs_crates_search` with `fetch_content: true`
- [ ] Add `test_query_rs_lightweight_mode` for `fetch_content: false`
- [ ] Add `test_url_deduplication` to verify HashSet works
- [ ] Add `test_mixed_results_classification` for crates + Reddit

### Phase 5: Documentation 📚
- [ ] Document `fetch_content` parameter in README
- [ ] Add examples of both modes to user guide
- [ ] Update API documentation
- [ ] Add performance notes about caching

## Configuration Examples

### Lightweight Mode (Default)
```json
{
  "location": "https://query.rs/graph",
  "service": "QueryRs",
  "fetch_content": false
}
```
**Result**: Fast indexing, minimal content, just titles and URLs

### Full Content Mode
```json
{
  "location": "https://query.rs/graph",
  "service": "QueryRs",
  "fetch_content": true
}
```
**Result**: Comprehensive indexing with API data, Reddit content, etc.

## Performance Considerations

1. **URL Deduplication**: O(1) lookup with HashSet
2. **Memory**: HashSet grows with unique URLs (typically < 1000 per query)
3. **Network**: Controlled by `fetch_content` flag
4. **Rate Limiting**: Add exponential backoff for API calls
5. **Caching**: Consider persisting URL cache across runs (future enhancement)

## Error Handling Strategy

1. **API Failures**: Log warning, return minimal content, don't fail entire index
2. **Network Errors**: Retry with backoff, then graceful degradation
3. **Parsing Errors**: Log error, use title as fallback content
4. **Rate Limits**: Respect 429 responses, implement exponential backoff

## Testing Strategy

1. **Unit Tests**: Each component (classification, fetching, deduplication)
2. **Integration Tests**: Full indexing with mocked HTTP responses
3. **E2E Tests**: Real API calls (marked with `#[ignore]` by default)
4. **Performance Tests**: Measure deduplication effectiveness

## Migration Path

1. **Backward Compatibility**: `fetch_content: None` defaults to `false`
2. **Existing Configs**: Continue to work without changes
3. **Opt-in Enhancement**: Users enable `fetch_content: true` when needed
4. **Deprecation**: None required (additive change)

## Files to Modify

1. `crates/terraphim_types/src/lib.rs` - Add `fetch_content` field
2. `crates/terraphim_middleware/src/haystack/query_rs.rs` - Main logic
3. `crates/terraphim_middleware/src/haystack/reddit.rs` - New module
4. `crates/terraphim_middleware/src/tests/query_rs_haystack_test.rs` - Tests
5. `crates/terraphim_config/src/lib.rs` - Config handling
6. `desktop/src/lib/generated/types.ts` - TypeScript bindings update

## Success Criteria

✅ All tests pass including the currently failing crates search test
✅ URL deduplication prevents duplicate fetches (verified in logs)
✅ `fetch_content: false` mode is fast (< 1s for typical query)
✅ `fetch_content: true` mode provides comprehensive data
✅ Reddit posts are properly handled and formatted
✅ Crates.io API data is correctly fetched and formatted
✅ No breaking changes to existing configurations
