# Design: Gitea #84 -- Trigger-Based Contextual KG Retrieval

## Context

Gitea issue #84 adds a `trigger::` field and `pinned::` field to KG entries, enabling contextual retrieval complementing existing `synonyms::` exact-match via Aho-Corasick. This is Phase 3 of the knowledge suggestion mechanism plan.

## Scope

Three changes across two crates:

1. **Parse `trigger::` and `pinned::` directives** from KG markdown files
2. **Build a simple TF-IDF index** over trigger descriptions at startup
3. **Two-pass search**: Aho-Corasick first, TF-IDF fallback when no matches found
4. **CLI: `--include-pinned` flag** and `kg list --pinned` command

## Architecture Decision: TF-IDF over BM25

BM25 lives in `terraphim_service/src/score/bm25.rs` (414 lines + tests). Importing it into `terraphim_rolegraph` would create a circular dependency (rolegraph -> service, but service already depends on rolegraph). Options:

- **(A) Extract BM25 to shared crate**: Too much refactoring for this ticket
- **(B) Standalone `bm25` crate from crates.io**: Adds external dependency
- **(C) Simple TF-IDF in rolegraph**: ~80 lines, no new dependencies, sufficient for short trigger descriptions (typically 5-20 words)

**Decision**: Option C. TF-IDF is adequate for matching query text against short trigger descriptions. BM25's length normalisation advantage is negligible when all documents are similarly short.

---

## Detailed Design

### 1. Extend `MarkdownDirectives` (terraphim_types)

**File**: `crates/terraphim_types/src/lib.rs`

Add two fields to `MarkdownDirectives`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownDirectives {
    #[serde(default)]
    pub doc_type: DocumentType,
    #[serde(default)]
    pub synonyms: Vec<String>,
    #[serde(default)]
    pub route: Option<RouteDirective>,
    #[serde(default)]
    pub priority: Option<u8>,
    // NEW:
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}
```

### 2. Parse `trigger::` and `pinned::` in markdown directives parser

**File**: `crates/terraphim_automata/src/markdown_directives.rs`

In `parse_markdown_directives_content()`, add two new branches after the existing `synonyms::` and `priority::` parsing:

```rust
// After existing priority:: parsing:

if lower.starts_with("trigger::") {
    if trigger.is_some() {
        continue; // First trigger wins, like other directives
    }
    let value = trimmed["trigger::".len()..].trim();
    if !value.is_empty() {
        trigger = Some(value.to_string());
    }
    continue;
}

if lower.starts_with("pinned::") {
    let value = trimmed["pinned::".len()..].trim().to_ascii_lowercase();
    pinned = matches!(value.as_str(), "true" | "yes" | "1");
    continue;
}
```

Update the return:

```rust
MarkdownDirectives {
    doc_type,
    synonyms,
    route,
    priority,
    trigger,
    pinned,
}
```

### 3. TF-IDF Trigger Index in RoleGraph

**File**: `crates/terraphim_rolegraph/src/lib.rs`

Add a lightweight TF-IDF index structure:

```rust
/// Simple TF-IDF index over trigger descriptions for semantic fallback search.
/// Used when Aho-Corasick finds no exact synonym matches.
#[derive(Debug, Clone, Default)]
pub struct TriggerIndex {
    /// Map from node_id to its trigger description tokens (lowercased)
    triggers: AHashMap<u64, Vec<String>>,
    /// Inverse document frequency for each token
    idf: AHashMap<String, f64>,
    /// Total number of documents with triggers
    doc_count: usize,
    /// Configurable relevance threshold (0.0-1.0)
    threshold: f64,
}

impl TriggerIndex {
    pub fn new(threshold: f64) -> Self {
        Self {
            triggers: AHashMap::new(),
            idf: AHashMap::new(),
            doc_count: 0,
            threshold,
        }
    }

    /// Build the index from a map of node_id -> trigger description
    pub fn build(&mut self, triggers: AHashMap<u64, String>) {
        self.triggers.clear();
        self.idf.clear();
        self.doc_count = triggers.len();

        // Tokenise each trigger
        let mut doc_freq: AHashMap<String, usize> = AHashMap::new();
        for (node_id, trigger_text) in &triggers {
            let tokens: Vec<String> = Self::tokenise(trigger_text);
            // Count unique tokens per document for DF
            let unique: ahash::AHashSet<&str> = tokens.iter().map(|s| s.as_str()).collect();
            for token in &unique {
                *doc_freq.entry(token.to_string()).or_insert(0) += 1;
            }
            self.triggers.insert(*node_id, tokens);
        }

        // Compute IDF: log((N + 1) / (df + 1)) + 1 (smoothed)
        let n = self.doc_count as f64;
        for (token, df) in &doc_freq {
            let idf = ((n + 1.0) / (*df as f64 + 1.0)).ln() + 1.0;
            self.idf.insert(token.clone(), idf);
        }
    }

