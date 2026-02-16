---
title: "zvec vs Terraphim: Two Paths to Semantic Search"
description: "A deep dive comparing Alibaba's zvec neural vector database with Terraphim's graph-based approach to semantic search"
author: "Terraphim Team"
date: "2026-02-16"
tags: ["vector-search", "knowledge-graph", "semantic-search", "comparison"]
---

# zvec vs Terraphim: Two Paths to Semantic Search

When it comes to semantic search, there are fundamentally different architectural approaches. Alibaba's [zvec](https://github.com/alibaba/zvec) and Terraphim represent two distinct philosophies: neural embeddings vs. knowledge graphs, scale vs. interpretability, dense vectors vs. co-occurrence relationships.

Let's explore how these systems differ and when to choose one over the other.

## The Core Philosophy

### zvec: Neural Embeddings at Scale

zvec is a lightweight, in-process vector database built on Alibaba's battle-tested Proxima engine. It transforms documents into high-dimensional vectors using neural embedding models (BERT, OpenAI, etc.), then uses Approximate Nearest Neighbor (ANN) algorithms like HNSW to find similar documents.

**Key Characteristics:**
- Dense vectors (typically 384-1536 dimensions)
- ANN indexing (HNSW, IVF, Flat)
- Built-in embedding models (OpenAI, Qwen, SentenceTransformers)
- Billions of vectors, millisecond query times
- Black-box interpretability

### Terraphim: Knowledge Graphs for Understanding

Terraphim takes a radically different approach. Instead of converting documents to opaque vectors, it builds a knowledge graph from term co-occurrences. Each concept becomes a node, relationships become edges, and relevance is calculated by traversing this graph structure.

**Key Characteristics:**
- Co-occurrence graph embeddings
- Aho-Corasick automata for fast pattern matching
- Domain-specific thesauri for synonym expansion
- Role-based graphs for persona-driven search
- Fully explainable relevance scoring

## Architectural Comparison

```
┌─────────────────────────────────────────────────────────────┐
│                         zvec                                 │
├─────────────────────────────────────────────────────────────┤
│  Document → Neural Encoder → Dense Vector → HNSW Index      │
│                                                     ↓        │
│  Query → Neural Encoder → Query Vector → ANN Search → Top-K │
└─────────────────────────────────────────────────────────────┘
                              vs
┌─────────────────────────────────────────────────────────────┐
│                       Terraphim                              │
├─────────────────────────────────────────────────────────────┤
│  Document → Term Extraction → Co-occurrence → Graph         │
│                                                    ↓         │
│  Query → Aho-Corasick Match → Graph Traversal → Ranked Docs │
└─────────────────────────────────────────────────────────────┘
```

### Data Structures

| Component | zvec | Terraphim |
|-----------|------|-----------|
| **Storage Unit** | Collection (table-like) | RoleGraph (knowledge graph) |
| **Document ID** | String | String |
| **Representations** | Dense/Sparse vectors (768-dim+) | Nodes, Edges, Thesaurus |
| **Index Types** | HNSW, IVF, Flat, Inverted | Hash maps + Aho-Corasick |
| **Persistence** | Disk-based collections | JSON serialization |

### Query Semantics

**zvec Query:**
```python
import zvec

# Semantic similarity via vector comparison
results = collection.query(
    zvec.VectorQuery("embedding", vector=[0.1, -0.3, ...]),
    topk=10,
    filter="category == 'tech'"
)
# Returns: documents with similar vectors (cosine similarity)
```

**Terraphim Query:**
```rust
// Graph traversal with term expansion
let results = role_graph.query_graph(
    "async programming",
    Some(0),  // offset
    Some(10)  // limit
);
// Returns: documents ranked by graph connectivity
// Matched nodes: "async", "programming", "concurrency", "tokio"
```

## Feature Matrix

| Feature | zvec | Terraphim |
|---------|------|-----------|
| **Dense Embeddings** | ✅ Native | ❌ Not used |
| **Sparse Vectors** | ✅ BM25 supported | ✅ BM25/BM25F/BM25Plus |
| **Knowledge Graph** | ❌ No | ✅ Core architecture |
| **ANN Search** | ✅ HNSW/IVF/Flat | ❌ Not applicable |
| **SQL-like Filters** | ✅ SQL engine | ❌ Graph-based filtering |
| **Explainability** | ⚠️ Low (black box) | ✅ High (show path) |
| **Synonym Expansion** | ⚠️ Via embedding model | ✅ Via thesaurus |
| **Role/Persona Support** | ❌ No | ✅ RoleGraphs |
| **Multi-Haystack** | ❌ Single collection | ✅ Multiple sources |
| **Built-in Rerankers** | ✅ RRF, Weighted | ❌ Graph ranks directly |
| **Quantization** | ✅ INT8/FP16 | ❌ Not needed |
| **Hybrid Search** | ✅ Vectors + Filters | ✅ Graph + Haystacks |

## Performance Characteristics

### zvec (Benchmarks from 10M vector dataset)

- **Throughput**: 2,000-8,000 QPS depending on configuration
- **Recall**: 96-97% with HNSW
- **Latency**: Milliseconds for 10M vectors
- **Memory**: Compressed vectors (INT8/FP16)
- **Scale**: Billions of vectors

