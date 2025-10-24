# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

You are an expert in Rust, async programming, and concurrent systems and WASM.

Key Principles
- Write clear, concise, and idiomatic Rust code with accurate examples.
- Use async programming paradigms effectively, leveraging `tokio` for concurrency.
- Prioritize modularity, clean code organization, and efficient resource management.
- Use expressive variable names that convey intent (e.g., `is_ready`, `has_data`).
- Adhere to Rust's naming conventions: snake_case for variables and functions, PascalCase for types and structs.
- Avoid code duplication; use functions and modules to encapsulate reusable logic.
- Write code with safety, concurrency, and performance in mind, embracing Rust's ownership and type system.

Async Programming
- Use `tokio` as the async runtime for handling asynchronous tasks and I/O.
- Implement async functions using `async fn` syntax.
- Leverage `tokio::spawn` for task spawning and concurrency.
- Use `tokio::select!` for managing multiple async tasks and cancellations.
- Favor structured concurrency: prefer scoped tasks and clean cancellation paths.
- Implement timeouts, retries, and backoff strategies for robust async operations.

Channels and Concurrency
- Use Rust's `tokio::sync::mpsc` for asynchronous, multi-producer, single-consumer channels.
- Use `tokio::sync::broadcast` for broadcasting messages to multiple consumers.
- Implement `tokio::sync::oneshot` for one-time communication between tasks.
- Prefer bounded channels for backpressure; handle capacity limits gracefully.
- Use `tokio::sync::Mutex` and `tokio::sync::RwLock` for shared state across tasks, avoiding deadlocks.

Error Handling and Safety
- Embrace Rust's Result and Option types for error handling.
- Use `?` operator to propagate errors in async functions.
- Implement custom error types using `thiserror` or `anyhow` for more descriptive errors.
- Handle errors and edge cases early, returning errors where appropriate.
- Use `.await` responsibly, ensuring safe points for context switching.

Testing
- Write unit tests with `tokio::test` for async tests.
- Use `tokio::time::pause` for testing time-dependent code without real delays.
- Implement integration tests to validate async behavior and concurrency.
- Never use mocks in tests.

Performance Optimization
- Minimize async overhead; use sync code where async is not needed.
- Use non-blocking operations and atomic data types for concurrency.
- Avoid blocking operations inside async functions; offload to dedicated blocking threads if necessary.
- Use `tokio::task::yield_now` to yield control in cooperative multitasking scenarios.
- Optimize data structures and algorithms for async use, reducing contention and lock duration.
- Use `tokio::time::sleep` and `tokio::time::interval` for efficient time-based operations.

Key Conventions
1. Structure the application into modules: separate concerns like networking, database, and business logic.
2. Use environment variables for configuration management (e.g., std crate).
3. Ensure code is well-documented with inline comments and Rustdoc.

