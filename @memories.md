# Terraphim AI Project Memories

## Project Interaction History

[v1.0.1] Development: Updated @lessons-learned.md with comprehensive TUI implementation insights covering CLI architecture patterns for Rust TUI applications including hierarchical subcommand structure with clap derive API, event-driven architecture with tokio channels and crossterm for terminal input handling, async/sync boundary management using bounded channels to decouple UI rendering from network operations. Documented integration patterns with existing API endpoints through shared client architecture, type reuse strategies from server implementation, and consistent configuration management. Added detailed error handling for network timeouts and feature flags including graceful degradation patterns, runtime feature detection, and progressive timeout strategies. Included ASCII graph visualization techniques using Unicode box-drawing characters, data density optimization for terminal constraints, and interactive navigation capabilities. Covered command structure design with hierarchical organization, argument validation with sensible defaults, and support for both interactive and non-interactive modes. Implementation best practices include cross-platform terminal handling with feature detection, centralized state management patterns, and performance optimization with smart redraw strategies and virtual scrolling for large datasets.

[v1.0.2] Validation: Cross-referenced tracking files for consistency - verified version numbers match across @memories.md, @lessons-learned.md, and @scratchpad.md. All TUI implementation features marked as ✅ COMPLETE with validation status synchronized. QueryRs haystack integration shows 28 results for Iterator queries with proper Reddit and std documentation integration. OpenRouter summarization and chat features validated as implemented and functional across server, desktop, and configuration systems. Task dependencies in scratchpad updated to reflect completion status with proper cross-referencing to memory entries and lessons learned documentation.

[v1.0.3] LLM Abstraction: Introduced provider-agnostic LLM layer (`terraphim_service::llm`) with trait `LlmClient`, OpenRouter + Ollama adapters (feature-gated), and selection via role config `extra` keys. Rewired summarization path to use the abstraction while keeping OpenRouter compatibility. Compiles under default features and `openrouter`; tests build. Desktop Config Wizard exposes generic LLM (Ollama) provider fields.

[v1.0.3.1] E2E Ollama: Added mock and live tests for Ollama. Live test uses role with `llm_provider=ollama`, model `deepseek-coder:latest`, against local instance (`OLLAMA_BASE_URL` or default `http://127.0.0.1:11434`).

[v1.0.3.2] E2E Atomic/OpenRouter: Atomic server reachable at localhost:9883; basic tests pass, some ignored full-flow tests fail with JSON-AD URL error (environment-specific). OpenRouter live test executed with .env key but returned 401 (likely invalid key).

[v1.0.4] MCP Integration: Added `ServiceType::Mcp` and `McpHaystackIndexer` with SSE reachability and HTTP/SSE tool calls. Introduced features `mcp-sse` (default-off) and `mcp-rust-sdk` (optional) with `mcp-client`. Implemented transports: stdio (feature-gated), SSE (localhost with optional OAuth bearer), and HTTP fallback mapping server-everything `search/list` results to `terraphim_types::Document`. Added live test `crates/terraphim_middleware/tests/mcp_haystack_test.rs` (ignored) gated by `MCP_SERVER_URL`.

[v1.0.4.1] MCP SDK: Fixed content parsing using `mcp-spec` (`Content::as_text`, `EmbeddedResource::get_text`) and replaced ad-hoc `reqwest::Error` construction with `Error::Indexation` mapping. `mcp-rust-sdk` feature now compiles green.

[v1.0.5] Automata: Added `extract_paragraphs_from_automata` in `terraphim_automata::matcher` to return paragraph slices starting at matched terms. Includes paragraph end detection and unit test. Documented in `docs/src/automata-paragraph-extraction.md` and linked in SUMMARY.

[v1.0.6] RoleGraph: Added `is_all_terms_connected_by_path` to verify if matched terms in text can be connected by a single path in the graph. Included unit tests, a throughput benchmark, and docs at `docs/src/graph-connectivity.md`.

[v1.0.7] MCP Server Development: Implemented comprehensive MCP server (`terraphim_mcp_server`) exposing all `terraphim_automata` and `terraphim_rolegraph` functions as MCP tools. Added autocomplete functionality with both `autocomplete_terms` and `autocomplete_with_snippets` endpoints. Implemented text matching tools (`find_matches`, `replace_matches`), thesaurus management (`load_thesaurus`, `load_thesaurus_from_json`), and graph connectivity (`is_all_terms_connected_by_path`). Created Novel editor integration with autocomplete service leveraging built-in Novel autocomplete functionality. Replaced RocksDB with non-locking OpenDAL backends (memory, dashmap, sqlite, redb) for local development. Current challenge: MCP server's `tools/list` method not properly routing to `list_tools` function via stdio transport, returning empty responses despite successful protocol handshake.

## Current Project Status (2025-01-31)

### MCP Server Implementation Status
- **Core MCP Tools**: ✅ All `terraphim_automata` functions exposed as MCP tools
- **Autocomplete**: ✅ `autocomplete_terms` and `autocomplete_with_snippets` implemented
- **Text Processing**: ✅ `find_matches`, `replace_matches`, `extract_paragraphs_from_automata`
- **Thesaurus Management**: ✅ `load_thesaurus`, `load_thesaurus_from_json`, `json_decode`
- **Graph Connectivity**: ✅ `is_all_terms_connected_by_path`
- **Database Backend**: ✅ Non-locking OpenDAL profiles replacing RocksDB
- **UI Integration**: ✅ Novel editor autocomplete service implemented

### Current Blocking Issue
- **MCP Protocol Routing**: `tools/list` method not reaching `list_tools` function
- **Debug Evidence**: Debug prints in `list_tools` not appearing in test output
- **Test Results**: Protocol handshake successful, but tools list response empty
- **Investigation Status**: Multiple approaches attempted (manual trait implementation, macro-based approach, signature fixes)

### Project Architecture
- **Backend**: Rust-based MCP server with `rmcp` crate integration
- **Frontend**: Svelte + Novel editor with TypeScript autocomplete service
- **Database**: OpenDAL with multiple non-locking backends
- **Transport**: Both stdio and SSE/HTTP supported
- **Testing**: Comprehensive Rust integration tests for MCP functionality
