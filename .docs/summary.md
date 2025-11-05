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

---

## Recent Implementation: MCP Authentication System (2025-11-02)

### Critical Security Discovery & Remediation

The project completed a major security implementation using Test-Driven Development (TDD) for MCP (Model Context Protocol) authentication. A **CRITICAL vulnerability** was discovered and fixed: authentication middleware was fully tested but not applied to production routes.

**Security Score Improvement**: 2/10 → 8/10

### MCP Authentication Architecture

```
HTTP Request → validate_api_key Middleware → McpPersistenceImpl → MCP API Handlers
                ├─ Extract Bearer token
                ├─ SHA256 hash verification
                ├─ Enabled status check
                ├─ Expiration validation
                └─ Security logging
```

### Key Components

#### 1. Authentication Middleware (`terraphim_server/src/mcp_auth.rs`)
- Bearer token validation with SHA256 hashing
- Three-layer security: exists + enabled + not expired
- Structured logging for attack detection
- Uses shared `AppState.mcp_persistence` for data persistence

#### 2. MCP API Handlers (`terraphim_server/src/api_mcp.rs`)
- CRUD operations for namespaces, endpoints, API keys
- Auto-generated UUIDs and timestamps
- API key format: `tpai_{uuid}` (plaintext returned once, hash stored)
- Comprehensive audit trail with latency tracking

#### 3. Persistence Layer (`crates/terraphim_persistence/src/mcp.rs`)
- Multi-backend storage via OpenDAL (Memory, Filesystem, S3, Azure)
- Hierarchical JSON structure: `mcp/{resource}/{uuid}.json`
- Thread-safe: `Arc<RwLock<Operator>>` for concurrent access
- 7 unit tests validating CRUD operations

#### 4. Test Suite (`terraphim_server/tests/mcp_auth_tests.rs`)
- 11 comprehensive tests (100% passing in <0.01s)
- Core authentication + security edge cases
- No mocks (uses real Memory backend)
- Validates production router configuration

### The Critical Bug

**What Was Broken:**
```rust
// tests/mcp_auth_tests.rs - ✅ HAD AUTHENTICATION
protected_mcp_routes.route_layer(middleware::from_fn_with_state(..., validate_api_key))

// src/lib.rs (PRODUCTION) - ❌ NO AUTHENTICATION
.route("/metamcp/namespaces", post(api_mcp::create_namespace))
// All MCP endpoints were completely unprotected!
```

**Impact:** Anyone could create namespaces, endpoints, API keys, execute tools, and access audit logs without authentication.

**Fix (Commit 35c9cfc0):** Applied authentication middleware to all production MCP routes.

### TDD Lessons Learned

1. **TDD Validates Logic, Not Configuration** - Tests passed but production lacked middleware
2. **Integration Tests Must Match Production** - Same router config in tests and production
3. **Security Requires Layered Validation** - Check exists + enabled + not expired
4. **Retrospectives Catch Critical Issues** - "What would you do better?" led to discovery
5. **Shared State is Critical** - Must use `AppState.mcp_persistence`, not new instances

### Security Improvements Implemented

| Feature | Before | After |
|---------|--------|-------|
| Production Routes | ❌ No auth | ✅ Fully protected |
| Expiration Check | ❌ Missing | ✅ Validated |
| Enabled Status | ❌ Missing | ✅ Validated |
| Test Coverage | 7 tests | 11 tests (+57%) |
| Security Logging | None | Comprehensive |

### Technical Details

**API Key Security:**
- SHA256 hashing (never stores plaintext)
- Format: `tpai_{uuid_without_hyphens}`
- Expiration support via `expires_at` timestamp
- Revocation via `enabled` flag (no deletion needed)

**Performance Characteristics:**
- API key verification: O(n) linear search (acceptable for <1000 keys)
- List operations: Unbounded (future: add pagination)
- Audit logging: O(n log n) due to timestamp sorting

**Known Limitations:**
- No rate limiting (Phase 2 planned)
- No in-memory cache for key verification
- SQLite backend incompatible (hierarchical paths vs blob storage)

### Documentation

**Individual File Summaries Created:**
- `.docs/summary-terraphim_server-src-mcp_auth.rs.md` - Middleware implementation
- `.docs/summary-terraphim_server-tests-mcp_auth_tests.rs.md` - Test suite analysis
- `.docs/summary-terraphim_server-src-api_mcp.rs.md` - API handlers
- `.docs/summary-terraphim_persistence-src-mcp.rs.md` - Persistence layer
- `.docs/summary-tmp-retrospective_improvements.md.md` - Security retrospective

**External Documentation:**
- `/tmp/retrospective_improvements.md` - Complete incident analysis
- `/tmp/tdd_learnings.md` - TDD process documentation

### Future Enhancements (Phase 2)

- [ ] Rate limiting with `tower-governor`
- [ ] Constant-time comparison (prevent timing attacks)
- [ ] In-memory LRU cache for key verification
- [ ] Structured logging with `tracing`
- [ ] Prometheus metrics for monitoring

### GitHub Integration

- **Issue #285**: TDD Success Story
- **Commits**:
  - `b667597b` - Initial TDD implementation
  - `35c9cfc0` - CRITICAL SECURITY FIX
- **Comment**: https://github.com/terraphim/terraphim-ai/issues/285#issuecomment-3477816591

---

*Summary updated: 2025-11-02 - MCP Authentication implementation complete with critical security fixes*
