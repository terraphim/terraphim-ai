# Design & Implementation Plan: AI Assistant Session Haystack

## 1. Summary of Target Behavior

A **unified haystack** for searching across AI coding assistant session logs. Uses `terraphim-session-analyzer`'s connector system to support:

| Connector | Source ID | Format | Default Path |
|-----------|-----------|--------|--------------|
| Claude Code | `claude-code` | JSONL | `~/.claude/projects/` |
| OpenCode | `opencode` | JSONL | `~/.opencode/` |
| Cursor IDE | `cursor` | SQLite | `~/.config/Cursor/User/` |
| Aider | `aider` | Markdown | `~/projects/.aider.chat.history.md` |
| Codex | `codex` | JSONL | Codex CLI data |

Users configure haystacks with `ServiceType::AiAssistant` and specify the connector via `extra_parameters["connector"]`.

### Example Configurations

```json
{
  "haystacks": [
    {
      "name": "Claude Sessions",
      "service": "AiAssistant",
      "location": "~/.claude/projects/",
      "extra_parameters": {
        "connector": "claude-code"
      }
    },
    {
      "name": "OpenCode Sessions",
      "service": "AiAssistant",
      "location": "~/.opencode/",
      "extra_parameters": {
        "connector": "opencode"
      }
    },
    {
      "name": "Cursor Chats",
      "service": "AiAssistant",
      "location": "~/.config/Cursor/User/",
      "extra_parameters": {
        "connector": "cursor"
      }
    }
  ]
}
```

## 2. Key Invariants and Acceptance Criteria

### Invariants
- **I1**: Session files are read-only (never modified by haystack)
- **I2**: All Documents have unique IDs (`{connector}:{session_id}:{message_idx}`)
- **I3**: Each connector uses its own parsing logic via `SessionConnector` trait
- **I4**: All connectors produce `NormalizedSession` → `Document` mapping

### Acceptance Criteria
- **AC1**: `ServiceType::AiAssistant` compiles and is recognized
- **AC2**: Config with `connector: "claude-code"` indexes Claude sessions
- **AC3**: Config with `connector: "opencode"` indexes OpenCode sessions
- **AC4**: Config with `connector: "cursor"` indexes Cursor chats
- **AC5**: Config with `connector: "aider"` indexes Aider history
- **AC6**: Search term matches message content, session title, project path
- **AC7**: Invalid connector name returns helpful error

## 3. High-Level Design and Boundaries

```
┌────────────────────────────────────────────────────────────────┐
│                     terraphim_middleware                        │
├────────────────────────────────────────────────────────────────┤
│  indexer/mod.rs                                                │
│    └─ search_haystacks()                                       │
│        └─ match ServiceType::AiAssistant                       │
│             └─ AiAssistantHaystackIndexer.index()              │
├────────────────────────────────────────────────────────────────┤
│  haystack/                                                     │
│    ├─ mod.rs (add ai_assistant module)                         │
│    └─ ai_assistant.rs (NEW)                                    │
│         ├─ AiAssistantHaystackIndexer                          │
│         └─ Uses ConnectorRegistry to get connector             │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────┐
│                     terraphim-session-analyzer                        │
├────────────────────────────────────────────────────────────────┤
│  connectors/mod.rs                                             │
│    ├─ SessionConnector trait                                   │
│    ├─ ConnectorRegistry (finds connectors)                     │
│    ├─ NormalizedSession (unified session format)               │
│    └─ NormalizedMessage (unified message format)               │
│                                                                 │
│  connectors/                                                   │
│    ├─ ClaudeCodeConnector (claude-code)                        │
│    ├─ OpenCodeConnector (opencode)                             │
│    ├─ CursorConnector (cursor)                                 │
│    ├─ AiderConnector (aider)                                   │
│    └─ CodexConnector (codex)                                   │
└────────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Single ServiceType**: `AiAssistant` instead of 5 separate types
2. **Connector Selection**: Via `extra_parameters["connector"]`
3. **Feature Flag**: `connectors` feature in terraphim-session-analyzer (Cursor needs SQLite)
4. **Document Mapping**: `NormalizedSession` → multiple `Document` (one per message)

## 4. File/Module-Level Change Plan

| File/Module | Action | Change | Dependencies |
|-------------|--------|--------|--------------|
| `terraphim_config/src/lib.rs:273` | Modify | Add `AiAssistant` to ServiceType | None |
| `terraphim_middleware/Cargo.toml` | Modify | Add `terraphim-session-analyzer = { features = ["connectors"] }` | terraphim-session-analyzer |
| `terraphim_middleware/src/haystack/mod.rs` | Modify | Add `ai_assistant` module + export | ai_assistant.rs |
| `terraphim_middleware/src/haystack/ai_assistant.rs` | Create | `AiAssistantHaystackIndexer` | terraphim-session-analyzer connectors |
| `terraphim_middleware/src/indexer/mod.rs` | Modify | Add match arm for `ServiceType::AiAssistant` | ai_assistant module |

## 5. Step-by-Step Implementation Sequence

### Step 1: Add ServiceType variant
**File**: `crates/terraphim_config/src/lib.rs`
**Change**: Add after line 273:
```rust
/// Use AI coding assistant session logs (Claude Code, OpenCode, Cursor, Aider)
AiAssistant,
```
**Deployable**: Yes

### Step 2: Add dependency with connectors feature
**File**: `crates/terraphim_middleware/Cargo.toml`
**Change**: Add to `[dependencies]`:
```toml
terraphim-session-analyzer = { path = "../terraphim-session-analyzer", features = ["connectors"] }
```
**Deployable**: Yes

### Step 3: Create ai_assistant haystack module
**File**: `crates/terraphim_middleware/src/haystack/ai_assistant.rs` (NEW)
**Structure**:
```rust
pub struct AiAssistantHaystackIndexer;

