# Implementation Plan: Replace RipgrepIndexer with FffIndexer in terraphim_middleware

**Status**: Draft  
**Research Doc**: `.docs/research-1873-fffindexer-migration.md`  
**Author**: AI Agent (Phase 2 Disciplined Design)  
**Date**: 2026-05-25  
**Estimated Effort**: 8-10 hours  
**Issue**: #1873

---

## Overview

### Summary

This plan replaces the `RipgrepIndexer` in `terraphim_middleware` with a new `FffIndexer` that uses the native `fff-search` Rust library instead of the external `ripgrep` (`rg`) binary. The new indexer will:

1. Eliminate the runtime dependency on the external `rg` binary
2. Preserve all existing functionality (markdown indexing, caching, `update_document` write-back)
3. Maintain API parity with `RipgrepIndexer` so `search_haystacks` requires zero changes
4. Lay the groundwork for future KG scoring integration via an optional `ExternalScorer` field

### Approach

**Chosen approach**: Additive migration with builder-pattern extensibility.

- Create `FffIndexer` in a new file `src/indexer/fff.rs`
- Implement `IndexMiddleware` for `FffIndexer` using `FilePicker::grep` from `fff-search`
- Replicate document field construction exactly (id, title, url, body, description)
- Preserve `update_document` write-back for desktop UI compatibility
- Use `cached` macro with equivalent cache key format
- Keep `RipgrepIndexer` and `RipgrepCommand` in place (separate cleanup issue)
- Add optional `kg_scorer` field on `FffIndexer` via builder pattern (default `None`)

**Rejected approaches**:
- Direct deletion of `RipgrepIndexer` before validation
- Extending `IndexMiddleware` trait with `ConfigState` parameter (would force 7+ indexer changes)
- Using crates.io `fff-search` 0.8.2 (would split versions across workspace)

### Scope

**In Scope:**
1. New `FffIndexer` struct implementing `IndexMiddleware`
2. `cached` wrapper with equivalent cache key semantics
3. Document construction replicating RipgrepIndexer output
4. `update_document` write-back method
5. Markdown-only filtering (parity with current default)
6. `extra_parameters` support for `glob`, `type`, `max_count`, `context`, `case_sensitive`
7. Integration tests mirroring `tests/ripgrep.rs`
8. `search_haystacks` dispatcher switch to `FffIndexer`

**Out of Scope:**
1. KG path scoring in initial implementation (follow-up issue)
2. Frecency scoring (follow-up issue)
3. Cursor pagination (requires `SearchQuery` changes)
4. Deletion of `RipgrepIndexer` / `RipgrepCommand` (separate cleanup)
5. Non-markdown file support as default (opt-in via `extra_parameters`)

**Avoid At All Cost** (from 5/25 analysis):
1. **Extending `IndexMiddleware` trait** — would cascade to 7+ haystack indexers; violates "minimal intrusions" principle
2. **KG scoring in initial migration** — requires `Thesaurus` access which the trait cannot provide; would delay the core migration by weeks
3. **Frecency integration** — requires `FFF_FRECENCY_PATH` env plumbing and `SharedFrecency` lifecycle management; pure distraction from core goal
4. **Non-markdown default** — changes existing semantics; users expect markdown-only for `ServiceType::Ripgrep`
5. **Deleting `RipgrepCommand` before validation** — removes rollback option; violates additive-then-subtractive principle

---

## Architecture

### Component Diagram

