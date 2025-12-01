# REPL Extraction Plan

## Phase 2: REPL Binary (from MINIMAL_RELEASE_PLAN.md)

**Goal**: Extract standalone REPL from terraphim_tui for minimal v1.0.0 release

## Current Structure Analysis

### Module Organization

```
crates/terraphim_tui/src/
├── main.rs                    # Entry point with TUI + REPL subcommands
├── repl/
│   ├── mod.rs                # Feature-gated exports
│   ├── handler.rs            # Main REPL loop with rustyline (1527 lines)
│   ├── commands.rs           # Command definitions and parsing (1094 lines)
│   ├── chat.rs               # Chat functionality (repl-chat feature)
│   ├── mcp_tools.rs          # MCP tools (repl-mcp feature)
│   ├── file_operations.rs    # File operations (repl-file feature)
│   └── web_operations.rs     # Web operations (repl-web feature)
├── app.rs                    # TUI application state
├── ui.rs                     # TUI rendering
├── client.rs                 # API client
└── service.rs                # Local service wrapper
```

### Current Feature Flags

| Feature | Purpose | Commands |
|---------|---------|----------|
| `repl` | Base REPL | search, config, role, graph, help, quit, clear |
| `repl-chat` | AI integration | chat, summarize |
| `repl-mcp` | MCP tools | autocomplete, extract, find, replace, thesaurus |
| `repl-file` | File operations | file search/list/info |
| `repl-web` | Web operations | web get/post/scrape/screenshot/pdf/api |
| `repl-custom` | Custom commands | (experimental) |
| `repl-full` | All features | Combines all above |

### Dependencies Analysis

**REPL-specific (keep for minimal release)**:
- `rustyline = "14.0"` - Readline interface with history
- `colored = "2.1"` - Terminal colors
- `comfy-table = "7.1"` - Table formatting
- `dirs = "5.0"` - Home directory for history file

**TUI-specific (exclude from REPL binary)**:
- `ratatui = "0.29"` - Full-screen TUI framework
- `crossterm = "0.28"` - Terminal manipulation
- Only used in: `app.rs`, `ui.rs`, `main.rs` (TUI mode)

**Shared (required)**:
- `terraphim_service` - Core service layer
- `terraphim_config` - Configuration management
- `terraphim_types` - Type definitions
- `tokio` - Async runtime
- `anyhow` - Error handling
- `serde`, `serde_json` - Serialization

## REPL Extraction Strategy

### Approach 1: New Binary Crate (Recommended)

**Create**: `crates/terraphim_repl/` as a new lightweight binary crate

**Advantages**:
- Clean separation from TUI code
- Minimal dependencies
- Easier to maintain and document
- Better for cargo install terraphim-repl
- Can reuse code from terraphim_tui without bringing TUI deps

**Structure**:
```
crates/terraphim_repl/
├── Cargo.toml           # Minimal dependencies
├── README.md            # REPL documentation
├── src/
│   ├── main.rs          # Simple entry point
│   ├── assets.rs        # Embedded default config/thesaurus
│   └── repl/            # Copy from terraphim_tui/src/repl/
│       ├── mod.rs
│       ├── handler.rs   # Minimal feature set
│       └── commands.rs  # Minimal command set
└── assets/              # Embedded resources
    ├── default_config.json
    └── default_thesaurus.json
```

### Approach 2: Feature Flag (Alternative)

**Modify**: `terraphim_tui` to have `repl-only` feature

**Advantages**:
- No code duplication
- Shares maintenance with TUI

**Disadvantages**:
- Still pulls TUI dependencies as optional
- More complex build setup
- Less clear separation

**Conclusion**: Go with Approach 1 for cleaner minimal release.

## Implementation Plan

### Step 1: Create New Crate Structure

```bash
cargo new --bin crates/terraphim_repl
```

### Step 2: Minimal Cargo.toml

```toml
[package]
name = "terraphim-repl"
version = "1.0.0"
edition = "2024"
description = "Offline-capable REPL for semantic knowledge graph search"
license = "Apache-2.0"

[[bin]]
name = "terraphim-repl"
path = "src/main.rs"

[dependencies]
# Core terraphim crates
terraphim_service = { path = "../terraphim_service", version = "1.0.0" }
terraphim_config = { path = "../terraphim_config", version = "1.0.0" }
terraphim_types = { path = "../terraphim_types", version = "1.0.0" }
terraphim_automata = { path = "../terraphim_automata", version = "1.0.0" }

# REPL interface
rustyline = "14.0"
colored = "2.1"
comfy-table = "7.1"
dirs = "5.0"

# Async runtime
tokio = { version = "1.42", features = ["full"] }

# Error handling
anyhow = "1.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Asset embedding
rust-embed = "8.5"

[features]
default = ["repl-minimal"]
repl-minimal = []  # Base commands only
```

### Step 3: Embed Default Assets

Create `crates/terraphim_repl/assets/`:
- `default_config.json` - Minimal role with local search
- `default_thesaurus.json` - Small starter thesaurus (100-200 common tech terms)

Use `rust-embed` to bundle:
```rust
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;
```

### Step 4: Minimal Command Set

