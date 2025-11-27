# terraphim_rolegraph

[![Crates.io](https://img.shields.io/crates/v/terraphim_rolegraph.svg)](https://crates.io/crates/terraphim_rolegraph)
[![Documentation](https://docs.rs/terraphim_rolegraph/badge.svg)](https://docs.rs/terraphim_rolegraph)
[![License](https://img.shields.io/crates/l/terraphim_rolegraph.svg)](https://github.com/terraphim/terraphim-ai/blob/main/LICENSE-Apache-2.0)

Knowledge graph implementation for semantic document search.

## Overview

`terraphim_rolegraph` provides a role-specific knowledge graph that connects concepts, relationships, and documents for graph-based semantic search. Results are ranked by traversing relationships between matched concepts.

## Features

- **📊 Graph-Based Search**: Navigate concept relationships for smarter results
- **🔍 Multi-Pattern Matching**: Fast Aho-Corasick text scanning
- **🎯 Semantic Ranking**: Sum node + edge + document ranks
- **🔗 Path Connectivity**: Check if matched terms connect via graph paths
- **⚡ High Performance**: O(n) matching, efficient graph traversal
- **🎭 Role-Specific**: Separate graphs for different user personas

## Installation

```toml
[dependencies]
terraphim_rolegraph = "1.0.0"
```

## Quick Start

### Creating and Querying a Graph

```rust
use terraphim_rolegraph::RoleGraph;
use terraphim_types::{RoleName, Thesaurus, NormalizedTermValue, NormalizedTerm, Document};

#[tokio::main]
async fn main() -> Result<(), terraphim_rolegraph::Error> {
    // Create thesaurus
    let mut thesaurus = Thesaurus::new("engineering".to_string());
    thesaurus.insert(
        NormalizedTermValue::from("rust"),
        NormalizedTerm {
            id: 1,
            value: NormalizedTermValue::from("rust programming"),
            url: Some("https://rust-lang.org".to_string()),
        }
    );
    thesaurus.insert(
        NormalizedTermValue::from("async"),
        NormalizedTerm {
            id: 2,
            value: NormalizedTermValue::from("asynchronous programming"),
            url: Some("https://rust-lang.github.io/async-book/".to_string()),
        }
    );

    // Create role graph
    let mut graph = RoleGraph::new(
        RoleName::new("engineer"),
        thesaurus
    ).await?;

    // Index documents
    let doc = Document {
        id: "rust-async-guide".to_string(),
        title: "Async Rust Programming".to_string(),
        body: "Learn rust and async programming with tokio".to_string(),
        url: "https://example.com/rust-async".to_string(),
        description: Some("Comprehensive async Rust guide".to_string()),
        summarization: None,
        stub: None,
        tags: Some(vec!["rust".to_string(), "async".to_string()]),
        rank: None,
        source_haystack: None,
    };
    let doc_id = doc.id.clone();
    graph.insert_document(&doc_id, doc);

    // Query the graph
    let results = graph.query_graph("rust async", None, Some(10))?;
    for (id, indexed_doc) in results {
        println!("Document: {} (rank: {})", id, indexed_doc.rank);
    }

    Ok(())
}
```

### Path Connectivity Checking

```rust
use terraphim_rolegraph::RoleGraph;
use terraphim_types::{RoleName, Thesaurus};

#[tokio::main]
async fn main() -> Result<(), terraphim_rolegraph::Error> {
    let thesaurus = Thesaurus::new("engineering".to_string());
    let graph = RoleGraph::new(RoleName::new("engineer"), thesaurus).await?;

    // Check if matched terms are connected by a graph path
    let text = "rust async tokio programming";
    let connected = graph.is_all_terms_connected_by_path(text);

    if connected {
        println!("All terms are connected - they form a coherent topic!");
    } else {
        println!("Terms are disconnected - possibly unrelated concepts");
    }

    Ok(())
}
```

### Multi-term Queries with Operators

```rust
use terraphim_rolegraph::RoleGraph;
use terraphim_types::{RoleName, Thesaurus, LogicalOperator};

#[tokio::main]
async fn main() -> Result<(), terraphim_rolegraph::Error> {
    let thesaurus = Thesaurus::new("engineering".to_string());
    let mut graph = RoleGraph::new(RoleName::new("engineer"), thesaurus).await?;

    // AND query - documents must contain ALL terms
    let results = graph.query_graph_with_operators(
        &["rust", "async", "tokio"],
        &LogicalOperator::And,
        None,
        Some(10)
    )?;
    println!("AND query: {} results", results.len());

    // OR query - documents may contain ANY term
    let results = graph.query_graph_with_operators(
        &["rust", "python", "go"],
        &LogicalOperator::Or,
        None,
        Some(10)
    )?;
    println!("OR query: {} results", results.len());

    Ok(())
}
```

## Architecture

### Graph Structure

The knowledge graph uses a three-layer structure:

1. **Nodes** (Concepts)
   - Represent terms from the thesaurus
   - Track frequency/importance (rank)
   - Connect to related concepts via edges

2. **Edges** (Relationships)
   - Connect concepts that co-occur in documents
   - Weighted by co-occurrence strength (rank)
   - Associate documents via concept pairs

3. **Documents** (Content)
   - Indexed by concepts they contain
   - Linked via edges between their concepts
   - Ranked by node + edge + document scores

### Ranking Algorithm

Search results are ranked by summing:

```
total_rank = node_rank + edge_rank + document_rank
```

- **node_rank**: How important/frequent the concept is
- **edge_rank**: How strong the concept relationship is
- **document_rank**: Document-specific relevance

Higher total rank = more relevant result.

### Performance Characteristics

- **Text Matching**: O(n) with Aho-Corasick multi-pattern matching
- **Graph Query**: O(k × e × d) where:
  - k = number of matched terms
  - e = average edges per node
  - d = average documents per edge
- **Memory**: ~100 bytes/node + ~200 bytes/edge
- **Connectivity Check**: DFS with backtracking (exponential worst case, fast for k≤8)

## API Overview

### Core Methods

- `RoleGraph::new()` - Create graph from thesaurus
- `insert_document()` - Index a document
- `query_graph()` - Simple text query
- `query_graph_with_operators()` - Multi-term query with AND/OR
- `is_all_terms_connected_by_path()` - Check path connectivity
- `find_matching_node_ids()` - Get matched concept IDs

### Graph Inspection

- `get_graph_stats()` - Statistics (node/edge/document counts)
- `get_node_count()` / `get_edge_count()` / `get_document_count()`
- `is_graph_populated()` - Check if graph has content
- `validate_documents()` - Find orphaned documents
- `find_document_ids_for_term()` - Reverse lookup

### Async Support

The graph uses `tokio::sync::Mutex` for thread-safe async operations:

```rust
use terraphim_rolegraph::RoleGraphSync;

let sync_graph = RoleGraphSync::new(graph);
let locked = sync_graph.lock().await;
let results = locked.query_graph("search term", None, Some(10))?;
```

## Utility Functions

### Text Processing

- `split_paragraphs()` - Split text into paragraphs

### Node ID Pairing

- `magic_pair(x, y)` - Create unique edge ID from two node IDs
- `magic_unpair(z)` - Extract node IDs from edge ID

## Examples

See the [examples/](../../examples/) directory for:
- Building graphs from markdown files
- Multi-role graph management
- Custom ranking strategies
- Path analysis and connectivity

## Minimum Supported Rust Version (MSRV)

This crate requires Rust 1.70 or later.

## License

Licensed under Apache-2.0. See [LICENSE](../../LICENSE-Apache-2.0) for details.

## Related Crates

- **[terraphim_types](../terraphim_types)**: Core type definitions
- **[terraphim_automata](../terraphim_automata)**: Text matching and autocomplete
- **[terraphim_service](../terraphim_service)**: Main service layer with search

## Support

- **Discord**: https://discord.gg/VPJXB6BGuY
- **Discourse**: https://terraphim.discourse.group
- **Issues**: https://github.com/terraphim/terraphim-ai/issues
