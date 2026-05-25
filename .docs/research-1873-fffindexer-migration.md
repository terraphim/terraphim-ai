# Research Document: Replace RipgrepIndexer with FffIndexer

**Issue:** #1873  
**Title:** Replace RipgrepIndexer with FffIndexer in terraphim_middleware  
**Date:** 2026-05-25  
**Researcher:** AI Agent (Phase 1 Disciplined Research)

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| **Does this problem energise us to solve it?** | **YES** | Unblocks code search for terraphim-agent/cli/server; eliminates runtime dependency on external `rg` binary; enables KG scoring and frecency across all search paths |
| **Does solving this leverage our unique capabilities?** | **YES** | fff-search is already integrated in terraphim-grep and MCP server; KgPathScorer already exists in terraphim_file_search; team has deep expertise in both codebases |
| **Does this meet a significant, validated need?** | **YES** | Issue #1873 was created from explicit user request after evaluation showed RipgrepIndexer only indexes `.md` files, ignoring code haystacks entirely |

**Proceed: YES** (3/3)

### Why This Matters

1. **What does RipgrepIndexer do today?** It runs the external `rg` binary against markdown haystacks, parses JSON output, and builds an `Index` (HashMap of `Document`s).
2. **What does fff-search provide?** A Rust-native file search library with fuzzy file finding, grep, multi-pattern grep, and external scorer support (e.g., KG path boosting).
3. **Why replace?** fff-search is pure Rust (no external binary dependency), supports KG scoring via `ExternalScorer`, and is already integrated into `terraphim_grep` and `terraphim_mcp_server`.
4. **What are the risks?** fff-search versions differ across crates; the `FilePicker` API has evolved (0.5.1 to 0.8.2); caching behaviour must be preserved; markdown-only filtering needs reimplementation.

---

## Problem Statement

The `terraphim_middleware` crate currently relies on the external `ripgrep` binary (`rg`) to index haystacks of type `ServiceType::Ripgrep`. This creates a runtime dependency on a separately installed binary, complicates deployment, and prevents integration with Terraphim's knowledge-graph scoring system. The goal is to replace `RipgrepIndexer` with a new `FffIndexer` that uses the native `fff-search` Rust library already used by `terraphim_grep` and `terraphim_mcp_server`.

---

## Current State Analysis

### Architecture

- `search_haystacks` in `crates/terraphim_middleware/src/indexer/mod.rs` dispatches haystack indexing based on `ServiceType`.
- `ServiceType::Ripgrep` is handled by `RipgrepIndexer::index(needle, haystack)`.
- `RipgrepIndexer` is cached via the `cached` macro (`size = 64`, key includes haystack location, needle, and extra parameters).
- The indexer produces an `Index` (essentially `AHashMap<String, Document>`).
- After indexing, `search_haystacks` tags every document with `source_haystack = Some(haystack.location.clone())` and adds documents to `config_state.add_to_roles()`.

### fff-search Adoption Elsewhere

- `terraphim_grep` uses `fff-search` 0.8.2 (crates.io) under the `code-search` feature.
- `terraphim_mcp_server` uses `fff-search` from a Git branch (`feat/external-scorer`) with `zlob` feature.
- `terraphim_file_search` uses the same Git branch and provides `KgPathScorer` (implements `fff_search::external_scorer::ExternalScorer`).

---

## Code Locations

