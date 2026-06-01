# Summary: .github/rust-toolchain.toml

## Purpose
Rust toolchain configuration ensuring consistent compiler version across CI/CD.

## Key Details

### Toolchain
- **Channel**: `1.94.0` (stable with edition 2024 support)
- **Components**: `rustfmt`, `clippy`

### Cross-Compilation Targets
- `x86_64-unknown-linux-gnu` - Primary Linux
- `aarch64-unknown-linux-gnu` - ARM64 Linux
- `x86_64-unknown-linux-musl` - Static Linux
- `wasm32-unknown-unknown` - WebAssembly

### Profile Settings
- `codegen-units = 1`
- `lto = true`
- `panic = abort`
- `strip = true`

### Note
This is separate from any `rust-toolchain.toml` at repo root (if exists). The CI explicitly references `.github/rust-toolchain.toml`.
