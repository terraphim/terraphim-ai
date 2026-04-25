# Terraphim AI Project Summary (2026-04-25)

**Status**: Active development on task/860-f1-2-exit-codes (Phase F1.2 exit code handling)

## Project Overview

Terraphim AI is a privacy-first Rust-based agent system for semantic search, knowledge graph traversal, and AI-assisted task execution. The system operates as modular workspace with support for both server-connected (online) and embedded (offline) modes.

**Core Identity**:
- Language: Rust, async/tokio-based throughout
- Architecture: Modular cargo workspace with 30+ crates
- Deployment: Desktop (Tauri), CLI (terraphim-agent), server (terraphim_server)
- Performance: Sub-2s microVM boot (Firecracker), WASM targets for browser

## Current Work: Task #860 (F1.2 Exit Codes)

**Objective**: Achieve typed exit code consistency across all CLI paths.

**Changes**:
- `robot/exit_codes.rs`: Enum with 8 exit codes (0=Success, 1=ErrorGeneral, 2=ErrorUsage, 3=ErrorIndexMissing, 4=ErrorNotFound, 5=ErrorAuth, 6=ErrorNetwork, 7=ErrorTimeout)
- `main.rs`: Listen mode (`--server` flag rejection) now uses typed `ExitCode::ErrorUsage(2)` instead of bare `process::exit(1)`
- Added explicit `1 => ErrorGeneral` arm in `from_code()` for self-documentation

**Rationale**: Enables consistent error propagation for machine consumption (robot mode, shell integration, monitoring).

## Architecture Overview

### Workspace Structure
```
crates/
├── terraphim_agent/         # CLI + TUI binary
├── terraphim_server/        # HTTP API server
├── terraphim_firecracker/   # Secure VM executor
├── terraphim_types/         # Shared type definitions
├── terraphim_service/       # Core business logic
├── terraphim_persistence/   # Multi-backend storage abstraction
├── terraphim_config/        # Configuration system
├── terraphim_rolegraph/     # Knowledge graph implementation
├── terraphim_automata/      # Text matching, autocomplete (WASM-capable)
└── [23 more specialized crates]
```

### Execution Modes

1. **CLI/TUI** (`terraphim-agent`):
   - Interactive REPL with ratatui
   - Robot mode for machine consumption
   - Forgiving CLI with error tolerance
   - KG-based command validation
   - Time/token/procedure learning capture

2. **Server** (`terraphim_server`):
   - HTTP API for remote clients
   - Role-based configuration
   - Search orchestration across haystacks
   - Document management and summarization

3. **Desktop** (Tauri):
   - Rich UI for search and knowledge graph visualization
   - Real-time results with web socket support
   - Configuration management UI
   - MCP-based autocomplete integration

4. **Firecracker VMs** (`terraphim_firecracker`):
   - Sub-2s boot time
   - Secure execution isolation
   - Command output capture with UTF-8 safety

## Key Systems

### Error Handling & Exit Codes
- **Typed ExitCode enum**: 8 standard codes with descriptions, JSON names, and round-trip conversion
- **Consistency goal**: All error paths (listen mode, robot mode, error handling) use typed codes
- **Machine consumption**: Exit codes enable shell integration, monitoring, CI/CD pipelines

### Configuration System
- **Multi-layer**: CLI → settings.toml → persistence → defaults
- **Role-based**: Each role has specialized configuration (LLM provider, haystacks, relevance function)
- **Bootstrap pattern**: Initial config load, then persistence override

### Knowledge Graph System
- **RoleGraph**: Per-role semantic graph with Aho-Corasick automata for fast matching
- **Concepts**: Named entities with relationships, synonyms, URLs
- **Thesaurus**: Maps normalized terms to concepts with relevance scoring
- **Session weighting**: Recent session concepts boosted in relevance

### Persistence Layer
- **Multi-backend**: Memory → DashMap → SQLite → S3
- **Cache write-back**: Automatic promotion to faster backends after load
- **Compression**: Objects >1MB compressed via zstd before caching
- **Profile support**: Different backends per operation profile

## Rust Development Standards

**Code Quality**:
- Async/await with tokio throughout
- No mocks in tests (real implementations or fakes)
- Zero unsafe code in critical paths
- Pre-commit hooks: fmt, clippy, tests

**Performance**:
- Profile before optimizing (cargo bench, flamegraph)
- Reuse buffers, favor iterators, holist allocations
- WASM-compatible subset (no std::env, std::fs in WASM code)

**Error Handling**:
- Result<T, E> with thiserror/anyhow
- Propagate early, don't swallow errors
- Typed error codes for external consumption

## Testing Philosophy

**Approach**:
- Unit tests inline with code (`mod tests {}`)
- Integration tests in `tests/` for multi-crate checks
- No mocks; use fakes or real services
- `tokio::test` for async tests

**Coverage**:
- Target 80%+ on critical paths
- Edge cases from specification findings
- Integration points verified

## Recent Achievements (April 2026)

| Week | Focus | Status |
|------|-------|--------|
| Apr 21-25 | F1.2 Exit Codes, Phase 3-4 Mesh benchmarks, Issue #828 security audit | In Progress |
| Apr 14-20 | Phase 2 Implementation (Token Budget, Session Enrichment), ADF routing | Merged |
| Apr 7-13 | CI baseline restoration, KG validation research, Feature freeze fixes | Complete |

## Critical Files

| File | Purpose | Size | Last Updated |
|------|---------|------|--------------|
| `.docs/agents_instructions.json` | Consolidated project context | 13KB | Mar 17 |
| `CLAUDE.md` | Rust/async development standards | - | May 2025 |
| `lessons-learned.md` | Technical decision log | 1.3MB | Apr 25 |
| `crates/terraphim_agent/src/robot/exit_codes.rs` | Typed exit codes (Task #860) | - | Apr 25 |
| `crates/terraphim_agent/src/main.rs` | CLI entry point with exit code handling | - | Apr 25 |

## Known Constraints & TODOs

**Active Issues**:
- Security audit #880: Port 11434 (Ollama) globally exposed (P0, 72+ hours overdue)
- CVE mitigation: 4 unmaintained dependencies in deny.toml allow-list (serenity blocker for 44+ cycles)
- Drift detection: Config persistence unresolved 4+ days (Conduit, Issue #887)

**In Scope for F1.2**:
- ✅ Exit code typing in listen mode
- ✅ Explicit `from_code(1)` mapping
- → Phase 3: Full CLI path instrumentation (all error sites)
- → Phase 4: Integration tests for exit code contracts

## Build & Deployment

**Prerequisites**:
- Rust 1.75+ (WASM support requires 1.85+)
- Tokio async runtime
- Optional: Tauri/Node.js for desktop, wasm-pack for WASM targets

**Quick Start**:
```bash
cargo build --release
./target/release/terraphim-agent              # CLI/TUI
cargo run -p terraphim_server                 # HTTP server
cd desktop && yarn dev                        # Tauri desktop
wasm-pack build --target web crates/terraphim_automata  # WASM
```

**CI/CD**: GitHub Actions matrix (Linux/macOS/Windows), Docker multi-stage builds, auto-update releases.

## Next Steps (Post-Task #860)

1. **Phase 3**: Extend exit code usage to all CLI command paths (search, chat, config, etc.)
2. **Phase 4**: Integration tests validating exit code contracts for error scenarios
3. **Phase 5**: User-facing documentation for shell integration and monitoring

---

**Project Lead**: Alex Mikhalev (alex@zestic.ai)  
**Updated**: 2026-04-25  
**Branch**: task/860-f1-2-exit-codes (clean, ready for dispatch)
