# Summary: CLAUDE.md

## Purpose
Provides comprehensive guidance to Claude Code (claude.ai/code) when working with the Terraphim AI codebase. Serves as the primary reference for AI assistants to understand project structure, conventions, and development workflows.

## Key Functionality
- **Rust & Async Programming Guidelines**: Tokio-based async patterns, channels, concurrency best practices
- **Project Architecture**: 29-crate workspace structure with specialized components for search, knowledge graphs, and AI integration
- **Development Commands**: Complete build, test, and deployment workflows including TUI, desktop app, and server
- **Testing Infrastructure**: Comprehensive testing strategy with unit, integration, and live tests; includes specialized testing scripts
- **Agent Systems**: Documents two agent systems (Superpowers Skills for workflows, Terraphim .agents for automation)
- **Configuration Management**: Role-based configs, feature flags, environment variables
- **Firecracker Integration**: Secure VM execution with sub-2 second boot times

## Critical Sections
- **Workspace Structure**: 29 library crates + 2 binaries (terraphim_server, terraphim_firecracker)
- **Critical Crates**: Service layer (terraphim_service, terraphim_middleware), agent system (6 crates), haystack integrations (4 crates)
- **Development Workflow**: Pre-commit hooks, testing scripts, watch commands, feature flags
- **Knowledge Graph System**: Thesaurus format, automata construction, rolegraph management
- **AI Integration**: OpenRouter, Ollama support with LLM client abstraction

## Recent Updates
- Added workspace structure section
- Expanded crate documentation (agent systems, haystacks)
- Added TUI build variations and feature flags
- Documented Firecracker integration
- Added testing scripts section
- Expanded desktop app development commands
- Added dependency constraints and troubleshooting

## Important Details
- Never use mocks in tests
- Pre-commit hooks mandatory for all commits
- Feature flags: `openrouter`, `mcp-rust-sdk`, `repl-full` for TUI
- Preferred storage backends: memory, dashmap, sqlite, redb (avoid RocksDB)
- Testing scripts location: `./quick-start-autocomplete.sh`, `./start-autocomplete-test.sh`
