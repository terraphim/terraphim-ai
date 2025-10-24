# Summary: AGENTS.md

## File Purpose
Agent development guide providing build commands, code style guidelines, and development workflow instructions for the Terraphim AI project.

## Key Functionality
- **Build Commands**: Rust and Svelte development commands with feature flags
- **Code Style Guidelines**: Rust and frontend coding standards and conventions
- **Development Workflow**: Quality assurance, testing, and deployment patterns
- **Documentation Management**: Guidelines for using the `.docs/` folder organization

## Important Details
- **Rust Guidelines**: Use tokio for async, snake_case naming, Result<T,E> error handling
- **Frontend Guidelines**: Svelte with TypeScript, Bulma CSS, yarn package manager
- **Testing Philosophy**: No mocks, use IDE diagnostics, check test coverage
- **Development Tools**: tmux for background tasks, gh tool for GitHub issues
- **Quality Assurance**: Pre-commit hooks, conventional commits, no sleep/timeout commands
- **Documentation Organization**: Uses `.docs/` folder for file summaries and comprehensive overview

## Architecture Impact
- Establishes consistent development patterns across the project
- Ensures code quality and maintainability standards
- Provides clear guidelines for new contributors
- Defines testing and deployment workflows
- Mandates `/init` command steps for documentation maintenance
