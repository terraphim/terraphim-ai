# Research Document: Cline Session Storage Format Investigation

**Status**: Draft
**Author**: Terraphim Research Agent
**Date**: 2026-04-17
**Reviewers**: TBD

## Executive Summary

This research document investigates the session storage format of Cline (formerly Claude Dev), a VS Code extension for autonomous AI coding assistance. The investigation maps Cline's data structures, storage locations, file formats, and task organisation to Terraphim's existing session models. Key findings: Cline stores sessions as JSON files (not JSONL) in per-task directories under VS Code's globalStorage, with separate files for API conversation history, UI messages, context history, and task metadata. Unlike Claude Code's JSONL format, Cline uses typed message arrays with checkpoint support via Git snapshots.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Cross-agent session search is a core differentiator for Terraphim |
| Leverages strengths? | Yes | We already have connectors for Claude Code, Cursor, Codex, Aider, OpenCode; Cline fills a gap |
| Meets real need? | Yes | Cline has 60.4k+ GitHub stars; users need unified search across all AI assistants |

**Proceed**: Yes - at least 2/3 YES

---

## Problem Statement

### Description

Terraphim's session search and analysis capabilities currently support Claude Code, Cursor, Codex, Aider, and OpenCode, but lack support for Cline -- one of the most popular AI coding assistants. To provide truly unified cross-agent session search, we need to understand and parse Cline's session storage format.

### Impact

- Users who primarily use Cline cannot benefit from Terraphim's session search
- Fragmented knowledge persists across the AI assistant ecosystem
- The `terraphim-session-analyzer` specification already lists Cline as a planned connector but it is unimplemented

### Success Criteria

1. Document Cline's storage paths, file formats, and data structures
2. Map Cline's data model to Terraphim's `NormalizedSession` / `SessionEntry` models
3. Identify risks and unknowns for future implementation
4. Provide a clear implementation path for a `ClineConnector`

---

## Current State Analysis

### Existing Implementation

The `terraphim-session-analyzer` crate (`crates/terraphim-session-analyzer/`) provides:

- **Connectors module** (`src/connectors/mod.rs`): Trait-based connector architecture with `SessionConnector`, `NormalizedSession`, and `NormalizedMessage`
- **Existing connectors**:
  - `ClaudeCodeConnector` (`src/connectors/mod.rs`): Wraps `SessionParser` for JSONL format
  - `CursorConnector` (`src/connectors/cursor.rs`): Parses SQLite `state.vscdb` databases
  - `CodexConnector` (`src/connectors/codex.rs`): Parses JSONL from `~/.codex/sessions/`
  - `AiderConnector` (`src/connectors/aider.rs`): Parses Markdown `.aider.chat.history.md`
  - `OpenCodeConnector` (`src/connectors/opencode.rs`): Parses `prompt-history.jsonl`
- **Models** (`src/models.rs`): `SessionEntry`, `Message`, `ContentBlock`, `ToolResultContent`, `AgentInvocation`, `FileOperation`, etc.
- **Parser** (`src/parser.rs`): `SessionParser` for Claude Code's JSONL format

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| SessionConnector trait | `crates/terraphim-session-analyzer/src/connectors/mod.rs` | Interface for all connectors |
| NormalizedSession | `crates/terraphim-session-analyzer/src/connectors/mod.rs` | Unified session representation |
| SessionParser | `crates/terraphim-session-analyzer/src/parser.rs` | Claude Code JSONL parser |
| SessionEntry model | `crates/terraphim-session-analyzer/src/models.rs` | Claude Code data structures |

### Data Flow

```
Cline Storage (JSON files)
    |
    v
ClineConnector (future)
    |
    v
NormalizedSession { source: "cline", messages: [...] }
    |
    v
SessionIndex (Tantivy)
    |
    v
Search / Analysis / Export
```

### Integration Points

- The `ConnectorRegistry` in `src/connectors/mod.rs` registers all connectors
- `ClaudeCodeConnector` is always available; others are behind `--features connectors`
- The specification (`docs/specifications/terraphim-agent-session-search-spec.md`) lists Cline as a supported source with format "JSON" and location `~/.cline/`

---

## Constraints

### Technical Constraints

