# Summary: .github/workflows/ci-pr.yml

**Purpose:** CI pipeline for PR validation.

**Key Details:**
- Triggers: PRs to main/develop branches
- Self-hosted runners: `[self-hosted, Linux, X64]`
- Jobs:
  1. **changes**: Detects Rust/frontend/dockerfile/docs changes using dorny/paths-filter
  2. **build-frontend**: Builds desktop if frontend changed (Node 20, yarn cache)
  3. **rust-format**: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --features zlob -- -D warnings`, compilation check
  4. **frontend-check**: `yarn lint` (allowed failure), `yarn run check` (svelte-check)
  5. **rust-tests**: `cargo test --workspace --lib --bins --features zlob -- --test-threads=2`
  6. **wasm-build**: `./scripts/build-wasm.sh web dev`
  7. **security-audit**: `cargo audit` (continue-on-error)
- Rust toolchain: 1.94.0
- Uses sccache for build acceleration
- Features enabled in CI: `zlob` (for fff-search)
- Cleanup: removes target/, dist/, node_modules between jobs
