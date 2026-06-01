# Chat & Session History - Quick Reference

## 🎯 Overview

Implementation guide for chat and session history functionality in Terraphim AI.

## 📊 Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────┐
│                      FRONTEND (Svelte)                       │
├─────────────────────────────────────────────────────────────┤
│  SessionList  │  Chat Component  │  Context Manager         │
│  Component    │  (Enhanced)      │  UI                      │
└───────┬───────────────┬────────────────────┬────────────────┘
        │               │                    │
        └───────────────┴────────────────────┘
                        │
        ┌───────────────▼────────────────┐
        │      Svelte Stores             │
        │  - currentConversation         │
        │  - conversationList            │
        │  - Auto-save logic             │
        └───────────────┬────────────────┘
                        │
        ┌───────────────▼────────────────┐
        │   Tauri Commands / HTTP API    │
        └───────────────┬────────────────┘
                        │
┌─────────────────────────────────────────────────────────────┐
│                     BACKEND (Rust)                           │
├─────────────────────────────────────────────────────────────┤
│            ConversationService                               │
│  - CRUD operations                                          │
│  - Search & filtering                                       │
│  - Import/Export                                            │
└───────────────┬─────────────────────────────────────────────┘
                │
        ┌───────▼───────┐
        │  Context      │
        │  Manager      │
        └───────┬───────┘
                │
┌───────────────▼─────────────────────────────────────────────┐
│              PERSISTENCE LAYER                               │
├─────────────────────────────────────────────────────────────┤
│         ConversationPersistence (OpenDAL)                    │
│  ┌─────────┬──────────┬──────────┬─────────────┐           │
│  │ SQLite  │ DashMap  │  Memory  │  S3 (opt)   │           │
│  └─────────┴──────────┴──────────┴─────────────┘           │
└─────────────────────────────────────────────────────────────┘
```

## 🔑 Key Components

### Backend Components

| Component | Location | Purpose |
|-----------|----------|---------|
| `ConversationService` | `crates/terraphim_service/src/conversation_service.rs` | **NEW** - Business logic for conversation management |
| `ConversationPersistence` | `crates/terraphim_persistence/src/conversation.rs` | **NEW** - Persistence trait and OpenDAL implementation |
| `ContextManager` | `crates/terraphim_service/src/context_manager.rs` | **ENHANCE** - Add archive, restore, clone methods |
| API Endpoints | `terraphim_server/src/api.rs` | **ENHANCE** - Add 8 new REST endpoints |
| Tauri Commands | `desktop/src-tauri/src/cmd.rs` | **ENHANCE** - Add 9 new commands |

### Frontend Components

| Component | Location | Purpose |
|-----------|----------|---------|
| `SessionList.svelte` | `desktop/src/lib/Chat/SessionList.svelte` | **NEW** - Conversation list with search/filter |
| `Chat.svelte` | `desktop/src/lib/Chat/Chat.svelte` | **ENHANCE** - Integrate session management |
| Stores | `desktop/src/lib/stores.ts` | **ENHANCE** - Add conversation stores, auto-save |

### Data Types (Already Exist)

| Type | Location | Purpose |
|------|----------|---------|
| `Conversation` | `crates/terraphim_types/src/lib.rs:1053` | Full conversation with messages and context |
| `ConversationSummary` | `crates/terraphim_types/src/lib.rs:1123` | Lightweight summary for listing |
| `ChatMessage` | `crates/terraphim_types/src/lib.rs:981` | Individual message with context |
| `ContextItem` | `crates/terraphim_types/src/lib.rs:706` | Context attached to messages |

## 📋 Implementation Checklist

### Phase 1: Backend Foundation ✅ Complete
- [x] Create `ConversationPersistence` trait
- [x] Implement `OpenDALConversationPersistence`
- [x] Create `ConversationService` with:
  - [x] `create_conversation()`
  - [x] `get_conversation()`
  - [x] `update_conversation()`
  - [x] `delete_conversation()`
  - [x] `list_conversations()`
  - [x] `search_conversations()`
- [x] Add 8 new API endpoints (REST)
- [x] Add 9 new Tauri commands
- [x] Write unit tests (target: 80% coverage)

### Phase 2: Frontend UI ✅ Complete
- [x] Create `SessionList.svelte` component
- [x] Add conversation stores to `stores.ts`
- [x] Implement auto-save with 2s debounce
- [x] Enhance `Chat.svelte`:
  - [x] Session sidebar toggle
  - [x] Load from `currentConversation` store
  - [x] Auto-save integration
- [ ] Write component tests — Partially complete

### Phase 3: Search & Filtering ✅ Complete
- [x] Backend search implementation
- [x] Frontend search UI
- [x] Filter by role, date, tags
- [ ] Keyboard shortcuts — NOT IMPLEMENTED

### Phase 4: Import/Export ✅ Complete
- [x] JSON export endpoint
- [x] JSON import with validation
- [ ] Bulk export — NOT IMPLEMENTED
- [x] Frontend UI for import/export

### Phase 5: Polish 🔄 Partial
- [ ] Performance optimization — NOT IMPLEMENTED
- [ ] Pagination for messages — NOT IMPLEMENTED
- [ ] Virtual scrolling — NOT IMPLEMENTED
- [ ] Analytics dashboard — NOT IMPLEMENTED
- [ ] Documentation — NOT IMPLEMENTED
- [ ] E2E tests — Partially implemented (Playwright tests exist)

## 🚀 Quick Start Guide

### 1. Backend Setup

```rust
// crates/terraphim_persistence/src/conversation.rs
#[async_trait]
pub trait ConversationPersistence {
    async fn save(&self, conversation: &Conversation) -> Result<()>;
    async fn load(&self, id: &ConversationId) -> Result<Conversation>;
    async fn delete(&self, id: &ConversationId) -> Result<()>;
    async fn list_ids(&self) -> Result<Vec<ConversationId>>;
}

