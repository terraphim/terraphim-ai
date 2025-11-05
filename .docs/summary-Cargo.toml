# Summary: Cargo.toml

## Purpose
Workspace-level Cargo configuration for the Terraphim AI project. Defines workspace members, shared dependencies, and build settings for the entire multi-crate project.

## Key Functionality
- **Workspace Definition**: Organizes 29+ crates under unified workspace
- **Dependency Management**: Centralizes common dependencies for consistency across crates
- **Build Configuration**: Sets Rust edition 2024 and resolver version 2

## Workspace Structure
- **Members**:
  - `crates/*` - All library crates (29 crates)
  - `terraphim_server` - Main HTTP API server binary
  - `desktop/src-tauri` - Tauri desktop application
  - `terraphim_firecracker` - Firecracker microVM binary
- **Excluded**: `crates/terraphim_agent_application` (experimental with incomplete APIs)
- **Default Member**: `terraphim_server`

## Shared Dependencies
- **Async Runtime**: `tokio` 1.0 with full features
- **HTTP Client**: `reqwest` 0.12 with JSON and rustls-tls (no default features)
- **Serialization**: `serde` 1.0, `serde_json` 1.0
- **UUID**: `uuid` 1.0 with v4 and serde features
- **Time**: `chrono` 0.4 with serde
- **Async Utilities**: `async-trait` 0.1
- **Error Handling**: `thiserror` 1.0, `anyhow` 1.0
- **Logging**: `log` 0.4

## Patches
- **genai**: Patched to use terraphim fork from `https://github.com/terraphim/rust-genai.git` (main branch)

## Critical Details
- Edition 2024: Uses latest Rust edition features
- Resolver 2: Modern dependency resolution algorithm
- Centralized dependencies reduce version conflicts across workspace
- OpenRouter AI integration dependencies grouped together
- Default features disabled for reqwest (explicit feature selection)

## Build Implications
- All crates share same Rust edition and resolver
- Common dependencies version-locked across workspace
- Workspace-level features can be enabled/disabled per crate
- Patch enables custom genai fork without version conflicts