- **Language**: Rust (connector must be idiomatic Rust)
- **Dependencies**: Can use `serde_json`, `walkdir`, `jiff`, `anyhow` (already in project)
- **Feature gate**: Must be behind `#[cfg(feature = "connectors")]` like other connectors
- **Cross-platform**: Must support macOS, Linux, and Windows paths

### Business Constraints

- Cline's storage format may change between versions (currently v3.79.0)
- Must not require Cline to be running to read sessions
- Privacy-first: local file access only, no cloud APIs

### Non-Functional Requirements

| Requirement | Target | Notes |
|-------------|--------|-------|
| Import speed | >100 sessions/sec | Cline typically has fewer sessions than Claude Code |
| Memory usage | <50MB | Per-connector budget |
| Parse robustness | Handle missing/optional fields | Cline files may be partially written during active tasks |

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Must parse JSON (not JSONL) | Cline uses structured JSON arrays, not line-delimited JSON | Source code analysis of `disk.ts` |
| Must handle per-task directories | Each task has its own directory with multiple files | `ensureTaskDirectoryExists(taskId)` |
| Must map ClineMessage to NormalizedMessage | Cline's message structure differs significantly from Claude Code | Type definitions in ExtensionMessage.ts |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Real-time sync with Cline | Not needed; batch import on demand is sufficient |
| Checkpoint restoration | Analysis only; restoring Cline checkpoints is out of scope |
| Cline settings/rules parsing | Focus on session messages, not configuration |
| MCP server state | Too volatile; not part of session history |

---

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim-session-analyzer` | Connector lives here | Low - stable crate |
| `NormalizedSession` struct | Must fit Cline data | Low - flexible structure |
| `ConnectorRegistry` | Must register Cline | Low - simple registration |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `serde_json` | 1.0 | Low | Standard, already used |
| `walkdir` | 2.x | Low | Already in Cargo.toml |
| `jiff` | 0.1 | Low | Already used for timestamps |

---

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Cline format changes between versions | Medium | High | Version detection, graceful degradation |
| Partial/corrupt JSON during active tasks | High | Medium | Atomic write detection, skip unparseable files |
| Missing task directories (deleted tasks) | Medium | Low | Filter missing dirs during enumeration |
| VS Code globalStorage path varies by install | Medium | Medium | Support custom path via ImportOptions |
| Checkpoints stored in Git (not JSON) | High | Medium | Skip checkpoint data or use git2 crate |

### Open Questions

1. **What is the exact VS Code globalStorage path?** - We know the pattern but need to verify `globalStorageFsPath` resolution across platforms. *Can be answered by testing on each platform.*
2. **How does Cline handle task deletion?** - Does it clean up the directory or leave orphaned files? *Can be answered by examining Cline's delete logic.*
3. **What is the `context_history.json` format?** - Not fully documented in source. *Requires sample file analysis.*
4. **Are there multiple tasks per directory?** - Each task has its own directory, but need to confirm no nesting.
5. **How stable is the `ClineMessage` schema across versions?** - The type has many optional fields that may evolve.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Cline stores all task data in `{globalStorage}/tasks/{taskId}/` | Source code: `ensureTaskDirectoryExists` | If Cline migrates to `~/.cline/`, connector breaks | Partially - dual path support needed |
| Task history is in `taskHistory.json` (state dir) | Source code: `getTaskHistoryStateFilePath()` | If history moves, we can't list sessions | No - need to verify |
| `api_conversation_history.json` contains the canonical message history | Source code: `getSavedApiConversationHistory` | If UI messages diverge, we may miss data | No - both files should be parsed |
| Cline uses VS Code's ExtensionContext.globalStoragePath | VS Code API docs | CLI version of Cline may use different path | No - Cline CLI may differ |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **Option A**: Parse only `api_conversation_history.json` | Simpler, but may miss UI-specific metadata | Rejected - UI messages contain timestamps and display info |
| **Option B**: Parse only `ui_messages.json` | Captures display state, but may miss API-level tool calls | Rejected - API history has complete conversation |
| **Option C**: Parse both and merge | Most complete, but more complex | **Chosen** - provides full picture |
| **Option D**: Use `taskHistory.json` as index, then load per-task dirs | Most robust, follows existing connector pattern | **Chosen** - aligns with how Cline itself works |

---

## Research Findings

### 1. Storage Locations

Cline stores data in **two primary locations**:

#### A. VS Code Extension Global Storage (Primary)

This is where the VS Code extension stores its data via `ExtensionContext.globalStorageUri`.

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/` |
| Linux | `~/.config/Code/User/globalStorage/saoudrizwan.claude-dev/` |
| Windows | `%APPDATA%/Code/User/globalStorage/saoudrizwan.claude-dev/` |

