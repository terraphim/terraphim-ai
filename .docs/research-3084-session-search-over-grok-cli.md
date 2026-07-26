# Research Document: Terraphim Session Search Over Grok CLI Search (#3084)

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-07-12
**Issue**: #3084

## Executive Summary

The local repository was fast-forwarded to `origin/main` and `gitea/main` at `101532826` before research. Follow-up validation confirmed that Grok already maintains a local SQLite FTS search store at `~/.grok/sessions/session_search.sqlite`; Terraphim should query that store read-only rather than importing or copying Grok search data. Grok CLI search is useful but isolated; Terraphim can provide a unified local search surface by adding a direct-query Grok adapter and merging those transient results with existing Terraphim session results.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Terraphim's core value is local knowledge search across AI coding history. |
| Leverages strengths? | Yes | Existing `terraphim_sessions` connectors, BM25 ranking, KG enrichment, and Terraphim CLI make this a natural fit. |
| Meets real need? | Yes | The user explicitly asked to plan updating session search over Grok CLI search; Grok sessions are currently outside Terraphim's connector registry. |

**Proceed**: Yes - 3/3 yes.

## Problem Statement

### Description

Terraphim session search should become the preferred local search path for AI coding sessions, including sessions created by Grok CLI. Today, Grok can search its own sessions via `grok sessions search`, but Terraphim cannot query Grok's local search index and therefore cannot return Grok matches alongside Claude Code, OpenCode, Codex, Aider, and Cline sessions.

### Impact

- Developers must remember which assistant produced the relevant context.
- Grok CLI search is isolated from Terraphim's cross-source session search.
- Terraphim cannot enrich Grok conversations with role concepts or return them alongside Claude Code, OpenCode, Codex, Aider, and Cline results.
- AI agents cannot rely on one canonical local session-search command.

### Success Criteria

1. `terraphim-agent sessions sources` detects Grok when `~/.grok/sessions/session_search.sqlite` exists or when `GROK_HOME` points to an equivalent directory.
2. `terraphim-agent sessions search <query>` queries Grok's SQLite FTS store read-only without copying rows into Terraphim's session cache or writing a second Terraphim index.
3. Terraphim returns Grok matches alongside other session sources with source metadata, title, project path, score, and snippet.
4. Terraphim preserves Grok as the owner of its own search store; no import, migration, or mutation of Grok data is required.
5. Tests use real temporary SQLite databases, not mocks.

## Current State Analysis

### Repository Update State

| Item | Result |
|------|--------|
| Current branch | `main` |
| Pull action | `git fetch --all --prune`, then `git merge --ff-only origin/main` |
| New local HEAD | `101532826` |
| Relevant remotes updated | `origin/main`, `gitea/main` |
| Remote fetch caveat | Remote `terraphim` failed with `Repository not found`; it is not a relevant source for this plan. |
| Existing untracked file | `.docs/handover-2026-07-06.md`, left untouched. |
| Merge hook caveat | Post-merge hook reported `Failed to import bd changes after merge`; tracked code was fast-forwarded successfully. |

### Existing Implementation

| Component | Location | Purpose |
|-----------|----------|---------|
| Session model | `crates/terraphim_sessions/src/model.rs` | Unified `Session`, `Message`, `SessionMetadata`, and message roles. |
| Connector trait and registry | `crates/terraphim_sessions/src/connector/mod.rs` | Source detection, import options, feature-gated connector registration. |
| Native Claude connector | `crates/terraphim_sessions/src/connector/native.rs` | Always-available Claude Code JSONL importer and file watcher. |
| OpenCode connector | `crates/terraphim_sessions/src/connector/opencode.rs` | SQLite and legacy JSONL import pattern. |
| Codex connector | `crates/terraphim_sessions/src/connector/codex.rs` | JSONL session importer closest to Grok's per-session files. |
| Search adapter | `crates/terraphim_sessions/src/search.rs` | Converts sessions to `Document`, scores with BM25, and optionally boosts KG matches. |
| Session service | `crates/terraphim_sessions/src/service.rs` | Auto-import, cache, search, statistics, source filtering. |
| REPL command parser | `crates/terraphim_agent/src/repl/commands.rs` | Parses `/sessions search`, `/sessions import`, `/sessions list`, etc. |
| REPL session handler | `crates/terraphim_agent/src/repl/handler.rs` | Formats session source, import, list, search, stats, show, and expand output. |
| Session-search specification | `docs/specifications/terraphim-agent-session-search-spec.md` | Existing goals for cross-agent search, robot mode, aliases, token-aware output, and KG enrichment. |

### Current Terraphim Search Flow

```text
terraphim-agent sessions search <query>
  -> ReplCommand::Sessions { Search { query } }
  -> ReplHandler::handle_sessions
  -> static SessionService
  -> maybe_auto_import
  -> ConnectorRegistry::import_all
  -> SessionService::search or search_with_thesaurus
  -> search_sessions or search_sessions_hybrid
  -> human-readable table output
```

### Current Search Behaviour

