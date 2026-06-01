# Summary: .config/nextest.toml

## Purpose
Configuration for `cargo nextest` test runner.

## Key Details

### Default Profile
- `fail-fast = false` - Run all tests even if some fail
- `slow-timeout = { period = "60s", terminate-after = 3 }` - Kill tests hanging >3 minutes

### CI Profile
- `test-threads = 4` - Limit parallelism in CI

### Usage in CI
```bash
cargo nextest run --workspace --profile ci --lib --bins --features zlob
```

### Installation
```bash
cargo install cargo-nextest --locked
```
