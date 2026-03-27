# Cargo.toml Summary

## Purpose
Defines the workspace configuration and dependencies for the Terraphim AI project, managing multiple crates and external dependencies.

## Key Functionality
- Configures a Rust workspace with multiple member crates
- Sets up workspace-level dependencies (tokio, reqwest, serde, etc.)
- Defines profile configurations for different build scenarios (release, ci, etc.)
- Specifies excluded crates (experimental, Python bindings, desktop-specific)
- Applies patches for specific dependencies (genai, self_update)

## Important Details
- Workspace includes crates/, terraphim_server, terraphim_firecracker, terraphim_ai_nodejs
- Excludes experimental crates like terraphim_agent_application, terraphim_truthforge, etc.
- Uses edition 2024 and version 1.14.0
- Configured for async/await with tokio runtime
- Features conditional compilation via cfg attributes
- Manages cross-platform builds with specific exclusions