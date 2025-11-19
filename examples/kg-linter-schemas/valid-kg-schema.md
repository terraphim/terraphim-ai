---
schema_type: "knowledge_graph"
schema_name: "rust_programming"
version: "1.0.0"
namespace: "terraphim.kg.rust"

node_types:
  - name: "Concept"
    description: "A programming concept or term"
    properties:
      - name: "id"
        type: "u64"
        required: true
        unique: true
        description: "Unique identifier for the concept"

      - name: "value"
        type: "NormalizedTermValue"
        required: true
        description: "Normalized term (lowercase, trimmed)"

      - name: "rank"
        type: "u64"
        required: false
        default: 0
        description: "Co-occurrence count across documents"

      - name: "metadata"
        type: "HashMap<String, String>"
        required: false
        description: "Additional metadata (tags, categories, etc.)"

  - name: "Document"
    description: "Source document containing concepts"
    properties:
      - name: "id"
        type: "string"
        required: true
        unique: true
        description: "Unique document identifier"

      - name: "url"
        type: "string"
        required: true
        description: "Document URL or file path"
        validation:
          pattern: "^(https?://|file://|/)"

      - name: "title"
        type: "string"
        required: true
        description: "Document title"

      - name: "body"
        type: "string"
        required: true
        description: "Full document content"

      - name: "tags"
        type: "Vec<String>"
        required: false
        description: "Document tags for categorization"

edge_types:
  - name: "RelatedTo"
    description: "Concept co-occurrence relationship"
    from: "Concept"
    to: "Concept"
    properties:
      - name: "id"
        type: "u64"
        required: true
        unique: true
        description: "Edge identifier (pairing function of node IDs)"

      - name: "rank"
        type: "u64"
        required: true
        default: 1
        description: "Number of co-occurrences"

      - name: "doc_hash"
        type: "HashMap<String, u64>"
        required: true
        description: "Map of document IDs to occurrence counts"

  - name: "ContainedIn"
    description: "Concept appears in document"
    from: "Concept"
    to: "Document"
    properties:
      - name: "occurrences"
        type: "u64"
        required: true
        description: "Number of times concept appears in document"

relationships:
  - type: "path_connectivity"
    description: "All matched terms should be connected by a single path"
    validator: "is_all_terms_connected_by_path"
    applies_to: ["Concept", "Concept"]

  - type: "bidirectional"
    description: "RelatedTo edges are symmetric"
    constraint: "symmetric"
    applies_to: ["RelatedTo"]

validation_rules:
  - rule: "unique_concept_ids"
    description: "All concept IDs must be unique"
    severity: "error"
    validator: "validate_unique_ids"

  - rule: "normalized_terms"
    description: "Concept values must be lowercase and trimmed"
    severity: "error"
    validator: "validate_normalized_terms"

  - rule: "valid_edge_references"
    description: "Edge node references must exist"
    severity: "error"
    validator: "validate_edge_references"

  - rule: "document_urls"
    description: "Document URLs should be valid"
    severity: "warning"
    validator: "validate_urls"

security:
  read_permissions:
    - "kg:read"
    - "concept:view"
    - "document:view"

  write_permissions:
    - "kg:write"
    - "concept:create"
    - "concept:update"
    - "document:create"

  delete_permissions:
    - "kg:admin"
    - "concept:delete"
    - "document:delete"

automata_config:
  match_kind: "LeftmostLongest"
  ascii_case_insensitive: true
  enable_fuzzy: true
  fuzzy_algorithm: "jaro_winkler"
  fuzzy_threshold: 0.85
---

# Rust Programming Knowledge Graph

This knowledge graph represents Rust programming concepts, their relationships,
and source documentation for semantic search and learning.

## Graph Structure

### Nodes

- **Concepts**: Programming terms like "async/await", "ownership", "borrowing"
- **Documents**: Rust documentation, blog posts, code examples

### Edges

- **RelatedTo**: Connects related concepts (e.g., "async" ↔ "tokio")
- **ContainedIn**: Links concepts to documents where they appear

## Example

```
[async programming] --RelatedTo--> [tokio]
[async programming] --RelatedTo--> [futures]
[tokio] --ContainedIn--> [tokio-tutorial.md]
```

## Validation

The schema enforces:
1. Unique concept IDs
2. Normalized term format (lowercase)
3. Valid edge references
4. Path connectivity for search queries
5. Symmetric relationships

## Usage

This schema is used by:
- `terraphim_rolegraph` for graph construction
- `terraphim_automata` for fast term matching
- `terraphim_service` for semantic search
- The linter for validation
