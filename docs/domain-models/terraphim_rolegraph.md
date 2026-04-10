# terraphim_rolegraph - Knowledge Graph Implementation

## Overview

`terraphim_rolegraph` implements the knowledge graph system for Terraphim AI. It provides graph-based search, concept relationships, and semantic connectivity features. The crate uses Aho-Corasick automata for fast text matching and TF-IDF for semantic fallback search.

## Domain Model

### Core Concepts

#### RoleGraph
Per-role knowledge graph containing nodes, edges, documents, and thesaurus.

```rust
pub struct RoleGraph {
    pub role: RoleName,
    pub nodes: AHashMap<u64, Node>,
    pub edges: AHashMap<u64, Edge>,
    pub documents: AHashMap<String, IndexedDocument>,
    pub thesaurus: Thesaurus,
    pub aho_corasick_values: Vec<u64>,
    pub ac: AhoCorasick,
    pub ac_reverse_nterm: AHashMap<u64, NormalisedTermValue>,
    pub trigger_index: TriggerIndex,
    pub pinned_node_ids: Vec<u64>,
}
```

**Key Responsibilities:**
- Store knowledge graph structure
- Manage document-index relationships
- Provide text matching via Aho-Corasick
- Enable semantic search via TF-IDF
- Track pinned (always-included) nodes

#### Node
Concept entity in the knowledge graph.

```rust
pub struct Node {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub url: Option<String>,
    pub connected_with: Vec<u64>,
}
```

**Key Responsibilities:**
- Represent abstract concepts
- Store metadata and descriptions
- Link to external resources
- Maintain adjacency lists

#### Edge
Relationship between two nodes in the knowledge graph.

```rust
pub struct Edge {
    pub id: u64,
    pub from_node_id: u64,
    pub to_node_id: u64,
    pub relationship: String,
}
```

**Key Responsibilities:**
- Define concept relationships
- Enable graph traversal
- Support relationship types

#### IndexedDocument
Document with search indexes and concept links.

```rust
pub struct IndexedDocument {
    pub document: Document,
    pub index: Index,
    pub connected_node_ids: Vec<u64>,
}
```

**Key Responsibilities:**
- Link documents to concepts
- Store search indexes
- Enable semantic document retrieval

### Text Matching

#### TriggerIndex
TF-IDF index over trigger descriptions for semantic fallback search.

```rust
pub struct TriggerIndex {
    pub triggers: AHashMap<u64, Vec<String>>,
    pub idf: AHashMap<String, f64>,
    pub doc_count: usize,
    pub threshold: f64,
    pub custom_stopwords: Option<ahash::AHashSet<String>>,
}
```

**Key Responsibilities:**
- Provide semantic search fallback
- Use TF-IDF similarity scoring
- Support custom stopwords
- Configure relevance thresholds

**Default Threshold:**
```rust
pub const DEFAULT_TRIGGER_THRESHOLD: f64 = 0.3;
```

## Data Models

### Graph Statistics

#### GraphStats
Statistics about graph structure for debugging and monitoring.

```rust
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub document_count: usize,
    pub thesaurus_size: usize,
    pub is_populated: bool,
}
```

**Use Cases:**
- Monitoring graph health
- Debugging graph issues
- Performance analysis
- Capacity planning

### Serialisation

#### SerializableRoleGraph
Serializable representation of RoleGraph for JSON storage.

```rust
pub struct SerializableRoleGraph {
    pub role: RoleName,
    pub nodes: AHashMap<u64, Node>,
    pub edges: AHashMap<u64, Edge>,
    pub documents: AHashMap<String, IndexedDocument>,
    pub thesaurus: Thesaurus,
    pub aho_corasick_values: Vec<u64>,
    pub ac_reverse_nterm: AHashMap<u64, NormalisedTermValue>,
    pub trigger_descriptions: AHashMap<u64, String>,
    pub pinned_node_ids: Vec<u64>,
}
```

**Use Cases:**
- Persist graph to disk
- Transfer between processes
- Cache graph state
- Enable graph reconstruction

## Implementation Patterns

### Graph Construction

#### Initialisation
```rust
impl RoleGraph {
    pub async fn new(role: RoleName, thesaurus: Thesaurus) -> Result<Self> {
        Self::new_sync(role, thesaurus)
    }

    pub fn new_sync(role: RoleName, thesaurus: Thesaurus) -> Result<Self> {
        let (ac, aho_corasick_values, ac_reverse_nterm) =
            Self::build_aho_corasick(&thesaurus)?;

        Ok(Self {
            role,
            nodes: AHashMap::new(),
            edges: AHashMap::new(),
            documents: AHashMap::new(),
            thesaurus,
            aho_corasick_values,
            ac,
            ac_reverse_nterm,
            trigger_index: TriggerIndex::new(DEFAULT_TRIGGER_THRESHOLD),
            pinned_node_ids: Vec::new(),
        })
    }
}
```

