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

## Testing Guidelines
- Keep fast unit tests inline with `mod tests {}`; put multi-crate checks in `tests/` or `test_*.sh`.
- Scope runs with `cargo test -p crate test`; add regression coverage for new failure modes.

## Rust Performance Practices
- Profile first (`cargo bench`, `cargo flamegraph`, `perf`) and land only measured wins.
- Borrow ripgrep tactics: reuse buffers with `with_capacity`, favor iterators, reach for `memchr`/SIMD, and hoist allocations out of loops.
- Apply inline directives sparingly—mark tiny wrappers `#[inline]`, keep cold errors `#[cold]`, and guard cleora-style `rayon::scope` loops with `#[inline(never)]`.
- Prefer zero-copy types (`&[u8]`, `bstr`) and parallelize CPU-bound graph work with `rayon`, feature-gated for graceful fallback.

## Commit & Pull Request Guidelines
- Use Conventional Commit prefixes (`fix:`, `feat:`, `refactor:`) and keep changes scoped.
- Ensure commits pass `cargo fmt`, `cargo clippy`, required `cargo test`, and desktop checks.
- PRs should explain motivation, link issues, list manual verification commands, and attach UI screenshots or logs when behavior shifts.

## Configuration & Security Tips
- Keep secrets in 1Password or `.env`. Use `build-env.sh` or `scripts/` helpers to bootstrap integrations.
- Wrap optional features (`openrouter`, `mcp-rust-sdk`) with graceful fallbacks for network failures.

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

## Important Rules

- **Never use sleep before curl** - Use proper wait mechanisms instead
- **Never use timeout command** - This command doesn't exist on macOS
- **Never use mocks in tests** - Use real implementations or integration tests

## Terraphim Hooks for AI Coding Agents

Terraphim provides hooks to automatically enforce code standards and attribution through knowledge graph-based text replacement.

### Installed Hooks

**PreToolUse Hook (`.claude/hooks/npm_to_bun_guard.sh`)**:
- Intercepts Bash commands containing npm/yarn/pnpm
- Automatically replaces with bun equivalents using knowledge graph
- Knowledge graph files: `docs/src/kg/bun.md`, `docs/src/kg/bun_install.md`

**Pre-LLM Validation Hook (`.claude/hooks/pre-llm-validate.sh`)**:
- Validates input before LLM calls for semantic coherence
- Checks if terms are connected in knowledge graph
- Advisory mode - warns but doesn't block

**Post-LLM Check Hook (`.claude/hooks/post-llm-check.sh`)**:
- Validates LLM outputs against domain checklists
- Checks code changes for tests, docs, error handling, security, performance
- Advisory mode - provides feedback without blocking

**Git prepare-commit-msg Hook (`scripts/hooks/prepare-commit-msg`)**:
- Replaces "Claude Code" and "Claude" with "Terraphim AI" in commit messages
- Optionally extracts concepts from diff (enable with `TERRAPHIM_SMART_COMMIT=1`)
- Knowledge graph files: `docs/src/kg/terraphim_ai.md`, `docs/src/kg/generated_with_terraphim.md`

### Knowledge Graph Validation Commands

```bash
# Validate semantic connectivity
terraphim-agent validate --connectivity "text to check"

# Validate against code review checklist
terraphim-agent validate --checklist code_review "LLM output"

# Validate against security checklist
terraphim-agent validate --checklist security "implementation"

# Get fuzzy suggestions for typos
terraphim-agent suggest --fuzzy "terraphm" --threshold 0.7

# Unified hook handler
terraphim-agent hook --hook-type pre-tool-use --input "$JSON"

# Enable smart commit
TERRAPHIM_SMART_COMMIT=1 git commit -m "message"
```

### Quick Commands

```bash
# Test replacement
echo "npm install" | ./target/release/terraphim-agent replace

# Install all hooks
./scripts/install-terraphim-hooks.sh --easy-mode

# Test hooks
./scripts/test-terraphim-hooks.sh

# Test validation workflow
terraphim-agent validate --connectivity --json "haystack service uses automata"
```

### Extending Knowledge Graph

To add new replacement patterns, create markdown files in `docs/src/kg/`:

