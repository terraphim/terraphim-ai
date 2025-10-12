# Technology Stack

## Backend (Rust)
- **Language**: Rust 1.75.0+ (2021 edition)
- **Web Framework**: Axum with tower-http middleware
- **Async Runtime**: Tokio with full features
- **Serialization**: Serde with JSON support
- **HTTP Client**: Reqwest with rustls-tls
- **Logging**: env_logger with structured logging support

## Frontend
- **Framework**: Svelte 3.x with TypeScript
- **Desktop**: Tauri 1.x for native app packaging
- **Build Tool**: Vite for development and bundling
- **CSS Framework**: Bulma with Bulmaswatch themes
- **Package Manager**: Yarn (preferred) or npm
- **Testing**: Vitest for unit tests, Playwright for E2E

## Storage Backends
- **Default**: In-memory, DashMap, SQLite, ReDB (no setup required)
- **Optional**: RocksDB, Redis, AWS S3 (cloud deployments)
- **Configuration**: Automatic fallback to local storage

## Development Tools
- **Code Quality**: Pre-commit hooks with cargo fmt, clippy, Biome
- **Commit Format**: Conventional Commits (enforced)
- **Hook Managers**: Support for pre-commit, prek, lefthook, or native Git hooks
- **Build System**: Cargo workspace with custom build configuration

## Common Commands

### Backend Development
```bash
# Start server
cargo run

# Run specific binary
cargo run --bin terraphim-tui

# Run tests
cargo test --workspace
cargo test -p terraphim_service

# Format and lint
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
```

### Frontend Development
```bash
cd desktop

# Install dependencies
yarn install

# Development server
yarn run dev

# Desktop app development
yarn run tauri dev

# Build for production
yarn run build
yarn run tauri build

# Testing
yarn test              # Unit tests
yarn run e2e          # End-to-end tests
yarn test:coverage    # Coverage report
```

### Project Setup
```bash
# Install development hooks
./scripts/install-hooks.sh

# Setup role-specific configurations
./scripts/setup_rust_engineer.sh
./scripts/setup_system_operator.sh
```

## Architecture Patterns
- **Workspace Structure**: Multi-crate Rust workspace with clear separation
- **Service Layer**: Dedicated service crates for different concerns
- **Configuration**: TOML-based with environment variable overrides
- **Error Handling**: thiserror for structured error types
- **Async/Await**: Tokio-based async throughout the stack
