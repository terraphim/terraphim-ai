# Duplicate Handling in Terraphim AI

## Overview

This document explains how Terraphim AI handles duplicate results when searching across multiple haystacks (data sources), particularly when using combinations like QueryRs and GrepApp for code search.

## Current Behavior

### How Results Are Merged

When a role has multiple haystacks configured, Terraphim AI searches each haystack independently and merges the results into a single index. The merging process is implemented in `crates/terraphim_middleware/src/indexer/mod.rs`:

```rust
pub async fn search_haystacks(config_state, search_query) -> Result<Index> {
    let mut full_index = Index::new();  // HashMap<String, Document>

    for haystack in &role.haystacks {
        let index = match haystack.service {
            ServiceType::QueryRs => query_rs.index(needle, haystack).await?,
            ServiceType::GrepApp => grep_app.index(needle, haystack).await?,
            // ... other haystack types
        };

        // Tag documents with source
        for (doc_id, mut document) in index {
            document.source_haystack = Some(haystack.location.clone());
            tagged_index.insert(doc_id, document);
        }

        full_index.extend(tagged_index);  // Merge into combined index
    }
    Ok(full_index)
}
```

### Key Points

1. **HashMap-Based Merging**: Results are stored in a `HashMap<String, Document>` where the key is the document ID
2. **Document ID Uniqueness**: Each haystack generates its own document IDs:
   - **QueryRs**: Uses URLs from API responses (e.g., `https://docs.rs/tokio/...`)
   - **GrepApp**: Uses format `grepapp:{repo}:{branch}:{path}` (e.g., `grepapp:tokio_tokio_main_src_lib.rs`)
3. **Last-Wins Strategy**: When using `HashMap::extend()`, if two documents have the same ID, the last one (from the most recently processed haystack) overwrites the previous one
4. **No Explicit Deduplication**: There is **no automatic deduplication** based on URL normalization or content similarity

### Source Tracking

Every document is tagged with its source haystack via the `source_haystack` field:
- Example: `"source_haystack": "https://query.rs"`
- Example: `"source_haystack": "https://grep.app"`

This allows users to see which haystack provided each result.

## Duplicate Scenarios

### Scenario 1: Same File from Different Sources

**Example**: Searching for "tokio spawn" with Rust Engineer role (QueryRs + GrepApp)

- **QueryRs** returns: `https://github.com/tokio-rs/tokio/blob/master/src/task/mod.rs`
- **GrepApp** returns: Same file with ID `grepapp:tokio-rs_tokio_master_src_task_mod.rs`

**Current Behavior**: Both results appear in search results as separate documents because they have different document IDs.

**User Impact**:
- ✅ Users can see the same result from multiple sources
- ⚠️ Results may appear duplicated in the UI
- ℹ️ The `source_haystack` field distinguishes the sources

### Scenario 2: URL Duplicates

**Example**: Same URL returned by two haystacks with different parameters

**Current Behavior**: If the exact same URL is returned but with different document IDs, both results appear.

**User Impact**:
- May see identical links
- Different snippets or metadata might provide additional context
- Relevance scoring applied to each result independently

### Scenario 3: Content Duplicates

**Example**: Different URLs pointing to the same content (mirrors, forks, copies)

**Current Behavior**: No content-based deduplication. Each unique URL is treated as a separate document.

## Relevance Function Behavior

