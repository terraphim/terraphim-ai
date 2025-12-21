# Summary: README.md

## Purpose
User-facing documentation for Terraphim AI - a privacy-first AI assistant.

## v1.0.0 Release Highlights
- **Packages Available**:
  - Rust: `cargo install terraphim-repl` / `cargo install terraphim-cli`
  - Node.js: `npm install @terraphim/autocomplete`
  - Python: `pip install terraphim-automata`
- **Lightweight**: 15 MB RAM, 13 MB disk, <200ms operations

## Key Features
- Semantic knowledge graph search
- Smart text linking (markdown/html/wiki)
- Offline-capable with embedded defaults
- Auto-update system with GitHub Releases

## Installation Methods
- **Homebrew**: `brew install terraphim/terraphim-ai/terraphim-ai`
- **Debian/Ubuntu**: dpkg packages
- **Docker**: `docker run ghcr.io/terraphim/terraphim-server:latest`
- **Direct Download**: GitHub Releases

## Terminology
- **Haystack**: Data source (folder, Notion, email, etc.)
- **Knowledge Graph**: Structured entity-relationship graph
- **Role**: User profile with search preferences
- **Rolegraph**: Knowledge graph with Aho-Corasick scoring

## Claude Code Integration
- Text replacement via hooks and skills
- Codebase quality evaluation with deterministic KG assessment
- CI/CD ready quality gates

## Contributing
- Follow Conventional Commits
- Run `./scripts/install-hooks.sh` for code quality tools
- Pinned dependencies: wiremock=0.6.4, schemars=0.8.22, thiserror=1.0.x
