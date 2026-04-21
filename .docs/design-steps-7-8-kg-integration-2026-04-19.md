# Implementation Plan: Knowledge Graph Integration & Cross-Agent Feedback Loop (Steps 7-8)

**Status**: Draft
**Research Doc**: `.docs/research-steps-7-8-kg-integration-2026-04-19.md`
**Author**: Terraphim AI Agent
**Date**: 2026-04-19
**Estimated Effort**: 4-5 days

## Overview

### Summary
This plan implements bidirectional integration between Terraphim's shared learning system and its RoleGraph knowledge graph. Step 7 makes learnings discoverable through graph queries by indexing them as Documents. Step 8 closes the feedback loop by propagating learning quality to graph rankings and graph usage back to learning metrics.

### Approach
Treat learnings as first-class Documents in the RoleGraph. Use the existing thesaurus for keyword-to-node resolution. Implement feedback as rank adjustments on `IndexedDocument` and quality metric updates on `SharedLearning`. Keep all changes behind feature flags and preserve backward compatibility.

### Scope

**In Scope:**
- Step 7: Index SharedLearnings as RoleGraph Documents with node/edge linking
- Step 7: Thesaurus-based keyword-to-node resolution for learnings
- Step 8: Learning quality boosts linked document/node ranks
- Step 8: Graph query usage increments learning applied_count
- Feature flags: `kg-integration` and `feedback-loop`
- Middleware query merging (graph results + learning results)

**Out of Scope:**
- Learning-to-learning graph edges
- Embedding-based semantic similarity
- Real-time graph mutations (batch updates only)
- LLM-based learning extraction from graph
- Distributed/multi-node graph consensus

**Avoid At All Cost:**
- Modifying RoleGraph serialization format in a breaking way
- Adding terraphim_agent -> terraphim_rolegraph dependency (creates cycle risk)
- Storing learning content in the graph itself (keep markdown files as source of truth)
- Building a separate query language for learnings

## Architecture

### Component Diagram

```
+----------------------------------------------------------+
|                   terraphim_middleware                     |
|  (already depends on rolegraph + types)                  |
|                                                          |
|  +-------------------+      +-------------------------+  |
|  |  LearningIndexer  |----->|  RoleGraph (extended)   |  |
|  |  (Step 7)         |      |  - learning_documents   |  |
|  +-------------------+      |  - thesaurus_lookup     |  |
|                             +-------------------------+  |
|  +-------------------+      +-------------------------+  |
|  |  FeedbackLoop     |----->|  SharedLearningStore    |  |
|  |  (Step 8)         |      |  (via types only)       |  |
|  +-------------------+      +-------------------------+  |
+----------------------------------------------------------+
        |                           |
        v                           v
+----------------------------------------------------------+
|                   Query Pipeline                         |
|  [Query] -> [RoleGraph.search()] -> [Merge Results]     |
|                      ^                                   |
|                      | Learning docs included            |
+----------------------------------------------------------+
```

### Data Flow

**Step 7 - Indexing Flow:**
```
[SharedLearning] -> [Extract Keywords] -> [Thesaurus Lookup] -> [Node IDs]
                                                         |
[SharedLearning] -> [Convert to Document] -> [IndexedDocument with node_ids]
                                                         |
                                    [Add to RoleGraph.documents]
```

