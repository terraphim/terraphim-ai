# Minimal Release Plan: Lib, REPL, and CLI

**Version:** v1.0.0-minimal
**Target Timeline:** 3 weeks
**Branch:** `claude/create-plan-01D3gjdfghh3Ak17cnQMemFG`
**Created:** 2025-01-22

## 🎯 Release Scope

A minimal release focused on three core components:
1. **Library (lib)** - Core knowledge graph and automata functionality
2. **REPL** - Interactive terminal interface
3. **CLI** - Command-line tools for search and management

## 📦 Component 1: Library Release (Crates.io)

### Core Crates (3)

**Publish to crates.io in dependency order:**

#### 1. terraphim_types v1.0.0
- **Purpose**: Shared type definitions across Terraphim ecosystem
- **Location**: `crates/terraphim_types/`
- **Dependencies**: Minimal (serde, ahash, chrono, uuid, thiserror)
- **Features**:
  - Core types: Document, SearchQuery, LogicalOperator, RoleName
  - WASM-ready with conditional compilation
  - TypeScript type generation via `tsify` (optional)
- **WASM Support**: ✅ Full support with `typescript` feature

#### 2. terraphim_automata v1.0.0
- **Purpose**: Text matching, autocomplete, and thesaurus engine
- **Location**: `crates/terraphim_automata/`
- **Dependencies**: terraphim_types, aho-corasick, fst, strsim, serde
- **Features**:
  - `remote-loading`: HTTP thesaurus loading
  - `tokio-runtime`: Async runtime support
  - `typescript`: TypeScript bindings
  - `wasm`: WebAssembly target support
- **Key Functions**:
  - `load_thesaurus()` - Load and parse thesaurus files
  - `autocomplete_terms()` - Fast autocomplete
  - `fuzzy_autocomplete_search_jaro_winkler()` - Fuzzy search
  - `find_matches()` - Text pattern matching
  - `extract_paragraphs_from_automata()` - Context extraction
- **WASM Support**: ✅ Full support, tested with `wasm-pack`

#### 3. terraphim_rolegraph v1.0.0
- **Purpose**: Knowledge graph construction and querying
- **Location**: `crates/terraphim_rolegraph/`
- **Dependencies**: terraphim_types, terraphim_automata, ahash, regex
- **Key Functions**:
  - Graph construction from documents and thesaurus
  - Node/edge relationship management
  - Path connectivity analysis
  - Document-to-concept mappings
- **WASM Support**: ⚠️ Requires tokio, limited WASM compatibility

### Library Features

- ✅ Knowledge graph construction from thesaurus files (JSON format)
- ✅ Fast text matching with Aho-Corasick automata
- ✅ Fuzzy autocomplete with Jaro-Winkler distance
- ✅ Graph path connectivity analysis (`is_all_terms_connected_by_path`)
- ✅ WASM bindings for browser usage (automata only)
- ✅ Caching with `cached` crate for performance
- ✅ Comprehensive error handling with `thiserror`

### Documentation Requirements

**For each crate:**
- [ ] README.md with:
  - Overview and purpose
  - Installation instructions
  - Basic usage examples
  - Feature flags documentation
  - API overview
  - Links to full docs
- [ ] Comprehensive rustdoc comments on:
  - All public functions
  - All public types and structs
  - Module-level documentation
  - Examples in doc comments
