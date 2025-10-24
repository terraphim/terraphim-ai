# Terraphim AI - Comprehensive Project Overview

## Architecture Overview

Terraphim AI is a privacy-first AI assistant built as a multi-crate Rust workspace with a Svelte frontend. The system operates locally on user hardware for complete data control and deterministic behavior.

### Core Components

**Backend Architecture:**
- **Multi-Crate Workspace**: Organized under `crates/*` with `terraphim_server` as the main binary
- **Rust Edition 2024**: Modern Rust with async runtime (Tokio) and comprehensive error handling
- **Modular Design**: Separate crates for persistence, config, middleware, rolegraph, automata, service, multi_agent, and truthforge
- **Web Framework**: Axum for HTTP API and WebSocket support with Tower for middleware

**Frontend Architecture:**
- **Svelte with TypeScript**: Vanilla JavaScript framework with type safety
- **Bulma CSS Framework**: Consistent styling without Tailwind dependencies
- **Tauri Integration**: Desktop app capabilities with `desktop/src-tauri`
- **Comprehensive Testing**: Playwright for E2E, Vitest for unit tests, WebDriver support

**AI Integration:**
- **Multi-Provider Support**: OpenRouter (Claude, GPT, Gemini) and Ollama (local models)
- **Dynamic Model Selection**: 4-level priority system (request > role > global > default)
- **MCP Server**: Model Context Protocol for AI tool integration
- **VM Execution**: Firecracker microVMs for secure code execution

**Knowledge Systems:**
- **Haystacks**: Data sources (filesystem, Notion, GitHub, QueryRs for Rust docs)
- **Knowledge Graphs**: Structured entity relationships with RoleGraph
- **Search Systems**: Multiple relevance functions (TitleScorer, BM25, TerraphimGraph)
- **Autocomplete**: Fast text matching with Aho-Corasick automata

## Documentation Organization

All project documentation is organized in the `.docs/` folder:
- **Individual File Summaries**: `.docs/summary-<normalized-path>.md` - Detailed summaries of each working file
- **Comprehensive Overview**: `.docs/summary.md` - Consolidated project overview and architecture analysis
- **Agent Instructions**: `.docs/agents_instructions.json` - Machine-readable agent configuration and workflows

## Security Analysis

**Privacy-First Design:**
- Local operation with no external data sharing
- User-controlled infrastructure for complete data sovereignty
- Graceful degradation when cloud services unavailable

**Comprehensive Security Testing:**
- **Vulnerability Remediation**: Fixed prompt injection, command injection, unsafe memory operations, and network validation
- **Security Test Suites**: 99+ tests across prompt sanitization, concurrent access, error boundaries, and DoS prevention
- **Deployment Security**: Caddy reverse proxy with automatic HTTPS, 1Password CLI for secret management

**Secure Development Practices:**
- No mocks in tests for realistic validation
- Structured concurrency with scoped tasks
- Input validation and sanitization throughout
- IDE diagnostics for early error detection

## Testing Strategy

**Multi-Level Testing:**
- **Unit Tests**: Tokio-based async testing for all components
- **Integration Tests**: Cross-crate functionality validation
- **E2E Tests**: Full workflow testing with Playwright automation
- **Security Tests**: Comprehensive vulnerability and bypass testing

**Test Infrastructure:**
- **Interactive Test Suite**: `test-all-workflows.html` for manual validation
- **Browser Automation**: Playwright with screenshot capture and CI integration
- **Performance Testing**: Benchmarking and resource usage validation
- **Test Coverage**: 100% pass rates across 99+ tests

**Quality Assurance:**
- **Pre-commit Hooks**: Format, lint, and compilation checks
- **CI/CD Pipeline**: GitHub Actions with matrix builds for multiple platforms
- **Conventional Commits**: Structured commit messages with validation

## Business Value

**Privacy and Control:**
- Complete data ownership with local operation
- No vendor lock-in or external dependencies
- GDPR and privacy regulation compliant

**Technical Excellence:**
- Production-ready architecture with comprehensive documentation
- Multi-agent workflows for complex task automation
- Real-time capabilities with WebSocket integration
- Cross-platform deployment (Linux, macOS, Windows)

**Developer Experience:**
- Extensive documentation and setup guides
- Modern development tools and IDE integration
- Automated testing and deployment pipelines
- Clear contribution guidelines and code standards

**Scalability and Maintenance:**
- Modular architecture for easy feature addition
- Multi-platform build system with Earthly
- Comprehensive monitoring and observability
- Long-term support with active development

## Key Features and Capabilities

**Multi-Agent Workflows:**
- 5 workflow patterns: prompt-chain, routing, parallel, orchestration, optimization
- Real AI execution with knowledge graph intelligence
- Individual agent evolution with memory and task tracking

**Advanced AI Features:**
- Dynamic model selection across providers
- Context enrichment from knowledge graphs
- Secure code execution in isolated VMs
- Comprehensive error handling and recovery

**Production Deployment:**
- Docker and native binary support
- Caddy-based reverse proxy with TLS
- Systemd service management
- 1Password integration for secrets

## Mandatory /init Command Steps

When user executes `/init` command, you MUST perform these two steps in order:

### Step 1: Summarize Working Files
Can you summarize the working files? Save each file's summary in `.docs/summary-<normalized-path>.md`

- Identify all relevant working files in the project
- Create individual summaries for each file
- Save summaries using the pattern: `.docs/summary-<normalized-path>.md`
- Include file purpose, key functionality, and important details
- Normalize file paths (replace slashes with hyphens, remove special characters)

### Step 2: Create Comprehensive Summary
Can you summarize your context files ".docs/summary-*.md" and save the result in `.docs/summary.md`
- Read all individual summary files created in Step 1
- Synthesize into a comprehensive project overview
- Include architecture, security, testing, and business value analysis
- Save the consolidated summary as `.docs/summary.md`
- Update any relevant documentation references

Both steps are MANDATORY for every `/init` command execution.

## Documentation References

**Core Documentation:**
- `README.md`: Primary project overview and getting started guide
- `AGENTS.md`: Agent development guide with build commands and code style
- `agents_instructions.json`: Machine-readable agent configuration and workflows
- `memories.md`: Detailed development journal and progress tracking

**Technical Documentation:**
- `agents_history.txt`: Chronological log of development activities
- `test_rust_engineer.sh`: Validation script for Rust Engineer role setup
- `build_config.toml`: Build configuration and deployment parameters
- `Earthfile`: Multi-platform build pipeline definition

**Architecture Documentation:**
- `Cargo.toml`: Workspace configuration and dependency management
- `desktop/package.json`: Frontend build and testing configuration
- `terraphim_server/Cargo.toml`: Server binary dependencies and features
- `crates/terraphim_kg_agents/Cargo.toml`: Knowledge graph agent crate configuration

This comprehensive overview represents the complete Terraphim AI system as of the current development state, providing a solid foundation for privacy-first AI assistance with production-ready capabilities and extensive testing validation.