**Step 8 - Feedback Flow:**
```
[Learning Applied] -> [Update Quality Metrics] -> [Boost Linked Document Rank]
                                                           |
[Graph Query for Nodes] -> [Find Linked Learnings] -> [Increment applied_count]
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Learnings as Documents | Reuses existing indexing, ranking, and query infrastructure. Minimal new code. | Separate learning index (harder to merge at query time), Graph nodes (changes graph semantics) |
| Integration in middleware | `terraphim_middleware` already depends on `rolegraph` and `types`. Adding `agent` types dependency is safe (types crate only). | Integration in `agent` crate (creates agent -> rolegraph dependency), Integration in `rolegraph` crate (pollutes core graph with learning concepts) |
| Thesaurus keyword lookup | Reuses existing synonym resolution. Fast (Aho-Corasick). No embeddings needed. | Exact string match (misses synonyms), Embedding similarity (adds complexity, deps) |
| Rank boost via Document.rank | Simple u64 adjustment. Already affects ranking in middleware tests. | Custom ranking field (unnecessary), Node rank adjustment (too coarse) |
| Batch feedback updates | Learning application is infrequent; batching is simpler and safer than real-time hooks. | Real-time updates (complexity, race conditions), Event-driven (needs pub/sub infra) |
| Feature flags per step | Allows deploying Step 7 without Step 8. Simpler testing. | Single flag (all-or-nothing), No flags (always on, risky) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Learning as graph nodes with edges | Would require Edge type changes, serialization updates, and new graph algorithms | Breaking changes to core graph; weeks of extra work |
| Embedding-based keyword expansion | Adds `bert` or `openai` dependencies; inference latency; not essential | Bloats default build; adds failure modes |
| Real-time feedback hooks | Would require event bus or callback infrastructure | Over-engineering for current query volume |
| New crate for integration | 4-5 files of integration doesn't justify a new crate | Maintenance overhead, slower builds |
| Modify SharedLearning to store node IDs | Would couple learning schema to graph internals | Learning markdown files shouldn't contain graph IDs |

### Simplicity Check

**What if this could be easy?**

The simplest design: When a learning is saved, create a Document from it. Look up its keywords in the thesaurus to get node IDs. Add the Document to RoleGraph like any other document. When the learning is applied, boost its Document's rank. When querying the graph, learning documents appear in results naturally.

**Senior Engineer Test**: A senior engineer would say "You're just treating learnings as another document type. That's exactly right."

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimisation

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_middleware/src/learning_indexer.rs` | Step 7: Converts SharedLearnings to Documents and indexes them |
| `crates/terraphim_middleware/src/feedback_loop.rs` | Step 8: Propagates quality metrics between learnings and graph |
| `crates/terraphim_middleware/src/learning_query.rs` | Query-time merging of graph results with learning results |
| `crates/terraphim_middleware/tests/learning_kg_integration_test.rs` | Integration tests for Steps 7-8 |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_types/src/lib.rs` | Add `DocumentType::Learning`, optional `learning_id` field on Document |
| `crates/terraphim_rolegraph/src/lib.rs` | Add `index_learning()` and `get_learning_documents()` methods |
| `crates/terraphim_middleware/src/lib.rs` | Add new modules behind feature flags |
| `crates/terraphim_agent/src/shared_learning/store.rs` | Add `record_graph_query()` method for feedback loop |
| `crates/terraphim_agent/src/shared_learning/types.rs` | Add `linked_node_ids` helper (computed, not persisted) |
| `crates/terraphim_middleware/Cargo.toml` | Add `terraphim_agent` dependency (types only) |

### Deleted Files

None.

## API Design

### Public Types

```rust
/// Feature-gated in terraphim_types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg(feature = "kg-integration")]
pub enum DocumentType {
    KgEntry,
    Article,
    Learning,  // NEW
}

/// Extended Document with optional learning linkage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    // ... existing fields ...

    /// If this document represents a learning, the learning ID
    #[cfg(feature = "kg-integration")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub learning_id: Option<String>,
}

/// Configuration for learning indexing
#[derive(Debug, Clone)]
#[cfg(feature = "kg-integration")]
pub struct LearningIndexerConfig {
    /// Minimum trust level to index
    pub min_trust_level: TrustLevel,
    /// Boost factor for success_rate when calculating document rank
    pub rank_boost_factor: f64,
    /// Whether to include learning content in the document body
    pub include_content: bool,
}

impl Default for LearningIndexerConfig {
    fn default() -> Self {
        Self {
            min_trust_level: TrustLevel::L2,
            rank_boost_factor: 100.0,
            include_content: true,
        }
    }
}

/// Feedback loop configuration
#[derive(Debug, Clone)]
#[cfg(feature = "feedback-loop")]
pub struct FeedbackConfig {
    /// How much to boost document rank per successful application
    pub rank_boost_per_success: u64,
    /// How much to penalise rank per failed application
    pub rank_penalty_per_failure: u64,
    /// Whether to update learning metrics on graph query
    pub update_on_query: bool,
}

