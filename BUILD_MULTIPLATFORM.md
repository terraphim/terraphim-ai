# Multi-Platform Build Guide for Terraphim AI

## Current Release: v1.0.2

### ✅ Successfully Built & Released

#### macOS (All Architectures)
- **Universal Binary**: `terraphim-ai-v1.0.2-macos-universal.tar.gz` (43MB)
- **ARM64 (Apple Silicon)**: `terraphim-ai-v1.0.2-macos-aarch64.tar.gz` (21MB)
- **x86_64 (Intel)**: `terraphim-ai-v1.0.2-macos-x86_64.tar.gz` (22MB)
- **Desktop App**: `terraphim-desktop-v1.0.2-macos-aarch64.dmg` (11MB)

### 🚧 Platform Build Requirements

#### Linux Builds
Requires Docker or cross-compilation environment:
```bash
# Using Docker
docker build -f Dockerfile.build -t terraphim-linux .
docker run --rm -v $(pwd)/releases:/output terraphim-linux

# Using Earthly (requires Docker daemon)
earthly +build-all

# Using cross (requires Docker daemon)
cross build --release --target x86_64-unknown-linux-gnu
```

#### Windows Builds
Requires MinGW toolchain:
```bash
# Install on macOS
brew install mingw-w64

# Build
cargo build --release --target x86_64-pc-windows-gnu
```

### Build Commands Used

#### Native macOS Build
```bash
cargo build --release \
    --package terraphim_server \
    --package terraphim_mcp_server \
    --package terraphim_tui
```

#### Cross-Platform Targets
```bash
# macOS targets
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Linux targets (requires cross/Docker)
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-gnu

# Windows targets (requires MinGW)
cargo build --release --target x86_64-pc-windows-gnu
```

#### Desktop App (Tauri)
```bash
cd desktop

# Fix svelte-jsoneditor issue first
# Edit node_modules/svelte-jsoneditor/components/modals/TransformWizard.svelte
# Wrap <tr> elements in <tbody>

# Build
yarn tauri build

# Output location
# macOS: target/release/bundle/dmg/
# Linux: target/release/bundle/appimage/ (requires Linux host)
# Windows: target/release/bundle/msi/ (requires Windows host)
```

### Known Issues & Fixes

1. **svelte-jsoneditor Table Structure**
   - Error: "`<tr>` cannot be a child of `<table>`"
   - Fix: Edit `node_modules/svelte-jsoneditor/components/modals/TransformWizard.svelte`
   - Wrap `<tr>` elements with `<tbody>` tags

2. **Atomic Feature Missing**
   - Error: "could not find `__cmd__save_article_to_atomic`"
   - Fix: Comment out the command in `desktop/src-tauri/src/main.rs:378`
   - Or enable with: `--features atomic`

3. **Docker/Earthly Issues**
   - Earthly requires Docker daemon running
   - OrbStack can be used as Docker alternative on macOS
   - Start with: `open -a OrbStack`

4. **Cross-Compilation Dependencies**
   - Linux: Requires Docker or proper toolchain
   - Windows: Requires `mingw-w64` toolchain
   - Install: `brew install mingw-w64` (macOS)

### Release Process

1. Build all binaries
2. Create archives:
   ```bash
   cd releases/v1.0.2/platform/arch
   tar -czf ../terraphim-ai-v1.0.2-platform-arch.tar.gz *
   ```

3. Create GitHub release:
   ```bash
   gh release create v1.0.2 \
     --title "Terraphim AI v1.0.2" \
     --notes-file RELEASE_NOTES.md \
     releases/v1.0.2/**/*.{tar.gz,dmg,zip}
   ```

4. Upload additional artifacts:
   ```bash
   gh release upload v1.0.2 new-artifact.tar.gz
   ```

### CI/CD Recommendations

For future releases, consider:
1. GitHub Actions for automated multi-platform builds
2. Docker buildx for cross-platform container builds
3. cargo-dist for Rust binary distribution
4. Tauri GitHub Action for desktop app builds

### Testing Checklist

Before release, verify:
- [ ] TUI REPL commands work
- [ ] Server starts and responds
- [ ] MCP server functions
- [ ] Desktop app launches
- [ ] Role switching works
- [ ] Search returns results
- [ ] Configuration persists