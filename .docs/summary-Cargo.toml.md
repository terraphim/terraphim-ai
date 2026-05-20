# Summary: Cargo.toml

## Purpose

Root workspace configuration for the Terraphim AI project.

## Workspace Structure

```toml
[workspace]
resolver = "2"
members = ["crates/*", "terraphim_server", "terraphim_firecracker", "terraphim_ai_nodejs", "infrastructure/*"]
```

## Excluded Crates

- `terraphim_agent_application`, `terraphim_truthforge`, `terraphim_automata_py` (experimental)
- `terraphim_rolegraph_py` (needs `maturin develop`)
- `desktop/src-tauri` (built separately via Tauri CLI)
- `terraphim_repl` (superseded by terraphim_agent)
- `terraphim_symphony` (build separately)
- `terraphim_github_runner`, `terraphim_github_runner_server` (private git dependency)
- `terraphim_lsp` (missing Cargo.toml)
- Infrastructure templates (vm-templates, rust-cache-stack, etc.)

## Workspace Package

```toml
[workspace.package]
version = "1.19.2"
edition = "2024"
```

## Key Dependencies

- **Async Runtime**: tokio 1.0 (full features)
- **HTTP**: reqwest 0.12 (rustls-tls), reqwest-middleware 0.4, reqwest-retry 0.7
- **Serialization**: serde 1.0, serde_json 1.0
- **Utilities**: uuid 1.21, chrono 0.4, async-trait 0.1, thiserror 1.0, anyhow 1.0
- **Logging**: log 0.4, tracing 0.1
- **Security**: rustls 0.23+, rustls-webpki 0.103.12+

## Patched Dependencies

- `genai`: Git branch `merge-upstream-20251103`
- `self_update`: Git branch `update-zipsign-api-v0.2`
- `tokio-tungstenite`: Tag `v0.28.0` (for rustls 0.23+)
- `rustls-webpki`: Tag `v/0.103.12` (RUSTSEC-2026-0049 fix)

## Build Profiles

```toml
[profile.release]
panic = "unwind"
lto = false
codegen-units = 1
opt-level = 3

[profile.release-lto]
inherits = "release"
lto = true
panic = "abort"

[profile.ci]
inherits = "dev"
incremental = false
codegen-units = 16
split-debuginfo = "off"
debug = false
strip = true

[profile.ci-release]
inherits = "release"
lto = "thin"
codegen-units = 8
```

## Default Members

```toml
default-members = ["terraphim_server"]
```

Only `terraphim_server` is built by default; full workspace requires `cargo build --workspace`.