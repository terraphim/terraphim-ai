# Research Document: Issue #611 - Sessions Files and By-File Subcommands

**Status**: Approved
**Author**: Claude Code
**Date**: 2026-03-11
**Reviewers**: Engineering Team

## Executive Summary

Issue #611 requests adding `sessions files <session-id>` and `sessions by-file <file-path>` subcommands to terraphim-agent. Research found that the session infrastructure already exists in `terraphim_sessions` crate with JSONL parsing capability. The implementation requires extending the `SessionsSubcommand` enum and `SessionService` to extract file paths from tool invocations.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Extends session analysis capabilities |
| Leverages strengths? | Yes | Builds on existing terraphim_sessions infrastructure |
| Meets real need? | Yes | Issue #611 explicitly requests this |

**Proceed**: Yes - 3/3 YES

---

## Problem Statement

### Description
Add file-level tracking to the `sessions` subcommand group:
1. `sessions files <session-id>` - List files touched by a specific session
2. `sessions by-file <file-path>` - List sessions that touched a given file path

### Impact
Enables developers to:
- Track which files were modified in a specific coding session
- Find all sessions that touched a particular file
- Understand code evolution across AI-assisted sessions

### Success Criteria
1. `sessions files <session-id>` extracts and displays file paths from tool invocations
2. `sessions by-file <file-path>` returns sessions touching that file (substring match)
3. Files are categorized as "read" vs "written"
4. Both commands support `--json` for programmatic use
5. Human-readable table output by default

---

## Current State Analysis

### Existing Implementation

**terraphim_sessions crate structure:**
```
crates/terraphim_sessions/
├── src/
│   ├── lib.rs
│   ├── model.rs           # Session, Message, ContentBlock
│   ├── service.rs         # SessionService
│   ├── connector/
│   │   └── native.rs      # Native JSONL connector
│   └── enrichment/
│       └── enricher.rs    # Session enrichment
```

**Session Model (model.rs:159-179):**
```rust
pub struct Session {
    pub id: SessionId,
    pub source: String,
    pub external_id: String,
    pub title: Option<String>,
    pub source_path: PathBuf,
    pub started_at: Option<jiff::Timestamp>,
    pub ended_at: Option<jiff::Timestamp>,
    pub messages: Vec<Message>,
    pub metadata: SessionMetadata,
}
```

**Message Model (model.rs:89-107):**
```rust
pub struct Message {
    pub idx: usize,
    pub role: MessageRole,
    pub author: Option<String>,
    pub content: String,
    pub blocks: Vec<ContentBlock>,  // Tool invocations here
    pub created_at: Option<jiff::Timestamp>,
    pub extra: serde_json::Value,
}
```

**ContentBlock (model.rs:57-76):**
```rust
pub enum ContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,  // File paths extracted from here
    },
    ToolResult { ... },
    Image { ... },
}
```

**Existing SessionsSubcommands (terraphim_agent/src/repl/commands.rs:155-194):**
```rust
pub enum SessionsSubcommand {
    Sources,
    Import { source, limit },
    List { source, limit },
    Search { query },
    Stats,
    Show { session_id },
    Concepts { concept },
    Related { session_id, min_shared },
    Timeline { group_by, limit },
    Export { format, output, session_id },
    Enrich { session_id },
    // NEW: Files { session_id, json }
    // NEW: ByFile { file_path, json }
}
```

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| SessionsSubcommand | `terraphim_agent/src/repl/commands.rs:155` | CLI command enum |
| handle_sessions | `terraphim_agent/src/repl/handler.rs:1762` | Command handler |
| SessionService | `terraphim_sessions/src/service.rs:14` | Business logic |
| Session model | `terraphim_sessions/src/model.rs:159` | Data structure |
| ContentBlock | `terraphim_sessions/src/model.rs:57` | Tool invocation data |

### Data Flow

```
terraphim-agent CLI
    ↓
SessionsSubcommand parsing (commands.rs)
    ↓
handle_sessions() (handler.rs)
    ↓
SessionService methods (terraphim_sessions)
    ↓
Session cache (HashMap<SessionId, Session>)
    ↓
Extract file paths from ContentBlock::ToolUse
```

---

## Constraints