```
+------------------------------------------------------------------+
|                    terraphim_middleware                            |
|                                                                    |
|  +-------------------+      +-----------------------+              |
|  | search_haystacks  |----->| FffIndexer            |              |
|  | (dispatcher)      |      | - index()             |              |
|  +-------------------+      | - update_document()   |              |
|           |                 +-----------------------+              |
|           |                          |                             |
|           |                    cached_fff_index()                  |
|           |                          |                             |
|           v                          v                             |
|  +-------------------+      +-----------------------+              |
|  | ConfigState       |<-----| FilePicker (fff-search)|             |
|  | - add_to_roles()  |      | - collect_files()      |             |
|  +-------------------+      | - grep()               |             |
|                             +-----------------------+              |
|                                       |                            |
|                                       v                            |
|                             +-----------------------+              |
|                             | Document construction |              |
|                             | - id (normalized)     |              |
|                             | - title (file stem)   |              |
|                             | - url (absolute path) |              |
|                             | - body (file contents)|              |
|                             | - description (match) |              |
|                             +-----------------------+              |
|                                       |                            |
|                                       v                            |
|                             +-----------------------+              |
|                             | Index (AHashMap)      |              |
|                             +-----------------------+              |
+------------------------------------------------------------------+
                              |
                              v
+------------------------------------------------------------------+
|  terraphim_service::create_document()                            |
|  -> FffIndexer::update_document() -> fs::write()                 |
+------------------------------------------------------------------+
```

### Data Flow

```
[SearchQuery] -> [search_haystacks] -> [FffIndexer::index]
                                            |
                                            v
                                    [cached_fff_index]
                                            |
                                            v
                                    [FilePicker::new]
                                            |
                                            v
                                    [collect_files()] -> Vec<FileItem>
                                            |
                                            v
                                    [Filter: .md extension]
                                            |
                                            v
                                    [parse_grep_query(needle)]
                                            |
                                            v
                                    [picker.grep(query, options)]
                                            |
                                            v
                                    [For each match:]
                                    [  read file body ]
                                    [  build Document ]
                                    [  insert into Index ]
                                            |
                                            v
                                    [Return Index]
                                            |
                                            v
                                    [search_haystacks tags source_haystack]
                                            |
                                            v
                                    [config_state.add_to_roles()]
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| **Use Git branch `feat/external-scorer` for `fff-search`** | Aligns with `terraphim_mcp_server` and `terraphim_file_search`; keeps door open for `KgPathScorer` integration without version split. | crates.io 0.8.2 (would create version mismatch in workspace) |
| **Keep `IndexMiddleware` trait unchanged** | Trait is used by 7+ indexers. Changing it forces cascading changes across entire middleware layer. | Extend trait with `ConfigState` parameter (cascading impact too high) |
| **Add `kg_scorer: Option<Arc<KgPathScorer>>` as builder field on `FffIndexer`** | Keeps trait clean while enabling future KG scoring. Follows same pattern as `McpService::with_kg_scorer()`. | Pass scorer through `index()` method (breaks trait contract) |
| **Implement markdown-only filtering client-side** | `fff-search` does not natively support `-tmarkdown`. Filter `FileItem` results by extension after `collect_files()`. | Use ripgrep for filtering (defeats purpose of migration) |
| **Cache key includes `extra_parameters` + `needle` + `location`** | Same inputs as RipgrepIndexer cache. Ensures cache invalidation when haystack config changes. | Hash-based key (harder to debug) |
| **Replicate document `id` format exactly** | Existing documents in persistence use `ripgrep_{path}` normalised format. Use `fff_{path}` for new indexer to avoid collisions. | Keep `ripgrep_` prefix (misleading, implies ripgrep still used) |
| **Read full file body via `tokio::fs::read_to_string`** | Parity with RipgrepIndexer behaviour. Simpler than streaming. | Memory-map files (over-optimisation for markdown files) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Extend `IndexMiddleware` with `ConfigState` parameter | Would force changes to QueryRs, ClickUp, MCP, Perplexity, Quickwit, Jmap, GrepApp indexers | 7+ files changed, weeks of validation, high regression risk |
| KG scoring in initial implementation | `IndexMiddleware::index()` cannot receive `Thesaurus`; requires trait redesign | Blocks core migration for speculative feature; trait redesign is separate architectural decision |
| Frecency scoring | Requires `FFF_FRECENCY_PATH` env var, `SharedFrecency` initialisation, persistence | Adds operational complexity; no validated user need |
| Cursor pagination | Requires `SearchQuery` struct changes and API versioning | Out of scope for indexer replacement; affects entire search pipeline |
| Non-markdown default | Current semantics are markdown-only; changing default surprises users | Breaks existing workflows; opt-in via `extra_parameters` is safer |
| Delete `RipgrepCommand` module | Removes rollback capability; module may be referenced elsewhere | Cannot revert if `FffIndexer` has issues in production |
| Use `grep_search()` instead of `FilePicker::grep()` | `grep_search()` is crate-private in fff-search 0.8.2 | API access issue; `FilePicker::grep()` is the public API |

### Simplicity Check

> "What if this could be easy?"

The simplest possible design: a single `FffIndexer::index()` method that creates a `FilePicker`, collects markdown files, runs `grep`, and builds documents. No trait changes. No KG scorer wiring. No frecency. Just file-in, documents-out.

Our design stays at this simplicity level. The only "complexity" added is:
1. A builder-pattern `kg_scorer` field (optional, default None) — this is future-proofing, not current functionality
2. Client-side markdown filtering — necessary because `fff-search` lacks native file-type filtering

**Senior Engineer Test**: A senior engineer would call this appropriately simple. The indexer does one thing: turn a filesystem haystack into an `Index`. All other concerns (scoring, pagination, frecency) are layered on later.

### Nothing Speculative Checklist

- [x] No features the user didn't request — KG scoring and frecency are explicitly out of scope
- [x] No abstractions "in case we need them later" — `kg_scorer` is a concrete type, not a generic trait
- [x] No flexibility "just in case" — markdown-only is the hardcoded default (same as ripgrep)
- [x] No error handling for scenarios that cannot occur — `FilePicker` init errors are real and handled
- [x] No premature optimisation — full file read, not memory mapping; simplicity over performance

---

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_middleware/src/indexer/fff.rs` | `FffIndexer` struct and `IndexMiddleware` implementation |
| `crates/terraphim_middleware/tests/fff_indexer.rs` | Integration tests mirroring `tests/ripgrep.rs` |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_middleware/Cargo.toml` | Add `fff-search` dependency from Git branch `feat/external-scorer` |
| `crates/terraphim_middleware/src/indexer/mod.rs` | Add `mod fff;`, change `search_haystacks` to use `FffIndexer` instead of `RipgrepIndexer`, add `pub use fff::FffIndexer;` |
| `crates/terraphim_middleware/src/lib.rs` | Add `pub use indexer::FffIndexer;` (if re-exporting) |

### Deleted Files

| File | Reason |
|------|--------|
| None | RipgrepIndexer kept as fallback until validation complete |

---

## API Design

### FffIndexer Struct Definition

```rust
use std::path::Path;
use std::sync::Arc;

