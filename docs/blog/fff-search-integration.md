# FFF Search Integration: Knowledge-Graph-Augmented File Search for AI Agents

## The Problem

AI coding agents spend a disproportionate amount of time searching for files. Whether it is finding where a function is defined, which module handles authentication, or where a particular concept appears in the codebase, search is the gateway to everything else.

Existing tools fall into two camps:

1. **Fuzzy file finders** (fzf, fff.nvim) -- fast path matching, no understanding of what files *mean*
2. **Content search** (ripgrep, grep) -- thorough text matching, no concept of relevance ordering

Neither understands your domain. When you search for "authentication", neither knows that `crates/terraphim_auth/src/middleware.rs` is more relevant than a test fixture that happens to contain the word "authentication" four times.

## The Insight: Search Should Understand Your Domain

Terraphim's knowledge graph already knows your domain concepts and their relationships. The Aho-Corasick automata built from your thesaurus can match `ActorAuth`, `PopulatedActorAuth`, `actor_auth`, and `authentication_middleware` as variations of the same concept.

What if we could use that understanding to rank search results?

This is the core idea behind the FFF integration: **knowledge-graph-augmented file search**. Files whose *paths* match more domain concepts get boosted in the results. Not because they contain more keywords, but because they are structurally more relevant to your domain.

## Architecture

The integration spans three layers:

```
+---------------------------------------------+
|          MCP Tool Interface                  |
|  terraphim_find_files                        |
|  terraphim_grep                              |
|  terraphim_multi_grep                        |
+---------------------------------------------+
                    |
+---------------------------------------------+
|       Terraphim File Search                  |
|  KgPathScorer  |  KgWatcher (hot-reload)     |
+---------------------------------------------+
                    |
+---------------------------------------------+
|          fff-search core                     |
|  FilePicker  |  GrepEngine  |  Frecency     |
|  Aho-Corasick multi-pattern matching         |
+---------------------------------------------+
```

### ExternalScorer: The Extension Point

Rather than forking fff-search, we contributed the `ExternalScorer` trait upstream:

```rust
pub trait ExternalScorer: Send + Sync {
    fn score(&self, file: &FileItem) -> i32;
}
```

Any type implementing this trait can plug into fff-search's scoring pipeline. `KgPathScorer` implements it by running file paths through Aho-Corasick automata built from the knowledge graph thesaurus, counting unique concept matches, and returning a weighted boost.

### KG Scoring in Practice

For a file path like `crates/terraphim_auth/src/middleware.rs` with a thesaurus containing `auth`, `middleware`, and `terraphim`:

- Each matched concept adds `weight_per_term` (default: 5) to the score
- Total boost is capped at `max_boost` (default: 30)
- A file matching 6+ unique concepts still gets 30, preventing score explosion

The result: conceptually dense files (deeply embedded in your domain) float to the top.

### Two Scoring Strategies

Fuzzy search and content search use KG scoring differently:

**Fuzzy file search** (`terraphim_find_files`):
1. Over-fetch 4x the requested limit using fuzzy matching
2. Apply KG boost to each result: `combined_score = fuzzy_score + kg_boost`
3. Re-sort and truncate to requested limit

**Content search** (`terraphim_grep`, `terraphim_multi_grep`):
1. Pre-sort the entire file list by KG path score
2. Search files in that order (most relevant first)
3. Stop when the match limit is reached

The pre-sorting strategy is critical for pagination. The first page always contains matches from the most domain-relevant files. Subsequent pages get progressively less relevant, which is exactly the ordering users and AI agents expect.

## Multi-Pattern Search with Aho-Corasick

The `terraphim_multi_grep` tool solves a specific problem: searching for the same concept across naming conventions.

Consider searching for "authentication" in a Rust codebase. The concept might appear as:

- `ActorAuth` (PascalCase struct name)
- `PopulatedActorAuth` (prefixed variant)
- `actor_auth` (snake_case module name)
- `AUTH_MIDDLEWARE` (SCREAMING_SNAKE constant)

Rather than running four separate grep passes, `terraphim_multi_grep` builds a single Aho-Corasick automaton from all patterns and searches each file in one SIMD-accelerated pass:

```json
{
  "tool": "terraphim_multi_grep",
  "arguments": {
    "patterns": ["ActorAuth", "PopulatedActorAuth", "actor_auth"],
    "path": "crates/terraphim_auth",
    "constraints": "*.rs !test/"
  }
}
```

The `constraints` parameter supports glob patterns and negations: `*.rs` includes Rust files, `!test/` excludes test directories.

## Frecency: Frequency + Recency

FFF's frecency scoring combines how often a file is accessed with how recently, using exponential decay:

```
total_frecency = sum(exp(-decay_constant * days_ago)) for each access
```

Two decay profiles recognise different access patterns:

| Mode | Half-life | History Window | Designed for |
|------|-----------|----------------|--------------|
| Normal | 10 days | 30 days | Human interactive editing |
| AI | 3 days | 7 days | Rapid AI agent sessions |

AI agents touch far more files per session than humans. The shorter half-life prevents a single intensive session from dominating scores for weeks.

Frecency is persisted in LMDB (Lightning Memory-Mapped Database) with BLAKE3-hashed keys, supporting concurrent reads without blocking the search thread.

## Stateless Cursor Pagination

Content search results are paginated with stateless cursors -- no server-side HashMap, no Redis, no external state.

The cursor is a Base64-encoded file index:

```
next_cursor: MjM0  // decodes to "234" -- start searching from file 234
```

Design decisions:

- **File-based, not match-based**: A file with 200 matches does not consume the entire first page. The cursor tracks position in the file list.
- **Ephemeral**: If files are added or removed between pages, the cursor may skip or repeat files. This is accepted as expected behaviour for a code search tool.
- **Zero infrastructure**: No cursor store needed. The MCP server is stateless and horizontally scalable.

## Hot-Reloadable Knowledge Graph

The `KgWatcher` monitors a directory for thesaurus JSON files using filesystem events (debounced at 500ms). When the knowledge graph is updated:

1. All JSON files in the watch directory are reloaded
2. A new Aho-Corasick automaton is built from the merged thesaurus
3. The scorer atomically swaps the old automaton for the new one via `parking_lot::RwLock`

No restart required. Add a term to the thesaurus, save the file, and the next search uses it.

## What This Enables

For AI agents using the Model Context Protocol:

1. **Smaller context windows**: KG-ordered search means the most relevant files appear first, reducing the number of pages an agent needs to request.
2. **Multi-convention search**: A single `multi_grep` call replaces multiple `grep` invocations when searching across naming conventions.
3. **Domain-aware ranking**: Files in conceptually dense directories (like `crates/terraphim_auth/`) rank higher than incidental matches in unrelated code.
4. **Persistent learning**: Frecency tracking means files an agent accesses frequently become easier to find in future sessions.

For humans:

1. **Drop-in ripgrep replacement**: The MCP tools use familiar `grep`-like interfaces with `file:line:content` output format.
2. **No configuration tax**: KG scoring is additive. If no thesaurus is configured, results fall back to standard fuzzy/grep ordering.
3. **Live updates**: Edit the knowledge graph, save, search immediately benefits.

## Implementation Status

| Component | Status |
|-----------|--------|
| `terraphim_find_files` (KG-boosted fuzzy search) | Shipped |
| `terraphim_grep` (KG-ordered content search) | Shipped |
| `terraphim_multi_grep` (Aho-Corasick multi-pattern) | Shipped |
| KgPathScorer + hot-reload | Shipped |
| Cursor pagination | Shipped |
| SharedFrecency persistence | Initialised, scoring pipeline pending |
| AI-mode frecency decay profile | Implemented in fff-search, pending wiring |

The three MCP tools are available now. Frecency scoring integration into the FilePicker pipeline is the remaining work to complete the full vision.

## Try It

```json
{
  "tool": "terraphim_multi_grep",
  "arguments": {
    "patterns": ["CommandRegistry", "command_registry", "command-registry"],
    "path": "crates/terraphim_agent",
    "constraints": "*.rs !test/",
    "limit": 20
  }
}
```

The knowledge graph makes the difference between finding files and understanding your codebase.

---

*Terraphim AI is an open-source platform for knowledge-graph-augmented AI agent tooling, built in Rust with SvelteKit frontends and Tauri desktop integration.*
