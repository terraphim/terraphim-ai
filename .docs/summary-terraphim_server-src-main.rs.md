# Summary: terraphim_server/src/main.rs

**Purpose:** HTTP API server entrypoint. Default workspace binary.

**Key Details:**
- Axum-based HTTP server with role-based deployment support
- CLI args: `--role` (default: "TerraphimEngineer"), `--config` (custom config path), `--check-update`, `--update`
- Auto-update feature currently disabled (terraphim_update not in workspace)
- Server initialization: loads config, builds rolegraphs, starts axum server
- Uses `terraphim_service::logging::init_logging()` for logging setup
- Returns `anyhow::Result<()>` with structured error handling
- Main routes served via `terraphim_server::axum_server()`
- Embedded assets feature for serving static files
