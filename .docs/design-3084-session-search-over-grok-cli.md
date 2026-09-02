# Implementation Plan: Terraphim Session Search Over Grok CLI Search (#3084)

**Status**: Draft
**Research Doc**: `.docs/research-3084-session-search-over-grok-cli.md`
**Author**: OpenCode
**Date**: 2026-07-12
**Estimated Effort**: 1.5 to 2 days

## Overview

### Summary

Add Grok CLI sessions as a first-class Terraphim search source so `terraphim-agent sessions search` can become the preferred local search command over `grok sessions search`. The revised design is a minimal feature-gated direct-query adapter in `terraphim_sessions` that opens Grok's existing `session_search.sqlite` read-only, executes FTS queries, and merges transient Grok hits with Terraphim's existing session search results.

### Approach

Do not wrap the Grok CLI and do not import/copy Grok's search store. Query Grok's SQLite FTS table directly and return a unified result projection that can be displayed beside existing Terraphim session hits.

### Scope

**In Scope:**

- Add `grok-search` feature to `crates/terraphim_sessions`.
- Add a direct-query Grok search provider, e.g. `crates/terraphim_sessions/src/search_provider/grok.rs` or `src/grok_search.rs`.
- Detect Grok search DB from `GROK_HOME/sessions/session_search.sqlite` or `~/.grok/sessions/session_search.sqlite`.
- Query `session_docs_fts` and `session_docs` read-only.
- Merge transient Grok hits with existing Terraphim session search hits.
- Add tests with real temporary SQLite files.
- Update user-facing documentation for preferred Terraphim search workflow.

**Out of Scope:**

- Modifying Grok CLI itself.
- Shelling out to `grok sessions search` at runtime.
- Importing Grok `session_docs` rows into Terraphim's session cache.
- Creating a duplicate Terraphim index over Grok data.
- Importing rewind file snapshots.
- Real-time watching for Grok sessions in the first implementation.
- Replacing Terraphim BM25/KG scoring.
- Adding new external services or network calls.

**Avoid At All Cost:**

- A second search engine just for Grok.
- A hard dependency on Grok being installed in `$PATH`.
- Copying Grok's search index into Terraphim.
- Broad public model changes before connector needs prove them necessary.
- Test mocks for Grok storage.

## Architecture

### Component Diagram

```text
~/.grok or $GROK_HOME
  -> sessions/session_search.sqlite
      -> GrokSearchProvider (read-only SQLite FTS)
          -> Vec<SessionSearchHit>
              -> SessionService search merge
                  -> terraphim-agent sessions search
```

### Data Flow

```text
sessions search <query>
  -> maybe_auto_import
  -> existing cached/imported session search
  -> GrokSearchProvider::search(query, limit) reads session_search.sqlite
  -> normalise and merge scores
  -> render unified results
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Add direct `GrokSearchProvider` under `terraphim_sessions` | Avoids forcing an external FTS store into an import/cache trait | `SessionConnector` import implementation. |
| Feature-gate as `grok-search` | Keeps optional SQLite/Grok-specific code isolated | Always-on dependency. |
| Use `GROK_HOME` before `~/.grok` | Supports documented override and default install | Hard-code only `~/.grok`. |
| Query `session_docs_fts` directly | Grok already maintains FTS and content hashes | Parse raw session directories or duplicate index. |
| Return a unified hit projection | Cached sessions and direct SQLite hits need a shared display shape | Force every result into full `Session`. |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Runtime `grok sessions search` wrapper | Search results would remain dependent on another CLI process | External process failures and poor result control. |
| Import Grok `session_docs` rows | Duplicates Grok's search store and creates stale cache risk | Sync complexity and ownership ambiguity. |
| Parse raw Grok session directories first | SQLite FTS already provides indexed searchable content | More format drift and larger scope. |
| Watcher support in first pass | Direct SQLite query sees current Grok indexed state | More concurrency and debounce complexity. |

### Simplicity Check

The easiest correct design is one small read-only SQLite search provider plus a unified search-result projection. The existing import connectors remain unchanged, and Grok remains the owner of its own search database. This avoids speculative indexing and keeps the first change reviewable.

**Nothing Speculative Checklist:**

- [x] No features the user did not request.
- [x] Only one small abstraction where the current import trait is the wrong fit.
- [x] No runtime dependency on the Grok CLI.
- [x] No new search scorer.
- [x] No watcher or import step for Grok.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_sessions/src/search_provider/mod.rs` | Defines direct search-provider abstraction for external indexed stores. |
| `crates/terraphim_sessions/src/search_provider/grok.rs` | Implements read-only Grok SQLite FTS queries. |
| `crates/terraphim_sessions/tests/grok_search_provider.rs` | Integration tests using real temporary Grok-shaped SQLite storage. |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_sessions/Cargo.toml` | Add `grok-search` feature and reuse optional `rusqlite`. |
| `crates/terraphim_sessions/src/lib.rs` | Export `search_provider` behind the feature if downstream callers need it. |
| `crates/terraphim_sessions/src/service.rs` | Merge direct-provider hits into session search output without importing Grok rows. |
| `docs/specifications/terraphim-agent-session-search-spec.md` | Add Grok to supported sources and document preferred Terraphim-over-Grok workflow. |
| `README.md` | Add concise example: query/search Grok sessions with Terraphim. |

### Optional Follow-Up Files

| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/repl/commands.rs` | Add `sessions search --source <id> --limit <n>` parsing. |
| `crates/terraphim_agent/src/repl/handler.rs` | Apply search source/limit and show snippets/scores if available. |

