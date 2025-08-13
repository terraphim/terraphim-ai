# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
- **Ripgrep**: Local filesystem search
- **AtomicServer**: Integration with Atomic Data
- **ClickUp**: Task management integration
- **Logseq**: Personal knowledge management

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

## MCP (Model Context Protocol) Integration

The system includes MCP server functionality in `crates/terraphim_mcp_server/` for integration with AI development tools.

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