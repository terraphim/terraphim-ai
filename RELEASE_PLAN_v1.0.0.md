# Terraphim AI v1.0.0 Release Plan

## Overview

This document outlines the comprehensive release plan for Terraphim AI v1.0.0, focusing on publishing the renamed `terraphim_agent` package and coordinating the release of core dependency crates.

## Major Changes in v1.0.0

### ✅ Completed Changes

1. **Package Rename**: `terraphim-tui` → `terraphim-agent`
   - Package name: `terraphim_tui` → `terraphim_agent`
   - Binary name: `terraphim-tui` → `terraphim-agent`
   - All CI/CD workflows updated
   - All documentation updated
   - All build scripts updated

2. **Core Infrastructure**
   - All tests compile successfully
   - Binary functionality verified working
   - Dependencies properly configured

## Publishing Strategy

### Dependency Hierarchy

The following crates must be published in this specific order due to dependencies:

1. **terraphim_types** (v1.0.0) - Foundation types
2. **terraphim_settings** (v1.0.0) - Configuration management
3. **terraphim_persistence** (v1.0.0) - Storage abstraction
4. **terraphim_config** (v1.0.0) - Configuration layer
5. **terraphim_automata** (v1.0.0) - Text processing and search
6. **terraphim_rolegraph** (v1.0.0) - Knowledge graph implementation
7. **terraphim_middleware** (v1.0.0) - Search orchestration
8. **terraphim_service** (v1.0.0) - Main service layer
9. **terraphim_agent** (v1.0.0) - CLI/TUI/REPL interface ⭐

### Publishing Commands

#### Option 1: Automated CI/CD Publishing (Recommended)

1. **Set up GitHub Secrets** (see `docs/github-secrets-setup.md`):
   - Add `ONEPASSWORD_SERVICE_ACCOUNT_TOKEN` from 1Password service account
   - Ensure the service account has access to `op://TerraphimPlatform/crates.io.token/token`

2. **Trigger Publishing Workflow**:
   ```bash
   # Dry run (testing)
   gh workflow run "Publish Rust Crates" --field dry_run=true

   # Live publishing
   gh workflow run "Publish Rust Crates" --field dry_run=false

   # Publish specific crate
   gh workflow run "Publish Rust Crates" --field crate=terraphim_agent --field dry_run=false
   ```

3. **Tag-based Publishing** (automatic):
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

#### Option 2: Manual Local Publishing

1. **Set up token locally**:
   ```bash
   # Use the setup script
   ./scripts/setup-crates-token.sh --update-env
   source .env

   # Or export manually
   export CARGO_REGISTRY_TOKEN=$(op read "op://TerraphimPlatform/crates.io.token/token")
   ```

2. **Publish in dependency order**:
   ```bash
   cargo publish --package terraphim_types
   # Wait for crates.io to process (usually 1-2 minutes)

   cargo publish --package terraphim_settings
   cargo publish --package terraphim_persistence
   cargo publish --package terraphim_config
   cargo publish --package terraphim_automata
   cargo publish --package terraphim_rolegraph
   cargo publish --package terraphim_middleware
   cargo publish --package terraphim_service
   cargo publish --package terraphim_agent
   ```

3. **Verify installation**:
   ```bash
   cargo install terraphim_agent
   terraphim-agent --version
   ```

## Version Updates Required

Before publishing, update all internal dependencies from path references to version references:

```toml
# Example for terraphim_agent/Cargo.toml
[dependencies]
terraphim_types = { version = "1.0.0" }
terraphim_settings = { version = "1.0.0" }
terraphim_persistence = { version = "1.0.0" }
terraphim_config = { version = "1.0.0" }
terraphim_automata = { version = "1.0.0" }
terraphim_service = { version = "1.0.0" }
terraphim_middleware = { version = "1.0.0" }
terraphim_rolegraph = { version = "1.0.0" }
```

## Release Validation Checklist

### Pre-Publishing Validation

