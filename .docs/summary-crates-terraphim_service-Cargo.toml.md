# Summary: crates/terraphim_service/Cargo.toml

## Purpose
Core service crate handling user requests, responses, and LLM integration. The business logic layer between API and data.

## Key Details

### Dependencies
- `terraphim_persistence` - Storage abstraction
- `terraphim_config` - Configuration
- `terraphim_middleware` - Middleware (grepapp, AI assistant, MCP)
- `terraphim_types` - Core type definitions
- `terraphim_automata` - Text processing (with `remote-loading`, `tokio-runtime`)
- `terraphim_rolegraph` - Knowledge graph
- `genai` (0.6.0, optional) - Unified multi-provider LLM access
- `terraphim_router` (optional) - LLM routing
- `opendal` (0.54) - Open Data Access Layer
- `reqwest` - HTTP client

### Features
- **default**: `ollama`, `llm_router`, `genai`
- **openrouter**: OpenRouter AI provider
- **ollama**: Ollama local LLM
- **genai**: GenAI crate integration
- **llm_router**: LLM routing with proxy support
- **proxy**: Proxy mode for LLM routing
- **tracing**: Structured logging with tracing

### Dev Dependencies
- `terraphim_settings` - Settings management
- `terraphim_test_utils` - Test utilities
- `serial_test` - Sequential tests
- `tempfile` - Temporary files

### Important
- Contains ranking regression tests (`ranking_regression`) that use Kendall-tau correlation against committed snapshots
- CI enforces Kendall-tau >= 0.95 for ranking changes
- To update snapshots intentionally: `UPDATE_RANKING_SNAPSHOTS=1 cargo test -p terraphim_service ranking_regression`
