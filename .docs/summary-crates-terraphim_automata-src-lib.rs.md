# Summary: terraphim_automata/src/lib.rs

**Purpose:** Fast text matching and autocomplete engine using Aho-Corasick automata and FST.

**Key Details:**
- **Features:** `remote-loading` (async HTTP), `tokio-runtime`, `typescript`, `wasm`
- Provides:
  - FST-based autocomplete with fuzzy matching (Levenshtein/Jaro-Winkler)
  - Aho-Corasick multi-pattern text matching
  - Link generation (Markdown, HTML, Wiki)
  - Paragraph extraction around matched terms
  - WASM-compatible browser bindings
n- Key exports: `build_autocomplete_index`, `fuzzy_autocomplete_search`, `load_thesaurus`, `replace_matches`, `LinkType`
- Thesaurus builder supports JSON and Markdown (Logseq) formats
- Used by `terraphim_rolegraph` for term matching and by frontend for autocomplete
