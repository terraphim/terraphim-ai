# Terraphim Crates Overview

Terraphim is built as a modular Rust project with multiple crates, each serving specific purposes in the knowledge graph and document management system.

## Core Crates

### terraphim_types
**Purpose**: Central type definitions and shared data structures
**Key Features**:
- `Document` struct for document representation
- `RoleName` and `Role` for role-based access control
- `SearchQuery` for search operations
- `NormalizedTermValue` for knowledge graph terms
- `Thesaurus` for synonym management
- `Index` and `IndexedDocument` for document indexing

**Dependencies**: Minimal, serves as foundation for other crates

### terraphim_config
**Purpose**: Configuration management and role-based settings
**Key Features**:
- Role-based configuration system
- Knowledge graph path management
- Relevance function selection
- Atomic server integration settings
- Environment-based configuration loading

**Dependencies**: terraphim_types

### terraphim_persistence
**Purpose**: Document persistence and storage abstraction
**Key Features**:
- Opendal-based storage abstraction
- Document save/load operations
- Memory-only storage for testing
- Atomic Data document integration
- Cross-platform storage support

**Dependencies**: opendal, terraphim_types

## Service Layer

### terraphim_service
**Purpose**: Main service layer for document search and ranking
**Key Features**:
- Document search and ranking algorithms
- BM25, TFIDF, Jaccard, and similarity-based scoring
- Role-based search configuration
- Knowledge graph integration
- OpenRouter AI integration
- Document preprocessing and enhancement

**Dependencies**: terraphim_types, terraphim_config, terraphim_persistence

### terraphim_middleware
**Purpose**: Middleware for document processing and indexing
**Key Features**:
- Haystack document indexing
- Atomic Data integration
- Document preprocessing
- Knowledge graph term extraction
- Role-based document filtering

**Dependencies**: terraphim_types, terraphim_config

## Knowledge Graph & Automata

### terraphim_automata
**Purpose**: Finite state automata for term matching and autocomplete
**Key Features**:
- Aho-Corasick algorithm implementation
- Fuzzy autocomplete with Jaro-Winkler similarity
- Term matching and replacement
- WASM-compatible design
- High-performance text processing

**Dependencies**: Minimal, focused on text processing

### terraphim_rolegraph
**Purpose**: Knowledge graph construction and management
**Key Features**:
- Graph-based term relationships
- Node and edge management
- Rank-based scoring
- Graph traversal algorithms
- Knowledge graph serialization

**Dependencies**: terraphim_types

## Client & Integration

### terraphim_atomic_client
**Purpose**: Atomic Data protocol client
**Key Features**:
- Atomic Data document operations
- Authentication and authorization
- Resource management
- HTTP client implementation
- WASM compatibility

**Dependencies**: reqwest, serde, terraphim_types

### terraphim_mcp_server
**Purpose**: Model Context Protocol server
**Key Features**:
- MCP protocol implementation
- Resource mapping
- Desktop integration
- Cross-platform compatibility
- Protocol version management

**Dependencies**: serde, tokio

## Session Management

### terraphim_sessions
**Purpose**: AI coding assistant session history management
**Key Features**:
- Multi-source session import (Claude Code, Cursor, Aider, OpenCode)
- Session caching and search
- Knowledge graph concept enrichment
- Related session discovery
- Timeline visualization
- Export to JSON/Markdown

**Feature Flags**:
- `claude-log-analyzer` - Enhanced Claude Code parsing via CLA
- `cla-full` - CLA with Cursor connector support
- `enrichment` - Knowledge graph concept matching
- `full` - All features enabled

**Dependencies**: tokio, serde, terraphim_automata (optional)

### claude-log-analyzer
**Purpose**: Parse and analyze Claude Code session logs
**Key Features**:
- JSONL session log parsing from `~/.claude/projects/`
- Agent type detection and attribution
- File operation tracking
- Timeline visualization
- Export to JSON, CSV, Markdown
- Real-time session monitoring
- Knowledge graph integration (optional)

**Connectors**:
- `cursor` - Cursor IDE sessions
- `aider` - Aider chat history
- `opencode` - OpenCode sessions
- `codex` - Codex sessions

**Dependencies**: serde_json, regex, home

### terraphim_hooks
**Purpose**: Unified hook infrastructure for AI coding agents
**Key Features**:
- ReplacementService for knowledge graph-based text transformation
- HookResult struct for structured JSON output
- Binary discovery utilities
- Fail-open error handling
- Support for Claude Code PreToolUse and Git hooks

**Usage**:
- CLI: `terraphim-agent replace` command
- MCP: `replace_matches` tool
- Hooks: npm→bun, Claude→Terraphim attribution

**Dependencies**: terraphim_automata, terraphim_types, serde

## Build & Configuration

### terraphim_build_args
**Purpose**: Build-time argument processing
**Key Features**:
- Environment variable handling
- Feature flag management
- Build configuration
- Cross-platform build support

**Dependencies**: Minimal build-time utilities

### terraphim_settings
**Purpose**: Application settings management
**Key Features**:
- TOML-based configuration
- Default settings
- Environment-specific configurations
- Settings validation

**Dependencies**: toml, serde

## Node.js Integration

### terraphim_ai_nodejs
**Purpose**: Node.js bindings for Terraphim
**Key Features**:
- Rust-to-Node.js bindings
- WASM compilation
- Cross-platform support
- TypeScript definitions
- Performance optimization

**Dependencies**: napi-rs, terraphim_service

## Desktop Application

### Desktop (Tauri)
**Purpose**: Cross-platform desktop application
**Key Features**:
- Svelte-based UI
- Tauri framework integration
- Native system integration
- Cross-platform deployment
- Real-time search interface

**Dependencies**: Tauri, Svelte, terraphim_service

## Architecture Patterns

### Modular Design
Each crate has a specific responsibility and minimal dependencies:
- **Core types** in `terraphim_types`
- **Service logic** in `terraphim_service`
- **Storage** in `terraphim_persistence`
- **UI** in desktop application

### Async-First
Most crates use async/await patterns:
- `tokio` runtime for concurrency
- Async I/O operations
- Non-blocking document processing
- Concurrent search operations

### WASM Compatibility
Key crates support WebAssembly:
- `terraphim_automata` for client-side processing
- `terraphim_service` for shared logic
- `terraphim_atomic_client` for browser integration

### Cross-Platform
Designed for multiple platforms:
- Desktop (Windows, macOS, Linux)
- Web browsers (WASM)
- Node.js environments
- Server deployments

## Development Workflow

### Adding New Features
1. **Types**: Start with `terraphim_types` for new data structures
2. **Core Logic**: Implement in appropriate service crate
3. **Integration**: Add to `terraphim_service` if needed
4. **UI**: Update desktop application
5. **Documentation**: Update this overview

### Testing Strategy
- **Unit tests**: In each crate
- **Integration tests**: In `terraphim_service`
- **E2E tests**: In desktop application
- **Performance tests**: Benchmarks in relevant crates

### Performance Considerations
- **Memory efficiency**: Shared types and minimal allocations
- **Async operations**: Non-blocking I/O throughout
- **Caching**: Document and configuration caching
- **Indexing**: Efficient document indexing and search
