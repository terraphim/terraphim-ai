# Implementation Plan: Issue #611 - Sessions Files and By-File Subcommands

**Status**: Draft
**Research Doc**: docs/research/research-issue-611.md
**Author**: Claude Code
**Date**: 2026-03-11
**Estimated Effort**: 4-6 hours

## Overview

### Summary
Add `sessions files <session-id>` and `sessions by-file <file-path>` subcommands to terraphim-agent. Extract file paths from tool invocations in session messages and categorize them as read vs written.

### Approach
Extend the existing session infrastructure:
1. Add data types (`FileAccess`, `FileOperation`) to `terraphim_sessions`
2. Add service methods (`extract_files`, `sessions_by_file`) to `SessionService`
3. Extend `SessionsSubcommand` enum and parsing in `terraphim_agent`
4. Add handler methods with table and JSON output

### Scope
**In Scope:**
- `sessions files <session-id> [--json]` subcommand
- `sessions by-file <file-path> [--json]` subcommand
- File path extraction from 7 tool types (Read, Edit, Write, MultiEdit, NotebookEdit, Glob, Grep)
- Read vs Write categorization
- Table output (default) and JSON output (--json)

**Out of Scope:**
- File content diff tracking
- Git integration
- Real-time file watching
- Path canonicalization/normalization

**Avoid At All Cost:**
- Complex path resolution (relative vs absolute)
- File system access (stat, exists checks)
- Content hashing or comparison

## Architecture

### Component Diagram
```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   CLI Parser    │────▶│ SessionsSubcommand│────▶│  Handler Logic  │
│  (commands.rs)  │     │   (commands.rs)   │     │  (handler.rs)   │
└─────────────────┘     └──────────────────┘     └────────┬────────┘
                                                         │
                              ┌────────────────────────┐
                              ▼
                    ┌──────────────────┐
                    │  SessionService  │
                    │ (terraphim_)     │
                    └────────┬─────────┘
                             │
                    ┌────────▼────────┐
                    │  Session Cache  │
                    │ (HashMap)       │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │ ContentBlock::  │
                    │ ToolUse parsing │
                    └─────────────────┘
```

### Data Flow
```
User Input: "sessions files abc123"
    ↓
Parse: SessionsSubcommand::Files { session_id: "abc123", json: false }
    ↓
Handler: handle_sessions() -> handle_files_subcommand()
    ↓
Service: session_service.extract_files("abc123")
    ↓
Extract: Iterate messages -> ContentBlock::ToolUse -> extract path
    ↓
Categorize: tool_name -> FileOperation::Read/Write
    ↓
Output: Table or JSON with file paths and operations
```

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Add types to terraphim_sessions | Single source of truth | Inline in handler |
| Simple string matching for paths | Avoids filesystem dependencies | Path canonicalization |
| Tool name determines read/write | Clear categorization | Analyzing tool output |

### Simplicity Check
**What if this could be easy?**
- Extract paths from JSON, no filesystem access
- Tool name determines operation type
- Simple substring matching for by-file search

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `terraphim_sessions/src/model.rs` | Add `FileAccess`, `FileOperation` types |
| `terraphim_sessions/src/service.rs` | Add `extract_files()`, `sessions_by_file()` |
| `terraphim_agent/src/repl/commands.rs` | Extend `SessionsSubcommand`, add parsing |
| `terraphim_agent/src/repl/handler.rs` | Add handler methods for new subcommands |

## API Design

### Public Types
```rust
/// File access operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOperation {
    Read,
    Write,
}

impl Display for FileOperation { ... }

/// Record of a file access in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccess {
    pub path: String,
    pub operation: FileOperation,
    pub timestamp: Option<jiff::Timestamp>,
    pub tool_name: String,
}
```

### Public Functions
```rust
impl SessionService {
    /// Extract all file accesses from a session
    pub async fn extract_files(&self, session_id: &SessionId) -> Vec<FileAccess>;

    /// Find sessions that accessed a file (substring match)
    pub async fn sessions_by_file(&self, file_path: &str) -> Vec<(Session, Vec<FileAccess>)>;
}

impl Session {
    /// Extract file accesses from this session
    pub fn extract_file_accesses(&self) -> Vec<FileAccess>;
}
```

