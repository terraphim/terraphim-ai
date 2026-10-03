# Terraphim Graph Embeddings: Learning Agent Guide

*Model: MiniMax-M2.5 | Version: 1.6.0*

## How Knowledge Graph Terms Improve Document Retrieval

Terraphim uses a unique approach to semantic search that combines knowledge graphs with graph-based ranking. Unlike traditional vector embeddings that represent documents as points in a high-dimensional space, Terraphim builds a **graph structure** where concepts are connected through their co-occurrence in documents.

---

## Understanding Graph Embeddings

### What Are Graph Embeddings?

Graph embeddings in Terraphim are not numerical vectors. Instead, they represent the **topological structure** of a knowledge graph:

```
Document: "The CAP theorem and Raft consensus are related"

Terms extracted: "cap theorem", "raft consensus", "related"

Graph structure created:
    [cap theorem] ----(edge)---- [raft consensus]
         |                           |
         +---------(edge)----------+
                    |
              [related concept]
```

### How They're Built

When you index documents into Terraphim:

1. **Term Matching**: The Aho-Corasick automaton finds all known terms from the thesaurus in each document
2. **Node Creation**: Each unique term becomes a node in the graph
3. **Edge Creation**: When two terms appear in the same document, an edge is created between them
4. **Rank Calculation**: Nodes and edges accumulate ranks based on frequency

```rust
// Simplified view of indexing
for document in documents {
    let terms = find_matching_terms(&document.body);
    for (term_a, term_b) in terms.iter().tuple_windows() {
        // Create edge between co-occurring terms
        add_or_update_edge(term_a, term_b, &document.id);
        // Update node ranks
        increment_node_rank(term_a);
        increment_node_rank(term_b);
    }
}
```

---

## The Ranking Formula

When searching, Terraphim calculates relevance using:

```
total_rank = node.rank + edge.rank + document_rank
```

### Breaking It Down

| Component | Description | How It's Calculated |
|-----------|-------------|---------------------|
| `node.rank` | Term importance | Number of documents containing the term |
| `edge.rank` | Term relationship strength | Number of documents where connected terms co-occur |
| `document_rank` | Term frequency | How often the term appears in a specific document |

---

## Creating a Learning Agent with Knowledge Graph

### Step 1: Define Your Agent's Knowledge Domain

```rust
// Create a thesaurus with your domain-specific terms
let mut thesaurus = Thesaurus::new("Learning Assistant".to_string());

// Add core concepts
thesaurus.insert(
    NormalizedTermValue::new("machine learning".to_string()),
    NormalizedTerm::new(1, NormalizedTermValue::new("machine learning".to_string()))
);

// Add synonyms
thesaurus.insert(
    NormalizedTermValue::new("ml".to_string()),
    NormalizedTerm::new(1, NormalizedTermValue::new("machine learning".to_string()))
);
```

### Step 2: Build the RoleGraph

```rust
let role_name = RoleName::new("Learning Assistant");
let mut rolegraph = RoleGraph::new(role_name, thesaurus).await?;

// Index your learning documents
for doc in learning_documents {
    rolegraph.insert_document(&doc.id, doc);
}
```

### Step 3: Query with Graph Ranking

```rust
// Search for relevant learnings
let results = rolegraph.query_graph(
    "consensus algorithms",
    Some(0),  // offset
    Some(10)  // limit
)?;
```

---

## How Adding Knowledge Graph Terms Improves Retrieval

### Before Enhancement

With only generic terms:

| Query | Results |
|-------|---------|
| "raft consensus" | No results (term not in thesaurus) |
| "cap theorem" | No results (term not in thesaurus) |
| "database sharding" | No results (term not in thesaurus) |

### After Enhancement

Adding domain-specific terms with synonyms:

| New Term | Synonyms | Result |
|----------|----------|--------|
| "cap theorem" | "consistency", "availability", "partition tolerance" | Found! |
| "raft consensus" | "leader election", "log replication", "raft" | Found! |
| "database sharding" | "horizontal partitioning", "sharding" | Found! |

---

## Live Demo Output

```
Query: 'raft leader election'

Initial thesaurus: No results (terms not in thesaurus)
Enhanced thesaurus: 1. doc_raft (rank: 124)

  -> Found 1 MORE documents with enhanced thesaurus!
```

---

## Practical Example: Learning Assistant

### Complete Code

See `crates/terraphim_rolegraph/examples/terraphim_graph_embeddings_learnings.rs` for the full working example.