### Terraphim (Observed Performance)

- **Throughput**: In-memory graph traversal (very fast)
- **Recall**: Deterministic graph-based ranking
- **Latency**: Sub-millisecond for typical graphs
- **Memory**: Entire graph in memory
- **Scale**: Thousands to tens of thousands of documents

## When to Use Which

### Choose zvec When:

1. **You need to search billions of documents**
   - ANN algorithms scale to massive datasets
   - Production workloads at Alibaba scale

2. **You're building RAG systems with LLMs**
   - Dense embeddings align with LLM representations
   - Built-in OpenAI/SentenceTransformer support

3. **You need image/audio similarity search**
   - Requires dense embeddings
   - CLIP-style multimodal search

4. **Exact semantic similarity matters**
   - "King - Man + Woman ≈ Queen" works
   - Captures semantic relationships beyond keywords

### Choose Terraphim When:

1. **You need explainable results**
   - "Why did this document rank high?"
   - Graph path shows: matched node X via edge Y to document Z

2. **You have domain-specific knowledge**
   - Custom thesauri for technical terms
   - Synonym relationships: "async" ↔ "asynchronous" ↔ "non-blocking"

3. **You're building personal knowledge management**
   - Note-taking apps, research assistants
   - Domain expert systems

4. **You need role-based search**
   - Different personas see different results
   - Engineer vs. Scientist vs. Writer views

## Code Comparison

### Document Indexing

**zvec (Python):**
```python
import zvec

schema = zvec.CollectionSchema(
    name="docs",
    vectors=zvec.VectorSchema("emb", zvec.DataType.VECTOR_FP32, 768),
)

collection = zvec.create_and_open(path="./data", schema=schema)

# Documents must have pre-computed embeddings
collection.insert([
    zvec.Doc(
        id="doc1",
        vectors={"emb": embedding_model.encode("Rust async programming")},
        fields={"title": "Async in Rust"}
    ),
])
```

**Terraphim (Rust):**
```rust
use terraphim_rolegraph::RoleGraph;
use terraphim_types::{Document, RoleName};

let mut graph = RoleGraph::new(
    RoleName::new("engineer"),
    thesaurus
).await?;

// Documents are indexed into the graph
graph.index_documents(vec![
    Document {
        id: "doc1".into(),
        title: "Async in Rust".into(),
        body: "Rust's async/await syntax...".into(),
        // Graph extracts terms automatically
        ..Default::default()
    },
]).await?;
```

### Searching

**zvec:**
```python
# Vector similarity search
query_vec = embedding_model.encode("how to write async code")
results = collection.query(
    zvec.VectorQuery("emb", vector=query_vec),
    topk=5
)
# Results ranked by cosine similarity
```

**Terraphim:**
```rust
// Graph-based search
let results = graph.query_graph("async code", None, Some(5))?;
// Results ranked by:
// 1. Node rank (concept frequency)
// 2. Edge rank (relationship strength)
// 3. Document rank (occurrence count)
```

## Can They Work Together?

Absolutely! Here are some integration patterns:

### 1. Hybrid Retrieval

Use zvec for initial broad retrieval, Terraphim for reranking:

```python
# Step 1: zvec ANN for candidate retrieval
candidates = zvec_collection.query(query_vector, topk=100)

# Step 2: Terraphim graph reranking
# Load candidates into temporary graph
# Re-rank based on knowledge graph connectivity
```

### 2. Sparse BM25 in Terraphim

zvec includes a `BM25EmbeddingFunction` for sparse vectors. Terraphim could add this as another haystack:

```rust
// Hypothetical: Terraphim with zvec-style sparse embeddings
Haystack {
    service: ServiceType::BM25Sparse,
    embedding_function: "zvec::BM25EmbeddingFunction",
}
```

### 3. Explainable Vector Search

Use Terraphim's graph to explain zvec results:

```
User: "Why did this document match?"
System:
  - zvec: "Vector similarity: 0.92"
  - Terraphim: "Matched via concepts: async → tokio → concurrency"
```

## Conclusion

zvec and Terraphim solve semantic search with fundamentally different approaches:

- **zvec** scales neural embeddings to billions of documents using ANN algorithms. It's the right choice for large-scale RAG systems, e-commerce search, and any application requiring dense vector similarity.

- **Terraphim** builds interpretable knowledge graphs from term relationships. It excels at personal knowledge management, domain-specific expert systems, and any application where understanding *why* a document matched is as important as finding it.

The exciting possibility is combining both: zvec's scale with Terraphim's explainability. The future of semantic search might just be hybrid.

## References

- zvec GitHub: https://github.com/alibaba/zvec
- zvec Documentation: https://zvec.org/en/docs/
- Terraphim Documentation: https://terraphim.ai/docs
- Proxima (Alibaba's Vector Engine): https://github.com/alibaba/proxima

---

*Have you used zvec or Terraphim? We'd love to hear about your experiences in the comments or on [GitHub Discussions](https://github.com/terraphim/terraphim-ai/discussions).*
