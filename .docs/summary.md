# Terraphim AI Project Summary

## Overview

Terraphim AI is a Rust-based AI agent system with a modular workspace architecture. It provides both online (server-connected) and offline (embedded) capabilities for knowledge work, document processing, and AI-assisted tasks through role-based knowledge graph search.

## Architecture

### Workspace Structure

The project is a Cargo workspace with multiple member crates:

```
terraphim-ai/
├── crates/
│   ├── terraphim_automata/   # Text matching & autocomplete engine
│   ├── terraphim_config/      # Configuration management
│   ├── terraphim_persistence/ # Storage abstraction layer
│   ├── terraphim_rolegraph/  # Knowledge graph implementation
│   ├── terraphim_service/    # Core service layer
│   ├── terraphim_types/      # Shared type definitions
│   ├── terraphim_agent/       # TUI, robot mode, CLI
│   └── ... (50+ other crates)
├── terraphim_server/         # Axum HTTP server
├── terraphim_firecracker/    # Firecracker VM management
├── terraphim_ai_nodejs/      # Node.js bindings
└── desktop/                  # Tauri Svelte desktop app
```

### Data Flow

```
User Query
    │
    ▼
┌─────────────────────────────────────────────────────┐
│              terraphim_server (Axum)               │
│  REST API + WebSocket + Workflow Management         │
└─────────────────────┬───────────────────────────────┘
                      │
    ┌─────────────────┴─────────────────┐
    ▼                                   ▼
┌────────────────────┐         ┌─────────────────────┐
│ terraphim_service  │         │  Workflow Engine    │
│ - Search           │         │  - Parallel Exec    │
│ - Indexing         │         │  - Prompt Chain     │
│ - Summarisation    │         │  - Orchestration    │
└───────┬────────────┘         └─────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────┐
│              terraphim_config (ConfigState)          │
│  Role Management + Knowledge Graph Configuration     │
└─────────────────────┬───────────────────────────────┘
                      │
        ┌─────────────┴─────────────┐
        ▼                           ▼
┌───────────────────┐     ┌───────────────────┐
│ terraphim_rolegraph│     │   terraphim_automata │
│ - Graph Search     │     │   - Text Matching   │
│ - TF-IDF Fallback │     │   - Autocomplete    │
│ - Ranking         │     │   - FST Index       │
└─────────┬─────────┘     └───────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────┐
│           terraphim_persistence (DeviceStorage)      │
│  OpenDAL Operators (Memory, SQLite, ReDB, S3)        │
│  + zstd compression + Cache write-back              │
└─────────────────────────────────────────────────────┘
```

## Core Components

### 1. terraphim_automata

**Purpose:** Fast text matching using Aho-Corasick automata and finite state transducers (FST).

**Key Features:**
- FST-based autocomplete with fuzzy matching (Levenshtein/Jaro-Winkler)
- Aho-Corasick multi-pattern matching (LeftmostLongest)
- Link generation (Markdown, HTML, Wiki formats)
- Paragraph extraction around matched terms
- WASM/TypeScript support for browser integration

**Key Exports:**
- `build_autocomplete_index`, `fuzzy_autocomplete_search`
- `load_thesaurus`, `replace_matches`
- `LinkType` enum for output format selection

### 2. terraphim_config

**Purpose:** Role-based configuration management with knowledge graph orchestration.

**Key Types:**
- `Config`: Top-level config with roles, global shortcut, default/selected role
- `Role`: User profile with haystacks, relevance function, theme, LLM settings
- `Haystack`: Data source descriptor with service type
- `KnowledgeGraph`: Automata path and/or local KG files

**Service Types Supported:**
- Ripgrep, Atomic Server, QueryRs, ClickUp, MCP, Perplexity, GrepApp, AiAssistant, Quickwit, JMAP

**LLM Configuration:**
- OpenRouter, Ollama providers
- Chat interface with context management
- Intelligent LLM routing (6-phase architecture)

### 3. terraphim_persistence

**Purpose:** Unified storage abstraction over OpenDAL operators with cache hierarchy.

**Architecture:**
- Operators ordered by latency (slowest to fastest)
- Transparent write-back to fastest operator via fire-and-forget tokio::spawn
- Objects >1MB compressed with zstd
- Schema evolution detection with cache invalidation

**Storage Backends:**
- Memory (DashMap), SQLite, ReDB, S3

**Key Methods:**
- `load_from_operator()`: Speed-ordered fallback loading
- `save_to_all()`: Broadcast to all profiles
- `load_documents_by_ids()`: Parallel loading via JoinSet

### 4. terraphim_rolegraph