impl IndexMiddleware for AiAssistantHaystackIndexer {
    fn index(&self, needle: &str, haystack: &Haystack) -> impl Future<Output = Result<Index>> {
        async move {
            // 1. Get connector name from extra_parameters["connector"]
            // 2. Get connector from ConnectorRegistry
            // 3. Import sessions with connector.import()
            // 4. Convert NormalizedSession/Message to Documents
            // 5. Filter by needle (search term)
            // 6. Return Index
        }
    }
}

fn session_to_documents(session: NormalizedSession, needle: &str) -> Vec<Document> {
    // One document per message that matches needle
}
```
**Deployable**: Yes (not wired up yet)

### Step 4: Export ai_assistant module
**File**: `crates/terraphim_middleware/src/haystack/mod.rs`
**Change**: Add:
```rust
pub mod ai_assistant;
pub use ai_assistant::AiAssistantHaystackIndexer;
```
**Deployable**: Yes

### Step 5: Wire up in search_haystacks()
**File**: `crates/terraphim_middleware/src/indexer/mod.rs`
**Change**: Add match arm:
```rust
ServiceType::AiAssistant => {
    let indexer = AiAssistantHaystackIndexer::default();
    indexer.index(needle, haystack).await
}
```
**Deployable**: Yes (feature complete)

### Step 6: Add integration tests
**File**: `crates/terraphim_middleware/tests/ai_assistant_haystack_test.rs` (NEW)
**Content**: Tests for each connector type with fixtures
**Deployable**: Yes

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| AC1: ServiceType compiles | Compile | Automatic |
| AC2: claude-code connector | Unit | `ai_assistant.rs::tests` |
| AC3: opencode connector | Unit | `ai_assistant.rs::tests` |
| AC4: cursor connector | Integration | Needs SQLite fixture |
| AC5: aider connector | Unit | Uses markdown fixture |
| AC6: Search matches content | Unit | `ai_assistant.rs::tests` |
| AC7: Invalid connector error | Unit | `ai_assistant.rs::tests` |

### Test Fixtures
- Create minimal session files in `terraphim_middleware/fixtures/ai_sessions/`:
  - `claude-code/session.jsonl`
  - `opencode/session.jsonl`
  - `aider/.aider.chat.history.md`

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| SQLite dependency for Cursor | Feature-gate Cursor connector | Minimal |
| Large session directories | ConnectorRegistry streams efficiently | Low |
| Multiple message formats | All connectors normalize to NormalizedMessage | None |
| Missing connector name | Return clear error with valid options | None |

## 8. Document Mapping Strategy

Each `NormalizedMessage` becomes one `Document`:

```rust
Document {
    id: format!("{}:{}:{}", session.source, session.external_id, msg.idx),
    title: format!("[{}] {}", session.source.to_uppercase(),
                   session.title.unwrap_or("Session".to_string())),
    url: session.source_path.to_string_lossy().to_string(),
    body: msg.content.clone(),
    description: Some(format!(
        "{} message from {} session",
        msg.role,
        session.source
    )),
    tags: Some(vec![
        session.source.clone(),
        msg.role.clone(),
        "ai-assistant".to_string(),
    ]),
    ..Default::default()
}
```

## 9. Open Questions / Decisions for Human Review

1. **Granularity**: One document per message (current plan) or one per session?
   - **Recommendation**: Per message for precise search results

2. **Search scope**: Search message content only, or also session metadata?
   - **Recommendation**: Both, with content weighted higher

3. **Connector auto-detection**: Should we auto-detect if `connector` param is missing?
   - **Recommendation**: No, require explicit connector for clarity

---

**Do you approve this plan as-is, or would you like to adjust any part?**