## API Design

### Unified Result Type

```rust
#[derive(Debug, Clone)]
pub struct SessionSearchHit {
    pub source: String,
    pub session_id: String,
    pub external_id: String,
    pub title: Option<String>,
    pub project_path: Option<String>,
    pub source_path: Option<PathBuf>,
    pub score: f64,
    pub snippet: Option<String>,
    pub updated_at: Option<jiff::Timestamp>,
}
```

### Direct Search Provider Trait

```rust
#[async_trait]
pub trait SessionSearchProvider: Send + Sync {
    fn source_id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn detect(&self) -> ConnectorStatus;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SessionSearchHit>>;
    async fn get(&self, session_id: &str) -> Result<Option<SessionSearchHit>>;
}
```

### Internal Helper Signatures

```rust
#[derive(Debug, Default)]
pub struct GrokSearchProvider;

impl GrokSearchProvider {
    fn grok_home(&self) -> Option<PathBuf>;
    fn database_path(&self) -> Option<PathBuf>;
    fn open_readonly(&self, db_path: &Path) -> Result<rusqlite::Connection>;
    fn escape_fts_query(query: &str) -> String;
    fn normalise_grok_score(raw_bm25: f64) -> f64;
}
```

### Suggested Internal Data Types

```rust
#[derive(Debug, Deserialize)]
struct GrokSqliteRow {
    session_id: String,
    cwd: String,
    updated_at: i64,
    title: String,
    content: String,
    content_hash: String,
    last_indexed_offset: i64,
}
```

Keep these private to the Grok search provider module.

### SQLite Mapping Rules

| Grok Field | Terraphim Field |
|------------|-----------------|
| `session_id` | `SessionSearchHit.session_id = format!("grok:{session_id}")`, `external_id = session_id` |
| `cwd` | `project_path` |
| `title` | `title` |
| `updated_at` | `updated_at` after timestamp conversion |
| `bm25(session_docs_fts)` | Normalised `score` |
| `snippet(...)` | `snippet` |
| `content_hash`, `last_indexed_offset` | Optional internal metadata if a metadata map is added later |

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `detects_grok_database_from_env` | `grok_search_provider.rs` | Verifies `GROK_HOME` override is honoured. |
| `detects_missing_database` | `grok_search_provider.rs` | Returns `NotFound` when no SQLite store exists. |
| `queries_sqlite_fts` | `grok_search_provider.rs` | Builds a real SQLite DB and returns FTS hits. |
| `returns_snippets_and_project_path` | `grok_search_provider.rs` | Verifies snippet/title/cwd projection. |
| `handles_title_only_rows` | `grok_search_provider.rs` | Rows with empty content can still be returned for title matches. |
| `opens_database_readonly` | `grok_search_provider.rs` | Verifies no writes are required. |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `grok_hits_are_merged_with_session_hits` | `crates/terraphim_sessions/tests/grok_search_provider.rs` | Query Grok fixture and existing session fixture, assert unified output includes both sources. |
| `source_filter_limits_to_grok` | `crates/terraphim_sessions/tests/grok_search_provider.rs` | Search with source filter and assert only Grok hits are returned. |

### Manual Verification

```bash
`cargo test -p terraphim_sessions --features grok-search,search-index`
cargo test -p terraphim_sessions --features full
~/.cargo/bin/terraphim-agent sessions sources
~/.cargo/bin/terraphim-agent sessions search "session search"
/home/alex/.grok/bin/grok sessions search "session search" --limit 5
```

Expected manual outcome:

- Terraphim detects `grok` as a direct search source.
- Terraphim returns Grok hits plus any matching non-Grok sessions.
- Terraphim does not create a copy of Grok's search store or require `sessions import --source grok`.

### Coverage Check

After implementation, run a scoped coverage check for changed Rust crates:

```bash
cargo llvm-cov -p terraphim_sessions --features grok-search,search-index --summary-only
```

