# FFF Search Integration -- Capability Statement

## Summary

Knowledge-graph-augmented file search exposed via the Model Context Protocol (MCP). Combines fuzzy matching, frecency scoring, and Aho-Corasick multi-pattern content search with domain-aware result ranking from the Terraphim knowledge graph.

## Capability Details

| Attribute | Detail |
|-----------|--------|
| **Domain** | File search, content search |
| **Interface** | MCP (Model Context Protocol) via `rmcp` 0.9 |
| **Language** | Rust |
| **Dependencies** | `fff-search` (Aho-Corasick, frecency), `terraphim_file_search` (KG scoring), `terraphim_automata` (thesaurus) |
| **Persistence** | LMDB via `heed` crate (frecency), filesystem JSON (thesaurus) |
| **Thread safety** | `Arc<RwLock<>>` for shared state, `parking_lot::RwLock` for hot-reload |

## Exposed MCP Tools

### `terraphim_find_files`
Fuzzy file path search with knowledge-graph boost.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | - | Fuzzy search string |
| `path` | string | No | `"."` | Root directory |
| `limit` | integer | No | 20 | Maximum results |

**Scoring**: Over-fetches 4x, adds KG path score, re-sorts by combined score.

### `terraphim_grep`
Single-pattern content search with KG-ordered files and cursor pagination.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | - | Search pattern |
| `path` | string | No | `"."` | Root directory |
| `limit` | integer | No | 50 | Maximum matches |
| `output_mode` | `"content"` or `"files"` | No | `"content"` | Output format |
| `cursor` | string | No | - | Base64 pagination token |

**Scoring**: Files pre-sorted by KG path score before searching.

### `terraphim_multi_grep`
Multi-pattern OR content search using SIMD-accelerated Aho-Corasick matching.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `patterns` | array of strings | Yes | - | OR-patterns |
| `path` | string | No | `"."` | Root directory |
| `constraints` | string | No | `""` | File constraints (`*.rs !test/`) |
| `limit` | integer | No | 50 | Maximum matches |
| `output_mode` | `"content"` or `"files"` | No | `"content"` | Output format |
| `cursor` | string | No | - | Base64 pagination token |

**Scoring**: Same KG pre-sort as `terraphim_grep`. Aho-Corasick matches all patterns in a single SIMD pass per file.

## Knowledge Graph Integration

### KgPathScorer
Implements `ExternalScorer` trait from `fff-search`. Scores file paths by running them through Aho-Corasick automata built from the domain thesaurus.

**Scoring formula**: `min(unique_concepts_matched * weight_per_term, max_boost)`

Default configuration:
- `weight_per_term`: 5
- `max_boost`: 30

### KgWatcher
Filesystem watcher for hot-reload of thesaurus JSON files.
- Debounce: 500ms
- Atomic swap via `parking_lot::RwLock`
- No restart required

## Frecency System

### Scoring Algorithm
Exponential decay combining frequency and recency:

```
frecency(file) = sum(exp(-lambda * days_ago)) for each access
```

With diminishing returns: `min(f, 10 + sqrt(max(0, f - 10)))` for `f > 10`.

### Decay Profiles

| Profile | Lambda | Half-life | Retention |
|---------|--------|-----------|-----------|
| Normal | ln(2)/10 | 10 days | 30 days |
| AI | ln(2)/3 | 3 days | 7 days |

### Storage
- LMDB environment: 24 MiB
- Keys: BLAKE3 hash of file path (32 bytes)
- Values: `VecDeque<u64>` (timestamped access log)
- GC: Background thread purges expired entries and compacts

### Configuration
Environment variable `FFF_FRECENCY_PATH` sets the LMDB database location.

## Pagination

Stateless cursor-based pagination using Base64-encoded file offsets.

- Cursor format: URL-safe Base64 (no padding) encoding of file index integer
- `next_cursor` returned in content when more results exist
- File-based granularity (not match-based)
- Ephemeral: invalidated by filesystem changes between pages

## Performance Characteristics

| Operation | Characteristic |
|-----------|---------------|
| KG path scoring | O(n) in path length per file, regardless of thesaurus size |
| Aho-Corasick multi-pattern | SIMD-accelerated single pass per file |
| Frecency lookup | O(1) LMDB read via BLAKE3 key |
| File scan | Synchronous per request, no caching between calls |
| Max file size | 10 MiB |
| Max matches per file | 200 |

## Implementation Completeness

| Component | Status |
|-----------|--------|
| MCP tool interface (3 tools) | Complete |
| KG path scoring | Complete |
| KG hot-reload | Complete |
| Aho-Corasick multi-pattern grep | Complete |
| Cursor pagination | Complete |
| Frecency persistence (LMDB) | Initialised |
| Frecency wired to FilePicker scoring | Pending |
| AI-mode frecency profile activation | Pending |

## Integration Points

- **MCP server**: `crates/terraphim_mcp_server/src/lib.rs`
- **KG scorer**: `crates/terraphim_file_search/src/kg_scorer.rs`
- **KG watcher**: `crates/terraphim_file_search/src/watcher.rs`
- **Scorer config**: `crates/terraphim_file_search/src/config.rs`
- **Tests**: `crates/terraphim_mcp_server/tests/test_find_files.rs`
- **Benchmarks**: `crates/terraphim_file_search/benches/kg_scoring.rs`
