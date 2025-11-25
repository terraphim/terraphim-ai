# Changelog

All notable changes to `terraphim-repl` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2025-01-25

### Added

#### Core REPL Features
- **Offline Operation**: Embedded default configuration and thesaurus for zero-setup usage
- **Semantic Search**: Graph-based document search with intelligent ranking
- **Knowledge Graph**: View top concepts and relationships
- **Role Management**: List and switch between different knowledge domains
- **Command History**: Persistent history across sessions with tab completion
- **Colorful UI**: Pretty tables and colored terminal output

#### Commands
- `/search <query> [--role <role>] [--limit <n>]` - Search documents
- `/config [show]` - Display current configuration
- `/role list | select <name>` - Manage roles
- `/graph [--top-k <n>]` - Show knowledge graph concepts
- `/help [command]` - Show help information
- `/quit`, `/exit`, `/q` - Exit REPL
- `/clear` - Clear screen

#### Asset Embedding
- Default configuration with minimal role
- Starter thesaurus with 30 common technical terms
- Automatic first-run setup in `~/.terraphim/`

#### Configuration
- `~/.terraphim/config.json` - User configuration
- `~/.terraphim/default_thesaurus.json` - Default thesaurus
- `~/.terraphim_repl_history` - Command history

#### Performance
- Optimized binary size (<50MB with release profile)
- Link-time optimization (LTO) enabled
- Symbol stripping for minimal footprint
- Fast startup with embedded assets

#### Dependencies
- Minimal dependency set (8 crates + terraphim stack)
- No TUI framework (ratatui/crossterm)
- rustyline for REPL interface
- colored + comfy-table for terminal UI
- rust-embed for asset bundling

### Technical Details

**Architecture:**
- Standalone binary with embedded assets
- Wrapper around `TerraphimService` for offline operation
- Simplified command set (8 commands vs terraphim_tui's 20+)
- REPL-only interface (no full-screen TUI)

**Build Configuration:**
- Rust edition 2024
- Release profile optimized for size (`opt-level = "z"`)
- LTO enabled for better optimization
- Single codegen unit for maximum optimization

**Compatibility:**
- Works with terraphim_types v1.0.0
- Works with terraphim_automata v1.0.0
- Works with terraphim_rolegraph v1.0.0
- Works with terraphim_service v1.0.0

### Features for Future Releases

Future versions (v1.1.0+) may include:
- `repl-chat` - AI chat integration
- `repl-mcp` - MCP tools (autocomplete, extract, find, replace)
- `repl-file` - File operations
- `repl-web` - Web operations

These are deliberately excluded from v1.0.0 minimal release to keep the binary small and focused on core search functionality.

[Unreleased]: https://github.com/terraphim/terraphim-ai/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/terraphim/terraphim-ai/releases/tag/v1.0.0
