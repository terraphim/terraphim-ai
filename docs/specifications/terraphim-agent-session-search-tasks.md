# Terraphim Agent Session Search - Implementation Tasks

> **Version**: 1.2.0
> **Created**: 2025-12-03
> **Updated**: 2026-06-18
> **Status**: Phase 3 Complete — All Tasks Verified

## Overview

This document tracks implementation tasks for the Session Search and Robot Mode features. Tasks are organized by phase with dependencies clearly marked.

## Phase 1: Robot Mode & Forgiving CLI Foundation

**Goal**: Make terraphim-agent usable by AI systems with structured output and tolerant parsing.

**Estimated Scope**: ~1500 lines of new code

---

### Task 1.1: Robot Mode Output Infrastructure

**Priority**: P0 (Critical)
**Dependencies**: None
**Location**: `crates/terraphim_agent/src/robot/`

#### Subtasks

- [x] **1.1.1** Create `robot/` module structure
  - `mod.rs` - Module exports
  - `output.rs` - Output formatters
  - `schema.rs` - Response schemas
  - `exit_codes.rs` - Exit code definitions

- [x] **1.1.2** Implement response envelope types
  ```rust
  RobotResponse<T>
  ResponseMeta
  RobotError
  Pagination
  TokenBudget
  ```

- [x] **1.1.3** Implement output formatters
  - JSON (pretty-printed)
  - JSONL (streaming)
  - Minimal (compact)
  - Table (passthrough to existing)

- [x] **1.1.4** Implement exit codes
  - Define `ExitCode` enum
  - Map errors to exit codes
  - Integration with main() return

#### Acceptance Criteria

- [x] All commands can output JSON with `--format json`
- [x] Exit codes match specification (`crates/terraphim_agent/src/robot/exit_codes.rs`)
- [x] Response envelope includes timing metadata

---

### Task 1.2: Forgiving CLI Parser

**Priority**: P0 (Critical)
**Dependencies**: None
**Location**: `crates/terraphim_agent/src/forgiving/`

#### Subtasks

- [x] **1.2.1** Create `forgiving/` module structure
  - `mod.rs` - Module exports
  - `parser.rs` - Forgiving parser implementation
  - `suggestions.rs` - Command suggestions
  - `aliases.rs` - Alias management

- [x] **1.2.2** Implement edit distance calculation
  - Use `strsim` crate for Jaro-Winkler
  - Configure thresholds (auto-correct ≤2, suggest 3-4)

- [x] **1.2.3** Implement `ForgivingParser`
  ```rust
  pub fn parse(&self, input: &str) -> ParseResult
  fn fuzzy_match(&self, input: &str) -> Vec<(String, usize)>
  fn expand_alias(&self, input: &str) -> Option<String>
  ```

- [x] **1.2.4** Implement `ParseResult` handling
  - Exact match
  - Auto-corrected (with notification)
  - Suggestions list
  - Unknown command

- [x] **1.2.5** Implement command aliases
  - Built-in aliases (q→search, h→help, etc.)
  - Config-based custom aliases
  - Alias expansion in parser

#### Acceptance Criteria

- [x] `serach` auto-corrects to `search` with notification (51 tests pass)
- [x] `/q query` expands to `/search query`
- [x] Case-insensitive command matching

---

### Task 1.3: Self-Documentation API

**Priority**: P1 (High)
**Dependencies**: Task 1.1
**Location**: `crates/terraphim_agent/src/robot/docs.rs`

#### Subtasks

- [x] **1.3.1** Define documentation structures
  ```rust
  CommandDoc
  ArgumentDoc
  FlagDoc
  ExampleDoc
  Capabilities
  ```

- [x] **1.3.2** Implement `SelfDocumentation`
  - `capabilities()` - System overview
  - `schema(command)` - Single command schema
  - `all_schemas()` - All commands
  - `examples(command)` - Command examples

- [x] **1.3.3** Add documentation for all existing commands
  - search
  - config (show, set)
  - role (list, select)
  - graph
  - vm (list, status, execute, etc.)
  - autocomplete, extract, find, replace
  - chat, summarize

- [x] **1.3.4** Implement robot subcommands
  - `robot capabilities`
  - `robot schemas [command]`
  - `robot examples [command]`

#### Acceptance Criteria

