# Development Setup Guide

This guide walks you through setting up a development environment for contributing to Terraphim AI, including code quality tools and pre-commit hooks.

## Prerequisites

### Required
- **Git**: Version control system
- **Rust**: Latest stable version via [rustup](https://rustup.rs/)
- **Node.js**: Version 18+ for desktop app development
- **Yarn**: Package manager for JavaScript dependencies

### Optional but Recommended
- **Pre-commit hook manager**: One of the following:
  - [prek](https://github.com/j178/prek) (Rust-based, no Python required)
  - [lefthook](https://github.com/evilmartians/lefthook) (Go-based, single binary)
  - [pre-commit](https://pre-commit.com/) (Python-based, original)

## Quick Start

1. **Clone the repository**:
   ```bash
   git clone https://github.com/terraphim/terraphim-ai.git
   cd terraphim-ai
   ```

2. **Install development dependencies**:
   ```bash
   # Install Rust dependencies
   cargo build
   
   # Install desktop dependencies
   cd desktop
   yarn install
   cd ..
   ```

3. **Set up code quality tools**:
   ```bash
   ./scripts/install-hooks.sh
   ```

This script will automatically:
- Detect available hook managers (prek, lefthook, pre-commit)
- Install appropriate hooks
- Set up Biome for JavaScript/TypeScript formatting
- Create secrets detection baseline
- Install native Git hooks as fallback

## Pre-commit Hooks Overview

Our pre-commit hooks enforce code quality and security standards:

### Code Quality Checks
- **Rust formatting**: `cargo fmt --check`
- **Rust linting**: `cargo clippy` with strict warnings
- **JavaScript/TypeScript**: Biome for linting and formatting
- **Trailing whitespace**: Automatic removal
- **File syntax**: YAML, TOML, JSON validation

### Security Checks
- **Secret detection**: Prevents accidental credential commits
- **Private key detection**: Blocks SSH keys and certificates
- **Large file prevention**: Stops accidental binary commits (>1MB)

### Commit Standards
- **Conventional commits**: Enforces structured commit messages
- **Message validation**: Checks format and content

## Hook Manager Options

### Option 1: prek (Recommended - No Python Required)

**Installation:**
```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/j178/prek/releases/download/v0.1.4/prek-installer.sh | sh

# Windows PowerShell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/j178/prek/releases/download/v0.1.4/prek-installer.ps1 | iex"

# Then install hooks
prek install
prek install --hook-type commit-msg
```

**Features:**
- Single binary, no dependencies
- ~10x faster than pre-commit
- Fully compatible with existing configurations
- Built in Rust

### Option 2: lefthook (Go-based Alternative)

**Installation:**
```bash
# Install lefthook
curl -sSfL https://raw.githubusercontent.com/evilmartians/lefthook/master/install.sh | sh

# Or via Homebrew
brew install lefthook

# Install hooks
lefthook install
```

**Features:**
- Single binary (Go-based)
- Parallel hook execution
- YAML configuration
- Fast and lightweight

### Option 3: pre-commit (Python-based Original)

**Installation:**
```bash
# Via pip
pip install pre-commit

# Via Homebrew
brew install pre-commit

# Install hooks
pre-commit install
pre-commit install --hook-type commit-msg
```

**Features:**
- Original and most mature
- Extensive plugin ecosystem
- Wide language support
- Requires Python runtime

### Option 4: Native Git Hooks (Fallback)

If you prefer not to install additional tools, native Git hooks are automatically installed as a fallback:

```bash
# These are copied to .git/hooks/ by the install script
.git/hooks/pre-commit
.git/hooks/commit-msg
```

## Manual Commands

### Running Hooks Manually

```bash
# Run all pre-commit checks manually
./scripts/hooks/pre-commit

# Check commit message format
./scripts/hooks/commit-msg .git/COMMIT_EDITMSG

# With hook managers:
pre-commit run --all-files    # pre-commit
prek run --all-files          # prek
lefthook run pre-commit       # lefthook
```

### Code Quality Commands

```bash
# Rust formatting
cargo fmt                              # Format code
cargo fmt --check                      # Check formatting

# Rust linting
cargo clippy --workspace --all-targets --all-features -- -D warnings

# JavaScript/TypeScript (Biome)
cd desktop
npx @biomejs/biome check --write       # Format and fix issues
npx @biomejs/biome check --write false # Check only
npx @biomejs/biome lint                # Lint only
npx @biomejs/biome format              # Format only
```

### Secret Detection

```bash
# Scan for secrets (if detect-secrets is available)
detect-secrets scan --baseline .secrets.baseline

# Update baseline with new findings
detect-secrets scan --baseline .secrets.baseline --update
```

## Conventional Commits

We enforce [Conventional Commits](https://www.conventionalcommits.org/) format:

### Format
```
<type>(<scope>): <description>

<body>

<footer>
```

### Types
- **feat**: New feature
- **fix**: Bug fix
- **docs**: Documentation changes
- **style**: Code style changes (formatting, etc.)
- **refactor**: Code refactoring
- **perf**: Performance improvements
- **test**: Adding or updating tests
- **chore**: Maintenance tasks
- **build**: Build system changes
- **ci**: CI/CD changes
- **revert**: Reverting previous commits

### Examples
```bash
feat: add user authentication system
fix(api): resolve memory leak in request handler
docs(readme): update installation instructions
chore(deps): bump tokio from 1.34.0 to 1.35.0
feat!: remove deprecated API endpoint (breaking change)
```

### Breaking Changes
For breaking changes, use:
- `feat!:` or `fix!:` (with exclamation mark)
- Or include `BREAKING CHANGE:` in commit body

## Biome Configuration

Biome is configured in `desktop/biome.json` with:

- **Linting**: TypeScript, JavaScript, and JSON files
- **Formatting**: Consistent code style
- **Import sorting**: Automatic import organization
- **Performance**: ~10x faster than ESLint + Prettier

### Biome Commands
```bash
cd desktop

# Check and fix all issues
npx @biomejs/biome check --write

# Check without fixing
npx @biomejs/biome check

# Format only
npx @biomejs/biome format --write

# Lint only
npx @biomejs/biome lint
```

## IDE Integration

### VS Code
Install these extensions for the best experience:
- **rust-analyzer**: Rust language support
- **Biome**: JavaScript/TypeScript formatting and linting
- **Conventional Commits**: Commit message assistance

### Other IDEs
- Most editors support Rust via rust-analyzer
- Biome has plugins for major editors
- Git hooks work regardless of editor

## Bypassing Hooks (Emergency Only)

Sometimes you need to bypass hooks for emergency commits:

```bash
# Skip all hooks
git commit --no-verify -m "emergency fix"

# Skip specific hook manager checks but keep native checks
SKIP=cargo-clippy git commit -m "skip clippy only"
```

**Note**: Use sparingly and fix issues in follow-up commits.

## Troubleshooting

### Common Issues

#### Hooks not running
```bash
# Check if hooks are installed
ls -la .git/hooks/

# Reinstall hooks
./scripts/install-hooks.sh
```

#### Biome not found
```bash
cd desktop
npm install --save-dev @biomejs/biome
# or
yarn add --dev @biomejs/biome
```

#### Rust tools not available
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Add required components
rustup component add rustfmt clippy
```

#### Permission denied on hooks
```bash
chmod +x .git/hooks/pre-commit
chmod +x .git/hooks/commit-msg
```

#### Secrets detection false positives
```bash
# Update baseline to ignore false positives
detect-secrets scan --baseline .secrets.baseline --update .secrets.baseline

# Or edit .secrets.baseline manually
```

### Getting Help

1. Check hook manager documentation:
   - [prek](https://github.com/j178/prek)
   - [lefthook](https://github.com/evilmartians/lefthook)
   - [pre-commit](https://pre-commit.com/)

2. Run hooks manually to see detailed error messages
3. Check our [development chat](https://discord.gg/terraphim) for community support

## Contributing

When contributing to Terraphim AI:

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feat/your-feature`
3. **Install development tools**: `./scripts/install-hooks.sh`
4. **Make your changes**: Code, test, commit with conventional format
5. **Push and create PR**: Automated checks will run

Your commits will be automatically validated for:
- Code formatting and style
- Security vulnerabilities
- Conventional commit format
- Breaking changes detection

This ensures consistent code quality across all contributions!