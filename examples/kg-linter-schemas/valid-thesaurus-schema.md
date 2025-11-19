---
schema_type: "thesaurus"
thesaurus_name: "rust_stdlib"
version: "1.0.0"
namespace: "terraphim.thesaurus.rust"
case_sensitive: false
match_mode: "leftmost_longest"

term_structure:
  - name: "id"
    type: "u64"
    description: "Unique identifier for the normalized term"
    required: true
    validation:
      min: 1

  - name: "nterm"
    type: "string"
    description: "Normalized term value (lowercase, trimmed)"
    required: true
    validation:
      pattern: "^[a-z0-9\\s\\-_\\.::]+$"
      min_length: 1
      max_length: 200

  - name: "url"
    type: "string"
    description: "Documentation URL for the term"
    required: false
    validation:
      pattern: "^https?://"

automata_config:
  match_kind: "LeftmostLongest"
  ascii_case_insensitive: true
  enable_fuzzy: true
  fuzzy_algorithm: "jaro_winkler"
  fuzzy_threshold: 0.85
  min_pattern_length: 2
  max_patterns: 100000

validation_rules:
  - rule: "unique_ids"
    description: "All term IDs must be unique"
    severity: "error"

  - rule: "unique_nterms"
    description: "Normalized terms should be unique (warn on duplicates)"
    severity: "warning"

  - rule: "normalized_format"
    description: "All nterm values must be lowercase and trimmed"
    severity: "error"

  - rule: "valid_urls"
    description: "URLs must be valid HTTP/HTTPS when present"
    severity: "warning"

  - rule: "consistent_namespacing"
    description: "Rust terms should use :: for module paths"
    severity: "info"

synonym_groups:
  - canonical: "async/await"
    synonyms:
      - "async await"
      - "asynchronous programming"
      - "async rust"

  - canonical: "std::collections::hashmap"
    synonyms:
      - "hashmap"
      - "hash map"
      - "std hashmap"

  - canonical: "ownership"
    synonyms:
      - "rust ownership"
      - "ownership model"
      - "ownership system"

metadata:
  source: "Rust Standard Library Documentation"
  version: "1.75.0"
  generated_date: "2024-01-15"
  maintainer: "Terraphim AI"
  license: "MIT"

security:
  read_permissions:
    - "thesaurus:read"

  write_permissions:
    - "thesaurus:write"
    - "thesaurus:update"

  delete_permissions:
    - "thesaurus:admin"
---

# Rust Standard Library Thesaurus

This thesaurus provides term normalization and synonym mapping for Rust
standard library concepts, enabling semantic search and autocomplete.

## Format

The thesaurus follows the Terraphim JSON format:

```json
{
  "name": "rust_stdlib",
  "data": {
    "std::collections::HashMap": {
      "id": 1,
      "nterm": "std::collections::hashmap",
      "url": "https://doc.rust-lang.org/std/collections/struct.HashMap.html"
    },
    "async/await": {
      "id": 2,
      "nterm": "async/await",
      "url": "https://rust-lang.github.io/async-book/01_getting_started/01_chapter.html"
    }
  }
}
```

## Features

- **Case-Insensitive**: Matches "HashMap", "hashmap", "HASHMAP"
- **Leftmost-Longest**: Prefers longer matches (e.g., "std::vec::Vec" over "vec")
- **Fuzzy Matching**: Handles typos with Jaro-Winkler similarity
- **Synonym Groups**: Maps variations to canonical terms

## Usage

### Autocomplete

```rust
use terraphim_automata::{autocomplete_search, load_autocomplete_index};

let index = load_autocomplete_index(path).await?;
let results = autocomplete_search(&index, "hashm", 5);
// Returns: ["HashMap", "hash map", "std::collections::HashMap", ...]
```

### Fuzzy Search

```rust
let results = fuzzy_autocomplete_search(&index, "hasmhap", 3);
// Returns matches with similarity >= 0.85
// ["HashMap" (0.93), "hash map" (0.87), ...]
```

## Validation

The schema enforces:
1. Unique term IDs
2. Normalized term format (lowercase, specific character set)
3. Valid URLs for documentation links
4. Consistent Rust namespacing with `::`

## Integration

This thesaurus is used by:
- `terraphim_automata` for building Aho-Corasick automata
- `terraphim_rolegraph` for concept normalization
- MCP autocomplete service for IDE integration
