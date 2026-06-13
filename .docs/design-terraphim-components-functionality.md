# Implementation Plan: Terraphim Components Full Functionality

**Status**: Draft
**Research Doc**: `.docs/research-terraphim-components-functionality.md`
**Author**: AI Agent
**Date**: 2026-06-13
**Estimated Effort**: 8-12 days

## Overview

### Summary
Bring four Terraphim AI components (terraphim-lsp, terraphim-rlm, terraphim-grep, terraphim-agent) to full functionality. The work spans from a complete greenfield LSP implementation, through a critical RLM security fix (KG validation bypass), to minor CI wiring and Gitea issue deduplication.

### Approach
Phased by priority: (1) Fix RLM KG validation security gap, (2) Build LSP from scratch, (3) Add session REPL commands, (4) Polish CI and executor bugs.

### Scope

**In Scope:**
- terraphim_lsp: Full LSP server with hover, completion, diagnostics for KG markdown
- terraphim_rlm: Wire KG validation into hot paths (rlm.rs, query_loop.rs)
- terraphim_rlm: Fix FirecrackerExecutor::cleanup() VM leak
- terraphim_rlm: Fix FirecrackerExecutor::list_snapshots()
- terraphim_grep: Add `code-search` feature to CI test matrix
- terraphim_agent: Implement /sessions import, search, list, expand REPL commands
- terraphim_agent: Fix firecracker.rs feature gate (#3011)
- Gitea: Deduplicate session/Cursor issues (batch close with explanation)

**Out of Scope:**
- Full Firecracker VM pool integration (ensure_pool -- requires deep fcctl-core changes)
- New LSP features beyond hover/completion/diagnostics (MVP scope)
- terraphim_agent TUI rewrite
- Cross-crate API refactoring
- E2E tests requiring KVM (Linux-only, gated behind `firecracker` feature on CI)

**Avoid At All Cost:**
- Adding a new LSP framework dependency that duplicates existing MCP infrastructure
- Refactoring RLM trait hierarchy (ExecutionEnvironment is adequate)
- Creating another session import mechanism (use existing terraphim_sessions infrastructure)
- Full-codebase audit beyond the four named components

## Architecture

### Component Diagram
```
terraphim_lsp (NEW)
    ├── tower-lsp (LSP protocol)
    ├── terraphim_automata (KG thesaurus matching)
    ├── terraphim_types (Thesaurus, RoleGraph types)
    └── terraphim_rolegraph (role graph traversal)

terraphim_rlm (FIX)
    ├── TerraphimRlm (public API)  ← ADD validate() before execute_code/execute_command
    ├── QueryLoop                  ← ADD validate() at command execution point
    ├── validator::KnowledgeGraphValidator (already implemented, unused)
    ├── FirecrackerExecutor        ← FIX validate(), cleanup(), list_snapshots()
    └── DockerExecutor             ← FIX validate() stub

terraphim_agent (ADD)
    └── REPL /sessions commands    ← NEW subcommands using terraphim_sessions crate

terraphim_grep (CI)
    └── CI workflow                ← ADD --features code-search to test matrix
```

### Data Flow

**LSP request flow (new):**
```
Editor → LSP request (hover/completion/diagnostic)
    → LSP server (tower-lsp)
    → Load thesaurus (terraphim_automata)
    → Match terms in document text
    → Return LSP response (hover info, completion items, diagnostics)
```

**RLM validation flow (fixed):**
```
execute_code/execute_command
    → executor.validate(input)  ← NEW: KG safety check
    → validator::KnowledgeGraphValidator
    → terraphim_automata (Aho-Corasick term matching)
    → Return: Valid (proceed) or Blocked (reject with KG concepts)
    → THEN execute in VM
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| tower-lsp for LSP | Standard async Rust LSP framework, tokio-native | lsp-server (sync, less ergonomic); custom JSON-RPC (redundant effort) |
| KG validation at rlm.rs API layer | Single point of enforcement; all entry points validated | Per-executor validation only (risk: new executors may forget) |
| Session REPL as feature-gated subcommand | Follows existing pattern (chat, file, web are all feature-gated) | Standalone binary (deployment complexity) |
| Firecracker cleanup: fix structural issue | The cleanup() method clears in-memory state but never calls stop_vm | Rewrite whole VM lifecycle (scope creep) |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| LSP as standalone binary crate | Adds deployment complexity; MCP is already embedded in RLM | Maintenance burden of separate deployment |
| Rewrite FirecrackerExecutor pool management | ensure_pool() requires deep fcctl-core integration | Weeks of work; unrelated to functionality gaps |
| Merge stalled RLM validation PRs as-is | Both PRs may conflict; cleaner to implement directly | Merge conflict resolution overhead |
| Add AI-powered LSP features (code generation, refactoring) | Violates MVP scope; speculative | Complexity without validated need |

### Simplicity Check

**What if this could be easy?**

- **LSP**: tower-lsp gives us a working server in <200 lines. The KG matching reuses terraphim_automata which is already battle-tested.
- **RLM validation**: The validator module already exists and works. It just needs to be called. One call site addition per path.
- **Sessions REPL**: terraphim_sessions already has import, search, list, and expand functionality. The REPL commands are thin wrappers.
- **Firecracker fixes**: cleanup() just needs to call existing stop_vm methods; list_snapshots() needs to actually call SnapshotManager.

**Senior Engineer Test**: No component requires architectural redesign. Each fix is localised to 1-3 files. The LSP is a greenfield add, not a refactor.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimisation

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_lsp/src/server.rs` | Tower-LSP server: initialise, hover, completion, diagnostic handlers |
| `crates/terraphim_lsp/src/kg_analysis.rs` | KG markdown analysis: term extraction, hover info, diagnostics |
| `crates/terraphim_lsp/src/completion.rs` | Completion provider: KG term suggestions |
| `crates/terraphim_lsp/src/diagnostics.rs` | Diagnostic provider: unknown term warnings, connectivity checks |
| `crates/terraphim_lsp/tests/lsp_integration_tests.rs` | LSP integration tests using lsp_types |
| `crates/terraphim_lsp/tests/fixtures/sample_kg.md` | Test fixture: sample KG markdown file |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_lsp/Cargo.toml` | Remove local Cargo.lock; bump edition to 2024; add deps: tower-lsp, tokio, serde_json, terraphim_automata, terraphim_types, terraphim_rolegraph |
| `crates/terraphim_lsp/src/lib.rs` | Replace placeholder: export server module, LSP initialisation function |
| `crates/terraphim_rlm/src/rlm.rs` | Add `self.executor.validate(code/command).await?` before execute_code/execute_command calls; wire KnowledgeGraphValidator into TerraphimRlm struct |
| `crates/terraphim_rlm/src/query_loop.rs` | Add `self.executor.validate(command).await?` at command execution point in execute() loop |
| `crates/terraphim_rlm/src/executor/firecracker.rs` | Replace validate() stub with actual terraphim_automata matching; fix cleanup() to call vm_manager.stop_vm(); fix list_snapshots() to call SnapshotManager |
| `crates/terraphim_rlm/src/executor/docker.rs` | Replace validate() stub with actual terraphim_automata matching |
| `crates/terraphim_rlm/src/executor/local.rs` | Replace validate() stub with actual terraphim_automata matching |
| `crates/terraphim_agent/src/repl/commands.rs` | Add Session subcommands (Import, Search, List, Expand) under feature gate |
| `crates/terraphim_agent/src/repl/handler.rs` | Add session command handlers |
| `crates/terraphim_agent/src/main.rs` | Wire session commands into CLI and REPL dispatch |
| `crates/terraphim_agent/src/repl/mod.rs` | Feature-gate sessions module |
| `.github/workflows/ci-main.yml` | Add `--features code-search` to terraphim_grep test step |

### Deleted Files

| File | Reason |
|------|--------|
| `crates/terraphim_lsp/Cargo.lock` | Conflicts with workspace Cargo.lock (orphaned from standalone build) |
| `crates/terraphim_lsp/target/` | Stale build artefacts from standalone build |

## API Design

### terraphim_lsp: Public Types

```rust
/// LSP server configuration
#[derive(Debug, Clone)]
pub struct LspServerConfig {
    /// Path to thesaurus for KG matching
    pub thesaurus_path: Option<PathBuf>,
    /// Role name for thesaurus scope
    pub role: Option<String>,
    /// Strictness level for diagnostics
    pub strictness: KgStrictness,
}

/// Analysis result for a KG markdown document
#[derive(Debug, Clone)]
pub struct KgAnalysis {
    /// Matched KG terms in the document
    pub matched_terms: Vec<TermMatch>,
    /// Unknown terms (not in KG)
    pub unknown_terms: Vec<String>,
    /// Hover information for matched terms
    pub hover_info: Vec<HoverInfo>,
    /// Suggested completions at cursor
    pub completions: Vec<CompletionItem>,
}

/// A matched term with position and KG concept info
#[derive(Debug, Clone)]
pub struct TermMatch {
    pub term: String,
    pub range: Range,
    pub concept: Option<String>,
    pub description: Option<String>,
}
```

### terraphim_lsp: Public Functions

```rust
/// Initialise and run the LSP server on stdin/stdout
///
/// # Arguments
/// * `config` - Server configuration including thesaurus path
///
/// # Returns
/// Ok if server shuts down cleanly
pub async fn run_lsp_server(config: LspServerConfig) -> Result<(), Box<dyn std::error::Error>>;

/// Analyse a KG markdown document for hover and diagnostic info
///
/// # Arguments
/// * `text` - Document text
/// * `thesaurus` - Loaded thesaurus for term matching
///
/// # Returns
/// Analysis result with matched terms, unknowns, and completions
pub fn analyse_kg_document(text: &str, thesaurus: &Thesaurus) -> KgAnalysis;
```

### terraphim_lsp: Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("thesaurus not loaded: {0}")]
    ThesaurusNotLoaded(String),

    #[error("KG analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("LSP protocol error: {0}")]
    ProtocolError(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}
```

### terraphim_rlm: Modified Public API (TerraphimRlm)

```rust
impl TerraphimRlm {
    // --- Existing methods modified ---

    /// Execute Python code in the session's VM.
    /// NOW VALIDATES against knowledge graph before execution.
    /// Returns RlmError::ValidationBlocked if KG check fails.
    pub async fn execute_code(
        &self,
        session_id: &SessionId,
        code: &str,
    ) -> RlmResult<ExecutionResult> {
        // NEW: Validate against KG before execution
        let validation = self.executor.validate(code).await.map_err(|e| {
            RlmError::ValidationFailed { message: e.to_string() }
        })?;
        if validation.is_blocked() {
            return Err(RlmError::ValidationBlocked {
                reason: validation.block_reason.unwrap_or_default(),
                suggestions: validation.suggestions,
            });
        }

        // ... existing session validation and execution logic
    }

    /// Execute a bash command in the session's VM.
    /// NOW VALIDATES against knowledge graph before execution.
    pub async fn execute_command(
        &self,
        session_id: &SessionId,
        command: &str,
    ) -> RlmResult<ExecutionResult> {
        // NEW: Validate against KG before execution
        let validation = self.executor.validate(command).await?;
        if validation.is_blocked() {
            return Err(RlmError::ValidationBlocked {
                reason: validation.block_reason.unwrap_or_default(),
                suggestions: validation.suggestions,
            });
        }

        // ... existing session validation and execution logic
    }
}
```

### terraphim_rlm: New Error Variants

```rust
#[derive(Debug, thiserror::Error)]
pub enum RlmError {
    // ... existing variants ...

    #[error("KG validation failed: {message}")]
    ValidationFailed { message: String },

    #[error("Command blocked by KG validation: {reason}")]
    ValidationBlocked {
        reason: String,
        suggestions: Vec<String>,
    },
}
```

### terraphim_agent: New REPL Commands

```rust
#[cfg(feature = "repl-sessions")]
pub enum ReplCommand {
    // ... existing commands ...

    /// Import sessions from external sources (Cursor, Claude, Aider)
    Sessions {
        subcommand: SessionsSubcommand,
    },
}

#[cfg(feature = "repl-sessions")]
pub enum SessionsSubcommand {
    /// Import sessions from configured sources
    Import {
        source: Option<String>,   // "cursor", "claude", "aider", or "all"
        path: Option<PathBuf>,    // Specific path (e.g. Cursor SQLite db)
    },
    /// Search imported sessions
    Search {
        query: String,
        limit: Option<usize>,
    },
    /// List imported sessions with metadata
    List {
        limit: Option<usize>,
        source: Option<String>,
    },
    /// Expand a session to full context
    Expand {
        session_id: String,
    },
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_analyse_kg_document_with_terms` | `crates/terraphim_lsp/src/kg_analysis.rs` | Verify KG term matching in markdown |
| `test_analyse_kg_document_no_terms` | `crates/terraphim_lsp/src/kg_analysis.rs` | Empty document handling |
| `test_completion_at_cursor` | `crates/terraphim_lsp/src/completion.rs` | Completion suggestions are correct |
| `test_diagnostics_unknown_terms` | `crates/terraphim_lsp/src/diagnostics.rs` | Unknown terms generate warnings |
| `test_validate_blocks_unknown_command` | `crates/terraphim_rlm/src/rlm.rs` | KG validation rejects unknown commands |
| `test_validate_allows_known_command` | `crates/terraphim_rlm/src/rlm.rs` | KG validation passes known commands |
| `test_execute_code_with_validation` | `crates/terraphim_rlm/src/rlm.rs` | execute_code calls validate first |
| `test_execute_command_with_validation` | `crates/terraphim_rlm/src/rlm.rs` | execute_command calls validate first |
| `test_query_loop_validates` | `crates/terraphim_rlm/src/query_loop.rs` | QueryLoop calls validate on commands |
| `test_firecracker_validate_with_kg` | `crates/terraphim_rlm/src/executor/firecracker.rs` | Firecracker validate uses automata |
| `test_docker_validate_with_kg` | `crates/terraphim_rlm/src/executor/docker.rs` | Docker validate uses automata |
| `test_firecracker_cleanup_stops_vms` | `crates/terraphim_rlm/src/executor/firecracker.rs` | Cleanup calls stop_vm |
| `test_firecracker_list_snapshots` | `crates/terraphim_rlm/src/executor/firecracker.rs` | list_snapshots returns actual data |
| `test_sessions_import_parses_args` | `crates/terraphim_agent/src/repl/commands.rs` | Sessions import command parsing |
| `test_sessions_search_dispatches` | `crates/terraphim_agent/src/repl/handler.rs` | Sessions search calls sessions crate |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_lsp_hover_on_kg_term` | `crates/terraphim_lsp/tests/lsp_integration_tests.rs` | Hover returns KG concept info |
| `test_lsp_completion_for_kg_terms` | `crates/terraphim_lsp/tests/lsp_integration_tests.rs` | Completion returns KG terms |
| `test_lsp_diagnostic_unknown_term` | `crates/terraphim_lsp/tests/lsp_integration_tests.rs` | Unknown terms flagged as warnings |
| `test_rlm_validation_e2e` | `crates/terraphim_rlm/tests/e2e_validation.rs` | Full RLM flow with KG validation gates |

### Property Tests

```rust
proptest! {
    #[test]
    fn kg_analysis_never_panics(text: String) {
        let thesaurus = Thesaurus::default();
        let _ = analyse_kg_document(&text, &thesaurus);
    }

    #[test]
    fn validate_never_panics_on_arbitrary_input(input: String) {
        let validator = KnowledgeGraphValidator::new(ValidatorConfig::default());
        let _ = validator.validate(&input);
    }
}
```

## Implementation Steps

### Step 1: terraphim_lsp -- Foundation (Cargo.toml + lib.rs fix)
**Files:** `crates/terraphim_lsp/Cargo.toml`, `crates/terraphim_lsp/src/lib.rs`
**Delete:** `crates/terraphim_lsp/Cargo.lock`
**Description:** Make the crate compilable from workspace root. Remove orphaned Cargo.lock, bump edition to 2024, add minimal dependencies (tower-lsp, tokio). Replace lib.rs placeholder with working boilerplate.
**Tests:** `cargo check -p terraphim_lsp` from workspace root
**Dependencies:** None (standalone)
**Estimated:** 1 hour

### Step 2: terraphim_lsp -- KG Analysis Engine
**Files:** `crates/terraphim_lsp/src/kg_analysis.rs` (new)
**Description:** Implement `analyse_kg_document()` using terraphim_automata for Aho-Corasick term matching. Extract matched terms, unknown terms, and hover info from markdown text.
**Tests:** Unit tests for term matching, comparison to test fixture
**Dependencies:** Step 1
**Estimated:** 3 hours

### Step 3: terraphim_lsp -- LSP Server (hover + completion + diagnostics)
**Files:** `crates/terraphim_lsp/src/server.rs`, `completion.rs`, `diagnostics.rs` (new)
**Description:** Implement tower-lsp server with textDocument/hover, textDocument/completion, textDocument/diagnostic handlers. Wire KG analysis engine into each handler.
**Tests:** Integration tests using lsp_types protocol messages against sample markdown
**Dependencies:** Step 2
**Estimated:** 5 hours

### Step 4: terraphim_rlm -- KG Validation on Hot Paths
**Files:** `crates/terraphim_rlm/src/rlm.rs`, `crates/terraphim_rlm/src/query_loop.rs`
**Description:** Add `self.executor.validate(code/command).await?` calls before execute_code and execute_command in TerraphimRlm. Add validate call in QueryLoop::execute() at command execution point. Add new RlmError variants (ValidationFailed, ValidationBlocked).
**Tests:** Unit tests verifying validate is called before execution (check that unknown commands are blocked)
**Dependencies:** None (independent)
**Estimated:** 4 hours

### Step 5: terraphim_rlm -- Implement Executor validate() Methods
**Files:** `crates/terraphim_rlm/src/executor/firecracker.rs`, `docker.rs`, `local.rs`
**Description:** Replace stub validate() implementations with actual KnowledgeGraphValidator usage. Each executor gets a reference to a shared validator (or creates one). The validator uses terraphim_automata for Aho-Corasick term matching.
**Tests:** Unit tests verifying that validate() returns non-empty results for known KG terms
**Dependencies:** Step 4 (for error types)
**Estimated:** 4 hours

### Step 6: terraphim_rlm -- Fix Firecracker Executor Bugs
**Files:** `crates/terraphim_rlm/src/executor/firecracker.rs`
**Description:**
- `cleanup()`: Iterate session_to_vm and call vm_manager.stop_vm() for each VM before clearing
- `list_snapshots()`: Call snapshot_manager.list_snapshots() instead of returning empty
**Tests:** Unit tests (KVM-gated)
**Dependencies:** None (independent)
**Estimated:** 3 hours

### Step 7: terraphim_agent -- Session REPL Commands
**Files:** `crates/terraphim_agent/src/repl/commands.rs`, `handler.rs`, `mod.rs`, `src/main.rs`
**Description:** Add Sessions subcommand enum with Import, Search, List, Expand variants. Wire into REPL dispatch and CLI argument parsing. Delegate to existing terraphim_sessions crate methods.
**Tests:** Command parsing tests, integration tests for each subcommand
**Dependencies:** None (uses existing terraphim_sessions infrastructure)
**Estimated:** 6 hours

### Step 8: terraphim_agent -- Fix Firecracker Feature Guard (#3011)
**Files:** `crates/terraphim_agent/src/...firecracker.rs` (find via grep)
**Description:** Add `#[cfg(feature = "firecracker")]` guards around firecracker API client method calls.
**Tests:** Compile check with and without firecracker feature
**Dependencies:** None
**Estimated:** 1 hour

### Step 9: terraphim_grep -- CI Feature Coverage
**Files:** `.github/workflows/ci-main.yml` or equivalent
**Description:** Add `--features code-search` to the terraphim_grep test step in CI config.
**Tests:** CI run on PR
**Dependencies:** None
**Estimated:** 0.5 hours

### Step 10: Gitea Issue Deduplication
**Files:** No code changes (Gitea operations only)
**Description:**
- Close duplicate Cursor SQLite connector issues (keep #2515 or #3530 as canonical)
- Close duplicate sessions import issues (keep #3396 as canonical)
- Close duplicate sessions expand issues (keep #3085 as canonical)
- Close duplicate learn procedure from-session issues (keep #3279 as canonical)
- Comment on each closed issue: "Consolidated into #[CANONICAL]. Closing duplicate."
**Tests:** Verify `gtr list-issues` shows reduced count
**Dependencies:** None
**Estimated:** 1 hour

### Step 11: Final Integration Testing
**Files:** All modified crates
**Description:** Run full test suite, clippy, fmt across all four crates. Verify no regressions.
**Tests:** `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cargo fmt --all -- --check`
**Dependencies:** All previous steps
**Estimated:** 2 hours

## Rollback Plan

If issues discovered:
1. Each crate is independently revertible (git revert the branch)
2. LSP server can be feature-gated behind `#[cfg(feature = "lsp")]` if needed
3. RLM validation can be made opt-in via a config flag during rollout
4. No database migrations required

Feature flags for safety:
- LSP: No feature flag needed (it's a new binary/crate, doesn't affect others)
- RLM validation: `RlmConfig::default().validation_enabled` flag (add ability to skip)
- Session REPL: Already gated behind `repl-sessions` feature

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| tower-lsp | 0.20 | LSP protocol implementation, tokio-native, widely used |
| lsp-types | 0.97 | LSP type definitions (used by tower-lsp transitively) |
| dashmap | 6.1 (already in workspace) | Thread-safe hashmap for LSP document state |

### No Dependency Updates Required
All crates use existing workspace dependencies. No version bumps needed.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| LSP hover response | < 50ms | Aho-Corasick is O(n) on text length |
| LSP completion | < 100ms | FST prefix lookup, sub-millisecond |
| RLM validation latency | < 50ms | Aho-Corasick on command text (< 10KB typical) |
| Session search | < 100ms | Already benchmark target from #2776 |

### Benchmarks to Add

```rust
#[bench]
fn bench_lsp_kg_analysis_10kb(b: &mut Bencher) {
    let text = include_str!("../tests/fixtures/sample_kg.md");
    let thesaurus = load_test_thesaurus();
    b.iter(|| analyse_kg_document(text, &thesaurus));
}

#[bench]
fn bench_rlm_validate_typical_command(b: &mut Bencher) {
    let validator = KnowledgeGraphValidator::new(ValidatorConfig::default());
    let command = "pip install requests && python script.py";
    b.iter(|| validator.validate(command));
}
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Which Cursor SQLite issue is canonical? | Needs decision | Human |
| LSP binary name (terraphim-lsp vs embedded in agent) | Needs decision | Human |
| RLM validation strictness default (Normal vs Permissive) | Needs decision | Human |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