**Pattern:**
- Build Aho-Corasick from thesaurus
- Initialise empty collections
- Set default threshold
- Support both sync and async API

#### Aho-Corasick Building
```rust
impl RoleGraph {
    fn build_aho_corasick(
        thesaurus: &Thesaurus,
    ) -> Result<(AhoCorasick, Vec<u64>, AHashMap<u64, NormalisedTermValue>)> {
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut ac_reverse_nterm = AHashMap::new();

        for (key, normalised_term) in thesaurus {
            keys.push(key.as_str());
            values.push(normalised_term.id);
            ac_reverse_nterm.insert(normalised_term.id, normalised_term.value.clone());
        }

        let ac = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .ascii_case_insensitive(true)
            .build(keys)?;

        Ok((ac, values, ac_reverse_nterm))
    }
}
```

**Pattern:**
- Extract keys and values from thesaurus
- Build reverse lookup map
- Configure case-insensitive matching
- Use leftmost-longest matching strategy

### Graph Operations

#### Document Indexing
```rust
impl RoleGraph {
    pub fn index_document(&mut self, document: Document) -> Result<()> {
        let index = Index::from_document(&document)?;
        let mut connected_node_ids = Vec::new();

        // Find matching nodes via Aho-Corasick
        let matches = self.find_matching_node_ids(&document.body);

        // Create indexed document with connections
        let indexed_doc = IndexedDocument {
            document,
            index,
            connected_node_ids,
        };

        self.documents.insert(indexed_doc.document.id.clone(), indexed_doc);
        Ok(())
    }
}
```

**Pattern:**
- Create search index
- Find matching concepts
- Link document to concepts
- Store in documents map

#### Text Matching
```rust
impl RoleGraph {
    pub fn find_matching_node_ids(&self, text: &str) -> Vec<u64> {
        log::trace!("Finding matching node IDs for text: '{}'", text);
        self.ac
            .find_iter(text)
            .map(|mat| self.aho_corasick_values[mat.pattern()])
            .collect()
    }
}
```

**Pattern:**
- Use Aho-Corasick for fast matching
- Return node IDs only (not full nodes)
- Trace-level logging for debugging
- Efficient iteration

#### Semantic Fallback
```rust
impl RoleGraph {
    pub fn find_matching_node_ids_with_fallback(
        &self,
        text: &str,
        include_pinned: bool,
    ) -> Vec<u64> {
        let mut results = self.find_matching_node_ids(text);

        // Pass 2: TF-IDF fallback when Aho-Corasick found nothing
        if results.is_empty() && !self.trigger_index.is_empty() {
            let trigger_matches = self.trigger_index.query(text);
            results.extend(trigger_matches.iter().map(|(id, _score)| *id));
        }

        // Always include pinned entries
        if include_pinned {
            for pinned_id in &self.pinned_node_ids {
                if !results.contains(pinned_id) {
                    results.push(*pinned_id);
                }
            }
        }

        results
    }
}
```

**Pattern:**
- Two-pass search strategy
- Exact match first
- Semantic fallback second
- Always include pinned nodes
- Deduplicate results

### Graph Connectivity

#### Path Checking
```rust
impl RoleGraph {
    pub fn is_all_terms_connected_by_path(&self, text: &str) -> bool {
        let mut targets = self.find_matching_node_ids(text);
        targets.sort_unstable();
        targets.dedup();
        let k = targets.len();

        if k <= 1 {
            return true;
        }

        // Build adjacency map
        let mut adj: AHashMap<u64, ahash::AHashSet<u64>> = AHashMap::new();
        for node_id in &targets {
            if let Some(node) = self.nodes.get(node_id) {
                for neighbor_id in &node.connected_with {
                    if targets.contains(neighbor_id) {
                        adj.entry(*node_id)
                            .or_insert_with(ahash::AHashSet::new)
                            .insert(*neighbor_id);
                    }
                }
            }
        }

        // DFS/backtracking to find path visiting all targets
        Self::find_path_visiting_all(&adj, &targets)
    }
}
```

**Pattern:**
- Find all matching nodes
- Build adjacency map from targets
- Use DFS/backtracking for path finding
- Early exit for trivial cases (k <= 1)

### Trigger Index

