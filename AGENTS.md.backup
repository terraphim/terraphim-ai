# Terraphim AI - Agent Development Guide

## Documentation Organization

All project documentation is organized in the `.docs/` folder:
- **Individual File Summaries**: `.docs/summary-<normalized-path>.md` - Detailed summaries of each working file
- **Comprehensive Overview**: `.docs/summary.md` - Consolidated project overview and architecture analysis
- **Agent Instructions**: `.docs/agents_instructions.json` - Machine-readable agent configuration and workflows

## Mandatory /init Command Steps

When user executes `/init` command, you MUST perform these two steps in order:

### Step 1: Summarize Working Files
Can you summarize the working files? Save each file's summary in `.docs/summary-<normalized-path>.md`

- Identify all relevant working files in the project
- Create individual summaries for each file
- Save summaries using the pattern: `.docs/summary-<normalized-path>.md`
- Include file purpose, key functionality, and important details
- Normalize file paths (replace slashes with hyphens, remove special characters)

### Step 2: Create Comprehensive Summary
Can you summarize your context files ".docs/summary-*.md" and save the result in `.docs/summary.md`
- Read all individual summary files created in Step 1
- Synthesize into a comprehensive project overview
- Include architecture, security, testing, and business value analysis
- Save the consolidated summary as `.docs/summary.md`
- Update any relevant documentation references

Both steps are MANDATORY for every `/init` command execution.

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
- Never use `timeout` in command line; this command does not exist on macOS
- Never use mocks in tests
- Use IDE diagnostics to find and fix errors
- Always check test coverage after implementation
- Keep track of all tasks in GitHub issues using the `gh` tool
- Commit every change and keep GitHub issues updated with the progress using the `gh` tool
- Use `tmux` to spin off background tasks, read their output, and drive interaction
- Use `tmux` instead of `sleep` to continue working on a project and then read log output

## Documentation Management

### File Summaries
- Create individual summaries for each working file in `.docs/summary-<normalized-path>.md`
- Include file purpose, key functionality, and important details
- Normalize file paths (replace slashes with hyphens, remove special characters)

### Comprehensive Documentation
- Maintain consolidated overview in `.docs/summary.md`
- Include architecture, security, testing, and business value analysis
- Update documentation references when making changes

### Agent Instructions
- Use `.docs/agents_instructions.json` as primary reference for project patterns
- Contains machine-readable instructions for AI agents
- Includes project context, critical lessons, and established practices