pub struct OpenDALConversationPersistence {
    storage: Arc<DeviceStorage>,
    cache: LruCache<ConversationId, Conversation>,
}
```

### 2. Service Layer

```rust
// crates/terraphim_service/src/conversation_service.rs
pub struct ConversationService {
    persistence: Arc<Mutex<dyn ConversationPersistence>>,
    context_manager: Arc<Mutex<ContextManager>>,
    cache: LruCache<ConversationId, Conversation>,
}

impl ConversationService {
    pub async fn create_conversation(
        &mut self,
        title: String,
        role: RoleName,
    ) -> Result<Conversation> {
        let conversation = Conversation::new(title, role);
        self.persistence.lock().await.save(&conversation).await?;
        self.cache.put(conversation.id.clone(), conversation.clone());
        Ok(conversation)
    }
}
```

### 3. API Endpoints

```rust
// terraphim_server/src/api.rs
pub async fn list_conversations(
    Query(params): Query<ListConversationsParams>,
) -> Result<Json<ListConversationsResponse>> {
    let service = get_conversation_service().lock().await;
    let conversations = service.list_conversations(params.into()).await?;
    Ok(Json(ListConversationsResponse { conversations }))
}
```

### 4. Frontend Store

```typescript
// desktop/src/lib/stores.ts
export const currentConversation: Writable<Conversation | null> = writable(null);
export const conversationList: Writable<ConversationSummary[]> = writable([]);

export function setupAutoSave() {
  let saveTimeout: NodeJS.Timeout;
  currentConversation.subscribe(conversation => {
    if (conversation) {
      clearTimeout(saveTimeout);
      saveTimeout = setTimeout(async () => {
        await saveConversation(conversation);
      }, 2000);
    }
  });
}
```

### 5. Frontend Component

```svelte
<!-- desktop/src/lib/Chat/SessionList.svelte -->
<script lang="ts">
  async function loadConversations() {
    const response = $is_tauri
      ? await invoke('list_all_conversations', { limit: 100 })
      : await fetch(`${CONFIG.ServerURL}/conversations?limit=100`).then(r => r.json());
    conversationList.set(response.conversations);
  }
</script>

<div class="session-list">
  {#each $filteredConversations as conversation}
    <div class="session-item" on:click={() => selectConversation(conversation.id)}>
      {conversation.title}
    </div>
  {/each}
</div>
```

## 📡 API Reference

### REST Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/conversations` | List conversations with pagination |
| `GET` | `/api/conversations/:id` | Get specific conversation |
| `POST` | `/api/conversations` | Create new conversation |
| `PUT` | `/api/conversations/:id` | Update conversation metadata |
| `DELETE` | `/api/conversations/:id` | Delete conversation |
| `GET` | `/api/conversations/search` | Search conversations |
| `POST` | `/api/conversations/:id/export` | Export to JSON |
| `POST` | `/api/conversations/import` | Import from JSON |

### Tauri Commands

| Command | Parameters | Returns |
|---------|------------|---------|
| `list_all_conversations` | `skip, limit, filter` | `ListConversationsResponse` |
| `create_new_conversation` | `title, role` | `CreateConversationResponse` |
| `load_conversation` | `conversation_id` | `GetConversationResponse` |
| `update_conversation_info` | `conversation_id, title, metadata` | `UpdateConversationResponse` |
| `delete_conversation_by_id` | `conversation_id` | `DeleteConversationResponse` |
| `search_conversation_history` | `query, limit` | `SearchConversationsResponse` |
| `export_conversation_to_file` | `conversation_id, file_path` | `ExportConversationResponse` |
| `import_conversation_from_file` | `file_path` | `ImportConversationResponse` |
| `get_conversation_stats` | - | `ConversationStatistics` |

## 💾 Storage Structure

```
conversations/
├── index.json                    # Index of all conversations
├── {uuid-1}.json                # Individual conversation files
├── {uuid-2}.json
├── {uuid-3}.json
└── archive/
    ├── {uuid-archived}.json     # Archived conversations
    └── ...
```

## 🧪 Testing Commands

```bash
# Backend tests
cargo test --package terraphim_persistence conversation_persistence
cargo test --package terraphim_service conversation_service

# Frontend tests
cd desktop
yarn test:unit SessionList
yarn test:e2e chat-session

# Integration tests
./scripts/test_conversation_integration.sh
```

## 🔧 Configuration

No new configuration required! Uses existing:
- OpenDAL profiles for persistence
- Tauri configuration for desktop
- Server configuration for web mode

## 📚 Related Documentation

- **Full Specification**: `docs/specifications/chat-session-history-spec.md`
- **Architecture Docs**: `docs/architecture/`
- **API Documentation**: `docs/api/`
- **Types Documentation**: `crates/terraphim_types/src/lib.rs` (lines 979-1377)

## 🎯 Success Criteria

- [x] Users can create, view, edit, and delete conversations
- [x] Conversations persist across sessions
- [x] Search returns relevant results in < 500ms
- [x] Auto-save works without data loss
- [x] UI is responsive with 100+ conversations
- [x] Export/import maintains data integrity
- [ ] 80%+ test coverage for new code — Partial (frontend tests incomplete)
- [x] Zero regressions in existing functionality

## 📞 Support

For questions or issues during implementation:
1. Check the full specification document
2. Review existing `ContextManager` implementation
3. Examine `terraphim_types` for data structures
4. Refer to OpenDAL documentation for persistence patterns

---

**Quick Ref Version**: 1.0.0
**Last Updated**: 2026-06-01
**Status**: Implemented — All core features complete