| File | Lines | Purpose |
|------|-------|---------|
| `crates/terraphim_middleware/src/indexer/mod.rs` | 1-175 | `IndexMiddleware` trait and `search_haystacks` dispatcher |
| `crates/terraphim_middleware/src/indexer/ripgrep.rs` | 1-331 | Current `RipgrepIndexer` implementation |
| `crates/terraphim_middleware/src/command/ripgrep.rs` | 1-444 | `RipgrepCommand` wrapper around `rg` binary |
| `crates/terraphim_middleware/Cargo.toml` | 1-89 | Current dependencies (no `fff-search`) |
| `crates/terraphim_grep/src/hybrid_searcher.rs` | 295-370 | `search_code` using `FilePicker::grep` |
| `crates/terraphim_mcp_server/src/lib.rs` | 1252-1573 | `find_files`, `grep_files`, `multi_grep_files` |
| `crates/terraphim_file_search/src/kg_scorer.rs` | 1-191 | `KgPathScorer` implementing `ExternalScorer` |
| `crates/terraphim_file_search/src/config.rs` | 1-17 | `KgScorerConfig` (default `weight_per_term: 5`, `max_boost: 30`) |
| `crates/terraphim_types/src/lib.rs` | 666-703 | `Document` struct definition |
| `crates/terraphim_types/src/lib.rs` | 894-983 | `Index` struct definition |
| `crates/terraphim_persistence/src/lib.rs` | 214-469 | `Persistable` trait with `normalize_key` |
| `crates/terraphim_middleware/tests/ripgrep.rs` | 1-110 | Existing integration tests for `RipgrepIndexer` |

---

## Data Flow

### Current RipgrepIndexer Flow

```
search_haystacks(needle, haystack)
  -> RipgrepIndexer::index(needle, haystack)
    -> cached_ripgrep_index(needle, haystack) [cached macro]
      -> RipgrepCommand::run(needle, haystack_path)
        -> spawn "rg" binary with --json -C3 --ignore-case -tmarkdown
        -> parse JSON Lines into Vec<Message>
      -> RipgrepIndexer::index_inner(messages)
        -> for each Message::Begin/Match/Context/End
          -> build Document { id, title, url, body, description, ... }
          -> insert into Index
    -> return Index
  -> tag documents with source_haystack
  -> add_to_roles(indexed_doc)
```

### Proposed FffIndexer Flow

```
search_haystacks(needle, haystack)
  -> FffIndexer::index(needle, haystack)
    -> FilePicker::new(FilePickerOptions { base_path: haystack.location, mode: FFFMode::Ai, ... })
    -> picker.collect_files()
    -> filter files by extension (.md) and optional extra_parameters
    -> parse_grep_query(needle)
    -> picker.grep(&fff_query, &GrepSearchOptions { ... })
    -> for each match
      -> read file body
      -> build Document { id, title, url, body, description, ... }
      -> insert into Index
    -> return Index
```

---

## Constraints

1. **IndexMiddleware trait signature must remain unchanged.** The trait is used by multiple haystack indexers (`QueryRsHaystackIndexer`, `ClickUpHaystackIndexer`, etc.).
2. **Caching must be preserved.** The `cached` macro on `cached_ripgrep_index` prevents redundant indexing. The new `FffIndexer` should use equivalent caching.
3. **Markdown-only filtering.** The current `RipgrepCommand` defaults to `-tmarkdown`. The new indexer must replicate this unless `extra_parameters` override it.
4. **`source_haystack` tagging.** `search_haystacks` tags documents after indexing; the indexer itself does not need to set this field.
5. **`normalize_key` compatibility.** Document IDs are normalised via `Persistable::normalize_key` (replaces non-alphanumeric with underscores). Any new indexer should follow the same convention.
6. **`update_document` support.** `RipgrepIndexer` has an `update_document` method that writes edited body back to the original markdown file. This must be preserved or relocated.
7. **`extra_parameters` support.** Haystacks can specify `tag`, `glob`, `type`, `max_count`, `context`, and `case_sensitive` extra parameters. These are currently parsed into `rg` arguments. An fff-based indexer needs equivalent filtering.

---

## fff-search API Reference

### Versions and Sources

| Crate | fff-search Spec | Feature Flags |
|-------|-----------------|---------------|
| `terraphim_grep` | `fff-search = { version = "0.8.2", optional = true }` | `code-search` |
| `terraphim_mcp_server` | `fff-search = { git = "https://github.com/AlexMikhalev/fff.nvim.git", branch = "feat/external-scorer" }` | `zlob` |
| `terraphim_file_search` | Same Git branch as MCP server | `zlob` |