impl Default for FeedbackConfig {
    fn default() -> Self {
        Self {
            rank_boost_per_success: 10,
            rank_penalty_per_failure: 5,
            update_on_query: true,
        }
    }
}
```

### Public Functions

```rust
/// Step 7: Index a shared learning into the RoleGraph
///
/// Creates an IndexedDocument from the learning, resolves keywords to node IDs
/// via thesaurus lookup, and adds it to the graph's document index.
///
/// # Errors
/// Returns `RoleGraphError` if keyword resolution fails or document indexing fails.
#[cfg(feature = "kg-integration")]
pub fn index_learning(
    graph: &mut RoleGraph,
    learning: &SharedLearning,
    config: &LearningIndexerConfig,
) -> Result<IndexedDocument, RoleGraphError>;

/// Step 7: Batch index multiple learnings
#[cfg(feature = "kg-integration")]
pub fn index_learnings(
    graph: &mut RoleGraph,
    learnings: &[SharedLearning],
    config: &LearningIndexerConfig,
) -> Vec<Result<IndexedDocument, RoleGraphError>>;

/// Step 7: Find learnings related to a graph query
///
/// Returns learning documents whose keywords match the query terms.
#[cfg(feature = "kg-integration")]
pub fn find_learning_documents(
    graph: &RoleGraph,
    query: &str,
    limit: usize,
) -> Vec<&IndexedDocument>;

/// Step 8: Record that a learning was applied and update graph ranks
///
/// Increments learning quality metrics and boosts/panalties linked document ranks.
#[cfg(feature = "feedback-loop")]
pub async fn record_learning_application(
    graph: &mut RoleGraph,
    store: &SharedLearningStore,
    learning_id: &str,
    effective: bool,
    config: &FeedbackConfig,
) -> Result<(), FeedbackError>;

/// Step 8: Record that a graph query touched nodes linked to learnings
///
/// Increments applied_count for linked learnings (lightweight feedback).
#[cfg(feature = "feedback-loop")]
pub fn record_graph_query(
    graph: &RoleGraph,
    store: &SharedLearningStore,
    query: &str,
) -> Result<(), FeedbackError>;
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
#[cfg(feature = "kg-integration")]
pub enum LearningIndexError {
    #[error("learning trust level {got:?} below minimum {need:?}")]
    TrustLevelTooLow { got: TrustLevel, need: TrustLevel },

    #[error("no nodes found for keywords: {0:?}")]
    NoMatchingNodes(Vec<String>),

    #[error("graph error: {0}")]
    Graph(#[from] terraphim_rolegraph::Error),

    #[error("learning store error: {0}")]
    Store(String),
}

#[derive(Debug, thiserror::Error)]
#[cfg(feature = "feedback-loop")]
pub enum FeedbackError {
    #[error("learning not found: {0}")]
    LearningNotFound(String),

    #[error("document not found for learning: {0}")]
    DocumentNotFound(String),

