# Terraphim AI Project Summary

## Overview
Terraphim AI is a Rust-based AI agent system featuring a modular architecture with multiple crates, providing both online (server-connected) and offline (embedded) capabilities for knowledge work, document processing, and AI-assisted tasks.

## Major Update: PR #426 - RLM (Recursive Language Model) Orchestration

### 🎉 New Feature: Production-Ready RLM with Firecracker Isolation

**Merged**: March 31, 2026
**PR**: #426
**Status**: ✅ Complete and Fully Tested

Terraphim AI now includes a complete Recursive Language Model (RLM) implementation that combines the conceptual elegance of recursive LLM architectures with enterprise-grade security and isolation.

### Key Features

#### 1. **Multiple Isolation Backends**
- **Firecracker MicroVMs** (Primary): Full VM isolation with <500ms allocation
- **Docker Containers** (Fallback): gVisor/runsc support for enhanced isolation
- **Mock Executor** (Testing): Fast, deterministic execution for CI

#### 2. **Six MCP Tools for AI Integration**
1. `rlm_code` - Execute Python in isolated VM
2. `rlm_bash` - Execute bash commands in isolated VM
3. `rlm_query` - Query LLM recursively from VM context
4. `rlm_context` - Get/set session context and budget
5. `rlm_snapshot` - Create/restore VM snapshots
6. `rlm_status` - Get session status and history

#### 3. **Dual Budget System**
- Token budget: Maximum LLM tokens per session
- Time budget: Maximum wall-clock execution time
- Recursion depth limits
- Iteration limits

#### 4. **Knowledge Graph Validation**
- Strict mode: Reject unknown terms
- Normal mode: Warn with suggestions
- Permissive mode: Log only, never block
- Automatic retry with context escalation

### Technical Stats
- **144 tests passing** (132 unit + 9 integration + 3 doc tests)
- **6 MCP tools** fully functional
- **Firecracker v1.1.0** + KVM integration
- **Feature-gated**: `firecracker`, `docker-backend`, `e2b-backend`, `mcp`

### Architecture
```
TerraphimRlm (public API)
    ├── SessionManager (VM affinity, snapshots, extensions)
    ├── QueryLoop (command parsing, execution, result handling)
    ├── BudgetTracker (token counting, time tracking, depth limits)
    └── KnowledgeGraphValidator (term matching, retry, strictness)

ExecutionEnvironment trait
    ├── FirecrackerExecutor (primary, KVM-based VMs)
    ├── DockerExecutor (fallback, container isolation)
    └── MockExecutor (testing, CI-friendly)

MCP Integration
    └── RlmMcpService (6 tools for AI tool use)
```

### Usage

```rust
use terraphim_rlm::{TerraphimRlm, RlmConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RlmConfig::default();
    let rlm = TerraphimRlm::new(config).await?;

    let session = rlm.create_session().await?;
    let result = rlm.execute_code(&session, "print('Hello, RLM!')").await?;

    println!("Output: {}", result.stdout);
    Ok(())
}
```

### Documentation
- **Blog Post**: `.docs/blog-post-rlm-announcement.md`
- **Crate Location**: `crates/terraphim_rlm/`
- **Tests**: `cargo test -p terraphim_rlm --features firecracker,mcp`

---

## Architecture
- **Workspace Structure**: Cargo workspace with multiple member crates including `terraphim_agent`, `terraphim_server`, `terraphim_firecracker`, `terraphim_rlm`, and `terraphim_ai_nodejs`
- **Modular Design**: Separation of concerns between client communication, service logic, RLM orchestration, and core functionality
- **Async Runtime**: Built on Tokio for asynchronous operations throughout the codebase
- **Configuration System**: Multi-layer config loading with priority: CLI flags → settings.toml → persistence → embedded defaults

## Key Components

### Communication Layer (`terraphim_agent/src/client.rs`)
- HTTP client for server API communication
- Features configurable timeouts and user agent
- Role resolution that falls back to creating RoleName from raw string when not found in server config
- Supports chat, document summarization, thesaurus access, autocomplete, and VM management operations

### Service Layer (`terraphim_agent/src/service.rs`)
- TUI service managing application state and business logic
- Coordinates between configuration persistence and core TerraphimService
- Role resolution that returns error when role not found in config (contrasts with client.rs)
- Provides thesaurus-backed text operations, search, extraction, summarization, and connectivity checking
- Implements bootstrap-then-persistence pattern for role configuration

### RLM Layer (`crates/terraphim_rlm/`)
- Recursive Language Model orchestration
- Sandboxed code execution via Firecracker VMs
- Model Context Protocol (MCP) integration
- Session management with snapshots and budget tracking
- Knowledge graph validation with configurable strictness

### Dependencies & Tooling
- Core dependencies: tokio, reqwest, serde, thiserror, anyhow, async-trait
- RLM dependencies: fcctl-core (Firecracker), rmcp (MCP), bollard (Docker)
- Conditional compilation via cfg attributes for feature flags
- Cross-platform build management with specific exclusions
- Development tooling includes Clippy, Rustfmt, and custom validation scripts

## Notable Patterns & Characteristics
1. **Role Resolution Inconsistency**:
   - Online mode (client.rs): Falls back to raw role string when not found in config
   - Offline mode (service.rs): Returns error when role not found in config
   - This creates different behavior between connected and disconnected states

2. **Configuration Loading**: Sophisticated multi-source configuration system with fallback hierarchy

3. **Error Handling**: Consistent use of `anyhow::Result` and `thiserror` for error propagation

4. **Async/Await**: Extensive use of asynchronous patterns with proper timeout handling

5. **Feature Gating**: Heavy use of feature flags for optional components (firecracker, mcp, docker-backend)

6. **Testing Approach**: Emphasis on integration testing with real services rather than mocks

## Current Focus Areas
Based on recent documentation and code review activities:
- ✅ **RLM Implementation**: Complete with PR #426 merge
- Cross-mode consistency between online and offline operations
- Role resolution and validation improvements
- Test coverage and compilation fixes
- Documentation maintenance and accuracy
- Performance optimization and benchmarking

## Build & Development
- Standard Rust toolchain with cargo workspace
- Features: openrouter, mcp-rust-sdk, firecracker, mcp for conditional compilation
- Profile configurations for release, CI, and LTO-optimized builds
- Pre-commit hooks for code quality assurance

### RLM-Specific Commands
```bash
# Build RLM with Firecracker support
cargo build -p terraphim_rlm --features firecracker,mcp

# Run RLM tests
cargo test -p terraphim_rlm --features firecracker,mcp

# Run integration tests
cargo test -p terraphim_rlm --features firecracker,mcp --test integration_test

# Run code execution tests
cargo test -p terraphim_rlm --features firecracker,mcp --test code_execution_test
```