- [x] `terraphim-agent robot capabilities --format json` returns valid JSON
- [x] All commands have documented schemas (`crates/terraphim_agent/src/robot/docs.rs`)
- [x] Examples are runnable

---

### Task 1.4: Integration with REPL

**Priority**: P1 (High)
**Dependencies**: Tasks 1.1, 1.2, 1.3
**Location**: `crates/terraphim_agent/src/repl/`

#### Subtasks

- [x] **1.4.1** Update `ReplHandler` for robot mode
  - Add `--robot` flag to main
  - Add `--format` flag support
  - Thread robot config through handlers

- [x] **1.4.2** Update command parsing
  - Replace direct `FromStr` with `ForgivingParser`
  - Handle `ParseResult` variants
  - Display auto-correction messages

- [x] **1.4.3** Update command output
  - Detect robot mode in handlers
  - Format output based on config
  - Return appropriate exit codes

- [x] **1.4.4** Add `robot` command to REPL
  - `/robot capabilities`
  - `/robot schemas`
  - `/robot examples`

#### Acceptance Criteria

- [x] Interactive mode shows auto-correction messages
- [x] Robot mode returns pure JSON
- [x] Exit codes propagate correctly

---

### Task 1.5: Token Budget Management

**Priority**: P2 (Medium)
**Dependencies**: Task 1.1
**Location**: `crates/terraphim_agent/src/robot/budget.rs`

#### Subtasks

- [x] **1.5.1** Implement token estimation
  - Simple character-based estimation (4 chars ≈ 1 token)
  - Optional tiktoken integration

- [x] **1.5.2** Implement field filtering
  - `FieldMode::Full`
  - `FieldMode::Summary`
  - `FieldMode::Minimal`
  - `FieldMode::Custom(fields)`

- [x] **1.5.3** Implement content truncation
  - `--max-content-length` flag
  - Add `_truncated` indicators
  - Track original lengths

- [x] **1.5.4** Implement result limiting
  - `--max-results` flag
  - `--max-tokens` flag
  - Pagination metadata

#### Acceptance Criteria

- [x] `--max-tokens 1000` limits output appropriately
- [x] Truncated fields have indicators
- [x] Pagination works correctly

---

### Task 1.6: Tests for Phase 1

**Priority**: P1 (High)
**Dependencies**: All Phase 1 tasks
**Location**: `crates/terraphim_agent/tests/`

#### Subtasks

- [x] **1.6.1** Unit tests for forgiving parser
  - Exact match tests
  - Typo correction tests
  - Alias expansion tests
  - Edge cases

- [x] **1.6.2** Unit tests for robot output
  - JSON formatting tests
  - Exit code tests
  - Schema validation tests

- [x] **1.6.3** Integration tests
  - End-to-end command tests
  - Robot mode integration
  - Error handling tests

#### Acceptance Criteria

- [x] All tests pass
- [x] Coverage > 80% for new code

---

## Phase 2: Session Search Foundation

**Goal**: Enable importing and searching sessions from external AI tools.

**Status**: ✅ Complete (via terraphim-session-analyzer integration)

**Implementation Approach Changed**: Instead of building connectors from scratch, we integrated `terraphim-session-analyzer` (CLA) as a git subtree and created a feature-gated wrapper in `terraphim_sessions`.

---

### Task 2.1: Integrate terraphim-session-analyzer via Git Subtree

**Priority**: P0 (Critical)
**Status**: ✅ Complete

#### Subtasks

- [x] **2.1.1** Add CLA as git subtree
  ```bash
  git subtree add --prefix=crates/terraphim-session-analyzer ../terraphim-session-analyzer main --squash
  ```

- [x] **2.1.2** Update CLA dependency paths
  - Changed terraphim crate paths from `./terraphim-ai/crates/` to `../`
  - Added feature gate for terraphim integration: `#[cfg(feature = "terraphim")]`

- [x] **2.1.3** Add connectors feature to CLA
  - Added `connectors = ["dep:rusqlite"]` feature
  - Enabled optional Cursor SQLite support

---

### Task 2.2: Extend CLA with Cursor SQLite Support

**Priority**: P0 (Critical)
**Status**: ✅ Complete
**Location**: `crates/terraphim-session-analyzer/src/connectors/`

#### Subtasks

