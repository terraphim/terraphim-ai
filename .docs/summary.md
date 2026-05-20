# Terraphim AI Project Summary

## Overview

Terraphim AI is a Rust-based AI agent system with modular workspace architecture providing both online (server-connected) and offline (embedded) capabilities for knowledge work, document processing, and AI-assisted tasks through role-based knowledge graph search.

**Current Version:** 1.19.2 (workspace package)

## Architecture

### Workspace Structure

```
terraphim-ai/
├── crates/
│   ├── terraphim_automata/   # FST/Aho-Corasick text matching engine
│   ├── terraphim_config/      # Role-based configuration management
│   ├── terraphim_persistence/ # OpenDAL storage abstraction
│   ├── terraphim_rolegraph/  # Per-role knowledge graph
│   ├── terraphim_service/    # Core service layer
│   ├── terraphim_types/      # Shared type definitions
│   ├── terraphim_agent/      # TUI, robot mode, CLI, forgiving parser
│   ├── terraphim_orchestrator/ # Multi-agent dark factory orchestration
│   ├── terraphim_rlm/        # Recursive Language Model orchestration
│   ├── terraphim_mcp_server/ # Model Context Protocol server
│   ├── terraphim_goal_alignment/ # Goal alignment subsystem
│   ├── terraphim_kg_agents/  # KG-based agent coordination
│   ├── terraphim_task_decomposition/ # Task decomposition
│   └── ... (40+ other crates)
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
┌─────────────────────────────────────────────────────────────┐
│                   terraphim_server (Axum)                   │
│         REST API + WebSocket + Workflow Management          │
└───────────────────────────┬─────────────────────────────────┘
                            │
    ┌───────────────────────┼───────────────────────┐
    ▼                       ▼                       ▼
┌────────────────────┐ ┌────────────────────┐ ┌─────────────────────┐
│ terraphim_service  │ │  terraphim_agent    │ │ terraphim_orchestrator│
│ - Search          │ │  - TUI/REPL        │ │ - Dark Factory     │
│ - Indexing        │ │  - Robot Mode      │ │ - Multi-agent     │
│ - Summarisation   │ │  - MCP Tools       │ │ - Scheduling      │
└───────┬────────────┘ └────────────────────┘ └─────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│                    terraphim_config                         │
│          Role Management + Knowledge Graph Config            │
└───────────────────────────┬─────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                                       ▼
┌────────────────────┐               ┌─────────────────────┐
│ terraphim_rolegraph│               │ terraphim_automata  │
│ - Graph Search    │               │ - FST Index        │
│ - TF-IDF Fallback│               │ - Aho-Corasick     │
│ - Ranking        │               │ - Fuzzy Matching   │
└────────┬─────────┘               └─────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│              terraphim_persistence (OpenDAL)                 │
│    Memory → SQLite → ReDB → S3 (ordered by latency)         │
│              + zstd compression + Cache write-back         │
└─────────────────────────────────────────────────────────────┘
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
- `ProjectConfig`: Project-level overrides in `.terraphim/config.json`

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

### 4. terraphim_rolegraph

**Purpose:** Per-role knowledge graph for semantic document search.

**Architecture:**
```
Thesaurus → TriggerIndex (TF-IDF) → RoleGraph (nodes + edges) → Ranked Results
```

**Key Operations:**
- `query_graph()`: Single-term ranked search
- `query_graph_with_trigger_fallback()`: Two-pass search (AC + TF-IDF)
- `query_graph_with_operators()`: Multi-term AND/OR queries
- `is_all_terms_connected_by_path()`: Path connectivity validation

### 5. terraphim_service

**Purpose:** Integration layer coordinating knowledge graph, thesaurus, and relevance pipeline.

**Core Methods:**
- `ensure_thesaurus_loaded()`: Load/build with cache invalidation
- `preprocess_document_content()`: KG term linking (`terraphim_it` mode)
- `create_document()`: Persist, index, and update haystack files
- `get_document_by_id()`: Retrieve with normalized ID fallback

**KG Preprocessing:**
- Replaces KG terms with `[term](kg:concept)` links
- Filters generic technical terms, max 8 terms per document
- Supports Markdown link format for frontend interception

### 6. terraphim_agent

**Purpose:** TUI, robot mode, CLI, and multi-agent coordination.

**Key Features:**
- Interactive REPL with forgiving CLI parser (typo tolerance)
- Robot mode JSON output for AI agent integration
- MCP tool index for discovering and searching MCP tools
- Onboarding workflows
- Optional shared-learning store

**Feature Flags:** `server`, `repl`, `shared-learning`

### 7. terraphim_orchestrator

**Purpose:** Multi-agent orchestration with scheduling, budgeting, and compound review. Implements the "dark factory" pattern.

**Core Components:**
- `AgentOrchestrator`: Main dark factory orchestrator
- `DualModeOrchestrator`: Real-time and batch processing with fairness scheduling
- `CompoundReviewWorkflow`: Multi-agent review swarm with persona-based specialisation
- `NightwatchMonitor`: Drift detection and rate limiting
- `MetaCoordinator`: Cross-project issue-driven dispatch with PageRank prioritisation

**Key Features:**
- Safety-layer agents spawned immediately on startup
- Layer-based agent classification (Safety, Meta, Review, Implementation)
- Worktree management with automatic cleanup on agent crash
- Concurrency control with per-project agent limits
- KG-boosted exit classification for structured error categorisation
- Provider health tracking with circuit breakers
- Per-PR rate limiting for verdict polling
- Auto-merge deduplication
- TTL-based failure dedupe cache
- Shared learning store integration
- Agent evolution manager for snapshots

### 8. terraphim_rlm

**Purpose:** Recursive Language Model orchestration with isolated code execution.

**Architecture:**
```
TerraphimRlm (public API)
    ├── SessionManager (VM affinity, context, snapshots, extensions)
    ├── QueryLoop (command parsing, execution, result handling)
    ├── BudgetTracker (token counting, time tracking, depth limits)
    └── KnowledgeGraphValidator (term matching, retry, strictness)

