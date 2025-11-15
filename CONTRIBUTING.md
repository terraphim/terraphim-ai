## Earthly-Based Development

You can run the full stack using Earthly.
From the project root, execute the following command:

```sh
earthly ls
```

This will list all the available targets. You can then run the full stack using the following command:

```sh
earthly +pipeline
```

This will build the full stack using Earthly.

# Contributing to Terraphim AI

Thank you for your interest in contributing to Terraphim AI! This guide will help you get started with the development environment and contribution workflow.

## Quick Start for Contributors

1. **Fork and clone the repository**:
   ```bash
   git clone https://github.com/YOUR_USERNAME/terraphim-ai.git
   cd terraphim-ai
   ```

2. **Set up development environment**:
   ```bash
   # Install pre-commit hooks for code quality
   ./scripts/install-hooks.sh

   # Install sample data for system_operator role
   git clone https://github.com/terraphim/INCOSE-Systems-Engineering-Handbook.git /tmp/system_operator/
   ```

3. **Configure Git remotes** (if working with private code):
   ```bash
   # Add private repository remote
   git remote add private git@github.com:zestic-ai/terraphim-private.git
   ```

3. **Start development servers**:
   ```bash
   # Terminal 1: Backend server
   cargo run

   # Terminal 2: Frontend (web)
   cd desktop
   yarn install
   yarn run dev

   # Alternative: Desktop app
   yarn run tauri dev

   # Alternative: Terminal interface
   cargo run --bin terraphim-agent
   ```

## Development Environment Setup

