# Summary: terraphim_server/src/lib.rs

**Purpose:** Axum-based HTTP server providing REST API for Terraphim AI with workflow management and WebSocket support.

**Server Setup:**
- Binds to configurable `SocketAddr`
- CORS layer allowing any origin/method/header
- Extended `AppState` with workflow sessions and WebSocket broadcaster

**AppState:**
```rust
pub struct AppState {
    pub config_state: ConfigState,
    pub workflow_sessions: Arc<workflows::WorkflowSessions>,
    pub websocket_broadcaster: workflows::WebSocketBroadcaster,
}
```

**API Routes:**

| Route | Method | Purpose |
|-------|--------|---------|
| `/health` | GET | Health check |
| `/documents` | POST | Create document |
| `/documents/search` | GET/POST | Search documents |
| `/documents/summarize` | POST | Synchronous summarisation |
| `/documents/async_summarize` | POST | Async summarisation |
| `/summarization/batch` | POST | Batch summarisation |
| `/summarization/task/{id}/status` | GET | Task status |
| `/summarization/task/{id}/cancel` | POST | Cancel task |
| `/summarization/queue/stats` | GET | Queue statistics |
| `/chat` | POST | Chat completion |
| `/config` | GET/POST | Get/update config |
| `/config/schema` | GET | Config JSON schema |
| `/config/selected_role` | POST | Update selected role |
| `/rolegraph` | GET | Get rolegraph data |
| `/roles/{name}/kg_search` | GET | KG-term search |
| `/thesaurus/{role}` | GET | Get thesaurus |
| `/autocomplete/{role}/{query}` | GET | Autocomplete |
| `/conversations` | GET/POST | List/create conversations |
| `/conversations/{id}` | GET | Get conversation |
| `/conversations/{id}/messages` | POST | Add message |
| `/conversations/{id}/context` | POST | Add context |
| `/conversations/{id}/search-context` | POST | Add search context |
| `/conversations/{id}/context/{cid}` | DELETE/PUT | Delete/update context |

**Workflow Routes:**
- WebSocket endpoint for real-time updates
- Workflow session management
- Parallel execution handlers
- Prompt chaining
- Orchestration and routing

**Startup Process:**
1. Load configuration
2. For each role with TerraphimGraph relevance:
   - Build thesaurus from local KG files via Logseq builder
   - Create RoleGraph
   - Index KG markdown files as documents
   - Process haystack directories recursively
3. Merge local rolegraphs with config_state
4. Initialize summarisation manager
5. Initialize workflow management with WebSocket broadcaster

**Document Description Generation:**
- Extracts meaningful content from markdown
- Collects first header, synonyms, and content lines
- Truncates at 400 characters with safe UTF-8 boundary

**Rolegraph Indexing:**
- Normalizes document IDs via regex `[^a-zA-Z0-9]+` -> `""` -> lowercase
- Saves documents to persistence layer first
- Then indexes into rolegraph for KG queries
- Skips KG files already processed

**Embedded Assets:**
- `embedded-assets` feature: Serves frontend from Rust binary via rust-embed
- Falls back to filesystem lookup when feature disabled

**Static File Handling:**
- Serves `dist/` folder when embedded
- Falls back to `index.html` for SPA routing
- MIME type detection via mime_guess