- [ ] CHANGELOG.md following [Keep a Changelog](https://keepachangelog.com/)
- [ ] LICENSE file (Apache-2.0)

**Special documentation:**
- [ ] WASM usage guide for terraphim_automata
- [ ] Integration examples showing all three crates together
- [ ] Performance benchmarks and optimization tips
- [ ] Migration guide from older versions (if applicable)

## 🖥️ Component 2: REPL Binary

### Package: terraphim-repl

**Source**: `crates/terraphim_tui/` (refactored)
**Binary Name**: `terraphim-repl`
**Build Command**:
```bash
cargo build -p terraphim_tui --features repl-full --release --bin terraphim-repl
```

### REPL Features (Keep Existing)

**Search & Query:**
- `/search "query"` - Semantic search with knowledge graphs
- `/autocomplete "prefix"` - Autocomplete suggestions
- `/graph "term1" "term2"` - Check graph connectivity

**AI Integration:**
- `/chat "message"` - AI conversation (requires LLM provider)
- `/summarize` - Document summarization

**Configuration:**
- `/config` - Configuration management
- `/roles` - Role switching and listing
- `/roles switch <name>` - Change active role

**Advanced:**
- `/commands list` - List markdown-defined custom commands
- `/vm` - VM management (if Firecracker available)
- `/file read <path>` - File operations
- `/web fetch <url>` - Web fetching

**Utility:**
- `/help` - Interactive help system
- `/help <command>` - Command-specific help
- `/history` - Command history
- `/clear` - Clear screen
- `/exit` - Exit REPL

### Simplifications for Minimal Release

**Remove:**
- [ ] Full-screen TUI mode (ratatui-based interface)
- [ ] Server API mode (`--server` flag)
- [ ] Remote server dependencies
- [ ] Advanced haystack integrations (Atlassian, Discourse, JMAP)
- [ ] MCP tools integration
- [ ] Complex agent workflows

**Keep:**
- [x] REPL-only interactive mode
- [x] Self-contained offline operation
- [x] Autocomplete and search
- [x] Basic configuration management
- [x] Role switching
- [x] File operations
- [x] Command history with rustyline

**Simplify:**
- [ ] Bundle minimal default thesaurus files
- [ ] Include example config in binary (rust-embed)
- [ ] Reduce optional features to essentials
- [ ] Remove dependency on terraphim_server crates

### Binary Configuration

**Embedded Assets:**
```rust
#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

// Include:
// - default_config.json
// - minimal_thesaurus.json
// - help.txt
// - LICENSE
```

**Features:**
```toml
[features]
default = ["repl-basic"]
repl-basic = ["dep:rustyline", "dep:colored", "dep:comfy-table"]
repl-full = ["repl-basic", "repl-file"]
repl-file = ["repl-basic"]
```

### Distribution

**Binary Packages:**
- `terraphim-repl-v1.0.0-linux-x86_64.tar.gz`
- `terraphim-repl-v1.0.0-linux-aarch64.tar.gz`
- `terraphim-repl-v1.0.0-macos-x86_64.tar.gz`
- `terraphim-repl-v1.0.0-macos-aarch64.tar.gz`
- `terraphim-repl-v1.0.0-windows-x86_64.zip`

**Package Contents:**
```
terraphim-repl/
├── bin/
│   └── terraphim-repl          # Binary
├── LICENSE                      # Apache-2.0
├── README.md                    # Quick start
└── examples/
    ├── config.json             # Example config
    └── thesaurus.json          # Example thesaurus
```

**Installation Methods:**
```bash
# Direct binary download
curl -L https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-repl-linux-x86_64.tar.gz | tar xz
sudo mv terraphim-repl/bin/terraphim-repl /usr/local/bin/

# Cargo install (requires Rust)
cargo install terraphim-repl

# Package managers (future)
# brew install terraphim-repl
# apt install terraphim-repl
```

### Auto-update Support

Uses `terraphim_update` crate:
```bash
terraphim-repl update check
terraphim-repl update install
```

## 🔧 Component 3: CLI Binary

### Option A: Extract from TUI (Recommended)

**Package: terraphim-cli**
**Source**: New binary crate using TUI's service layer
**Binary Name**: `terraphim-cli`

**Commands:**
```bash
# Search
terraphim-cli search "rust async" --role engineer --limit 10
terraphim-cli search "kubernetes" --terms pod,service --operator and

# Autocomplete
terraphim-cli autocomplete "knowl" --max-results 5
terraphim-cli autocomplete "auth" --fuzzy --threshold 0.8

# Roles
terraphim-cli roles list
terraphim-cli roles show engineer
terraphim-cli roles switch engineer

# Configuration
terraphim-cli config show
terraphim-cli config get role
terraphim-cli config set role engineer

# Graph operations
terraphim-cli graph build --thesaurus thesaurus.json
terraphim-cli graph query "authentication" "authorization" --check-path
terraphim-cli graph stats
```

### CLI Features

**Automation-Friendly:**
- JSON output for all commands (`--json` flag)
- Exit codes:
  - 0: Success
  - 1: General error
  - 2: Not found
  - 3: Configuration error
- No interactive prompts by default
- Scriptable output format

**Output Modes:**
```bash
# Human-readable (default)
terraphim-cli search "rust" --limit 5

# JSON output
terraphim-cli search "rust" --limit 5 --json
# {"results": [...], "total": 42, "time_ms": 123}

# Quiet mode (IDs only)
terraphim-cli search "rust" --quiet
# doc-id-1
# doc-id-2
```

**Optional Features:**
- Colored output (auto-detect TTY, `--no-color` to disable)
- Progress indicators for long operations (`--no-progress`)
- Verbose logging (`-v`, `-vv`, `-vvv`)

### CLI Implementation

**Cargo.toml:**
```toml
[package]
name = "terraphim-cli"
version = "1.0.0"
edition = "2021"

[dependencies]
terraphim_types = { path = "../terraphim_types", version = "1.0.0" }
terraphim_automata = { path = "../terraphim_automata", version = "1.0.0" }
terraphim_rolegraph = { path = "../terraphim_rolegraph", version = "1.0.0" }
terraphim_config = { path = "../terraphim_config", version = "1.0.0" }
terraphim_service = { path = "../terraphim_service", version = "1.0.0" }

clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde_json = "1.0"
colored = "3.0"
indicatif = { version = "0.18", optional = true }
anyhow = "1.0"
```

**Structure:**
```
terraphim-cli/
├── src/
│   ├── main.rs              # Entry point, CLI parser
│   ├── commands/
│   │   ├── search.rs        # Search command
│   │   ├── autocomplete.rs  # Autocomplete command
│   │   ├── roles.rs         # Role management
│   │   ├── config.rs        # Configuration
│   │   └── graph.rs         # Graph operations
│   ├── output.rs            # Output formatting
│   └── error.rs             # Error handling
└── tests/
    └── integration.rs       # Integration tests
```

**Completion Scripts:**
Generate for major shells:
```bash
terraphim-cli completions bash > terraphim-cli.bash
terraphim-cli completions zsh > _terraphim-cli
terraphim-cli completions fish > terraphim-cli.fish
```

### Distribution

Same as REPL: multi-platform binaries via GitHub releases.

## 📋 Implementation Phases

### Phase 1: Library Preparation (Week 1)

**Tasks:**
- [ ] **Day 1-2**: Audit terraphim_types
  - Review all public APIs
  - Add comprehensive rustdoc comments
  - Create README with examples
  - Add CHANGELOG.md
  - Test compilation and all features

- [ ] **Day 3-4**: Audit terraphim_automata
  - Review all public APIs
  - Add comprehensive rustdoc comments
  - Create README with examples
  - Test WASM build thoroughly
  - Add WASM usage guide
  - Benchmark critical functions
  - Add CHANGELOG.md

- [ ] **Day 5-6**: Audit terraphim_rolegraph
  - Review all public APIs
  - Add comprehensive rustdoc comments
  - Create README with examples
  - Add integration example using all 3 crates
  - Add CHANGELOG.md

- [ ] **Day 7**: Final library checks
  - Run all tests across all 3 crates
  - Test in fresh environment
  - Verify documentation builds
  - Check for any warnings
  - Prepare for crates.io publication

**Deliverables:**
- 3 crates ready for crates.io publication
- Comprehensive documentation
- Working examples
- All tests passing

### Phase 2: REPL Binary (Week 2)

**Tasks:**
- [ ] **Day 1-2**: Extract REPL mode
  - Create new binary target `terraphim-repl`
  - Remove TUI full-screen mode dependencies
  - Remove server mode code
  - Simplify feature flags

- [ ] **Day 3-4**: Bundle assets
  - Integrate rust-embed for configs
  - Bundle minimal thesaurus
  - Bundle help documentation
  - Test offline operation

- [ ] **Day 5**: Test and optimize
  - Test on all platforms
  - Optimize binary size
  - Add compression
  - Test installation scripts

- [ ] **Day 6-7**: Package and document
  - Create installation scripts
  - Write REPL user guide
  - Create demo recordings
  - Test auto-update feature

**Deliverables:**
- Self-contained REPL binary <50MB
- Multi-platform packages
- Installation scripts
- User documentation

### Phase 3: CLI Binary (Week 2, Days 6-7 overlap)

**Tasks:**
- [ ] **Day 1-2**: Create CLI structure
  - Set up new binary crate
  - Implement command structure with clap
  - Create output formatting module
  - Implement JSON output mode

- [ ] **Day 3-4**: Implement commands
  - Search command with all options
  - Autocomplete command
  - Roles management
  - Configuration commands
  - Graph operations

- [ ] **Day 5**: Polish and test
  - Add completion script generation
  - Test exit codes
  - Test JSON output parsing
  - Integration tests

- [ ] **Day 6**: Package
  - Create binaries for all platforms
  - Write CLI documentation
  - Create example scripts

**Deliverables:**
- Automation-friendly CLI binary
- Shell completion scripts
- CLI documentation
- Example scripts

### Phase 4: Documentation & Release (Week 3)

**Tasks:**
- [ ] **Day 1-2**: Documentation
  - Write main README for minimal release
  - Create quick-start guide (5-minute setup)
  - Write architecture overview
  - Create comparison guide (REPL vs CLI vs lib)

- [ ] **Day 3**: Demo content
  - Record demo GIFs for README
  - Create video tutorial (optional)
  - Write blog post announcement
  - Prepare social media content

- [ ] **Day 4**: Publication
  - Publish crates to crates.io:
    1. terraphim_types
    2. terraphim_automata
    3. terraphim_rolegraph
  - Verify crates published correctly
  - Test installation from crates.io

- [ ] **Day 5**: Binary release
  - Create GitHub release v1.0.0-minimal
  - Upload all binary packages
  - Tag the release
  - Update documentation links

- [ ] **Day 6**: Announcement
  - Update main repository README
  - Post to Discord
  - Post to Discourse forum
  - Share on social media
  - Monitor for issues

- [ ] **Day 7**: Buffer for fixes
  - Address any immediate issues
  - Update documentation based on feedback
  - Plan next iteration

**Deliverables:**
- Published crates on crates.io
- GitHub release with binaries
- Complete documentation
- Announcement materials

## 🎁 Release Artifacts

### Crates.io Packages

**Published crates:**
1. `terraphim_types` v1.0.0
   - https://crates.io/crates/terraphim_types
   - Documentation: https://docs.rs/terraphim_types

2. `terraphim_automata` v1.0.0
   - https://crates.io/crates/terraphim_automata
   - Documentation: https://docs.rs/terraphim_automata

3. `terraphim_rolegraph` v1.0.0
   - https://crates.io/crates/terraphim_rolegraph
   - Documentation: https://docs.rs/terraphim_rolegraph

### Binary Releases (GitHub)

**Release tag**: `v1.0.0-minimal`

**Artifacts:**
- `terraphim-repl-v1.0.0-linux-x86_64.tar.gz`
- `terraphim-repl-v1.0.0-linux-aarch64.tar.gz`
- `terraphim-repl-v1.0.0-macos-x86_64.tar.gz`
- `terraphim-repl-v1.0.0-macos-aarch64.tar.gz`
- `terraphim-repl-v1.0.0-windows-x86_64.zip`
- `terraphim-cli-v1.0.0-linux-x86_64.tar.gz`
- `terraphim-cli-v1.0.0-linux-aarch64.tar.gz`
- `terraphim-cli-v1.0.0-macos-x86_64.tar.gz`
- `terraphim-cli-v1.0.0-macos-aarch64.tar.gz`
- `terraphim-cli-v1.0.0-windows-x86_64.zip`
- `checksums.txt` - SHA256 checksums
- `RELEASE_NOTES.md` - Release notes

### Docker Images (Optional, Future)

```bash
docker pull terraphim/terraphim-repl:v1.0.0
docker pull terraphim/terraphim-cli:v1.0.0
```

**Dockerfile example:**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /build
COPY . .
RUN cargo build --release -p terraphim_tui --features repl-full

FROM debian:bookworm-slim
COPY --from=builder /build/target/release/terraphim-repl /usr/local/bin/
ENTRYPOINT ["terraphim-repl"]
```

## ✅ Success Criteria

### Library Release
- [x] **Published to crates.io**: All 3 crates available
- [ ] **Documentation complete**: README, rustdoc, examples for each
- [ ] **WASM working**: terraphim_automata WASM build succeeds
- [ ] **Examples tested**: All code examples compile and run
- [ ] **Zero warnings**: Clean compilation with no clippy warnings

### REPL Binary
- [ ] **Single binary**: Self-contained, no external dependencies
- [ ] **Offline capable**: Works without network connection
- [ ] **Size optimized**: Binary <50MB (release build)
- [ ] **Cross-platform**: Linux, macOS, Windows binaries
- [ ] **Auto-update works**: Update check and install functional

### CLI Binary
- [ ] **Automation-friendly**: JSON output, proper exit codes
- [ ] **Well-documented**: Help text, man page, examples
- [ ] **Shell completions**: Bash, Zsh, Fish scripts generated
- [ ] **Scriptable**: All commands work non-interactively
- [ ] **Fast**: Sub-second response for simple queries

### Overall
- [ ] **Documentation**: Quick-start works in <5 minutes
- [ ] **Testing**: All unit tests and integration tests passing
- [ ] **CI/CD**: GitHub Actions builds all platforms
- [ ] **Community**: Discord and Discourse announcements posted
- [ ] **Feedback**: Issue templates ready for user feedback

## 🚫 Out of Scope (Future Releases)

**Not included in v1.0.0-minimal:**

### Server Components
- Full HTTP server (`terraphim_server`)
- WebSocket support
- Multi-user authentication
- Rate limiting
- API versioning

### Desktop Application
- Tauri desktop app
- Electron alternative
- Native system integration
- File system watching

### Advanced Integrations
- Haystack providers:
  - Atlassian (Confluence, Jira)
  - Discourse forums
  - JMAP email
  - Notion API
  - Obsidian sync
- LLM integrations:
  - OpenRouter
  - Ollama
  - Local models
- MCP server and tools
- OAuth providers

### Agent System
- Agent supervisor (`terraphim_agent_supervisor`)
- Agent registry
- Multi-agent coordination
- Goal alignment
- Task decomposition
- Agent evolution

### Advanced Features
- Firecracker VM integration
- Redis/RocksDB backends
- Distributed search
- Real-time indexing
- Plugin system
- Custom themes

### Deployment
- Kubernetes manifests
- Terraform configs
- Docker Compose stacks
- Cloud provider integrations

**These features are planned for:**
- v1.1.0 - Server and API
- v1.2.0 - Desktop application
- v2.0.0 - Agent system and advanced features

## 📊 Metrics & Tracking

**Development Metrics:**
- Lines of code: Track for each component
- Test coverage: Target >80% for core libs
- Binary sizes: REPL <50MB, CLI <30MB
- Compile time: Track and optimize
- Documentation coverage: 100% public APIs

**Release Metrics:**
- Downloads per platform
- Crate dependencies (downloads)
- GitHub stars/forks
- Discord/Discourse engagement
- Issue reports and resolutions

**Success Indicators:**
- 100+ downloads in first week
- 5+ community contributions
- <10 critical issues reported
- Positive community feedback

## 🔗 Resources

**Documentation:**
- Main repo: https://github.com/terraphim/terraphim-ai
- Discourse: https://terraphim.discourse.group
- Discord: https://discord.gg/VPJXB6BGuY

**References:**
- Cargo publishing: https://doc.rust-lang.org/cargo/reference/publishing.html
- Rust API guidelines: https://rust-lang.github.io/api-guidelines/
- Keep a Changelog: https://keepachangelog.com/
- Semantic Versioning: https://semver.org/

**Tools:**
- cargo-release: Automated release workflow
- cargo-deny: Dependency checking
- cargo-audit: Security auditing
- wasm-pack: WASM packaging

---

**Last Updated:** 2025-01-22
**Status:** Planning Complete, Ready for Implementation
**Next Review:** After Phase 1 completion
