# Design: Issue #1052 -- Fix listen --server clap rejection

## Change Summary

Add `server: bool` argument to `Command::Listen` so clap accepts it, then reject it in application logic with the existing custom error message.

## Implementation Plan

### Step 1: Modify Command::Listen variant

File: `crates/terraphim_agent/src/main.rs`

Change:
```rust
    Listen {
        /// Agent identity/name for this listener instance
        #[arg(long, required = true)]
        identity: String,
        /// Optional listener configuration JSON file
        #[arg(long)]
        config: Option<String>,
    },
```

To:
```rust
    Listen {
        /// Agent identity/name for this listener instance
        #[arg(long, required = true)]
        identity: String,
        /// Optional listener configuration JSON file
        #[arg(long)]
        config: Option<String>,
        /// Start in server mode (rejected -- listen is offline-only)
        #[arg(long)]
        server: bool,
    },
```

### Step 2: Update handler match arm

Change:
```rust
        Some(Command::Listen { identity, config }) => {
            // Listen mode is offline-only - reject --server flag
            if cli.server {
```

To:
```rust
        Some(Command::Listen { identity, config, server }) => {
            // Listen mode is offline-only - reject --server flag
            if server {
```

## Verification Steps

1. Run the specific failing test:
   ```bash
   cargo test -p terraphim_agent --test exit_codes_integration_test listen_mode_with_server_flag_exits_error_usage
   ```

2. Run all exit code tests:
   ```bash
   cargo test -p terraphim_agent --test exit_codes_integration_test
   ```

3. Run clippy:
   ```bash
   cargo clippy --workspace -- -D warnings
   ```

4. Check formatting:
   ```bash
   cargo fmt --all -- --check
   ```

## Risk Assessment

- **Low risk**: Only adds an accepted argument that is immediately rejected. No functional behaviour change.
- **No breaking changes**: Existing `listen` invocations without `--server` continue to work identically.
- **Test coverage**: The existing integration test validates the fix.
