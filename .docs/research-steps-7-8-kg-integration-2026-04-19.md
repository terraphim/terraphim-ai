# Research Document: Knowledge Graph Integration & Cross-Agent Feedback Loop (Steps 7-8)

**Status**: Draft
**Author**: Terraphim AI Agent
**Date**: 2026-04-19
**Reviewers**: [To be assigned]

## Executive Summary

Steps 7 and 8 close the gap between Terraphim's shared learning system and its knowledge graph (RoleGraph) infrastructure. Currently, learnings are stored as markdown files but are invisible to graph queries. The knowledge graph contains rich semantic relationships but has no awareness of learning effectiveness. We need bidirectional integration: learnings enrich the graph (Step 7), and graph usage enriches learnings (Step 8).

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | This is the culmination of the Learning & KG cluster. Without it, Steps 1-6 are siloed. |
| Leverages strengths? | Yes | We have mature RoleGraph (terraphim_rolegraph), SharedLearning types, and markdown persistence. |
| Meets real need? | Yes | Agents currently cannot discover learnings through graph queries, and graph rankings ignore learning quality. |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Description
Terraphim has two rich data structures that don't talk to each other:
1. **SharedLearning** - markdown-backed learnings with trust levels, quality metrics, and keywords
2. **RoleGraph** - semantic knowledge graph with nodes, edges, thesaurus, and document rankings

When an agent queries the knowledge graph for "rust error handling", it finds graph nodes but misses relevant learnings. When a learning about "cargo clippy lints" accumulates high quality scores, the graph doesn't know to boost related nodes.

### Impact
- **Agent effectiveness**: Agents miss contextual learnings during graph queries
- **Graph accuracy**: Graph rankings don't reflect real-world learning effectiveness
- **Trust propagation**: High-quality learnings don't strengthen related graph concepts
- **Cross-agent value**: Shared learnings can't be discovered through the primary query interface

### Success Criteria
1. Graph queries return relevant learnings alongside nodes/documents
2. Learning quality metrics influence graph node rankings
3. New learnings automatically add keywords to the thesaurus
4. Learning application updates both learning quality and graph edge weights
5. No performance regression in graph query latency (< 100ms target)

## Current State Analysis

### Existing Implementation

**SharedLearning System (Post-Step 6):**
- `SharedLearningStore` backed by `MarkdownLearningStore`
- Learnings stored at `{data_dir}/learnings/{agent_id}/{id}.md`
- Shared learnings in `{data_dir}/learnings/shared/`
- BM25-based deduplication in `store.rs`
- Quality metrics: `applied_count`, `effective_count`, `agent_count`, `success_rate`
- Keywords stored in `SharedLearning.keywords` for search

**RoleGraph (terraphim_rolegraph):**
- `RoleGraph` with `AHashMap<u64, Node>`, `AHashMap<u64, Edge>`, `Thesaurus`
- `Thesaurus` maps `NormalizedTermValue` -> `NormalizedTerm` (with auto IDs)
- Aho-Corasick automata for fast text matching
- TF-IDF `TriggerIndex` for semantic fallback
- Documents indexed with rankings
- `find_matching_node_ids()` for querying
- `is_all_terms_connected_by_path()` for connectivity