### Command Parsing
```rust
pub enum SessionsSubcommand {
    // ... existing variants

    /// List files touched by a session
    Files {
        session_id: String,
        json: bool,
    },

    /// Find sessions that touched a file
    ByFile {
        file_path: String,
        json: bool,
    },
}
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_extract_files_read` | `terraphim_sessions/model.rs` | Verify Read tool extraction |
| `test_extract_files_write` | `terraphim_sessions/model.rs` | Verify Write tool extraction |
| `test_extract_files_notebook` | `terraphim_sessions/model.rs` | Verify notebook_path extraction |
| `test_sessions_by_file` | `terraphim_sessions/service.rs` | Verify file search |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_sessions_files_command` | `terraphim_agent/tests` | Full CLI flow |
| `test_sessions_byfile_command` | `terraphim_agent/tests` | Full CLI flow |

## Implementation Steps

### Step 1: Add FileAccess Types to terraphim_sessions
**Files:** `crates/terraphim_sessions/src/model.rs`
**Description:** Add `FileOperation` enum and `FileAccess` struct
**Tests:** Unit tests for type construction
**Estimated:** 30 minutes

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOperation {
    Read,
    Write,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccess {
    pub path: String,
    pub operation: FileOperation,
    pub timestamp: Option<jiff::Timestamp>,
    pub tool_name: String,
}
```

### Step 2: Add File Extraction to Session Model
**Files:** `crates/terraphim_sessions/src/model.rs`
**Description:** Implement `extract_file_accesses()` method on `Session`
**Tests:** Test with various tool input formats
**Dependencies:** Step 1
**Estimated:** 1 hour

```rust
impl Session {
    pub fn extract_file_accesses(&self) -> Vec<FileAccess> {
        // Iterate messages, find ToolUse blocks, extract paths
        // Return Vec<FileAccess>
    }
}
```

### Step 3: Add Service Methods
**Files:** `crates/terraphim_sessions/src/service.rs`
**Description:** Add `extract_files()` and `sessions_by_file()` to `SessionService`
**Tests:** Unit tests with test sessions
**Dependencies:** Step 2
**Estimated:** 45 minutes

```rust
impl SessionService {
    pub async fn extract_files(&self, session_id: &SessionId) -> Vec<FileAccess> {
        self.get_session(session_id).await
            .map(|s| s.extract_file_accesses())
            .unwrap_or_default()
    }

    pub async fn sessions_by_file(&self, file_path: &str) -> Vec<(Session, Vec<FileAccess>)> {
        // Search all sessions, return those with matching file paths
    }
}
```

### Step 4: Extend SessionsSubcommand
**Files:** `crates/terraphim_agent/src/repl/commands.rs`
**Description:** Add `Files` and `ByFile` variants, update parsing
**Tests:** Command parsing tests
**Dependencies:** None
**Estimated:** 45 minutes

### Step 5: Add Handler Methods
**Files:** `crates/terraphim_agent/src/repl/handler.rs`
**Description:** Add handlers for `Files` and `ByFile` subcommands
**Tests:** Integration with SessionService
**Dependencies:** Steps 3, 4
**Estimated:** 1 hour

```rust
async fn handle_sessions(&mut self, subcommand: SessionsSubcommand) -> Result<()> {
    match subcommand {
        // ... existing handlers
        SessionsSubcommand::Files { session_id, json } => {
            self.handle_files(session_id, json).await
        }
        SessionsSubcommand::ByFile { file_path, json } => {
            self.handle_by_file(file_path, json).await
        }
    }
}
```

### Step 6: Add Unit Tests
**Files:** `crates/terraphim_sessions/src/model.rs`, `service.rs`
**Description:** Comprehensive tests for extraction logic
**Estimated:** 45 minutes

### Step 7: Verification
**Command:** `cargo check --workspace`
**Command:** `cargo test -p terraphim_sessions`
**Command:** `cargo test -p terraphim_agent --features repl-sessions`
**Estimated:** 15 minutes

## Rollback Plan

If issues discovered:
1. Revert changes to the 4 modified files
2. Keep research and design documents for future reference

## Dependencies

No new dependencies required.

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Extraction time | < 10ms per session | Benchmark with 1000 message session |
| Memory overhead | O(n) where n = file accesses | Profiling |
| Search time | < 100ms for 1000 sessions | Benchmark |

## Open Items

None.

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
