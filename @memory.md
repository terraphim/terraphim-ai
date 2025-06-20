# Terraphim MCP Server Learnings

- Running `./run_mcp_e2e_tests.sh` shows `mcp` client hangs waiting for `initialize` response.
- Server logs indicate it starts correctly, creates roles, and logs "Initialized Terraphim MCP service", so startup finishes.
- The hang is during MCP handshake, not remote thesaurus fetch (remote URL resolves quickly).
- Need to investigate why `rmcp` server doesn't send `initialize` response; may require explicit handler or use of `ServiceExt::serve` API.

## Current Task: Expand Integration Test for Resource Search

- Created basic integration test at `crates/terraphim_mcp_server/tests/integration_test.rs`
- Test currently covers: tool listing, search tool, and config update tool
- Need to expand test to include:
  - `list_resources` functionality
  - `read_resource` functionality
  - Search with role filtering and pagination
  - Error handling for invalid resource URIs

## Integration Test Status (Updated)

### ✅ Fixed Issues:
1. **Compilation Errors**: Fixed multiple compilation errors in the integration test:
   - Removed incorrect `.await` from `TokioChildProcess::new()`
   - Fixed `String` to `Cow<str>` conversion using `.into()`
   - Fixed `json!` to `Map` conversion using `.as_object().cloned()`
   - Fixed `ResourceContents` pattern matching (used `blob` instead of `data`)
   - Fixed text content access using `.text` field from `RawTextContent`

2. **API Usage**: Corrected the MCP client API usage:
   - Used `().serve(transport).await?` instead of `transport.connect().await?`
   - Used `service.peer_info()` instead of `service.initialize().await?`
   - Used `Default::default()` for pagination parameters

### ✅ Working Features:
1. **Server Connection**: Tests successfully connect to the MCP server
2. **Tool Listing**: `list_tools` works correctly and returns expected tools
3. **Configuration Update**: `update_config_tool` successfully updates server configuration
4. **Basic Search**: Search tool responds without errors (though returns 0 results)
5. **Resource Listing**: `list_resources` works but returns empty list
6. **Error Handling**: Invalid resource URI correctly returns error

### ❌ Remaining Issues:
1. **Search Returns No Results**: All search queries return "Found 0 documents matching your query"
2. **Empty Resources**: `list_resources` returns empty list, suggesting documents aren't being indexed
3. **Test Failure**: `test_search_with_different_roles` fails due to transport closure

### 🔍 Root Cause Analysis:
The issue appears to be that the server configuration points to fixtures, but the documents aren't being indexed or searched properly. This could be due to:
- Documents not being loaded into the search index
- Search service not properly configured
- Path resolution issues with fixtures
- Missing initialization of the search backend

### 📋 Next Steps:
1. **Add logging to RipgrepIndexer** to see if files are being found and indexed
2. **Switch haystack to docs/src** for better test data
3. **Investigate why search returns 0 results** despite having fixtures
4. **Check if documents are being indexed properly**
5. **Verify the search service configuration**
6. **Add more comprehensive test data and search scenarios**
7. **Fix the transport closure issue in role-based search test**

## Current Investigation: Document Indexing Issue

### Problem:
- Search consistently returns "Found 0 documents matching your query"
- Ripgrep CLI works and finds matches in fixture files
- Server configuration points to correct haystack directory
- Need to add logging to understand why indexer isn't finding documents

### Plan:
1. Add debug logging to `RipgrepIndexer::index` method
2. Add logging to `index_inner` function to track document processing
3. Switch test configuration to use `docs/src` as haystack
4. Monitor log output to see if files are being found and processed

### 🛠 Fixes Implemented
8. Switched test server launch to run built binary directly (avoids nested cargo stdio closure).
9. Added `scripts/run_mcp_tests.sh` for convenient build + test with backtrace & logging.

## ✅ Logging Integration & Test Stability (2025-06-20)
- Integrated `tracing-log` bridge; server logs now routed through `tracing` without polluting stdout.
- Replaced `println!` with `log::*` across runtime crates; MCP JSON-RPC stream stable.
- Adjusted subscriber setup with `try_init` to avoid double-init panic in tests.
- All 4 integration tests now PASS consistently.

## ➡️ Next Focus: Richer Integration Test Coverage
- Verify pagination (`skip`/`limit`) behaviour of `search` tool.
- Add negative tests: malformed JSON input to `update_config_tool`, invalid pagination params.
- Validate `list_resources` pagination and MIME types.
- Round-trip test: `search` → pick resource URI → `read_resource` returns identical text.
- Concurrency test: spawn 3 parallel clients performing searches/config updates.
- Timeout/cancellation: ensure long-running search (regex with no matches) can be cancelled.

### 2025-06-20 – Role-Specific Search Queries
- Updated integration tests to use per-role queries that map to each role's thesaurus/markdown content:
  • Default: "terraphim"  
  • Engineer: "graph embeddings"  
  • System Operator: "service"
- All 7 integration tests pass; each role search call now yields ≥1 document (or at least non-zero content) and no longer returns empty result sets.