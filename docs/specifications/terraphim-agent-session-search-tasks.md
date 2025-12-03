# Terraphim Agent Session Search - Implementation Tasks

> **Version**: 1.0.0
> **Created**: 2025-12-03
> **Status**: In Progress

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

- [ ] All commands can output JSON with `--format json`
- [ ] Exit codes match specification
- [ ] Response envelope includes timing metadata

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

- [ ] `serach` auto-corrects to `search` with notification
- [ ] `/q query` expands to `/search query`
- [ ] Case-insensitive command matching

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

- [ ] `terraphim-agent robot capabilities --format json` returns valid JSON
- [ ] All commands have documented schemas
- [ ] Examples are runnable

---

### Task 1.4: Integration with REPL

**Priority**: P1 (High)
**Dependencies**: Tasks 1.1, 1.2, 1.3
**Location**: `crates/terraphim_agent/src/repl/`

#### Subtasks

- [ ] **1.4.1** Update `ReplHandler` for robot mode
  - Add `--robot` flag to main
  - Add `--format` flag support
  - Thread robot config through handlers

- [ ] **1.4.2** Update command parsing
  - Replace direct `FromStr` with `ForgivingParser`
  - Handle `ParseResult` variants
  - Display auto-correction messages

- [ ] **1.4.3** Update command output
  - Detect robot mode in handlers
  - Format output based on config
  - Return appropriate exit codes

- [ ] **1.4.4** Add `robot` command to REPL
  - `/robot capabilities`
  - `/robot schemas`
  - `/robot examples`

#### Acceptance Criteria

- [ ] Interactive mode shows auto-correction messages
- [ ] Robot mode returns pure JSON
- [ ] Exit codes propagate correctly

---

### Task 1.5: Token Budget Management

**Priority**: P2 (Medium)
**Dependencies**: Task 1.1
**Location**: `crates/terraphim_agent/src/robot/budget.rs`

#### Subtasks

- [ ] **1.5.1** Implement token estimation
  - Simple character-based estimation (4 chars ≈ 1 token)
  - Optional tiktoken integration

- [ ] **1.5.2** Implement field filtering
  - `FieldMode::Full`
  - `FieldMode::Summary`
  - `FieldMode::Minimal`
  - `FieldMode::Custom(fields)`

- [ ] **1.5.3** Implement content truncation
  - `--max-content-length` flag
  - Add `_truncated` indicators
  - Track original lengths

- [ ] **1.5.4** Implement result limiting
  - `--max-results` flag
  - `--max-tokens` flag
  - Pagination metadata

#### Acceptance Criteria

- [ ] `--max-tokens 1000` limits output appropriately
- [ ] Truncated fields have indicators
- [ ] Pagination works correctly

---

### Task 1.6: Tests for Phase 1

**Priority**: P1 (High)
**Dependencies**: All Phase 1 tasks
**Location**: `crates/terraphim_agent/tests/`

#### Subtasks

- [ ] **1.6.1** Unit tests for forgiving parser
  - Exact match tests
  - Typo correction tests
  - Alias expansion tests
  - Edge cases

- [ ] **1.6.2** Unit tests for robot output
  - JSON formatting tests
  - Exit code tests
  - Schema validation tests

- [ ] **1.6.3** Integration tests
  - End-to-end command tests
  - Robot mode integration
  - Error handling tests

#### Acceptance Criteria

- [ ] All tests pass
- [ ] Coverage > 80% for new code

---

## Phase 2: Session Search Foundation

**Goal**: Enable importing and searching sessions from external AI tools.

**Estimated Scope**: ~2500 lines of new code

---

### Task 2.1: Create terraphim_sessions Crate

**Priority**: P0 (Critical)
**Dependencies**: Phase 1 complete
**Location**: `crates/terraphim_sessions/`

#### Subtasks

- [ ] **2.1.1** Initialize crate
  ```bash
  cargo new --lib crates/terraphim_sessions
  ```
  - Add to workspace
  - Configure dependencies
  - Set up module structure

- [ ] **2.1.2** Define data models
  - `Session`
  - `Message`
  - `CodeSnippet`
  - `SessionMetadata`

- [ ] **2.1.3** Define connector trait
  ```rust
  pub trait SessionConnector: Send + Sync { ... }
  ```

- [ ] **2.1.4** Define service interface
  ```rust
  pub struct SessionService { ... }
  ```

#### Acceptance Criteria

- [ ] Crate compiles
- [ ] Models serialize/deserialize correctly
- [ ] Trait is implementable

---

### Task 2.2: Implement Claude Code Connector

**Priority**: P0 (Critical)
**Dependencies**: Task 2.1
**Location**: `crates/terraphim_sessions/src/connector/claude_code.rs`

#### Subtasks

- [ ] **2.2.1** Implement detection
  - Check `~/.claude/` exists
  - Count available sessions

- [ ] **2.2.2** Implement JSONL parsing
  - Parse session files
  - Extract messages
  - Handle code blocks

- [ ] **2.2.3** Implement import
  - Read sessions
  - Normalize to model
  - Return results

- [ ] **2.2.4** Add tests with fixtures
  - Create test JSONL files
  - Test parsing edge cases

#### Acceptance Criteria

- [ ] Detects Claude Code installation
- [ ] Imports sessions correctly
- [ ] Handles malformed files gracefully