All relevance functions tested show the same duplicate handling behavior since deduplication happens (or doesn't happen) at the indexing level before relevance scoring:

| Relevance Function | Duplicate Handling Behavior |
|--------------------|----------------------------|
| **TitleScorer**    | No deduplication; all results scored independently |
| **BM25**           | No deduplication; TF-IDF scoring applied to all results |
| **BM25F**          | No deduplication; field-weighted scoring applied to all |
| **BM25Plus**       | No deduplication; enhanced BM25 applied to all |
| **TerraphimGraph** | Not applicable; uses single KG source, not multiple remote haystacks |

### Test Results Example

```
Query: "tokio spawn"

TitleScorer:
  Total: 18, Unique URLs: 16, Duplicates: 2
  QueryRs: 9, GrepApp: 9

BM25:
  Total: 18, Unique URLs: 16, Duplicates: 2
  QueryRs: 9, GrepApp: 9
```

## Implementation Details

### Document ID Generation

#### QueryRs Haystack
```rust
// Uses URL from API response
let doc_id = doc.url.clone();
```

#### GrepApp Haystack
```rust
// Constructs ID from repo, branch, and path
let doc_id = format!("grepapp:{}:{}:{}",
    repo.replace('/', "_"),
    branch.replace('/', "_"),
    path.replace('/', "_").replace('.', "_")
);
```

### Source Tagging
```rust
document.source_haystack = Some(haystack.location.clone());
```

## Known Limitations

1. **No URL Normalization**: URLs with different query parameters or fragments are treated as different documents
2. **No Content Hashing**: Identical content at different URLs appears as separate results
3. **No Fuzzy Matching**: Similar titles or snippets don't trigger deduplication
4. **Last-Wins Overwriting**: If somehow two haystacks generate the same document ID, only the last one is kept

## User Recommendations

### For Users

1. **Use Source Filters**: Check the `source_haystack` field to understand where results come from
2. **Limit Results**: Use the `limit` parameter to control result set size
3. **Review All Sources**: Duplicates from different sources may have different snippets or context
4. **URL Comparison**: Manually compare URLs to identify duplicates

### For Developers

1. **Unique ID Generation**: Ensure each haystack generates truly unique document IDs
2. **Consistent Formatting**: Maintain consistent URL formatting across haystacks
3. **Source-Specific Metadata**: Leverage `source_haystack` for filtering and grouping

## Future Enhancement Opportunities

### Potential Deduplication Strategies

1. **URL Normalization**
   ```rust
   fn normalize_url(url: &str) -> String {
       // Remove query params and fragments
       // Standardize protocol (https)
       // Remove trailing slashes
   }
   ```

2. **Content-Based Hashing**
   ```rust
   fn content_hash(body: &str) -> String {
       // Hash document body
       // Merge documents with same hash
   }
   ```

3. **GitHub URL Detection**
   ```rust
   fn is_github_url_match(url1: &str, url2: &str) -> bool {
       // Extract repo, branch, path
       // Compare components
       // Handle blob vs raw URLs
   }
   ```

4. **Fuzzy Title Matching**
   ```rust
   fn title_similarity(title1: &str, title2: &str) -> f64 {
       // Calculate similarity score
       // Merge if above threshold
   }
   ```

5. **Post-Processing Deduplication**
   ```rust
   fn deduplicate_results(results: Vec<Document>) -> Vec<Document> {
       // Group by normalized URL
       // Keep highest-ranked result
       // Optionally merge snippets
   }
   ```

## Testing

Comprehensive tests for duplicate handling are available in:
- `terraphim_server/tests/relevance_functions_duplicate_test.rs`
- `terraphim_server/tests/rust_engineer_enhanced_integration_test.rs`

Run tests with:
```bash
# Run duplicate handling tests
cargo test -p terraphim_server --test relevance_functions_duplicate_test -- --ignored

# Run Rust Engineer dual haystack test
cargo test -p terraphim_server --test rust_engineer_enhanced_integration_test -- --ignored
```

## Examples

### Configuration Example

Rust Engineer role with multiple haystacks:

```json
{
  "name": "Rust Engineer",
  "haystacks": [
    {
      "location": "https://query.rs",
      "service": "QueryRs",
      "read_only": true,
      "extra_parameters": {
        "disable_content_enhancement": "true"
      }
    },
    {
      "location": "https://grep.app",
      "service": "GrepApp",
      "read_only": true,
      "extra_parameters": {
        "language": "Rust",
        "repo": "",
        "path": ""
      }
    }
  ]
}
```

### Search Result Example

```json
{
  "results": [
    {
      "id": "https://docs.rs/tokio/latest/tokio/task/fn.spawn.html",
      "url": "https://docs.rs/tokio/latest/tokio/task/fn.spawn.html",
      "title": "tokio::task::spawn - Rust",
      "source_haystack": "https://query.rs"
    },
    {
      "id": "grepapp:tokio-rs_tokio_master_tokio_src_task_spawn.rs",
      "url": "https://github.com/tokio-rs/tokio/blob/master/tokio/src/task/spawn.rs",
      "title": "tokio/src/task/spawn.rs",
      "source_haystack": "https://grep.app"
    }
  ]
}
```

## Conclusion

Terraphim AI's current duplicate handling strategy prioritizes:
1. **Transparency**: All results are shown with source attribution
2. **Completeness**: No potentially valuable results are filtered out
3. **Simplicity**: Straightforward merging logic without complex deduplication

This approach allows users to see all relevant results from multiple sources, with the trade-off of potential duplication. The `source_haystack` field enables users to understand and manage duplicates as needed.

For applications requiring strict deduplication, custom post-processing or configuration adjustments (limiting to a single haystack) may be appropriate.

---

**Last Updated**: 2025-11-14
**Test Coverage**: ✅ Comprehensive integration tests available
**Status**: Current behavior documented and tested
