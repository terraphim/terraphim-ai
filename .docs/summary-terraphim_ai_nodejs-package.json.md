# Summary: terraphim_ai_nodejs/package.json

## Purpose
Node.js native bindings package (`@terraphim/autocomplete`) exposing Rust functionality via NAPI-RS.

## Key Details

### Package Info
- Name: `@terraphim/autocomplete`
- Version: `1.3.1`
- Registry: GitHub Package Registry (`https://npm.pkg.github.com`)

### Build System
- Uses `@napi-rs/cli` for native addon compilation
- Platform-specific `.node` binaries for multiple targets
- Supports both Node.js and Bun runtimes

### Supported Platforms
- `x86_64-unknown-linux-gnu`
- `aarch64-apple-darwin` (macOS ARM64)
- `aarch64-unknown-linux-gnu` (Linux ARM64)
- `aarch64-pc-windows-msvc`
- `x86_64-pc-windows-msvc`
- `universal-apple-darwin` (macOS universal)

### Scripts
- `build`: `napi build --platform --release`
- `build:debug`: `napi build --platform`
- `test`: Node.js tests (autocomplete + knowledge graph + MS Teams SDK)
- `test:bun`: Bun runtime tests
- `prepublishOnly`: `napi prepublish -t npm`
- `universal`: `napi universal` (universal binary)

### Test Timeout
- AVA timeout: 3 minutes

### Native Addon Pattern
- Main entry: `index.js`
- Types: `index.d.ts`
- Native binary loaded based on platform

### Optional Platform Packages
Published as separate packages:
- `@terraphim/autocomplete-darwin-arm64`
- `@terraphim/autocomplete-darwin-universal`
- `@terraphim/autocomplete-linux-arm64-gnu`
- `@terraphim/autocomplete-linux-x64-gnu`
- `@terraphim/autocomplete-win32-arm64-msvc`
- `@terraphim/autocomplete-win32-x64-msvc`
