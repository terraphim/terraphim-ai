# Changelog

All notable changes to `terraphim_rolegraph` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2025-01-22

### Added

#### Core Functionality
- **RoleGraph**: Role-specific knowledge graph for semantic document search
- **Graph-Based Ranking**: Sum node rank + edge rank + document rank for relevance
- **Aho-Corasick Matching**: Fast multi-pattern text scanning with case-insensitive search
- **Path Connectivity**: Check if all matched terms connect via graph paths (DFS backtracking)
- **Multi-term Queries**: AND/OR logical operators for complex searches

#### API Methods

**Graph Construction:**
- `RoleGraph::new()` - Create graph from role and thesaurus (async)
- `insert_document()` - Index document and build graph structure
- `add_or_update_document()` - Add/update document with concept pair

**Querying:**
- `query_graph()` - Simple text query with offset/limit
- `query_graph_with_operators()` - Multi-term query with AND/OR operators
- `find_matching_node_ids()` - Get matched concept IDs from text
- `is_all_terms_connected_by_path()` - Check graph path connectivity

**Graph Inspection:**
- `get_graph_stats()` - Statistics (nodes, edges, documents, thesaurus size)
- `get_node_count()` / `get_edge_count()` / `get_document_count()`
- `is_graph_populated()` - Check if graph has indexed content
- `nodes_map()` / `edges_map()` - Access internal graph structures
- `validate_documents()` - Find orphaned/invalid documents
- `find_document_ids_for_term()` - Reverse lookup: term → document IDs

**Document Access:**
- `get_document()` - Retrieve indexed document by ID
- `get_all_documents()` - Iterator over all documents
- `has_document()` - Check if document exists
- `document_count()` - Total indexed documents

#### Types
- `Error` - Comprehensive error types (NodeIdNotFound, EdgeIdNotFound, etc.)
- `GraphStats` - Statistics structure with counts and population status
- `RoleGraphSync` - Thread-safe async wrapper using tokio::sync::Mutex

#### Utility Functions
- `split_paragraphs()` - Split text into paragraph vectors
- `magic_pair(x, y)` - Create unique edge ID from node IDs
- `magic_unpair(z)` - Extract node IDs from edge ID

### Performance
- O(n) text matching with Aho-Corasick
- O(k×e×d) graph query (k=terms, e=edges/node, d=docs/edge)
- ~100 bytes per node, ~200 bytes per edge
- Sub-10ms queries for typical workloads

### Documentation
- Comprehensive module-level documentation with examples
- Rustdoc comments on all public functions and types
- Usage examples for:
  - Creating and querying graphs
  - Path connectivity checking
  - Multi-term queries with operators
  - Document indexing
- README with architecture overview and quick start
- Full API documentation

### Implementation Details
- Aho-Corasick with LeftmostLongest matching
- Case-insensitive term matching
- Bidirectional graph navigation
- DFS-based path connectivity (with visited edge tracking)
- Hash-based storage using ahash::AHashMap
- Async-first design with tokio integration
- Memoization support with `memoize` crate
- Unicode text segmentation

### Features
- Full async/await support
- Thread-safe with `RoleGraphSync`
- No required feature flags
- Compatible with terraphim_types v1.0.0
- Compatible with terraphim_automata v1.0.0

[Unreleased]: https://github.com/terraphim/terraphim-ai/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/terraphim/terraphim-ai/releases/tag/v1.0.0
