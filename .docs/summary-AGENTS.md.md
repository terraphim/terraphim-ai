# Summary: AGENTS.md

**Purpose:** Agent instruction file for OpenCode sessions.

**Key Details:**
- Mandatory `/init` command: Step 1 (individual summaries), Step 2 (comprehensive summary)
- Workspace structure with member/excluded crates listed
- Key entrypoints: `terraphim_server/src/main.rs`, `terraphim_agent/src/main.rs`, `desktop/`
- Build commands: `cargo build --workspace`, `cargo check`, `cargo test`
- Frontend: Yarn-based, Vite dev server, Biome lint (not ESLint)
- Pre-commit: `cargo fmt`, `cargo clippy`, Biome check, `detect-secrets`
- Code style: tokio async, snake_case, Result<T,E>, thiserror/anyhow, no mocks
- Gitea task management with `gtr` CLI
- Session completion checklist (must push to remote)
- Constraints: no timeout command (macOS), no sleep before curl, no mocks, use tmux
