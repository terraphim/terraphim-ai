# Summary: terraphim_types/src/lib.rs

**Purpose:** Core type definitions shared across all Terraphim crates.

**Key Types:**

| Type | Purpose |
|------|---------|
| `Thesaurus` | Mapping from NormalizedTermValue to NormalizedTerm |
| `NormalizedTerm` | Term with ID, value, display_value, URL, rank |
| `NormalizedTermValue` | Canonical term value (display text) |
| `Document` | Indexed document with body, title, tags, description |
| `IndexedDocument` | Search result with matched edges and ranking |
| `Node` | Graph node with connected edges and rank |
| `Edge` | Graph edge with document references |
| `RoleName` | Role identifier wrapper |
| `SearchQuery` | Search request with terms, operators, pagination |
| `RelevanceFunction` | Ranking algorithm enum |
| `LogicalOperator` | AND/OR operators for multi-term queries |
| `Layer` | Search result layer/category |
| `DocumentType` | Document classification (KgEntry, etc.) |

**Thesaurus:**
- Implements `IntoIterator` for iteration over terms
- Stores terms as HashMap<NormalizedTermValue, NormalizedTerm>
- Supports JSON serialization via serde

**SearchQuery:**
- `search_term`: Primary search term
- `search_terms`: Alternative multi-term format
- `operator`: LogicalOperator (AND/OR)
- `limit`/`skip`: Pagination
- `include_pinned`: Include pinned entries
- `layer`: Result filtering layer

**RelevanceFunction:**
- `TitleScorer`: Simple title matching
- `TerraphimGraph`: Knowledge graph-based scoring

**Document Structure:**
```rust
pub struct Document {
    pub id: String,
    pub url: String,
    pub title: String,
    pub body: String,
    pub description: Option<String>,
    pub summarization: Option<String>,
    pub stub: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub rank: Option<u64>,
    pub source_haystack: Option<String>,
    pub doc_type: DocumentType,
    pub synonyms: Option<String>,
    pub route: Option<String>,
    pub priority: Option<i32>,
}
```