ExecutionEnvironment trait
    ├── FirecrackerExecutor (primary, full isolation)
    ├── DockerExecutor (fallback, gVisor/runc)
    └── E2bExecutor (cloud option)
```

**Key Constants:**
- `DEFAULT_TOKEN_BUDGET`: 100K tokens
- `DEFAULT_TIME_BUDGET_MS`: 5 minutes
- `DEFAULT_MAX_RECURSION_DEPTH`: 10
- `VM_ALLOCATION_TIMEOUT_MS`: 500ms
- `TARGET_BOOT_TIME_MS`: 2 seconds

### 9. terraphim_mcp_server

**Purpose:** Model Context Protocol server for AI agent integration.

**Features:**
- MCP tools for terraphim search, document retrieval, KG operations
- Role-based tool filtering
- Integration with terraphim_service for search operations

### 10. terraphim_types

**Purpose:** Core type definitions for the entire Terraphim ecosystem.

**Key Types:**
- Knowledge Graph: `Thesaurus`, `Concept`, `Node`, `Edge`, `NormalizedTerm`
- Document Management: `Document`, `IndexedDocument`, `Index`
- Search: `SearchQuery`, `LogicalOperator`, `Layer`, `RelevanceFunction`
- Conversation: `ConversationId`, `ContextItem`, `ContextType`
- LLM Routing: `RouteDirective`, `RoutingDecision`
- Multi-Agent: `MultiAgentContext`, `AgentInfo`
- Quality: `QualityScore` (K/L/S dimensions), `ReviewFinding`
- Personas: `PersonaDefinition`, `CharacteristicDef`, `SfiaSkillDef`

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
- ConfigWizard, ThemeSwitcher, RoleGraphVisualization

## terraphim_server

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

## Security Considerations

1. **Secret Management**: 1Password CLI integration via `op` commands
2. **TLS**: rustls 0.23+ enforced for RUSTSEC-2026-0049 compliance
3. **Schema Evolution**: Failed deserialization triggers cache eviction
4. **CORS**: Server accepts all origins (configurable in production)
5. **Input Validation**: Regex-based ID normalisation prevents path traversal
6. **DNS Allowlisting**: RLM restricts outbound DNS to pypi.org, github.com, raw.githubusercontent.com
7. **Circuit Breakers**: Provider-level circuit breakers prevent cascading failures

## Testing Strategy

- **Integration tests**: Real services rather than mocks
- **Parallel loading**: JoinSet for concurrent document fetch
- **Feature gates**: Conditional compilation for optional features
- **Error classification**: `is_recoverable()` distinguishes retryable vs fatal errors
- **No mocks**: Tests use actual implementations where possible

## Build & Development

**Commands:**
```bash
cargo build --workspace           # Build all crates
cargo test -p <crate> <test>     # Run single test
cargo fmt && cargo clippy        # Lint
```

**Profiles:**
- `release`: Default optimised build
- `release-lto`: LTO enabled
- `ci`: Fast CI builds with reduced debug info

**Feature Flags:**
- `openrouter`: OpenRouter AI integration
- `mcp-rust-sdk`: MCP server support
- `embedded-assets`: Frontend in binary
- `wasm`: WebAssembly compilation
- `medical`: SNOMED CT/UMLS extraction
- `kg-validation`: Knowledge graph validation in RLM
- `mcp`: MCP tools in RLM
- `quickwit`: Quickwit logging integration
- `typescript`: TypeScript type generation

## Recent Updates (v1.19.x)

- **RLM CLI binary**: New `terraphim_rlm` CLI with MCP integration
- **LLM bridge**: Wired `LlmClient` through `LlmBridge` replacing silent stub
- **Provider health**: Circuit breakers and probe system for model routing
- **Error signatures**: Per-provider stderr classification
- **Evolution system**: Agent snapshot and learning integration
- **Auto-merge**: PR automation with deduplication and failure tracking