    /// Query the index, returning node_ids with scores above threshold
    pub fn query(&self, text: &str) -> Vec<(u64, f64)> {
        if self.triggers.is_empty() {
            return vec![];
        }

        let query_tokens = Self::tokenise(text);
        if query_tokens.is_empty() {
            return vec![];
        }

        // Compute TF-IDF cosine similarity between query and each trigger
        let mut results: Vec<(u64, f64)> = Vec::new();

        // Query TF-IDF vector
        let mut query_tfidf: AHashMap<&str, f64> = AHashMap::new();
        for token in &query_tokens {
            let tf = 1.0; // Binary TF for query
            let idf = self.idf.get(token.as_str()).copied().unwrap_or(0.0);
            *query_tfidf.entry(token.as_str()).or_insert(0.0) += tf * idf;
        }
        let query_norm: f64 = query_tfidf.values().map(|v| v * v).sum::<f64>().sqrt();
        if query_norm == 0.0 {
            return vec![];
        }

        for (node_id, trigger_tokens) in &self.triggers {
            // Document TF-IDF vector
            let mut doc_tfidf: AHashMap<&str, f64> = AHashMap::new();
            for token in trigger_tokens {
                let tf = 1.0; // Binary TF for short descriptions
                let idf = self.idf.get(token.as_str()).copied().unwrap_or(0.0);
                *doc_tfidf.entry(token.as_str()).or_insert(0.0) += tf * idf;
            }
            let doc_norm: f64 = doc_tfidf.values().map(|v| v * v).sum::<f64>().sqrt();
            if doc_norm == 0.0 {
                continue;
            }

            // Cosine similarity
            let dot: f64 = query_tfidf
                .iter()
                .map(|(token, q_val)| {
                    let d_val = doc_tfidf.get(token).copied().unwrap_or(0.0);
                    q_val * d_val
                })
                .sum();
            let similarity = dot / (query_norm * doc_norm);

            if similarity >= self.threshold {
                results.push((*node_id, similarity));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Simple whitespace tokeniser with lowercasing and stopword removal
    fn tokenise(text: &str) -> Vec<String> {
        text.to_ascii_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2) // Skip very short words
            .filter(|w| !Self::is_stopword(w))
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| !w.is_empty())
            .collect()
    }

    fn is_stopword(word: &str) -> bool {
        matches!(
            word,
            "the" | "and" | "for" | "are" | "but" | "not" | "you"
            | "all" | "can" | "her" | "was" | "one" | "our" | "out"
            | "has" | "have" | "been" | "this" | "that" | "with"
            | "when" | "from" | "into" | "which" | "their" | "will"
        )
    }

    pub fn is_empty(&self) -> bool {
        self.triggers.is_empty()
    }
}
```

### 4. Integrate TriggerIndex into RoleGraph

**File**: `crates/terraphim_rolegraph/src/lib.rs`

Add field to `RoleGraph`:

```rust
pub struct RoleGraph {
    // ... existing fields ...
    /// TF-IDF index over trigger descriptions (semantic fallback)
    trigger_index: TriggerIndex,
    /// Node IDs that are pinned (always included in results)
    pinned_node_ids: Vec<u64>,
}
```

Also add to `SerializableRoleGraph`:

```rust
pub struct SerializableRoleGraph {
    // ... existing fields ...
    pub trigger_descriptions: AHashMap<u64, String>,
    pub pinned_node_ids: Vec<u64>,
}
```

Update `new_sync()` to initialise empty trigger index and pinned list.

### 5. New method: `find_matching_node_ids_with_fallback()`

**File**: `crates/terraphim_rolegraph/src/lib.rs`

```rust
/// Two-pass search: Aho-Corasick first, TF-IDF trigger fallback if no matches.
/// Pinned node IDs are always included when include_pinned is true.
pub fn find_matching_node_ids_with_fallback(
    &self,
    text: &str,
    include_pinned: bool,
) -> Vec<u64> {
    let mut results = self.find_matching_node_ids(text);

    // Pass 2: TF-IDF fallback when Aho-Corasick found nothing
    if results.is_empty() && !self.trigger_index.is_empty() {
        let trigger_matches = self.trigger_index.query(text);
        results.extend(trigger_matches.iter().map(|(id, _score)| *id));
    }

    // Always include pinned entries
    if include_pinned {
        for &pinned_id in &self.pinned_node_ids {
            if !results.contains(&pinned_id) {
                results.push(pinned_id);
            }
        }
    }

    results
}

/// Populate trigger index from parsed markdown directives.
/// Call after loading KG entries and building the thesaurus.
pub fn load_trigger_index(
    &mut self,
    triggers: AHashMap<u64, String>,
    pinned: Vec<u64>,
    threshold: f64,
) {
    let mut index = TriggerIndex::new(threshold);
    index.build(triggers);
    self.trigger_index = index;
    self.pinned_node_ids = pinned;
}
```

### 6. Wire into query methods

Update `query_graph()` and `query_graph_with_operators()` to optionally use the fallback path. Add `include_pinned` parameter (default false for backward compatibility):

```rust
pub fn query_graph_with_trigger_fallback(
    &self,
    query_string: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    include_pinned: bool,
) -> Result<Vec<(String, IndexedDocument)>> {
    let node_ids = self.find_matching_node_ids_with_fallback(query_string, include_pinned);
    // ... rest follows existing query_graph logic ...
}
```

### 7. CLI changes (terraphim_agent)

**File**: `crates/terraphim_agent/src/main.rs`

Add `--include-pinned` flag to the `search` subcommand and a `kg list --pinned` command:

```rust
// In SearchSub or equivalent:
#[clap(long, help = "Include pinned KG entries in results")]
include_pinned: bool,

// New KG subcommand:
#[clap(subcommand)]
Kg(KgSub),

#[derive(Debug, Subcommand)]
enum KgSub {
    /// List KG entries
    List {
        #[clap(long, help = "Show only pinned entries")]
        pinned: bool,
    },
}
```

---

## Test Cases

### Unit tests (in terraphim_automata)

1. `parses_trigger_directive` -- `trigger:: when managing dependencies` produces `trigger: Some("when managing dependencies")`
2. `parses_pinned_directive` -- `pinned:: true` produces `pinned: true`
3. `pinned_false_variants` -- `pinned:: false`, `pinned:: no`, `pinned:: 0` all produce `pinned: false`
4. `trigger_and_synonyms_coexist` -- Both `synonyms::` and `trigger::` in same file parse correctly
5. `empty_trigger_ignored` -- `trigger::` with no value produces `trigger: None`

### Unit tests (in terraphim_rolegraph, TriggerIndex)

6. `tfidf_empty_index_returns_empty` -- Query on empty index returns `vec![]`
7. `tfidf_exact_match_scores_high` -- Query matching all trigger tokens scores above threshold
8. `tfidf_no_match_scores_zero` -- Completely unrelated query returns empty
9. `tfidf_partial_match` -- Query sharing some tokens with trigger scores between 0 and 1
10. `tfidf_threshold_filters` -- Matches below threshold are excluded

### Integration tests (in terraphim_rolegraph)

11. `two_pass_aho_corasick_first` -- When Aho-Corasick finds matches, trigger index is not consulted
12. `two_pass_fallback_to_trigger` -- When Aho-Corasick finds nothing, trigger index returns results
13. `pinned_always_included` -- Pinned entries appear in results even when no match
14. `serializable_roundtrip_preserves_triggers` -- Serialise and deserialise preserves trigger data

---

## Files Changed

| File | Action | Lines (est.) |
|------|--------|------|
| `crates/terraphim_types/src/lib.rs` | MODIFY | +6 |
| `crates/terraphim_automata/src/markdown_directives.rs` | MODIFY | +40 (parsing + 5 tests) |
| `crates/terraphim_rolegraph/src/lib.rs` | MODIFY | +250 (TriggerIndex + integration + 8 tests) |
| `crates/terraphim_agent/src/main.rs` | MODIFY | +40 (CLI flags) |

**Total**: ~336 lines added

## Acceptance Criteria

1. `cargo test -p terraphim_automata` -- all existing + 5 new directive parsing tests pass
2. `cargo test -p terraphim_rolegraph` -- all existing + 8 new trigger/TF-IDF tests pass
3. `cargo clippy -p terraphim_rolegraph -p terraphim_automata -p terraphim_types` -- no warnings
4. KG markdown files with `trigger::` and `pinned::` fields are correctly parsed
5. Search falls back to trigger matching only when Aho-Corasick returns empty results
6. Pinned entries appear in results when `--include-pinned` is used
7. Backward compatible: existing KG files without trigger/pinned continue to work identically
