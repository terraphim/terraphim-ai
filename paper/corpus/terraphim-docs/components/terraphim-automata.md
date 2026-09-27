# Terraphim Automata

The `terraphim_automata` crate provides high-performance text processing capabilities using finite state automata (FST) for term matching, autocomplete, and fuzzy search.

## Architecture

### Core Components

#### Finite State Automata (FST)
The crate implements a finite state transducer for efficient term lookup and matching:

```rust
pub struct Automata {
    fst: Map<Vec<u8>>,
    // Internal FST implementation
}
```

**Key Features**:
- O(p+k) prefix search complexity
- Memory-efficient representation
- Fast term lookup and matching
- WASM-compatible design

#### Autocomplete System
Advanced autocomplete with fuzzy matching capabilities:

```rust
pub struct Autocomplete {
    automata: Automata,
    // Autocomplete implementation
}
```

## API Reference

### Basic Usage

#### Creating an Automata
```rust
use terraphim_automata::Automata;

let terms = vec![
    "rust".to_string(),
    "programming".to_string(),
    "systems".to_string(),
];

let automata = Automata::new(terms)?;
```

#### Prefix Search
```rust
let results = automata.prefix_search("ru")?;
// Returns: ["rust"]
```

#### Fuzzy Autocomplete
```rust
use terraphim_automata::Autocomplete;

let autocomplete = Autocomplete::new(terms)?;

// Jaro-Winkler similarity (default)
let results = autocomplete.fuzzy_autocomplete_search("rust", 0.8)?;

// Levenshtein distance
let results = autocomplete.fuzzy_autocomplete_search_levenshtein("rust", 2)?;
```

### Advanced Features

#### Custom Similarity Thresholds
```rust
// High similarity threshold for exact matches
let exact_matches = autocomplete.fuzzy_autocomplete_search("term", 0.95)?;

// Lower threshold for fuzzy matches
let fuzzy_matches = autocomplete.fuzzy_autocomplete_search("term", 0.7)?;
```

#### Term Replacement
```rust
use terraphim_automata::matcher;

let text = "Rust is a systems programming language";
let thesaurus = vec![
    ("rust".to_string(), "Rust".to_string()),
    ("systems".to_string(), "Systems".to_string()),
];

let matches = matcher::find_matches(&text, &thesaurus)?;
let replaced = matcher::replace_matches(&text, &thesaurus, Format::Wiki)?;
```

## Performance Characteristics

### Algorithm Comparison

#### Jaro-Winkler (Default)
- **Speed**: 2.3x faster than Levenshtein
- **Quality**: Superior for autocomplete scenarios
- **Use Case**: General-purpose fuzzy matching

#### Levenshtein Distance
- **Speed**: Baseline implementation
- **Quality**: Good for exact edit distance
- **Use Case**: When precise edit distance matters

### Benchmarks
```
10K terms processed in ~78ms
Throughput: 120+ MiB/s
Memory usage: Efficient FST representation
```

## WASM Compatibility

### WebAssembly Support
The crate is designed for WASM deployment:

```rust
#[cfg(target_arch = "wasm32")]
pub fn wasm_autocomplete(query: &str) -> Result<Vec<String>> {
    let autocomplete = Autocomplete::new(terms)?;
    autocomplete.fuzzy_autocomplete_search(query, 0.8)
}
```

### Feature Flags
```toml
[dependencies.terraphim_automata]
version = "0.1.0"
features = ["wasm-bindgen"]
```

## Text Processing

### Term Matching
```rust
use terraphim_automata::matcher;

// Find all term occurrences in text
let matches = matcher::find_matches(text, thesaurus)?;

// Replace terms with formatted links
let wiki_links = matcher::replace_matches(text, thesaurus, Format::Wiki)?;
let html_links = matcher::replace_matches(text, thesaurus, Format::Html)?;
let markdown_links = matcher::replace_matches(text, thesaurus, Format::Markdown)?;
```

### Pattern Validation (Important!)

Both `find_matches` and `replace_matches` automatically filter invalid patterns to prevent issues:

```rust
/// Minimum pattern length to prevent spurious matches.
/// Patterns shorter than this are filtered out to avoid:
/// - Empty patterns matching at every character position
/// - Single-character patterns causing excessive matches
const MIN_PATTERN_LENGTH: usize = 2;
```

**Why this matters**: Empty patterns in Aho-Corasick automata match at every position (index 0, 1, 2, ...), causing spurious text insertions between every character.

**Example of the bug (now fixed)**:
- Input: `npm install express`
- With empty pattern: `bun install exmatching...pmatching...` (broken!)
- With filtering: `bun install express` (correct!)

**Automatic filtering includes**:
- Empty strings (`""`)
- Single characters (`"a"`, `"x"`)
- Whitespace-only strings (`"   "`)

Invalid patterns are logged as warnings for debugging:
```
WARN: Skipping invalid pattern: "" (length 0 < 2)
```

### Format Options
```rust
pub enum Format {
    Wiki,      // [[term]]
    Html,      // <a href="kg:term">term</a>
    Markdown,  // [term](kg:term)
}
```

## Builder System

