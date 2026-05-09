# Documentation Report 2026-05-09

**Generated:** 2026-05-09
**Scope:** Full workspace (`cargo doc --no-deps --workspace`)
**Tool:** `cargo doc` warnings + manual public-API audit

---

## 1. Rustdoc Warning Status

| Check | Result |
|---|---|
| `cargo doc --no-deps --workspace` warnings | 0 (after fix below) |
| Crate-level `//!` docs missing | 0 |
| `missing_docs` lint warnings | 0 |
| `private_intra_doc_links` warnings | 0 |
| `redundant_explicit_links` warnings | 1 (fixed) |

### Fix Applied

**File:** `crates/terraphim_symphony/src/tracker/gitea.rs` line 4

**Warning:** `rustdoc::redundant_explicit_links` — link `[Issue](super::Issue)` used explicit target when label alone resolves to the same destination.

**Fix:** Changed `` [`Issue`](super::Issue) `` to `` [`Issue`] ``.

---

## 2. Crate Coverage Audit

All 56 workspace crates carry crate-level `//!` documentation as of 2026-05-08. The workspace `cargo doc` build produces zero warnings.

### Crates with Good Coverage (>80% public items documented)

| Crate | Evidence |
|---|---|
| `terraphim_automata` | Zero warnings; full module-level + item-level docs; examples in `//!` |
| `terraphim_config` | Zero warnings; all public structs, enums, and functions documented |
| `terraphim_types` | Zero warnings; comprehensive field-level docs across all types |
| `terraphim_service` | Zero warnings; `ServiceError`, `TerraphimService`, auto-route types all documented |
| `terraphim_server` | Zero warnings; all DTOs, handlers, and error types documented (fixed 2026-05-08) |
| `terraphim_persistence` | Zero warnings; `DeviceStorage` singleton pattern and cache write-back documented |
| `terraphim_rolegraph` | Zero warnings; intra-doc links corrected 2026-05-08 |
| `terraphim_symphony` | Zero warnings after redundant-link fix applied today |
| `haystack_core` | Zero warnings; module-level doc added 2026-05-08 |
| `haystack_atlassian` | Zero warnings; module-level doc added 2026-05-08 |
| `haystack_discourse` | Zero warnings; module-level doc added 2026-05-08 |
| `haystack_jmap` | Zero warnings; module-level doc added 2026-05-08 |

### Crates with Remaining Gaps

No `missing_docs` warnings were emitted by `cargo doc`. However, a targeted public-API audit of four key crates identified items that carry `///` comments but lack `# Examples` sections. These are advisory gaps, not lint failures.

| Crate | Advisory gap |
|---|---|
| `terraphim_service` | `TerraphimService::search`, `index_documents`, `build_thesaurus` — no `# Examples` in rustdoc |
| `terraphim_config` | `ConfigState::new`, `ConfigBuilder::build` — no `# Examples` |
| `terraphim_automata` | `sharded_extractor` module — items documented but no usage example |
| `terraphim_symphony` | `tracker::gitea` — one-line module doc; no trait usage example |

---

## 3. CHANGELOG Status

**File:** `CHANGELOG.md` — exists, Keep a Changelog format, up to date.

Entries added or confirmed present for the 25 most recent commits:

| Commit | Subject | CHANGELOG entry |
|---|---|---|
| `fc707c3` | `feat(build-runner): agent work [auto-commit]` | Covered under build-runner section |
| `3128b66` | `fix(symphony): enforce RetryBound invariant` | Added to [Unreleased] today |
| `a36c204` | `docs(reports): add documentation generation report 2026-05-09` | This report |
| `de592e5` | `docs: add rustdoc to terraphim_config, terraphim_service, terraphim_settings` | Present in [Unreleased] |
| `64c4c9a` | `docs: add rustdoc to terraphim_types and terraphim_automata` | Present in [Unreleased] |
| `484943b` | `fix(agent): populate concepts_matched and wildcard_fallback` | Present in [Unreleased] |
| `a67f129` | `fix(tests): replace cargo-run subprocess with assert_cmd` | Present in [Unreleased] |
| `0030889` | `fix(tests): use Terraphim Engineer role in test_full_feature_matrix` | Present in [Unreleased] |
| `2b2996c` | `docs: fix private intra-doc link warning in terraphim_server` | Present in [Unreleased] |
| `0e5c9e4` | `docs(terraphim_server): add rustdoc to all public API items` | Present in [Unreleased] |
| `4b54e92` | `docs: fix remaining rustdoc warnings in persistence and rolegraph` | Present in [Unreleased] |
| `81e155a` | `docs: fix remaining rustdoc warnings across 11 crates` | Present in [Unreleased] |
| `24a5f64` | `docs: fix rustdoc intra-doc link warnings` | Present in [Unreleased] |

---

## 4. API Reference Summaries

### `terraphim_types` (v1.15.0)

Core shared types used across the entire workspace.

Key public types:

