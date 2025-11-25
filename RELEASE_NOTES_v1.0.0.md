# Terraphim v1.0.0 - Minimal Release

**Release Date**: 2025-01-25
**Tag**: v1.0.0-minimal

## 🎉 Overview

First stable release of Terraphim's **minimal toolkit** for semantic knowledge graph search. This release provides three core library crates and two user-facing binaries optimized for offline operation and minimal dependencies.

## 📦 What's Included

### Library Crates (3)

1. **[terraphim_types](crates/terraphim_types)** v1.0.0
   - Core type definitions for knowledge graphs, documents, and search
   - 15+ data structures with comprehensive rustdoc
   - Zero dependencies beyond standard library + serde

2. **[terraphim_automata](crates/terraphim_automata)** v1.0.0
   - Fast text matching using Aho-Corasick automata
   - Autocomplete with fuzzy search (Levenshtein & Jaro-Winkler)
   - WASM support for browser usage
   - Link generation (Markdown, HTML, Wiki)

3. **[terraphim_rolegraph](crates/terraphim_rolegraph)** v1.0.0
   - Knowledge graph implementation for semantic search
   - Graph-based document ranking
   - Multi-term query operators (AND, OR, NOT)

### Binary Tools (2)

4. **[terraphim-repl](crates/terraphim_repl)** v1.0.0
   - Interactive REPL for semantic search
   - 11 commands including KG operations
   - Offline-capable with embedded defaults
   - Binary size: ~13MB

5. **[terraphim-cli](crates/terraphim_cli)** v1.0.0
   - Automation-friendly CLI with JSON output
   - 8 commands optimized for scripting
   - Shell completions (bash/zsh/fish)
   - Binary size: ~13MB

---

## ✨ Features

### terraphim_types v1.0.0

**Core Types:**
- `Document`: Full-text search documents with metadata
- `Thesaurus`: Knowledge graph term mappings
- `RoleName`: Case-insensitive role identifiers
- `SearchQuery`: Structured search with operators
- `Concept`, `Node`, `Edge`: Graph building blocks

**Documentation:**
- Comprehensive rustdoc with examples
- README with quick-start guide
- All types implement Clone + Debug + Serialize

### terraphim_automata v1.0.0

**Text Processing:**
- `find_matches()`: Aho-Corasick pattern matching
- `replace_matches()`: Generate linked text
- `autocomplete_search()`: Prefix-based suggestions
- `fuzzy_autocomplete_search()`: Fuzzy matching with thresholds

**Link Generation:**
- Markdown: `[term](url)`
- HTML: `<a href="url">term</a>`
- Wiki: `[[term]]`

**WASM Support:**
- Browser-compatible via wasm-pack
- TypeScript bindings via tsify
- ~200KB compressed bundle

### terraphim_rolegraph v1.0.0

**Graph Operations:**
- `insert_node()`, `insert_edge()`: Build graphs
- `insert_document()`: Index documents
- `query_graph()`: Semantic search
- `query_graph_with_operators()`: AND/OR/NOT queries
- `get_stats()`: Graph statistics

**Ranking:**
- Graph-based relevance scoring
- Path traversal between matched concepts
- Configurable ranking algorithms

### terraphim-repl v1.0.0

**Commands (11):**
- `/search <query>` - Search documents
- `/config show` - View configuration
- `/role list|select` - Manage roles
- `/graph [--top-k]` - Show concepts
- `/replace <text>` - Replace with links
- `/find <text>` - Find matches
- `/thesaurus` - View KG terms
- `/help`, `/quit`, `/clear` - Utilities

**Features:**
- Colored tables (comfy-table)
- Command history (rustyline)
- Tab completion
- Embedded default config + thesaurus

### terraphim-cli v1.0.0

**Commands (8):**
- `search <query>` - JSON search results
- `config` - Show configuration
- `roles` - List roles
- `graph` - Top concepts
- `replace <text>` - Link generation
- `find <text>` - Match finding
- `thesaurus` - KG terms
- `completions <shell>` - Shell completions

**Features:**
- JSON output (default) or JSON Pretty
- Exit codes: 0=success, 1=error
- `--quiet` flag for pure JSON
- Pipe-friendly design

---

## 📥 Installation

### From crates.io

```bash
# Library crates
cargo add terraphim_types
cargo add terraphim_automata
cargo add terraphim_rolegraph

# Binary tools
cargo install terraphim-repl
cargo install terraphim-cli
```

### From Source

```bash
git clone https://github.com/terraphim/terraphim-ai
cd terraphim-ai

# Build libraries
cargo build --release -p terraphim_types
cargo build --release -p terraphim_automata
cargo build --release -p terraphim_rolegraph

# Build binaries
cargo build --release -p terraphim-repl
cargo build --release -p terraphim-cli
```

---

## 🚀 Quick Start

### Library Usage

```rust
use terraphim_types::{Document, Thesaurus};
use terraphim_automata::find_matches;

// Load thesaurus
let thesaurus = Thesaurus::from_file("my_thesaurus.json")?;

// Find matches in text
let text = "Rust is great for async programming";
let matches = find_matches(text, thesaurus, true)?;

for m in matches {
    println!("Found: {} at position {:?}", m.term, m.pos);
}
```

### REPL Usage

```bash
$ terraphim-repl
🌍 Terraphim REPL v1.0.0
============================================================
Type /help for help, /quit to exit

Default> /search rust async
🔍 Searching for: 'rust async'
...

Default> /thesaurus
📚 Loading thesaurus for role: Default
✅ Thesaurus 'default' contains 30 terms
...
```

### CLI Usage