### Prerequisites
- **Rust**: Install via [rustup](https://rustup.rs/)
- **Node.js**: Version 18+
- **Yarn**: Package manager for JavaScript dependencies
- **Git**: For version control

### Code Quality Tools (Recommended)
Our project uses automated code quality checks. Run this once:

```bash
./scripts/install-hooks.sh
```

This installs pre-commit hooks that will:
- Format Rust code with `cargo fmt`
- Lint Rust code with `cargo clippy`
- Format JavaScript/TypeScript with Biome
- Validate commit message format
- Check for secrets and large files
- Ensure consistent code style

**No Python required!** The script supports multiple hook managers (prek, lefthook, or native Git hooks).

### Commit Standards

We use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <description>

Examples:
feat: add user authentication system
fix(api): resolve memory leak in handler
docs(readme): update installation steps
chore(deps): bump tokio to 1.35.0
```

**Valid types**: feat, fix, docs, style, refactor, perf, test, chore, build, ci, revert

### Code Formatting

- **Rust**: Automatically formatted with `cargo fmt`
- **JavaScript/TypeScript**: Uses [Biome](https://biomejs.dev/) for linting and formatting
- **Configuration**: Pre-commit hooks enforce these standards

## Local Development

### Method 1: Manual Setup

If you want to develop without using Earthly, you need a local Node.js, Yarn, and Rust environment.

#### Install sample data for `system_operator`
```bash
git clone https://github.com/terraphim/INCOSE-Systems-Engineering-Handbook.git /tmp/system_operator/
```

#### Run the backend
```bash
cargo run
```

#### Run the frontend in development mode
```bash
cd desktop
yarn install
yarn run dev
```

### Method 2: Earthly-Based Development (Alternative)

You can also run the full stack using Earthly. From the project root:

```bash
# List available targets
earthly ls

# Build full stack
earthly +pipeline
```

## Testing

### Running Tests
```bash
# Run all Rust tests
cargo test --workspace

# Run specific crate tests
cargo test -p terraphim_service

# Run frontend tests
cd desktop
yarn test

# Run end-to-end tests
yarn run e2e
```

### Writing Tests
- Add unit tests for new functionality
- Include integration tests for API endpoints
- Frontend components should have corresponding tests
- Use descriptive test names following Rust conventions

## Branch Protection and Naming Conventions

### Repository Structure

Terraphim AI uses a **dual-repository approach** for security:

- **Public Repository** (`terraphim/terraphim-ai`): Open-source code only
- **Private Repository** (`zestic-ai/terraphim-private`): Proprietary and sensitive code

### Branch Naming Rules

#### ✅ Allowed Branch Names (Public Repository)
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

#### ❌ Blocked Branch Names (Public Repository)
```bash
# Private patterns (will be blocked by pre-push hook)
private-feature          # Use private repository instead
private_tf              # Use private repository instead
internal-api            # Use private repository instead
internal_docs           # Use private repository instead
client-data             # Use private repository instead
client_config           # Use private repository instead
secret-auth             # Use private repository instead
secret_key              # Use private repository instead
wip-private-feature     # Use private repository instead
customer-data           # Use private repository instead
proprietary-code        # Use private repository instead
confidential-docs       # Use private repository instead
```

### Pre-Push Hook Protection

The repository has a **comprehensive pre-push hook** that automatically:

1. **Validates branch names** against private patterns
2. **Scans commit messages** for private markers (`[PRIVATE]`, `[INTERNAL]`, etc.)
3. **Checks file contents** for sensitive keywords
4. **Validates file patterns** using `.gitprivateignore`
5. **Provides clear error messages** with guidance

### Working with Private Code

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

3. **Develop normally**:
   ```bash
   git add .
   git commit -m "feat: add private feature"
   git push origin private-feature
   ```

4. **Switch back to public** (when ready for open-source work):
   ```bash
   git remote set-url origin https://github.com/terraphim/terraphim-ai.git
   ```

### Troubleshooting Branch Protection

#### Error: "Branch matches private pattern"
```bash
✗ Branch 'private_tf' matches private pattern '^private_'
Private branches should not be pushed to public remotes.
Push to private remote instead: git push private private_tf
```

**Solution:**
```bash
# Configure branch to push to private remote
git config branch.private_tf.remote private
git config branch.private_tf.pushRemote private

# Push to private repository
git push private private_tf
```

#### Error: "Sensitive keyword found"
```bash
✗ Sensitive keyword 'truthforge' found in file changes
Remove sensitive content before pushing to public remote.
```

**Solution:**
- Remove or replace sensitive content
- Use private repository for sensitive development
- Update `.gitprivateignore` if it's a false positive

## Pull Request Process

1. **Create a feature branch**:
   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **Make your changes**:
   - Follow our code style (enforced by pre-commit hooks)
   - Add tests for new functionality
   - Update documentation if needed
   - Ensure no private/sensitive content

3. **Commit with conventional format**:
   ```bash
   git commit -m "feat: add semantic search improvements"
   ```

4. **Push and create PR**:
   ```bash
   git push origin feat/your-feature-name
   ```
   Then create a Pull Request on GitHub.

5. **PR Requirements**:
   - [ ] All tests pass
   - [ ] Code follows style guidelines (automatic via hooks)
   - [ ] Commit messages follow conventional format
   - [ ] Documentation updated if needed
   - [ ] No breaking changes without justification
   - [ ] No private/sensitive content (enforced by pre-push hook)

## Project Structure

```
terraphim-ai/
├── crates/                    # Rust library crates
│   ├── terraphim_service/    # Main service logic
│   ├── terraphim_config/     # Configuration management
│   ├── terraphim_agent/        # Terminal UI
│   └── ...                   # Other crates
├── desktop/                   # Svelte frontend + Tauri app
├── terraphim_server/         # HTTP server binary
├── scripts/                  # Development scripts
│   ├── hooks/               # Native Git hooks
│   └── install-hooks.sh     # Hook installation script
├── docs/                     # Documentation
└── .pre-commit-config.yaml  # Code quality configuration
```

## Development Guidelines

### Rust Code
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting (automatic via hooks)
- Address all `cargo clippy` warnings
- Add comprehensive documentation for public APIs
- Use `#[cfg(test)]` for test modules

### Frontend Code
- Use TypeScript for type safety
- Follow component-based architecture
- Use Biome for formatting and linting (automatic via hooks)
- Keep components small and focused
- Add proper error handling

### Commit Messages
Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `style:` for formatting changes
- `refactor:` for code refactoring
- `test:` for adding tests
- `chore:` for maintenance tasks

### Documentation
- Update README.md for user-facing changes
- Add inline code documentation
- Update API documentation for breaking changes
- Include examples in documentation

## Getting Help

- **Discord**: Join our [Discord server](https://discord.gg/VPJXB6BGuY) for real-time discussion
- **Discourse**: Visit our [community forum](https://terraphim.discourse.group) for detailed discussions
- **GitHub Issues**: Report bugs and request features
- **Documentation**: Check our [development setup guide](docs/src/development-setup.md)

## Code of Conduct

We are committed to providing a welcoming and inclusive experience for everyone. Please be respectful and constructive in all interactions.

## License

By contributing to Terraphim AI, you agree that your contributions will be licensed under the [Apache 2.0 License](LICENSE).