- `search_sessions` builds a `Document` per session from project path, model, and every non-empty message, capped at `MAX_BODY_LENGTH = 50_000` bytes.
- Results are capped at `MAX_SEARCH_RESULTS = 50` and filtered to scores at least `10%` of the top score.
- With `enrichment`, `search_sessions_hybrid` applies KG concept boosting when a role thesaurus is available.
- Without `search-index`, `SessionService` falls back to substring matching over title, project path, and message content.

### Grok CLI Search Baseline

Observed CLI surface:

```text
grok sessions search [OPTIONS] <QUERY>
  -n, --limit <LIMIT>    default 20
```

Help text states that Grok search searches "summaries and first prompts". A sample local command returned one matching session for `session search`. Grok also exposes `grok sessions list --limit <n>`.

### Grok Local Storage Findings

Grok stores session data under `~/.grok/sessions` by default and supports `GROK_HOME` according to its local documentation.

Observed files:

```text
~/.grok/sessions/session_search.sqlite
~/.grok/sessions/<url-encoded-cwd>/prompt_history.jsonl
~/.grok/sessions/<url-encoded-cwd>/<session-id>/summary.json
~/.grok/sessions/<url-encoded-cwd>/<session-id>/updates.jsonl
~/.grok/sessions/<url-encoded-cwd>/<session-id>/chat_history.jsonl
~/.grok/sessions/<url-encoded-cwd>/<session-id>/signals.json
```

Observed SQLite schema, validated with `sqlite3 ~/.grok/sessions/session_search.sqlite .schema`:

```sql
CREATE TABLE session_docs (
    session_id TEXT PRIMARY KEY,
    cwd TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    last_indexed_offset INTEGER NOT NULL DEFAULT 0
);
CREATE VIRTUAL TABLE session_docs_fts USING fts5(title, content, content='session_docs', content_rowid='rowid');
```

Observed `prompt_history.jsonl` entries contain `timestamp`, `session_id`, `prompt`, and `is_bash`.

Follow-up validation confirmed direct FTS querying works:

```sql
SELECT d.session_id,
       bm25(session_docs_fts) AS score,
       snippet(session_docs_fts, 1, '[', ']', '...', 16) AS snippet
FROM session_docs_fts
JOIN session_docs d ON d.rowid = session_docs_fts.rowid
WHERE session_docs_fts MATCH 'session search'
ORDER BY score
LIMIT 5;
```

The local store contained 29 `session_docs` rows at validation time. Some rows have empty `content`, so the adapter must tolerate title-only entries.

## Constraints

### Technical Constraints

- `terraphim_sessions` is in the workspace; `crates/terraphim_agent` is currently excluded from workspace builds but still present and used by the installed CLI path.
- The current `SessionConnector` trait is import-oriented, so direct Grok querying should be introduced as a separate search-provider extension rather than forcing Grok into the import/cache model.
- SQLite support already exists through optional `rusqlite` for the OpenCode connector, so a Grok direct-query adapter can reuse that dependency behind a `grok-search` feature.
- Tests must not use mocks. Use temporary directories, real JSONL files, and real SQLite databases.
- Search should stay local and privacy-first; do not shell out to `grok`, mutate Grok's SQLite database, or duplicate Grok's search store.

### Business Constraints

- The design should minimise maintenance by reusing `SessionConnector`, `SessionService`, and `search_sessions` rather than adding a parallel search engine.
- The implementation should be incremental: direct-query adapter first, result merging second, CLI ergonomics third.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Local-only operation | No network or Grok API calls | Terraphim and Grok both store local files. |
| No duplicate index | Query Grok's FTS store in place | `session_search.sqlite` already contains `session_docs_fts`. |
| Read-only safety | Open SQLite with read-only flags | Avoids mutating user-owned Grok data. |
| Performance | Use Grok FTS for Grok results | Avoids loading every Grok row into Terraphim memory. |

## Vital Few

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Add a direct-query Grok search adapter instead of shelling out or importing | Keeps Terraphim the canonical search path while preserving Grok's search store ownership | Grok SQLite FTS schema is accessible and queryable. |
| Preserve a unified result model | Allows Grok hits to appear beside existing Terraphim session hits | Existing CLI needs source/title/snippet/session ID fields. |
| Use real SQLite artefacts in tests | Prevents format drift and complies with project test rules | No mocks are allowed. |

