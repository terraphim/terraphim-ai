# Summary: terraphim_rolegraph/src/lib.rs

**Purpose:** Per-role knowledge graph implementation for semantic search over indexed documents.

**Architecture:**
```
Thesaurus (JSON)
    |
    v
TriggerIndex (Aho-Corasick patterns)
    |
    v
RoleGraph (nodes + edges + document map)
    |
    v
Ranked search results
```

**Key Types:**
- **`RoleGraph`**: Main graph structure with nodes, edges, documents, thesaurus
- **`TriggerIndex`**: TF-IDF fallback index over trigger descriptions
- **`SerializableRoleGraph`**: JSON-serializable representation (excludes AC automata)
- **`RoleGraphSync`**: Thread-safe wrapper (Arc<Mutex<RoleGraph>>)

**TriggerIndex:**
- Simple TF-IDF index for semantic fallback when Aho-Corasick finds no exact matches
- Configurable relevance threshold (default 0.3)
- Default stopwords: "the", "and", "for", "are", etc.
- Cosine similarity scoring

**Aho-Corasick Configuration:**
- `MatchKind::LeftmostLongest`: Best match for overlapping patterns
- `ascii_case_insensitive`: Case-insensitive matching

**Graph Operations:**
- `find_matching_node_ids()`: O(n) concept detection over document text
- `query_graph()`: Single-term ranked search
- `query_graph_with_trigger_fallback()`: Two-pass search (AC first, TF-IDF fallback)
- `query_graph_with_operators()`: Multi-term AND/OR queries
- `is_all_terms_connected_by_path()`: Path connectivity check for matched terms

**Ranking:**
- Weighted mean average of node rank, edge rank, and document rank
- Documents sorted by descending rank
- Offset/limit pagination support

**Node/Edge Structure:**
- Nodes: id, connected_with (HashSet<edge_id>), rank
- Edges: id, doc_hash (document_id -> occurrence count)
- Magic pairing: `magic_pair(x, y)` and `magic_unpair(id)` for bidirectional edges

**Document Indexing:**
- `insert_document()`: Indexes document and creates co-occurrence edges
- `has_document()`: Check if document already indexed
- `find_document_ids_for_term()`: Reverse lookup by term

**Serialization:**
- Excludes Aho-Corasick automata (must be rebuilt on deserialization)
- Includes: nodes, edges, documents, thesaurus, aho_corasick_values, trigger_descriptions
- `to_serializable()` / `from_serializable_sync()` for persistence

**Medical Feature Gates:**
- `medical`: SNOMED CT and UMLS medical entity extraction
- `symbolic_embeddings`: Symbolic embeddings for medical concepts