use fff_search::{FFFMode, FilePicker, FilePickerOptions, GrepMode, GrepSearchOptions, parse_grep_query};
use terraphim_config::Haystack;
use terraphim_persistence::Persistable;
use terraphim_types::{Document, DocumentType, Index};
use tokio::fs;

use crate::{Error, Result};
use super::IndexMiddleware;

/// Optional knowledge-graph path scorer for future KG-boosted ordering.
/// Requires the `kg-integration` feature or manual construction.
#[cfg(feature = "kg-integration")]
use terraphim_file_search::KgPathScorer;

/// Middleware that uses fff-search to index Markdown haystacks.
///
/// Replaces `RipgrepIndexer` with a pure-Rust implementation that does
/// not require the external `rg` binary.
#[derive(Default)]
pub struct FffIndexer {
    /// Optional KG path scorer for boosting results by knowledge-graph
    /// concept matches. When `None`, no KG boosting is applied.
    ///
    /// This is a forward-compatibility field; the initial migration
    /// leaves it as `None`.
    #[cfg(feature = "kg-integration")]
    kg_scorer: Option<Arc<KgPathScorer>>,
}

impl FffIndexer {
    /// Create a new `FffIndexer` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a knowledge-graph path scorer for future use.
    ///
    /// This follows the same builder pattern as `McpService::with_kg_scorer()`.
    #[cfg(feature = "kg-integration")]
    pub fn with_kg_scorer(mut self, scorer: Arc<KgPathScorer>) -> Self {
        self.kg_scorer = Some(scorer);
        self
    }
}
```

### IndexMiddleware Trait Implementation

```rust
impl IndexMiddleware for FffIndexer {
    /// Index the haystack using fff-search and return an index of documents.
    ///
    /// # Errors
    ///
    /// Returns an error if the haystack path does not exist, `FilePicker`
    /// initialisation fails, or file I/O errors occur during document
    /// construction.
    async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        cached_fff_index(needle, haystack).await
    }
}
```

### Cached Wrapper

```rust
use cached::proc_macro::cached;

