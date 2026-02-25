# Desktop Extraction + Crate Dependency Minimization Review (2026-02-25)

## Scope
- Review Rust workspace/crates architecture for desktop coupling.
- Ensure backend/server crates are buildable without `desktop/src-tauri` as a workspace member.
- Reduce direct crate dependencies where low-risk removals are validated.

## Method
- Enumerated workspace members with `cargo metadata`.
- Searched for desktop coupling (`desktop/src-tauri`, `../desktop/dist`, `terraphim-ai-desktop`) across Cargo manifests, server build paths, tests, scripts, and Earthly build files.
- Ranked workspace crates by direct dependency counts.
- Applied only low-risk dependency removals validated by symbol-usage search + `cargo check`/targeted tests.

## Key Findings
1. Hard runtime/build coupling existed in `terraphim_server` via `../desktop/dist` embed/build path.
2. Desktop was still a workspace member, pulling Tauri crate graph into root workspace.
3. MCP integration tests attempted to build desktop crate from this repository.
4. Earthly pipelines imported and built `desktop` directly.

## Implemented Changes

### A) Workspace/Desktop Decoupling
- Removed desktop crate from workspace members.
  - `Cargo.toml`

### B) Server Asset Decoupling (`terraphim_server`)
- Switched embedded assets source from `../desktop/dist` to `dist`.
  - `terraphim_server/src/lib.rs`
- Rewrote build script to use only `terraphim_server/dist` (or `TERRAPHIM_SERVER_DIST`) and generate placeholder `index.html` if missing.
  - `terraphim_server/build.rs`
- Removed no-longer-needed build deps.
  - `terraphim_server/Cargo.toml` (`dircpy`, `walkdir` removed)

### C) Desktop Test Coupling Removal (MCP tests)
- Converted desktop integration tests to external-binary model via `TERRAPHIM_DESKTOP_BINARY`.
- If binary is unavailable, desktop-specific tests now skip gracefully instead of building in-repo desktop package.
  - `crates/terraphim_mcp_server/tests/desktop_mcp_integration.rs`
  - `crates/terraphim_mcp_server/tests/mcp_rolegraph_validation_test.rs`
  - `crates/terraphim_mcp_server/tests/test_mcp_fixes_validation.rs`

### D) Build Pipeline Decoupling
- Removed Earthly desktop imports/build steps and desktop artifact copy references.
  - `Earthfile`
  - `terraphim_server/Earthfile`

### E) Script Decoupling
- CI helper scripts now rely on `terraphim_server/dist` only.
  - `scripts/ci-check-rust.sh`
  - `scripts/ci-check-tests.sh`
- Frontend check script now exits successfully when desktop repo is absent (externalized).
  - `scripts/ci-check-frontend.sh`
- Desktop MCP test helper message updated to external binary guidance.
  - `scripts/test_mcp_servers.sh`

### F) Dependency Minimization (Low Risk)
- `crates/terraphim_agent/Cargo.toml`
  - removed: `jiff`, `handlebars`, `walkdir`, `console`, `indicatif`
  - updated feature `repl` to remove `dep:indicatif`
- `terraphim_server/Cargo.toml`
  - removed: `axum-extra`, `tokio-stream`, `tower`, `url`
  - moved `reqwest` from dependencies to dev-dependencies
- `crates/terraphim_service/Cargo.toml`
  - removed: `futures-util`, `async-stream`
- `crates/terraphim_mcp_server/Cargo.toml`
  - removed: `terraphim_update`, `tracing-appender`, `tracing-log`

## Verification Evidence
- `cargo fmt` ✅
- `cargo check -p terraphim_server -p terraphim_mcp_server -p terraphim_service -p terraphim_agent` ✅
- `cargo test -p terraphim_mcp_server --test desktop_mcp_integration -- --nocapture` ✅
- `cargo test -p terraphim_mcp_server --test test_mcp_fixes_validation -- --nocapture` ✅
- `cargo check --workspace` ⚠️ failed due to disk exhaustion (`No space left on device`), not due to compile errors in changed packages.

## Remaining Work (Not Yet Changed)
These still reference desktop in active CI workflows and should be migrated/split next:
- `.github/workflows/ci-pr.yml`
- `.github/workflows/ci-main.yml`
- `.github/workflows/test-ci.yml`
- `.github/workflows/ci-optimized.yml`
- `.github/workflows/frontend-build.yml`
- `.github/workflows/docker-multiarch.yml`
- `.github/workflows/release.yml`
- `.github/workflows/release-comprehensive.yml`

## Current State Conclusion
- Rust workspace and core server crates are now structurally decoupled from in-repo desktop crate membership.
- Server embedded assets no longer depend on `../desktop/dist`.
- Test/build paths now support an external desktop repository via explicit binary handoff.
- Additional CI workflow migration is required for full repository-level desktop externalization.
