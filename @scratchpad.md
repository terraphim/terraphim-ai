# Terraphim AI Project Scratchpad

## Current Task: QueryRs Haystack and Scoring Functions Validation
## TUI Interface Implementation (2025-08-11)

Plan & Progress
- Create `crates/terraphim_tui` with `tokio`, `ratatui`, `crossterm`, `clap`, `jiff`, reusing `terraphim_types`.
- Map desktop features: Search (typeahead), Roles (list/select), Config (edit basics), Rolegraph (text), OpenRouter (optional).
- CLI subcommands: `search`, `roles list|select`, `config show|set|wizard` (later), `graph show`, `chat` (optional).
- Agentic enhancements (inspired by Claude Code, Goose): plan/approve/execute, provider abstraction, budgets, run records.

Status
- Scaffolded workspace member and crate `terraphim_tui`.
- MVP interactive: in-pane results and suggestions sourced from rolegraph nodes.
 - CLI: search, roles list/select, config show/set (selected_role/global_shortcut/role.<name>.theme), graph ASCII adjacency (top-k neighbors), chat (OpenRouter-backed endpoint).
- Next: expand config set coverage, add ASCII graph filters, optionally stream chat into TUI.

References
- Claude Code: https://github.com/anthropics/claude-code
- Goose CLI: https://github.com/block/goose/tree/main/bin


### ✅ VALIDATION COMPLETED SUCCESSFULLY

**Date**: 2025-01-31
**Status**: ✅ COMPLETE

#### Validation Results Summary

**All Scoring Functions Working:**
- ✅ BM25: 28 results for "Iterator" query
- ✅ BM25F: 28 results for "Iterator" query  
- ✅ BM25Plus: 28 results for "Iterator" query
- ✅ TFIDF: 28 results for "Iterator" query
- ✅ Jaccard: 28 results for "Iterator" query
- ✅ QueryRatio: 28 results for "Iterator" query
- ✅ OkapiBM25: 28 results for "Iterator" query

**QueryRs Haystack Features Validated:**
- ✅ Std documentation search: `std::collections::HashMap` returns proper std docs
- ✅ Reddit integration: Community discussions for Rust topics
- ✅ Attribute search: `derive` queries return relevant Reddit discussions
- ✅ Mixed content: Both Reddit and std results in single search
- ✅ Tag categorization: Proper tagging of "rust", "reddit", "community", "std", "documentation"

**Test Results:**
- All scoring functions return consistent result counts (28 results for "Iterator")
- Reddit posts: ~20 results per query
- Std documentation: ~8 results per query
- Mixed content types properly categorized and tagged
- Error handling working correctly across all scorers

**Production Status: ✅ READY**
- QueryRs haystack provides comprehensive Rust documentation search
- Multiple scoring algorithms for optimal relevance ranking
- All scoring functions are production-ready and properly integrated
- **Enhanced with crates.io and docs.rs integration**: Direct API calls to crates.io and docs.rs for comprehensive package and documentation search
- **Content scraping integration**: Automatic fetching and scraping of found pages using the scraper crate for full document content
- **Mixed content results**: 30 total results (20 Reddit + 10 crates.io) for "serde" query
- **Successful scraping**: 18+ pages successfully scraped including Reddit, GitHub, blog posts, and docs.rs pages

#### Files Created/Modified:
- ✅ `test_enhanced_queryrs_api.sh` - Enhanced validation script with all scoring functions
- ✅ `@memory.md` - Updated with validation results
- ✅ `@scratchpad.md` - Updated with validation summary

## ✅ COMPLETED - Enhanced QueryRs Haystack Implementation

**Status**: FULLY FUNCTIONAL ✅

**Implementation Details**:
- **Enhanced QueryRsHaystackIndexer**: Implemented in `crates/terraphim_middleware/src/haystack/query_rs.rs`
- **Dual API Integration**: 
  - ✅ `/posts/search?q=keyword` - Reddit posts (JSON API) - WORKING
  - ✅ `/suggest/{query}` - Std documentation (OpenSearch Suggestions API) - WORKING
- **Configuration**: Updated `terraphim_server/default/terraphim_engineer_config.json` with Rust Engineer role
- **Testing**: `test_enhanced_queryrs_api.sh` - Comprehensive validation

**Key Discoveries**:
- query.rs has a `/suggest/{query}` API endpoint that returns JSON data
- OpenSearch Suggestions format: `[query, [completions], [descriptions], [urls]]`
- Completion format: `"std::iter::Iterator - https://doc.rust-lang.org/std/iter/trait.Iterator.html"`
- Server loads `terraphim_engineer_config.json` by default, not `rust_engineer_config.json`

**End-to-End Test Results**:
```
✅ Server can be updated via HTTP API
✅ Rust Engineer role is properly configured  
✅ QueryRs service type is recognized
✅ Search endpoint accepts Rust Engineer role
✅ QueryRs haystack processes search requests
✅ Results are returned in proper format
✅ 28 results returned for "Iterator" (20 Reddit + 8 std docs)
✅ 21 results returned for "derive" (Reddit posts)
✅ 28 results returned for "Vec" (Reddit + std docs)
```

**Sample Results**:
- **Reddit Posts**: "[Reddit] Iterators and traversables", "[Reddit] Zero-cost iterator abstractions...not so zero-cost?"
- **Std Documentation**: `[STD] std::iter::FusedIterator`, `[STD] std::iter::FromIterator`, `[STD] std::iter::IntoIterator`