---

### Task 2.3: Implement Cursor Connector

**Priority**: P1 (High)
**Dependencies**: Task 2.1
**Location**: `crates/terraphim_sessions/src/connector/cursor.rs`

#### Subtasks

- [ ] **2.3.1** Implement SQLite reading
  - Open Cursor database
  - Query conversation tables
  - Map to model

- [ ] **2.3.2** Implement import
  - Handle schema versions
  - Extract code snippets
  - Incremental import

#### Acceptance Criteria

- [ ] Reads Cursor SQLite database
- [ ] Handles different schema versions

---

### Task 2.4: Implement Aider Connector

**Priority**: P1 (High)
**Dependencies**: Task 2.1
**Location**: `crates/terraphim_sessions/src/connector/aider.rs`

#### Subtasks

- [ ] **2.4.1** Implement Markdown parsing
  - Parse `.aider.chat.history.md`
  - Extract conversation structure
  - Handle code blocks

- [ ] **2.4.2** Implement import
  - Read markdown files
  - Normalize to model
  - Handle multiple files

#### Acceptance Criteria

- [ ] Parses Aider markdown format
- [ ] Extracts code correctly

---

### Task 2.5: Implement Tantivy Index

**Priority**: P0 (Critical)
**Dependencies**: Task 2.1
**Location**: `crates/terraphim_sessions/src/index/`

#### Subtasks

- [ ] **2.5.1** Define index schema
  - Session fields
  - Message fields
  - Searchable/filterable configuration

- [ ] **2.5.2** Implement custom tokenizers
  - Edge n-gram for code
  - Standard for text

- [ ] **2.5.3** Implement writer
  - Add sessions
  - Batch commits
  - Incremental updates

- [ ] **2.5.4** Implement reader/search
  - Query parsing
  - Filtering
  - Result ranking

#### Acceptance Criteria

- [ ] Index creates and persists
- [ ] Search returns relevant results
- [ ] Performance meets targets (<100ms)

---

### Task 2.6: Session REPL Commands

**Priority**: P1 (High)
**Dependencies**: Tasks 2.2-2.5
**Location**: `crates/terraphim_agent/src/sessions/`

#### Subtasks

- [ ] **2.6.1** Implement `/sessions import`
  - Auto-detect sources
  - Source-specific import
  - Progress reporting

- [ ] **2.6.2** Implement `/sessions search`
  - Query sessions
  - Filter by source
  - Display results

- [ ] **2.6.3** Implement `/sessions list`
  - List imported sessions
  - Filter and sort

- [ ] **2.6.4** Implement `/sessions expand`
  - Show full session
  - Context around match

#### Acceptance Criteria

- [ ] Commands work in REPL
- [ ] Robot mode output works

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

- [ ] **3.1.1** Implement concept extraction
  - Use `terraphim_automata` for matching
  - Extract from messages and code
  - Track positions and confidence

- [ ] **3.1.2** Implement connection detection
  - Find concept pairs in sessions
  - Use rolegraph for relationship checking

- [ ] **3.1.3** Implement dominant topic identification
  - Frequency analysis
  - Concept clustering

#### Acceptance Criteria

- [ ] Sessions have concept annotations
- [ ] Concept connections are detected

---

### Task 3.2: Concept-Based Discovery Commands

**Priority**: P2 (Medium)
**Dependencies**: Task 3.1
**Location**: `crates/terraphim_agent/src/sessions/`

#### Subtasks

- [ ] **3.2.1** Implement `/sessions by-concept`
- [ ] **3.2.2** Implement `/sessions path`
- [ ] **3.2.3** Implement `/sessions related`

#### Acceptance Criteria

- [ ] Concept-based queries work
- [ ] Paths between sessions are found

---

### Task 3.3: Timeline and Analytics

**Priority**: P2 (Medium)
**Dependencies**: Phase 2 complete
**Location**: `crates/terraphim_agent/src/sessions/`

#### Subtasks

- [ ] **3.3.1** Implement `/sessions timeline`
  - Group by time period
  - Concept trends

- [ ] **3.3.2** Implement `/sessions stats`
  - Session counts
  - Source breakdown
  - Concept frequency

- [ ] **3.3.3** Implement `/sessions export`
  - Markdown export
  - JSON export

#### Acceptance Criteria

- [ ] Timeline displays correctly
- [ ] Stats are accurate
- [ ] Export produces valid files

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
| 1.1 | In Progress | - | Robot output infrastructure |
| 1.2 | In Progress | - | Forgiving CLI parser |
| 1.3 | In Progress | - | Self-documentation API |
| 1.4 | Not Started | - | REPL integration |
| 1.5 | Not Started | - | Token budget |
| 1.6 | Not Started | - | Tests |

### Phase 2 Status

| Task | Status | Assignee | Notes |
|------|--------|----------|-------|
| 2.1 | Not Started | - | |
| 2.2 | Not Started | - | |
| 2.3 | Not Started | - | |
| 2.4 | Not Started | - | |
| 2.5 | Not Started | - | |
| 2.6 | Not Started | - | |

### Phase 3 Status

| Task | Status | Assignee | Notes |
|------|--------|----------|-------|
| 3.1 | Not Started | - | |
| 3.2 | Not Started | - | |
| 3.3 | Not Started | - | |

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