> **Open Question:** Which version should `terraphim_middleware` depend on? The crates.io 0.8.2 release or the Git branch with `external_scorer` support? The Git branch is required for `KgPathScorer` integration.

### Key Types

```rust
// From terraphim_grep/src/hybrid_searcher.rs (lines 310-313)
use fff_search::{
    FFFMode, FilePicker, FilePickerOptions, GrepMode, GrepSearchOptions,
    parse_grep_query,
};

// From terraphim_mcp_server/src/lib.rs (lines 20-25)
use fff_search::{
    ContentCacheBudget, FFFMode, FilePicker, FilePickerOptions, FuzzySearchOptions, GrepMode,
    GrepSearchOptions, PaginationArgs, QueryParser, SharedFrecency, grep_search, multi_grep_search,
    parse_grep_query,
};
use fff_search::external_scorer::ExternalScorer;
```

### FilePicker Initialisation

```rust
let mut picker = FilePicker::new(FilePickerOptions {
    base_path: search_path.to_string_lossy().to_string(),
    mode: FFFMode::Ai,
    watch: false,
    cache_budget: None,
    ..FilePickerOptions::default()
})?;

picker.collect_files()?;
```

> **Note:** `terraphim_grep` (0.8.2) uses `..FilePickerOptions::default()` because `warmup_mmap_cache` was dropped in 0.8.2. The MCP server (Git branch) still uses `warmup_mmap_cache: false` explicitly.

### Grep Search

```rust
let fff_query = parse_grep_query(query);
let options = GrepSearchOptions {
    max_file_size: 10 * 1024 * 1024,
    max_matches_per_file: 200,
    smart_case: true,
    file_offset: 0,
    page_limit: limit,
    mode: GrepMode::PlainText,
    ..GrepSearchOptions::default()
};
let result = picker.grep(&fff_query, &options);
```

> In the MCP server, `grep_search(&files, &fff_query, &options, &budget, None, None, None)` is used instead of `picker.grep(...)`, where `files` is `picker.get_files().to_vec()`.

### Fuzzy Search

```rust
let result = FilePicker::fuzzy_search(
    files,
    &fff_query,
    None, // external scorer context
    FuzzySearchOptions {
        max_threads: 0,
        pagination: PaginationArgs { offset: 0, limit: max_results * 4 },
        ..Default::default()
    },
);
```

### External Scorer

```rust
pub trait ExternalScorer {
    fn score(&self, file: &FileItem) -> i32;
}
```

`KgPathScorer` implements this by running `terraphim_automata::find_matches` on `file.relative_path`.

---

## RipgrepIndexer Analysis

### Implementation Summary

```rust
#[derive(Default)]
pub struct RipgrepIndexer {}

#[cached(result = true, size = 64, key = "String", convert = r#"{ format!("{}::{}::{:?}", haystack.location, needle, haystack.get_extra_parameters()) }"#)]
async fn cached_ripgrep_index(needle: &str, haystack: &Haystack) -> Result<Index> {
    let command = RipgrepCommand::default();
    let haystack_path = Path::new(&haystack.location);
    // ... validate path exists ...
    let extra_params = haystack.get_extra_parameters();
    let extra_args = command.parse_extra_parameters(extra_params);
    let messages = if extra_args.is_empty() {
        command.run(needle, haystack_path).await?
    } else {
        command.run_with_extra_args(needle, haystack_path, &extra_args).await?
    };
    let indexer = RipgrepIndexer::default();
    let documents = indexer.index_inner(messages).await;
    Ok(documents)
}
```

### Document Construction

For each `Message::Begin`, a new `Document` is created:
- `id`: `normalize_document_id(&path)` -> `ripgrep_{file_path}` normalised
- `title`: file stem from path
- `url`: full path string
- `body`: entire file contents read via `tokio::fs::read_to_string`
- `description`: first match/context text, trimmed to 200 chars

### update_document Method

```rust
pub async fn update_document(&self, document: &Document) -> Result<()> {
    let path = Path::new(&document.url);
    let mut content = document.body.clone();
    if content.contains('<') && content.contains('>') {
        content = html2md::parse_html(&content);
    }
    fs::write(path, content).await?;
    Ok(())
}
```

