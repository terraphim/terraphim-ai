# terraphim-repl

[![Crates.io](https://img.shields.io/crates/v/terraphim-repl.svg)](https://crates.io/crates/terraphim-repl)
[![License](https://img.shields.io/crates/l/terraphim-repl.svg)](https://github.com/terraphim/terraphim-ai/blob/main/LICENSE-Apache-2.0)

Offline-capable REPL for semantic knowledge graph search.

## Overview

`terraphim-repl` is a lightweight, standalone command-line interface for semantic search across knowledge graphs. It works completely offline with embedded defaults - no configuration required!

## Features

- 🔍 **Semantic Search**: Graph-based search with intelligent ranking
- 💾 **Offline Operation**: Embedded config and thesaurus - works without setup
- 📊 **Knowledge Graph**: Explore concept relationships and top terms
- 🎯 **Role-Based**: Switch between different knowledge domains
- ⚡ **Fast**: Optimized binary size (<50MB) and quick startup
- 🎨 **Colorful TUI**: Pretty tables and colored output

## Installation

### From crates.io (Recommended)

```bash
cargo install terraphim-repl
```

### From Source

```bash
git clone https://github.com/terraphim/terraphim-ai
cd terraphim-ai
cargo build --release -p terraphim-repl
./target/release/terraphim-repl
```

## Quick Start

### Launch the REPL

```bash
terraphim-repl
```

You'll see:

```
============================================================
🌍 Terraphim REPL v1.0.0
============================================================
Type /help for help, /quit to exit
Mode: Offline Mode | Current Role: Default

Available commands:
  /search <query> - Search documents
  /config show - Display configuration
  /role [list|select] - Manage roles
  /graph - Show knowledge graph
  /help [command] - Show help
  /quit - Exit REPL

Default> _
```

### Basic Commands

**Search for documents:**
```
Default> /search rust async
🔍 Searching for: 'rust async'
```

**View knowledge graph:**
```
Default> /graph
📊 Top 10 concepts:
  1. rust programming language
  2. async
ynchronous programming
  3. tokio async runtime
  ...
```

**List available roles:**
```
Default> /role list
Available roles:
  ▶ Default
```

**Show configuration:**
```
Default> /config show
{
  "selected_role": "Default",
  ...
}
```

**Get help:**
```
Default> /help
Default> /help search  # Detailed help for a command
```

**Exit:**
```
Default> /quit
Goodbye! 👋
```

## Command Reference

### /search

Search for documents matching a query.

**Syntax:**
```
/search <query> [--role <role>] [--limit <n>]
```

**Examples:**
```
/search rust
/search api --role Engineer --limit 5
/search async tokio
```

### /config

Display current configuration.

**Syntax:**
```
/config [show]
```

**Example:**
```
/config show
```

### /role

Manage roles (list or select).

**Syntax:**
```
/role list
/role select <name>
```

**Examples:**
```
/role list
/role select Engineer
```

### /graph

Show the knowledge graph's top concepts.

**Syntax:**
```
/graph [--top-k <n>]
```

**Examples:**
```
/graph
/graph --top-k 20
```

### /help

Show help information.

**Syntax:**
```
/help [command]
```

**Examples:**
```
/help
/help search
```

### /quit, /exit

Exit the REPL.

**Syntax:**
```
/quit
/exit
/q
```

## Configuration

### First Run

On first run, `terraphim-repl` creates:
- `~/.terraphim/config.json` - Configuration file
- `~/.terraphim/default_thesaurus.json` - Starter thesaurus
- `~/.terraphim_repl_history` - Command history

### Custom Configuration

Edit `~/.terraphim/config.json` to:
- Add new roles with specific knowledge domains
- Configure haystacks (data sources)
- Customize relevance functions

Example custom role:

```json
{
  "roles": {
    "Engineer": {
      "name": "Engineer",
      "relevance_function": "title-scorer",
      "haystacks": [
        {
          "location": "~/docs",
          "service": "Ripgrep"
        }
      ]
    }
  },
  "selected_role": "Engineer"
}
```

## Offline Operation

`terraphim-repl` is designed to work completely offline:

1. **Embedded Defaults**: Ships with default config and thesaurus
2. **No Network Required**: All operations are local
3. **Local Data**: Searches your local documents only
4. **Self-Contained**: Zero external dependencies after installation

## Features vs terraphim_tui

`terraphim-repl` is a minimal subset of `terraphim_tui`:

| Feature | terraphim-repl | terraphim_tui |
|---------|----------------|---------------|
| REPL Interface | ✅ | ✅ |
| Full-screen TUI | ❌ | ✅ |
| Basic Search | ✅ | ✅ |
| Knowledge Graph | ✅ | ✅ |
| AI Chat | ❌ | ✅ |
| MCP Tools | ❌ | ✅ |
| Web Operations | ❌ | ✅ |
| VM Management | ❌ | ✅ |
| Binary Size | <50MB | ~100MB+ |

Use `terraphim-repl` for:
- Quick semantic search CLI
- Lightweight installations
- Offline-only usage
- Minimal dependencies

Use `terraphim_tui` for:
- Full feature set
- AI integration
- Web scraping
- Advanced workflows

## Command History

`terraphim-repl` maintains command history across sessions in `~/.terraphim_repl_history`.

**Features:**
- Tab completion for commands
- Up/Down arrows for history navigation
- Ctrl+C or Ctrl+D to exit
- `/clear` to clear screen

## Troubleshooting

### REPL won't start

Check that `~/.terraphim/` directory exists:
```bash
ls -la ~/.terraphim/
```

If not, the first run should create it automatically.

### No search results

1. Check your configuration has haystacks defined
2. Verify the haystack paths exist
3. Ensure you have documents in those locations

### Command not found

Make sure you've installed the binary:
```bash
cargo install terraphim-repl
# Or use full path:
./target/release/terraphim-repl
```

## Building from Source

### Requirements

- Rust 1.70 or later
- No external dependencies required

### Build

```bash
# Debug build
cargo build -p terraphim-repl

# Release build (optimized)
cargo build --release -p terraphim-repl

# Run directly
cargo run -p terraphim-repl
```

### Run Tests

```bash
cargo test -p terraphim-repl
```

## Project Structure

```
crates/terraphim_repl/
├── Cargo.toml              # Minimal dependencies
├── README.md               # This file
├── CHANGELOG.md            # Version history
├── assets/                 # Embedded defaults
│   ├── default_config.json
│   └── default_thesaurus.json
└── src/
    ├── main.rs             # Entry point + asset loading
    ├── service.rs          # Service wrapper
    └── repl/               # REPL implementation
        ├── mod.rs
        ├── commands.rs     # Command definitions
        └── handler.rs      # REPL loop + handlers
```

## Related Projects

- **[terraphim_types](../terraphim_types)**: Core type definitions
- **[terraphim_automata](../terraphim_automata)**: Text matching engine
- **[terraphim_rolegraph](../terraphim_rolegraph)**: Knowledge graph implementation
- **[terraphim_service](../terraphim_service)**: Main service layer
- **[terraphim_tui](../terraphim_tui)**: Full TUI application

## Support

- **Discord**: https://discord.gg/VPJXB6BGuY
- **Discourse**: https://terraphim.discourse.group
- **Issues**: https://github.com/terraphim/terraphim-ai/issues

## License

Licensed under Apache-2.0. See [LICENSE](../../LICENSE-Apache-2.0) for details.

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.
