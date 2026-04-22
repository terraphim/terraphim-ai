# Summary: terraphim_service/src/lib.rs

**Purpose:** Core business logic and service layer.

**Key Details:**
- Modules: `auto_route`, `llm`, `llm_proxy`, `http_client`, `logging`, `conversation_service`, `rate_limiter`, `summarization_manager`, `summarization_queue`, `summarization_worker`, `error`, `context`
- Feature-gated: `openrouter` module (OpenRouter LLM integration)
- `AutoRouteContext`: automatic role selection based on query content
- `auto_select_role()`: intelligently picks the best role for a query
- Integrates with `terraphim_config::ConfigState` for configuration
- Integrates with `terraphim_middleware` for haystack/thesaurus building
- Integrates with `terraphim_rolegraph` for graph operations
- Error type: `ServiceError` covering middleware, persistence, config errors
- Logging utilities: `terraphim_service::logging::init_logging()`
