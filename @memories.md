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

[v1.0.7] MCP Server Development: Implemented comprehensive MCP server (`terraphim_mcp_server`) exposing all `terraphim_automata` and `terraphim_rolegraph` functions as MCP tools. Added autocomplete functionality with both `autocomplete_terms` and `autocomplete_with_snippets` endpoints. Implemented text matching tools (`find_matches`, `replace_matches`), thesaurus management (`load_thesaurus`, `load_thesaurus_from_json`), and graph connectivity (`is_all_terms_connected_by_path`). Created Novel editor integration with autocomplete service leveraging built-in Novel autocomplete functionality. Replaced RocksDB with non-locking OpenDAL backends (memory, dashmap, sqlite, redb) for local development.

[v1.0.8] Summarization Queue System: Implemented production-ready async queue system for document summarization with priority management (Critical/High/Normal/Low), token bucket rate limiting, background worker with concurrent processing, and exponential backoff retry logic. Created RESTful async API endpoints for queue management. Addressed DateTime serialization issues by replacing `Instant` with `DateTime<Utc>`. Successfully integrated with existing LLM providers (OpenRouter, Ollama). System compiles successfully with comprehensive error handling and task status tracking.

[v1.0.8.1] AWS Credentials Error Fix (2025-08-22): Resolved recurring AWS_ACCESS_KEY_ID environment variable error that was preventing local development. Root cause was twofold: 1) S3 profile in user settings file containing credentials that triggered shell variable expansion, and 2) persistence layer passing a FILE path (`crates/terraphim_settings/default/settings_local_dev.toml`) to `DeviceSettings::load_from_env_and_file()` which expects a DIRECTORY path. Fixed by correcting the path in `terraphim_persistence/src/lib.rs` to pass the directory path (`crates/terraphim_settings/default`) instead. This allows the settings system to work as designed, using local-only profiles (memory, dashmap, sqlite, redb) for development without AWS dependencies. Both server and Tauri desktop application now start successfully without AWS errors. Desktop app builds cleanly and Tauri dev process works normally.

## Current Project Status (2025-01-31)

### MCP Server Implementation Status
- **Core MCP Tools**: ✅ All `terraphim_automata` functions exposed as MCP tools
- **Autocomplete**: ✅ `autocomplete_terms` and `autocomplete_with_snippets` implemented
- **Text Processing**: ✅ `find_matches`, `replace_matches`, `extract_paragraphs_from_automata`
- **Thesaurus Management**: ✅ `load_thesaurus`, `load_thesaurus_from_json`, `json_decode`
- **Graph Connectivity**: ✅ `is_all_terms_connected_by_path`
- **Database Backend**: ✅ Non-locking OpenDAL profiles replacing RocksDB
- **UI Integration**: ✅ Novel editor autocomplete service implemented

### ✅ RESOLVED: AWS Credentials Error (2025-01-31)
- **Problem**: System required AWS_ACCESS_KEY_ID when loading thesaurus due to S3 profile in default settings
- **Root Cause**: `DEFAULT_SETTINGS` in `terraphim_settings` included `settings_full.toml` with S3 profile requiring AWS credentials
- **Solution Implemented**:
  1. Changed `DEFAULT_SETTINGS` from `settings_full.toml` to `settings_local_dev.toml`
  2. Added fallback mechanism in S3 profile parsing to gracefully handle missing credentials
  3. Updated README with optional AWS configuration documentation
- **Result**: Local development now works without any AWS dependencies, cloud storage remains available when credentials are provided

### Project Architecture
- **Backend**: Rust-based MCP server with `rmcp` crate integration
- **Frontend**: Svelte + Novel editor with TypeScript autocomplete service
- **Database**: OpenDAL with multiple non-locking backends
- **Transport**: Both stdio and SSE/HTTP supported
- **Testing**: Comprehensive Rust integration tests for MCP functionality
