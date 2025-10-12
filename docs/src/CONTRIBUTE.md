# Contributing to Terraphim AI

## Overview

This guide covers the complete contribution workflow for Terraphim AI, including our dual-repository approach, branch protection, and development setup.

## Repository Structure

Terraphim AI uses a **dual-repository approach** for security:

- **Public Repository** (`terraphim/terraphim-ai`): Open-source code only
- **Private Repository** (`zestic-ai/terraphim-private`): Proprietary and sensitive code

## Branch Protection and Naming

### Allowed Branch Names (Public Repository)
```bash
# Feature branches
feat/new-ui-component
feat/semantic-search
fix/memory-leak
docs/update-readme
refactor/cleanup-code
test/add-unit-tests

# Development branches
wip/experimental-feature
experimental/new-algorithm
```

### Blocked Branch Names (Public Repository)
```bash
# Private patterns (blocked by pre-push hook)
private-feature          # Use private repository
private_tf              # Use private repository
internal-api            # Use private repository
client-data             # Use private repository
secret-auth             # Use private repository
wip-private-feature     # Use private repository
customer-data           # Use private repository
proprietary-code        # Use private repository
confidential-docs       # Use private repository
```

### Pre-Push Hook Protection

The repository automatically validates:
- Branch naming conventions
- Commit message markers (`[PRIVATE]`, `[INTERNAL]`, etc.)
- File content for sensitive keywords
- File patterns using `.gitprivateignore`

## Development Setup

### Prerequisites
- **Rust**: Latest stable version via [rustup](https://rustup.rs/)
- **Node.js**: Version 18+ for desktop app development
- **Yarn**: Package manager for JavaScript dependencies
- **Git**: For version control

### Quick Start

1. **Clone and setup**:
   ```bash
   git clone https://github.com/terraphim/terraphim-ai.git
   cd terraphim-ai

   # Install pre-commit hooks
   ./scripts/install-hooks.sh

   # Add private remote (if needed)
   git remote add private git@github.com:zestic-ai/terraphim-private.git
   ```

2. **Install sample data**:
   ```bash
   git clone https://github.com/terraphim/INCOSE-Systems-Engineering-Handbook.git /tmp/system_operator/
   ```

## Development Methods

### Method 1: Local Development

#### Run the backend
```bash
cargo run
```

#### Run the frontend
```bash
cd desktop
yarn install
yarn run dev
```

#### Run Tauri desktop app
```bash
cd desktop
yarn run tauri dev
```

### Method 2: Earthly-Based Development

You can run the full stack using Earthly:

```bash
# List available targets
earthly ls

# Build full stack
earthly +pipeline
```

## Working with Private Code

If you need to work with private or sensitive code:

1. **Switch to private repository**:
   ```bash
   git remote set-url origin git@github.com:zestic-ai/terraphim-private.git
   ```

2. **Create private branch**:
   ```bash
   git checkout -b private-feature
   # or
   git checkout -b internal-api
   ```

3. **Develop and push**:
   ```bash
   git add .
   git commit -m "feat: add private feature"
   git push origin private-feature
   ```

4. **Switch back to public** (when ready):
   ```bash
   git remote set-url origin https://github.com/terraphim/terraphim-ai.git
   ```

## Troubleshooting

### Branch Protection Errors

#### "Branch matches private pattern"
```bash
✗ Branch 'private_tf' matches private pattern '^private_'
Private branches should not be pushed to public remotes.
Push to private remote instead: git push private private_tf
```

**Solution:**
```bash
git config branch.private_tf.remote private
git config branch.private_tf.pushRemote private
git push private private_tf
```

#### "Sensitive keyword found"
```bash
✗ Sensitive keyword 'truthforge' found in file changes
Remove sensitive content before pushing to public remote.
```

**Solution:**
- Remove or replace sensitive content
- Use private repository for sensitive development
- Update `.gitprivateignore` if false positive

## Code Quality

The project enforces code quality through:

- **Pre-commit hooks**: Formatting, linting, security checks
- **Conventional commits**: Structured commit messages
- **Automated testing**: CI/CD pipeline validation
- **Branch protection**: Prevents private content leaks

## Getting Help

- **Discord**: [Terraphim Discord](https://discord.gg/VPJXB6BGuY)
- **Discourse**: [Community Forum](https://terraphim.discourse.group)
- **GitHub Issues**: Bug reports and feature requests
- **Documentation**: Check `docs/src/` for detailed guides
