# Changelog

All notable changes to Terraphim AI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive Ollama integration for local LLM support
  - Chat completion functionality with local models
  - Document summarization using local models
  - Support for qwen, llama, and other open-source models
  - Privacy-first AI features without data leaving your machine
- Extract paragraphs functionality for knowledge graph text processing
- Multi-term search with logical operators (AND/OR)
- Enhanced API endpoints for LLM operations
- Improved test coverage for desktop and core functionality

### Changed
- Fixed compilation errors in desktop Tauri tests
- Improved thesaurus loading to handle empty datasets gracefully
- Enhanced search query processing for multi-term queries
- Updated CI/CD infrastructure with GitHub Actions migration
- Improved error handling in test environments

### Fixed
- Resolved Matched struct field access patterns in tests
- Fixed thesaurus method name inconsistencies (count() → len())
- Corrected tokio::join! timeout patterns in async tests
- Fixed SearchQuery.get_all_terms() to include primary search term
- Addressed temporary value lifetime issues in test code
- Resolved service mutability errors in comprehensive tests

### Technical
- Migrated from Earthly to GitHub Actions + Docker Buildx for CI/CD
- Added multi-platform build support (linux/amd64, linux/arm64, linux/arm/v7)
- Improved Docker layer optimization for faster builds
- Enhanced local testing with nektos/act integration
- Updated test fixtures and knowledge graph test data

## [0.1.0] - Previous Release

### Added
- Initial release of Terraphim AI
- Semantic search across multiple knowledge repositories
- Knowledge graph system with custom graph-based search
- Desktop application with Svelte frontend and Tauri integration
- Support for multiple data sources (Ripgrep, AtomicServer, ClickUp, Logseq)
- Role-based configuration system
- Multiple relevance functions (TitleScorer, BM25, TerraphimGraph)
- Persistence layer with multi-backend storage
- HTTP API server with comprehensive endpoints
- MCP (Model Context Protocol) integration
- OpenRouter integration for cloud AI models