/// Cached wrapper that performs fff-search indexing for a given haystack/query.
///
/// Cache key includes all inputs that affect output:
/// - `haystack.location`: filesystem path being indexed
/// - `needle`: search term
/// - `haystack.get_extra_parameters()`: extra filtering/options
///
/// Cache size: 64 entries (same as RipgrepIndexer).
#[cached(
    result = true,
    size = 64,
    key = "String",
    convert = r#"{ format!("{}::{}::{:?}", haystack.location, needle, haystack.get_extra_parameters()) }"#
)]
async fn cached_fff_index(needle: &str, haystack: &Haystack) -> Result<Index> {
    let indexer = FffIndexer::default();
    indexer.index_inner(needle, haystack).await
}
```

### Core Indexing Logic (index_inner)

```rust
impl FffIndexer {
    async fn index_inner(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        let haystack_path = Path::new(&haystack.location);

        // Validate path exists
        if !haystack_path.exists() {
            log::warn!("Haystack path does not exist: {:?}", haystack_path);
            return Ok(Index::default());
        }

        // Initialise FilePicker
        let mut picker = FilePicker::new(FilePickerOptions {
            base_path: haystack.location.clone(),
            mode: FFFMode::Ai,
            watch: false,
            cache_budget: None,
            ..FilePickerOptions::default()
        })
        .map_err(|e| Error::Indexer(format!("FilePicker init failed: {e}")))?;

        picker
            .collect_files()
            .map_err(|e| Error::Indexer(format!("File scan failed: {e}")))?;

        // Filter to markdown files only (parity with ripgrep -tmarkdown)
        let files: Vec<_> = picker
            .get_files()
            .iter()
            .filter(|f| f.path.extension().map_or(false, |ext| ext == "md"))
            .cloned()
            .collect();

        log::debug!("Found {} markdown files in haystack: {:?}", files.len(), haystack_path);

        // Apply extra_parameters filtering (glob, type) client-side if needed
        let files = apply_extra_parameters_filter(files, haystack.get_extra_parameters());

        // Parse query and run grep
        let fff_query = parse_grep_query(needle);
        let options = GrepSearchOptions {
            max_file_size: 10 * 1024 * 1024,
            max_matches_per_file: 200,
            smart_case: true,
            file_offset: 0,
            page_limit: None,
            mode: GrepMode::PlainText,
            ..GrepSearchOptions::default()
        };

        let result = picker.grep(&fff_query, &options);

        // Build documents from matches
        let mut index = Index::default();
        let mut seen_paths = std::collections::HashSet::new();

        for m in result.matches {
            let file = match result.files.get(m.file_index) {
                Some(f) => f,
                None => continue,
            };

            let path = &file.path;

            // Skip non-markdown files (defensive; should already be filtered)
            if path.extension().map_or(true, |ext| ext != "md") {
                continue;
            }

            // Skip duplicates
            if !seen_paths.insert(path.clone()) {
                continue;
            }

            // Build document
            let body = match fs::read_to_string(path).await {
                Ok(b) => b,
                Err(e) => {
                    log::warn!("Failed to read file: {} - {:?}", path.display(), e);
                    continue;
                }
            };

            let id = Self::normalize_document_id(&path.to_string_lossy());
            let title = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let description = if m.line_content.len() > 200 {
                let safe_end = floor_char_boundary(&m.line_content, 197);
                format!("{}...", &m.line_content[..safe_end])
            } else {
                m.line_content.clone()
            };

            let document = Document {
                id,
                title,
                url: path.to_string_lossy().to_string(),
                body,
                description: Some(description),
                summarization: None,
                stub: None,
                tags: None,
                rank: None,
                source_haystack: None, // set by search_haystacks dispatcher
                doc_type: DocumentType::KgEntry,
                synonyms: None,
                route: None,
                priority: None,
                quality_score: None,
            };

            index.insert(document.id.clone(), document);
        }

        log::debug!("FffIndexer built index with {} documents", index.len());
        Ok(index)
    }
}
```

### Public Methods

```rust
impl FffIndexer {
    /// Update the underlying Markdown file on disk with the edited document body.
    ///
    /// The `Document.url` field is expected to hold an absolute or haystack-relative
    /// path to the original file. When haystacks are marked as read-only this
    /// method SHOULD NOT be called.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub async fn update_document(&self, document: &Document) -> Result<()> {
        let path = Path::new(&document.url);

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                log::warn!("Parent directory does not exist for {:?}", path);
            }
        }

        let mut content = document.body.clone();
        // Heuristically detect HTML (presence of tags). If HTML detected, convert to Markdown.
        if content.contains('<') && content.contains('>') {
            log::debug!("Converting HTML content to Markdown for file {:?}", path);
            content = html2md::parse_html(&content);
        }

        log::info!("Writing updated document back to markdown file: {:?}", path);
        fs::write(path, content).await?;
        Ok(())
    }

    /// Normalise document ID to match persistence layer expectations.
    fn normalize_document_id(&self, file_path: &str) -> String {
        let dummy_doc = Document {
            id: "dummy".to_string(),
            title: "dummy".to_string(),
            body: "dummy".to_string(),
            url: "dummy".to_string(),
            description: None,
            summarization: None,
            stub: None,
            tags: None,
            rank: None,
            source_haystack: None,
            doc_type: DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
            quality_score: None,
        };
        let original_id = format!("fff_{}", file_path);
        dummy_doc.normalize_key(&original_id)
    }
}
```

### Error Types

No new error types required. `FffIndexer` uses the existing `crate::Error` enum. If `fff-search` returns errors, they are mapped to `Error::Indexer(String)`.

Current `Error` type in `terraphim_middleware` (assumed from usage):
```rust
pub enum Error {
    // ... existing variants ...
    #[error("Indexer error: {0}")]
    Indexer(String),
}
```

If `Error::Indexer` does not exist, add it as a minimal change:
```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    // ... existing variants ...
    #[error("Indexer error: {0}")]
    Indexer(String),
}
```

### Caching Strategy

**Cache key format** (identical semantics to RipgrepIndexer):
```rust
format!("{}::{}::{:?}", haystack.location, needle, haystack.get_extra_parameters())
```

This ensures:
1. Same haystack + needle + params = cache hit
2. Any change to `extra_parameters` = cache miss
3. Same cache size (64 entries) as RipgrepIndexer

**Cache invalidation**: Automatic via `cached` crate LRU eviction. No manual invalidation needed for initial migration.

---

## Test Strategy

### Unit Tests (in `src/indexer/fff.rs`)

| Test | Purpose |
|------|---------|
| `test_normalize_document_id` | Verify `fff_{path}` format with `normalize_key` |
| `test_update_document_html_conversion` | Verify HTML-to-Markdown conversion in `update_document` |
| `test_update_document_plain_markdown` | Verify plain markdown write-back |

### Integration Tests (in `tests/fff_indexer.rs`)

| Test | Mirrors | Purpose |
|------|---------|---------|
| `test_fff_indexer_basic` | `test_indexer` | Basic indexing with "test" needle |
| `test_fff_search_graph` | `test_search_graph` | Search for "graph" term |
| `test_fff_search_machine_learning` | `test_search_machine_learning` | Search for "graph", print documents |
| `test_fff_role_configuration` | `test_role_configuration` | Verify role/haystack config |

### Test Fixtures

Reuse existing fixtures at `terraphim_server/fixtures/haystack`:
- `@Intelligent safe operation and maintenance of OGPS.md`
- `machine_learning.md`
- `neural_networks.md`
- `System Operator.md`
- `terraphim.md`
- `testconcept.md`
- And 12 other markdown files

Test haystack configuration:
```rust
let haystack = Haystack {
    location: "terraphim_server/fixtures/haystack".to_string(),
    service: ServiceType::Ripgrep, // keep variant for backward compat
    read_only: true,
    fetch_content: false,
    atomic_server_secret: None,
    extra_parameters: std::collections::HashMap::new(),
};
```

### Parity Verification

To verify `FffIndexer` produces equivalent output to `RipgrepIndexer`:

1. **Document count parity**: Assert `fff_index.len() == ripgrep_index.len()` for same needle/haystack
2. **Document field parity**: For each document, assert:
   - `title` matches (file stem)
   - `url` matches (path string)
   - `body` matches (file contents)
   - `description` is non-empty (exact match may differ due to match text extraction differences)
3. **ID format parity**: Assert IDs follow `normalize_key` pattern
4. **Cache behaviour parity**: Run same query twice, verify second is faster (cached)

**Expected differences** (documented and acceptable):
- `id` prefix: `ripgrep_` vs `fff_` — intentional to avoid persistence collisions
- `description` content may differ slightly: ripgrep extracts from `-C3` context; fff-search extracts from match line
- Match ordering may differ: ripgrep orders by file path then match position; fff-search orders by its own ranking

### Extra Parameters Tests

| Test | Purpose |
|------|---------|
| `test_fff_extra_parameters_glob` | Verify `glob: "*.md"` filters correctly |
| `test_fff_extra_parameters_type` | Verify `type: "markdown"` filters correctly |
| `test_fff_extra_parameters_case_sensitive` | Verify `case_sensitive: "true"` affects search |

---

## Implementation Steps

### Step 1: Add fff-search dependency to Cargo.toml

**Files:** `crates/terraphim_middleware/Cargo.toml`
**Description:** Add `fff-search` from Git branch `feat/external-scorer` with `zlob` feature (aligns with `terraphim_mcp_server` and `terraphim_file_search`).
**Tests:** `cargo check -p terraphim_middleware` compiles without errors.
**Dependencies:** None.
**Estimated:** 15 minutes.

```toml
# Add to [dependencies]
fff-search = { git = "https://github.com/AlexMikhalev/fff.nvim.git", branch = "feat/external-scorer", features = ["zlob"] }
```

### Step 2: Create FffIndexer struct and basic index() method

**Files:** `crates/terraphim_middleware/src/indexer/fff.rs`
**Description:** Create the module file with `FffIndexer` struct, `IndexMiddleware` impl, `cached_fff_index` wrapper, and stub `index_inner` that returns empty `Index`.
**Tests:** Compile check. Add `mod fff;` to `indexer/mod.rs` and verify build.
**Dependencies:** Step 1.
**Estimated:** 1 hour.

Key code:
```rust
// src/indexer/fff.rs - skeleton
#[derive(Default)]
pub struct FffIndexer {}

