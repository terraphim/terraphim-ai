# Summary: README.md

## Purpose
This is the main project README for Terraphim AI, serving as the primary documentation and entry point for users, developers, and contributors.

## Key Functionality
- Introduces Terraphim as a privacy-first AI assistant with local operation and deterministic behavior
- Explains core concepts: Haystacks (data sources), Knowledge Graphs, Profiles, Roles, and Rolegraphs
- Provides comprehensive getting started guide with quick start, installation methods (Homebrew, Docker, Debian), and development setup
- Documents configuration for storage backends (local vs cloud), environment variables, and deployment scenarios
- Includes terminology definitions, contribution guidelines, and code style standards
- Features build/lint/test commands for both Rust backend and Svelte frontend

## Important Details
- Emphasizes privacy-first design with local infrastructure operation
- Supports multiple installation methods for end users and developers
- Includes detailed code style guidelines for Rust (Tokio, snake_case, Result<T,E>) and frontend (Svelte, Bulma CSS)
- Mandates conventional commits and pre-commit checks for contributions
- Documents deployment patterns using Caddy reverse proxy and 1Password CLI for secrets
- Contains troubleshooting guides and links to additional documentation