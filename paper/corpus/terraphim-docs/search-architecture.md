# Terraphim Search Architecture

This document describes the complete search architecture of Terraphim AI -- a multi-layered, offline-first retrieval system combining knowledge graph traversal, statistical ranking, and symbolic similarity. The architecture is deliberately non-neural: no dense vector embeddings, no LLM inference at query time, deterministic and reproducible results, ~15-20 MB RAM footprint.

## Design Principles

| Principle | Implication |
|-----------|-------------|
| **Offline-first** | No network calls or LLM inference required at search time |
| **Deterministic** | Same query + same corpus = same results, always |
| **Explainable** | Every score is decomposable into frequency counts, field weights, or set overlaps |
| **Low footprint** | ~15-20 MB RAM for a typical knowledge graph; no GPU, no float vectors |
| **Graph-native** | Explicit edges and nodes encode domain relationships, not latent geometry |

## Pipeline Overview

```
Query String
    |
    v
[1. Lexical Extraction] -- Aho-Corasick multi-pattern matching (O(n) in text length)
    |
    v
[2. Graph Traversal]    -- RoleGraph: matched nodes -> edges -> documents
    |                       Accumulates: node.rank + edge.rank + doc.rank
    v
[3. Statistical Ranking] -- BM25 / BM25F / BM25+ / TFIDF / Jaccard / QueryRatio
    |                        Pluggable scorer selected per query
    v
[4. Similarity Re-ranking] -- Optional: Levenshtein, Jaro, Jaro-Winkler
    |                          Applied post-hoc for fuzzy matching
    v
[5. Symbolic Embeddings]  -- Optional (medical feature): IS-A hierarchy
    |                        Jaccard (70%) + path distance (30%)
    v
Ranked Results
```

### Stage 1: Lexical Extraction (`terraphim_rolegraph`)

- **Algorithm**: Aho-Corasick finite state automaton
- **Crate**: `aho-corasick = "1.0.2"` (via `terraphim_automata`)
- **Behaviour**: Case-insensitive, leftmost-longest matching against all thesaurus terms simultaneously
- **Method**: `RoleGraph::find_matching_node_ids(&text)` returns matched node IDs in sequence
- **Complexity**: O(n) in input text length regardless of number of patterns
- **Thesaurus expansion**: Synonyms map to normalised terms before automata construction

### Stage 2: Graph Traversal (`terraphim_rolegraph`)

The `RoleGraph` maintains three data structures:

| Structure | Type | Purpose |
|-----------|------|---------|
| `nodes` | `AHashMap<u64, Node>` | Concepts with frequency rank and connected edge list |
| `edges` | `AHashMap<u64, Edge>` | Co-occurrence pairs with document frequency map |
| `documents` | `AHashMap<String, IndexedDocument>` | Indexed documents with aggregate rank |

**Indexing** (document insertion):
1. Parse document, split sentences, run Aho-Corasick
2. For each consecutive matched pair (x, y): `edge.rank += 1`, `node.rank += 1`, `doc.rank += 1`
3. Ranks are integer frequency counts -- monotonic, deterministic

**Querying** (`query_graph`):
1. Extract matched node IDs from query via Aho-Corasick
2. For each matched node, traverse connected edges
3. Collect documents from edge `doc_hash`
4. Score: `total_rank = node.rank + edge.rank + document.rank`
5. Deduplicate, sort descending, apply offset/limit pagination

**Operators** (`query_graph_with_operators`):
- **OR**: Union of document sets, ranks accumulate from multiple term matches
- **AND**: Intersection -- only documents matching ALL search terms returned

**Path connectivity** (`is_all_terms_connected_by_path`):
- DFS with backtracking checks if all matched terms form a connected subgraph
- Useful for validating query coherence (k <= 8 terms, exponential worst case)

**Hybrid TF-IDF re-scoring** (implemented):
- For `TerraphimGraph` relevance function, documents are re-scored: 70% graph rank + 30% TF-IDF
- Addresses tie-breaking and long-tail term discrimination

### Stage 3: Statistical Ranking (`terraphim_service/src/score/`)

Six pluggable scorers, selected per query via `QueryScorer` enum:

| Scorer | File | Algorithm | When to use |
|--------|------|-----------|-------------|
| **OkapiBM25** | `bm25_additional.rs` | Standard BM25: `idf * (tf * (k1+1)) / (k1 * length_norm + tf)` | General-purpose ranking |
| **BM25F** | `bm25.rs` | Multi-field: `weighted_tf = w_title*tf_title + w_body*tf_body + ...` | Structured documents with title/body/tags |
| **BM25+** | `bm25.rs` | BM25 + delta: `... + delta` | Fine-grained parameter tuning |
| **TFIDF** | `bm25_additional.rs` | `tf * ln(N / n_t)` | Simple, interpretable, no length bias |
| **Jaccard** | `bm25_additional.rs` | `|A intersect B| / |A union B|` (ngram sets) | Term overlap measurement |
| **QueryRatio** | `bm25_additional.rs` | `|matched_terms| / |query_terms|` | Percentage of query terms present |

**BM25F field weights** (defaults):
- Title: 2.0, Body: 1.0, Description: 1.5, Tags: 0.5

**Parameters**: k1 = 1.2, b = 0.75, delta = 1.0 (BM25+ only)

**Processing flow**:
1. `sort_documents()` dispatches to selected scorer
2. Scorer initialises corpus statistics (IDF, average document length)
3. Each document scored against query terms
4. Results sorted by descending score

### Stage 4: Similarity Re-ranking (`terraphim_service/src/score/`)

Optional post-processing layer using string similarity metrics:

| Metric | Implementation | Use case |
|--------|---------------|----------|
| **Levenshtein** | `strsim` crate | Edit distance for typo tolerance |
| **Jaro** | `strsim` crate | Positional character similarity |
| **Jaro-Winkler** | `strsim` crate | Jaro with prefix bonus (good for short terms) |

These are applied as re-rankers on top of BM25/graph scores, not as primary scorers.

### Stage 5: Symbolic Embeddings (`terraphim_rolegraph/src/symbolic_embeddings.rs`)

Feature-gated (`medical` feature). Encodes node position in IS-A concept hierarchy:

**Embedding structure** (`SymbolicEmbedding`):
- Ancestor set (transitive closure of IS-A parents)
- Descendant set (transitive closure of IS-A children)
- Depth in hierarchy
- Semantic type classification

**Similarity computation**:
```
similarity(a, b) = 0.7 * jaccard(ancestors_a union descendants_a,
                                  ancestors_b union descendants_b)
                 + 0.3 * path_distance_score(a, b)
```

Where `path_distance_score = 1.0 / (1.0 + estimated_path_length)` via LCA depth estimation.

**Methods**:
- `nearest_neighbors(node_id, k)` -- k-NN retrieval by descending score
- `nearest_neighbors_by_type(node_id, k, semantic_type)` -- type-filtered k-NN
- `build_from_hierarchy()` -- computes transitive closure via DFS

**Key property**: No float vectors, no neural inference. Similarity is computed from explicit graph structure -- ancestors, descendants, and path length.

## Relevance Functions (Integration Points)

The system exposes three top-level relevance functions:

| Function | Pipeline stages used | Configured via |
|----------|---------------------|----------------|
| `TitleScorer` | Stage 1 (AC matching) only | `relevance_function: "title-scorer"` |
| `BM25` family | Stage 1 + Stage 3 | `relevance_function: "bm25-scorer"` |
| `TerraphimGraph` | Stage 1 + Stage 2 + Stage 3 (hybrid 70/30) | `relevance_function: "terraphim-graph"` |

Each haystack data source can use a different relevance function.

## Session Search (`terraphim_sessions`)

The `terraphim-agent sessions search` command provides cross-source session retrieval:

- **Sources**: Claude Code, Cursor, Aider, OpenCode, Codex
- **Indexing**: Tantivy full-text search engine
- **Enrichment**: Optional Aho-Corasick concept matching from knowledge graph
- **Import**: `terraphim-agent sessions import` (manual, batch)
- **Search**: `terraphim-agent sessions search "query"` (keyword-based via Tantivy)

## Performance Characteristics

| Operation | Complexity | Typical latency |
|-----------|-----------|-----------------|
| Aho-Corasick matching | O(n) in text length | Sub-millisecond |
| Graph query (single term) | O(e * d) per matched node | < 1 ms |
| BM25 scoring (1000 docs) | O(n * q) | < 5 ms |
| Symbolic embedding similarity | O(1) cached / O(ancestors) uncached | < 1 ms |
| Full pipeline (graph + BM25) | Sum of above | < 10 ms typical |

Memory: ~100 bytes/node + ~200 bytes/edge. A 10,000-node graph uses ~3 MB.
