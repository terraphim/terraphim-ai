# Terraphim AI Lessons Learned

## Enhanced QueryRs Haystack Implementation (2025-01-31)

### 🎯 Key Success Factors

1. **API Discovery is Critical**
   - **Lesson**: Initially planned HTML parsing, but discovered `/suggest/{query}` JSON API
   - **Discovery**: query.rs has server-side JSON APIs, not just client-side HTML
   - **Benefit**: Much more reliable than HTML parsing, better performance

2. **OpenSearch Suggestions Format**
   - **Lesson**: `/suggest/{query}` returns OpenSearch Suggestions format
   - **Format**: `[query, [completions], [descriptions], [urls]]`
   - **Parsing**: Completion format is `"title - url"` with space-dash-space separator
   - **Implementation**: Smart parsing with `split_once(" - ")`

3. **Configuration Loading Priority**
   - **Lesson**: Server hardcoded to load `terraphim_engineer_config.json` first
   - **Discovery**: Custom config files need to be integrated into default loading path
   - **Solution**: Updated existing config file instead of creating new one

4. **Concurrent API Integration**
   - **Lesson**: Using `tokio::join!` for parallel API calls improves performance
   - **Implementation**: Reddit API + Suggest API called concurrently
   - **Benefit**: Faster response times and better user experience

### 🔧 Technical Implementation Insights

1. **Smart Search Type Detection**
```rust
fn determine_search_type(&self, title: &str, url: &str) -> &'static str {
    if url.contains("doc.rust-lang.org") {
        if title.contains("attr.") { "attribute" }
        else if title.contains("trait.") { "trait" }
        else if title.contains("struct.") { "struct" }
        // ... more patterns
    }
}
```

2. **Result Classification**
   - **Reddit Posts**: Community discussions with score ranking
   - **Std Documentation**: Official Rust documentation with proper categorization
   - **Tag Generation**: Automatic tag assignment based on content type

3. **Error Handling Strategy**
   - Return empty results instead of errors for network failures
   - Log warnings for debugging but don't fail the entire search
   - Graceful degradation improves user experience

### 🚀 Performance and Reliability

1. **API Response Times**
   - Reddit API: ~500ms average response time
   - Suggest API: ~300ms average response time
   - Combined: <2s total response time
   - Concurrent calls reduce total latency

2. **Result Quality**
   - **Reddit**: 20+ results per query (community discussions)
   - **Std Docs**: 5-10 results per query (official documentation)
   - **Combined**: 25-30 results per query (comprehensive coverage)

3. **Reliability**
   - JSON APIs more reliable than HTML parsing
   - Graceful fallback when one API fails
   - No brittle CSS selectors or HTML structure dependencies

### 📊 Testing Best Practices

1. **Comprehensive Test Scripts**
```bash
# Test multiple search types
test_search "Iterator" 10 "std library trait"
test_search "derive" 5 "Rust attributes"
test_search "async" 15 "async/await"
```

2. **Result Validation**
   - Count results by type (Reddit vs std)
   - Validate result format and content
   - Check performance metrics

3. **Configuration Testing**
   - Verify role availability
   - Test configuration loading
   - Validate API integration

### 🎯 User Experience Considerations

1. **Result Formatting**
   - Clear prefixes: `[Reddit]` for community posts, `[STD]` for documentation
   - Descriptive titles with full std library paths
   - Proper tagging for filtering and categorization

2. **Search Coverage**
   - Comprehensive coverage of Rust ecosystem
   - Community insights + official documentation
   - Multiple search types (traits, structs, functions, modules)

3. **Performance**
   - Fast response times (<2s)
   - Concurrent API calls
   - Graceful error handling

### 🔍 Debugging Techniques

1. **API Inspection**
```bash
# Check suggest API directly
curl -s "https://query.rs/suggest/Iterator" | jq '.[1][0]'

# Test server configuration
curl -s http://localhost:8000/config | jq '.config.roles | keys'
```

2. **Result Analysis**
   - Count results by type
   - Validate result format
   - Check performance metrics

3. **Configuration Debugging**
   - Verify config file loading
   - Check role availability
   - Validate API endpoints

### 📈 Success Metrics

- ✅ **28 results** for "Iterator" (20 Reddit + 8 std docs)
- ✅ **21 results** for "derive" (Reddit posts)
- ✅ **<2s response time** for comprehensive searches
- ✅ **Multiple search types** supported (traits, structs, functions, modules)
- ✅ **Error handling** graceful and informative
- ✅ **Configuration integration** seamless

### 🚀 Future Enhancements

## OpenRouter Summarization + Chat (2025-08-08)

### Key Insights
- Feature-gated integration lets default builds stay lean; enable with `--features openrouter` on server/desktop.
- Role config needs sensible defaults for all OpenRouter fields to avoid initializer errors.
- Summarization must handle `Option<Document>` carefully and avoid holding config locks across awaits.

### Implementation Notes
- Backend:
  - Added endpoints: POST `/documents/summarize`, POST `/chat` (axum).
  - `OpenRouterService` used for summaries and chat completions; rate-limit and error paths covered.
  - `Role` extended with: `openrouter_auto_summarize`, `openrouter_chat_enabled`, `openrouter_chat_model`, `openrouter_chat_system_prompt`.
  - Fixed borrow checker issues by cloning role prior to dropping lock; corrected `get_document_by_id` usage.
- Desktop:
  - `ConfigWizard.svelte` updated to expose auto-summarize and chat settings.
  - New `Chat.svelte` with minimal streaming-free chat UI (Enter to send, model hint, error display).

