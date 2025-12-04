# Terraphim Agent Session Search - Implementation Tasks

> **Version**: 1.1.0
> **Created**: 2025-12-03
> **Updated**: 2025-12-04
> **Status**: In Progress
> **Leverages**: Claude Code log ecosystem (clog, vibe-log-cli, claude-conversation-extractor)

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
**References**:
- [clog JSONL schema](https://github.com/HillviewCap/clog)
- [vibe-log-cli sanitization](https://github.com/vibe-log/vibe-log-cli)
- [claude-conversation-extractor parsing](https://github.com/ZeroSumQuant/claude-conversation-extractor)

#### Claude Code Log Format (from ecosystem analysis)

```json
{
  "parentUuid": "string",     // Thread relationship
  "sessionId": "string",       // Groups interactions
  "version": "string",
  "gitBranch": "string",
  "cwd": "string",             // Project path
  "message": {
    "role": "user|assistant",
    "content": [
      {"type": "text", "text": "..."},
      {"type": "tool_use", "id": "...", "name": "...", "input": {}},
      {"type": "tool_result", "tool_use_id": "...", "content": "..."}
    ],
    "usage": {
      "input_tokens": 0,
      "output_tokens": 0,
      "cache_creation_input_tokens": 0,
      "cache_read_input_tokens": 0
    }
  },
  "uuid": "string",
  "timestamp": "ISO-8601"
}
```

#### Subtasks

- [ ] **2.2.1** Implement detection
  - Check `~/.claude/projects/` exists (macOS/Linux)
  - Check `%USERPROFILE%\.claude\projects\` (Windows)
  - Count `chat_*.jsonl` files for estimate

- [ ] **2.2.2** Define JSONL data structures
  ```rust
  #[derive(Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct ClaudeLogEntry {
      uuid: String,
      parent_uuid: Option<String>,
      session_id: String,
      git_branch: Option<String>,
      cwd: Option<String>,
      timestamp: DateTime<Utc>,
      message: ClaudeMessage,
  }

  #[derive(Deserialize)]
  #[serde(tag = "type", rename_all = "snake_case")]
  enum ClaudeContent {
      Text { text: String },
      ToolUse { id: String, name: String, input: Value },
      ToolResult { tool_use_id: String, content: String },
  }
  ```

- [ ] **2.2.3** Implement JSONL parsing
  - Line-by-line streaming parser
  - Handle malformed lines with logging
  - Apply date filters (since/until)
  - Track token usage from `usage` field

- [ ] **2.2.4** Implement file watching (notify crate)
  - Watch `~/.claude/projects/*/*.jsonl`
  - Emit `SessionEvent` on file changes
  - Incremental re-indexing on modifications

- [ ] **2.2.5** Add tests with real fixtures
  - Create sample JSONL files from actual Claude Code format
  - Test content type parsing (text, tool_use, tool_result)
  - Test thread reconstruction via parentUuid
  - Test token usage extraction

#### Acceptance Criteria

- [ ] Detects Claude Code installation on all platforms
- [ ] Parses all content types correctly
- [ ] Token usage tracking works
- [ ] File watching enables real-time updates
- [ ] Handles malformed files gracefully with logging

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
| 1.1 | ✅ Complete | - | Robot output infrastructure implemented |
| 1.2 | ✅ Complete | - | Forgiving CLI parser with strsim |
| 1.3 | ✅ Complete | - | Self-documentation API |
| 1.4 | Partial | - | Robot command added, integration ongoing |
| 1.5 | Not Started | - | Token budget |
| 1.6 | ✅ Complete | - | 101 tests for Phase 1 modules |

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

---

## Ecosystem References

### Claude Code Log Analysis Tools

These tools were analyzed to inform our implementation:

| Tool | Language | Key Features | Repository |
|------|----------|--------------|------------|
| **clog** | JavaScript | JSONL schema, real-time monitoring, file watching | [HillviewCap/clog](https://github.com/HillviewCap/clog) |
| **vibe-log-cli** | Node.js | Privacy sanitization, productivity reports, Claude SDK | [vibe-log/vibe-log-cli](https://github.com/vibe-log/vibe-log-cli) |
| **claude-conversation-extractor** | Python | Export/search, 97% test coverage, cross-platform | [ZeroSumQuant/claude-conversation-extractor](https://github.com/ZeroSumQuant/claude-conversation-extractor) |
| **claude-code-history-viewer** | Rust/Tauri | Desktop app, activity heatmap, token analytics | [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer) |
| **cc-log-viewer** | Rust | Web interface, tool visualization, search | [crates.io/cc-log-viewer](https://crates.io/crates/cc-log-viewer) |

### Key Implementation Insights

1. **JSONL Format**: Claude Code stores logs in `~/.claude/projects/*/chat_*.jsonl`
2. **Threading**: `parentUuid` establishes conversation thread relationships
3. **Content Types**: Support `text`, `tool_use`, and `tool_result` content blocks
4. **Token Tracking**: `usage` field includes cache metrics for cost analysis
5. **Real-Time**: Use `notify` crate for file watching (File System Access API in web)
6. **Privacy**: Consider sanitization patterns from vibe-log-cli for sensitive data

### Sanitization Patterns (from vibe-log-cli)

For optional privacy mode:
- Code blocks → `[CODE_BLOCK_1: javascript]`
- API keys/tokens → `[CREDENTIAL_1]`
- File paths → `[PATH_1]`
- URLs → `[URL_1]`
- Emails → `[EMAIL_1]`
