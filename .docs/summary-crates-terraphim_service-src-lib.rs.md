# Summary: terraphim_service/src/lib.rs

**Purpose:** Main service layer integrating knowledge graph, thesaurus, and relevance-scoring pipeline for document search, indexing, and AI-assisted summarisation.

**Key Components:**
- **`TerraphimService`**: Main service facade coordinating config, graphs, and storage
- **`auto_route`**: Automatic role selection and context routing
- **`llm`**: Generic LLM layer for multiple providers (OpenRouter, Ollama)
- **`summarization_*`**: Async document summarisation queue system
- **`proxy_client`**: LLM proxy service for unified provider management
- **`http_client`**: Centralized HTTP client creation and configuration

**Core Methods:**
- `ensure_thesaurus_loaded()`: Load/build thesaurus with cache invalidation
- `preprocess_document_content()`: KG term linking with `terraphim_it` mode
- `preprocess_document_content_with_search()`: KG linking + search term highlighting
- `create_document()`: Persist and index new documents
- `get_document_by_id()`: Retrieve with normalized ID fallback

**KG Preprocessing (`terraphim_it`):**
- Replaces KG terms in document body with `[term](kg:concept)` links
- Filters terms: excludes generic technical terms, prioritises important KG concepts
- Max 8 KG terms per document to avoid over-linking
- Supports Markdown link format intercepted by frontend

**Document Processing Pipeline:**
1. Load/persist document via fastest operator
2. Index into all role graphs
3. Write back to on-disk Markdown files for writable ripgrep haystacks
4. Apply KG preprocessing if enabled

**LLM Integration:**
- OpenRouter support (feature-gated)
- Ollama support for local inference
- Async summarisation with queue system
- Chat completion with context management

**Auto-Route Context:**
- `AutoRouteContext`: Role selection based on conversation context
- `JMAP_MISSING_TOKEN_PENALTY`: Penalty score for missing JMAP credentials
- `auto_select_role()`: Intelligent role selection algorithm

**Error Handling:**
- `ServiceError` enum with category classification
- `is_recoverable()`: Distinguishes retryable vs fatal errors
- Integrates middleware, OpenDAL, persistence, config errors

**Haystack Indexing:**
- RipgrepIndexer for local filesystem
- Supports recursive directory traversal
- Atomic Server integration for structured data
- MCP client for Model Context Protocol servers