Under this directory:
- `tasks/{taskId}/` - Per-task directory containing:
  - `api_conversation_history.json` - API conversation (Anthropic.MessageParam[])
  - `ui_messages.json` - UI messages (ClineMessage[])
  - `context_history.json` - Context tracking history
  - `task_metadata.json` - Task metadata
  - `settings.json` - Task-specific settings
- `state/taskHistory.json` - Task history index (HistoryItem[])
- `state/` - Global state
- `cache/` - Cached data

#### B. Cline Home Directory (Future / CLI)

```typescript
// From disk.ts
export function getClineHomePath(): string {
    return path.join(os.homedir(), ".cline")
}
```

This is the newer path intended for CLI usage and may eventually replace VS Code globalStorage. Currently used for:
- Skills: `~/.cline/skills/`

#### C. Documents Directory (Legacy Fallback)

```typescript
// From disk.ts - used for rules, workflows, MCP servers
const userDocumentsPath = await getDocumentsPath()
// macOS: ~/Documents/Cline/
// Linux: ~/Documents/Cline/ (or XDG_DOCUMENTS_DIR)
// Windows: %USERPROFILE%/Documents/Cline/
```

### 2. File Formats

#### taskHistory.json (Task Index)

**Format**: JSON array of `HistoryItem` objects
**Location**: `{globalStorage}/state/taskHistory.json`

```typescript
type HistoryItem = {
    id: string           // Task ID (timestamp-based)
    ulid?: string        // ULID for better tracking
    ts: number           // Unix timestamp (milliseconds)
    task: string         // Task description/prompt
    tokensIn: number
    tokensOut: number
    cacheWrites?: number
    cacheReads?: number
    totalCost: number
    size?: number
    shadowGitConfigWorkTree?: string
    cwdOnTaskInitialization?: string
    conversationHistoryDeletedRange?: [number, number]
    isFavorited?: boolean
    checkpointManagerErrorMessage?: string
    modelId?: string
}
```

#### api_conversation_history.json (API Messages)

**Format**: JSON array of `Anthropic.MessageParam` objects
**Location**: `{globalStorage}/tasks/{taskId}/api_conversation_history.json`

This follows the Anthropic SDK format:
```json
[
  {
    "role": "user",
    "content": [
      {"type": "text", "text": "..."},
      {"type": "image", "source": {...}}
    ]
  },
  {
    "role": "assistant",
    "content": [
      {"type": "text", "text": "..."},
      {"type": "tool_use", "id": "...", "name": "...", "input": {...}},
      {"type": "tool_result", "tool_use_id": "...", "content": "..."}
    ]
  }
]
```

#### ui_messages.json (UI Messages)

**Format**: JSON array of `ClineMessage` objects
**Location**: `{globalStorage}/tasks/{taskId}/ui_messages.json`

