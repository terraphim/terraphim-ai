# Research Document: Flush Compiled Thesaurus Cache After KG Markdown Edits

## 1. Problem Restatement and Scope

**Problem in our words:**
Terraphim compiles knowledge graph (KG) markdown files into Aho-Corasick automata-backed thesauri and persists them as JSON blobs in SQLite (and other backends). Once cached, these compiled thesauri are never invalidated when their source markdown files change. A developer or agent can edit a KG markdown file, but the running system continues to serve stale mappings until the process restarts or the SQLite database is manually deleted.

**IN scope:**
- Detecting when source KG markdown files have changed relative to the cached thesaurus
- Invalidating and recompiling only the affected role's thesaurus (per-role invalidation)
- Lazy, on-demand invalidation (no background threads or file watchers)
- A CLI subcommand for manual cache flush (`terraphim-agent cache flush [--role ROLE]`)
- Graceful fallback when cache is missing, corrupted, or DB is locked
- Regression test for the stale-cache scenario

**OUT of scope:**
- Real-time file watching (inotify/FSEvents) — proposed as future enhancement behind `file-watch` feature flag
- Incremental thesaurus updates (diffing individual terms)
- Cache TTL or time-based expiry
- Modifying the ripgrep-based builder or markdown parser

## 2. User & Business Outcomes

**User-visible changes:**
- After editing a KG markdown file (e.g., `docs/src/kg/npm.md`), the next call to `terraphim-agent replace` or MCP `replace_matches` returns the updated mappings without requiring process restart
- Agents can edit KG files programmatically and see immediate results in subsequent tool calls
- Developers can manually flush cache via CLI when needed

**Business value:**
- Faster edit-test cycle for KG authoring
- Reduced support burden from "why isn't my synonym working?" questions
- Enables agentic workflows that self-modify knowledge graphs

## 3. System Elements and Dependencies

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| `Logseq` builder | `crates/terraphim_automata/src/builder.rs` | Scans markdown with ripgrep, builds `Thesaurus` | `terraphim_types::Thesaurus`, `terraphim_markdown_parser` |
| `Thesaurus` type | `crates/terraphim_types/src/lib.rs:702` | In-memory representation of synonym mappings | `ahash::AHashMap`, `NormalizedTerm`, `NormalizedTermValue` |
| `Persistable` trait | `crates/terraphim_persistence/src/lib.rs:197` | Generic save/load across storage backends | `opendal::Operator`, `serde_json` |
| Thesaurus persistence | `crates/terraphim_persistence/src/thesaurus.rs` | `Persistable` impl for `Thesaurus` | `Persistable`, `Thesaurus` |
| `DeviceStorage` | `crates/terraphim_persistence/src/lib.rs:41` | Manages operators (SQLite, DashMap, ReDB, etc.) | `DeviceSettings`, `opendal` |
| `TerraphimService` | `crates/terraphim_service/src/lib.rs:124` | Orchestrates thesaurus loading with fallback | `ConfigState`, `RoleGraph`, `Persistable` |
| `ConfigState` | `crates/terraphim_config/src/lib.rs` | Holds role configurations and `RoleGraph`s | `Role`, `KnowledgeGraph`, `KnowledgeGraphLocal` |
| `Role` | `crates/terraphim_config/src/lib.rs:198` | Defines role with KG source paths | `KnowledgeGraph`, `Haystack` |
| `RoleGraph` | `crates/terraphim_rolegraph` | Runtime Aho-Corasick automata from `Thesaurus` | `Thesaurus`, `ahocorasick` |
| `terraphim-agent` CLI | `crates/terraphim_agent/src/main.rs` | Interactive TUI/REPL entry point | `TuiService`, `clap` |
| `terraphim-cli` | `crates/terraphim_cli/src/main.rs` | Non-interactive CLI entry point | `CliService`, `clap` |
| MCP server | `crates/terraphim_mcp_server/src/lib.rs` | MCP tool server exposing `replace_matches` | `TerraphimService` |
| Settings | `crates/terraphim_settings/` | Defines cache paths (SQLite, etc.) | TOML config |

**Data flow (current):**
```
KG markdown files (*.md)
  → Logseq::build() [ripgrep scan]
  → Thesaurus { name, data: HashMap }
  → serde_json::to_string()
  → opendal Operator::write()
  → SQLite `terraphim_kv` (key = "thesaurus_<role>.json")
  → TerraphimService::ensure_thesaurus_loaded()
  → RoleGraph::new() [Aho-Corasick automaton]
  → replace_matches()
```

## 4. Constraints and Their Implications

