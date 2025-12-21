# Summary: Cargo.toml

## Purpose
Workspace-level Cargo configuration defining the multi-crate Rust project structure.

## Key Configuration
- **Edition**: Rust 2024
- **Resolver**: Version 2 for optimal dependency resolution
- **Members**: `crates/*`, `terraphim_server`, `desktop/src-tauri`
- **Default Member**: `terraphim_server` (main HTTP API server)
- **Excluded**: `terraphim_agent_application`, `terraphim_truthforge`, `terraphim_automata_py`

## Workspace Dependencies
- **Async**: tokio with full features
- **HTTP**: reqwest with json, rustls-tls
- **Serialization**: serde, serde_json
- **Identity**: uuid v4 with serde
- **Time**: chrono with serde
- **Traits**: async-trait
- **Errors**: thiserror, anyhow
- **Logging**: log

## Patched Dependencies
- `genai`: Custom fork at github.com/terraphim/rust-genai.git (merge-upstream-20251103 branch)

## Release Profiles
- **release**: panic=unwind, lto=false, codegen-units=1, opt-level=3
- **release-lto**: Inherits release with lto=true, panic=abort (production builds)
