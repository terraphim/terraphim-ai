# Summary: terraphim_automata/src/lib.rs

**Purpose:** Fast text matching and autocomplete engine for knowledge graphs using Aho-Corasick automata and finite state transducers (FST).

**Key Features:**
- **Fast Autocomplete**: Prefix-based search with fuzzy matching (Levenshtein/Jaro-Winkler algorithms)
- **Text Matching**: Multi-pattern matching via Aho-Corasick automaton for link generation
- **Link Generation**: Convert matched terms to Markdown, HTML, or Wiki links
- **Paragraph Extraction**: Extract context around matched terms
- **WASM Support**: Browser-compatible autocomplete with TypeScript bindings
- **Remote Loading**: Async loading of thesaurus files from HTTP (feature-gated)

**Architecture:**
- **AutocompleteIndex**: FST-based prefix search with metadata storage
- **AhoCorasick Matcher**: Leftmost-longest match kind for optimal term detection
- **Thesaurus Builder**: Parse knowledge graphs from JSON/Logseq Markdown formats

**Key Exports:**
- `build_autocomplete_index`, `fuzzy_autocomplete_search`, `load_thesaurus`
- `replace_matches` with `LinkType` enum (MarkdownLinks, HTMLLinks, WikiLinks)
- `extract_paragraphs_from_automata`, `find_matches`

**Cargo Features:**
- `remote-loading`: Enable async HTTP loading of thesaurus files
- `tokio-runtime`: Add tokio runtime support
- `typescript`: Generate TypeScript definitions via tsify
- `wasm`: Enable WebAssembly compilation

**Thesaurus Formats:**
- New format: `{"name": "...", "data": {"term": {"id": N, "nterm": "..."}, ...}}`
- Legacy format: Flat map of terms (backwards compatible)

**Error Handling:**
- `TerraphimAutomataError` enum with variants: InvalidThesaurus, Serde, Dict, Io, AhoCorasick, Fst
- Result type alias for ergonomic error propagation

**Usage:** Powers Terraphim's autocomplete in frontend, text matching in rolegraph queries, and knowledge graph linking in document preprocessing.