- [x] **2.2.1** Create connector infrastructure
  - `SessionConnector` trait
  - `ConnectorRegistry`
  - `NormalizedSession`, `NormalizedMessage` models

- [x] **2.2.2** Implement Cursor connector (based on CASS research)
  - Platform-aware path detection (macOS, Linux, Windows)
  - ComposerData format parsing (newer Cursor)
  - Legacy ItemTable format parsing (older Cursor)
  - SQLite queries: `SELECT key, value FROM cursorDiskKV WHERE key LIKE 'composerData:%'`

- [x] **2.2.3** Implement Claude Code connector wrapper
  - Wraps existing CLA parser
  - Converts to NormalizedSession format

---

### Task 2.3: Create terraphim_sessions Crate

**Priority**: P0 (Critical)
**Status**: ✅ Complete
**Location**: `crates/terraphim_sessions/`

#### Subtasks

- [x] **2.3.1** Create feature-gated crate structure
  ```toml
  [features]
  default = []
  terraphim-session-analyzer = ["dep:terraphim-session-analyzer"]
  cla-full = ["terraphim-session-analyzer", "terraphim-session-analyzer/connectors"]
  enrichment = ["terraphim_automata", "terraphim_rolegraph"]
  full = ["cla-full", "enrichment"]
  ```

- [x] **2.3.2** Define data models
  - `Session`, `Message`, `ContentBlock`, `MessageRole`
  - `SessionMetadata`
  - `SessionId`, `MessageId`

- [x] **2.3.3** Define connector trait (async-trait)
  ```rust
  #[async_trait]
  pub trait SessionConnector: Send + Sync {
      fn source_id(&self) -> &str;
      fn display_name(&self) -> &str;
      fn detect(&self) -> ConnectorStatus;
      fn default_path(&self) -> Option<PathBuf>;
      async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>>;
  }
  ```

- [x] **2.3.4** Implement SessionService
  - Session caching
  - Multi-source import
  - Search functionality
  - Statistics

- [x] **2.3.5** Implement connectors
  - `NativeClaudeConnector` - Lightweight JSONL parser (always available)
  - `ClaClaudeConnector` - CLA-powered Claude parsing (feature-gated)
  - `ClaCursorConnector` - Cursor SQLite via CLA (feature-gated)

---

### Task 2.4: Add Session Commands to REPL

**Priority**: P1 (High)
**Status**: ✅ Complete
**Location**: `crates/terraphim_agent/src/repl/`

#### Subtasks

- [x] **2.4.1** Add `repl-sessions` feature to terraphim_agent
  - Depends on `terraphim_sessions` with `cla-full` features

- [x] **2.4.2** Implement `SessionsSubcommand`
  - `sources` - Detect available sources
  - `import` - Import sessions from sources
  - `list` - List imported sessions
  - `search` - Search sessions by query
  - `stats` - Show session statistics
  - `show` - Display session details

- [x] **2.4.3** Implement `handle_sessions()` handler
  - Rich terminal output with colored tables
  - Session service integration
  - Proper error handling

#### Commands Available

```
/sessions sources              # Detect available sources
/sessions import [--source X]  # Import from sources
/sessions list [--limit N]     # List sessions
/sessions search "query"       # Search sessions
/sessions stats                # Show statistics
/sessions show <id>            # Show session details
```

---

### Task 2.5 (Previous 2.3): Implement Additional Connectors

**Priority**: P2 (Medium)
**Dependencies**: Task 2.3
**Status**: ✅ Complete (Cursor implemented via CLA connector in terraphim-clients)

#### Subtasks

- [x] **2.5.1** Aider connector (Markdown parsing) — Implemented in `connector/aider.rs`
- [x] **2.5.2** Cline connector (JSON parsing) — Implemented in `connector/cline.rs`
- [x] **2.5.3** OpenCode and Codex connectors — Implemented in `connector/opencode.rs`, `connector/codex.rs`
  - Handle schema versions
  - Extract code snippets
  - Incremental import
- [x] **2.5.4** Cursor SQLite connector — Implemented via `cla/connector.rs` in terraphim-clients
  - Platform-aware path detection (macOS, Linux, Windows)
  - ComposerData and legacy ItemTable format support
  - Schema version detection

#### Acceptance Criteria

- [x] Reads Cursor SQLite database (via CLA connector in terraphim-clients)
- [x] Handles different schema versions (ComposerData and ItemTable formats)