#[cached(...)]
async fn cached_fff_index(...) -> Result<Index> { ... }

impl IndexMiddleware for FffIndexer { ... }

impl FffIndexer {
    async fn index_inner(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        Ok(Index::default()) // stub
    }
}
```

### Step 3: Implement document building from FilePicker results

**Files:** `crates/terraphim_middleware/src/indexer/fff.rs`
**Description:** Flesh out `index_inner` with `FilePicker` initialisation, `collect_files()`, markdown filtering, `grep()` execution, and document construction loop. Replicate all document fields from `RipgrepIndexer::index_inner`.
**Tests:**
- `cargo test -p terraphim_middleware test_fff_indexer_basic`
- `cargo test -p terraphim_middleware test_fff_search_graph`
- Verify document count > 0 for known needles
**Dependencies:** Step 2.
**Estimated:** 3 hours.

Key code: Full `index_inner` implementation (see API Design section).

### Step 4: Implement update_document() for write-back

**Files:** `crates/terraphim_middleware/src/indexer/fff.rs`
**Description:** Add `update_document()` method identical to `RipgrepIndexer::update_document` (HTML detection, `html2md` conversion, `tokio::fs::write`).
**Tests:**
- Create temp markdown file, construct `Document`, call `update_document()`, verify file contents
- Create temp HTML file, construct `Document` with HTML body, verify markdown conversion on write
**Dependencies:** Step 3.
**Estimated:** 1 hour.

### Step 5: Switch search_haystacks to use FffIndexer + add tests

**Files:**
- `crates/terraphim_middleware/src/indexer/mod.rs` — switch `RipgrepIndexer` to `FffIndexer`
- `crates/terraphim_middleware/src/lib.rs` — re-export `FffIndexer`
- `crates/terraphim_middleware/tests/fff_indexer.rs` — new integration tests
**Description:**
1. Change `search_haystacks` to instantiate `FffIndexer::default()` instead of `RipgrepIndexer::default()`
2. Add `pub use fff::FffIndexer;` to `indexer/mod.rs`
3. Create `tests/fff_indexer.rs` mirroring `tests/ripgrep.rs`
4. Run full test suite for `terraphim_middleware`
**Tests:**
- All `tests/fff_indexer.rs` tests pass
- All existing `tests/ripgrep.rs` tests still pass (RipgrepIndexer not deleted)
- `cargo test -p terraphim_middleware` passes
**Dependencies:** Steps 1-4.
**Estimated:** 2 hours.

---

## Rollback Plan

### Immediate Rollback (if issues found during testing)

1. Revert `src/indexer/mod.rs` to use `RipgrepIndexer` instead of `FffIndexer`
2. Revert `src/lib.rs` re-export if added
3. `FffIndexer` code remains in tree but is unreachable

### Feature Flag Approach (not implemented for initial migration)

If runtime toggling is needed later, add a feature flag:

```rust
// In search_haystacks
let indexer: Box<dyn IndexMiddleware> = if cfg!(feature = "fff-indexer") {
    Box::new(FffIndexer::default())
} else {
    Box::new(RipgrepIndexer::default())
};
```

**Decision**: No feature flag for initial migration. The switch is a compile-time change in `search_haystacks`. If issues arise in production, revert the single line in `indexer/mod.rs`.

### Data Compatibility

- Document IDs use `fff_` prefix instead of `ripgrep_` — this means existing persisted documents with `ripgrep_` prefix remain untouched
- New documents from `FffIndexer` will have `fff_` prefix
- No migration of existing documents needed

---

## Dependencies

### New Dependencies

| Crate | Source | Justification |
|-------|--------|---------------|
| `fff-search` | Git: `AlexMikhalev/fff.nvim.git`, branch `feat/external-scorer`, feature `zlob` | Core file search and grep functionality. Git branch required for `ExternalScorer` trait alignment with `terraphim_mcp_server` and `terraphim_file_search`. |

### Dependency Updates

| Crate | From | To | Reason |
|-------|------|-----|--------|
| None | — | — | No existing dependency updates required |

### Workspace Alignment

Follow-up issue recommended: Align all crates to use the same `fff-search` source:
- `terraphim_grep` currently uses `version = "0.8.2"` from crates.io
- `terraphim_mcp_server` and `terraphim_file_search` use Git branch
- `terraphim_middleware` (this change) uses Git branch

Future work: Publish `feat/external-scorer` to crates.io or switch all crates to a consistent version.

---

## Performance Considerations

### Expected Performance

| Metric | RipgrepIndexer (current) | FffIndexer (expected) | Notes |
|--------|-------------------------|----------------------|-------|
| Indexing latency | ~10-100ms for small haystacks | Comparable | fff-search uses SIMD grep; pure Rust overhead may differ |
| Memory per index | File contents + overhead | Same | Both read full file bodies |
| Cache hit latency | ~1 microsecond | ~1 microsecond | Same `cached` macro |
| Binary size | Depends on `rg` binary | +~500KB | `fff-search` linked statically |
| Startup cost | Spawns `rg` process | None | fff-search is in-process |

### Benchmarks to Add

Add to `tests/fff_indexer.rs` or a new `benches/indexer.rs`:

```rust
// In tests/fff_indexer.rs
#[tokio::test]
async fn test_fff_indexer_performance() {
    let haystack = create_test_haystack();
    let indexer = FffIndexer::default();
    
    let start = std::time::Instant::now();
    let index = indexer.index("test", &haystack).await.unwrap();
    let first_duration = start.elapsed();
    
    let start = std::time::Instant::now();
    let _index = indexer.index("test", &haystack).await.unwrap();
    let second_duration = start.elapsed();
    
    println!("First query: {:?}", first_duration);
    println!("Cached query: {:?}", second_duration);
    
    // Cached query should be significantly faster
    assert!(second_duration < first_duration / 2);
}
```

### Performance Risks

| Risk | Mitigation |
|------|-----------|
| fff-search slower than `rg` on very large codebases | Benchmark on representative haystacks; if >20% slower, investigate `FilePickerOptions` tuning |
| Memory pressure from `FilePicker` file list | `FilePicker` lazily loads file metadata; should be comparable to `rg` |
| Cache misses on repeated queries | Cache key identical to RipgrepIndexer; no change in miss rate |

---

## Open Items

| Item | Status | Owner | Resolution Path |
|------|--------|-------|----------------|
| `fff-search` Git branch stability | Pending | Team | Monitor branch for breaking changes; pin to specific commit if needed |
| Exact `description` text parity | TBD | Implementer | May need to tune `GrepSearchOptions` (context lines) to match ripgrep `-C3` output |
| `extra_parameters` mapping completeness | TBD | Implementer | Document exact mapping in code comments; test each parameter |
| Workspace-wide `fff-search` version alignment | Follow-up issue | Team | Create issue to align `terraphim_grep`, `terraphim_mcp_server`, `terraphim_file_search`, `terraphim_middleware` |
| KG scorer integration design | Follow-up issue | Team | Create issue to extend `FffIndexer` with `KgPathScorer` when `Thesaurus` access is available |

---

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

---

## Appendix A: extra_parameters Mapping

| extra_parameters Key | Ripgrep Behaviour | FffIndexer Mapping |
|---------------------|-------------------|-------------------|
| `tag` | `--tag` flag (content filtering) | Post-filter grep results by tag content (not directly supported by fff-search) |
| `glob` | `--glob` pattern | Client-side filter: `path.match(glob)` after `collect_files()` |
| `type` | `-t` flag (file type) | Map to extension filter: `type=markdown` -> `.md` |
| `max_count` | `-m` flag | Map to `GrepSearchOptions.max_matches_per_file` |
| `context` | `-C` flag | fff-search match includes line content; context lines not directly configurable |
| `case_sensitive` | `--case-sensitive` / `--ignore-case` | Map to `GrepSearchOptions.smart_case = false` when `case_sensitive=true` |

---

## Appendix B: Cache Key Format Comparison

| Aspect | RipgrepIndexer | FffIndexer |
|--------|---------------|------------|
| Key format | `"{location}::{needle}::{extra_parameters:?}"` | `"{location}::{needle}::{extra_parameters:?}"` |
| Size limit | 64 entries | 64 entries |
| Result caching | `result = true` (caches `Result<Index>`) | `result = true` (caches `Result<Index>`) |
| Cache type | In-memory LRU | In-memory LRU |

---

## Appendix C: Document Field Mapping

| Field | RipgrepIndexer Source | FffIndexer Source |
|-------|----------------------|-------------------|
| `id` | `normalize_key("ripgrep_{path}")` | `normalize_key("fff_{path}")` |
| `title` | `Path::file_stem()` | `Path::file_stem()` |
| `url` | Full path string | Full path string |
| `body` | `tokio::fs::read_to_string(path)` | `tokio::fs::read_to_string(path)` |
| `description` | First match/context text, trimmed to 200 chars | First match line content, trimmed to 200 chars |
| `doc_type` | `DocumentType::KgEntry` | `DocumentType::KgEntry` |
| `source_haystack` | `None` (set by dispatcher) | `None` (set by dispatcher) |

*End of Design Document*