### Technical Constraints
- Must extract file paths from `serde_json::Value` tool input
- Tool names vary by source (Claude Code, Cursor, etc.)
- File paths may be relative or absolute
- Must handle missing/invalid tool input gracefully

### Business Constraints
- Maintain backward compatibility with existing session commands
- Keep `--json` output consistent with other commands
- Don't break existing session parsing

### Non-Functional Requirements
| Requirement | Target |
|-------------|--------|
| Extraction latency | < 100ms per session |
| Memory overhead | Minimal (streaming extraction) |
| JSON output | Machine-parseable format |

---

## Vital Few

### Essential Constraints
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Correct file path extraction | Core feature requirement | Issue #611 |
| Read vs Write categorization | User needs to understand impact | Issue description |
| Support --json flag | Programmatic use case | Issue description |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| File content diff | Too complex for this issue |
| Git integration | Out of scope - just session tracking |
| Real-time file watching | Different feature entirely |

---

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_sessions | Core models and service | Low - stable API |
| terraphim_agent | CLI integration point | Low - existing patterns |
| comfy_table | Table formatting | Low - already used |

### External Dependencies
| Dependency | Version | Risk |
|------------|---------|------|
| serde_json | 1.0 | Low - for tool input parsing |

---

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Tool input format variations | Medium | Medium | Test with multiple sources |
| Performance on large sessions | Low | Low | Lazy extraction, caching |
| Path normalization issues | Medium | Low | Use PathBuf, canonicalize |

### Open Questions
1. Should we normalize file paths (absolute vs relative)? - Use as-is from tool input
2. How to handle glob patterns in paths? - Treat as literal strings

---

## Research Findings

### Key Insights
1. **Tool Input Structure**: File paths are in `ContentBlock::ToolUse.input` as `serde_json::Value`
2. **Tool Name Mapping**: Different tools use different field names:
   - Read: `file_path`
   - Edit/Write/MultiEdit: `file_path`
   - NotebookEdit: `notebook_path`
   - Glob/Grep: `path`
3. **Read vs Write**: Can be determined by tool name:
   - Read: `Read`, `Glob`, `Grep`
   - Write: `Edit`, `Write`, `MultiEdit`, `NotebookEdit`

### Tool Input Extraction Targets

| Tool | Input Field | Operation |
|------|-------------|-----------|
| Read | `tool_input.file_path` | read |
| Edit | `tool_input.file_path` | write |
| Write | `tool_input.file_path` | write |
| MultiEdit | `tool_input.file_path` | write |
| NotebookEdit | `tool_input.notebook_path` | write |
| Glob | `tool_input.path` | read |
| Grep | `tool_input.path` | read |

### Relevant Prior Art
- `terraphim_agent/src/repl/handler.rs:1762` - handle_sessions implementation
- `terraphim_sessions/src/service.rs:110` - search() method pattern
- `comfy_table` usage for table formatting

---

## Recommendations

### Proceed/No-Proceed
**Proceed** - Clear requirement, existing infrastructure, straightforward implementation.

### Scope
1. Add `Files` variant to `SessionsSubcommand`
2. Add `ByFile` variant to `SessionsSubcommand`
3. Implement file extraction logic in `terraphim_sessions`
4. Add handler methods in `terraphim_agent`
5. Support both table and JSON output

### Risk Mitigation
- Add unit tests for tool input parsing
- Test with real session files from multiple sources
- Handle malformed tool input gracefully

---

## Next Steps

1. Create design document with implementation steps
2. Add `FileAccess` struct and extraction methods to `terraphim_sessions`
3. Extend `SessionsSubcommand` enum
4. Implement handlers in `terraphim_agent`
5. Add tests and documentation

---

## Appendix

### Proposed Data Structures

```rust
/// File access record extracted from session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccess {
    pub path: String,
    pub operation: FileOperation,
    pub timestamp: Option<jiff::Timestamp>,
    pub tool_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperation {
    Read,
    Write,
}
```

### Proposed Service Methods

```rust
impl SessionService {
    /// Extract files accessed in a session
    pub async fn extract_files(&self, session_id: &SessionId) -> Vec<FileAccess>;

    /// Find sessions that accessed a file
    pub async fn sessions_by_file(&self, file_path: &str) -> Vec<Session>;
}
```
