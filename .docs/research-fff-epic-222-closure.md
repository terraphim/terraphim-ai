# Research Document: Close FFF Epic #222 -- Remaining Work

**Status**: Approved
**Author**: opencode (disciplined-research)
**Date**: 2026-04-17
**Related**: Gitea #222 (epic), #223 (closed), #224 (Phase 2 remaining), #225/#226 (stale), #227 (Phase 3 done)

## Executive Summary

FFF integration is ~80% complete. Phase 1 (sidecar) and Phase 3 (KG-boosted scoring crate) are done. Four remaining items block epic closure: `terraphim_multi_grep` MCP tool, SharedFrecency persistence, cursor pagination, and sidecar removal. All are well-understood with clear reference implementations in fff-mcp.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Closes a major epic, cleans up technical debt |
| Leverages strengths? | Yes | Pattern already established in terraphim_grep/find_files |
| Meets real need? | Yes | Agents need multi-pattern search and persistent frecency |

**Proceed**: Yes (3/3)

## Current State Analysis

### Code Locations
| Component | Location | Status |
|-----------|----------|--------|
| terraphim_file_search crate | `crates/terraphim_file_search/` | Done: lib.rs, kg_scorer.rs, config.rs, watcher.rs |
| MCP terraphim_find_files | `crates/terraphim_mcp_server/src/lib.rs:1170-1244` | Done |
| MCP terraphim_grep | `crates/terraphim_mcp_server/src/lib.rs:1250-1336` | Done |
| MCP terraphim_multi_grep | -- | **Missing** |
| SharedFrecency wiring | `crates/terraphim_mcp_server/src/lib.rs:59` | **Not wired** |
| Cursor pagination | `crates/terraphim_mcp_server/src/lib.rs` (next_cursor: None) | **Not implemented** |
| fff-mcp sidecar | bigbox: PID running | **Not removed** |

### Reference: fff-mcp multi_grep Implementation
Location: `~/.cargo/git/checkouts/fff.nvim-14ad43e6a8691b70/efd1552/crates/fff-mcp/src/server.rs:545-594`

Key API: `grep::multi_grep_search(files, &patterns_refs, constraints, &options, budget, None)`
- Takes `Vec<&str>` patterns (OR logic)
- Uses same `GrepSearchOptions` as single grep
- Returns same `GrepResult` type
- Has `CursorStore` for pagination

### Reference: fff-core SharedFrecency
Location: `~/.cargo/git/checkouts/fff.nvim-14ad43e6a8691b70/efd1552/crates/fff-core/src/shared.rs:120`

```rust
pub struct SharedFrecency(pub(crate) Arc<RwLock<Option<FrecencyTracker>>>);
```
- Already exported from `fff-search` crate
- Current terraphim MCP: `SharedFrecency` is imported but field `frecency` not used (0 references to it)

### Existing MCP Tool Pattern (for adding new tools)
1. Add async method on `McpService` (e.g., `find_files`, `grep_files`)
2. Add `Tool` entry in `ServerHandler::get_info()` (~line 1722, 1749)
3. Add match arm in `ServerHandler::call_tool()` (~line 2139, 2159)

## Remaining Work Items (from #224)

| Item | Effort | Dependencies | Parallelizable? |
|------|--------|-------------|-----------------|
| 1. Add terraphim_multi_grep MCP tool | 1-2h | None | Yes |
| 2. Wire SharedFrecency with LMDB persistence | 2-3h | fff-search exposes FrecencyTracker | Yes |
| 3. Add cursor-based pagination | 2-3h | None | Yes |
| 4. Remove standalone fff-mcp sidecar | 30min | Items 1-3 validated | No (last) |
| 5. Close stale sub-issues #225, #226 | 5min | None | Yes |
| 6. Update epic #222 and close #224 | 5min | All above | No (last) |

## Constraints

### Technical
- `fff-search` is a git dependency (branch `feat/external-scorer`) -- cannot modify its API
- `multi_grep_search` is already public in `fff-search::grep` -- no fork needed
- `SharedFrecency` requires an LMDB path -- must be configurable
- MCP server uses `rmcp` 0.9 for protocol -- tool registration pattern is fixed
- `CursorStore` in fff-mcp uses opaque string IDs -- we can replicate or simplify

### Vital Few

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| multi_grep_search API is public | Enables multi-pattern tool without forking | fff-core/src/grep.rs:870 |
| Existing tools are the pattern | New tools follow find_files/grep pattern | lib.rs:1170-1336 |
| SharedFrecency already imported | Just needs wiring, not new code | lib.rs:7 imports it |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Upstream ExternalScorer trait contribution | Not blocking, separate effort |
| KG content scoring (score file contents, not just paths) | Future enhancement, not in epic scope |
| Query completion tracking | Nice-to-have, frecency is sufficient |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| LMDB path not writable on bigbox | Low | Medium | Configurable path, fallback to temp |
| multi_grep_search API differs from grep_search | Low | Low | Read fff-mcp server.rs reference |
| Cursor pagination state lost on MCP restart | Medium | Low | Expected; cursors are ephemeral |
| fff-mcp sidecar still used by other tools | Medium | Medium | Check before removing |

## Assumptions

| Assumption | Basis | Risk if Wrong |
|------------|-------|---------------|
| `multi_grep_search` has same signature style as `grep_search` | Both in fff-core/src/grep.rs, same author | Low -- signature differs only in `Vec<&str>` vs single pattern |
| SharedFrecency can be initialised with a path | fff-core has `init_db(path)` function | Medium -- may need to check if LMDB is available on target |
| fff-mcp sidecar is not used by other projects | Only terraphim-ai configured it | Medium -- verify before removing |