**Knowledge Graph Consumers:**
- `terraphim_task_decomposition` - queries graph for task planning
- `terraphim_goal_alignment` - analyses goal conflicts via graph
- `terraphim_router` - routes queries based on graph concepts
- `terraphim_middleware` - ranks search results using graph

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| SharedLearning types | `crates/terraphim_agent/src/shared_learning/types.rs` | Core learning data structures |
| SharedLearning store | `crates/terraphim_agent/src/shared_learning/store.rs` | Markdown-backed store with BM25 dedup |
| Markdown store | `crates/terraphim_agent/src/shared_learning/markdown_store.rs` | File persistence layer |
| RoleGraph | `crates/terraphim_rolegraph/src/lib.rs` | Knowledge graph with nodes/edges/thesaurus |
| Node/Edge types | `crates/terraphim_types/src/lib.rs:550-600` | Graph primitive types |
| Thesaurus | `crates/terraphim_types/src/lib.rs:632+` | Synonym -> concept mapping |
| Task Decomposition KG | `crates/terraphim_task_decomposition/src/knowledge_graph.rs` | Graph queries for task planning |
| Goal Alignment KG | `crates/terraphim_goal_alignment/src/knowledge_graph.rs` | Graph-based goal analysis |
| Middleware ranking | `crates/terraphim_middleware/tests/rolegraph_knowledge_graph_ranking_test.rs` | Graph ranking tests |

### Data Flow (Current)

```
[Session] -> [Learning Capture] -> [SharedLearning] -> [Markdown File]
                                                          ↓
[Agent Query] -> [RoleGraph] -> [Nodes/Edges/Documents]
```

**Gap**: No connection between the two branches.

### Data Flow (Target)

```
[Session] -> [Learning Capture] -> [SharedLearning] -> [Markdown File]
                                                          ↓ (Step 7)
[Agent Query] -> [RoleGraph] -> [Nodes/Edges/Documents/Learnings]
                                                          ↓ (Step 8)
                                           [Quality Updates] -> [SharedLearning]
```

## Constraints

### Technical Constraints

1. **No breaking changes to RoleGraph API**: The graph is used by multiple crates. Changes must be backward-compatible.
2. **Async context**: `SharedLearningStore` is async (tokio). `RoleGraph` has both sync and async APIs.
3. **Markdown-first**: Learnings must remain markdown files. No SQLite or secondary storage.
4. **Feature flags**: New functionality must be gated (e.g., `kg-integration`, `feedback-loop`).
5. **Performance**: Graph query latency must remain < 100ms. Learning index must not slow down graph builds.

### Business Constraints

