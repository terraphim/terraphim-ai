# crates/terraphim_task_decomposition

## Purpose
Knowledge-graph-based task decomposition with complexity analysis and execution planning.

## Status: Production-Ready
- ~4,614 LOC (non-test)
- 40/40 tests passing (100%)

## Key Types

### TaskDecomposer (trait)
Decomposition interface for breaking complex tasks into subtasks.

### KnowledgeGraphTaskDecomposer
Implementation using knowledge graph for concept-aware decomposition.

### DecompositionStrategy (enum)
- Sequential - Linear execution
- Parallel - Independent parallel tasks
- Hybrid - Mixed sequential/parallel
- RoleBased - Assign based on agent roles

### ExecutionPlan
Step-by-step execution plan with dependency tracking.

### TaskAnalyzer
Complexity analysis using knowledge graph traversal.

### TerraphimKnowledgeGraph
Knowledge graph integration via concept lookup, path analysis, related terms.

## Integration Points
- Uses terraphim_automata for term matching
- Uses terraphim_rolegraph for graph queries
- Generates execution plans with dependency ordering

## Relevance to TinyClaw Rebuild
Phase 3 feature. Not present in PicoClaw or nanobot. Could enable sophisticated task handling where a chat message like "research X, then summarize Y, then email Z" gets decomposed into a multi-step plan. Similar concept to nanobot's subagent spawning but more structured.