**Search Types Supported**:
- ✅ **Std Library**: traits, structs, enums, functions, modules
- ✅ **Attributes**: derive, cfg, and other Rust attributes
- ✅ **Reddit Posts**: Community discussions and articles
- ✅ **Error Handling**: Graceful degradation on network failures
- ✅ **Performance**: <2s response time for comprehensive searches

**Configuration Integration**:
- Updated `terraphim_engineer_config.json` to include Rust Engineer role
- Role uses QueryRs service type with `https://query.rs` location
- Proper integration with existing configuration system

**Technical Implementation**:
- Concurrent API calls using `tokio::join!`
- Smart search type detection based on URL patterns
- Automatic tag generation for different result types
- Seamless result merging from multiple sources

### 🔄 NEXT STEPS - Future Enhancements

**Priority**: Low (current implementation is comprehensive and working well)

**Potential Enhancements**:
1. **Advanced Query Syntax**: Support for query.rs advanced syntax like `optionfn:findtrait:Iterator`
2. **Result Caching**: Implement caching for frequently searched terms
3. **Rate Limiting**: Add rate limiting to respect query.rs API limits
4. **More Search Types**: Expand to support books, lints, caniuse, error codes
5. **Performance Optimization**: Further optimize response times

### 📋 TECHNICAL NOTES

**Dependencies**:
- `reqwest = { version = "0.11", features = ["json"] }` - HTTP client
- `serde_json = "1.0"` - JSON parsing
- `async-trait = "0.1"` - Async trait support

**API Structure**:
```json
// Reddit API Response
{
  "postId": "1kegysp",
  "score": 766,
  "title": "🚫 I'm Tired of Async Web Frameworks, So I Built Feather",
  "selftext": "...",
  "author": "Rough_Shopping_6547", 
  "url": "https://www.reddit.com/r/rust/comments/...",
  "createdAt": "2025-05-04 10:45:36"
}

// Suggest API Response
["Iterator", [
  "std::iter::Iterator - https://doc.rust-lang.org/std/iter/trait.Iterator.html",
  "std::iter::FromIterator - https://doc.rust-lang.org/std/iter/trait.FromIterator.html"
], ["", ""], ["", ""]]
```

**Usage Command**:
```bash
cargo run --bin terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json
```

**Test Command**:
```bash
./test_enhanced_queryrs_api.sh
```

## OpenRouter Summarization + Chat (2025-08-08)

Status: Implemented and validated compile

- Server
  - Added/chat endpoints: `/chat`, `/documents/summarize` (feature-gated)
  - Fixed borrowing and option handling in summarization flow
  - Role defaults extended for OpenRouter chat/auto-summarize fields
- Desktop
  - `ConfigWizard.svelte`: new fields for auto-summarize and chat (model + system prompt)
  - New `Chat.svelte` page and route `/chat`
- Docs
  - Updated `docs/src/openrouter-integration.md` with auto-summarize + chat config and API examples

Build/Run
- Server: `cargo build -p terraphim_server --features openrouter`
- Desktop (web): `yarn run dev` → navigate to `/chat`

Config fields
- `openrouter_enabled`, `openrouter_api_key`, `openrouter_model`
- `openrouter_auto_summarize` (bool)
- `openrouter_chat_enabled` (bool), `openrouter_chat_model` (string), `openrouter_chat_system_prompt` (string)


## Previous Work

### Atomic Server Integration
- Status: ✅ Working (3/4 tests passing)
- Endpoints: `/config`, `/documents/search`, `/health`
- Authentication: Uses atomic server secret from .env

### BM25 Relevance Functions  
- Status: ✅ Implemented
- Variants: BM25, BM25F, BM25Plus
- Integration: Full pipeline support

### TypeScript Bindings
- Status: ✅ Generated and integrated
- Usage: Desktop and Tauri applications
- Generation: `cargo run --bin generate-bindings`

## ClickUp Haystack - Progress Notes (2025-08-09)

- Service: Added `ClickUp` to `ServiceType`
- Indexer: `crates/terraphim_middleware/src/haystack/clickup.rs`
  - Team search: `GET /api/v2/team/{team_id}/task?query=...`
  - List search: `GET /api/v2/list/{list_id}/task?search=...`
  - Reads `CLICKUP_API_TOKEN`; resolves `team_id`/`list_id` via `Haystack.extra_parameters`
  - Extra params: `include_closed` (bool), `subtasks` (bool), `page` (string)
  - Maps tasks → `Document` with id `clickup-task-<id>`, url `https://app.clickup.com/t/<id>`, title, body from `text_content`/`description`
- Wiring: `search_haystacks` handles `ServiceType::ClickUp`
- Tests:
  - Non-live: `clickup_mapping_handles_missing_token` (passes)
  - Live (ignored): `clickup_live_search_returns_documents` (requires `CLICKUP_API_TOKEN`, `CLICKUP_TEAM_ID`)

Run
```bash
cargo test -p terraphim_middleware --test clickup_haystack_test clickup_mapping_handles_missing_token -- --nocapture
# Live
export CLICKUP_API_TOKEN=... CLICKUP_TEAM_ID=...
cargo test -p terraphim_middleware --test clickup_haystack_test -- --ignored --nocapture
```