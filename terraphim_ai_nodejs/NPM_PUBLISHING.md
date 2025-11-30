# npm Publishing Guide for @terraphim/autocomplete

This comprehensive guide explains how to publish the `@terraphim/autocomplete` Node.js package to npm using our CI/CD pipelines with 1Password integration and Bun package manager support.

## 🚀 Overview

The `@terraphim/autocomplete` package provides:
- **Autocomplete Engine**: Fast prefix search with Aho-Corasick automata
- **Knowledge Graph**: Semantic connectivity analysis and graph traversal
- **Native Performance**: Rust backend with NAPI bindings
- **Cross-Platform**: Linux, macOS, Windows, ARM64 support
- **Package Manager Support**: npm, yarn, and Bun compatibility
- **TypeScript**: Auto-generated type definitions included

## 📦 Package Structure

```
@terraphim/autocomplete/
├── index.js                 # Main entry point with exports
├── index.d.ts              # TypeScript type definitions
├── terraphim_ai_nodejs.*.node # Native binaries (per platform)
├── package.json            # Package metadata and configuration
├── README.md               # Usage documentation
├── NPM_PUBLISHING.md       # This publishing guide
└── PUBLISHING.md           # General publishing information
```

### Supported Platforms

- **Linux**: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
- **macOS**: `x86_64-apple-darwin`, `aarch64-apple-darwin`, `universal-apple-darwin`
- **Windows**: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`

## 🔐 Token Management with 1Password

### 1Password Setup

The publishing workflows use 1Password for secure token management:

**Required 1Password Items:**
- `op://TerraphimPlatform/npm.token/token` - Main npm publishing token
- `op://TerraphimPlatform/bun.token/token` - Bun registry token (optional)

### Token Fallback Strategy

If 1Password tokens are not available, workflows fall back to:
- `NPM_TOKEN` (GitHub Secrets) - Main npm token
- `BUN_TOKEN` (GitHub Secrets) - Bun registry token

### Setting up Publishing Tokens

1. **Generate npm Token:**
   ```bash
   # Login to npm
   npm login

   # Generate automation token (recommended for CI/CD)
   npm token create --access=public
   ```

2. **Store in 1Password:**
   - Open 1Password and access the "TerraphimPlatform" vault
   - Create/update the `npm.token` item with your npm access token
   - Ensure the token has publishing permissions for the `@terraphim` scope
   - Set appropriate access level and expiration

3. **Configure GitHub Secrets (Backup):**
   ```bash
   # In GitHub repository settings > Secrets and variables > Actions
   NPM_TOKEN=your_npm_token_here
   BUN_TOKEN=your_bun_token_here  # Optional
   ```

## 🏗️ Publishing Methods

### Method 1: Automated via Tag (Recommended)

**For npm Publishing:**
```bash
# Create and push version tag
git tag nodejs-v1.0.0
git push origin nodejs-v1.0.0
```

**For Bun-Optimized Publishing:**
```bash
# Create and push Bun version tag
git tag bun-v1.0.0
git push origin bun-v1.0.0
```

**Features:**
- ✅ Automatic multi-platform building
- ✅ Comprehensive testing before publishing
- ✅ 1Password token management
- ✅ Automatic GitHub release creation
- ✅ Package verification after publishing

### Method 2: Manual Workflow Dispatch

**From GitHub Actions:**
1. Go to Actions → "Publish Node.js Package to npm" or "Publish to Bun Registry"
2. Click "Run workflow"
3. Fill in parameters:
   - **Version**: Semantic version (e.g., `1.0.1`)
   - **Tag**: npm/Bun tag (`latest`, `beta`, `alpha`, `rc`)
   - **Dry Run**: Enable for testing without publishing

### Method 3: Local Publishing (Development)

**For testing and development:**
```bash
# Build the package locally
npm run build

# Run tests
npm test

# Test package locally
npm pack --dry-run

# Publish (requires npm token in ~/.npmrc)
npm publish --access public
```

## 📋 Version Management

### Semantic Versioning

- **Major (X.0.0)**: Breaking changes
- **Minor (X.Y.0)**: New features, backward compatible
- **Patch (X.Y.Z)**: Bug fixes, backward compatible

### Package Tags

- `latest`: Stable releases (default)
- `beta`: Pre-release versions
- `alpha`: Early development versions
- `rc`: Release candidates

### Automatic Tagging

The publishing workflows automatically determine the package tag based on:
- Version suffixes (`-beta`, `-alpha`, `-rc`)
- Release type (workflow vs git tag)
- Target registry (npm vs Bun)

