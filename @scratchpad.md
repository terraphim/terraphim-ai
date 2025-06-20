# Plan to Fix MCP Server Initialize Hang

Problem
-------
`mcp` client hangs waiting for `initialize` response. Server starts but never answers.

Hypothesis
----------
`rmcp` server expects `McpService` to implement `ServerHandler::open_session` or similar; maybe missing default handshake response registration. The default handler may require `OpenAIExt` trait; Or we might need to wrap `McpService` with `role_server()` function to start session.

Tasks
-----
1. Review `rmcp::ServiceExt::serve` usage; ensure we call `.serve()` on `McpService.role_server()` not directly on service? (Check examples in rust-sdk).
2. Compare with rust-sdk example at [link](https://github.com/modelcontextprotocol/rust-sdk/tree/main/examples).
3. If mismatch, update `main.rs` accordingly, possibly:
   ```rust
   let service = McpService::new(Arc::new(config_state)).role_server();
   let server = service.serve((io::stdin(), io::stdout())).await?;
   ```

## Integration Test Development Status

### ✅ Completed Tasks:
1. **Created comprehensive integration test** at `crates/terraphim_mcp_server/tests/integration_test.rs`
2. **Fixed all compilation errors**:
   - TokioChildProcess API usage
   - String to Cow<str> conversions
   - JSON to Map conversions
   - ResourceContents pattern matching
   - Text content access patterns

3. **Implemented test coverage for**:
   - MCP server connection and initialization
   - Tool listing (`list_tools`)
   - Configuration updates (`update_config_tool`)
   - Search functionality (`search`)
   - Resource listing (`list_resources`)
   - Resource reading (`read_resource`)
   - Error handling for invalid URIs

### ❌ Current Issues:
1. **Search returns 0 results**: All search queries return "Found 0 documents matching your query"
2. **Empty resource list**: `list_resources` returns empty list
3. **Test failure**: `test_search_with_different_roles` fails due to transport closure

### 🔍 Investigation Needed:
1. **Document Indexing**: Check if fixtures are being loaded into search index
2. **Search Service**: Verify search backend is properly initialized
3. **Path Resolution**: Ensure fixture paths are correctly resolved
4. **Configuration**: Check if server config properly points to test data

### 📋 Next Actions:
1. Add debug logging to understand why search returns 0 results
2. Check if documents are being indexed by the search service
3. Verify the search backend initialization
4. Test with simpler search queries
5. Investigate the transport closure issue in role-based tests

### 🐛 Bugs Found and Fixed:
1. **API Usage Errors**: Fixed incorrect MCP client API usage patterns
2. **Type Conversion Issues**: Fixed String/Cow conversions and JSON handling
3. **Pattern Matching Errors**: Fixed ResourceContents enum pattern matching
4. **Text Content Access**: Fixed RawTextContent field access

### 📊 Test Results:
- `test_mcp_server_integration`: ✅ PASS
- `test_resource_uri_mapping`: ✅ PASS  
- `test_search_with_different_roles`: ❌ FAIL (transport closure)

# Current Task: Debug Document Indexing Issue

## Problem Statement
- Search consistently returns 0 results despite having test fixtures
- Ripgrep CLI works and finds matches in fixture files
- Need to understand why the indexer isn't finding or processing documents

## Investigation Plan

### 1. Add Logging to RipgrepIndexer
- Add debug logging to `RipgrepIndexer::index` method
- Log the haystack path being searched
- Log the search term being used
- Log the number of ripgrep messages received

### 2. Add Logging to index_inner Function
- Log when documents are being processed
- Log document creation and insertion
- Log any errors during file reading
- Track the final index size

### 3. Switch to docs/src Haystack
- Update test configuration to use `docs/src` instead of fixtures
- `docs/src` contains more comprehensive documentation
- Should provide better test data for search functionality

### 4. Monitor Log Output
- Run tests with logging enabled
- Check if files are being found by ripgrep
- Verify documents are being created and indexed
- Identify where the indexing process might be failing

## Implementation Steps
1. Add logging to `crates/terraphim_middleware/src/indexer/ripgrep.rs`
2. Update test configuration to use `docs/src` haystack
3. Run tests and analyze log output
4. Fix any issues identified in the indexing process 

### Implemented
- Test spawns `target/debug/terraphim_mcp_server` instead of `cargo run`.
- Added `scripts/run_mcp_tests.sh` to rebuild & run integration tests with env vars.

### Next
- Re-run integration tests; expect RipgrepIndexer logs.
- If still 0 docs, inspect logs.
- Then implement list_resources & read_resource validation. 

## 2025-06-20 – Plan: Richer Integration Tests

### New Tests To Implement
1. **Pagination Happy-Path**
   - Search with `limit = 2` should return at most 2 resources + text heading.
   - Subsequent call with `skip = 2` should not repeat first batch.

2. **Pagination Error Cases**
   - Negative `skip` or `limit` → expect `is_error: true`.
   - Excessive `limit` (>1000) → expect error.

3. **Round-Trip Resource Retrieval**
   - Run `search` for term that yields >0 docs.
   - Extract first resource URI.
   - Call `read_resource`; assert body equals content embedded in search response.

4. **Concurrent Clients**
   - Use `tokio::join!` to spawn three clients:
     * Client A: constant search queries.
     * Client B: updates config every second.
     * Client C: lists resources randomly.
   - Assert none of them error within 5-second window.

5. **Timeout / Cancellation**
   - Launch a `search` with impossible regex; cancel after 1s using `tokio::time::timeout`. Ensure cancellation propagated (server closes call, not transport).

### Implementation Steps
- Create new test file `tests/integration_pagination.rs` for pagination cases.
- Extend existing helper utilities (e.g., `spawn_server`) into shared `mod util` inside `tests/` directory.
- Use `tokio::select!` pattern for concurrent test.
- Add helper `get_first_resource_text()` for round-trip validation.

### Estimate
Pagination & round-trip: ~60 LOC
Error cases: +40 LOC
Concurrency/timeout: ~120 LOC

### Acceptance
`cargo test -p terraphim_mcp_server -- --nocapture` passes with all new tests.

### 2025-06-20 – Fix: Role-aware query terms
- **Problem**: Search for roles Engineer/System Operator returned 0 docs with generic query "terraphim".
- **Investigation**: Examined docs/src and thesaurus JSON → found role-specific synonym terms.
- **Solution**: Updated `integration_test.rs` mapping `role_queries`:
  ```rust
  let role_queries = vec![
      ("Default", "terraphim"),
      ("Engineer", "graph embeddings"),
      ("System Operator", "service"),
  ];
  ```
- Re-ran `cargo test -p terraphim_mcp_server --test integration_test` => **7/7 tests PASS**. 