### Automata Construction
```rust
use terraphim_automata::builder;

let builder = builder::Builder::new();
let automata = builder
    .add_term("rust".to_string())
    .add_term("programming".to_string())
    .add_term("systems".to_string())
    .build()?;
```

### Batch Processing
```rust
let terms = vec![
    "rust".to_string(),
    "programming".to_string(),
    "systems".to_string(),
    "automata".to_string(),
];

let automata = builder::Builder::from_terms(terms)?.build()?;
```

## Error Handling

### Custom Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum AutomataError {
    #[error("FST construction failed: {0}")]
    FstConstruction(String),

    #[error("Invalid term: {0}")]
    InvalidTerm(String),

    #[error("Search failed: {0}")]
    SearchError(String),
}
```

### Result Types
```rust
pub type Result<T> = std::result::Result<T, AutomataError>;

// Usage
let automata = Automata::new(terms)?;
let results = automata.prefix_search("term")?;
```

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_search() {
        let automata = Automata::new(vec!["rust".to_string()])?;
        let results = automata.prefix_search("ru")?;
        assert_eq!(results, vec!["rust"]);
    }

    #[test]
    fn test_fuzzy_autocomplete() {
        let autocomplete = Autocomplete::new(vec!["rust".to_string()])?;
        let results = autocomplete.fuzzy_autocomplete_search("rust", 0.8)?;
        assert!(!results.is_empty());
    }
}
```

### Integration Tests
```rust
#[test]
fn test_algorithm_comparison() {
    let terms = vec!["rust".to_string(), "programming".to_string()];
    let autocomplete = Autocomplete::new(terms)?;

    // Jaro-Winkler (faster)
    let jw_results = autocomplete.fuzzy_autocomplete_search("rust", 0.8)?;

    // Levenshtein (baseline)
    let lev_results = autocomplete.fuzzy_autocomplete_search_levenshtein("rust", 2)?;

    // Results should be similar but Jaro-Winkler is faster
    assert!(jw_results.len() >= lev_results.len());
}
```

## Configuration

### Performance Tuning
```rust
// Adjust similarity threshold for different use cases
let strict_matching = autocomplete.fuzzy_autocomplete_search("term", 0.9)?;
let loose_matching = autocomplete.fuzzy_autocomplete_search("term", 0.6)?;

// Adjust edit distance for Levenshtein
let exact_matching = autocomplete.fuzzy_autocomplete_search_levenshtein("term", 0)?;
let fuzzy_matching = autocomplete.fuzzy_autocomplete_search_levenshtein("term", 3)?;
```

### Memory Management
```rust
// Efficient FST construction
let automata = Automata::new(terms)?;

// Memory-efficient term storage
let autocomplete = Autocomplete::new(terms)?;
```

## Best Practices

### Term Selection
```rust
// Use meaningful, normalized terms
let good_terms = vec![
    "rust-programming".to_string(),
    "systems-programming".to_string(),
    "knowledge-graph".to_string(),
];

// Avoid overly generic terms
let bad_terms = vec![
    "the".to_string(),
    "and".to_string(),
    "or".to_string(),
];
```

### Performance Optimization
```rust
// Reuse automata instances when possible
let automata = Automata::new(terms)?;

// Batch operations for better performance
let queries = vec!["rust", "programming", "systems"];
for query in queries {
    let results = automata.prefix_search(query)?;
    // Process results
}
```

### Error Handling
```rust
match autocomplete.fuzzy_autocomplete_search("term", 0.8) {
    Ok(results) => {
        // Process results
    }
    Err(AutomataError::InvalidTerm(term)) => {
        // Handle invalid term
    }
    Err(e) => {
        // Handle other errors
    }
}
```

## Integration Examples

### Desktop Application
```rust
// In desktop app
let autocomplete = Autocomplete::new(terms)?;
let suggestions = autocomplete.fuzzy_autocomplete_search(&query, 0.8)?;

// Update UI with suggestions
update_suggestions(suggestions);
```

### Web Application (WASM)
```rust
#[wasm_bindgen]
pub fn get_autocomplete_suggestions(query: &str) -> Result<JsValue> {
    let autocomplete = Autocomplete::new(terms)?;
    let results = autocomplete.fuzzy_autocomplete_search(query, 0.8)?;
    Ok(JsValue::from_serde(&results)?)
}
```

### Knowledge Graph Integration
```rust
// Replace terms with KG links
let text = "Rust is a systems programming language";
let thesaurus = load_knowledge_graph_terms()?;
let enhanced_text = matcher::replace_matches(&text, &thesaurus, Format::Wiki)?;
```

## Dependencies

### Internal Dependencies
- Minimal dependencies for WASM compatibility
- Focus on performance and memory efficiency

### External Dependencies
- `fst`: Finite state transducer implementation
- `serde`: Serialization support
- `wasm-bindgen`: WASM bindings (optional)

## Migration Guide

### From Simple String Matching
1. Replace string-based search with FST-based search
2. Use `Automata::new()` for prefix search
3. Use `Autocomplete::new()` for fuzzy search
4. Update error handling for new error types

### Performance Optimization
1. Use Jaro-Winkler for general autocomplete
2. Use Levenshtein for exact edit distance needs
3. Batch operations when possible
4. Reuse automata instances