## 🧪 Testing Before Publishing

### Local Testing

```bash
# Install dependencies
npm install

# Build native binaries
npm run build

# Run Node.js tests
npm run test:node

# Run Bun tests (if Bun installed)
npm run test:bun

# Run all tests
npm run test:all
```

### Dry Run Publishing

```bash
# Local dry run
npm publish --dry-run

# Workflow dry run (via GitHub Actions)
# Use workflow dispatch with dry_run=true
```

### Pre-Publishing Checklist

- [ ] All tests pass on Node.js 18+ and 20+
- [ ] All tests pass on Bun latest and LTS versions
- [ ] Native binaries build successfully for all platforms
- [ ] TypeScript definitions are up to date
- [ ] Documentation is accurate and complete
- [ ] Version number follows semantic versioning
- [ ] 1Password tokens are configured and valid

## 🔄 CI/CD Workflow Details

### npm Publishing Workflow (`publish-npm.yml`)

**Trigger Events:**
- `workflow_dispatch`: Manual publishing with parameters
- `push` on `nodejs-v*` tags: Automatic version publishing
- `release` types: `[published]`: Release-based publishing

**Jobs:**
1. **validate**: Package validation and basic testing
2. **build**: Multi-platform binary compilation
3. **test-universal**: Cross-platform compatibility testing
4. **create-universal-macos**: Universal macOS binary creation
5. **publish**: npm publishing with 1Password authentication

### Bun Publishing Workflow (`publish-bun.yml`)

**Trigger Events:**
- `workflow_dispatch`: Manual Bun-optimized publishing
- `push` on `bun-v*` tags: Automatic Bun version publishing
- `release` types: `[published]`: Release-based publishing

**Jobs:**
1. **validate**: Bun-specific validation and testing
2. **build**: Multi-platform binary compilation (same as npm)
3. **test-bun-compatibility**: Multi-version Bun testing and performance benchmarking
4. **create-universal-macos-bun**: Universal macOS binary for Bun
5. **publish-to-bun**: Bun-optimized npm publishing

### Enhanced CI Workflow (`CI.yml`)

**Auto-Publishing:**
- Commits with semantic version messages trigger automatic publishing
- Version detection from commit message: `^[0-9]+\.[0-9]+\.[0-9]+$`
- Fallback to `next` tag for pre-release versions

## 📊 Package Features and API

### Autocomplete Functions

```javascript
import * as autocomplete from '@terraphim/autocomplete';

// Build autocomplete index from JSON thesaurus
const indexBytes = autocomplete.buildAutocompleteIndexFromJson(thesaurusJson);

// Perform autocomplete search
const results = autocomplete.autocomplete(indexBytes, prefix, limit);

// Fuzzy search with Jaro-Winkler distance
const fuzzyResults = autocomplete.fuzzyAutocompleteSearch(
  indexBytes, prefix, minDistance, limit
);
```

### Knowledge Graph Functions

```javascript
// Build knowledge graph from role and thesaurus
const graphBytes = autocomplete.buildRoleGraphFromJson(roleName, thesaurusJson);

// Check if terms are connected in the graph
const isConnected = autocomplete.areTermsConnected(graphBytes, searchText);

// Query the graph for related terms
const queryResults = autocomplete.queryGraph(graphBytes, query, offset, limit);

// Get graph statistics
const stats = autocomplete.getGraphStats(graphBytes);
```

### Usage with Different Package Managers

**npm:**
```bash
npm install @terraphim/autocomplete
```

**yarn:**
```bash
yarn add @terraphim/autocomplete
```

**Bun:**
```bash
bun add @terraphim/autocomplete
```

## 🔍 Publishing Verification

### After Publishing

1. **Check npm registry:**
   ```bash
   npm view @terraphim/autocomplete
   npm view @terraphim/autocomplete versions
   ```

2. **Test installation:**
   ```bash
   # Fresh install test
   mkdir test-dir && cd test-dir
   npm init -y
   npm install @terraphim/autocomplete@latest

   # Test functionality
   node -e "
   const pkg = require('@terraphim/autocomplete');
   console.log('Available functions:', Object.keys(pkg));
   console.log('Autocomplete test:', pkg.autocomplete instanceof Function);
   "
   ```

3. **Verify with Bun:**
   ```bash
   bunx pm install @terraphim/autocomplete@latest --dry-run
   ```

### Package Analytics

