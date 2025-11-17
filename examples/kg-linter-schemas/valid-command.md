---
name: "kg-search"
description: "Search knowledge graph with semantic matching"
version: "1.0.0"
execution_mode: "local"
risk_level: "low"
category: "knowledge-graph"

parameters:
  - name: "query"
    type: "string"
    required: true
    description: "Search query for knowledge graph"
    validation:
      min_length: 1
      max_length: 500
      pattern: "^[a-zA-Z0-9\\s\\-_]+$"

  - name: "max_results"
    type: "number"
    required: false
    default_value: 10
    description: "Maximum number of results to return"
    validation:
      min: 1
      max: 100

  - name: "fuzzy_threshold"
    type: "number"
    required: false
    default_value: 0.85
    description: "Fuzzy matching threshold (0.0-1.0)"
    validation:
      min: 0.0
      max: 1.0

permissions:
  - "kg:read"
  - "search:execute"

knowledge_graph_required:
  - "knowledge graph"
  - "semantic search"

resource_limits:
  max_memory_mb: 256
  max_cpu_time: 10
  network_access: false

timeout: 30
---

# Knowledge Graph Search Command

This command performs semantic search across the knowledge graph using
Terraphim's automata and graph embedding systems.

## Features

- **Fast Matching**: Aho-Corasick automata for O(n) pattern matching
- **Semantic Understanding**: Graph embeddings for concept relationships
- **Fuzzy Search**: Jaro-Winkler distance for typo tolerance
- **Path Connectivity**: Validates concept relationships in results

## Usage

```bash
/kg-search query="rust async programming" max_results=20
```

## Implementation

The command leverages:
1. `terraphim_automata::autocomplete_search` for fast term matching
2. `terraphim_rolegraph::find_matching_node_ids` for concept extraction
3. `is_all_terms_connected_by_path` for relationship validation
4. BM25 scoring for relevance ranking

## Returns

JSON array of matched documents with:
- Document ID and URL
- Matched concepts
- Relevance score
- Graph connectivity metadata