#### TF-IDF Indexing
```rust
impl TriggerIndex {
    pub fn build(&mut self, triggers: AHashMap<u64, String>) {
        self.triggers.clear();
        self.idf.clear();
        self.doc_count = triggers.len();

        // Tokenise each trigger
        let mut doc_freq: AHashMap<String, usize> = AHashMap::new();
        for (node_id, trigger_text) in &triggers {
            let tokens: Vec<String> = self.tokenise(trigger_text);
            let unique: ahash::AHashSet<&str> =
                tokens.iter().map(|s| s.as_str()).collect();
            for token in &unique {
                *doc_freq.entry(token.to_string()).or_insert(0) += 1;
            }
            self.triggers.insert(*node_id, tokens);
        }

        // Compute IDF: log((N + 1) / (df + 1)) + 1 (smoothed)
        let n = self.doc_count as f64;
        for (token, df) in &doc_freq {
            let idf = ((n + 1.0) / (*df as f64 + 1.0)).ln() + 1.0;
            self.idf.insert(token.clone(), idf);
        }
    }
}
```

**Pattern:**
- Tokenise triggers
- Count document frequency
- Compute inverse document frequency
- Use smoothing to avoid zero division

#### Query Execution
```rust
impl TriggerIndex {
    pub fn query(&self, text: &str) -> Vec<(u64, f64)> {
        if self.triggers.is_empty() {
            return vec![];
        }

        let query_tokens = self.tokenise(text);
        if query_tokens.is_empty() {
            return vec![];
        }

        // Compute TF-IDF vectors
        let mut query_tfidf: AHashMap<&str, f64> = AHashMap::new();
        for token in &query_tokens {
            let tf = 1.0; // Binary TF for query
            let idf = self.idf.get(token.as_str()).copied().unwrap_or(0.0);
            *query_tfidf.entry(token.as_str()).or_insert(0.0) += tf * idf;
        }

        // Cosine similarity computation
        let query_norm: f64 = query_tfidf.values()
            .map(|v| v * v).sum::<f64>().sqrt();

        let mut results: Vec<(u64, f64)> = Vec::new();
        for (node_id, trigger_tokens) in &self.triggers {
            let mut doc_tfidf: AHashMap<&str, f64> = AHashMap::new();
            for token in trigger_tokens {
                let tf = 1.0;
                let idf = self.idf.get(token.as_str()).copied().unwrap_or(0.0);
                *doc_tfidf.entry(token.as_str()).or_insert(0.0) += tf * idf;
            }

            let doc_norm: f64 = doc_tfidf.values()
                .map(|v| v * v).sum::<f64>().sqrt();

            // Dot product and cosine similarity
            let dot: f64 = query_tfidf.iter()
                .map(|(token, q_val)| {
                    let d_val = doc_tfidf.get(token).copied().unwrap_or(0.0);
                    q_val * d_val
                })
                .sum();

            let similarity = dot / (query_norm * doc_norm);
            if similarity >= self.threshold {
                results.push((*node_id, similarity));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}
```

**Pattern:**
- Tokenise query text
- Compute TF-IDF vectors
- Use cosine similarity
- Filter by threshold
- Sort by relevance

## Serialisation

#### Graph Export
```rust
impl RoleGraph {
    pub fn to_serializable(&self) -> SerializableRoleGraph {
        SerializableRoleGraph {
            role: self.role.clone(),
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            documents: self.documents.clone(),
            thesaurus: self.thesaurus.clone(),
            aho_corasick_values: self.aho_corasick_values.clone(),
            ac_reverse_nterm: self.ac_reverse_nterm.clone(),
            trigger_descriptions: self.trigger_index.get_trigger_descriptions(),
            pinned_node_ids: self.pinned_node_ids.clone(),
        }
    }
}
```

**Pattern:**
- Clone all serialisable data
- Exclude non-serialisable automata
- Include trigger descriptions
- Preserve pinned node IDs

#### Graph Import
```rust
impl RoleGraph {
    pub fn from_serializable_sync(serializable: SerializableRoleGraph) -> Result<Self> {
        // Build trigger index from serialized descriptions
        let mut trigger_index = TriggerIndex::new(0.3);
        trigger_index.build(serializable.trigger_descriptions.clone());

        let mut role_graph = RoleGraph {
            role: serializable.role,
            nodes: serializable.nodes,
            edges: serializable.edges,
            documents: serializable.documents,
            thesaurus: serializable.thesaurus,
            aho_corasick_values: serializable.aho_corasick_values,
            ac: AhoCorasick::new([""])?, // Will be rebuilt
            ac_reverse_nterm: serializable.ac_reverse_nterm,
            trigger_index,
            pinned_node_ids: serializable.pinned_node_ids,
        };

        // Rebuild Aho-Corasick automata
        role_graph.rebuild_automata()?;

        Ok(role_graph)
    }
}
```

**Pattern:**
- Reconstruct trigger index
- Create empty automata (to be rebuilt)
- Copy all serialisable fields
- Rebuild automata for functionality

## Performance Optimisations

### Aho-Corasick Configuration

