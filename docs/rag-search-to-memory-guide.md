# Adding Search Results to Memory: RAG Workflow Guide

Complete guide for using the Search → Select → Chat workflow with persistent memory in Terraphim TUI/REPL.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Detailed Workflow](#detailed-workflow)
- [Command Reference](#command-reference)
- [Persistence and Sessions](#persistence-and-sessions)
- [Advanced Usage](#advanced-usage)
- [Troubleshooting](#troubleshooting)

## Overview

The RAG (Retrieval-Augmented Generation) workflow allows you to:

1. **Search** knowledge graph with semantic/embeddings search
2. **Select** specific documents to add as context
3. **Chat** with LLM using selected documents (RAG)
4. **Persist** conversations across sessions

### Key Concepts

- **Conversation**: A chat session with message history and context
- **Context**: Documents added to inform the LLM's responses
- **Session**: Your current REPL session with active conversation
- **Persistence**: Conversations saved to disk (sqlite/memory/s3)

## Quick Start

### Basic Workflow (5 steps)

```bash
# 1. Start REPL
$ terraphim-tui repl

# 2. Search knowledge graph
Terraphim Engineer> /search graph
✅ Found 36 result(s)
  [ 0] 43677 - @memory
  [ 1] 38308 - Architecture
  [ 2] 24464 - knowledge-graph
  [ 3] 24439 - knowledge-graph-system
💡 Use /context add <indices> to add to context

# 3. Add documents to memory/context
Terraphim Engineer> /context add 1,2,3
📝 Created conversation: Session 2025-10-27 15:30
✅ Added [1]: Architecture
✅ Added [2]: knowledge-graph
✅ Added [3]: knowledge-graph-system

# 4. View what's in memory
Terraphim Engineer> /context list
Context items (3):
  [ 0] Architecture (score: 38308)
  [ 1] knowledge-graph (score: 24464)
  [ 2] knowledge-graph-system (score: 24439)

# 5. Chat with context (RAG)
Terraphim Engineer> /chat Explain how the knowledge graph system works
🤖 Response (with context):
Based on the provided documentation (Architecture, knowledge-graph,
knowledge-graph-system), the knowledge graph system in Terraphim...
[LLM response informed by selected documents]
```

## Detailed Workflow

### Step 1: Search with TerraphimGraph

The search uses semantic/embeddings-based ranking with knowledge graph:

```bash
# Basic search
/search <query>

# With options
/search <query> --limit 20
/search <query> --semantic
/search <query> --concepts
/search <query> --role "Rust Engineer"
```

**Example:**
```bash
Terraphim Engineer> /search knowledge graph architecture
🔍 Searching for: 'knowledge graph architecture' (Standard Search)
📱 Offline mode - searching local haystacks
🧠 TerraphimGraph search initiated for role: Terraphim Engineer
╭────┬───────┬─────────────────────────────┬────────────────────╮
│ #  ┆ Rank  ┆ Title                       ┆ URL                │
╞════╪═══════╪═════════════════════════════╪════════════════════╡
│  0 ┆ 43677 ┆ @memory                     ┆ docs/src/...       │
│  1 ┆ 38308 ┆ Architecture                ┆ docs/src/...       │
│  2 ┆ 24464 ┆ knowledge-graph             ┆ docs/src/kg/...    │
│  3 ┆ 24439 ┆ knowledge-graph-system      ┆ docs/src/kg/...    │
╰────┴───────┴─────────────────────────────┴────────────────────╯
✅ Found 36 result(s) using Standard Search

💡 Use /context add <indices> to add documents to context
```

**Key Points:**
- Results ranked by knowledge graph connectivity
- Higher rank = more relevant via graph relationships
- Index column `#` shows selection numbers
- Results stored in memory for next step

### Step 2: Add Documents to Context (Memory)

Select documents from search results to add as context:

```bash
# Single document
/context add 1

# Multiple documents (comma-separated)
/context add 1,2,3

# Range of documents
/context add 1-5

# Combination
/context add 0,2-4,7
```

**What Happens:**
1. Auto-creates conversation if none exists
2. Converts selected documents to ContextItems
3. Adds to conversation's global_context
4. Persists conversation to disk
5. Context available for all future chat messages

**Example:**
```bash
Terraphim Engineer> /context add 1,2,3
📝 Created conversation: Session 2025-10-27 15:30
✅ Added [1]: Architecture
✅ Added [2]: knowledge-graph
✅ Added [3]: knowledge-graph-system
```

**Context Item Structure:**
```json
{
  "id": "ctx-uuid",
  "type": "Document",
  "title": "Architecture",
  "summary": "Document description",
  "content": "Full document text...",
  "relevance_score": 38308,
  "created_at": "2025-10-27T15:30:00Z"
}
```

### Step 3: View Context (Memory)

Check what documents are in conversation memory:

```bash
/context list
```

**Example Output:**
```bash
Terraphim Engineer> /context list
Context items (3):
  [ 0] Architecture (score: 38308)
  [ 1] knowledge-graph (score: 24464)
  [ 2] knowledge-graph-system (score: 24439)
```

**Each item shows:**
- Index for removal
- Document title
- Relevance score from original search

### Step 4: Chat with Context (RAG)

Ask questions and get answers informed by context:

```bash
# Chat with context
/chat <message>

# Interactive mode (prompts for input)
/chat
```

**Example:**
```bash
Terraphim Engineer> /chat Explain how the knowledge graph system integrates with search

💬 Sending message: 'Explain how the knowledge graph system integrates with search'

🤖 Response (with context):

Based on the provided documentation:

The knowledge graph system integrates with search through several mechanisms:

1. **Semantic Ranking** (from Architecture.md):
   The TerraphimGraph scorer uses graph connectivity to rank results...

2. **Concept Expansion** (from knowledge-graph.md):
   The knowledge graph enables automatic concept expansion...

3. **Integration Points** (from knowledge-graph-system.md):
   The system connects through the ConfigState which maintains RoleGraphs...

[Detailed response using all 3 documents as context]
```

**What Happens Under the Hood:**
1. Gets current conversation from memory
2. Builds LLM messages with context:
   ```
   System: Context Information:
   ### Architecture
   [Full text from Architecture.md]

   ### knowledge-graph
   [Full text from knowledge-graph.md]

   ### knowledge-graph-system
   [Full text from knowledge-graph-system.md]

   User: Explain how the knowledge graph system integrates with search
   ```
3. Sends to LLM (configured in role)
4. Gets response
5. Saves both user message and assistant response to conversation
6. Persists conversation to disk

**Note:** The LLM sees **full document text** from context, not just summaries.

### Step 5: Manage Context

```bash
# Remove a specific document
/context remove 1

# Clear all context
/context clear

# Add more from new search
/search rust async
/context add 0,1
```

**Example:**
```bash
Terraphim Engineer> /context remove 2
✅ Removed: knowledge-graph-system

Terraphim Engineer> /context list
Context items (2):
  [ 0] Architecture (score: 38308)
  [ 1] knowledge-graph (score: 24464)

Terraphim Engineer> /context clear
✅ Context cleared

Terraphim Engineer> /context list
No context items
```

### Step 6: Manage Conversations

```bash
# Create new conversation
/conversation new "Research Topic"

# List all conversations
/conversation list

# Show current conversation
/conversation show

# Load previous conversation
/conversation load conv-abc123

# Delete conversation
/conversation delete conv-abc123
```

**Example:**
```bash
Terraphim Engineer> /conversation new "Knowledge Graph Research"
✅ Created conversation: Knowledge Graph Research (ID: conv-a1b2c3d4)

Terraphim Engineer> /conversation list
Conversations (2):
  ▶ conv-a1b2c3d4 - Knowledge Graph Research (0 msg, 3 ctx)
    conv-d4e5f6g7 - Previous Session (4 msg, 2 ctx)

Terraphim Engineer> /conversation show
Conversation: Knowledge Graph Research
ID: conv-a1b2c3d4
Role: Terraphim Engineer
Messages: 0
Context Items: 3
```

## Command Reference

### Search Commands

```bash
# Basic search
/search <query>

# Search with limit
/search <query> --limit 10

# Semantic search (uses embeddings)
/search <query> --semantic

# With concept expansion
/search <query> --concepts

# Specific role
/search <query> --role "Rust Engineer"

# Combined
/search async programming --semantic --concepts --limit 20
```

### Context Commands

```bash
# Add from last search
/context add <indices>

# Examples:
/context add 1              # Single document
/context add 1,2,3          # Multiple (comma-separated)
/context add 1-5            # Range
/context add 0,2-4,7        # Combination

# Manage context
/context list               # Show all context items
/context remove <index>     # Remove by index
/context clear              # Remove all
```

### Conversation Commands

```bash
# Create new
/conversation new [title]

# If no title provided, auto-generates: "Session YYYY-MM-DD HH:MM"
/conversation new           # Auto title
/conversation new "My Research Project"

# Load existing
/conversation load <id>

# List all
/conversation list
/conversation list --limit 10

# Show current
/conversation show

# Delete
/conversation delete <id>
```

### Chat Commands

```bash
# Chat with message
/chat <message>

# Interactive mode
/chat
💬 Message: [type your message]

# Context behavior:
# - If conversation exists: Uses context (RAG)
# - If no conversation: Direct chat without context
```

## Persistence and Sessions

### How Conversations Are Saved

Conversations are automatically saved to **all configured backends**:

```toml
# Device settings (crates/terraphim_settings/default/settings_*.toml)
[profiles.memory]
type = "memory"

[profiles.sqlite]
type = "sqlite"
datadir = "~/.local/share/terraphim/conversations"

[profiles.s3]
type = "s3"
bucket = "terraphim-conversations"
```

**Every time you:**
- Add context → Auto-saved
- Send chat message → Auto-saved
- Receive response → Auto-saved
- Remove context → Auto-saved

**Storage locations:**
- **Memory**: In-process (lost on exit)
- **SQLite**: `~/.local/share/terraphim/conversations/conv-*.json`
- **S3**: Cloud storage (if configured)

### Resuming Conversations

```bash
# Start new session
$ terraphim-tui repl

# List saved conversations
Terraphim Engineer> /conversation list
Conversations (3):
    conv-a1b2c3 - Knowledge Graph Research (4 msg, 3 ctx)
    conv-d4e5f6 - Rust Async Patterns (2 msg, 1 ctx)
    conv-g7h8i9 - System Architecture (6 msg, 5 ctx)

# Load previous conversation
Terraphim Engineer> /conversation load conv-a1b2c3
✅ Loaded: Knowledge Graph Research (4 messages, 3 context items)

# Continue where you left off
Terraphim Engineer> /chat What were we discussing?
🤖 Response (with context):
We were discussing the knowledge graph system architecture...
[Continues with full context from persisted conversation]
```

### Conversation Structure

Each conversation contains:
```json
{
  "id": "conv-uuid",
  "title": "Knowledge Graph Research",
  "role": "Terraphim Engineer",
  "created_at": "2025-10-27T15:30:00Z",
  "updated_at": "2025-10-27T16:45:00Z",
  "messages": [
    {
      "id": "msg-1",
      "role": "user",
      "content": "Explain architecture",
      "context_items": [],
      "created_at": "2025-10-27T15:35:00Z"
    },
    {
      "id": "msg-2",
      "role": "assistant",
      "content": "The architecture consists of...",
      "model": "llama3.2:3b",
      "created_at": "2025-10-27T15:35:15Z"
    }
  ],
  "global_context": [
    {
      "id": "ctx-1",
      "type": "Document",
      "title": "Architecture",
      "content": "...",
      "relevance_score": 38308
    }
  ]
}
```

## Advanced Usage

### Multi-Document Context Building

Build context from multiple searches:

```bash
# Search 1: Architecture
Terraphim Engineer> /search architecture
/context add 0,1

# Search 2: Implementation
Terraphim Engineer> /search rust implementation
/context add 2,3

# Search 3: Testing
Terraphim Engineer> /search testing strategies
/context add 1,4

# View all collected context
Terraphim Engineer> /context list
Context items (6):
  [ 0] Architecture Overview
  [ 1] System Design
  [ 2] Rust Best Practices
  [ 3] Implementation Guide
  [ 4] Testing Strategies
  [ 5] Integration Tests

# Chat uses ALL 6 documents as context
Terraphim Engineer> /chat How do I test the architecture implementation?
```

### Refining Context

Remove irrelevant documents to improve LLM responses:

```bash
Terraphim Engineer> /context list
Context items (5):
  [ 0] Architecture (score: 38308)
  [ 1] knowledge-graph (score: 24464)
  [ 2] Unrelated Doc (score: 1000)
  [ 3] Another Good Doc (score: 20000)
  [ 4] Old Version (score: 500)

# Remove low-relevance items
Terraphim Engineer> /context remove 2
Terraphim Engineer> /context remove 4

Terraphim Engineer> /context list
Context items (3):
  [ 0] Architecture (score: 38308)
  [ 1] knowledge-graph (score: 24464)
  [ 2] Another Good Doc (score: 20000)

# Now chat with refined context
Terraphim Engineer> /chat <question>
```

### Role-Specific Knowledge

Each role has different knowledge sources:

```bash
# Terraphim Engineer - Project docs
Terraphim Engineer> /search components
  # Results from docs/src haystack

# Rust Engineer - Rust docs + Reddit
Terraphim Engineer> /role select "Rust Engineer"
Rust Engineer> /search tokio
  # Results from query.rs (Rust std docs + Reddit)

# Each role gets separate conversations
Rust Engineer> /conversation list
  # Only shows Rust Engineer conversations
```

### Conversation Workflows

**Research Workflow:**
```bash
/conversation new "Feature Research"
/search <topic>
/context add <relevant docs>
/chat Summarize the key points
/chat What are the tradeoffs?
/chat How would you implement this?
```

**Debugging Workflow:**
```bash
/conversation new "Bug Investigation"
/search <error message>
/context add <related code/docs>
/chat What could cause this error?
/chat How do I fix it?
```

**Learning Workflow:**
```bash
/conversation new "Learning Rust Async"
/search rust async
/context add 0-5
/chat Explain async/await
/chat Show me examples
/chat What are common pitfalls?
```

## Context Limits

Default limits (configurable in `terraphim_service::context::ContextConfig`):

```rust
max_context_items: 50          // Max documents per conversation
max_context_length: 100_000    // ~100K characters total
```

**If you exceed limits:**
```bash
Terraphim Engineer> /context add 1,2,3,...,60
❌ Error: Maximum context items reached for this conversation

# Solution: Remove old items or start new conversation
Terraphim Engineer> /context clear
Terraphim Engineer> /context add 1,2,3  # Add fresh context
```

## Persistence Backends

### Memory (Default)

```toml
[profiles.memory]
type = "memory"
```

- **Pros**: Fast, no disk I/O
- **Cons**: Lost on exit
- **Use case**: Temporary sessions

### SQLite (Recommended)

```toml
[profiles.sqlite]
type = "sqlite"
datadir = "~/.local/share/terraphim/conversations"
```

- **Pros**: Persistent, fast, local
- **Cons**: Single machine only
- **Use case**: Desktop usage

### S3 (Cloud)

```toml
[profiles.s3]
type = "s3"
bucket = "terraphim-conversations"
region = "us-east-1"
access_key_id = "..."
secret_access_key = "..."
```

- **Pros**: Synced across devices
- **Cons**: Requires network, setup
- **Use case**: Multi-device workflows

### Multiple Backends

Conversations saved to **all** backends simultaneously:

```bash
# Search reads from fastest
# Writes go to all backends

# Example config with 3 backends:
- memory (fastest read)
- sqlite (local persistence)
- s3 (cloud backup)
```

## Example: Complete Research Session

```bash
$ terraphim-tui repl

# Start research
Terraphim Engineer> /conversation new "Knowledge Graph Deep Dive"
✅ Created conversation: Knowledge Graph Deep Dive

# Initial search - architecture
Terraphim Engineer> /search knowledge graph architecture
✅ Found 36 result(s)
  [ 0] @memory
  [ 1] Architecture
  [ 2] knowledge-graph
  [ 3] knowledge-graph-system

# Add core docs
Terraphim Engineer> /context add 1-3
✅ Added [1]: Architecture
✅ Added [2]: knowledge-graph
✅ Added [3]: knowledge-graph-system

# Ask foundational question
Terraphim Engineer> /chat Explain the overall architecture
🤖 Response (with context):
The knowledge graph architecture in Terraphim consists of three main layers...
[Detailed response using 3 documents]

# Search for implementation details
Terraphim Engineer> /search rolegraph implementation
✅ Found 24 result(s)
  [ 0] terraphim-rolegraph crate
  [ 1] RoleGraph implementation
  ...

# Add implementation docs
Terraphim Engineer> /context add 0,1
✅ Added [0]: terraphim-rolegraph crate
✅ Added [1]: RoleGraph implementation

# Now context has 5 documents
Terraphim Engineer> /context list
Context items (5):
  [ 0] Architecture
  [ 1] knowledge-graph
  [ 2] knowledge-graph-system
  [ 3] terraphim-rolegraph crate
  [ 4] RoleGraph implementation

# Ask implementation question
Terraphim Engineer> /chat How is RoleGraph implemented?
🤖 Response (with context):
Based on the architecture and implementation docs, RoleGraph is implemented as...
[Response informed by all 5 documents]

# Continue conversation
Terraphim Engineer> /chat What are the key methods?
Terraphim Engineer> /chat How does it integrate with search?

# Save and exit
Terraphim Engineer> /quit
```

**Resume later:**
```bash
$ terraphim-tui repl

Terraphim Engineer> /conversation list
  ▶ conv-abc - Knowledge Graph Deep Dive (6 msg, 5 ctx)

Terraphim Engineer> /conversation load conv-abc
✅ Loaded: Knowledge Graph Deep Dive (6 messages, 5 context items)

# All context restored - continue researching
Terraphim Engineer> /chat Let's dive deeper into...
```

## Troubleshooting

### No Results from Search

**Problem:**
```bash
/search test
✅ Found 0 result(s)
```

**Solutions:**
- Check role has haystacks configured
- Verify haystack path exists
- Try different search terms
- Use different role with different knowledge base

### Context Add Fails

**Problem:**
```bash
/context add 5
⚠️  Index 5 out of range (have 3 results)
```

**Solution:**
- Search returns 0-indexed results
- Check available indices in search output
- Add only valid indices

### Chat Without Context

**Problem:**
```bash
/chat Hello
🤖 Response:
No LLM configured for role...
```

**Solutions:**
- Configure LLM in role (ollama_base_url, llm_model)
- Check role has LLM provider set
- See role configuration in `/config show`

### Conversation Not Found

**Problem:**
```bash
/conversation load conv-xyz
❌ Error: Conversation not found
```

**Solutions:**
- List conversations: `/conversation list`
- Check ID is correct
- Conversation may have been deleted
- Check persistence backend is working

### Out of Context Space

**Problem:**
```bash
/context add 1,2,3,...
❌ Maximum context items reached
```

**Solutions:**
```bash
# Option 1: Remove old items
/context remove 0
/context remove 1

# Option 2: Clear and rebuild
/context clear
/context add <new items>

# Option 3: Start new conversation
/conversation new "New Topic"
/context add <items>
```

## Best Practices

### 1. Focused Context

**Do:**
```bash
/search specific topic
/context add top 3-5 most relevant
/chat <specific question>
```

**Don't:**
```bash
/context add 0-20  # Too broad
/chat <vague question>
```

### 2. Iterative Refinement

```bash
# Start broad
/search architecture
/context add 0,1

# Chat and identify gaps
/chat Explain X
# Response reveals need for Y

# Add targeted docs
/search Y implementation
/context add 0

# Chat again with better context
/chat Now explain X with Y
```

### 3. Conversation Organization

```bash
# Separate conversations by topic
/conversation new "Feature A Research"
/conversation new "Bug B Investigation"
/conversation new "Learning Topic C"

# Easy to resume each independently
/conversation list
/conversation load <specific topic>
```

### 4. Context Hygiene

```bash
# Periodically review context
/context list

# Remove outdated/irrelevant items
/context remove <index>

# Start fresh when topic changes
/conversation new "New Topic"
```

## Integration with Knowledge Graph

### How TerraphimGraph Search Works

```
User: /search graph
    ↓
TerraphimGraph scorer activated
    ↓
Query expanded via knowledge graph concepts
    ↓
Documents ranked by:
    - Direct matches
    - Graph connectivity
    - Concept relationships
    - Semantic similarity
    ↓
Results with high rank = strong graph connections
```

**Example:**
```
Query: "graph"
  ↓
KG expands to: graph, knowledge-graph, graph-database, visualization, ...
  ↓
Searches for ALL related concepts
  ↓
Ranks by graph connectivity:
  - @memory: 43677 (highly connected)
  - Architecture: 38308 (many graph links)
  - knowledge-graph: 24464 (core concept)
```

### Context Selection Strategy

**High-rank documents (>30000):**
- Core concepts with many connections
- Frequently referenced
- Comprehensive coverage

**Medium-rank documents (10000-30000):**
- Specific topics
- Implementation details
- Targeted information

**Low-rank documents (<10000):**
- Peripheral information
- May be less relevant
- Consider excluding

**Recommendation:**
```bash
# Add high and medium rank docs
/context add 0-5  # Ranks 43677 to 24464

# Skip very low rank docs
# Don't add indices with rank < 5000
```

## Advanced: Custom Persistence

### Configure Storage

Edit device settings file:

```toml
# ~/.config/terraphim/settings.toml
[profiles.custom]
type = "fs"
root = "/path/to/my/conversations"
```

### Backup Conversations

```bash
# Conversations stored as JSON
$ ls ~/.local/share/terraphim/conversations/
conv-a1b2c3d4.json
conv-d4e5f6g7.json
index.json

# Backup
$ tar -czf conversations-backup.tar.gz ~/.local/share/terraphim/conversations/
```

### Export Conversation

```bash
# Load conversation
/conversation load conv-abc

# Show in REPL, then copy/paste or:
# Find JSON file and copy
$ cp ~/.local/share/terraphim/conversations/conv-abc.json exported/
```

## See Also

- [TUI Usage Guide](./tui-usage.md) - General TUI/REPL usage
- [Command Execution System](./command-execution-system.md) - How commands work
- [RAG Workflow Implementation Plan](./rag-workflow-implementation-plan.md) - Technical details

## Related Code

| Component | File |
|-----------|------|
| TuiService RAG Methods | `crates/terraphim_tui/src/service.rs:292-509` |
| Context Handlers | `crates/terraphim_tui/src/repl/handler.rs:3431-3611` |
| Conversation Handlers | `crates/terraphim_tui/src/repl/handler.rs:3500-3587` |
| Command Definitions | `crates/terraphim_tui/src/repl/commands.rs:275-303` |
| ContextManager | `crates/terraphim_service/src/context.rs` |
| ConversationPersistence | `crates/terraphim_persistence/src/conversation.rs` |
