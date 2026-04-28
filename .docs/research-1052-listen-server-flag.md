# Research: Issue #1052 -- listen --server argument rejected by clap

## Problem

`terraphim-agent listen --server` fails at the clap parser level before application logic can print a custom error message. The test `listen_mode_with_server_flag_exits_error_usage` expects:
- Exit code 2 (ERROR_USAGE)
- Stderr containing "listen mode does not support --server flag"

But clap rejects `--server` because `Command::Listen` does not declare it as a valid argument.

## Root Cause

`Cli` struct has a top-level `server: bool` flag (line 678). `Command::Listen` only accepts `identity` and `config` (lines 898-905). When `--server` appears after the subcommand name, clap treats it as a subcommand argument and errors because `Listen` has no such field.

## Evidence

### Current Command::Listen definition (main.rs:898-905)
```rust
Listen {
    #[arg(long, required = true)]
    identity: String,
    #[arg(long)]
    config: Option<String>,
},
```

### Current handler (main.rs:1718-1724)
```rust
Some(Command::Listen { identity, config }) => {
    if cli.server {
        eprintln!("error: listen mode does not support --server flag");
        eprintln!("The listener runs in offline mode only.");
        std::process::exit(2);
    }
```

### Pattern from Repl command (main.rs:853-860)
```rust
Repl {
    #[arg(long)]
    server: bool,
    #[arg(long, default_value = "http://localhost:8000")]
    server_url: String,
},
```

## Fix Strategy

Add `server: bool` to `Command::Listen` with `#[arg(long)]`, matching the pattern used by `Command::Repl`. Update the handler to destructure `server` from the `Listen` variant and check the local variable instead of `cli.server`.

## Files to Modify

- `crates/terraphim_agent/src/main.rs`
  - Add `server: bool` field to `Command::Listen`
  - Update match arm at line 1718 to destructure `server`
  - Update check at line 1720 to use local `server` variable

## Acceptance Criteria Verification

- [ ] `cargo test -p terraphim_agent --test exit_codes_integration_test listen_mode_with_server_flag_exits_error_usage` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
