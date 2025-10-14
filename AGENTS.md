# Terraphim AI - Agent Development Guide

## Build/Lint/Test Commands

### Rust Backend
```bash
# Build all workspace crates
cargo build --workspace

# Run single test
cargo test -p <crate_name> <test_name>

# Run tests with features
cargo test --features openrouter
cargo test --features mcp-rust-sdk

# Format and lint
cargo fmt
cargo clippy
```

### Frontend (Svelte)
```bash
cd desktop
yarn install
yarn run dev          # Development server
yarn run build        # Production build
yarn run check        # Type checking
yarn test             # Unit tests
yarn e2e              # End-to-end tests
```

## Code Style Guidelines

### Rust
- Use `tokio` for async runtime with `async fn` syntax
- Snake_case for variables/functions, PascalCase for types
- Use `Result<T, E>` with `?` operator for error handling
- Prefer `thiserror`/`anyhow` for custom error types
- Use `dyn` keyword for trait objects (e.g., `Arc<dyn StateManager>`)
- Remove unused imports regularly
- Feature gates: `#[cfg(feature = "openrouter")]`

### Frontend
- Svelte with TypeScript, Vite build tool
- Bulma CSS framework (no Tailwind)
- Use `yarn` package manager
- Component naming: PascalCase
- File naming: kebab-case

### General
- Never use `sleep` before `curl` (Cursor rule)
- Commit only relevant changes with clear technical descriptions
- All commits must pass pre-commit checks (format, lint, compilation)
- Use structured concurrency with scoped tasks
- Implement graceful degradation for network failures