```typescript
interface ClineMessage {
    ts: number                    // Unix timestamp (milliseconds)
    type: "ask" | "say"
    ask?: ClineAsk                // Question/request to user
    say?: ClineSay                // Statement/output from Cline
    text?: string                 // Message text content
    reasoning?: string            // Reasoning content
    images?: string[]             // Base64-encoded images
    files?: string[]              // Attached files
    partial?: boolean             // Is this a partial message?
    commandCompleted?: boolean
    lastCheckpointHash?: string
    isCheckpointCheckedOut?: boolean
    isOperationOutsideWorkspace?: boolean
    conversationHistoryIndex?: number
    conversationHistoryDeletedRange?: [number, number]
    modelInfo?: ClineMessageModelInfo
}

type ClineAsk =
    | "followup"
    | "plan_mode_respond"
    | "act_mode_respond"
    | "command"
    | "command_output"
    | "completion_result"
    | "tool"
    | "api_req_failed"
    | "resume_task"
    | "resume_completed_task"
    | "mistake_limit_reached"
    | "browser_action_launch"
    | "use_mcp_server"
    | "new_task"
    | "condense"
    | "summarize_task"
    | "report_bug"
    | "use_subagents"

type ClineSay =
    | "task"
    | "error"
    | "error_retry"
    | "api_req_started"
    | "api_req_finished"
    | "text"
    | "reasoning"
    | "completion_result"
    | "user_feedback"
    | "user_feedback_diff"
    | "api_req_retried"
    | "command"
    | "command_output"
    | "tool"
    | "shell_integration_warning"
    | "browser_action_launch"
    | "browser_action"
    | "browser_action_result"
    | "mcp_server_request_started"
    | "mcp_server_response"
    | "mcp_notification"
    | "use_mcp_server"
    | "diff_error"
    | "deleted_api_reqs"
    | "clineignore_error"
    | "command_permission_denied"
    | "checkpoint_created"
    | "load_mcp_documentation"
    | "generate_explanation"
    | "info"
    | "task_progress"
    | "hook_status"
    | "hook_output_stream"
    | "subagent"
    | "use_subagents"
    | "subagent_usage"
    | "conditional_rules_applied"
```

#### task_metadata.json

**Format**: JSON object
**Location**: `{globalStorage}/tasks/{taskId}/task_metadata.json`

```typescript
interface TaskMetadata {
    files_in_context: FileMetadataEntry[]
    model_usage: ModelMetadataEntry[]
    environment_history: EnvironmentMetadataEntry[]
}

interface FileMetadataEntry {
    path: string
    record_state: "active" | "stale"
    record_source: "read_tool" | "user_edited" | "cline_edited" | "file_mentioned"
    cline_read_date: number | null
    cline_edit_date: number | null
    user_edit_date?: number | null
}

interface ModelMetadataEntry {
    ts: number
    model_id: string
    model_provider_id: string
    mode: string
}

interface EnvironmentMetadataEntry {
    ts: number
    os_name: string
    os_version: string
    os_arch: string
    host_name: string
    host_version: string
    cline_version: string
}
```

### 3. Task Structure and Threading

#### Task IDs
- Primary: Unix timestamp in milliseconds (e.g., `1712345678901`)
- ULID: Additional unique identifier for telemetry

#### Task Lifecycle
1. **Creation**: Controller creates Task with `taskId` and `ulid`
2. **Active**: Messages flow through `ask` and `say` methods
3. **Persistence**: Messages saved atomically to JSON files
4. **History**: Controller updates `taskHistory.json` via StateManager
5. **Resumption**: Tasks can be resumed from saved state
6. **Deletion**: Remove from taskHistory + optionally delete directory

#### Multi-Step Tasks
- Cline handles multi-step tasks within a single Task instance
- No explicit "turn" concept like Codex's JSONL protocol
- Messages accumulate in `apiConversationHistory` and `clineMessages`
- Context window management via truncation (deleted ranges)

### 4. Checkpoints

Cline implements checkpoints using **Git snapshots**:
- `ICheckpointManager` interface with implementations for single-root and multi-root workspaces
- Checkpoints are stored as Git commits in a shadow repository
- `lastCheckpointHash` and `isCheckpointCheckedOut` fields on messages
- Checkpoints are **not** stored in JSON files but managed via Git
- For session search, checkpoint hashes are metadata; actual diffs require Git access

### 5. Comparison with Claude Code Format

| Aspect | Claude Code | Cline |
|--------|-------------|-------|
| **Format** | JSONL | JSON arrays |
| **Storage** | `~/.claude/projects/` | VS Code globalStorage |
| **Per-session** | One `.jsonl` file | Directory with multiple JSON files |
| **Messages** | `SessionEntry` with `parentUuid` | `ClineMessage` with timestamps |
| **Threading** | Parent-child UUID chain | Linear array with deleted ranges |
| **Tool calls** | `ContentBlock::ToolUse` | Embedded in Anthropic.MessageParam |
| **Checkpoints** | None | Git-based snapshots |
| **Metadata** | Inline (cwd, version, gitBranch) | Separate `task_metadata.json` |

