# Summary: terraphim_config/src/lib.rs

## Purpose

Role-based configuration management with knowledge graph orchestration. Provides `Config`, `Role`, `Haystack`, and `KnowledgeGraph` types with support for multiple service backends and LLM integration.

## Key Types

### Config
- `ConfigId`: Server, Desktop, or Embedded deployment mode
- `Config`: Top-level config with roles, global shortcut, default/selected role
- `Role`: User profile with haystacks, relevance function, theme, LLM settings
- `Haystack`: Data source descriptor with service type
- `KnowledgeGraph`: Automata path and/or local KG files

### Role Configuration
- `shortname`, `name`, `relevance_function`, `theme`
- `kg`: Optional knowledge graph (automata_path + knowledge_graph_local)
- `haystacks`: Vec of data sources
- LLM settings: `llm_enabled`, `llm_api_key`, `llm_model`, `llm_auto_summarize`, `llm_chat_enabled`
- `llm_router_enabled`: Intelligent LLM routing

### ServiceType (enum)
- Ripgrep, Atomic, QueryRs, ClickUp, Mcp, Perplexity, GrepApp, AiAssistant, Quickwit, Jmap

### Path Expansion
- `expand_path()`: Supports `~`, `$HOME`, `${VAR:-default}` syntax
- Project-level config discovery in `.terraphim/config.json`

### ConfigState
- Holds Terraphim Config and RoleGraphs
- Creates RoleGraph for each role using TerraphimGraph relevance function
- Extracts triggers/pinned directives from KG markdown files

### ConfigBuilder
- `build_default_embedded()`: WASM/library mode with Default, Terraphim Engineer, Rust Engineer roles
- `build_default_server()`: Server mode with Default, Engineer, System Operator roles
- `build_default_desktop()`: Desktop mode with Default, Terraphim Engineer, Rust Engineer roles
- `add_role()`, `default_role()`, `merge_with()` for configuration composition

### Persistence
- Implements `Persistable` trait for async save/load
- Uses fastest operator for storage
- Schema evolution detection

## Key Features

1. **Role-based search**: Different relevance functions per role
2. **Multi-haystack**: Multiple data sources per role
3. **LLM integration**: Chat, summarization, intelligent routing
4. **KG preprocessing**: Trigger extraction, pinned entries, source hashing
5. **Project overrides**: Per-project config in `.terraphim/config.json`