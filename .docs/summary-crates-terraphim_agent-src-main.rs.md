# Summary: terraphim_agent/src/main.rs

**Purpose:** CLI/TUI agent entrypoint. Interactive terminal interface for Terraphim.

**Key Details:**
- Ratatui-based TUI with crossterm backend
- Subcommand CLI via clap
- Modules: `tui_backend`, `guard_patterns`, `listener`, `onboarding`, `service`, `shell_dispatch`, `forgiving`, `robot`, `learnings`, `kg_validation`
- Optional modules (feature-gated): `client` (server feature), `repl` (repl feature)
- Uses `tokio::runtime::Runtime` for async operations
- Integrates with `terraphim_update` for auto-updates
- Key types: `Document`, `Layer`, `LogicalOperator`, `SearchQuery`
- UTF-8 safe snippet truncation (`truncate_snippet` function)
- Large file (~3959 lines) - main TUI application logic
