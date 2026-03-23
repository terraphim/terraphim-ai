# crates/terraphim_goal_alignment

## Purpose
Multi-level goal management with hierarchy, alignment scoring, and conflict detection via knowledge graph.

## Status: Functional (some tests ignored)
- ~4,614 LOC (non-test)
- 15/21 tests passing (6 ignored)

## Key Types

### Goal
Goal representation with hierarchy levels.

### GoalHierarchy
Multi-level goal management: global -> high-level -> local.

### GoalAlignment
Alignment tracking and scoring (0.0-1.0 scale).

### ConflictDetector
Semantic conflict detection using knowledge graph's `is_all_terms_connected_by_path`.

### GoalPropagator
Goal distribution through role hierarchies.

### KnowledgeGraphGoalAnalyzer
Knowledge graph integration for semantic analysis.

## Integration Points
- Uses terraphim_rolegraph for knowledge graph queries
- Caching for performance optimization
- Goal propagation through agent hierarchies

## Relevance to TinyClaw Rebuild
Phase 3 feature. Not present in PicoClaw or nanobot. Enables multi-agent routing where messages are directed to the most goal-aligned agent. Not needed for MVP but differentiates Terraphim's offering.
