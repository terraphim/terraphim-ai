# Summary: terraphim_service/src/lib.rs

## Purpose

Main service layer for Terraphim AI providing document search, indexing, and AI-assisted summarisation across multiple haystack backends. Integrates knowledge graph, thesaurus, and relevance-scoring pipeline.

## Core Methods

### TerraphimService
- `new(config_state)`: Create service with ConfigState
- `ensure_thesaurus_loaded()`: Load/build thesaurus with cache invalidation
- `preprocess_document_content()`: KG term linking (terraphim_it mode)
- `create_document()`: Persist, index, and update haystack files
- `get_document_by_id()`: Retrieve with normalized ID fallback

## Document Pipeline

1. Persist document via fastest operator
2. Index into all role graphs
3. Write back to Markdown files (ripgrep haystacks)
4. Apply KG preprocessing if enabled

## KG Preprocessing (terraphim_it)

- Replaces KG terms with `[term](kg:concept)` links
- Filters generic technical terms (40+ excluded terms)
- Prioritizes important KG terms (graph, haystack, service, terraphim, etc.)
- Max 8 terms per document
- Supports Markdown link format for frontend interception

## Key Modules

- **auto_route**: `AutoRouteContext`, `auto_select_role`, `AutoRouteReason`, `AutoRouteResult`
- **llm**: Generic LLM layer for multiple providers
- **llm_proxy**: Unified provider management
- **http_client**: Centralized HTTP client creation
- **logging**: Standardized logging utilities
- **conversation_service**: Chat conversation management
- **summarization_queue/manager/worker**: Async summarisation pipeline
- **rate_limiter**: Service rate limiting
- **context**: LLM conversation context management
- **error**: Centralized error handling with `ServiceError` enum

## Error Handling

```rust
pub enum ServiceError {
    Middleware(terraphim_middleware::Error),
    OpenDal(Box<opendal::Error>),
    Persistence(terraphim_persistence::Error),
    Config(String),
    #[cfg(feature = "openrouter")]
    OpenRouter(crate::openrouter::OpenRouterError),
    Common(crate::error::CommonError),
}
```

Implements `TerraphimError` trait with `category()` and `is_recoverable()`.

## Cache Invalidation

Thesaurus cache stale detection via `compute_kg_source_hash()`:
- Compares cached source hash vs current KG source
- Automatically rebuilds when hash changes
- Handles optional files gracefully (DEBUG level for file-not-found)