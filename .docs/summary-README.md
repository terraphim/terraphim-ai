# Summary: README.md

## Purpose
Main project documentation for Terraphim AI, a privacy-first AI assistant that operates locally. Provides installation instructions, quick start guide, and project overview for end users and developers.

## Key Functionality
- **Project Vision**: Privacy-first local AI assistant for searching across personal, team, and public knowledge repositories
- **Installation Methods**: Docker, Homebrew, Debian/Ubuntu packages, direct download, development setup
- **Quick Start**: Automated installation scripts, manual setup instructions
- **TUI Interface**: Comprehensive terminal UI with REPL, sub-2 second VM boot, markdown command system
- **Multiple Interfaces**: Backend server, web frontend (Svelte), desktop app (Tauri), terminal UI

## Critical Components
- **Haystack**: Data sources (folders, Notion, email, Jira, Confluence)
- **Knowledge Graph**: Structured information with node/edge relationships
- **Role**: User profiles with specific knowledge domains and search preferences
- **Rolegraph**: Knowledge graph using Aho-Corasick automata for ranking

## Installation Options

### 🎉 v1.0.0 Multi-Language Packages

**🦀 Rust (crates.io)**:
```bash
cargo install terraphim_agent
terraphim-agent --help
```

**📦 Node.js (npm)**:
```bash
npm install @terraphim/autocomplete
# or with Bun
bun add @terraphim/autocomplete
```

**🐍 Python (PyPI)**:
```bash
pip install terraphim-automata
```

### Traditional Installation
- **Docker**: `docker run ghcr.io/terraphim/terraphim-server:latest`
- **Homebrew**: `brew install terraphim/terraphim-ai/terraphim-ai`
- **Development**: `git clone && cargo run`

## Development Setup
1. Clone repository
2. Install pre-commit hooks: `./scripts/install-hooks.sh`
3. Start backend: `cargo run`
4. Start frontend: `cd desktop && yarn run dev` (web) or `yarn run tauri dev` (desktop)
5. TUI: `cargo build -p terraphim_tui --features repl-full --release && ./target/release/terraphim-agent`

## Important Details
- Storage backends: Local by default (memory, dashmap, sqlite, redb); optional AWS S3 for cloud
- No cloud dependencies required for local development
- Dependency constraints enforced: wiremock 0.6.4, schemars 0.8.22, thiserror 1.0.x
- Code quality: Conventional Commits, cargo fmt, Biome for JS/TS, no secrets in commits
- License: Apache 2.0
- Trademark: Terraphim registered in UK, US, and internationally (WIPO)

## Key Features (TUI)
- AI Chat Integration (OpenRouter, Ollama)
- Sub-500ms VM allocation
- Firecracker microVM pools
- Markdown command system (YAML frontmatter)
- Security-first execution modes (Local, Firecracker, Hybrid)
- Knowledge graph validation before execution