> This is a **file write-back** feature that edits the original markdown. It must be preserved for parity.

---

## KgPathScorer Integration

### Current Usage

`KgPathScorer` is currently **not** used by `terraphim_middleware`. It is only wired into:
- `terraphim_mcp_server::McpService::find_files` — adds KG boost to fuzzy search scores
- `terraphim_mcp_server::McpService::grep_files` — sorts files by KG score before grepping
- `terraphim_mcp_server::McpService::multi_grep_files` — same sorting

### Scorer Wiring

```rust
// McpService::new
let kg_scorer: Option<Arc<KgPathScorer>> = None;

// McpService::with_kg_scorer
pub fn with_kg_scorer(mut self, scorer: Arc<KgPathScorer>) -> Self {
    self.kg_scorer = Some(scorer);
    self
}
```

### Implications for FffIndexer

If `FffIndexer` is to support KG-boosted ordering (a likely requirement), it needs:
1. Access to a `Thesaurus` (from `ConfigState` or `RoleGraph`)
2. Construction of `KgPathScorer`
3. Sorting `FilePicker` results by scorer before building documents

However, the current `IndexMiddleware` trait does **not** pass `ConfigState` or a thesaurus to `index()`. This is a significant interface gap.

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| **Must preserve `update_document` write-back** | `terraphim_service::create_document()` (line 1038) actively calls `ripgrep.update_document()` for every non-read-only haystack. Breaking this breaks the desktop UI's edit-and-save workflow. | Code trace: `terraphim_service/src/lib.rs:1038` |
| **Must use Git branch `feat/external-scorer`** | `terraphim_mcp_server` and `terraphim_file_search` already depend on this branch for `ExternalScorer`/`KgPathScorer`. Using crates.io 0.8.2 would create a version split in the middleware layer. | `Cargo.toml` analysis across crates |
| **Must preserve `IndexMiddleware` trait signature** | The trait is used by 7+ indexers (QueryRs, ClickUp, MCP, Perplexity, Quickwit, Jmap, GrepApp). Changing it forces cascading changes across the entire middleware layer. | `crates/terraphim_middleware/src/indexer/mod.rs` |

### Eliminated from Scope

Apply the 5/25 Rule. These items are explicitly NOT in scope for the initial migration:

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| **KG path scoring in initial FffIndexer** | Requires `Thesaurus` access, but `IndexMiddleware::index()` trait signature does not pass `ConfigState` or thesaurus. Requires trait extension — follow-up issue. |
| **Frecency scoring in initial FffIndexer** | Requires `FFF_FRECENCY_PATH` env var and `SharedFrecency` initialisation. Can be added as a builder method on `FffIndexer` without changing the trait. |
| **Cursor pagination in initial FffIndexer** | Requires `SearchQuery` API changes (adding `cursor` field). Needs coordination with terraphim_types and terraphim_service. |
| **Non-markdown file support as default** | The current behaviour is markdown-only. Expanding to all text files changes semantics and may surprise users. Can be enabled via `extra_parameters` opt-in. |
| **Deleting `RipgrepIndexer` / `RipgrepCommand`** | Keep as fallback until `FffIndexer` is validated in production. Separate cleanup issue. |

---

## Risks and Unknowns

| Risk | Severity | Mitigation |
|------|----------|------------|
| fff-search version mismatch between 0.8.2 and Git branch | High | Pin a single version across all crates; consider publishing the external-scorer branch to crates.io or using a git submodule |
| `FilePickerOptions` API differences between versions | Medium | Use `..Default::default()` for forward compatibility (as done in `terraphim_grep`) |
| `extra_parameters` filtering (tag, glob, type) not supported by fff-search | Medium | Implement client-side filtering after `collect_files()`, or extend fff-search constraints API |
| KG scorer requires `Thesaurus` which `IndexMiddleware::index()` does not receive | Medium | Extend the trait or pass scorer via `FffIndexer` constructor |
| `update_document` writes back to disk — fff-search is read-only | Low | Keep `update_document` as a standalone method on `FffIndexer` (it does not need fff-search) |
| Performance regression vs `rg` on large haystacks | Medium | Benchmark before/after; fff-search uses SIMD but may differ on very large codebases |
| Caching key must include all inputs that affect output | Low | Replicate existing cache key format with fff-specific inputs |

