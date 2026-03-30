# Terraphim AI Project Summary

## Overview
Terraphim AI is a Rust-based AI agent system featuring a modular architecture with multiple crates, providing both online (server-connected) and offline (embedded) capabilities for knowledge work, document processing, and AI-assisted tasks.

## Architecture
- **Workspace Structure**: Cargo workspace with multiple member crates including `terraphim_agent`, `terraphim_server`, `terraphim_firecracker`, and `terraphim_ai_nodejs`
- **Modular Design**: Separation of concerns between client communication, service logic, and core functionality
- **Async Runtime**: Built on Tokio for asynchronous operations throughout the codebase
- **Configuration System**: Multi-layer config loading with priority: CLI flags → settings.toml → persistence → embedded defaults

## Key Components

### Communication Layer (`terraphim_agent/src/client.rs`)
- HTTP client for server API communication
- Features configurable timeouts and user agent
- Role resolution that falls back to creating RoleName from raw string when not found in server config
- Supports chat, document summarization, thesaurus access, autocomplete, and VM management operations

### Service Layer (`terraphim_agent/src/service.rs`)
- TUI service managing application state and business logic
- Coordinates between configuration persistence and core TerraphimService
- Role resolution that returns error when role not found in config (contrasts with client.rs)
- Provides thesaurus-backed text operations, search, extraction, summarization, and connectivity checking
- Implements bootstrap-then-persistence pattern for role configuration

### Dependencies & Tooling
- Core dependencies: tokio, reqwest, serde, thiserror, anyhow, async-trait
- Conditional compilation via cfg attributes for feature flags
- Cross-platform build management with specific exclusions
- Development tooling includes Clippy, Rustfmt, and custom validation scripts

## Notable Patterns & Characteristics
1. **Role Resolution Inconsistency**: 
   - Online mode (client.rs): Falls back to raw role string when not found in config
   - Offline mode (service.rs): Returns error when role not found in config
   - This creates different behavior between connected and disconnected states

2. **Configuration Loading**: Sophisticated multi-source configuration system with fallback hierarchy

3. **Error Handling**: Consistent use of `anyhow::Result` and `thiserror` for error propagation

4. **Async/Await**: Extensive use of asynchronous patterns with proper timeout handling

5. **Testing Approach**: Emphasis on integration testing with real services rather than mocks

## Current Focus Areas
Based on recent documentation and code review activities:
- Cross-mode consistency between online and offline operations
- Role resolution and validation improvements
- Test coverage and compilation fixes
- Documentation maintenance and accuracy
- Performance optimization and benchmarking

## Build & Development
- Standard Rust toolchain with cargo workspace
- Features: openrouter, mcp-rust-sdk for conditional compilation
- Profile configurations for release, CI, and LTO-optimized builds
- Pre-commit hooks for code quality assurance