### Testing
- Build server: `cargo build -p terraphim_server --features openrouter` (compiles green).
- Manual chat test via curl:
  ```bash
  curl -s X POST "$SERVER/chat" -H 'Content-Type: application/json' -d '{"role":"Default","messages":[{"role":"user","content":"hello"}]}' | jq
  ```

### Future Work
- Add streaming SSE for chat, caching for summaries, and model list fetch UI.
1. **Advanced Query Syntax**
   - Support for `optionfn:findtrait:Iterator` syntax
   - Function signature search
   - Type signature matching

2. **Performance Optimization**
   - Result caching for frequent queries
   - Rate limiting for API calls
   - Connection pooling

3. **Feature Expansion**
   - Support for books, lints, caniuse, error codes
   - Advanced filtering options
   - Result ranking improvements

## QueryRs Haystack Integration (2025-01-29)

### 🎯 Key Success Factors

1. **Repository Analysis is Critical**
   - Always clone and examine the actual repository structure
   - Don't assume API endpoints based on URL patterns
   - Look for server-side code to understand actual implementation

2. **API Response Format Verification**
   - **Lesson**: Initially assumed query.rs returned JSON, but it returns HTML for most endpoints
   - **Solution**: Used `curl` and `jq` to verify actual response formats
   - **Discovery**: Only `/posts/search?q=keyword` returns JSON (Reddit posts)

3. **Incremental Implementation Approach**
   - Start with working endpoints (Reddit JSON API)
   - Leave placeholders for complex features (HTML parsing)
   - Focus on end-to-end functionality first

4. **End-to-End Testing is Essential**
   - Unit tests with mocked responses miss real-world issues
   - Use `curl` and `jq` for API validation
   - Test actual server startup and configuration updates

### 🔧 Technical Implementation Insights

1. **Async Trait Implementation**
```rust
   // Correct pattern for async traits
   fn index(
       &self,
       needle: &str,
       _haystack: &Haystack,
   ) -> impl std::future::Future<Output = Result<Index>> + Send {
       async move {
           // Implementation here
  }
}
```

2. **Error Handling Strategy**
   - Return empty results instead of errors for network failures
   - Log warnings for debugging but don't fail the entire search
   - Graceful degradation improves user experience

3. **Type Safety**
   - `rank: Option<u64>` not `Option<f64>` in Document struct
   - Always check actual type definitions, not assumptions

### 🚀 Performance and Reliability

1. **External API Dependencies**
   - QueryRs Reddit API is reliable and fast
   - Consider rate limiting for production use
   - Cache results when possible

2. **HTML Parsing Complexity**
   - Server-rendered HTML is harder to parse than JSON
   - CSS selectors can be brittle
   - Consider using dedicated HTML parsing libraries

### 📊 Testing Best Practices

1. **Comprehensive Test Scripts**
```bash
   # Test server health
   curl -s http://localhost:8000/health
   
   # Test configuration updates
   curl -X POST http://localhost:8000/config -H "Content-Type: application/json" -d @config.json
   
   # Test search functionality
   curl -X POST http://localhost:8000/documents/search -H "Content-Type: application/json" -d '{"search_term": "async", "role": "Rust Engineer"}'
   ```

2. **Validation Points**
   - Server startup and health
   - Configuration loading and updates
   - Role recognition and haystack integration
   - Search result format and content

### 🎯 User Experience Considerations

1. **Result Formatting**
   - Clear prefixes: `[Reddit]` for Reddit posts
   - Descriptive titles with emojis preserved
   - Author and score information included

2. **Error Messages**
   - Informative but not overwhelming
   - Graceful fallbacks when services are unavailable
   - Clear indication of what's working vs. what's not

### 🔍 Debugging Techniques

1. **API Inspection**
```bash
   # Check actual response format
   curl -s "https://query.rs/posts/search?q=async" | jq '.[0]'
   
   # Verify HTML vs JSON responses
   curl -s "https://query.rs/reddit" | head -10
   ```

2. **Server Logs**
   - Enable debug logging for development
   - Check for network errors and timeouts
   - Monitor response parsing success/failure

### 📈 Success Metrics

- ✅ **20 results returned** for each test query
- ✅ **Proper Reddit metadata** (author, score, URL)
- ✅ **Server configuration updates** working
- ✅ **Role-based search** functioning correctly
- ✅ **Error handling** graceful and informative

### 🚀 Future Enhancements

1. **HTML Parsing Implementation**
   - Analyze query.rs crates page structure
   - Implement std docs parsing
   - Add pagination support

2. **Performance Optimization**
   - Implement result caching
   - Add rate limiting
   - Consider parallel API calls

3. **Feature Expansion**
   - Add more query.rs endpoints
   - Implement search result filtering
   - Add result ranking improvements

## Previous Lessons

### Atomic Server Integration
- Public access pattern works well for read operations
- Environment variable loading from project root is crucial
- URL construction requires proper slashes

### BM25 Implementation
- Multiple relevance function variants provide flexibility
- Integration with existing pipeline requires careful type handling
- Performance testing is essential for ranking algorithms

### TypeScript Bindings
- Generated types ensure consistency across frontend and backend
- Single source of truth prevents type drift
- Proper integration requires updating all consuming components 

## ClickUp Haystack Integration (2025-08-09)

- Prefer list-based search (`/api/v2/list/{list_id}/task?search=...`) when `list_id` is provided; otherwise team-wide search via `/api/v2/team/{team_id}/task?query=...`.
- Map `text_content` (preferred) or `description` into `Document.body`; construct URL as `https://app.clickup.com/t/<task_id>`.
- Read `CLICKUP_API_TOKEN` from env; pass scope (`team_id`, `list_id`) and flags (`include_closed`, `subtasks`, `page`) via `Haystack.extra_parameters`.
- Keep live API tests `#[ignore]` and provide a non-live test that verifies behavior without credentials.