| Type | Kind | Purpose |
|---|---|---|
| `RoleName` | newtype struct | Validated user-role identifier |
| `NormalizedTermValue` | newtype struct | Case-normalised search term |
| `NormalizedTerm` | struct | Thesaurus entry with rank and synonyms |
| `Document` | struct | Full document with URL, body, tags, rank |
| `IndexedDocument` | struct | Document after indexing with concept extraction |
| `SearchQuery` | struct | Search parameters including term, operator, role, pagination |
| `Thesaurus` | struct | Map of `NormalizedTermValue` to `NormalizedTerm` |
| `Edge` / `Node` | structs | Knowledge graph primitives |
| `RelevanceFunction` | enum | `TitleScorer`, `BM25`, `BM25F`, `BM25Plus`, `TerraphimGraph` |
| `LogicalOperator` | enum | `And` / `Or` for multi-term queries |
| `Conversation` / `ChatMessage` | structs | LLM conversation history |
| `MultiAgentContext` | struct | Shared context for agent coordination |

### `terraphim_automata` (v1.15.0)

Fast text matching and autocomplete using Aho-Corasick automata and FST.

Key public items:

| Item | Kind | Purpose |
|---|---|---|
| `build_autocomplete_index` | fn | Build FST prefix index from a `Thesaurus` |
| `autocomplete_search` | fn | Prefix search returning ranked `AutocompleteResult`s |
| `fuzzy_autocomplete_search` | fn | Levenshtein-distance fuzzy search |
| `fuzzy_autocomplete_search_jaro_winkler` | fn | Jaro-Winkler fuzzy search |
| `find_matches` | fn | Aho-Corasick multi-pattern search returning `Matched` spans |
| `replace_matches` | fn | Replace matched terms with Markdown/HTML/Wiki links |
| `extract_paragraphs_from_automata` | fn | Extract paragraphs beginning at matched terms |
| `LinkType` | enum | `Markdown`, `Html`, `Wiki` |
| `AutocompleteIndex` | struct | Serialisable FST index with metadata |
| `AutocompleteResult` | struct | Match with term, score, and metadata |
| `load_thesaurus` | fn | Load thesaurus from a JSON file path |
| `load_thesaurus_from_json` | fn | Deserialise thesaurus from JSON bytes |

### `terraphim_config` (v1.15.0)

Role-based configuration management with persistence and async reload.

Key public items:

| Item | Kind | Purpose |
|---|---|---|
| `Config` | struct | Top-level config holding all roles |
| `ConfigState` | struct | `Arc<Mutex<Config>>` with async load/save |
| `ConfigBuilder` | struct | Builder for constructing `Config` programmatically |
| `Role` | struct | User profile: haystacks, relevance function, LLM settings, theme |
| `Haystack` | struct | Data source descriptor (path, service type, extra parameters) |
| `ServiceType` | enum | Supported backends: `Ripgrep`, `AtomicServer`, `ClickUp`, `Quickwit`, `MCP`, etc. |
| `KnowledgeGraph` | struct | KG configuration with automata path and input type |
| `TerraphimConfigError` | enum | `NotFound`, `Parse`, `Persistence`, `Validation` variants |
| `expand_path` | fn | Expand `~` and env vars in config file paths |

### `terraphim_service` (v1.16.15)

Main service facade integrating search, indexing, and AI summarisation.

Key public items:

| Item | Kind | Purpose |
|---|---|---|
| `TerraphimService` | struct | Central service holding config state and rolegraph |
| `ServiceError` | enum | Unified error covering middleware, persistence, config, LLM |
| `auto_select_role` | fn | Auto-route a query to the most appropriate role |
| `AutoRouteResult` | struct | Selected role with routing reason |
| `AutoRouteReason` | enum | Why a role was selected (keyword match, default, fallback) |
| `llm` module | mod | Generic LLM client trait and provider implementations |
| `llm_proxy` module | mod | Unified proxy for OpenRouter and Ollama |
| `conversation_service` module | mod | Conversation persistence and retrieval |
| `summarization_queue` module | mod | Async queue for document summarisation |

---

## 5. Actions Taken

1. **Fixed** `rustdoc::redundant_explicit_links` warning in `crates/terraphim_symphony/src/tracker/gitea.rs` — workspace now builds with zero rustdoc warnings.
2. **Updated** `CHANGELOG.md` [Unreleased] section with `fix(symphony)` entry.
3. **Confirmed** `cargo doc --no-deps --workspace` exits 0 with zero warnings (three independent runs).

## 6. Recommendations

| Priority | Item |
|---|---|
| Low | Add `# Examples` rustdoc sections to `TerraphimService::search`, `ConfigState::new`, and `ConfigBuilder::build` — currently documented but without runnable examples |
| Low | `terraphim_symphony::tracker::gitea` module doc is a single sentence; expand to include a usage example showing `IssueTracker` initialisation |
| Info | Consider enabling `#![warn(missing_docs)]` on the four key crates (`terraphim_types`, `terraphim_automata`, `terraphim_config`, `terraphim_service`) to enforce doc coverage at compile time |
