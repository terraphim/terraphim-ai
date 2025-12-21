# terraphim_automata

[![Crates.io](https://img.shields.io/crates/v/terraphim_automata.svg)](https://crates.io/crates/terraphim_automata)
[![Documentation](https://docs.rs/terraphim_automata/badge.svg)](https://docs.rs/terraphim_automata)
[![License](https://img.shields.io/crates/l/terraphim_automata.svg)](https://github.com/terraphim/terraphim-ai/blob/main/LICENSE-Apache-2.0)

Fast text matching and autocomplete engine for knowledge graphs.

## Overview

`terraphim_automata` provides high-performance text processing using Aho-Corasick automata and finite state transducers (FST). It powers Terraphim's autocomplete and knowledge graph linking features with sub-millisecond performance.

## Features

- **⚡ Fast Autocomplete**: FST-based prefix search with ~1ms response time
- **🔍 Fuzzy Matching**: Levenshtein and Jaro-Winkler distance algorithms
- **🔗 Link Generation**: Convert terms to Markdown, HTML, or Wiki links
- **📝 Text Processing**: Multi-pattern matching with Aho-Corasick
- **🌐 WASM Support**: Browser-compatible with TypeScript bindings
- **🚀 Async Loading**: HTTP-based thesaurus loading (optional)

## Installation

```toml
[dependencies]
terraphim_automata = "1.0.0"
```

With remote loading support:

```toml
[dependencies]
terraphim_automata = { version = "1.0.0", features = ["remote-loading", "tokio-runtime"] }
```

For WASM/browser usage:

```toml
[dependencies]
terraphim_automata = { version = "1.0.0", features = ["wasm", "typescript"] }
```

## Quick Start

### Autocomplete with Fuzzy Matching

```rust
use terraphim_automata::{build_autocomplete_index, fuzzy_autocomplete_search};
use terraphim_types::{Thesaurus, NormalizedTermValue, NormalizedTerm};

// Create a thesaurus
let mut thesaurus = Thesaurus::new("programming".to_string());
thesaurus.insert(
    NormalizedTermValue::from("rust"),
    NormalizedTerm { id: 1, value: NormalizedTermValue::from("rust"), url: None }
);
thesaurus.insert(
    NormalizedTermValue::from("rust async"),
    NormalizedTerm { id: 2, value: NormalizedTermValue::from("rust async"), url: None }
);

// Build autocomplete index
let index = build_autocomplete_index(thesaurus, None).unwrap();

// Fuzzy search (handles typos)
let results = fuzzy_autocomplete_search(&index, "rast", 0.8, Some(5)).unwrap();
println!("Found {} matches", results.len());
```

### Text Matching and Link Generation

```rust
use terraphim_automata::{load_thesaurus_from_json, replace_matches, LinkType};

let json = r#"{
  "name": "programming",
  "data": {
    "rust": {
      "id": 1,
      "nterm": "rust programming",
      "url": "https://rust-lang.org"
    }
  }
}"#;

let thesaurus = load_thesaurus_from_json(json).unwrap();
let text = "I love rust programming!";

// Replace with Markdown links
let linked = replace_matches(text, thesaurus.clone(), LinkType::MarkdownLinks).unwrap();
println!("{}", String::from_utf8(linked).unwrap());
// Output: "I love [rust](https://rust-lang.org) programming!"

// Or HTML links
let html = replace_matches(text, thesaurus.clone(), LinkType::HTMLLinks).unwrap();
// Output: 'I love <a href="https://rust-lang.org">rust</a> programming!'

// Or Wiki links
let wiki = replace_matches(text, thesaurus, LinkType::WikiLinks).unwrap();
// Output: "I love [[rust]] programming!"
```

### Loading Thesaurus Files

```rust
use terraphim_automata::{AutomataPath, load_thesaurus};

# #[cfg(feature = "remote-loading")]
# async fn example() {
// From local file
let local_path = AutomataPath::from_local("thesaurus.json");
let thesaurus = load_thesaurus(&local_path).await.unwrap();

// From remote URL
let remote_path = AutomataPath::from_remote("https://example.com/thesaurus.json").unwrap();
let thesaurus = load_thesaurus(&remote_path).await.unwrap();
# }
```

## Performance

- **Autocomplete**: ~1-2ms for 10,000+ terms
- **Fuzzy Search**: ~5-10ms with Jaro-Winkler
- **Text Matching**: O(n+m) with Aho-Corasick (n=text length, m=pattern count)
- **Memory**: ~100KB per 1,000 terms in FST

## WebAssembly Support

Build for the browser:

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
wasm-pack build --target web --features wasm

# Build for Node.js
wasm-pack build --target nodejs --features wasm
```

Use in JavaScript/TypeScript:

```typescript
import init, { build_autocomplete_index, fuzzy_autocomplete_search } from './pkg';

await init();

const thesaurus = {
  name: "programming",
  data: {
    "rust": { id: 1, nterm: "rust", url: null },
    "rust async": { id: 2, nterm: "rust async", url: null }
  }
};

const index = build_autocomplete_index(thesaurus, null);
const results = fuzzy_autocomplete_search(index, "rast", 0.8, 5);
console.log("Matches:", results);
```

See [wasm-test/](wasm-test/) for a complete example.

## Cargo Features

| Feature | Description |
|---------|-------------|
| `remote-loading` | Enable async HTTP loading of thesaurus files |
| `tokio-runtime` | Add tokio runtime support (required for `remote-loading`) |
| `typescript` | Generate TypeScript definitions via tsify |
| `wasm` | Enable WebAssembly compilation |

## API Overview

### Autocomplete Functions

- `build_autocomplete_index()` - Build FST index from thesaurus
- `autocomplete_search()` - Exact prefix matching
- `fuzzy_autocomplete_search()` - Fuzzy matching with Jaro-Winkler
- `fuzzy_autocomplete_search_levenshtein()` - Fuzzy matching with Levenshtein
- `serialize_autocomplete_index()` / `deserialize_autocomplete_index()` - Index serialization

### Text Matching Functions

- `find_matches()` - Find all pattern matches in text
- `replace_matches()` - Replace matches with links
- `extract_paragraphs_from_automata()` - Extract context around matches

### Thesaurus Loading

- `load_thesaurus()` - Load from file or URL (async)
- `load_thesaurus_from_json()` - Parse from JSON string (sync)

## Link Types

- **MarkdownLinks**: `[term](url)`
- **HTMLLinks**: `<a href="url">term</a>`
- **WikiLinks**: `[[term]]`

## Examples

See the [examples/](../../examples/) directory for:
- Complete autocomplete UI
- Knowledge graph linking
- WASM browser integration
- Custom thesaurus builders

## Minimum Supported Rust Version (MSRV)

This crate requires Rust 1.70 or later.

## License

Licensed under Apache-2.0. See [LICENSE](../../LICENSE-Apache-2.0) for details.

## Related Crates

- **[terraphim_types](../terraphim_types)**: Core type definitions
- **[terraphim_rolegraph](../terraphim_rolegraph)**: Knowledge graph implementation
- **[terraphim_service](../terraphim_service)**: Main service layer

## Support

- **Discord**: https://discord.gg/VPJXB6BGuY
- **Discourse**: https://terraphim.discourse.group
- **Issues**: https://github.com/terraphim/terraphim-ai/issues