Monitor your package:
- [npm package page](https://www.npmjs.com/package/@terraphim/autocomplete)
- Download statistics and trends
- Dependency graph analysis
- Version adoption metrics

## 🚨 Troubleshooting

### Common Issues

**1. "npm token not found" Error**
```bash
# Check 1Password configuration
op read "op://TerraphimPlatform/npm.token/token"

# Check GitHub secrets
echo $NPM_TOKEN

# Verify token permissions
npm token list
```

**2. "Build failed" Errors**
```bash
# Check Rust toolchain
rustc --version
cargo --version

# Verify NAPI targets
rustup target list --installed

# Local build test
npm run build
```

**3. "Test failed" Errors**
```bash
# Run tests locally
npm test

# Check Node.js version
node --version  # Should be 14+

# Platform-specific testing
npm run test:node
npm run test:bun  # If Bun installed
```

**4. "Package not found" After Publishing**
- Wait 5-10 minutes for npm registry to update
- Check GitHub Actions workflow logs
- Verify successful publishing completion
- Check correct package name and version

**5. "Permission denied" Errors**
```bash
# Verify npm authentication
npm whoami

# Check package scope permissions
npm access ls-collaborators @terraphim/autocomplete
```

### Debug Mode

Enable debug logging in workflows:
```yaml
env:
  DEBUG: napi:*
  RUST_LOG: debug
  NAPI_DEBUG: 1
```

### Platform-Specific Issues

**macOS Universal Binary:**
```bash
# Verify universal binary creation
lipo -info *.node

# Test on both architectures
arch -x86_64 node test.js
arch -arm64 node test.js
```

**Linux ARM64:**
```bash
# Test with QEMU emulation
docker run --rm --platform linux/arm64 node:20-alpine node test.js
```

**Windows:**
```bash
# Test PowerShell compatibility
powershell -Command "node test.js"

# Verify DLL loading
node -e "console.log(process.arch, process.platform)"
```

## 📚 Additional Resources

### Documentation
- [npm Publishing Documentation](https://docs.npmjs.com/cli/v8/commands/npm-publish)
- [NAPI-RS Documentation](https://napi.rs/)
- [Bun Package Manager Documentation](https://bun.sh/docs)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)

### Tools and Utilities
- [1Password CLI Documentation](https://developer.1password.com/docs/cli/)
- [Semantic Versioning Specification](https://semver.org/)
- [Node.js API Documentation](https://nodejs.org/api/)

### Related Projects
- [Terraphim AI Repository](https://github.com/terraphim/terraphim-ai)
- [Rust Crate Registry](https://crates.io/crates/terraphim_automata)
- [Python Package (PyPI)](https://pypi.org/project/terraphim-automata/)

## 🤝 Contributing to Publishing Process

When making changes that affect publishing:

1. **Test locally first**
   ```bash
   npm run build
   npm test
   npm pack --dry-run
   ```

2. **Use dry-run mode in CI**
   - Enable `dry_run=true` in workflow dispatch
   - Review all build and test outputs

3. **Verify all platforms**
   - Check workflow matrix builds
   - Ensure all target platforms compile successfully

4. **Update documentation**
   - Keep this NPM_PUBLISHING.md current
   - Update PUBLISHING.md if needed
   - Ensure README.md reflects latest changes

5. **Version management**
   - Follow semantic versioning
   - Update CHANGELOG.md if applicable
   - Create appropriate git tags

## 📋 Quick Reference

### Essential Commands
```bash
# Local development
npm install
npm run build
npm test

# Publishing commands
npm publish --dry-run
npm publish --access public

# Verification
npm view @terraphim/autocomplete
npm info @terraphim/autocomplete

# Git tagging for auto-publishing
git tag nodejs-v1.0.0
git push origin nodejs-v1.0.0
```

### Key Files
- `package.json` - Package metadata and configuration
- `index.js` - Main entry point and exports
- `index.d.ts` - TypeScript definitions
- `NPM_PUBLISHING.md` - This publishing guide
- `.github/workflows/publish-npm.yml` - npm publishing CI/CD
- `.github/workflows/publish-bun.yml` - Bun publishing CI/CD

### Important URLs
- npm Package: https://www.npmjs.com/package/@terraphim/autocomplete
- Repository: https://github.com/terraphim/terraphim-ai
- Issues: https://github.com/terraphim/terraphim-ai/issues

---

*Generated on: 2025-11-16*
*Last updated: 2025-11-16*
*Maintainer: Terraphim AI Team*
