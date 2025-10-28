# RAG Workflow Implementation Progress

**Feature Branch:** `feature/rag-workflow-context-chat`
**Started:** 2025-10-27
**Status:** In Progress (Phase 1 Complete)

## Goal

Implement complete Search → Select → Chat workflow with conversation persistence in TUI/REPL.

## Progress

### ✅ Phase 1: TuiService Infrastructure (COMPLETE)

**Commit:** 75094110

**Implemented:**
- Added `ContextManager` and `ConversationPersistence` to TuiService
- 11 conversation management methods
- 5 context management methods
- 1 chat_with_context() method for RAG
- Auto-persistence to all backends
- Build passing with all methods

**Methods Added:**
```rust
// Conversation
create_conversation(title) -> ConversationId
load_conversation(id) -> Conversation
list_conversations() -> Vec<ConversationSummary>
delete_conversation(id)
get_conversation(id) -> Option<Conversation>

// Context
add_document_to_context(conv_id, document)
add_search_results_to_context(conv_id, query, docs)
list_context(conv_id) -> Vec<ContextItem>
clear_context(conv_id)
remove_context_item(conv_id, context_id)

// RAG Chat
chat_with_context(conv_id, message, model) -> String
```

### ⏳ Phase 2: REPL Commands (TODO)

**Remaining Work:**

1. **Add Command Enums** (`commands.rs`):
```rust
ReplCommand::Context { subcommand: ContextSubcommand }
ReplCommand::Conversation { subcommand: ConversationSubcommand }

enum ContextSubcommand {
    Add { indices: Vec<usize> },
    List,
    Clear,
    Remove { index: usize },
}

enum ConversationSubcommand {
    New { title: Option<String> },
    Load { id: String },
    List { limit: Option<usize> },
    Show,
    Delete { id: String },
}
```

2. **Add Session State** (`handler.rs`):
```rust
pub struct ReplHandler {
    // ... existing fields
    current_conversation_id: Option<ConversationId>,
    last_search_results: Vec<Document>,
}
```

3. **Implement Handlers** (`handler.rs`):
   - `handle_context()` - Context management
   - `handle_conversation()` - Conversation management
   - Update `handle_search()` - Store results, show indices
   - Update `handle_chat()` - Use chat_with_context() when conversation exists

4. **Build and Test:**
   - Integration tests for complete workflow
   - Test conversation persistence
   - Test context addition from search
   - Test RAG chat

## Example Usage (When Complete)

```bash
$ terraphim-tui repl

# Create conversation
Terraphim Engineer> /conversation new "Research"
✅ Created conversation

# Search with semantic ranking
Terraphim Engineer> /search graph
✅ Found 36 result(s)
  [ 0] 43677 - @memory
  [ 1] 38308 - Architecture
  [ 2] 24464 - knowledge-graph

# Add to context
Terraphim Engineer> /context add 1,2
✅ Added [1]: Architecture
✅ Added [2]: knowledge-graph

# Chat with context (RAG)
Terraphim Engineer> /chat Explain the architecture
🤖 Response: [Uses Architecture + knowledge-graph as context]

# Resume later
Terraphim Engineer> /conversation list
  ▶ conv-123 - Research (2 messages, 2 context)
```

## Technical Details

### Persistence Flow

```
User adds context/sends message
    ↓
TuiService method (add_document_to_context, chat_with_context)
    ↓
Update ContextManager (in-memory)
    ↓
Get updated Conversation
    ↓
ConversationPersistence.save()
    ↓
Saved to all backends (memory, sqlite, s3, etc.)
    ↓
ConversationIndex updated
```

### Infrastructure Used

- `terraphim_service::context::ContextManager` ✅
- `terraphim_persistence::conversation::OpenDALConversationPersistence` ✅
- `terraphim_service::context::build_llm_messages_with_context()` ✅
- `terraphim_types::{Conversation, ChatMessage, ContextItem}` ✅

## Next Steps

1. Add command enums to commands.rs
2. Add session state to ReplHandler
3. Implement command handlers
4. Update handle_search with indices
5. Update handle_chat with context support
6. Build and test
7. Create integration tests
8. Update documentation
9. Merge to main branch

## Files Modified

- `crates/terraphim_tui/src/service.rs` - Added 238 lines
- `docs/rag-workflow-implementation-plan.md` - Implementation plan

## Files To Modify

- `crates/terraphim_tui/src/repl/commands.rs` - Add command enums
- `crates/terraphim_tui/src/repl/handler.rs` - Add handlers + session state
- `crates/terraphim_tui/tests/rag_workflow_integration_test.rs` - New tests
- `docs/rag-workflow-guide.md` - User documentation

## Estimated Remaining Work

- ~300-400 lines of code
- ~200 lines of tests
- ~200 lines of documentation
- 2-3 hours of development time

## Dependencies

No new dependencies - using existing infrastructure only.