---

## Assumptions

1. The replacement is **additive-then-subtractive**: `FffIndexer` will be created, tested, and only then will `RipgrepIndexer` be removed.
2. The `ServiceType::Ripgrep` enum variant name will remain for backward compatibility (even though the underlying implementation no longer uses ripgrep).
3. Markdown-only indexing is the default behaviour and must be preserved.
4. The haystack `location` field is always a valid filesystem path for `ServiceType::Ripgrep` haystacks.

---

## Open Questions

1. **Which fff-search dependency should `terraphim_middleware` use?**
   - Option A: `version = "0.8.2"` from crates.io (stable, but may lack `external_scorer`)
   - Option B: Git branch `feat/external-scorer` (has `external_scorer`, but unstable)
   - *Impact:* If we want KG scoring in the indexer, we need the Git branch or a published version that includes `ExternalScorer`.

2. **Should `IndexMiddleware` be extended to accept a `KgPathScorer` or `ConfigState`?**
   - Currently the trait only receives `needle: &str` and `haystack: &Haystack`.
   - If KG boosting is required, the trait or `FffIndexer` constructor must change.

3. **How should `extra_parameters` be mapped to fff-search?**
   - `tag` (line content filtering) — not directly supported by fff-search; may need post-filtering.
   - `glob` and `type` — may be mappable to `FilePicker` constraints or filtered after `collect_files()`.
   - `max_count`, `context`, `case_sensitive` — some map to `GrepSearchOptions`, others do not.

4. **Should the `cached` macro key change?**
   - The current key includes `haystack.get_extra_parameters()`. If fff-search uses different options, the key must reflect them.

5. **What happens to `RipgrepCommand` and `command::ripgrep` module?**
   - If `RipgrepIndexer` is fully removed, the entire `command/ripgrep.rs` module becomes dead code unless other indexers use it (they do not).

6. **Does `FffIndexer` need to support `update_document`?**
   - Yes, for feature parity with the desktop UI's edit-and-save workflow.

7. **What test fixtures exist?**
   - `fixtures/haystack` is used by `tests/ripgrep.rs`. The same fixtures can validate `FffIndexer`.

---

## Recommendations

1. **Use the Git branch `feat/external-scorer` for `fff-search`** in `terraphim_middleware` to keep the door open for `KgPathScorer` integration. Align all crates to the same source in a follow-up issue.

2. **Keep `IndexMiddleware` trait unchanged** for the initial migration. Add KG scoring later by extending `FffIndexer` with an optional `Arc<KgPathScorer>` field and a builder method (similar to `McpService::with_kg_scorer`).

3. **Implement `FffIndexer` in a new file** `crates/terraphim_middleware/src/indexer/fff.rs` and add `mod fff;` to `indexer/mod.rs`. Do not delete `ripgrep.rs` until the new implementation is fully validated.

4. **Preserve caching** by wrapping the new indexer's core logic in a `cached` function with an equivalent cache key.

5. **Replicate document fields exactly:**
   - `id`: use `normalize_document_id` pattern (`fff_{relative_path}`)
   - `title`: file stem
   - `url`: absolute or relative path
   - `body`: full file text (read via `tokio::fs::read_to_string`)
   - `description`: first matching line content, truncated at 200 chars

6. **Write tests first.** Create `tests/fff_indexer.rs` that mirrors `tests/ripgrep.rs` and asserts identical output for the same needle/haystack pairs.

7. **Schedule removal of `RipgrepCommand` and `RipgrepIndexer`** as a separate cleanup issue once `FffIndexer` has been running in production without issues.

---

*End of Research Document*
