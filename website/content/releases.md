+++
title = "Releases"
description = "Latest Terraphim AI releases and changelog"
date = 2026-01-27
sort_by = "date"
paginate_by = 10
+++

# Releases

Stay up-to-date with the latest Terraphim AI releases.

## Latest Release: v1.5.2

**Released:** January 20, 2026

[Download from GitHub](https://github.com/terraphim/terraphim-ai/releases/latest) | [Full Changelog](https://github.com/terraphim/terraphim-ai/blob/main/terraphim_server/CHANGELOG.md)

### Quick Install

\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash
\`\`\`

### What's New

v1.5.2 includes bug fixes and performance improvements:

- Fixed GitHub Actions workflow issues
- Improved memory usage for large knowledge graphs
- Enhanced search performance for complex queries
- Updated dependencies for better security

### Installation

Choose your preferred method:

\`\`\`bash
# Universal installer
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash

# Homebrew
brew install terraphim-server terraphim-agent

# Cargo
cargo install terraphim-repl terraphim-cli

# npm
npm install @terraphim/autocomplete

# PyPI
pip install terraphim-automata
\`\`\`

[Installation Guide](/docs/installation)

## Recent Releases

### v1.5.1 - January 18, 2026

[Release Notes](https://github.com/terraphim/terraphim-ai/releases/tag/v1.5.1)

Minor update with documentation improvements and bug fixes.

### v1.5.0 - January 16, 2026

[Release Notes](https://github.com/terraphim/terraphim-ai/releases/tag/v1.5.0)

Major feature release:

- New role-based search system
- Improved knowledge graph connectivity
- Enhanced CLI with 8 commands
- Updated REPL with 11 commands
- Multi-language support improvements

### v1.4.8 - January 11, 2026

[Release Notes](https://github.com/terraphim/terraphim-ai/releases/tag/v1.4.8)

Performance and stability improvements.

### v1.4.7 - January 6, 2026

[Release Notes](https://github.com/terraphim/terraphim-ai/releases/tag/v1.4.7)

Bug fixes and documentation updates.

## All Releases

View complete release history on [GitHub Releases](https://github.com/terraphim/terraphim-ai/releases).

## Release Channels

### Stable

Stable releases are recommended for production use. They have been thoroughly tested and are the most reliable version.

**Latest Stable:** v1.5.2

### Development

Development releases contain the latest features and improvements but may have more bugs. Use these for testing new features.

Check the [main branch](https://github.com/terraphim/terraphim-ai/tree/main) for development builds.

## Upgrade Guide

### From Any Version to Latest

\`\`\`bash
# Universal installer (recommended)
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash

# Homebrew
brew upgrade terraphim-server terraphim-agent

# Cargo
cargo install terraphim-repl --force
cargo install terraphim-cli --force

# npm
npm update @terraphim/autocomplete

# PyPI
pip install --upgrade terraphim-automata
\`\`\`

### Configuration Compatibility

Terraphim maintains backward compatibility for configuration files across minor versions. Major version bumps (e.g., 1.x to 2.0) may require configuration updates.

## Migration Guides

If you're upgrading from a significantly older version, check these migration guides:

- [v1.4.x to v1.5.x](https://docs.terraphim.ai/migration/1.4-to-1.5.html)
- [v1.3.x to v1.4.x](https://docs.terraphim.ai/migration/1.3-to-1.4.html)

## Release Notes Archive

For detailed release notes and changelogs, visit:

- [Server Changelog](https://github.com/terraphim/terraphim-ai/blob/main/terraphim_server/CHANGELOG.md)
- [Desktop Changelog](https://github.com/terraphim/terraphim-ai/blob/main/desktop/CHANGELOG.md)
- [GitHub Releases](https://github.com/terraphim/terraphim-ai/releases)

## Verify Your Installation

After installation or upgrade, verify your version:

\`\`\`bash
terraphim-server --version
terraphim-agent --version
terraphim-repl --version
\`\`\`

Expected output: \`Terraphim Server v1.5.2\` (or your installed version).

## Beta Testing

Want to test new features before they're released?

Join our [Discord server](https://discord.gg/VPJXB6BGuY) and look for \#beta-testing channel. Beta testers get early access to new features and help shape product.

## Security Updates

Security updates are released as soon as they're available. Stay informed by:

- Watching the [repository](https://github.com/terraphim/terraphim-ai/watchers)
- Subscribing to [security advisories](https://github.com/terraphim/terraphim-ai/security/advisories)
- Following [@TerraphimAI](https://twitter.com/alex_mikhalev) on Twitter

## Need Help?

If you encounter issues with a release:

1. Check the [troubleshooting section](https://docs.terraphim.ai/troubleshooting.html)
2. Search [existing issues](https://github.com/terraphim/terraphim-ai/issues)
3. [Create a new issue](https://github.com/terraphim/terraphim-ai/issues/new)
4. Join [Discord community](https://discord.gg/VPJXB6BGuY) for support