```markdown
# replacement_term

Description of what this term represents.

synonyms:: term_to_replace, another_term, third_term
```

The Aho-Corasick automata use LeftmostLongest matching, so longer patterns match first.

## Claude Code Skills Plugin

Terraphim provides a Claude Code skills plugin with specialized capabilities:

**Installation:**
```bash
claude plugin marketplace add terraphim/terraphim-claude-skills
claude plugin install terraphim-engineering-skills@terraphim-ai
```

**Terraphim-Specific Skills:**
- `terraphim-hooks` - Knowledge graph-based text replacement with hooks
- `session-search` - Search AI coding session history with concept enrichment

**Engineering Skills:**
- `architecture`, `implementation`, `testing`, `debugging`
- `rust-development`, `rust-performance`, `code-review`
- `disciplined-research`, `disciplined-design`, `disciplined-implementation`

**Session Search Commands (REPL):**
```bash
/sessions sources       # Detect available sources
/sessions import        # Import from Claude Code, Cursor, Aider
/sessions search "query" # Full-text search
/sessions concepts "term" # Knowledge graph concept search
/sessions related <id>   # Find related sessions
/sessions timeline       # Timeline visualization
```

**Documentation:** See [Claude Code Skills](docs/src/claude-code-skills.md) for full details.

**Repository:** [github.com/terraphim/terraphim-claude-skills](https://github.com/terraphim/terraphim-claude-skills)

## Memory and Task Management

Throughout all user interactions, maintain three key files:
- **memories.md**: Interaction history and project status
- **lessons-learned.md**: Knowledge retention and technical insights
- **scratchpad.md**: Active task management and current work

### Consolidated Agent Instructions

For comprehensive project knowledge, patterns, and best practices, refer to:
- **agents_instructions.json**: Machine-readable consolidated instructions combining all knowledge from memories, lessons learned, and scratchpad
  - Contains project context, status, and active features
  - Critical lessons on deployment patterns, UI development, security, Rust development, and TruthForge
  - Complete architecture overview with all crates and components
  - Development commands and workflows
  - Best practices for Rust, frontend, deployment, testing, and security
  - Common patterns for extending the system
  - Troubleshooting guide and recent achievements
  - Use this as your primary reference for understanding project patterns and established practices

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

## Workspace Structure

The project is organized as a Cargo workspace with multiple components:

- **Core crates**: `crates/*` - Library crates providing specialized functionality (run `ls crates/` for current list)
- **Binaries**:
  - `terraphim_server` - Main HTTP API server (default workspace member)
  - `terraphim_firecracker` - Firecracker microVM integration for secure execution
- **Frontend**: `desktop/src-tauri` - Tauri-based desktop application
- **Excluded**: `crates/terraphim_agent_application` - Experimental crate with incomplete API implementations

The workspace uses Rust edition 2024 and resolver version 2 for optimal dependency resolution.

## Key Architecture Components

### Core System Architecture
- **Rust Backend**: Multi-crate workspace with specialized components
- **Svelte Frontend**: Desktop application with web and Tauri variants
- **Knowledge Graph System**: Custom graph-based semantic search using automata
- **Persistence Layer**: Multi-backend storage (local, Atomic Data, cloud)
- **Search Infrastructure**: Multiple relevance functions (TitleScorer, BM25, TerraphimGraph)
- **Firecracker Integration**: Secure VM-based command execution with sub-2 second boot times

### Critical Crates

**Core Service Layer**:
- `terraphim_service`: Main service layer with search, document management, and AI integration
- `terraphim_middleware`: Haystack indexing, document processing, and search orchestration
- `terraphim_rolegraph`: Knowledge graph implementation with node/edge relationships
- `terraphim_automata`: Text matching, autocomplete, and thesaurus building
- `terraphim_config`: Configuration management and role-based settings
- `terraphim_persistence`: Document storage abstraction layer with cache warm-up
- `terraphim_server`: HTTP API server (main binary)

### Persistence Layer Cache Warm-up

The persistence layer (`terraphim_persistence`) supports transparent cache warm-up for multi-backend configurations:

**Cache Write-back Behavior:**
- When data is loaded from a slower fallback operator (e.g., SQLite, S3), it is automatically cached to the fastest operator (e.g., memory, dashmap)
- Uses fire-and-forget pattern with `tokio::spawn` - non-blocking, doesn't slow load path
- Objects over 1MB are compressed using zstd before caching
- Schema evolution handling: if cached data fails to deserialize, the cache entry is deleted and data is refetched from persistent storage

**Configuration:**
- Operators are ordered by speed (memory > dashmap > sqlite > s3)
- Same-operator detection: skips redundant cache write if only one backend is configured
- Tracing spans for observability: `load_from_operator{key}`, `try_read{profile}`, `cache_writeback{key, size}`

**Testing:**
- Use `DeviceStorage::init_memory_only()` for test isolation (single memory backend)
- Multi-profile cache write-back tested via integration tests in `tests/persistence_warmup.rs`

**Agent System Crates**:
- `terraphim_agent_supervisor`: Agent lifecycle management and supervision
- `terraphim_agent_registry`: Agent discovery and registration
- `terraphim_agent_messaging`: Inter-agent communication infrastructure
- `terraphim_agent_evolution`: Agent learning and adaptation mechanisms
- `terraphim_goal_alignment`: Goal-driven agent orchestration
- `terraphim_task_decomposition`: Breaking complex tasks into subtasks
- `terraphim_multi_agent`: Multi-agent coordination and collaboration
- `terraphim_kg_agents`: Knowledge graph-specific agent implementations
- `terraphim_kg_orchestration`: Knowledge graph workflow orchestration

**Haystack Integration Crates**:
- `haystack_core`: Core haystack abstraction and interfaces
- `haystack_atlassian`: Confluence and Jira integration
- `haystack_discourse`: Discourse forum integration
- `haystack_jmap`: Email integration via JMAP protocol

**Supporting Crates**:
- `terraphim_settings`: Device and server settings
- `terraphim_types`: Shared type definitions
- `terraphim_mcp_server`: MCP server for AI tool integration
- `terraphim_tui`: Terminal UI implementation with REPL
- `terraphim_atomic_client`: Atomic Data integration
- `terraphim_onepassword_cli`: 1Password CLI integration
- `terraphim-markdown-parser`: Markdown parsing utilities
- `terraphim_build_args`: Build-time argument handling

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

# Build with release optimizations
cargo build --release

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

# Build Tauri release
cd desktop
yarn run tauri build

# Build Tauri debug version
cd desktop
yarn run tauri build --debug
```

### TUI Build Options
```bash
# Build with all features (recommended)
cargo build -p terraphim_tui --features repl-full --release

# Run minimal version
cargo run --bin terraphim-agent

# Launch interactive REPL
./target/release/terraphim-agent

# Available REPL commands:
# /help           - Show all commands
# /search "query" - Semantic search
# /chat "message" - AI conversation
# /commands list  - List available markdown commands
# /vm list        - VM management with sub-2s boot
```

### Feature Flags
```bash
# Build with OpenRouter support
cargo build --features openrouter
cargo test --features openrouter

# Build with MCP SDK
cargo build --features mcp-rust-sdk
cargo test --features mcp-rust-sdk

# Build TUI with full REPL
cargo build -p terraphim_tui --features repl-full

# Build terraphim_automata for WASM
cargo build -p terraphim_automata --target wasm32-unknown-unknown --features wasm
```

### WASM Support

The `terraphim_automata` crate supports WebAssembly for browser-based autocomplete functionality.

**Prerequisites:**
```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

**Build WASM module:**
```bash
# Development build
./scripts/build-wasm.sh web dev

# Production build (optimized)
./scripts/build-wasm.sh web release

# Node.js target
./scripts/build-wasm.sh nodejs release
```

**Test WASM module:**
```bash
# Test in Chrome (headless)
./scripts/test-wasm.sh chrome headless

# Test in Firefox
./scripts/test-wasm.sh firefox headless

# Test in Node.js
./scripts/test-wasm.sh node
```

**WASM Features:**
- ✅ Full autocomplete API exposed to JavaScript
- ✅ TypeScript type definitions via `tsify`
- ✅ Browser-compatible random number generation
- ✅ ~200KB compressed bundle size (release build)
- ✅ Compatible with Chrome 57+, Firefox 52+, Safari 11+

**Example WASM directory:**
- `crates/terraphim_automata/wasm-test/` - Complete WASM example with tests
- See `crates/terraphim_automata/wasm-test/README.md` for detailed usage

### Testing
```bash
# Run Rust unit tests
cargo test

# Run all tests in workspace
cargo test --workspace

# Run specific crate tests
cargo test -p terraphim_service

# Run a specific test by name
cargo test test_name

# Run specific test with output visible
cargo test test_name -- --nocapture

# Run tests in a specific file
cargo test --test integration_test_name

# Run ignored/live tests
cargo test test_name -- --ignored

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

### Development Watch Commands
```bash
# Watch and auto-rebuild on changes
cargo watch -x build

# Watch and run tests
cargo watch -x test

# Watch specific package
cargo watch -p terraphim_service -x test

# Watch with clippy
cargo watch -x clippy
```

### Linting and Formatting
```bash
# Format Rust code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Run Rust linter
cargo clippy

# Run clippy with all warnings
cargo clippy -- -W clippy::all

# Frontend linting/formatting
cd desktop
yarn run check
```

### Pre-commit Hooks

Install code quality hooks for automatic formatting, linting, and validation:
```bash
./scripts/install-hooks.sh
```

This sets up:
- Automatic `cargo fmt` on Rust files
- Automatic Biome formatting on JavaScript/TypeScript
- Conventional commit message validation
- Secret detection
- Large file blocking

## Testing Scripts and Automation

### Novel Autocomplete Testing

The project includes comprehensive testing scripts for Novel editor autocomplete integration.
See `TESTING_SCRIPTS_README.md` for full documentation.

Quick start testing scripts:
```bash
# Interactive testing menu
./quick-start-autocomplete.sh

# Start full testing environment
./start-autocomplete-test.sh

# Start only MCP server
./start-autocomplete-test.sh --mcp-only --port 8001

# Stop all services
./stop-autocomplete-test.sh

# Check service status
./stop-autocomplete-test.sh --status
```

Testing scenarios:
```bash
# Full testing environment
./quick-start-autocomplete.sh full

# Development environment (MCP + Desktop, no tests)
./quick-start-autocomplete.sh dev

# Run tests only
./quick-start-autocomplete.sh test

# MCP server only
./quick-start-autocomplete.sh mcp
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
- `RUST_LOG`: Rust-specific logging configuration

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
- **Atlassian**: Confluence and Jira integration
- **Discourse**: Forum integration
- **JMAP**: Email integration
- **Quickwit**: Cloud-native search engine for log and observability data with hybrid index discovery

### Quickwit Haystack Integration

Quickwit provides powerful log and observability data search capabilities for Terraphim AI.

**Key Features:**
- **Hybrid Index Discovery**: Choose explicit (fast) or auto-discovery (convenient) modes
- **Dual Authentication**: Bearer token and Basic Auth support
- **Glob Pattern Filtering**: Filter auto-discovered indexes with patterns
- **Graceful Error Handling**: Network failures never crash searches
- **Production Ready**: Based on try_search deployment at logs.terraphim.cloud

**Configuration Modes:**

1. **Explicit Index (Production - Fast)**
   ```json
   {
     "location": "http://localhost:7280",
     "service": "Quickwit",
     "extra_parameters": {
       "default_index": "workers-logs",
       "max_hits": "100"
     }
   }
   ```
   - Performance: ~100ms (1 API call)
   - Best for: Production monitoring, known indexes

2. **Auto-Discovery (Exploration - Convenient)**
   ```json
   {
     "location": "http://localhost:7280",
     "service": "Quickwit",
     "extra_parameters": {
       "max_hits": "50"
     }
   }
   ```
   - Performance: ~300-500ms (N+1 API calls)
   - Best for: Exploring unfamiliar instances

3. **Filtered Discovery (Balanced)**
   ```json
   {
     "location": "https://logs.terraphim.cloud/api",
     "service": "Quickwit",
     "extra_parameters": {
       "auth_username": "cloudflare",
       "auth_password": "${QUICKWIT_PASSWORD}",
       "index_filter": "workers-*",
       "max_hits": "100"
     }
   }
   ```
   - Performance: ~200-400ms (depends on matches)
   - Best for: Multi-service monitoring with control

**Authentication Examples:**
```bash
# Bearer token
export QUICKWIT_TOKEN="Bearer abc123"

# Basic auth with 1Password
export QUICKWIT_PASSWORD=$(op read "op://Private/Quickwit/password")

# Start agent
terraphim-agent --config quickwit_production_config.json
```

**Query Syntax:**
```bash
# Simple text search
/search error

# Field-specific
/search "level:ERROR"
/search "service:api-server"

# Boolean operators
/search "error AND database"
/search "level:ERROR OR level:WARN"

# Time ranges
/search "timestamp:[2024-01-01 TO 2024-01-31]"

# Combined
/search "level:ERROR AND service:api AND timestamp:[2024-01-13T10:00:00Z TO *]"
```

**Documentation:**
- User Guide: `docs/quickwit-integration.md`
- Example: `examples/quickwit-log-search.md`
- Skill: `skills/quickwit-search/skill.md`
- Configs: `terraphim_server/default/quickwit_*.json`

## Firecracker Integration

The project includes Firecracker microVM support for secure command execution:

- **Location**: `terraphim_firecracker/` binary crate
- **Use case**: Security-first execution with VM sandboxing
- **Execution modes**:
  - **Local**: Direct execution for trusted operations
  - **Firecracker**: Full VM isolation for untrusted code
  - **Hybrid**: Intelligent mode selection based on operation type
- **Performance**:
  - Sub-2 second VM boot times
  - Sub-500ms VM allocation
  - Optimized VM pooling and reuse
- **Features**:
  - Knowledge graph validation before execution
  - Secure web request sandboxing
  - Isolated file operations

## AI Integration

### Supported Providers
- OpenRouter (with feature flag `openrouter`)
- Generic LLM interface for multiple providers
- Ollama support for local models

### AI Features
- Document summarization
- Intelligent descriptions for search results
- Context-aware content processing
- Chat completion with role-based context

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
- Live tests gated by environment variables

## Project Structure

```
terraphim-ai/
├── crates/                          # Core library crates (29 crates)
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
│   ├── terraphim-markdown-parser/  # Markdown parsing utilities
│   ├── terraphim_agent_*/          # Agent system crates (6 crates)
│   ├── terraphim_kg_*/             # Knowledge graph orchestration (2 crates)
│   ├── haystack_*/                 # Haystack integrations (4 crates)
│   └── terraphim_build_args/       # Build-time argument handling
├── terraphim_server/                # Main HTTP server binary
│   ├── default/                    # Default configurations
│   └── fixtures/                   # Test data and examples
├── terraphim_firecracker/           # Firecracker microVM binary
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

**📖 Complete Specification**: See [`docs/specifications/terraphim-desktop-spec.md`](docs/specifications/terraphim-desktop-spec.md) for comprehensive technical documentation including architecture, features, data models, testing, and deployment.

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
- Novel editor with MCP-based autocomplete

### Desktop App Development
```bash
# Install dependencies
cd desktop && yarn install

# Run in development mode
yarn run dev

# Run Tauri desktop app
yarn run tauri dev

# Build release
yarn run tauri build

# Build debug version
yarn run tauri build --debug

# Run tests
yarn test

# Run linting/formatting
yarn run check
```

## Troubleshooting

### Common Issues
- **Config loading fails**: Check file paths and JSON validity
- **Search returns no results**: Verify haystack configuration and indexing
- **Knowledge graph empty**: Ensure thesaurus files exist and are valid
- **Frontend connection issues**: Confirm backend is running on correct port
- **Port already in use**: Use `lsof -i :PORT` to find conflicting process

### Debug Logging
Set `LOG_LEVEL=debug` or `RUST_LOG=debug` for detailed logging across all components.

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
- Firecracker VM pooling reduces startup overhead

## Recent Implementations and Features

> **Note**: This section captures recent significant features. For historical context, refer to `memories.md`, `lessons-learned.md`, and `agents_instructions.json`.

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
- **Atlassian**: Confluence and Jira integration
- **Discourse**: Forum integration
- **JMAP**: Email integration

### LLM Support
- **OpenRouter**: Feature-gated integration with `--features openrouter`
- **Ollama**: Local model support with `llm_provider=ollama` in role config
- **Generic LLM Interface**: Provider-agnostic `LlmClient` trait

### Advanced Features
- **Paragraph Extraction**: Extract paragraphs starting at matched terms
- **Graph Path Connectivity**: Verify if matched terms connect via single path
- **TUI Interface**: Terminal UI with hierarchical commands and ASCII graphs
- **Autocomplete Service**: MCP-based autocomplete for Novel editor
- **Firecracker VMs**: Sub-2 second boot, secure execution sandboxing

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

### Dependency Constraints
Some dependencies are pinned to specific versions to ensure compatibility:
- `wiremock = "0.6.4"` - Version 0.6.5 uses unstable Rust features requiring nightly compiler
- `schemars = "0.8.22"` - Version 1.0+ introduces breaking API changes
- `thiserror = "1.0.x"` - Version 2.0+ requires code migration for breaking changes

These constraints are enforced in `.github/dependabot.yml` to prevent automatic updates that would break CI.

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
      "service": "Ripgrep|AtomicServer|QueryRs|MCP|Quickwit",
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

2. **Install Pre-commit Hooks (Recommended)**
   ```bash
   ./scripts/install-hooks.sh
   ```

3. **Run Backend Server**
   ```bash
   cargo run --release -- --config terraphim_engineer_config.json
   ```

4. **Run Frontend (separate terminal)**
   ```bash
   cd desktop
   yarn install
   yarn dev
   ```

5. **Run with Ollama Support**
   ```bash
   # Start Ollama first
   ollama serve

   # Run with Ollama config
   cargo run --release -- --config ollama_llama_config.json
   ```

6. **Run MCP Server**
   ```bash
   cd crates/terraphim_mcp_server
   ./start_local_dev.sh
   ```

7. **Run TUI Interface**
   ```bash
   cargo build -p terraphim_tui --features repl-full --release
   ./target/release/terraphim-agent
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

````markdown
## UBS Quick Reference for AI Agents

UBS stands for "Ultimate Bug Scanner": **The AI Coding Agent's Secret Weapon: Flagging Likely Bugs for Fixing Early On**

**Install:** `curl -sSL https://raw.githubusercontent.com/Dicklesworthstone/ultimate_bug_scanner/master/install.sh | bash`

**Golden Rule:** `ubs <changed-files>` before every commit. Exit 0 = safe. Exit >0 = fix & re-run.

**Commands:**
```bash
ubs file.ts file2.py                    # Specific files (< 1s) — USE THIS
ubs $(git diff --name-only --cached)    # Staged files — before commit
ubs --only=js,python src/               # Language filter (3-5x faster)
ubs --ci --fail-on-warning .            # CI mode — before PR
ubs --help                              # Full command reference
ubs sessions --entries 1                # Tail the latest install session log
ubs .                                   # Whole project (ignores things like .venv and node_modules automatically)
```

**Output Format:**
```
⚠️  Category (N errors)
    file.ts:42:5 – Issue description
    💡 Suggested fix
Exit code: 1
```
Parse: `file:line:col` → location | 💡 → how to fix | Exit 0/1 → pass/fail

**Fix Workflow:**
1. Read finding → category + fix suggestion
2. Navigate `file:line:col` → view context
3. Verify real issue (not false positive)
4. Fix root cause (not symptom)
5. Re-run `ubs <file>` → exit 0
6. Commit

**Speed Critical:** Scope to changed files. `ubs src/file.ts` (< 1s) vs `ubs .` (30s). Never full scan for small edits.

**Bug Severity:**
- **Critical** (always fix): Null safety, XSS/injection, async/await, memory leaks
- **Important** (production): Type narrowing, division-by-zero, resource leaks
- **Contextual** (judgment): TODO/FIXME, console logs

**Anti-Patterns:**
- ❌ Ignore findings → ✅ Investigate each
- ❌ Full scan per edit → ✅ Scope to file
- ❌ Fix symptom (`if (x) { x.y }`) → ✅ Root cause (`x?.y`)
````