```bash
# Search with JSON output
$ terraphim-cli search "rust async"
{
  "query": "rust async",
  "role": "Default",
  "results": [...],
  "count": 5
}

# Pipe to jq
$ terraphim-cli search "rust" | jq '.results[].title'
"Async Programming in Rust"
"The Rust Programming Language"

# Generate completions
$ terraphim-cli completions bash > terraphim-cli.bash
```

---

## 📊 Performance

### Binary Sizes (Linux x86_64)
- `terraphim-repl`: 13MB (stripped, LTO-optimized)
- `terraphim-cli`: 13MB (stripped, LTO-optimized)

### Library Characteristics
- `terraphim_types`: Minimal dependencies, fast compilation
- `terraphim_automata`: Aho-Corasick O(n) text matching
- `terraphim_rolegraph`: In-memory graph operations

### WASM Bundle
- terraphim_automata: ~200KB compressed
- Browser compatible: Chrome 57+, Firefox 52+, Safari 11+

---

## 🔧 Technical Details

### Rust Edition & Toolchain
- **Edition**: 2024
- **MSRV**: Rust 1.70+
- **Resolver**: Version 2

### Build Profiles
```toml
[profile.release]
opt-level = "z"     # Size optimization
lto = true          # Link-time optimization
codegen-units = 1   # Maximum optimization
strip = true        # Strip symbols
```

### Dependencies Philosophy
- **Minimal**: Only essential dependencies
- **No network**: All tools work offline
- **Embedded defaults**: Zero configuration required

### Offline Operation
Both binaries include:
- Embedded default configuration
- Starter thesaurus (30 tech terms)
- Auto-create `~/.terraphim/` on first run

---

## 📚 Documentation

### Per-Crate READMEs
- [terraphim_types/README.md](crates/terraphim_types/README.md)
- [terraphim_automata/README.md](crates/terraphim_automata/README.md)
- [terraphim_rolegraph/README.md](crates/terraphim_rolegraph/README.md)
- [terraphim-repl/README.md](crates/terraphim_repl/README.md)
- [terraphim-cli/README.md](crates/terraphim_cli/README.md)

### Changelogs
- [terraphim_types/CHANGELOG.md](crates/terraphim_types/CHANGELOG.md)
- [terraphim_automata/CHANGELOG.md](crates/terraphim_automata/CHANGELOG.md)
- [terraphim_rolegraph/CHANGELOG.md](crates/terraphim_rolegraph/CHANGELOG.md)
- [terraphim-repl/CHANGELOG.md](crates/terraphim_repl/CHANGELOG.md)
- [terraphim-cli/CHANGELOG.md](crates/terraphim_cli/CHANGELOG.md)

### API Documentation
```bash
# Generate docs
cargo doc --no-deps -p terraphim_types --open
cargo doc --no-deps -p terraphim_automata --open
cargo doc --no-deps -p terraphim_rolegraph --open
```

---

## 🎯 Use Cases

### Library Crates
- **terraphim_types**: Data models for knowledge graph applications
- **terraphim_automata**: Fast text processing and autocomplete
- **terraphim_rolegraph**: Semantic search with graph ranking

### REPL Binary
- Interactive knowledge graph exploration
- Learning the Terraphim system
- Ad-hoc semantic queries
- Configuration management

### CLI Binary
- CI/CD pipelines
- Shell scripts and automation
- Batch text processing
- API integration via JSON

---

## 🔄 Migration Guide

This is the **first stable release**, so there's no migration needed. However, note:

- Future v1.x releases will maintain API compatibility
- v2.0 will be reserved for breaking changes
- Deprecations will be announced one minor version in advance

---

## 🐛 Known Issues & Limitations

### v1.0.0 Scope
- **No AI Integration**: LLM chat and summarization excluded (future v1.1+)
- **No MCP Tools**: Advanced MCP operations excluded (future v1.1+)
- **No Web/File Ops**: Web scraping and file operations excluded (future v1.1+)
- **Placeholder Graph Data**: Real role graph integration pending

### Workarounds
- For AI features: Use full `terraphim_tui` from main branch
- For MCP tools: Use `terraphim_mcp_server` separately
- For production deployments: See `terraphim_server`

---

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Key areas for contribution:**
- Additional thesaurus examples
- More comprehensive documentation
- Platform-specific packaging (Homebrew, apt, etc.)
- WASM examples and tutorials

---

## 📄 License

Licensed under Apache-2.0. See [LICENSE-Apache-2.0](LICENSE-Apache-2.0) for details.

---

## 🙏 Acknowledgments

Built with:
- [Aho-Corasick](https://github.com/BurntSushi/aho-corasick) - Fast string matching
- [FST](https://github.com/BurntSushi/fst) - Finite state transducers
- [Clap](https://github.com/clap-rs/clap) - CLI argument parsing
- [Rustyline](https://github.com/kkawakam/rustyline) - REPL interface
- [Tokio](https://tokio.rs) - Async runtime

---

## 🔗 Links

- **Repository**: https://github.com/terraphim/terraphim-ai
- **Discord**: https://discord.gg/VPJXB6BGuY
- **Discourse**: https://terraphim.discourse.group
- **Issues**: https://github.com/terraphim/terraphim-ai/issues

---

## 📈 What's Next

### v1.1.0 (Planned)
- REPL: Add `repl-chat` feature for AI integration
- REPL: Add `repl-mcp` feature for MCP tools
- CLI: Add `--output` flag for file output
- Libraries: Performance optimizations

### v1.2.0 (Planned)
- REPL: Add `repl-web` and `repl-file` features
- CLI: Add batch processing mode
- Libraries: Additional graph algorithms

### v2.0.0 (Future)
- Full integration with terraphim_service
- Real role graph implementation
- API compatibility guaranteed within v1.x

---

**Thank you for using Terraphim! 🌍**
