# Summary: terraphim_rolegraph/src/lib.rs

**Purpose:** Knowledge graph implementation with role-based indexing.

**Key Details:**
- Core types: `RoleGraph`, `RoleGraphSync` for thread-safe graph operations
- Uses `AhoCorasick` for fast term matching
- Uses `ahash::AHashMap` for high-performance hash maps
- `TriggerIndex`: TF-IDF fallback search when exact matches fail
- Default trigger threshold: 0.3
- Medical feature gate: `medical` feature enables medical loaders and symbolic embeddings
- Error type: `Error` with variants for NodeIdNotFound, EdgeIdNotFound, JsonConversionError
- Graph stats: node_count, edge_count, document_count, thesaurus_size
- Unicode-aware tokenization via `unicode_segmentation`
