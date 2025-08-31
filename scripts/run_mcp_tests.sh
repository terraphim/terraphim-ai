#!/usr/bin/env bash
set -euo pipefail

# Build the project (all crates) in debug mode
cargo build --workspace

# Run only the MCP server integration tests with backtrace & logs
RUST_BACKTRACE=1 RUST_LOG=debug \
  cargo test -p terraphim_mcp_server --test integration_test -- --nocapture
