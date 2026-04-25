# Summary: crates/terraphim_agent/src/main.rs (Exit Code Handling)

**Purpose**: CLI entry point for terraphim-agent with TUI, robot mode, and server integration.

**Exit Code Path (Task #860)**:
- Module structure: `robot` module contains typed `ExitCode` enum
- Listen mode (`--server` flag) now uses `ErrorUsage` exit code when rejecting standalone mode
- Previously used bare `process::exit(1)`, now uses typed `exit_codes::ExitCode::ErrorUsage(2)`
- Maintains consistency with all other error sites that use the typed exit code mapping

**Key Modules**:
- `robot`: Exit codes and robot-mode CLI output
- `forgiving`: Error tolerance mode for command parsing
- `listener`: Event handling
- `service`: TUI service and business logic
- `client`: Server API communication (feature-gated: `server`)
- `repl`: Interactive mode (feature-gated: `repl`)
- `learnings`, `kg_validation`: Command error capture and KG-based validation

**Notable Functions**:
- `truncate_snippet()`: UTF-8-safe string truncation for output formatting

**Recent Change (Commit 4f9beed1)**:
- Listen mode now uses typed `ExitCode::ErrorUsage` instead of bare `process::exit(1)`
- Improves consistency with error handling throughout the codebase

**Design Pattern**: Modular architecture with feature flags, error propagation via typed exit codes, async runtime using tokio.
