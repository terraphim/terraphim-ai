# Publishing Node.js Packages

This document explains how to publish the `@terraphim/autocomplete` Node.js package to npm using our CI/CD pipelines with 1Password integration.

## 🚀 Publishing Methods

### 1. Automated Publishing via CI.yml (Simple)

Trigger publishing automatically by committing a semantic version:

```bash
git commit -m "1.0.0"
git push origin main
```

**How it works:**
- The existing `CI.yml` workflow checks if the commit message is a semantic version
- If it matches `[major].[minor].[patch]`, it publishes to npm with the `latest` tag
- Uses existing `NPM_TOKEN` from GitHub Secrets

### 2. Enhanced Publishing via publish-npm.yml (Recommended)

For more control over the publishing process:

```bash
# Create a version tag
git tag nodejs-v1.0.0
git push origin nodejs-v1.0.0
```

**Features:**
- ✅ 1Password integration for secure token management
- ✅ Multi-platform binary building (Linux, macOS, Windows, ARM64)
- ✅ Comprehensive testing before publishing
- ✅ Dry-run mode for testing
- ✅ Custom npm tags (latest, beta, alpha, rc)
- ✅ Automatic GitHub release creation
- ✅ Package verification after publishing

### 3. Manual Publishing via Workflow Dispatch

You can manually trigger publishing from the GitHub Actions tab:

1. Go to Actions → "Publish Node.js Package to npm"
2. Click "Run workflow"
3. Fill in the parameters:
   - **Version**: Semantic version (e.g., `1.0.1`)
   - **Tag**: npm tag (`latest`, `beta`, `alpha`, `rc`)
   - **Dry Run**: Enable for testing without publishing

### 4. WASM Package Publishing

For WebAssembly versions:

```bash
# Create WASM version tag
git tag wasm-v1.0.0
git push origin wasm-v1.0.0
```

This publishes `@terraphim/autocomplete-wasm` with browser support.

## 🔐 Token Management with 1Password

### 1Password Setup

The publishing workflows use 1Password for secure token management:

**1Password Items:**
- `op://TerraphimPlatform/npm.token/token` - Main npm publishing token
- `op://TerraphimPlatform/npm-wasm.token/token` - WASM package token (optional)

### Token Fallback

If 1Password tokens are not available, the workflows fall back to:
- `NPM_TOKEN` (GitHub Secrets) - Main npm token
- `NPM_WASM_TOKEN` (GitHub Secrets) - WASM package token

### Setting up 1Password Tokens

1. Open 1Password and access the "TerraphimPlatform" vault
2. Create/update the `npm.token` item with your npm access token
3. Ensure the token has publishing permissions for the `@terraphim` scope
4. The CI/CD pipeline will automatically fetch and use the token

## 🏗️ Build Process

### Native Package (@terraphim/autocomplete)

**Supported Platforms:**
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-unknown-linux-gnu` (Linux)
- `aarch64-unknown-linux-gnu` (Linux ARM64)
- `x86_64-pc-windows-msvc` (Windows)
- `aarch64-pc-windows-msvc` (Windows ARM64)

**Build Steps:**
1. Multi-platform compilation using NAPI
2. Universal macOS binary creation
3. Cross-platform testing
4. Package assembly with all binaries
5. npm publishing with provenance

### WASM Package (@terraphim/autocomplete-wasm)

**Targets:**
- `wasm32-unknown-unknown` (WebAssembly)
- Node.js and browser compatibility

**Build Steps:**
1. Rust WASM compilation using `wasm-pack`
2. Web and Node.js target builds
3. Browser testing with Puppeteer
4. Package creation with dual exports
5. npm publishing

## 📦 Package Structure

### Native Package

```
@terraphim/autocomplete/
├── index.js                 # Main entry point
├── terraphim_ai_nodejs.*.node # Native binaries (per platform)
├── package.json             # Package metadata
└── README.md               # Documentation
```

### WASM Package

```
@terraphim/autocomplete-wasm/
├── terraphim_automata.js     # Node.js entry
├── terraphim_automata_bg.wasm # WebAssembly binary
├── web/                     # Browser-specific files
│   └── terraphim_automata.js
├── package.json
└── README.md
```

## 🧪 Testing Before Publishing

### Local Testing

```bash
# Build and test locally
npm run build
npm test

# Test autocomplete functionality
node test_autocomplete.js

# Test knowledge graph functionality
node test_knowledge_graph.js
```

### Dry Run Publishing

```bash
# Use workflow dispatch with dry_run=true
# Or locally:
npm publish --dry-run
```

## 📋 Version Management

### Semantic Versioning

- **Major (X.0.0)**: Breaking changes
- **Minor (X.Y.0)**: New features, backward compatible
- **Patch (X.Y.Z)**: Bug fixes, backward compatible

### NPM Tags

- `latest`: Stable releases (default)
- `beta`: Pre-release versions
- `alpha`: Early development versions
- `rc`: Release candidates

### Automatic Tagging

The publishing workflow automatically determines the npm tag based on:
- Version suffixes (`-beta`, `-alpha`, `-rc`)
- Release type (workflow dispatch vs git tag)

## 🔍 Publishing Verification

### After Publishing

1. **Check npm registry:**
   ```bash
   npm view @terraphim/autocomplete
   ```

2. **Test installation:**
   ```bash
   npm install @terraphim/autocomplete@latest
   ```

3. **Verify functionality:**
   ```bash
   node -e "
   const pkg = require('@terraphim/autocomplete');
   console.log('Available functions:', Object.keys(pkg));
   "
   ```

### Package Analytics

Monitor your package on npm:
- Downloads and usage statistics
- Dependency graph
- Version adoption

## 🚨 Troubleshooting

### Common Issues

**1. "npm token not found"**
- Check 1Password item exists: `op://TerraphimPlatform/npm.token/token`
- Verify GitHub secrets: `NPM_TOKEN`
- Ensure token has proper publishing permissions

**2. "Build failed"**
- Check Rust toolchain is installed correctly
- Verify all platform targets are available
- Check for compilation errors in workflow logs

**3. "Test failed"**
- Ensure all test files are present
- Check Node.js version compatibility
- Verify native libraries load correctly

**4. "Package not found" after publishing
- Wait 5-10 minutes for npm registry to update
- Check if publishing completed successfully
- Verify correct package name and version

### Debug Mode

Enable debug logging in workflows:

```yaml
env:
  DEBUG: napi:*
  RUST_LOG: debug
```

## 📚 Additional Resources

- [npm Publishing Documentation](https://docs.npmjs.com/cli/v8/commands/npm-publish)
- [NAPI-RS Documentation](https://napi.rs/)
- [WASM-Pack Documentation](https://rustwasm.github.io/wasm-pack/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)

## 🤝 Contributing

When making changes that affect publishing:

1. Test locally first
2. Use dry-run mode in CI
3. Verify all platforms build correctly
4. Update this documentation if needed

---

*Generated on: $(date)*
*Last updated: 2025-11-16*