If `cargo llvm-cov` is not installed, document that gap and run the full test suite above plus `ubs` on changed files.

## Implementation Steps

### Step 1: Add Feature Flag and Search Provider Skeleton

**Files:** `crates/terraphim_sessions/Cargo.toml`, `crates/terraphim_sessions/src/search_provider/mod.rs`, `crates/terraphim_sessions/src/search_provider/grok.rs`

**Description:** Add `grok-search = ["dep:rusqlite"]`; define `SessionSearchProvider` and `SessionSearchHit`; add a skeletal `GrokSearchProvider` under the feature gate.

**Tests:** `cargo test -p terraphim_sessions --no-default-features`

**Estimated:** 1 hour

### Step 2: Implement Grok Database Detection

**Files:** `crates/terraphim_sessions/src/search_provider/grok.rs`

**Description:** Implement `source_id`, `display_name`, `grok_home`, `database_path`, and `detect`. Detection should return `Available` when `session_search.sqlite` exists and estimate count from `session_docs`.

**Tests:** detection tests with temporary real directories.

**Estimated:** 2 hours

### Step 3: Implement Read-Only SQLite FTS Search

**Files:** `crates/terraphim_sessions/src/search_provider/grok.rs`

**Description:** Open `session_search.sqlite` read-only. Query `session_docs_fts` joined to `session_docs`, returning `SessionSearchHit` values with session ID, cwd, title, snippet, updated timestamp, and normalised score.

**Tests:** create a real SQLite database in a temp dir and import it.

**Estimated:** 3 hours

### Step 4: Merge Direct Hits With Existing Session Search

**Files:** `crates/terraphim_sessions/src/service.rs`, `crates/terraphim_sessions/src/search.rs`

**Description:** Add a search path that projects existing `Scored<Session>` results into `SessionSearchHit`, queries `GrokSearchProvider`, normalises scores, merges, sorts, and limits.

**Tests:** temp directory fixture with two sessions and one malformed record.

**Estimated:** 4 hours

### Step 5: CLI Surface

**Files:** `crates/terraphim_agent/src/repl/commands.rs`, `crates/terraphim_agent/src/repl/handler.rs`

**Description:** Add `sessions search --source <source> --limit <n>` if feasible. Ensure `--source grok` uses direct-provider results and does not call import.

**Tests:** fixture where the same ID appears in SQLite and directory data.

**Estimated:** 2 hours

### Step 6: Search Pipeline Regression Test

**Files:** `crates/terraphim_sessions/tests/grok_search_provider.rs`

**Description:** Build a real SQLite FTS fixture, search for a term in indexed content, and assert source is `grok` without any import step.

**Tests:** run `cargo test -p terraphim_sessions --features grok-search,search-index`.

**Estimated:** 2 hours

### Step 7: Documentation Update

**Files:** `README.md`, `docs/specifications/terraphim-agent-session-search-spec.md`

**Description:** Document Grok as a supported source and position Terraphim session search as the canonical cross-tool search path. Include comparison to `grok sessions search` without disparaging Grok.

**Tests:** documentation review; no code tests.

**Estimated:** 1 hour

### Step 8: Quality Gates

**Files:** changed files only

**Commands:**

```bash
cargo fmt --check
cargo clippy -p terraphim_sessions --features grok-search,search-index --all-targets -- -D warnings
cargo test -p terraphim_sessions --features grok-search,search-index
cargo test -p terraphim_sessions --features full
ubs crates/terraphim_sessions/src/search_provider/grok.rs crates/terraphim_sessions/src/search_provider/mod.rs crates/terraphim_sessions/src/service.rs crates/terraphim_sessions/Cargo.toml
```

**Estimated:** 2 hours

## Rollback Plan

If the Grok direct-query provider causes regressions:

1. Remove `grok-search` from `full` while leaving the module behind the explicit feature.
2. Disable direct-provider merging while keeping existing imported-session search unchanged.
3. Keep documentation marked experimental until the provider passes local verification.

## Dependencies

### New Dependencies

No new crate is required if `rusqlite` is reused behind `grok-search`.

### Dependency Updates

None planned.

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Grok query latency | Use SQLite FTS directly with `LIMIT` | Integration fixture plus manual local query timing. |
| Search merge latency | Preserve existing session-search target where practical | Search against local corpus with Grok provider enabled. |
| Memory | Avoid loading all Grok rows | Fetch only limited FTS hits. |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Decide if `grok-search` joins `full` immediately | Pending | Human reviewer |
| Define exact score normalisation for Grok FTS versus Terraphim BM25 | Pending | Implementer |
| Decide first-pass `--source` and `--limit` CLI support | Pending | Human reviewer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Grok direct-query feature inclusion decided
- [ ] Human approval received before implementation
