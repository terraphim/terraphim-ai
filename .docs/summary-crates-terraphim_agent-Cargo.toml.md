# Summary: crates/terraphim_agent/Cargo.toml

## Purpose
CLI/TUI application crate providing the `terraphim-agent` binary - the primary user-facing interface for Terraphim AI.

## Key Details

### Binary
- `terraphim-agent` - Main CLI binary with REPL, TUI, and command modes

### Feature Flags (Complex)
- **default**: `repl-interactive`, `llm`, `repl-sessions`
- **server**: HTTP client support (`reqwest`, `urlencoding`)
- **llm**: LLM provider integration via `terraphim_service`
- **repl**: Basic REPL (`rustyline`, `colored`, `comfy-table`)
- **repl-interactive**: Interactive REPL mode
- **repl-full**: All REPL features + chat + MCP + file + custom + web + sessions
- **repl-chat**: Chat with AI
- **repl-mcp**: MCP server integration
- **repl-file**: File operations
- **repl-custom**: Custom commands
- **repl-web**: Web operations
- **repl-web-advanced**: Advanced web features
- **firecracker**: Firecracker microVM support
- **update-tests**: Self-update testing
- **repl-sessions**: Session search (`terraphim_sessions`)
- **enrichment**: Session enrichment
- **shared-learning**: Feedback loop and KG integration
- **grepapp**: Grep.app integration
- **jmap**: JMAP email search
- **cross-agent-injection**: Cross-agent learning

### Key Dependencies
- `ratatui` (0.30) - Terminal UI framework
- `crossterm` (0.29) - Cross-platform terminal control
- `clap` (v4) - CLI argument parsing
- `rustyline` (optional) - Line editing for REPL
- `pulldown-cmark` - Markdown parsing
- `dialoguer` - Interactive CLI prompts

### Dev Dependencies
- `assert_cmd` - CLI testing
- `proptest` - Property-based testing
- `insta` - Snapshot testing
- `wiremock` (0.6) - HTTP mocking
- `terraphim_test_utils` - Shared test utilities

### Important
- Feature `server` must NOT be enabled when building for server mode (enforced by CI script `scripts/ci-guard-terraphim-agent-server-mode.sh`)
- Self-updating binary via `terraphim_update` crate