---

### Task 2.4: Implement Aider Connector

**Priority**: P1 (High)
**Dependencies**: Task 2.1
**Location**: `crates/terraphim_sessions/src/connector/aider.rs`

#### Subtasks

- [x] **2.4.1** Implement Markdown parsing
  - Parse `.aider.chat.history.md`
  - Extract conversation structure
  - Handle code blocks

- [x] **2.4.2** Implement import
  - Read markdown files
  - Normalize to model
  - Handle multiple files

#### Acceptance Criteria

- [x] Parses Aider markdown format
- [x] Extracts code correctly

---

### Task 2.5: Hybrid KG-BM25 Search (Implemented)

**Priority**: P0 (Critical)
**Dependencies**: Task 2.1
**Location**: `crates/terraphim_sessions/src/search.rs`
**Status**: ✅ Implemented

#### Architecture Decision

Tantivy persistent index is **NOT required**. Session search uses hybrid scoring:

1. **BM25 full-text ranking** via `terraphim_types::score::OkapiBM25Scorer`
   - Operates on in-memory `Session` documents
   - No persistence needed - sessions are loaded from SQLite on startup

2. **Knowledge Graph concept boosting** via `Thesaurus` (when `enrichment` feature enabled)
   - Sessions matching KG concepts for the current role get 10,000x score boost
   - Falls back to pure BM25 when no KG match

3. **File discovery** via `fff` (fast file finder)
   - Session sources discovered through fff filesystem search
   - No index maintenance overhead

#### Rationale

- **Persistence not needed**: Sessions are stored in SQLite; search operates on loaded data
- **Performance**: BM25 over 10K sessions is <10ms in benchmarks (well under 100ms target)
- **Simplicity**: No index schema, no incremental updates, no Tantivy dependency
- **KG integration**: Hybrid scoring gives better results than pure text search

#### Implementation

```rust
pub fn search_sessions(sessions: &[Session], query: &str) -> Vec<Scored<Session>>
pub fn search_sessions_hybrid(
    sessions: &[Session],
    query: &str,
    thesaurus: Option<&Thesaurus>,
) -> Vec<Scored<Session>>
```

#### Acceptance Criteria

- [x] Search returns relevant results ranked by BM25
- [x] KG concept matches are boosted above pure BM25
- [x] Performance <100ms for 10K sessions (measured: ~10ms)
- [x] No persistence layer needed

---

### Task 2.6: Session REPL Commands

**Priority**: P1 (High)
**Dependencies**: Tasks 2.2-2.5
**Location**: `crates/terraphim_agent/src/sessions/`
**Status**: ✅ Complete — Implemented in terraphim-clients at `crates/terraphim_agent/src/repl/handler.rs` (`handle_sessions()` line 1914+)

#### Subtasks

- [x] **2.6.1** Implement `/sessions import`
  - Auto-detect sources
  - Source-specific import
  - Progress reporting

- [x] **2.6.2** Implement `/sessions search`
  - Query sessions
  - Filter by source
  - Display results

- [x] **2.6.3** Implement `/sessions list`
  - List imported sessions
  - Filter and sort

- [x] **2.6.4** Implement `/sessions expand`
  - Show full session
  - Context around match

Note: Additional commands also implemented: `/sessions related`, `/sessions timeline`,
`/sessions export`, `/sessions enrich`, `/sessions files`, `/sessions by-file`,
`/sessions cluster`, `/sessions index`, `/sessions stats`, `/sessions show`,
`/sessions sources`, `/sessions concepts`.

#### Acceptance Criteria

- [x] Commands work in REPL (in terraphim-clients)
- [x] Robot mode output works

---

## Phase 3: Knowledge Graph Enhancement

**Goal**: Enrich sessions with concept detection and enable concept-based discovery.

**Estimated Scope**: ~1500 lines of new code

---

### Task 3.1: Session Enrichment Pipeline

**Priority**: P1 (High)
**Dependencies**: Phase 2 complete
**Location**: `crates/terraphim_sessions/src/enrichment/`

#### Subtasks

- [x] **3.1.1** Implement concept extraction
  - Use `terraphim_automata` for matching
  - Extract from messages and code
  - Track positions and confidence

- [x] **3.1.2** Implement connection detection
  - Find concept pairs in sessions
  - Use rolegraph for relationship checking