For v1.0.0 minimal release, include only:
- `/search <query>` - Search documents
- `/config show` - View configuration
- `/role list` - List available roles
- `/role select <name>` - Switch roles
- `/graph` - Show knowledge graph top concepts
- `/help` - Show command help
- `/quit` - Exit REPL

**Exclude from minimal** (save for v1.1.0+):
- `/chat` - Requires LLM integration
- `/autocomplete`, `/extract`, `/find`, `/replace` - MCP tools
- `/file` - File operations
- `/web` - Web operations
- `/vm` - VM management

### Step 5: Simplified Entry Point

```rust
// crates/terraphim_repl/src/main.rs

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load embedded default config if no config exists
    let service = terraphim_service::TuiService::new().await?;

    // Launch REPL
    let mut handler = repl::ReplHandler::new_offline(service);
    handler.run().await
}
```

### Step 6: Update Workspace Configuration

Add to `Cargo.toml`:
```toml
members = [
    # ... existing members ...
    "crates/terraphim_repl",
]

default-members = ["terraphim_server", "crates/terraphim_repl"]
```

## Offline Operation Strategy

### Default Assets Bundle

1. **Minimal Config** (`default_config.json`):
```json
{
  "selected_role": "Default",
  "server_host": "127.0.0.1",
  "server_port": 3000,
  "roles": {
    "Default": {
      "name": "Default",
      "relevance_function": "TitleScorer",
      "theme": "dark",
      "haystacks": []
    }
  }
}
```

2. **Starter Thesaurus** (`default_thesaurus.json`):
- 100-200 common tech terms for demonstration
- Examples: "rust", "async", "tokio", "cargo", "http", "api", etc.
- Pulled from existing terraphim_server/default/ files

3. **Sample Documents**:
- 10-20 minimal markdown docs about Rust/Terraphim basics
- Demonstrates search functionality without external dependencies

### First-Run Experience

```
🌍 Terraphim REPL v1.0.0
==================================================
Welcome! Running in offline mode with default configuration.

To get started:
  /search rust        - Search sample documents
  /graph              - View knowledge graph
  /help               - Show all commands

Type /quit to exit

Default> _
```

## Testing Plan

### Unit Tests
- [ ] Command parsing (commands.rs tests exist)
- [ ] Asset loading from embedded resources
- [ ] Offline service initialization

### Integration Tests
- [ ] REPL launches without external dependencies
- [ ] Search works with embedded thesaurus
- [ ] Config loads from embedded defaults
- [ ] History persists across sessions

### Manual Testing
```bash
# Build REPL binary
cargo build -p terraphim-repl --release

# Test offline operation (no network, no config files)
./target/release/terraphim-repl

# Test commands
/search rust
/graph
/role list
/config show
/quit
```

## Installation Strategy

### Cargo Install
```bash
cargo install terraphim-repl
```

### Pre-built Binaries
Package for:
- Linux x86_64 (statically linked)
- macOS x86_64 + ARM64
- Windows x86_64

### Distribution
- GitHub Releases with binaries
- crates.io for Rust users
- Homebrew formula (future)
- apt/yum packages (future)

## Documentation Plan

### README.md for terraphim_repl

```markdown
# terraphim-repl

Offline-capable REPL for semantic knowledge graph search.

## Quick Start

```bash
cargo install terraphim-repl
terraphim-repl
```

## Features

- 🔍 Semantic search across local documents
- 📊 Knowledge graph visualization
- 💾 Offline operation with embedded defaults
- 🎯 Role-based configuration
- ⚡ Fast autocomplete and matching

## Commands

- `/search <query>` - Search documents
- `/graph` - Show knowledge graph
- `/role list` - List roles
- `/config show` - View configuration
- `/help` - Show all commands
- `/quit` - Exit

## Configuration

Default config is embedded. To customize:
1. Run REPL once to generate `~/.terraphim/config.json`
2. Edit config with your roles and haystacks
3. Restart REPL

## Examples

...
```

### CHANGELOG.md

Document v1.0.0 minimal release with:
- Initial REPL release
- Embedded defaults for offline use
- Core commands (search, config, role, graph)
- Installation instructions

## Success Criteria

- [ ] Binary builds with zero external dependencies required
- [ ] REPL launches and works offline without any setup
- [ ] Search functionality works with embedded thesaurus
- [ ] Documentation is complete and clear
- [ ] Binary size is < 50MB (release build)
- [ ] Installation via `cargo install` works
- [ ] Pre-built binaries for Linux/macOS/Windows
- [ ] Tests pass for offline operation

## Timeline (from MINIMAL_RELEASE_PLAN.md)

**Week 2, Days 1-5**:
- Day 1: Create crate structure, minimal Cargo.toml
- Day 2: Copy REPL code, simplify dependencies
- Day 3: Embed default assets, test offline operation
- Day 4: Build scripts, cross-platform testing
- Day 5: Documentation, final testing

## Next Steps

1. ✅ Analysis complete - document REPL structure
2. ⏭️ Create `crates/terraphim_repl/` directory structure
3. ⏭️ Write minimal Cargo.toml
4. ⏭️ Create simplified main.rs
5. ⏭️ Copy REPL modules from terraphim_tui
6. ⏭️ Create and embed default assets
7. ⏭️ Test offline operation
8. ⏭️ Write README and documentation
9. ⏭️ Build release binaries