#### Matching Strategy
```rust
let ac = AhoCorasick::builder()
    .match_kind(MatchKind::LeftmostLongest)
    .ascii_case_insensitive(true)
    .build(keys)?;
```

**Strategy:**
- `LeftmostLongest`: Prefer longer, earlier matches
- `ascii_case_insensitive`: Case-insensitive matching
- Balances precision and recall

### Stopword Filtering

#### Default Stopwords
```rust
impl TriggerIndex {
    pub fn is_default_stopword(word: &str) -> bool {
        matches!(
            word,
            "the" | "and" | "for" | "are" | "but" | "not" |
            "you" | "all" | "can" | "her" | "was" | "one" |
            "our" | "out" | "has" | "have" | "been" |
            "this" | "that" | "with" | "when" | "from" |
            "into" | "which" | "their" | "will"
        )
    }
}
```

**Pattern:**
- Filter common words
- Reduce noise
- Improve relevance
- Customisable stopword lists

## Error Handling

### Error Types

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The given node ID was not found")]
    NodeIdNotFound,

    #[error("The given Edge ID was not found")]
    EdgeIdNotFound,

    #[error("Cannot convert IndexedDocument to JSON: {0}")]
    JsonConversionError(#[from] serde_json::Error),

    #[error("Error while driving terraphim automata: {0}")]
    TerraphimAutomataError(#[from] terraphim_automata::TerraphimAutomataError),

    #[error("Indexing error: {0}")]
    AhoCorasickError(#[from] aho_corasick::BuildError),
}
```

**Categories:**
- **Data**: Node/edge not found
- **Serialisation**: JSON conversion errors
- **Integration**: Automata errors
- **Indexing**: Aho-Corasick build errors

## Testing Patterns

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_initialisation() {
        let mut thesaurus = Thesaurus::new("test".to_string());
        thesaurus.insert(
            NormalisedTermValue::from("rust"),
            NormalisedTerm::new(1, NormalisedTermValue::from("rust"))
        );

        let graph = RoleGraph::new_sync(
            RoleName::new("test"),
            thesaurus
        ).unwrap();

        assert_eq!(graph.nodes.len(), 0);
        assert!(!graph.thesaurus.is_empty());
    }

    #[test]
    fn test_text_matching() {
        let mut thesaurus = Thesaurus::new("test".to_string());
        thesaurus.insert(
            NormalisedTermValue::from("programming"),
            NormalisedTerm::new(1, NormalisedTermValue::from("programming"))
        );

        let graph = RoleGraph::new_sync(
            RoleName::new("test"),
            thesaurus
        ).unwrap();

        let matches = graph.find_matching_node_ids("I love programming!");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_trigger_index() {
        let mut index = TriggerIndex::new(0.3);

        let mut triggers = AHashMap::new();
        triggers.insert(1, "rust programming language".to_string());
        triggers.insert(2, "python scripting".to_string());

        index.build(triggers);

        let results = index.query("programming scripts");
        assert!(results.len() >= 1);
    }
}
```

## Best Practices

### Graph Design

- Use unique IDs consistently
- Maintain adjacency relationships
- Support bidirectional traversal
- Validate graph consistency

### Text Matching

- Use Aho-Corasick for exact matches
- Provide semantic fallback
- Configure appropriate thresholds
- Handle edge cases gracefully

### Performance

- Cache computed results
- Minimise allocations
- Use efficient data structures
- Profile critical paths

### Serialisation

- Exclude non-serialisable data
- Rebuild automata on import
- Preserve all metadata
- Validate on import

## Future Enhancements

### Planned Features

#### Graph Visualisation
```rust
impl RoleGraph {
    pub fn to_dot_format(&self) -> String {
        // Generate Graphviz DOT format
    }

    pub fn to_mermaid(&self) -> String {
        // Generate Mermaid diagram
    }
}
```

#### Graph Analytics
```rust
impl RoleGraph {
    pub fn compute_centrality(&self) -> AHashMap<u64, f64> {
        // Compute node centrality measures
    }

    pub fn find_communities(&self) -> Vec<Vec<u64>> {
        // Detect graph communities
    }
}
```

#### Incremental Updates
```rust
impl RoleGraph {
    pub fn add_node(&mut self, node: Node) -> Result<()> {
        // Add node without rebuilding entire graph
    }

    pub fn remove_document(&mut self, doc_id: &str) -> Result<()> {
        // Remove document and update indexes
    }
}
```

## References

- [Aho-Corasick algorithm](https://github.com/BurntSushi/aho-corasick)
- [TF-IDF explained](https://en.wikipedia.org/wiki/Tf%E2%80%93idf)
- [ahash for fast hash maps](https://github.com/tkaitch/ahash)
- [ThisError for error handling](https://docs.rs/thiserror/)