1. **Time**: These are the final steps of the Learning & KG cluster.
2. **Scope**: Must not expand into full graph learning or GNN territory.
3. **Compatibility**: Must work with existing Aider/Cline connectors from Steps 2-3.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Graph query latency | < 100ms | ~10-50ms (estimated) |
| Learning injection latency | < 50ms per learning | N/A (new) |
| Memory overhead | < 10MB for learning index | N/A (new) |
| Startup hydration | < 500ms for 1000 learnings | ~100ms for 100 learnings |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| RoleGraph API stability | Breaking changes would require updating 4+ downstream crates | middleware, router, task_decomposition, goal_alignment all depend on it |
| Markdown persistence | Core Terraphim pattern; divergence would create maintenance burden | Design doc explicitly rejected SQLite |
| Feature flags | Default builds must remain lean; KG integration is opt-in | Steps 1-6 all used feature flags |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Graph Neural Networks / embeddings | Overkill for current scale; TF-IDF + BM25 suffice |
| Real-time graph updates on every learning | Batch updates are simpler and sufficient |
| Distributed graph consensus | Single-node assumption holds for current architecture |
| Learning-to-learning edges | Adds complexity without clear value; keywords connect to existing nodes |
| Automatic learning extraction from graph queries | Would require LLM integration; defer to later phase |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_rolegraph` | Core graph API; must extend without breaking | Medium - widely used |
| `terraphim_types` | Node/Edge/Thesaurus types | Low - stable |
| `terraphim_agent` | SharedLearning store | Low - just completed Step 6 |
| `terraphim_middleware` | May need ranking integration | Medium - needs careful coordination |
| `terraphim_task_decomposition` | Consumes graph + learnings | Low - can adapt after |
| `terraphim_goal_alignment` | Consumes graph + learnings | Low - can adapt after |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `ahash` | 0.8 | Low | Standard HashMap (slower) |
| `aho_corasick` | 1.x | Low | Regex (slower) |
| `tokio` | 1.x | Low | async-std (not used) |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| RoleGraph serialization format changes | Medium | High | Add new fields as `Option`; test backward compat |
| Circular dependency (agent -> rolegraph -> agent) | Medium | High | Keep integration at middleware layer, not in agent crate |
| Performance regression in graph queries | Medium | Medium | Benchmark before/after; lazy loading of learning index |
| Learning keyword overlap with thesaurus | Low | Medium | Merge strategy: prefer existing thesaurus terms |

### Open Questions

1. **Should learnings be Documents in the graph?** Or a separate index?
   - As Documents: Reuses existing ranking, but requires `Document` type changes
   - As separate index: Cleaner separation, but requires query-time merging

2. **How do we map learning keywords to node IDs?**
   - Exact match: Fast but misses synonyms
   - Thesaurus lookup: Reuses existing synonym resolution
   - Embedding similarity: More accurate but adds complexity

3. **Which crate owns the integration?**
   - `terraphim_agent`: Natural fit for learning side, but would depend on `rolegraph`
   - `terraphim_middleware`: Natural fit for query side, but would couple routing to learning
   - New crate `terraphim_learning_kg`: Clean separation, but adds crate overhead

4. **When does the feedback loop run?**
   - On every graph query: Real-time but potentially slow
   - On learning application: Event-driven, but misses graph-query-side feedback
   - Scheduled batch: Simple but delayed

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Learnings should be discoverable through graph queries | User request: "connect learnings to graph nodes" | If wrong, we'd build indexing without query integration | Yes - stated in Step 7 |
| Learning quality should influence graph rankings | User request: "close the loop between learning and knowledge graph" | If wrong, we'd only do one-way integration | Yes - stated in Step 8 |
| Keywords are sufficient for linking learnings to nodes | SharedLearning already has keywords field | If wrong, we'd need NLP/embedding extraction | Partial - needs validation |
| RoleGraph queries happen more often than learning storage | Typical usage: many queries, few learnings | If wrong, optimization priorities invert | No - needs telemetry |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| **A**: Learnings become first-class graph nodes | Full graph integration; learnings have edges to other nodes | Rejected: Too invasive; changes graph semantics |
| **B**: Learnings are indexed as Documents | Reuses document ranking; minimal graph changes | **Chosen for Step 7**: Aligns with existing Document flow |
| **C**: Learnings are separate from graph, merged at query time | Cleanest separation; no graph changes | Rejected for Step 7: Harder to rank together; may use for advanced cases |
| **D**: Feedback updates node/edge ranks directly | Graph rankings reflect learning quality | **Chosen for Step 8**: Direct impact on query results |
| **E**: Feedback only updates learning quality, not graph | Safer; no graph mutation | Rejected: Doesn't "close the loop" to the graph |

## Research Findings

### Key Insights

1. **Document type is the natural integration point**: `Document` already has `doc_type`, `tags`, `rank`, and `synonyms`. Adding a `Learning` document type is the least invasive way to include learnings in graph queries.

2. **`IndexedDocument` wraps Document with graph context**: The graph indexes `IndexedDocument` (not raw `Document`), which includes `node_ids` and `edge_ids`. This is where learning-to-node linking happens.

3. **Thesaurus already supports keyword expansion**: When a learning has keyword "rust", the thesaurus can map it to node ID 42 ("rust programming language"). This gives us synonym resolution for free.

4. **Quality metrics map naturally to document rank**: `SharedLearning.quality.success_rate` (0.0-1.0) can boost `Document.rank` (u64). A learning with 90% success rate gets higher rank than one with 30%.

5. **Middleware ranking tests show extension points**: `crates/terraphim_middleware/tests/rolegraph_knowledge_graph_ranking_test.rs` demonstrates how graph results are ranked and returned. This is where learning results merge with graph results.

6. **Feature flags keep default builds lean**: All prior steps used feature flags. We should continue with `kg-integration` for Step 7 and `feedback-loop` for Step 8.

### Relevant Prior Art

- **Terraphim RoleGraph ranking**: `crates/terraphim_rolegraph/examples/knowledge_graph_role_demo.rs` shows how documents are ranked via node rank + edge rank + doc_rank.
- **Middleware ranking tests**: `crates/terraphim_middleware/tests/rolegraph_knowledge_graph_ranking_test.rs` shows query -> graph -> ranked results pipeline.
- **Task decomposition KG**: `crates/terraphim_task_decomposition/src/knowledge_graph.rs` shows how external crates consume graph queries.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Document type extension | Verify `Document` can carry learning metadata without breaking serialization | 2 hours |
| Query-time merging | Prototype merging learning results with graph results in middleware | 4 hours |
| Rank influence | Test how adjusting `Document.rank` affects overall result ordering | 2 hours |

## Recommendations

### Proceed/No-Proceed

**Proceed** - The integration is essential for realising value from Steps 1-6. The architecture is clear, risks are manageable, and we have a natural integration point via `Document`/`IndexedDocument`.

### Scope Recommendations

1. **Step 7 (KG Integration)**: Add learnings as `Document` entries with `doc_type = Learning`. Index them in `RoleGraph` like any other document. Link learning keywords to node IDs via thesaurus lookup.
2. **Step 8 (Feedback Loop)**: On learning application, boost ranks of linked graph nodes. On graph query for linked nodes, increment learning's `applied_count`.
3. **Defer**: Learning-to-learning edges, embedding-based similarity, real-time graph mutations.

### Risk Mitigation Recommendations

1. **API Stability**: Add learning integration as new methods on `RoleGraph`, don't modify existing query signatures.
2. **Performance**: Lazy-load learning index; only build when `kg-integration` feature is enabled.
3. **Circular deps**: Integration lives in `terraphim_middleware` (which already depends on both `rolegraph` and `agent` types), not in `terraphim_agent`.

## Next Steps

If approved:
1. Conduct technical spikes (8 hours total)
2. Create detailed design document for Step 7 (KG Integration)
3. Create detailed design document for Step 8 (Feedback Loop)
4. Request human approval for design
5. Implement Step 7
6. Implement Step 8
7. Run quality gates and merge

## Appendix

### Reference Materials

- `.docs/design-learning-kg-2026-04-17.md` - Original 8-step implementation plan
- `.docs/design-shared-learning-store-migration-2026-04-19.md` - Step 6 design (completed)
- `crates/terraphim_rolegraph/src/lib.rs` - RoleGraph implementation
- `crates/terraphim_types/src/lib.rs:479-600` - Document, Node, Edge types
- `crates/terraphim_agent/src/shared_learning/store.rs` - SharedLearningStore (post-Step 6)
- `crates/terraphim_middleware/tests/rolegraph_knowledge_graph_ranking_test.rs` - Ranking test examples

### Code Snippets

**RoleGraph query interface:**
```rust
pub fn find_matching_node_ids(&self, text: &str) -> Vec<u64> {
    self.ac.find_iter(text)
        .map(|mat| self.aho_corasick_values[mat.pattern()])
        .collect()
}
```

**Document type (from terraphim_types):**
```rust
pub struct Document {
    pub id: String,
    pub title: String,
    pub body: String,
    pub doc_type: DocumentType, // KgEntry, Article, etc.
    pub rank: Option<u64>,
    pub tags: Option<Vec<String>>,
    // ...
}
```

**SharedLearning quality metrics:**
```rust
pub struct QualityMetrics {
    pub applied_count: u32,
    pub effective_count: u32,
    pub agent_count: u32,
    pub success_rate: Option<f64>,
}
```

**IndexedDocument (graph-indexed document):**
```rust
pub struct IndexedDocument {
    pub document: Document,
    pub node_ids: Vec<u64>,
    pub edge_ids: Vec<u64>,
    pub rank: u64,
}
```
