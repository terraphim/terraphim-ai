# QueryRs Haystack Integration

## Overview

The QueryRs haystack provides comprehensive Rust documentation search capabilities by integrating with [query.rs](https://query.rs), a powerful search engine for Rust. This haystack supports searching across multiple Rust documentation sources including Reddit posts, standard library documentation, crates, attributes, lints, books, caniuse, and error codes.

## Features

### ✅ Currently Working
- **Reddit Posts**: Full JSON API integration with `/posts/search?q=keyword`
  - Returns Reddit posts from r/rust with metadata (author, score, URL)
  - Proper tagging and formatting with `[Reddit]` prefix
  - Real-time search results from the Rust community

### 🔄 Planned Implementation
- **Standard Library Docs**: Stable and nightly documentation search
- **Crates.io**: Search for Rust crates and their documentation
- **Attributes**: Rust attribute search (e.g., `#[derive]`, `#[cfg]`)
- **Clippy Lints**: Search for Clippy lint documentation
- **Rust Books**: Search official Rust book content
- **Caniuse**: Rust feature compatibility search
- **Error Codes**: Rust compiler error code documentation

## Architecture

### Search Types Supported

The QueryRs haystack implements concurrent search across all query.rs endpoints:

```rust
// Concurrent search across all endpoints
let (reddit_results, stable_results, nightly_results, crates_results, 
     attributes_results, lints_results, books_results, caniuse_results, 
     error_results) = tokio::join!(
    self.search_reddit_posts(needle),
    self.search_std_docs(needle, "stable"),
    self.search_std_docs(needle, "nightly"),
    self.search_crates(needle),
    self.search_attributes(needle),
    self.search_lints(needle),
    self.search_books(needle),
    self.search_caniuse(needle),
    self.search_error_codes(needle),
);
```

### API Endpoints

| Search Type | Endpoint | Status | Response Format |
|-------------|----------|--------|-----------------|
| Reddit Posts | `/posts/search?q=keyword` | ✅ Working | JSON |
| Std Docs (Stable) | `/stable?q=keyword` | 🔄 Planned | HTML |
| Std Docs (Nightly) | `/nightly?q=keyword` | 🔄 Planned | HTML |
| Crates | `/crates?q=keyword` | 🔄 Planned | HTML |
| Attributes | `/attributes?q=keyword` | 🔄 Planned | HTML |
| Lints | `/lints?q=keyword` | 🔄 Planned | HTML |
| Books | `/books?q=keyword` | 🔄 Planned | HTML |
| Caniuse | `/caniuse?q=keyword` | 🔄 Planned | HTML |
| Error Codes | `/errors?q=keyword` | 🔄 Planned | HTML |

## Configuration

### Rust Engineer Role

The QueryRs haystack is configured through the "Rust Engineer" role:

```json
{
  "Rust Engineer": {
    "shortname": "rust-engineer",
    "name": "Rust Engineer",
    "relevance_function": "title-scorer",
    "terraphim_it": false,
    "theme": "cosmo",
    "kg": null,
    "haystacks": [
      {
        "location": "https://query.rs",
        "service": "QueryRs",
        "read_only": true,
        "atomic_server_secret": null,
        "extra_parameters": {}
      }
    ],
    "extra": {}
  }
}
```

## Usage

### Server Startup

```bash
cargo run --bin terraphim_server -- --config terraphim_server/default/rust_engineer_config.json
```

### Search Examples

```bash
# Search for async programming content
curl -X POST http://localhost:8000/documents/search \
  -H 'Content-Type: application/json' \
  -d '{"search_term": "async", "role": "Rust Engineer"}'

# Search for standard library items
curl -X POST http://localhost:8000/documents/search \
  -H 'Content-Type: application/json' \
  -d '{"search_term": "Iterator", "role": "Rust Engineer"}'

# Search for crates
curl -X POST http://localhost:8000/documents/search \
  -H 'Content-Type: application/json' \
  -d '{"search_term": "tokio", "role": "Rust Engineer"}'

# Search for attributes
curl -X POST http://localhost:8000/documents/search \
  -H 'Content-Type: application/json' \
  -d '{"search_term": "derive", "role": "Rust Engineer"}'

# Search for Clippy lints
curl -X POST http://localhost:8000/documents/search \
  -H 'Content-Type: application/json' \
  -d '{"search_term": "if_let", "role": "Rust Engineer"}'

# Search for error codes
curl -X POST http://localhost:8000/documents/search \
  -H 'Content-Type: application/json' \
  -d '{"search_term": "E0038", "role": "Rust Engineer"}'
```

## Document Format

### Reddit Posts (Currently Working)

```json
{
  "id": "reddit-https://www.reddit.com/r/rust/comments/...",
  "url": "https://www.reddit.com/r/rust/comments/...",
  "title": "[Reddit] Announcing \"yap\": A small, iterator based, zero dependency parsing library",
  "description": "by username (score: 123)",
  "body": "Post content...",
  "tags": ["rust", "reddit", "community"],
  "rank": 123
}
```

### Planned Document Formats

#### Standard Library Docs
```json
{
  "id": "std-stable-iterator",
  "url": "https://doc.rust-lang.org/std/iter/trait.Iterator.html",
  "title": "[STABLE] Iterator",
  "description": "A trait for dealing with iterators.",
  "tags": ["rust", "std", "stable", "trait"]
}
```

#### Crates
```json
{
  "id": "crate-tokio",
  "url": "https://crates.io/crates/tokio",
  "title": "tokio",
  "description": "An asynchronous runtime for Rust",
  "tags": ["rust", "crate", "async"]
}
```

## Implementation Details

### QueryRsHaystackIndexer

The main implementation is in `crates/terraphim_middleware/src/haystack/query_rs.rs`:

```rust
#[derive(Debug, Clone)]
pub struct QueryRsHaystackIndexer {
    client: Client,
}

impl QueryRsHaystackIndexer {
    // Reddit posts search (JSON API)
    async fn search_reddit_posts(&self, query: &str) -> Result<Vec<Document>>
    
    // Standard library docs search (HTML parsing)
    async fn search_std_docs(&self, query: &str, channel: &str) -> Result<Vec<Document>>
    
    // Crates search (HTML parsing)
    async fn search_crates(&self, query: &str) -> Result<Vec<Document>>
    
    // Attributes search (HTML parsing)
    async fn search_attributes(&self, query: &str) -> Result<Vec<Document>>
    
    // Clippy lints search (HTML parsing)
    async fn search_lints(&self, query: &str) -> Result<Vec<Document>>
    
    // Books search (HTML parsing)
    async fn search_books(&self, query: &str) -> Result<Vec<Document>>
    
    // Caniuse search (HTML parsing)
    async fn search_caniuse(&self, query: &str) -> Result<Vec<Document>>
    
    // Error codes search (HTML parsing)
    async fn search_error_codes(&self, query: &str) -> Result<Vec<Document>>
}
```

### Error Handling

The implementation uses graceful degradation:
- Network failures return empty results instead of errors
- API errors are logged as warnings and continue with other sources
- Parse errors are handled gracefully for malformed responses

### Performance

- **Concurrent Search**: All endpoints are searched concurrently using `tokio::join!`
- **Caching**: Results can be cached to improve performance
- **Rate Limiting**: Consider implementing rate limiting for production use

## Testing

### End-to-End Testing

Run the comprehensive test script:

```bash
./test_rust_engineer_api.sh
```

This script validates:
- Server startup and configuration
- Role configuration updates
- Search functionality across multiple query types
- Result formatting and metadata

### Test Queries

The test script covers various search types:

```bash
# Reddit posts
"async", "tokio", "serde"

# Standard library docs
"Iterator", "Vec", "Result"

# Attributes
"derive", "cfg"

# Clippy lints
"if_let", "try"

# Books
"pin", "error"

# Caniuse
"const", "slice"

# Error codes
"E0038"
```

## Dependencies

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
scraper = "0.19.0"
serde_json = "1.0"
async-trait = "0.1"
```

## Future Enhancements

### HTML Parsing Implementation

The next phase involves implementing HTML parsing for the remaining endpoints:

1. **Standard Library Docs**: Parse search results from `/stable` and `/nightly`
2. **Crates**: Parse crate search results from `/crates`
3. **Attributes**: Parse attribute documentation from `/attributes`
4. **Lints**: Parse Clippy lint documentation from `/lints`
5. **Books**: Parse Rust book content from `/books`
6. **Caniuse**: Parse feature compatibility from `/caniuse`
7. **Error Codes**: Parse error code documentation from `/errors`

### Advanced Features

- **Type Signature Search**: Support for searching by function signatures
- **Advanced Filtering**: Filter results by type, category, or source
- **Result Ranking**: Improved ranking based on relevance and popularity
- **Caching Strategy**: Implement intelligent caching for frequently searched terms
- **Rate Limiting**: Add rate limiting to respect query.rs API limits

## Troubleshooting

### Common Issues

1. **No Results from HTML Endpoints**: HTML parsing is not yet implemented for most endpoints
2. **Network Timeouts**: Increase timeout values for slow network connections
3. **Rate Limiting**: Implement delays between requests if hitting rate limits

### Debug Commands

```bash
# Test Reddit API directly
curl -s "https://query.rs/posts/search?q=async" | jq '.[0]'

# Test server health
curl -s http://localhost:8000/health

# Check server logs for parsing errors
tail -f logs/terraphim-server.log
```

## Contributing

To implement HTML parsing for additional endpoints:

1. Analyze the HTML structure of the target endpoint
2. Implement the corresponding `parse_*_html` function
3. Add appropriate CSS selectors for data extraction
4. Test with various search queries
5. Update documentation

## License

This implementation follows the same license as the Terraphim project. 