# Terraphim Agent Session Search - Feature Specification

> **Version**: 1.1.0
> **Status**: Draft
> **Created**: 2025-12-03
> **Updated**: 2025-12-04
> **Inspired by**: [Coding Agent Session Search (CASS)](https://github.com/Dicklesworthstone/coding_agent_session_search)
> **Leverages**: Claude Code log ecosystem (clog, vibe-log-cli, claude-conversation-extractor)

## Executive Summary

This specification defines enhancements to `terraphim-agent` that enable cross-agent session search, AI-friendly CLI interfaces, and knowledge graph-enhanced session analysis. The goal is to unify coding assistant history across multiple tools while leveraging Terraphim's unique knowledge graph capabilities.

## Problem Statement

### Current Limitations

1. **Fragmented Knowledge**: Developers use multiple AI coding assistants (Claude Code, Cursor, Copilot, Aider, Cline). Solutions discovered in one tool are invisible to others.

2. **AI Integration Barriers**: Current CLI is designed for humans, not AI agents. Lacks structured output, tolerant parsing, and self-documentation.

3. **No Session Persistence**: `terraphim-agent` maintains command history but no conversation/session tracking or cross-session search.

4. **Limited Discoverability**: Past solutions are hard to find without remembering exact terms used.

## Goals

| Goal | Description | Success Metric |
|------|-------------|----------------|
| **G1** | Enable search across all AI coding assistant sessions | Search latency <100ms for 10K sessions |
| **G2** | Make CLI usable by AI agents | Zero parse failures from typos |
| **G3** | Self-documenting API | Complete JSON schema for all commands |
| **G4** | Knowledge graph enrichment | Connect sessions via shared concepts |
| **G5** | Token-aware output | Precise control over response size |

## Non-Goals

- Real-time sync with cloud services (privacy-first, local only)
- Training or fine-tuning models on session data
- Replacing existing search functionality (augmenting it)

---

## Feature Specifications

### F1: Robot Mode

#### F1.1 Structured Output

**Description**: All commands support machine-readable output formats.

**Formats**:
- `json`: Pretty-printed JSON (default for robot mode)
- `jsonl`: Newline-delimited JSON for streaming
- `table`: Human-readable tables (default for interactive)
- `minimal`: Compact single-line JSON

**Syntax**:
```bash
terraphim-agent robot <command> [args] --format <format>
terraphim-agent --robot search "query"  # Shorthand
```

**Output Schema**:
```json
{
  "success": true,
  "meta": {
    "command": "search",
    "elapsed_ms": 42,
    "timestamp": "2025-12-03T10:30:00Z",
    "version": "0.1.0"
  },
  "data": { ... },
  "errors": []
}
```

**Error Schema**:
```json
{
  "success": false,
  "meta": { ... },
  "data": null,
  "errors": [
    {
      "code": "E001",
      "message": "Index not found",
      "details": "Session index has not been initialized",
      "suggestion": "Run: terraphim-agent sessions init"
    }
  ]
}
```

#### F1.2 Exit Codes

| Code | Name | Description |
|------|------|-------------|
| 0 | `SUCCESS` | Operation completed successfully |
| 1 | `ERROR_GENERAL` | Unspecified error |
| 2 | `ERROR_USAGE` | Invalid arguments or syntax |
| 3 | `ERROR_INDEX_MISSING` | Required index not initialized |
| 4 | `ERROR_NOT_FOUND` | No results for query |
| 5 | `ERROR_AUTH` | Authentication required |
| 6 | `ERROR_NETWORK` | Network/connectivity issue |
| 7 | `ERROR_TIMEOUT` | Operation timed out |

#### F1.3 Token Budget Management

**Description**: Control output size for LLM context windows.

**Parameters**:
- `--max-tokens <n>`: Maximum tokens in response (estimated)
- `--max-results <n>`: Maximum number of results
- `--max-content-length <n>`: Truncate content fields at n characters
- `--fields <mode>`: Field selection mode

**Field Modes**:
- `full`: All fields including body content
- `summary`: title, url, description, score, concepts
- `minimal`: title, url, score only
- `custom:field1,field2,...`: Specific fields

**Truncation Indicators**:
```json
{
  "body": "First 500 characters of content...",
  "body_truncated": true,
  "body_original_length": 15000
}
```

---

### F2: Forgiving CLI

#### F2.1 Typo Tolerance

**Description**: Auto-correct command typos using edit distance matching.

**Algorithm**: Jaro-Winkler similarity (existing in `terraphim_automata`)

**Thresholds**:
- Edit distance ≤ 2: Auto-correct with notification
- Edit distance 3-4: Suggest alternatives, don't auto-correct
- Edit distance > 4: Treat as unknown command

**Behavior**:
```
$ terraphim-agent serach "query"
⚡ Auto-corrected: serach → search

[search results...]
```

**Robot Mode Behavior**:
```json
{
  "meta": {
    "auto_corrected": true,
    "original_command": "serach",
    "corrected_command": "search"
  }
}
```

#### F2.2 Command Aliases

**Built-in Aliases**:
| Alias | Canonical Command |
|-------|-------------------|
| `/q`, `/query`, `/find` | `/search` |
| `/h`, `/?` | `/help` |
| `/c` | `/config` |
| `/r` | `/role` |
| `/s` | `/sessions` |
| `/ac` | `/autocomplete` |

**Custom Aliases** (via config):
```toml
[aliases]
ss = "sessions search"
si = "sessions import"
```

#### F2.3 Argument Flexibility

**Features**:
- Case-insensitive flags: `--Format` = `--format`
- Flag value separators: `--format=json` = `--format json`
- Boolean flag variations: `--verbose`, `-v`, `--verbose=true`
- Quoted argument handling: `"multi word query"` or `'multi word query'`

---

### F3: Self-Documentation API

#### F3.1 Capabilities Endpoint

**Command**: `terraphim-agent robot capabilities`

**Output**:
```json
{
  "name": "terraphim-agent",
  "version": "0.1.0",
  "description": "Privacy-first AI assistant with knowledge graph search",
  "features": {
    "session_search": true,
    "knowledge_graph": true,
    "llm_chat": true,
    "vm_execution": true
  },
  "commands": ["search", "sessions", "config", "role", ...],
  "supported_formats": ["json", "jsonl", "table", "minimal"],
  "index_status": {
    "sessions_indexed": 1234,
    "last_updated": "2025-12-03T10:00:00Z"
  }
}
```

#### F3.2 Schema Documentation

**Command**: `terraphim-agent robot schemas [command]`

**Output** (for search):
```json
{
  "command": "search",
  "description": "Search documents and sessions",
  "arguments": [
    {
      "name": "query",
      "type": "string",
      "required": true,
      "description": "Search query with optional operators"
    }
  ],
  "flags": [
    {
      "name": "--role",
      "short": "-r",
      "type": "string",
      "default": "current",
      "description": "Role context for search"
    },
    {
      "name": "--limit",
      "short": "-l",
      "type": "integer",
      "default": 10,
      "description": "Maximum results to return"
    }
  ],
  "examples": [
    {
      "description": "Basic search",
      "command": "search \"async error handling\""
    },
    {
      "description": "Search with role",
      "command": "search \"database migration\" --role DevOps"
    }
  ],
  "response_schema": { ... }
}
```

#### F3.3 Examples Endpoint

**Command**: `terraphim-agent robot examples [command]`

Provides runnable examples with expected outputs.

---

### F4: Session Search & Indexing

#### F4.1 Session Connectors

**Supported Sources**:

| Source | Format | Location | Notes |
|--------|--------|----------|-------|
| Claude Code | JSONL | `~/.claude/projects/*/chat_*.jsonl` | Well-documented, primary target |
| Cursor | SQLite | `~/.cursor/` | Proprietary format |
| Aider | Markdown | `.aider.chat.history.md` | Per-project history |
| Cline | JSON | `~/.cline/` | Similar to Claude Code |
| OpenCode | JSONL | `~/.opencode/` | Follows Claude Code format |
| Codex | JSONL | `~/.codex/` | Follows Claude Code format |

**Platform-Specific Paths**:
- **macOS/Linux**: `~/.claude/projects/*/chat_*.jsonl`
- **Windows**: `%USERPROFILE%\.claude\projects\*\chat_*.jsonl`

**Claude Code JSONL Schema** (from [clog](https://github.com/HillviewCap/clog)):
```json
{
  "parentUuid": "string",
  "sessionId": "string",
  "version": "string",
  "gitBranch": "string",
  "cwd": "string",
  "message": {
    "role": "user|assistant",
    "content": [
      {
        "type": "text|tool_use|tool_result",
        "text": "string",
        "name": "string",
        "input": {},
        "tool_use_id": "string"
      }
    ],
    "usage": {
      "input_tokens": 0,
      "output_tokens": 0,
      "cache_creation_input_tokens": 0,
      "cache_read_input_tokens": 0
    }
  },
  "uuid": "string",
  "timestamp": "ISO-8601",
  "toolUseResult": {}
}
```

**Key Insights from Ecosystem Analysis**:
- `parentUuid` establishes conversation thread relationships
- `sessionId` groups related interactions
- Token metrics track computational resource usage
- Tool interactions maintain bidirectional references (`tool_use_id`)
- Content types include: `text`, `tool_use`, `tool_result`

**Connector Interface**:
```rust
pub trait SessionConnector: Send + Sync {
    /// Source identifier
    fn source_id(&self) -> &str;

    /// Detect if source exists on this system
    async fn detect(&self) -> bool;

    /// Import sessions from source
    async fn import(&self, options: ImportOptions) -> Result<Vec<Session>>;

    /// Watch for new sessions (optional)
    async fn watch(&self) -> Option<mpsc::Receiver<Session>>;
}
```

#### F4.2 Session Data Model

**Internal Model** (normalized from source formats):
```rust
/// A session containing a conversation thread
pub struct Session {
    pub id: Uuid,
    pub source: String,              // "claude-code", "cursor", etc.
    pub source_session_id: String,   // Original sessionId from source
    pub project_path: Option<PathBuf>,
    pub git_branch: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
    pub metadata: SessionMetadata,
}

/// Individual message in a session
pub struct Message {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,     // parentUuid for threading
    pub role: MessageRole,           // User, Assistant, System
    pub content: Vec<ContentBlock>,  // Supports multiple content types
    pub timestamp: DateTime<Utc>,
    pub token_usage: Option<TokenUsage>,
    pub concepts: Vec<String>,       // Extracted via knowledge graph
}

/// Content block types (matches Claude Code format)
#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool
    },
    CodeSnippet(CodeSnippet),
}

/// Code extracted from messages
pub struct CodeSnippet {
    pub language: Option<String>,
    pub content: String,
    pub file_path: Option<String>,
    pub line_range: Option<(usize, usize)>,
}

/// Token usage tracking (from Claude Code logs)
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cache_creation_input_tokens: usize,
    pub cache_read_input_tokens: usize,
}

/// Session metadata for indexing and filtering
pub struct SessionMetadata {
    pub project_path: Option<PathBuf>,
    pub git_branch: Option<String>,
    pub tags: Vec<String>,
    pub total_tokens: usize,
    pub message_count: usize,
    pub has_code: bool,
    pub languages: Vec<String>,
    pub tools_used: Vec<String>,     // List of tools invoked
}
```

#### F4.3 Session Index

**Technology**: Tantivy (Rust full-text search, same as CASS)

**Index Schema**:
```rust
pub struct SessionIndexSchema {
    // Identifiers
    session_id: Field,
    message_id: Field,
    source: Field,

    // Searchable content
    content: Field,          // Full message content
    code_content: Field,     // Code snippets only

    // Filterable metadata
    timestamp: Field,
    role: Field,
    language: Field,
    project_path: Field,

    // Knowledge graph enrichment
    concepts: Field,         // Extracted concepts
}
```

**Tokenization**:
- Edge n-gram for code patterns (handles `snake_case`, `camelCase`, symbols)
- Standard tokenizer for natural language
- Language-specific tokenizers for code

#### F4.4 Session Commands

```bash
# Import sessions
/sessions import                     # Auto-detect all sources
/sessions import --source claude-code
/sessions import --source cursor --since "2024-01-01"

# Search sessions
/sessions search "authentication"
/sessions search "error handling" --source cursor --limit 20

# Timeline and analysis
/sessions timeline --group-by day --last 30d
/sessions stats
/sessions analyze --show concepts

# Export
/sessions export --format markdown --output sessions.md
/sessions export --session-id <uuid> --format json
```

---

### F5: Knowledge Graph Enhancement

#### F5.1 Session Enrichment

**Process**:
1. On import, extract text from messages
2. Run through `terraphim_automata` to identify concepts
3. Store concept matches with sessions
4. Update `RoleGraph` with session-concept relationships

**Enrichment Data**:
```rust
pub struct SessionEnrichment {
    pub session_id: Uuid,
    pub concepts: Vec<ConceptMatch>,
    pub concept_connections: Vec<(String, String)>,  // Concept pairs found
    pub dominant_topics: Vec<String>,
}

pub struct ConceptMatch {
    pub concept: String,
    pub occurrences: usize,
    pub message_ids: Vec<Uuid>,
    pub confidence: f32,
}
```

#### F5.2 Concept-Based Discovery

**Commands**:
```bash
# Find sessions by concept
/sessions by-concept "authentication"
/sessions by-concept "OAuth" --connected-to "JWT"

# Find concept paths between sessions
/sessions path <session-id-1> <session-id-2>

# Cluster sessions by concept similarity
/sessions cluster --algorithm kmeans --k 5
```

#### F5.3 Cross-Session Learning

**Integration with Agent Evolution**:
- Successful solutions become "lessons learned"
- Patterns across sessions inform future recommendations
- Concept frequency informs knowledge graph weighting

---

## User Experience

### Interactive Mode

```
$ terraphim-agent
🔮 Terraphim Agent v0.1.0

> /sessions search "async database"
╭──────┬────────────────────────────────┬──────────┬───────────╮
│ Rank │ Session                        │ Source   │ Date      │
├──────┼────────────────────────────────┼──────────┼───────────┤
│ 1    │ Fixing async pool exhaustion   │ claude   │ 2024-12-01│
│ 2    │ SQLx connection handling       │ cursor   │ 2024-11-28│
│ 3    │ Tokio runtime in tests         │ aider    │ 2024-11-15│
╰──────┴────────────────────────────────┴──────────┴───────────╯

Concepts matched: async, database, connection_pool, tokio
3 results in 42ms

> /sessions expand 1 --context 5
[Expands session 1 with 5 messages of context]
```

### Robot Mode

```bash
$ terraphim-agent robot search "async database" --format json --max-results 3
{
  "success": true,
  "meta": {
    "command": "search",
    "elapsed_ms": 42,
    "total_results": 156,
    "returned_results": 3,
    "concepts_matched": ["async", "database", "connection_pool", "tokio"],
    "wildcard_fallback": false
  },
  "data": {
    "results": [
      {
        "rank": 1,
        "session_id": "550e8400-e29b-41d4-a716-446655440000",
        "title": "Fixing async pool exhaustion",
        "source": "claude-code",
        "date": "2024-12-01",
        "score": 0.95,
        "preview": "The issue was that the connection pool..."
      }
    ]
  }
}
```

---

## Security & Privacy

### Data Handling

1. **Local Only**: All session data stored locally, never transmitted
2. **Source Paths**: Configurable, defaults respect source tool conventions
3. **Encryption at Rest**: Optional encryption for session index
4. **Access Control**: Sessions inherit file system permissions

### Sensitive Data

1. **API Keys**: Redacted during import (regex patterns)
2. **Secrets**: Optional secret scanning with configurable patterns
3. **PII**: No special handling (user responsibility)

---

## Performance Requirements

| Metric | Target | Notes |
|--------|--------|-------|
| Import speed | >1000 sessions/sec | Batch processing |
| Search latency | <100ms | For 10K sessions |
| Index size | <10MB per 1K sessions | With compression |
| Memory usage | <100MB | During search |
| Startup time | <500ms | With warm index |

---

## Compatibility

### Minimum Requirements

- Rust 1.75+
- 50MB disk space (base)
- 100MB RAM

### Platform Support

- Linux (primary)
- macOS
- Windows (via WSL recommended)

### Integration Points

- MCP server (existing)
- HTTP API (existing)
- Unix pipes (new)
- JSON-RPC (future)

---

## Success Criteria

### Phase 1 (Robot Mode)
- [ ] All commands support `--format json`
- [ ] Exit codes implemented and documented
- [ ] Token budget management working
- [ ] Forgiving CLI with <2 edit distance correction
- [ ] Self-documentation API complete

### Phase 2 (Session Search)
- [ ] 3+ connectors implemented (Claude Code, Cursor, Aider)
- [ ] Session index with <100ms search latency
- [ ] Basic session commands working

### Phase 3 (Knowledge Graph)
- [ ] Session enrichment pipeline
- [ ] Concept-based session discovery
- [ ] Cross-session learning integration

---

## References

### Core Inspiration
- [CASS Repository](https://github.com/Dicklesworthstone/coding_agent_session_search) - Original cross-agent session search

### Claude Code Log Ecosystem
- [clog](https://github.com/HillviewCap/clog) - Web-based viewer with JSONL schema documentation
- [vibe-log-cli](https://github.com/vibe-log/vibe-log-cli) - Session analysis with privacy-preserving sanitization
- [claude-conversation-extractor](https://github.com/ZeroSumQuant/claude-conversation-extractor) - Export/search tool with 97% test coverage
- [claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer) - Tauri+Rust desktop app
- [cc-log-viewer](https://crates.io/crates/cc-log-viewer) - Rust crate for log viewing

### Technical Resources
- [Tantivy Documentation](https://docs.rs/tantivy/) - Full-text search engine
- [strsim crate](https://crates.io/crates/strsim) - String similarity algorithms
- [Jaro-Winkler Algorithm](https://en.wikipedia.org/wiki/Jaro%E2%80%93Winkler_distance)
- [Terraphim Architecture](../specifications/terraphim-desktop-spec.md)
