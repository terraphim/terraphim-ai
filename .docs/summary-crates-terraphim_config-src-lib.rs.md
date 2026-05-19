# Summary: terraphim_config/src/lib.rs

**Purpose:** Configuration management for Terraphim AI with role-based settings and knowledge graph orchestration.

**Loading Priority:**
1. Explicit path via `TERRAPHIM_CONFIG` environment variable
2. Saved config retrieved from persistence layer
3. Hard-coded defaults in `terraphim_server/default/`

**Project-Level Config Discovery:**
- Searches parent directories for `.terraphim/` directory starting from current working directory
- Enables project-specific roles, global shortcuts, and configuration overrides
- Merged with global config via `Config::with_project()` and `Config::merge_with()`
- Roles merged by `RoleName`: project role fully replaces global role (no deep merge)
- `global_shortcut` field is optional in project configs

**Key Types:**
- **`Config`**: Top-level configuration holding all roles, global shortcut, default/selected role
- **`Role`**: User profile with haystacks, relevance function, theme, LLM settings
- **`Haystack`**: Data source descriptor (path, service type, extra parameters)
- **`ServiceType`**: Enum of supported haystack backends
- **`KnowledgeGraph`**: Automata path and/or local KG files for indexing

**Supported Service Types:**
- `Ripgrep`: Local filesystem search
- `Atomic`: Atomic Server integration
- `QueryRs`: query.rs API
- `ClickUp`: ClickUp API
- `Mcp`: Model Context Protocol client
- `Perplexity`: AI-powered web search
- `GrepApp`: GitHub code search
- `AiAssistant`: AI coding assistant session logs
- `Quickwit`: Log/observability search
- `Jmap`: Email search (RFC 8620/8621)

**LLM Configuration:**
- `llm_enabled`: Enable AI-powered article summaries
- `llm_api_key`, `llm_model`: Provider credentials
- `llm_auto_summarize`: Automatic summarization of search results
- `llm_chat_enabled`: Chat interface backed by LLM
- `llm_router_enabled`: Intelligent LLM routing with 6-phase architecture

**ConfigState:**
- Wraps Config with Arc<Mutex<Config>> for thread-safe access
- Maintains AHashMap<RoleName, RoleGraphSync> for per-role knowledge graphs
- Provides async methods: `new()`, `get_default_role()`, `get_role()`, `search_indexed_documents()`

**Path Expansion:**
- Supports `${HOME}`, `$HOME`, `${VAR:-default}` syntax
- Expands `~` at start of paths
- Uses dirs crate with fallback strategies

**RoleGraph Integration:**
- Extracts triggers and pinned directives from KG markdown files
- Builds RoleGraph from automata path or local KG files
- Fallback chain: automata_path -> local KG -> empty thesaurus

**Persistence:**
- Implements `Persistable` trait for async save/load
- Saves to all profiles via fire-and-forget tokio::spawn
- Schema evolution detection with cache invalidation