### 6. Mapping to Terraphim Session Model

#### NormalizedSession Mapping

```rust
NormalizedSession {
    source: "cline",
    external_id: history_item.id,           // Task ID
    title: Some(history_item.task.clone()), // Task description
    source_path: task_dir_path,
    started_at: parse_timestamp(history_item.ts),
    ended_at: None, // Would need to infer from last message
    messages: normalized_messages,
    metadata: json!({
        "ulid": history_item.ulid,
        "cwd": history_item.cwd_on_task_initialization,
        "model_id": history_item.model_id,
        "total_cost": history_item.total_cost,
        "tokens_in": history_item.tokens_in,
        "tokens_out": history_item.tokens_out,
        "is_favorited": history_item.is_favorited,
        "checkpoint_error": history_item.checkpoint_manager_error_message,
    }),
}
```

#### NormalizedMessage Mapping

From `api_conversation_history.json`:
```rust
// User message
NormalizedMessage {
    idx: index,
    role: "user".to_string(),
    author: None,
    content: extract_text_content(&message.content),
    created_at: None, // API messages don't have timestamps
    extra: json!({"source": "api_conversation"}),
}

// Assistant message
NormalizedMessage {
    idx: index,
    role: "assistant".to_string(),
    author: Some(model_id.clone()),
    content: extract_text_content(&message.content),
    created_at: None,
    extra: json!({"source": "api_conversation", "tools": extract_tools(&message.content)}),
}
```

From `ui_messages.json` (more metadata):
```rust
NormalizedMessage {
    idx: index,
    role: match message.type_ {
        "ask" => "user",   // User is being asked
        "say" => "assistant", // Cline is saying something
    },
    author: message.model_info.as_ref().map(|m| m.model_id.clone()),
    content: message.text.clone().unwrap_or_default(),
    created_at: Some(jiff::Timestamp::from_millisecond(message.ts as i64)),
    extra: json!({
        "ask_type": message.ask,
        "say_type": message.say,
        "is_partial": message.partial,
        "has_images": message.images.is_some(),
    }),
}
```

#### Content Extraction

For Anthropic.MessageParam content (can be string or array):
```rust
fn extract_text_content(content: &serde_json::Value) -> String {
    if let Some(text) = content.as_str() {
        text.to_string()
    } else if let Some(arr) = content.as_array() {
        arr.iter()
            .filter_map(|block| {
                if block.get("type")?.as_str()? == "text" {
                    block.get("text")?.as_str()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    }
}
```

### 7. Export Function / API

Cline has limited built-in export:
- **`exportTaskWithId(id)`**: Opens the task directory in file manager (not a true export)
- **Conversation history text export**: `writeConversationHistoryText()` creates human-readable text files for hooks
- **Markdown export**: `formatContentBlockToMarkdown()` for converting messages to Markdown
- **No formal export API** for programmatic access

---

## Recommendations

### Proceed/No-Proceed

**PROCEED** with Cline connector implementation. The format is well-understood, stable, and maps reasonably to Terraphim's model.

### Scope Recommendations

1. **Phase 1 (MVP)**:
   - Parse `taskHistory.json` to list sessions
   - Parse `api_conversation_history.json` for message content
   - Basic `ClineConnector` with `detect()`, `import()`, `default_path()`
   - Register in `ConnectorRegistry`

2. **Phase 2 (Enhanced)**:
   - Parse `ui_messages.json` for timestamps and metadata
   - Extract tool usage from message content
   - Parse `task_metadata.json` for file context and model usage

3. **Phase 3 (Advanced)**:
   - Handle checkpoints (read Git snapshot hashes)
   - Support Cline home directory (`~/.cline/`) for CLI usage
   - Handle conversation history truncation (deleted ranges)

### Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Format changes | Implement version detection from `task_metadata.json` |
| Partial writes | Check for `.tmp.*` files and skip active tasks |
| Missing fields | Use `serde(default)` for all optional fields |
| Path variations | Support `ImportOptions.path` override; check multiple locations |
| Git checkpoints | Document limitation; optionally use `git2` crate for diffs |