| Constraint | Why it matters | Implication |
|-----------|---------------|-------------|
| **Per-role invalidation only** | Recompiling all roles on any KG edit is wasteful | Must track which role's thesaurus maps to which source files |
| **Lazy/on-demand** | No background threads means simpler, safer code | Hash check must happen at load time, not on file change event |
| **No `#[cached]` macro modification** | The `cached` crate memoizes `index_inner()` in-process | In-process memoization is separate from persistent cache; both need consideration |
| **SQLite BLOB schema** | Current cache stores entire `Thesaurus` as one JSON blob | Cannot incrementally update; must store hash metadata alongside or separately |
| **Multiple storage backends** | Cache may be in SQLite, DashMap, ReDB, or memory | Hash metadata must be backend-agnostic or use fastest operator only |
| **Graceful degradation** | Cache may be missing, locked, or corrupted | Must always fall back to rebuilding from markdown source |
| **CLI subcommand availability** | Both `terraphim-cli` and `terraphim-agent` exist | Need to decide which binary gets the `cache flush` command (likely `terraphim-agent` as primary interactive tool) |

## 5. Risks, Unknowns, and Assumptions

### Unknowns
1. **How many KG files per role?** — Affects hash computation cost. If a role has hundreds of files, computing combined hash on every `replace` call may add latency.
2. **Is `ensure_thesaurus_loaded()` called on every replace?** — If so, hash check overhead is on the hot path. If thesaurus is cached in `RoleGraph` and reused, hash check frequency is lower.
3. **What is the typical `replace` latency budget?** — Unknown if 1-5ms of hash checking is acceptable.

### Assumptions
- **ASSUMPTION**: KG markdown files are the only source of truth for locally-built thesauri. Remote `automata_path` URLs are not expected to change without explicit reload.
- **ASSUMPTION**: The `Thesaurus` name is derived from `role_name.as_lowercase()`, giving us a stable cache key per role.
- **ASSUMPTION**: `terraphim-agent` is the primary user-facing CLI; `terraphim-cli` is automation-focused and may not need the flush command.

### Risks
| Risk | Severity | De-risking |
|------|----------|-----------|
| Hash computation adds unacceptable latency to every `replace` call | Medium | Benchmark before/after; consider caching hash results in `RoleGraph` or `ConfigState` |
| `#[cached]` macro on `index_inner()` masks file changes within same process | Medium | Clear in-process cache when persistent cache is invalidated |
| Concurrent edits during hash read + build cause race conditions | Low | SQLite WAL mode already handles concurrent reads; build is idempotent |
| Cache flush CLI fails silently or affects wrong role | Low | Explicit confirmation/logging; role-scoped keys prevent cross-contamination |

## 6. Context Complexity vs. Simplicity Opportunities

**Sources of complexity:**
1. **Multiple storage backends** — The `Persistable` abstraction writes to all backends but reads from fastest. Adding hash metadata must work across SQLite, DashMap, ReDB, etc.
2. **In-process memoization** — `#[cached]` on `index_inner()` adds a second layer of caching that could mask persistent cache invalidation.
3. **Multiple entry points** — `terraphim-agent`, `terraphim-cli`, and MCP server all load thesaurus differently.

**Simplification strategies:**
1. **Single hash per role, not per file** — Instead of tracking individual file hashes, compute a single combined hash of all files in the KG directory. Simpler, slightly coarser (any file change triggers recompile), but avoids per-file metadata.
2. **Store hash in cache key namespace** — Use a separate key like `thesaurus_<role>_hash` in the same KV store, rather than modifying the `Thesaurus` struct. Avoids schema changes and backend compatibility issues.
3. **Invalidate, don't update** — On hash mismatch, delete the old cache entry and rebuild. No need for partial updates or complex merging.

## 7. Questions for Human Reviewer

1. **Is a combined directory hash acceptable, or do we need per-file granularity?** A combined hash is simpler but will recompile if ANY file in the role's KG directory changes, even unrelated ones. Per-file granularity requires tracking which files belong to which thesaurus entries.

2. **Should the hash check happen in `TerraphimService::ensure_thesaurus_loaded()` or in `Thesaurus::load()`?** The former is more explicit and testable; the latter is more transparent but couples cache invalidation to the persistence layer.

3. **Should `terraphim-cli` also get the `cache flush` subcommand, or only `terraphim-agent`?** The CLI is automation-focused; agents/scripts may want programmatic flush.

4. **What hash algorithm?** SHA-256 is proposed but may be overkill. xxHash or Blake3 would be faster for file content hashing. Is cryptographic strength needed?

5. **Should we clear the `#[cached]` in-process memoization on cache invalidation?** The `cached` crate memoizes `index_inner()` by `(name, messages)` tuple. If markdown changes but cache is invalidated, the in-process cache could still return stale data until process restart.

6. **How should the hash be computed — file content only, or content + mtime?** Content hash is robust but slower; mtime is fast but unreliable across filesystems.

7. **Should there be a way to disable hash checking (e.g., for performance-critical deployments)?** A feature flag or config option could skip hash checks.

8. **What is the expected latency budget for `replace_matches`?** This determines whether 1-5ms of hash checking is acceptable.

9. **Should the cache flush command also clear the in-process `RoleGraph` cache, or just the persistent cache?** The issue description mentions "without requiring a manual cache flush or process restart", suggesting both layers need clearing.

10. **Are there existing integration tests for thesaurus persistence that we should extend?** The file `crates/terraphim_service/tests/thesaurus_persistence_test.rs` exists — should we add the regression test there?