- [ ] All crates compile with `cargo check --workspace`
- [ ] All tests pass with `cargo test --workspace --lib`
- [ ] Binary builds successfully: `cargo build --package terraphim_agent --features repl-full --release`
- [ ] Binary runs correctly: `./target/release/terraphim-agent --help`
- [ ] Documentation builds: `cargo doc --workspace --no-deps`
- [ ] All dependencies updated to use version numbers instead of paths
- [ ] CHANGELOG.md updated for v1.0.0
- [ ] Release notes prepared

### Post-Publishing Validation

- [ ] Installation test: `cargo install terraphim-agent`
- [ ] Basic functionality test: `terraphim-agent --help`
- [ ] REPL functionality test: `terraphim-agent repl`
- [ ] Integration tests with published crates
- [ ] Documentation available on docs.rs

## Key Features in v1.0.0

### terraphim_agent

- **CLI Interface**: Full command-line interface with subcommands
- **REPL System**: Interactive Read-Eval-Print Loop with comprehensive commands
- **Search Integration**: Semantic search across multiple haystacks
- **Configuration Management**: Role-based configuration system
- **AI Chat**: LLM integration for conversational AI
- **Knowledge Graph**: Interactive graph visualization and navigation
- **VM Management**: Firecracker microVM integration
- **File Operations**: Semantic file analysis and management
- **Web Operations**: Secure web request handling
- **Custom Commands**: Markdown-defined command system

### Supported Features

- **Multiple AI Providers**: OpenRouter, Ollama, generic LLM interface
- **Multiple Storage Backends**: Memory, SQLite, ReDB, Atomic Data
- **Search Algorithms**: BM25, TitleScorer, TerraphimGraph
- **Security Modes**: Local, Firecracker, Hybrid execution
- **Export Formats**: JSON, Markdown, structured data

## Migration Guide for Users

### Installation

```bash
# Install from crates.io (after publishing)
cargo install terraphim_agent

# Or build from source
cargo install --git https://github.com/terraphim/terraphim-ai terraphim_agent --features repl-full
```

### Breaking Changes

- Binary name changed from `terraphim-tui` to `terraphim-agent`
- Package name changed from `terraphim_tui` to `terraphim_agent`
- Some internal APIs reorganized (not affecting end users)

### Updated Usage

```bash
# Old command (no longer works)
terraphim-tui repl

# New command
terraphim-agent repl
```

## Current Status

### ✅ Completed
- Package rename implementation
- CI/CD workflow updates
- Documentation updates
- Test fixes and compilation validation
- Core functionality verification

### 🔄 In Progress
- Dependency version coordination
- Publishing preparation

### ⏳ Pending
- Acquire crates.io publishing token
- Execute publishing sequence
- Post-publishing validation

## Next Steps

1. **Immediate**: Acquire crates.io token from project maintainers
2. **Short-term**: Execute publishing sequence following dependency hierarchy
3. **Medium-term**: Update project documentation and announce release
4. **Long-term**: Begin v1.1.0 development with remaining PR merges

## Release Notes Draft

### 🚀 terraphim-agent v1.0.0

Major release introducing the renamed and enhanced Terraphim Agent CLI tool.

#### ✨ New Features
- Renamed package from `terraphim-tui` to `terraphim-agent`
- Enhanced CLI interface with comprehensive subcommands
- Full REPL functionality with interactive commands
- Integrated AI chat capabilities
- Advanced search and knowledge graph features
- Secure VM management with Firecracker integration
- Semantic file operations and web operations
- Custom command system defined in Markdown

#### 🔧 Improvements
- Updated all build scripts and CI/CD workflows
- Enhanced test coverage and compilation fixes
- Improved dependency management
- Better error handling and user feedback

#### 🔄 Breaking Changes
- Binary name changed: `terraphim-tui` → `terraphim-agent`
- Package name changed: `terraphim_tui` → `terraphim_agent`

#### 📦 Installation
```bash
cargo install terraphim_agent
```

---

*This release plan will be updated as we progress through the publishing process.*