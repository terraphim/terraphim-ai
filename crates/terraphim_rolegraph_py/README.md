# terraphim-rolegraph

Python bindings for the Rust `terraphim_rolegraph` crate -- knowledge graph
operations for AI agents.

## Installation

```bash
pip install terraphim-rolegraph
```

For development:

```bash
cd crates/terraphim_rolegraph_py
python -m venv .venv && source .venv/bin/activate
pip install maturin pytest pytest-cov
maturin develop
```

## Quick Start

```python
import json
from terraphim_rolegraph import RoleGraph, Document

# Define a thesaurus
thesaurus = json.dumps({
    "name": "Engineering",
    "data": {
        "machine learning": {"id": 1, "nterm": "machine learning", "url": ""},
        "deep learning": {"id": 2, "nterm": "deep learning", "url": ""},
        "neural network": {"id": 3, "nterm": "neural network", "url": ""},
    }
})

# Create a knowledge graph
rg = RoleGraph("engineer", thesaurus)

# Insert documents
doc = Document(
    id="doc1",
    url="https://example.com/doc1",
    title="ML Intro",
    body="machine learning uses deep learning and neural network architectures",
)
rg.insert_document("doc1", doc)

# Query the graph
results = rg.query_graph("machine learning")
for doc_id, indexed_doc in results:
    print(f"{doc_id}: rank={indexed_doc.rank}, tags={indexed_doc.tags}")

# Check graph statistics
stats = rg.get_graph_stats()
print(f"Nodes: {stats.node_count}, Edges: {stats.edge_count}")

# Serialize / deserialize
saved = rg.to_json()
restored = RoleGraph.from_json(saved)
```

## API Overview

### RoleGraph

| Method | Description |
|--------|-------------|
| `RoleGraph(role_name, thesaurus_json)` | Create a new knowledge graph |
| `insert_document(doc_id, document)` | Insert a document |
| `has_document(doc_id)` | Check if document exists |
| `get_document(doc_id)` | Get indexed document |
| `query_graph(query, offset?, limit?)` | Search by text |
| `query_graph_with_operators(terms, op, ...)` | Multi-term AND/OR search |
| `find_matching_node_ids(text)` | Get matched concept IDs |
| `is_all_terms_connected_by_path(text)` | Check path connectivity |
| `get_graph_stats()` | Get node/edge/document counts |
| `to_json()` / `from_json(s)` | Serialize/deserialize |

### Types

- `Document` -- writable document for insertion
- `IndexedDocument` -- read-only document with graph metadata
- `Node` -- graph node (concept)
- `Edge` -- graph edge (concept pair)
- `GraphStats` -- graph statistics
- `LogicalOperator` -- `And` / `Or` for multi-term queries

### Utility Functions

- `magic_pair(x, y)` -- combine two IDs into a unique edge ID
- `magic_unpair(z)` -- reverse a paired ID
- `split_paragraphs(text)` -- split text using unicode sentence boundaries

## Running Tests

```bash
maturin develop && pytest python/tests/ -v
```