    #[error("graph error: {0}")]
    Graph(#[from] terraphim_rolegraph::Error),

    #[error("store error: {0}")]
    Store(#[from] terraphim_agent::StoreError),
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_learning_to_document_conversion` | `learning_indexer.rs` | SharedLearning -> Document mapping |
| `test_keyword_to_node_resolution` | `learning_indexer.rs` | Thesaurus lookup for keywords |
| `test_trust_level_filtering` | `learning_indexer.rs` | L1 learnings rejected when min=L2 |
| `test_rank_boost_calculation` | `feedback_loop.rs` | Success rate -> rank mapping |
| `test_learning_not_found` | `feedback_loop.rs` | Graceful handling of missing learning |
| `test_document_not_linked` | `feedback_loop.rs` | Graceful handling of unindexed learning |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_learning_appears_in_graph_query` | `learning_kg_integration_test.rs` | End-to-end: index learning -> query -> find it |
| `test_learning_ranked_by_quality` | `learning_kg_integration_test.rs` | High-quality learning outranks low-quality |
| `test_feedback_updates_rank` | `learning_kg_integration_test.rs` | Apply learning -> rank increases |
| `test_feedback_updates_quality` | `learning_kg_integration_test.rs` | Graph query -> applied_count increases |
| `test_multiple_learnings_same_keyword` | `learning_kg_integration_test.rs` | Multiple learnings, correct merging |

### Property Tests

```rust
proptest! {
    #[test]
    fn rank_boost_never_negative(
        success_rate in 0.0f64..1.0,
        base_rank in 0u64..1000,
    ) {
        let boosted = calculate_boosted_rank(base_rank, success_rate);
        assert!(boosted >= base_rank || success_rate < 0.5);
    }

    #[test]
    fn document_roundtrip_preserves_learning_id(
        learning_id in "[a-z0-9-]{10,50}",
    ) {
        let doc = Document {
            learning_id: Some(learning_id.clone()),
            ..Document::default()
        };
        let json = serde_json::to_string(&doc).unwrap();
        let restored: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.learning_id, Some(learning_id));
    }
}
```

## Implementation Steps

### Step 1: Type Extensions (Foundation)
**Files:** `crates/terraphim_types/src/lib.rs`, `crates/terraphim_types/Cargo.toml`
**Description:** Add `DocumentType::Learning` and optional `learning_id` to `Document`. Behind `kg-integration` feature flag.
**Tests:** Serialization roundtrip tests
**Dependencies:** None
**Estimated:** 0.5 day

```rust
// Key code to write
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentType {
    KgEntry,
    Article,
    #[cfg(feature = "kg-integration")]
    Learning,
}

pub struct Document {
    // ... existing fields ...
    #[cfg(feature = "kg-integration")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub learning_id: Option<String>,
}
```

### Step 2: RoleGraph Extension
**Files:** `crates/terraphim_rolegraph/src/lib.rs`
**Description:** Add `index_learning_document()` and `get_learning_documents()` methods. Learning documents stored alongside regular documents.
**Tests:** Unit tests for indexing and retrieval
**Dependencies:** Step 1
**Estimated:** 1 day

```rust
// Key code to write
impl RoleGraph {
    #[cfg(feature = "kg-integration")]
    pub fn index_learning_document(
        &mut self,
        doc: IndexedDocument,
    ) -> Result<()> {
        self.documents.insert(doc.document.id.clone(), doc);
        Ok(())
    }

    #[cfg(feature = "kg-integration")]
    pub fn get_learning_documents(
        &self,
        query: &str,
    ) -> Vec<&IndexedDocument> {
        // Find matching node IDs, then filter documents with learning_id
    }
}
```

### Step 3: Learning Indexer (Step 7 Core)
**Files:** `crates/terraphim_middleware/src/learning_indexer.rs`
**Description:** Convert SharedLearning to Document, resolve keywords to node IDs, index in RoleGraph.
**Tests:** Unit tests for conversion, keyword resolution, trust filtering
**Dependencies:** Steps 1-2
**Estimated:** 1 day

```rust
// Key code to write
pub fn index_learning(
    graph: &mut RoleGraph,
    learning: &SharedLearning,
    config: &LearningIndexerConfig,
) -> Result<IndexedDocument, LearningIndexError> {
    if learning.trust_level < config.min_trust_level {
        return Err(LearningIndexError::TrustLevelTooLow { ... });
    }

    // Resolve keywords to node IDs via thesaurus
    let node_ids = resolve_keywords(graph, &learning.keywords)?;

    // Create Document
    let doc = Document {
        id: learning.id.clone(),
        title: learning.title.clone(),
        body: if config.include_content { learning.content.clone() } else { String::new() },
        doc_type: DocumentType::Learning,
        rank: calculate_initial_rank(&learning.quality, config.rank_boost_factor),
        tags: Some(learning.keywords.clone()),
        learning_id: Some(learning.id.clone()),
        ..Default::default()
    };

    // Create IndexedDocument with node links
    let indexed = IndexedDocument {
        document: doc,
        node_ids,
        edge_ids: Vec::new(),
        rank: doc.rank.unwrap_or(1),
    };

    graph.index_learning_document(indexed.clone())?;
    Ok(indexed)
}
```

### Step 4: Feedback Loop (Step 8 Core)
**Files:** `crates/terraphim_middleware/src/feedback_loop.rs`
**Description:** Bidirectional propagation: learning application -> rank boost; graph query -> quality update.
**Tests:** Unit tests for rank updates, quality updates, edge cases
**Dependencies:** Steps 2-3
**Estimated:** 1 day

```rust
// Key code to write
pub async fn record_learning_application(
    graph: &mut RoleGraph,
    store: &SharedLearningStore,
    learning_id: &str,
    effective: bool,
    config: &FeedbackConfig,
) -> Result<(), FeedbackError> {
    // Update learning quality
    store.record_application(learning_id, effective).await?;

    // Find linked document
    let doc = find_learning_document(graph, learning_id)?;

    // Update document rank
    let adjustment = if effective {
        config.rank_boost_per_success
    } else {
        config.rank_penalty_per_failure
    };
    update_document_rank(graph, &doc.document.id, adjustment)?;

    Ok(())
}

pub fn record_graph_query(
    graph: &RoleGraph,
    store: &SharedLearningStore,
    query: &str,
) -> Result<(), FeedbackError> {
    if !config.update_on_query {
        return Ok(());
    }

    let learning_docs = graph.get_learning_documents(query);
    for doc in learning_docs {
        if let Some(ref id) = doc.document.learning_id {
            // Increment applied_count (lightweight)
            // Note: In practice this might be batched
            store.record_graph_touch(id).await?;
        }
    }

    Ok(())
}
```

### Step 5: Query Merging
**Files:** `crates/terraphim_middleware/src/learning_query.rs`
**Description:** Merge learning documents with regular graph query results. Learnings ranked by quality.
**Tests:** Integration tests for merged rankings
**Dependencies:** Steps 3-4
**Estimated:** 0.5 day

```rust
// Key code to write
pub fn query_with_learnings(
    graph: &RoleGraph,
    query: &SearchQuery,
    include_learnings: bool,
) -> Vec<RankedResult> {
    let mut results = graph.query(query);

    if include_learnings {
        let learning_docs = graph.get_learning_documents(&query.search_term);
        for doc in learning_docs {
            results.push(RankedResult::from_learning(doc));
        }
        results.sort_by(|a, b| b.rank.cmp(&a.rank));
    }

    results
}
```

### Step 6: Store Integration
**Files:** `crates/terraphim_agent/src/shared_learning/store.rs`
**Description:** Add `record_graph_touch()` method to SharedLearningStore. Batched or immediate updates.
**Tests:** Unit test for graph touch recording
**Dependencies:** None (additive)
**Estimated:** 0.5 day

```rust
// Key code to write
impl SharedLearningStore {
    pub async fn record_graph_touch(
        &self,
        learning_id: &str,
    ) -> Result<(), StoreError> {
        let mut index = self.index.write().await;
        if let Some(learning) = index.get_mut(learning_id) {
            learning.quality.applied_count += 1;
            learning.updated_at = Utc::now();
            let updated = learning.clone();
            drop(index);
            self.persist(&updated).await?;
        }
        Ok(())
    }
}
```

### Step 7: CLI Integration
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Add `learn index` subcommand to manually trigger indexing. Optional `learn feedback` for debugging.
**Tests:** CLI integration test
**Dependencies:** Steps 3-6
**Estimated:** 0.5 day

### Step 8: Integration Tests & Documentation
**Files:** `crates/terraphim_middleware/tests/learning_kg_integration_test.rs`, `.docs/`
**Description:** End-to-end tests, feature flag verification, documentation.
**Tests:** Full integration test suite
**Dependencies:** All above
**Estimated:** 0.5 day

## Rollback Plan

If issues discovered:
1. Disable `kg-integration` and `feedback-loop` features in `Cargo.toml`
2. Remove learning documents from RoleGraph via `graph.documents.retain(|_, doc| doc.document.learning_id.is_none())`
3. Revert `DocumentType::Learning` additions (optional, harmless to leave)

Feature flags:
- `kg-integration` - Enables learning indexing and query merging
- `feedback-loop` - Enables bidirectional quality/rank updates

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| None | - | Reuses existing terraphim_rolegraph, terraphim_types, terraphim_agent |

### Dependency Updates

| Crate | From | To | Reason |
|-------|------|-----|--------|
| `terraphim_middleware/Cargo.toml` | - | Add `terraphim_agent` | For SharedLearning types (types-only, no runtime dep on store) |
| `terraphim_types/Cargo.toml` | - | Add `kg-integration` feature | Gates Document changes |
| `terraphim_rolegraph/Cargo.toml` | - | Add `kg-integration` feature | Gates new methods |

### Dependency Diagram

```
terraphim_middleware
  ├── terraphim_rolegraph (existing)
  ├── terraphim_types (existing)
  └── terraphim_agent (NEW - types only)

terraphim_rolegraph
  └── terraphim_types

terraphim_agent
  └── terraphim_types
```

**No circular dependencies.** The new `terraphim_middleware` -> `terraphim_agent` dependency is for types only (SharedLearning, TrustLevel, etc.), not for the store implementation.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Learning indexing latency | < 50ms per learning | Benchmark with 100 learnings |
| Query with learnings | < 100ms total | Benchmark vs baseline query |
| Feedback update latency | < 10ms | Benchmark rank adjustment |
| Memory overhead per learning | < 5KB | Heap profiling |
| Startup indexing (1000 learnings) | < 500ms | Benchmark batch index |

### Benchmarks to Add

```rust
#[bench]
fn bench_index_learning(b: &mut Bencher) {
    let mut graph = create_test_graph();
    let learning = create_test_learning();
    b.iter(|| index_learning(&mut graph, &learning, &LearningIndexerConfig::default()));
}

#[bench]
fn bench_query_with_learnings(b: &mut Bencher) {
    let graph = create_test_graph_with_learnings(100);
    let query = SearchQuery::new("rust error handling");
    b.iter(|| query_with_learnings(&graph, &query, true));
}
```

## Migration (if applicable)

### Data Migration

No data migration required. Existing learnings in markdown files are indexed on-demand:
1. On startup (optional), or
2. On first query, or
3. Via explicit `learn index` CLI command

Learnings are **not** stored in the graph serialisation. The graph document index is rebuilt from markdown files on each startup, just like regular documents are rebuilt from their sources.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm `terraphim_middleware` can depend on `terraphim_agent` types only | Pending | Verify Cargo.toml doesn't pull in heavy deps |
| Decide on batch vs immediate feedback updates | Pending | Benchmark both approaches |
| Confirm Document serialization backward compat with `learning_id` field | Pending | Test with existing graph files |
| Should learning documents be persisted in graph serialisation? | Pending | Currently leaning toward "no" (rebuild from markdown) |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

---

## Appendix: Implementation Detail

### Keyword-to-Node Resolution

```rust
fn resolve_keywords(graph: &RoleGraph, keywords: &[String]) -> Result<Vec<u64>> {
    let mut node_ids = Vec::new();
    for keyword in keywords {
        // Use Aho-Corasick for fast matching
        let matches = graph.find_matching_node_ids(keyword);
        node_ids.extend(matches);
    }
    node_ids.sort_unstable();
    node_ids.dedup();
    Ok(node_ids)
}
```

### Rank Calculation from Quality

```rust
fn calculate_initial_rank(quality: &QualityMetrics, boost_factor: f64) -> Option<u64> {
    let base_rank = 1u64;
    let quality_boost = (quality.success_rate.unwrap_or(0.5) * boost_factor) as u64;
    let agent_boost = quality.agent_count as u64 * 10;
    Some(base_rank + quality_boost + agent_boost)
}
```

### Learning Document Example

```rust
Document {
    id: "learning-abc123".to_string(),
    title: "Cargo Clippy Lints".to_string(),
    body: "Use `cargo clippy --all-targets` to catch common mistakes...".to_string(),
    doc_type: DocumentType::Learning,
    rank: Some(85), // High due to good success_rate
    tags: Some(vec!["rust".to_string(), "clippy".to_string(), "linting".to_string()]),
    learning_id: Some("learning-abc123".to_string()),
    ..Default::default()
}
```

### IndexedDocument with Node Links

```rust
IndexedDocument {
    document: doc,
    node_ids: vec![42, 57], // "rust" -> 42, "clippy" -> 57
    edge_ids: vec![],
    rank: 85,
}
```

This design preserves the markdown-first architecture, reuses existing graph infrastructure, adds no heavy dependencies, and keeps all changes behind feature flags.
