# Summary: CONTRIBUTING.md

## Purpose
Guide for contributors to understand how to participate in the Terraphim AI project, covering setup, standards, and submission process.

## Key Sections
- **Getting Started**: Clone, install hooks, environment setup
- **Code Quality Standards**: Conventional Commits, cargo fmt, Biome for JS/TS
- **Development Workflow**: Feature branches, proper testing, pull requests
- **Dependency Management**: Version pinning constraints, Dependabot configuration

## Code Quality Requirements
- **Commit Format**: Conventional Commits (feat:, fix:, docs:, etc.)
- **Rust**: Automatic formatting with `cargo fmt`
- **JavaScript/TypeScript**: Biome for linting and formatting
- **Security**: No secrets or large files allowed in commits
- **Testing**: Proper test coverage required for all changes

## Development Process
1. Create feature branch: `git checkout -b feat/your-feature`
2. Make changes with proper tests
3. Commit with conventional format: `git commit -m "feat: add amazing feature"`
4. Push and create Pull Request
5. Automated checks must pass (format, lint, tests, build)

## Dependency Constraints
Critical pinned versions:
- `wiremock = "0.6.4"` - Version 0.6.5 uses unstable Rust features
- `schemars = "0.8.22"` - Version 1.0+ introduces breaking API changes
- `thiserror = "1.0.x"` - Version 2.0+ requires code migration

These are enforced in `.github/dependabot.yml` to prevent automatic breaking updates.

## Setup Commands
```bash
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
./scripts/install-hooks.sh  # Sets up code quality tools
```

## Important Notes
- Pre-commit hooks are **required** (enforced in CI)
- All PRs must pass automated checks
- Follow existing code style and conventions
- Update documentation for user-facing changes
- Add tests for new functionality