Async Ecosystem
- Use `tokio` for async runtime and task management.
- Leverage `hyper` or `reqwest` for async HTTP requests.
- Use `serde` for serialization/deserialization.
- Use `sqlx` or `tokio-postgres` for async database interactions.
- Utilize `tonic` for gRPC with async support.
- use [salvo](https://salvo.rs/book/) for async web server and axum

## Memory and Task Management

Throughout all user interactions, maintain three key files:
- **memories.md**: Interaction history and project status
- **lessons-learned.md**: Knowledge retention and technical insights
- **scratchpad.md**: Active task management and current work

### Documentation Organization

All project documentation is organized in the `.docs/` folder:
- **Individual File Summaries**: `.docs/summary-<normalized-path>.md` - Detailed summaries of each working file
- **Comprehensive Overview**: `.docs/summary.md` - Consolidated project overview and architecture analysis
- **Agent Instructions**: `.docs/agents_instructions.json` - Machine-readable agent configuration and workflows

### Consolidated Agent Instructions

For comprehensive project knowledge, patterns, and best practices, refer to:
- **.docs/agents_instructions.json**: Machine-readable consolidated instructions combining all knowledge from memories, lessons learned, and scratchpad
  - Contains project context, status, and active features
  - Critical lessons on deployment patterns, UI development, security, Rust development, and TruthForge
  - Complete architecture overview with all crates and components
  - Development commands and workflows
  - Best practices for Rust, frontend, deployment, testing, and security
  - Common patterns for extending the system
  - Troubleshooting guide and recent achievements
  - Use this as your primary reference for understanding project patterns and established practices

### Documentation Organization

All project documentation is organized in the `.docs/` folder:
- **Individual File Summaries**: `.docs/summary-<normalized-path>.md` - Detailed summaries of each working file
- **Comprehensive Overview**: `.docs/summary.md` - Consolidated project overview and architecture analysis
- **Agent Instructions**: `.docs/agents_instructions.json` - Machine-readable agent configuration and workflows

### Mandatory /init Command Steps

When user executes `/init` command, you MUST perform these two steps in order:

#### Step 1: Summarize Working Files
Can you summarize the working files? Save each file's summary in `.docs/summary-<normalized-path>.md`

- Identify all relevant working files in the project
- Create individual summaries for each file
- Save summaries using the pattern: `.docs/summary-<normalized-path>.md`
- Include file purpose, key functionality, and important details
- Normalize file paths (replace slashes with hyphens, remove special characters)

#### Step 2: Create Comprehensive Summary
Can you summarize your context files ".docs/summary-*.md" and save the result in `.docs/summary.md`
- Read all individual summary files created in Step 1
- Synthesize into a comprehensive project overview
- Include architecture, security, testing, and business value analysis
- Save the consolidated summary as `.docs/summary.md`
- Update any relevant documentation references

Both steps are MANDATORY for every `/init` command execution.

## Agent Systems Integration

**Two Agent Systems Available**:

### 1. Superpowers Skills (External, Mandatory Workflows)
- **Location**: `~/.config/superpowers/skills/`
- **Use for**: Process workflows like brainstorming, systematic debugging, TDD, code review
- **Mandatory workflows**:
  - **Brainstorming before coding**: MUST run /brainstorm or read brainstorming skill before implementation
  - **Check skills before ANY task**: Use find-skills to search for relevant skills, read with Read tool if found
  - **Historical context search**: Dispatch subagent to search past conversations when needed
- **Trigger**: Automatically loaded via session-start hook
- **Pattern**: Skills with checklists require TodoWrite for each item

### 2. Terraphim .agents (Project-Specific Automation)
- **Location**: `.agents/` directory
- **Use for**: Project-specific automation tasks (git commits, code review, file exploration)
- **Technology**: TypeScript + CodeBuff framework
- **Available tools**: read_files, write_file, str_replace, find_files, code_search, run_terminal_command, spawn_agents, web_search, read_docs, think_deeply
- **Trigger**: Manual via `codebuff --agent agent-name`
- **Pattern**: Define agents in TypeScript with handleSteps() for multi-step workflows

**Integration Hierarchy**:
- **Skills workflows apply to all work**: Brainstorming, TDD, systematic debugging are mandatory processes
- **Terraphim .agents for automation**: Use for repetitive tasks specific to this project
- **When both apply**: Follow Skills workflows (like brainstorming) THEN use .agents for implementation

## Best Practices and Development Workflow

### Pre-commit Quality Assurance
- **Pre-commit Hook Integration**: All commits must pass pre-commit checks including format, lint, and compilation
- **Struct Evolution Management**: When adding fields to existing structs, update all initialization sites systematically
- **Feature Gate Handling**: Use `#[cfg(feature = "openrouter")]` for optional fields with proper imports (ahash::AHashMap)
- **Trait Object Compliance**: Always use `dyn` keyword for trait objects (e.g., `Arc<dyn StateManager>`)
- **Import Hygiene**: Remove unused imports regularly to prevent maintenance burden
- **Error Resolution Process**: Group similar compilation errors (E0063, E0782) and fix in batches
- **Clean Commits**: Commit only relevant changes with clear technical descriptions, avoid unnecessary attribution

### Async Programming Patterns
- Separate UI rendering from network operations using bounded channels
- Use `tokio::select!` for managing multiple async tasks
- Implement async/sync boundaries with proper channel communication
- Prefer structured concurrency with scoped tasks

### Error Handling Strategy
- Return empty results instead of errors for network failures
- Log warnings for debugging but maintain graceful degradation
- Implement progressive timeout strategies (quick for health checks, longer for searches)
- Use `Result<T, E>` propagation with fallback UI states

### Testing Philosophy
- Unit tests for individual components with `tokio::test`
- Integration tests for cross-crate functionality
- Live tests gated by environment variables
- End-to-end validation with actual service calls

### Configuration Management
- Role-based configuration with sensible defaults
- Feature flags for optional functionality
- Environment variable overrides for deployment flexibility
- JSON for role configs, TOML for system settings


## Project Overview

Terraphim AI is a privacy-first AI assistant that operates locally, providing semantic search across multiple knowledge repositories (personal, team, and public sources). The system uses knowledge graphs, semantic embeddings, and various search algorithms to deliver relevant results.

## Key Architecture Components

### Core System Architecture
- **Rust Backend**: Multi-crate workspace with specialized components
- **Svelte Frontend**: Desktop application with web and Tauri variants
- **Knowledge Graph System**: Custom graph-based semantic search using automata
- **Persistence Layer**: Multi-backend storage (local, Atomic Data, cloud)
- **Search Infrastructure**: Multiple relevance functions (TitleScorer, BM25, TerraphimGraph)

### Critical Crates
- `terraphim_service`: Main service layer with search, document management, and AI integration
- `terraphim_middleware`: Haystack indexing, document processing, and search orchestration
- `terraphim_rolegraph`: Knowledge graph implementation with node/edge relationships
- `terraphim_automata`: Text matching, autocomplete, and thesaurus building
- `terraphim_config`: Configuration management and role-based settings
- `terraphim_persistence`: Document storage abstraction layer
- `terraphim_server`: HTTP API server (main binary)

### Key Concepts
- **Roles**: User profiles with specific knowledge domains and search preferences
- **Haystacks**: Data sources (local folders, Notion, email, etc.)
- **Knowledge Graph**: Structured relationships between concepts and documents
- **Thesaurus**: Concept mappings and synonyms for semantic matching
- **Rolegraph**: Per-role knowledge graph for personalized search

## Development Commands

### Build and Run
```bash
# Build all components
cargo build

# Run the backend server
cargo run

# Run with specific config
cargo run -- --config terraphim_engineer_config.json

# Run desktop frontend (requires backend running)
cd desktop
yarn install
yarn run dev

# Run Tauri desktop app
cd desktop
yarn run tauri dev
```

### Testing
```bash
# Run Rust unit tests
cargo test

# Run all tests in workspace
cargo test --workspace

# Run specific crate tests
cargo test -p terraphim_service

# Run frontend tests
cd desktop
yarn test

# Run end-to-end tests
cd desktop
yarn run e2e

# Run atomic server integration tests
cd desktop
yarn run test:atomic
```

### Linting and Formatting
```bash
# Format Rust code
cargo fmt

# Run Rust linter
cargo clippy

# Frontend linting/formatting
cd desktop
yarn run check
```

## Configuration System

The system uses role-based configuration with multiple backends:

### Config Loading Priority
1. `terraphim_server/default/terraphim_engineer_config.json`
2. Saved config from persistence layer
3. Default server configuration

### Key Config Files
- `terraphim_engineer_config.json`: Main engineering role
- `system_operator_config.json`: System administration role
- `settings.toml`: Device and server settings

### Environment Variables
- `TERRAPHIM_CONFIG`: Override config file path
- `TERRAPHIM_DATA_DIR`: Data directory location
- `LOG_LEVEL`: Logging verbosity

## Search and Knowledge Graph System

### Relevance Functions
- **TitleScorer**: Basic text matching and ranking
- **BM25/BM25F/BM25Plus**: Advanced text relevance algorithms
- **TerraphimGraph**: Semantic graph-based ranking with thesaurus

### Knowledge Graph Workflow
1. Thesaurus building from documents or URLs
2. Automata construction for fast text matching
3. Document indexing with concept extraction
4. Graph construction with nodes/edges/documents
5. Query processing with semantic expansion

### Haystack Types
- **Ripgrep**: Local filesystem search using `ripgrep` command
- **AtomicServer**: Integration with Atomic Data protocol
- **ClickUp**: Task management with API token authentication
- **Logseq**: Personal knowledge management with markdown parsing
- **QueryRs**: Rust documentation and Reddit community search
- **MCP**: Model Context Protocol for AI tool integration

## AI Integration

### Supported Providers
- OpenRouter (with feature flag `openrouter`)
- Generic LLM interface for multiple providers
- Ollama support for local models

### AI Features
- Document summarization
- Intelligent descriptions for search results
- Context-aware content processing

## Common Development Patterns

### Adding New Search Providers
1. Implement indexer in `terraphim_middleware/src/indexer/`
2. Add configuration in `terraphim_config`
3. Integrate with search orchestration in `terraphim_service`

### Adding New Relevance Functions
1. Implement scorer in `terraphim_service/src/score/`
2. Update `RelevanceFunction` enum in `terraphim_types`
3. Add handling in main search logic

### Working with Knowledge Graphs
- Thesaurus files use specific JSON format with id/nterm/url structure
- Automata are built from thesaurus for efficient matching
- Use `terraphim_automata::load_thesaurus()` for loading
- RoleGraph manages document-to-concept relationships

### Testing Strategy
- Unit tests for individual components
- Integration tests for cross-crate functionality
- E2E tests for full user workflows
- Atomic server tests for external integrations

## Project Structure

```
terraphim-ai/
├── crates/                          # Core library crates
│   ├── terraphim_automata/         # Text matching, autocomplete, thesaurus
│   ├── terraphim_config/           # Configuration management
│   ├── terraphim_middleware/       # Haystack indexing and search orchestration
│   ├── terraphim_persistence/      # Storage abstraction layer
│   ├── terraphim_rolegraph/        # Knowledge graph implementation
│   ├── terraphim_service/          # Main service layer with AI integration
│   ├── terraphim_settings/         # Device and server settings
│   ├── terraphim_types/            # Shared type definitions
│   ├── terraphim_mcp_server/       # MCP server for AI tool integration
│   ├── terraphim_tui/              # Terminal UI implementation
│   ├── terraphim_atomic_client/    # Atomic Data integration
│   ├── terraphim_onepassword_cli/  # 1Password CLI integration
│   └── terraphim-markdown-parser/  # Markdown parsing utilities
├── terraphim_server/                # Main HTTP server binary
│   ├── default/                    # Default configurations
│   └── fixtures/                   # Test data and examples
├── desktop/                         # Svelte frontend application
│   ├── src/                        # Frontend source code
│   ├── src-tauri/                  # Tauri desktop integration
│   └── public/                     # Static assets
├── lab/                            # Experimental code and prototypes
└── docs/                           # Documentation

Key Configuration Files:
- Cargo.toml                        # Workspace configuration
- terraphim_server/default/*.json   # Role configurations
- desktop/package.json              # Frontend dependencies
- crates/terraphim_settings/default/*.toml  # System settings
```

## MCP (Model Context Protocol) Integration

The system includes comprehensive MCP server functionality in `crates/terraphim_mcp_server/` for integration with AI development tools. The MCP server exposes all `terraphim_automata` and `terraphim_rolegraph` functions as MCP tools:

### MCP Tools Available
- **Autocomplete**: `autocomplete_terms`, `autocomplete_with_snippets`
- **Text Processing**: `find_matches`, `replace_matches`, `extract_paragraphs_from_automata`
- **Thesaurus Management**: `load_thesaurus`, `load_thesaurus_from_json`
- **Graph Connectivity**: `is_all_terms_connected_by_path`
- **Fuzzy Search**: `fuzzy_autocomplete_search_jaro_winkler`

### MCP Transport Support
- **stdio**: For local development and testing
- **SSE/HTTP**: For production deployments
- **OAuth**: Optional bearer token authentication

## Desktop Application

### Frontend Architecture
- Svelte with TypeScript
- Vite for build tooling
- Tauri for native desktop integration
- Bulma CSS framework with custom theming

### Key Frontend Features
- Real-time search interface
- Knowledge graph visualization
- Configuration management UI
- Role switching and management

## Troubleshooting

### Common Issues
- **Config loading fails**: Check file paths and JSON validity
- **Search returns no results**: Verify haystack configuration and indexing
- **Knowledge graph empty**: Ensure thesaurus files exist and are valid
- **Frontend connection issues**: Confirm backend is running on correct port

### Debug Logging
Set `LOG_LEVEL=debug` for detailed logging across all components.

### Port Configuration
Default server runs on dynamically assigned port. Check logs for actual port or configure in settings.

## Performance Considerations

- Knowledge graph construction can be expensive - cache automata when possible
- Large thesaurus files may require memory tuning
- Search performance varies significantly by relevance function chosen
- Consider haystack size limits for responsive search
- Use concurrent API calls with `tokio::join!` for parallel operations
- Implement bounded channels for backpressure in async operations
- Virtual scrolling for large datasets in UI components

## Recent Implementations and Features

### CI/CD Infrastructure (2025-01-31)
- **Hybrid GitHub Actions**: Complete migration from Earthly to GitHub Actions + Docker Buildx
- **Multi-Platform Builds**: Support for linux/amd64, linux/arm64, linux/arm/v7 with cross-compilation
- **Docker Layer Optimization**: Efficient layer caching with builder.Dockerfile for faster builds
- **Matrix Configuration**: Fixed GitHub Actions matrix incompatibility issues
- **Local Testing**: Comprehensive nektos/act integration for local workflow validation
- **Workflow Variants**: Multiple approaches (native, Earthly hybrid, optimized) for different use cases

### Haystack Integrations
- **QueryRs**: Reddit API + Rust std documentation search with smart type detection
- **MCP**: Model Context Protocol with SSE/HTTP transports
- **ClickUp**: Task management with list/team search
- **Atomic Server**: Integration with Atomic Data protocol

### LLM Support
- **OpenRouter**: Feature-gated integration with `--features openrouter`
- **Ollama**: Local model support with `llm_provider=ollama` in role config
- **Generic LLM Interface**: Provider-agnostic `LlmClient` trait

### Advanced Features
- **Paragraph Extraction**: Extract paragraphs starting at matched terms
- **Graph Path Connectivity**: Verify if matched terms connect via single path
- **TUI Interface**: Terminal UI with hierarchical commands and ASCII graphs
- **Autocomplete Service**: MCP-based autocomplete for Novel editor

## Testing and Validation

### Test Commands
```bash
# Run specific integration tests
cargo test -p terraphim_service --test ollama_llama_integration_test
cargo test -p terraphim_middleware --test query_rs_haystack_test
cargo test -p terraphim_mcp_server --test test_tools_list

# Run with features
cargo test --features openrouter
cargo test --features mcp-rust-sdk

# Live tests (require services running)
MCP_SERVER_URL=http://localhost:3001 cargo test mcp_haystack_test -- --ignored
OLLAMA_BASE_URL=http://127.0.0.1:11434 cargo test ollama_live_test -- --ignored

# CI/CD Testing and Validation
./scripts/validate-all-ci.sh              # Comprehensive CI validation (15/15 tests)
./scripts/test-matrix-fixes.sh ci-native  # Matrix-specific testing
./scripts/validate-builds.sh              # Build consistency validation
act -W .github/workflows/ci-native.yml -j setup -n  # Local workflow testing
```

### Configuration Examples
```json
// Role with Ollama configuration
{
  "name": "Llama Engineer",
  "extra": {
    "llm_provider": "ollama",
    "ollama_base_url": "http://127.0.0.1:11434",
    "ollama_model": "llama3.2:3b"
  }
}
```

## Known Issues and Workarounds

### MCP Protocol
- `tools/list` routing issue - debug with `--nocapture` flag
- Use stdio transport for development, SSE for production

### Database Backends
- RocksDB can cause locking issues - use OpenDAL alternatives
- Preferred backends: memory, dashmap, sqlite, redb

### API Integration
- QueryRs `/suggest/{query}` returns OpenSearch format
- Reddit API ~500ms, Suggest API ~300ms response times
- Implement graceful degradation for network failures

## Critical Implementation Details

### Thesaurus Format
```json
{
  "id": "unique_id",
  "nterm": "normalized_term",
  "url": "https://example.com/resource"
}
```

### Document Structure
- **id**: Unique identifier
- **url**: Resource location
- **body**: Full text content
- **description**: Summary or excerpt
- **tags**: Classification labels
- **rank**: Optional relevance score

### Role Configuration Structure
```json
{
  "name": "Role Name",
  "relevance_function": "BM25|TitleScorer|TerraphimGraph",
  "theme": "UI theme name",
  "extra": {
    "llm_provider": "ollama|openrouter",
    "custom_settings": {}
  },
  "haystacks": [
    {
      "name": "Haystack Name",
      "service": "Ripgrep|AtomicServer|QueryRs|MCP",
      "extra_parameters": {}
    }
  ]
}
```

### API Endpoints
- `GET /health` - Server health check
- `POST /config` - Update configuration
- `POST /documents/search` - Search documents
- `POST /documents/summarize` - AI summarization
- `POST /chat` - Chat completion
- `GET /config` - Get current configuration
- `GET /roles` - List available roles

## Quick Start Guide

1. **Clone and Build**
   ```bash
   git clone https://github.com/terraphim/terraphim-ai
   cd terraphim-ai
   cargo build --release
   ```

2. **Run Backend Server**
   ```bash
   cargo run --release -- --config terraphim_engineer_config.json
   ```

3. **Run Frontend (separate terminal)**
   ```bash
   cd desktop
   yarn install
   yarn dev
   ```

4. **Run with Ollama Support**
   ```bash
   # Start Ollama first
   ollama serve

   # Run with Ollama config
   cargo run --release -- --config ollama_llama_config.json
   ```

5. **Run MCP Server**
   ```bash
   cd crates/terraphim_mcp_server
   ./start_local_dev.sh
   ```

## Frontend Technology Guidelines

**Svelte Desktop Application**:
- **Use for**: `desktop/` directory - Main Terraphim AI desktop application
- **Technology**: Svelte + TypeScript + Tauri + Vite
- **CSS Framework**: Bulma (no Tailwind)
- **Purpose**: Full-featured desktop app with real-time search, knowledge graph visualization, configuration UI

**Vanilla JavaScript Examples**:
- **Use for**: `examples/agent-workflows/` - Demonstration and testing workflows
- **Technology**: Vanilla JavaScript, HTML, CSS (no frameworks)
- **Pattern**: No build step, static files only
- **Purpose**: Simple, deployable examples that work without compilation
