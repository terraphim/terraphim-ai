# Summary: Cargo.toml

## Purpose
This is the root Cargo.toml file for the Terraphim AI workspace, defining the multi-crate project structure, dependencies, and build configuration.

## Key Functionality
- Defines a Rust workspace with multiple crates under `crates/*`, `terraphim_server`, and `desktop/src-tauri`
- Excludes experimental crate `terraphim_agent_application` from builds
- Sets default members to `terraphim_server` for focused development
- Specifies Rust edition 2024 for all workspace members
- Provides shared workspace dependencies including Tokio for async runtime, Reqwest for HTTP, Serde for serialization, and error handling libraries

## Important Details
- Uses resolver version 2 for dependency resolution
- Includes OpenRouter AI integration dependencies in workspace scope
- Features like `openrouter`, `mcp-rust-sdk` can be enabled for specific functionality
- Establishes common async and serialization patterns across the entire project