# Project Structure

## Root Directory Organization

```
terraphim-ai/
├── crates/                    # Rust library crates (workspace members)
├── terraphim_server/         # Main HTTP server binary
├── desktop/                  # Svelte frontend + Tauri desktop app
├── scripts/                  # Development and build scripts
├── docs/                     # Documentation and mdBook
├── browser_extensions/       # Browser extension implementations
├── tests/                    # Integration and E2E tests
└── .kiro/                    # Kiro IDE configuration and steering
```

## Core Crates Structure

### Service Layer
- `terraphim_service/` - Main service logic and request handling
- `terraphim_middleware/` - HTTP middleware and request processing
- `terraphim_persistence/` - Data storage abstractions and implementations

### Domain Logic
- `terraphim_rolegraph/` - Knowledge graph and role-based search
- `terraphim_automata/` - Aho-Corasick automata for text processing
- `terraphim_config/` - Configuration management and validation
- `terraphim_types/` - Shared type definitions

### Specialized Components
- `terraphim_agent/` - Terminal user interface
- `terraphim_mcp_server/` - Model Context Protocol server
- `terraphim_atomic_client/` - Atomic server integration
- `terraphim_settings/` - Settings management

### Agent System (Experimental)
- `terraphim_agent_*` - Agent-based architecture components
- `terraphim_goal_alignment/` - Goal alignment and task decomposition
- `terraphim_task_decomposition/` - Task breakdown system

## Frontend Structure

```
desktop/
├── src/                      # Svelte application source
├── src-tauri/               # Tauri backend (Rust)
├── tests/                   # Frontend tests (Vitest, Playwright)
├── scripts/                 # Frontend build scripts
└── public/                  # Static assets
```

## Configuration Files

### Build and Development
- `Cargo.toml` - Workspace configuration
- `build_config.toml` - Custom build configuration
- `.pre-commit-config.yaml` - Code quality hooks
- `package.json` - Node.js dependencies (root level)

### IDE and Tooling
- `.kiro/steering/` - AI assistant steering rules
- `.vscode/` - VS Code configuration
- `.github/` - GitHub Actions and templates

## Naming Conventions

### Rust Crates
- Prefix: `terraphim_` for all internal crates
- Snake case: `terraphim_service`, `terraphim_config`
- Descriptive: Names reflect primary responsibility

### File Organization
- `src/lib.rs` - Crate entry point with public API
- `src/main.rs` - Binary entry point (for executables)
- `tests/` - Integration tests (separate from unit tests)
- `benches/` - Performance benchmarks

### Configuration Files
- JSON for runtime configuration: `*_config.json`
- TOML for build-time configuration: `*.toml`
- Environment templates: `.env.template`

## Development Workflow Directories

### Scripts Directory
- `scripts/setup_*.sh` - Role-specific setup scripts
- `scripts/test_*.sh` - Testing automation
- `scripts/build_*.sh` - Build automation
- `scripts/hooks/` - Git hook implementations

### Test Organization
- Unit tests: Within each crate's `src/` directory
- Integration tests: `tests/` directory in each crate
- E2E tests: `desktop/tests/` and root `tests/`
- Fixtures: `test-fixtures/` for shared test data

## Binary Outputs
- `terraphim_server` - Main HTTP API server
- `terraphim-agent` - Terminal interface
- `terraphim-mcp-server` - MCP protocol server
- Desktop app - Built via Tauri in `desktop/src-tauri/`