---

## Next Steps

If approved:

1. **Create `crates/terraphim-session-analyzer/src/connectors/cline.rs`**
   - Implement `ClineConnector` struct
   - Implement `SessionConnector` trait
   - Add data structures for `HistoryItem`, `ClineMessage`, `TaskMetadata`

2. **Register connector in `src/connectors/mod.rs`**
   - Add `pub mod cline;` behind `#[cfg(feature = "connectors")]`
   - Add `Box::new(cline::ClineConnector)` to `ConnectorRegistry::new()`

3. **Write tests**
   - Mock Cline data in `tests/test_data/`
   - Test parsing of each file type
   - Test error handling for corrupt/missing files

4. **Update documentation**
   - Add Cline to README.md supported sources
   - Update session search spec
   - Document path detection logic

---

## Appendix

### Reference Materials

- [Cline GitHub Repository](https://github.com/cline/cline)
- [VS Code Extension API - ExtensionContext.globalStorageUri](https://code.visualstudio.com/api/references/vscode-api#ExtensionContext.globalStorageUri)
- [Anthropic TypeScript SDK - MessageParam](https://github.com/anthropics/anthropic-sdk-typescript)
- Terraphim session search spec: `docs/specifications/terraphim-agent-session-search-spec.md`

### Code Snippets

#### VS Code globalStorage path resolution
```typescript
// From Cline's disk.ts
async function getGlobalStorageDir(...subdirs: string[]) {
    const fullPath = path.resolve(HostProvider.get().globalStorageFsPath, ...subdirs)
    await fs.mkdir(fullPath, { recursive: true })
    return fullPath
}
```

#### Atomic write pattern
```typescript
async function atomicWriteFile(filePath: string, data: string): Promise<void> {
    const tmpPath = `${filePath}.tmp.${Date.now()}.${Math.random().toString(36).substring(7)}.json`
    try {
        await fs.writeFile(tmpPath, data, "utf8")
        await fs.rename(tmpPath, filePath)
    } catch (error) {
        fs.unlink(tmpPath).catch(() => {})
        throw error
    }
}
```

#### Task directory layout
```
saoudrizwan.claude-dev/
├── cache/
│   └── cline_mcp_marketplace_catalog.json
├── state/
│   └── taskHistory.json
├── settings/
│   └── cline_mcp_settings.json
└── tasks/
    ├── 1712345678901/
    │   ├── api_conversation_history.json
    │   ├── ui_messages.json
    │   ├── context_history.json
    │   ├── task_metadata.json
    │   └── settings.json
    └── 1712345679999/
        └── ...
```

### Test Data Sample

```json
// api_conversation_history.json excerpt
[
  {
    "role": "user",
    "content": [
      {"type": "text", "text": "<task>\nImplement user authentication\n</task>"}
    ]
  },
  {
    "role": "assistant",
    "content": [
      {"type": "text", "text": "I'll help you implement user authentication."},
      {"type": "tool_use", "id": "toolu_01ABC", "name": "read_file", "input": {"file_path": "/project/src/auth.rs"}}
    ]
  }
]
```

```json
// ui_messages.json excerpt
[
  {
    "ts": 1712345678901,
    "type": "say",
    "say": "task",
    "text": "Implement user authentication",
    "modelInfo": {"providerId": "anthropic", "modelId": "claude-3-5-sonnet-20241022", "mode": "act"}
  },
  {
    "ts": 1712345679012,
    "type": "say",
    "say": "api_req_started",
    "text": "{\"request\":\"...\",\"tokensIn\":1500}",
    "modelInfo": {"providerId": "anthropic", "modelId": "claude-3-5-sonnet-20241022", "mode": "act"}
  }
]
```

```json
// taskHistory.json excerpt
[
  {
    "id": "1712345678901",
    "ulid": "01HV8J3K2M4N5P6Q7R8S9T0UV",
    "ts": 1712345678901,
    "task": "Implement user authentication",
    "tokensIn": 15000,
    "tokensOut": 8000,
    "totalCost": 0.45,
    "cwdOnTaskInitialization": "/home/user/project",
    "modelId": "claude-3-5-sonnet-20241022"
  }
]
```