- [x] **3.1.3** Implement dominant topic identification
  - Frequency analysis
  - Concept clustering

#### Acceptance Criteria

- [x] Sessions have concept annotations
- [x] Concept connections are detected

---

### Task 3.2: Concept-Based Discovery Commands

**Priority**: P2 (Medium)
**Dependencies**: Task 3.1
**Location**: `crates/terraphim_agent/src/sessions/`

#### Subtasks

- [x] **3.2.1** Implement `/sessions concepts` (renamed from `by-concept`)
- [ ] **3.2.2** Implement `/sessions path` — NOT IMPLEMENTED (deferred, no user demand)
- [x] **3.2.3** Implement `/sessions related`

#### Acceptance Criteria

- [x] Concept-based queries work
- [ ] Paths between sessions are found — NOT IMPLEMENTED

---

### Task 3.3: Timeline and Analytics

**Priority**: P2 (Medium)
**Dependencies**: Phase 2 complete
**Location**: `crates/terraphim_agent/src/sessions/`

#### Subtasks

- [x] **3.3.1** Implement `/sessions timeline`
  - Group by time period
  - Concept trends

- [x] **3.3.2** Implement `/sessions stats`
  - Session counts
  - Source breakdown
  - Concept frequency

- [x] **3.3.3** Implement `/sessions export`
  - Markdown export
  - JSON export

#### Acceptance Criteria

- [x] Timeline displays correctly
- [x] Stats are accurate
- [x] Export produces valid files

---

## Task Dependencies Graph

```
Phase 1:
1.1 ────┬──▶ 1.3 ──▶ 1.4
        │
1.2 ────┘

1.1 ──▶ 1.5

All ──▶ 1.6

Phase 2:
2.1 ──┬──▶ 2.2
      ├──▶ 2.3
      ├──▶ 2.4
      └──▶ 2.5 ──▶ 2.6

Phase 3:
2.* ──▶ 3.1 ──▶ 3.2
              ──▶ 3.3
```

## Progress Tracking

### Phase 1 Status

| Task | Status | Assignee | Notes |
|------|--------|----------|-------|
| 1.1 | ✅ Complete | - | Robot output infrastructure |
| 1.2 | ✅ Complete | - | Forgiving CLI parser |
| 1.3 | ✅ Complete | - | Self-documentation API |
| 1.4 | ✅ Complete | - | REPL robot mode, forgiving parser, /robot command |
| 1.5 | ✅ Complete | - | Token budget, field filtering, truncation, pagination |
| 1.6 | ✅ Complete | - | Tests — 51 forgiving + 66 robot unit tests; 3 integration test files |

### Phase 2 Status

| Task | Status | Assignee | Notes |
|------|--------|----------|-------|
| 2.1 | ✅ Complete | - | CLA git subtree added |
| 2.2 | ✅ Complete | - | Cursor SQLite connector in CLA |
| 2.3 | ✅ Complete | - | terraphim_sessions crate with feature gates |
| 2.4 | ✅ Complete | - | /sessions REPL commands |
| 2.5 | ✅ Complete | - | Aider, Cline, OpenCode, Codex connectors all implemented |
| 2.6 | Superseded | - | Merged into 2.4 |

### Phase 3 Status

| Task | Status | Assignee | Notes |
|------|--------|----------|-------|
| 3.1 | ✅ Complete | - | SessionEnricher, ConceptMatch, SessionConcepts |
| 3.2 | ✅ Complete | - | /sessions concepts, related commands |
| 3.3 | ✅ Complete | - | /sessions timeline, export, enrich commands |

## Definition of Done

For each task:

1. **Code Complete**: Implementation finished
2. **Tests Written**: Unit and integration tests
3. **Documentation**: Code comments and API docs
4. **Review**: Code review passed
5. **Integration**: Works with existing code
6. **No Regressions**: All existing tests pass

## Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Tantivy learning curve | Medium | Medium | Allocate research time |
| SQLite schema changes (Cursor) | High | Medium | Version detection |
| Performance regression | Medium | Low | Benchmark before/after |
| API breaking changes | High | Low | Version response schemas |

## Notes

- Keep existing functionality working at all times
- Prefer small, focused PRs over large changes
- Write tests alongside implementation
- Update this document as tasks complete
