# crates/terraphim_agent_evolution

## Purpose
Versioned memory, task, and lesson tracking for agent learning and adaptation.

## Status: Production-Ready
- ~7,226 LOC (non-test)
- E2E tests passing

## Key Types

### VersionedMemory
Time-based memory snapshots with short-term and long-term buckets.
- `add_memory(item)` - Add with validation
- `update_memory(id, changes)` - Update existing
- `consolidate_memories()` - Merge related, archive old
- `save_version()` / `load_version()` - Timestamped snapshots

### MemoryItem
Individual memory entries with types:
- Conversation, Task, Lesson, Document, Concept, System, WorkflowEvent
- Access tracking (last accessed, access count)

### VersionedTaskList
Task lifecycle evolution with state tracking.
- Active, completed, archived tasks
- Task state evolution over time

### VersionedLessons
Lessons learned tracking with quality scoring.
- Categorized by domain
- Application tracking

### Workflow Patterns
- PromptChaining, Parallelization, Routing
- OrchestratorWorkers, EvaluatorOptimizer

## Integration Points
- Integrated with terraphim_persistence for durable storage
- Used by terraphim_multi_agent for agent memory
- Workflow patterns for multi-step agent operations

## Relevance to TinyClaw Rebuild
Maps to PicoClaw's SessionManager + MEMORY.md files but with much richer versioning. The VersionedMemory can replace both PicoClaw's session history and nanobot's daily notes / long-term memory split. Context compression (PicoClaw's LLM summarization) could be implemented as a consolidate_memories() operation.