**Purpose:** Per-role knowledge graph for semantic document search.

**Architecture:**
```
Thesaurus → TriggerIndex (TF-IDF) → RoleGraph (nodes + edges) → Ranked Results
```

**Key Types:**
- `RoleGraph`: Main graph with nodes, edges, documents, thesaurus
- `TriggerIndex`: TF-IDF fallback when Aho-Corasick finds no matches
- `SerializableRoleGraph`: JSON-serializable representation
- `RoleGraphSync`: Thread-safe wrapper (Arc<Mutex<RoleGraph>>)

**Search Operations:**
- `query_graph()`: Single-term ranked search
- `query_graph_with_trigger_fallback()`: Two-pass search (AC + TF-IDF)
- `query_graph_with_operators()`: Multi-term AND/OR queries
- `is_all_terms_connected_by_path()`: Path connectivity validation

**Ranking:**
- Weighted mean of node rank + edge rank + document rank
- Descending sort with offset/limit pagination

### 5. terraphim_service

**Purpose:** Integration layer coordinating knowledge graph, thesaurus, and relevance pipeline.

**Core Methods:**
- `ensure_thesaurus_loaded()`: Load/build with cache invalidation
- `preprocess_document_content()`: KG term linking (`terraphim_it` mode)
- `create_document()`: Persist and index new documents
- `get_document_by_id()`: Retrieve with normalized ID fallback

**KG Preprocessing:**
- Replaces KG terms with `[term](kg:concept)` links
- Filters generic technical terms, max 8 terms per document
- Supports Markdown link format for frontend interception

**Document Pipeline:**
1. Persist via fastest operator
2. Index into all role graphs
3. Write back to Markdown files (ripgrep haystacks)
4. Apply KG preprocessing if enabled

### 6. terraphim_server

**Purpose:** Axum-based HTTP server with REST API and workflow management.

**API Routes (40+ endpoints):**
- `/documents/*`: Create, search, summarise documents
- `/chat`: Chat completion
- `/config/*`: Configuration management
- `/rolegraph`, `/thesaurus/*`, `/autocomplete/*`: Knowledge graph access
- `/conversations/*`: Conversation management
- `/workflows/*`: Workflow execution
- `/summarization/*`: Async summarisation queue

**Startup Process:**
1. Load configuration
2. Build thesaurus from local KG files (Logseq builder)
3. Index KG markdown files as documents
4. Process haystack directories recursively
5. Initialize summarisation manager
6. Initialize workflow management with WebSocket broadcaster

**Deployment:**
- Embedded frontend via `rust-embed` (feature-gated)
- Static file serving for SPA routing
- CORS enabled for all origins

## Desktop Application

**Technology Stack:**
- Tauri 2.0 for desktop integration
- Svelte with TypeScript
- Bulma CSS framework
- Vite build tool, Yarn package manager

**Components:**
- `Search/`: KGSearchModal, KGSearchInput, ResultItem, TermChip, ArticleModal
- `Chat/`: Chat interface, SessionList, ContextEditModal
- `Editor/`: NovelWrapper for document editing
- `Fetchers/`: Role and tab data fetching
- ConfigWizard, ThemeSwitcher, RoleGraphVisualization

**Features:**
- Multi-role support with role switching
- KG-powered search with autocomplete
- Document editing with Novel editor
- Chat interface with conversation history
- Configuration wizard for setup

## Security Considerations

1. **Secret Management**: 1Password CLI integration via `op` commands
2. **TLS**: rustls 0.23+ enforced for RUSTSEC-2026-0049 compliance
3. **Schema Evolution**: Failed deserialization triggers cache eviction
4. **CORS**: Server accepts all origins (configurable in production)
5. **Input Validation**: Regex-based ID normalization prevents path traversal

## Testing Strategy

- **Integration tests**: Real services rather than mocks
- **Parallel loading**: JoinSet for concurrent document fetch
- **Feature gates**: Conditional compilation for optional features
- **Error classification**: `is_recoverable()` distinguishes retryable vs fatal errors

## Build & Development

**Commands:**
```bash
cargo build --workspace           # Build all crates
cargo test -p <crate> <test>      # Run single test
cargo fmt && cargo clippy          # Lint
```

**Profiles:**
- `release`: Default optimized build
- `release-lto`: LTO enabled
- `ci`: Fast CI builds with reduced debug info

**Feature Flags:**
- `openrouter`: OpenRouter AI integration
- `mcp-rust-sdk`: MCP server support
- `embedded-assets`: Frontend in binary
- `wasm`: WebAssembly compilation
- `medical`: SNOMED CT/UMLS extraction