## Eliminated From Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Replacing BM25 scoring | Existing scoring already works and supports KG boost. |
| Calling `grok sessions search` at runtime | Would make Terraphim depend on an external CLI and lose result-shaping control. |
| Importing Grok `session_docs` rows into Terraphim cache | Duplicates an existing FTS store and creates synchronisation/staleness risk. |
| Importing file snapshots or rewind points | High privacy and size cost; not needed for session search parity. |
| Real-time Grok watching in the first change | Direct SQLite queries always see Grok's current indexed state. |
| Changing Grok's own index | Out of Terraphim scope and creates upgrade risk. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| New direct search provider trait | Required extension point for Grok FTS queries without import | Medium |
| `SessionService::search` | Needs to merge cached session results with direct-provider results | Medium |
| Result projection type | Needed to represent source/title/snippet/score without requiring full `Session` import | Medium |
| REPL parser and handler | Needed for source filtering and display improvements | Medium, because command parser currently has limited flags for search. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `rusqlite` | Existing optional `0.32` bundled | Low, already used by OpenCode connector | Shell out to `sqlite3`, rejected. |
| Grok storage format | CLI-managed, not versioned here | Medium, may change | Prefer documented per-session JSON files and tolerate missing fields. |

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Grok file format changes | Medium | Medium | Prefer tolerant JSON parsing and use `session_search.sqlite` only as an optional fast path. |
| SQLite `content` duplicates or summarises rather than preserving full history | Medium | Medium | Treat Grok parity target as querying Grok's own search store; richer raw-log indexing is explicitly out of scope unless Grok exposes it in SQLite. |
| Importing tool outputs leaks too much sensitive data into search body | Medium | High | Limit first pass to prompts and assistant text; include tool names and compact summaries, not raw large payloads. |
| Current `terraphim-agent sessions search` emits noisy Cursor warnings | High | Low for Grok, medium for UX | Track separately or suppress unavailable Cursor connector warnings; not required for Grok connector. |
| `crates/terraphim_agent` excluded from workspace complicates CLI verification | Medium | Medium | Test `terraphim_sessions` crate directly; verify installed CLI separately if agent crate cannot be built in workspace. |

### Open Questions

1. Should the direct Grok search adapter be enabled by default under `full`, or should it require an explicit `grok-search` feature only?
2. What unified result type should carry both cached `Session` hits and direct SQLite hits without requiring Grok import?
3. Should `terraphim-agent sessions search` grow a `--source grok` and `--limit` parser in the first implementation?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Grok stores its search DB under `GROK_HOME/sessions` or `~/.grok/sessions` | Grok docs and observed filesystem | Adapter misses sessions if installation differs | Partially |
| `session_search.sqlite` is sufficient for the first Terraphim/Grok integration | Validated schema and FTS query | Some rich raw conversation fields may not be available | Yes |
| Terraphim should not shell out to `grok` for normal search | Terraphim privacy-first architecture and unified CLI goal | CLI fallback might be simpler but weaker | Yes |
| Terraphim should not import/copy Grok's search store | Existing SQLite FTS already solves storage/indexing | Direct result merging requires a small service change | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| "Update session search over Grok CLI search" means replace Grok's search implementation | Requires modifying Grok itself, outside this repo | Rejected. |
| It means make Terraphim use `grok sessions search` as a backend | Fast but external CLI-dependent and loses result-shaping control | Rejected. |
| It means import Grok sessions into Terraphim's cache/index | Fits existing import connectors but duplicates Grok's search store | Rejected after validation. |
| It means make Terraphim query Grok's local SQLite search store directly and merge results | Avoids duplicate storage and preserves Grok ownership | Chosen. |

## Research Findings

1. `terraphim_sessions` is already the correct extension point. It exposes `SessionConnector`, `ConnectorRegistry`, `SessionService`, and BM25/KG search.
2. Grok has enough local data for direct search integration: `session_search.sqlite` contains `session_docs`, `session_docs_fts`, content hashes, and triggers.
3. A direct SQLite adapter is preferable to import because it avoids duplicate storage, stale cached copies, and ownership ambiguity.
4. The smallest viable implementation is a feature-gated `GrokSearchProvider` that opens `session_search.sqlite` read-only, runs FTS queries, and returns transient session-search hits.
5. CLI search ergonomics lag behind the original session-search spec: parser support for `--source`, `--limit`, robot output, and richer result snippets should be planned around a unified search-result type.

## Recommendations

### Proceed

Proceed with a direct-query adapter implementation. Do not import Grok rows into Terraphim's session cache, do not create a second Terraphim index for Grok data, and do not wrap `grok sessions search`.

### Scope Recommendations

Phase 1 implementation should include:

- A feature-gated `grok-search` adapter in `terraphim_sessions`.
- Source detection using `GROK_HOME` then `~/.grok`.
- Read-only FTS queries against `session_search.sqlite`.
- Result merging between Terraphim cached session hits and Grok direct-query hits.
- Direct unit/integration tests with temporary real SQLite databases.
- Documentation updates showing `terraphim-agent sessions search` as the preferred path; no `sessions import --source grok` workflow.

Phase 2 follow-up should include:

- CLI flags for `sessions search --source grok --limit n`.
- Robot output for session search if not already available in the active CLI path.
- Optional watcher support after format stability is proven.

### Risk Mitigation Recommendations

- Open Grok SQLite with read-only flags.
- Normalise FTS `bm25()` scores before merging with Terraphim BM25 scores.
- Keep Grok-specific result metadata in a result projection rather than expanding `Session` unless a full-session view is needed.
- Add regression fixtures for SQLite-only Grok storage, including empty `content` rows and FTS snippets.

## Next Steps

1. Review and approve `.docs/design-3084-session-search-over-grok-cli.md`.
2. Decide whether `grok-search` belongs in `full` immediately or behind explicit opt-in for one release.
3. Implement with tests in a dedicated task branch, referencing #3084 or a follow-up implementation issue.
