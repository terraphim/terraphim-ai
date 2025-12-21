# Summary: CLAUDE.md

## Purpose
Project-level instructions for Claude Code providing guidance on Rust async programming, testing, development workflows, and project architecture.

## Key Sections
- **Rust Best Practices**: tokio async runtime, channels (mpsc/broadcast/oneshot), error handling with thiserror/anyhow
- **Testing Guidelines**: Unit tests with `tokio::test`, no mocks, regression coverage
- **Performance Practices**: Profile first, ripgrep-style optimizations, zero-copy types
- **Commit Guidelines**: Conventional commits, must pass fmt/clippy/test
- **Memory Management**: References to memories.md, lessons-learned.md, scratchpad.md
- **Agent Systems**: Superpowers Skills and .agents directory integration
- **Project Overview**: Privacy-first AI assistant with knowledge graphs and semantic search
- **Development Commands**: Build, test, run, watch commands
- **Configuration System**: Role-based config, environment variables, JSON/TOML formats
- **MCP Integration**: Model Context Protocol server with autocomplete tools

## Important Rules
- Never use sleep before curl
- Never use timeout command (doesn't exist on macOS)
- Never use mocks in tests
- Use 1Password for secrets