### Key Takeaways

1. **Graph embeddings are built from co-occurrence** - Every time terms appear together in a document, they're connected in the graph

2. **Adding domain-specific terms unlocks retrieval** - Without "raft" or "cap theorem" in the thesaurus, searches for those terms return nothing

3. **The ranking formula surfaces relevant docs** - Documents connecting more high-ranking terms get higher scores

4. **Graph connectivity indicates semantic coherence** - When query terms are connected in the graph, the query has high semantic meaning

---

## Configuration: Defining Terms in Markdown

You can define knowledge graph terms in Markdown files:

```markdown
# CAP Theorem

The CAP theorem states that distributed systems can only
guarantee two of: Consistency, Availability, Partition tolerance.

synonyms:: consistency, availability, partition tolerance, cap

Type: Concept
Domain: Distributed Systems
Related: raft, paxos, eventual consistency
```

Terraphim automatically builds the thesaurus from these files.

---

## Comparison with Vector Embeddings

| Aspect | Vector Embeddings | Terraphim Graph Embeddings |
|--------|-------------------|---------------------------|
| Representation | Numerical vectors | Graph topology |
| Similarity | Cosine/distance | Graph connectivity |
| Interpretability | Low (dense vectors) | High (explicit relationships) |
| Updates | Retrain required | Incremental updates |
| Privacy | May leak to server | Fully local |
| Domain adaptation | Fine-tuning | Add terms to thesaurus |

---

## Learning via Negativa

*An advanced technique for learning from failed commands*

One of the most powerful features of Terraphim is the ability to learn from mistakes. When a command fails or produces unexpected results, you can capture this as negative learning and use it to improve future retrieval.

### The Problem

Imagine you're a developer who frequently makes the same mistakes:

- Running `git push -f` when you meant `git push`
- Using `rm -rf *` in the wrong directory
- Typing `cargo run` instead of `cargo build` for certain workflows

### The Solution: Learning via Negativa

1. **Capture Failed Commands**: Store failed commands with their error context
2. **Create Correction Knowledge**: Build a knowledge graph mapping wrong → right
3. **Use Replace Tool**: Automatically suggest corrections on similar patterns

### Example: Command Correction Knowledge Graph

```markdown
# Git Push Force

Running git push with -f flag can overwrite remote changes.

synonyms:: git push force, git push --force, git force push

Error context: "refusing to force push", "denied by remote"

Correction: git push (without -f)

# Cargo Run vs Build

cargo run compiles and executes, cargo build only compiles.

synonyms:: cargo execute, cargo compile

Use cargo build when: you only need to verify compilation
Use cargo run when: you need to execute the program
```

### Live Demo Output

```
Query: 'git push force'

Without correction knowledge:
  Results: git push force (incorrect usage)

With correction knowledge:
  Results: git push (suggested), with warning about force push risks
  Rank boost for: safe alternatives
```

---

## Next Steps

1. **Try the example**: Run `cargo run -p terraphim_rolegraph --example learnings_demo`

2. **Create your own agent**: Define a thesaurus for your domain

3. **Add more terms**: The more precise your terms, the better the retrieval

4. **Use synonyms**: They create additional edges, improving ranking

---

## API Reference

### RoleGraph Methods

```rust
// Create a new RoleGraph
RoleGraph::new(role_name, thesaurus).await

// Index a document
rolegraph.insert_document(&doc_id, document)

// Query with graph ranking
rolegraph.query_graph(query, offset, limit)

// Check term connectivity
rolegraph.is_all_terms_connected_by_path(query)

// Get graph statistics
rolegraph.get_graph_stats()
```

### Thesaurus Methods

```rust
// Create thesaurus
Thesaurus::new(name)

// Insert terms
thesaurus.insert(key, normalized_term)

// Lookup
thesaurus.get(&normalized_term_value)
```

---

## Conclusion

Terraphim's graph-based approach provides a powerful alternative to vector embeddings. By explicitly modeling term relationships through co-occurrence, it offers:

- **Better interpretability** - You can see exactly why a document was retrieved
- **Easy domain adaptation** - Just add terms to the knowledge graph
- **Privacy-first** - All processing happens locally
- **Incremental updates** - Add new knowledge without retraining
- **Learning via negativa** - Learn from mistakes and improve over time

The key insight is that **adding domain-specific terms directly improves retrieval** - there's no need for fine-tuning or training. This makes Terraphim particularly well-suited for personal knowledge management and specialized applications.
