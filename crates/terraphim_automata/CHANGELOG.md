# Changelog

All notable changes to `terraphim_automata` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2025-01-22

### Added

#### Core Functionality
- **Autocomplete Index**: FST-based prefix search with O(log n) complexity
- **Fuzzy Search**: Jaro-Winkler and Levenshtein distance algorithms
- **Text Matching**: Aho-Corasick multi-pattern matching
- **Link Generation**: Convert matched terms to Markdown, HTML, or Wiki links
- **Paragraph Extraction**: Extract text context around matched terms

#### API Functions
- `build_autocomplete_index()` - Build FST index from thesaurus
- `autocomplete_search()` - Exact prefix matching
- `fuzzy_autocomplete_search()` - Fuzzy matching with Jaro-Winkler
- `fuzzy_autocomplete_search_levenshtein()` - Fuzzy matching with Levenshtein distance
- `find_matches()` - Multi-pattern text matching
- `replace_matches()` - Replace matches with links (Markdown/HTML/Wiki)
- `extract_paragraphs_from_automata()` - Context extraction around matches
- `serialize_autocomplete_index()` / `deserialize_autocomplete_index()` - Index persistence

#### Thesaurus Loading
- `load_thesaurus()` - Async loading from file or HTTP URL
- `load_thesaurus_from_json()` - Sync JSON parsing
- `load_thesaurus_from_json_and_replace()` - Combined load + replace operation
- `AutomataPath` enum for local/remote file handling

#### Types
- `AutocompleteIndex` - FST-based index with metadata
- `AutocompleteResult` - Search result with score
- `AutocompleteMetadata` - Term metadata (ID, URL, usage count)
- `AutocompleteConfig` - Index configuration
- `Matched` - Text match with position and metadata
- `LinkType` - Link format enum (MarkdownLinks, HTMLLinks, WikiLinks)
- `TerraphimAutomataError` - Comprehensive error types

#### Builders
- `ThesaurusBuilder` trait - Custom thesaurus parsers
- `Logseq` builder - Parse Logseq markdown files

### Features
- `remote-loading`: Enable async HTTP loading (requires tokio + reqwest)
- `tokio-runtime`: Tokio async runtime support
- `typescript`: TypeScript type generation via tsify
- `wasm`: WebAssembly compilation support

### Performance
- Sub-2ms autocomplete for 10,000+ terms
- O(n+m) text matching complexity
- ~100KB memory per 1,000 terms in FST
- Streaming text replacement for large documents

### Documentation
- Comprehensive module-level documentation with examples
- Rustdoc comments on all public functions and types
- Usage examples for:
  - Autocomplete with fuzzy matching
  - Text matching and link generation
  - Thesaurus loading (local and remote)
  - WASM browser integration
- README with quick start guide
- WASM example project in `wasm-test/`

### WASM Support
- Full browser compatibility
- TypeScript type definitions
- Example integration at `wasm-test/`
- Compatible with Chrome 57+, Firefox 52+, Safari 11+
- ~200KB compressed bundle size (release build)

### Implementation Details
- Aho-Corasick automata for fast multi-pattern matching
- FST (finite state transducer) for memory-efficient prefix search
- Cached fuzzy matching with `cached` crate
- Case-insensitive matching support
- Position tracking for context extraction
- Streaming replacement for memory efficiency

[Unreleased]: https://github.com/terraphim/terraphim-ai/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/terraphim/terraphim-ai/releases/tag/v1.0.0
