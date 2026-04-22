# Summary: terraphim_server/Cargo.toml

**Purpose:** Server package manifest defining features and dependencies.

**Key Details:**
- Default features: `sqlite`, `embedded-assets`
- **LLM provider features:** `openrouter`, `ollama`, `llm`
- **Database backends:** `sqlite`, `redis`, `s3`, `dashmap`, `redb`, `ipfs`, `full-db`
- **Middleware features:** `grepapp`, `ai-assistant`, `mcp`, `mcp-rust-sdk`
- **Advanced:** `vm-execution`, `workflows`
- **Convenience:** `full` = all features combined
- Dependencies: axum 0.8.7, tower-http 0.6.8, clap 4.5.60, rust-embed (optional)
- Dev deps: `serial_test`, `terraphim_agent`, `axum-test`, `tempfile`
- Key internal deps: `terraphim_persistence`, `terraphim_config`, `terraphim_middleware`, `terraphim_rolegraph`, `terraphim_service`, `terraphim_multi_agent`
