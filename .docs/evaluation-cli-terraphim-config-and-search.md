# CLI Binary .terraphim/ Config Support and Search Backend Evaluation

## Date: 2025-05-25

## Executive Summary

This document evaluates all CLI binaries in the Terraphim project for:
1. **`.terraphim/` project-local config discovery support**
2. **Search backend usage (ripgrep vs fff-search)**

## Findings: .terraphim/ Config Support

| Binary | File | Supports `.terraphim/` | Implementation |
|--------|------|------------------------|----------------|
| `terraphim-grep` | `crates/terraphim_grep/src/main.rs` | YES | `discover_project_dir()` → `ProjectConfig::load_from_dir()` → role resolution → thesaurus + role-config lookup |
| `terraphim-agent` | `crates/terraphim_agent/src/service.rs` | YES | `merge_project_config()` called in `TuiService::new()` for all config paths |
| `terraphim-cli` | `crates/terraphim_cli/src/service.rs` | **YES** (fixed in this PR) | `merge_project_config()` added to `CliService::new()` persistence path and `new_with_embedded_defaults()` |
| `terraphim-mcp-server` | `crates/terraphim_mcp_server/src/main.rs` | YES | `discover(None)` → `ProjectConfig::load_from_dir()` → `merge_project_into_base()` before profile selection |
| `adf` | `crates/terraphim_orchestrator/src/bin/adf.rs` | N/A | Uses `.terraphim/adf.toml` (different config system) |
| `adf-ctl` | `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | N/A | SSH remote control, no project config needed |

### terraphim-cli Fix Applied

The `CliService` was missing project config merging. Added `merge_project_config()` method identical to `TuiService::merge_project_config()`:

- Called after loading from persistence (line 100)
- Called after bootstrapping from `role_config` JSON (line 157)
- Called in `new_with_embedded_defaults()` (line 176)
- **Not called** when `--config` flag is explicitly provided (user override)

## Findings: Search Backend Evaluation

### Current Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  terraphim-grep │────▶│  HybridSearcher  │────▶│   fff-search    │
│  (standalone)   │     │  (in-grep crate) │     │  (FilePicker)   │
└─────────────────┘     └──────────────────┘     └─────────────────┘

┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ terraphim-agent │────▶│ TerraphimService │────▶│  search_haystacks│
│  terraphim-cli  │     │                  │     │  (middleware)   │
│  terraphim_server│    │                  │     │                 │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                          │
                              ┌───────────────────────────┼───────────┐
                              ▼                           ▼           ▼
                         ┌─────────┐                ┌──────────┐  ┌────────┐
                         │ Ripgrep │                │ QueryRs  │  │  MCP   │
                         │Indexer  │                │Indexer   │  │Indexer │
                         │(spawns  │                │(API call)│  │(tool)  │
                         │ rg bin) │                └──────────┘  └────────┘
                         └─────────┘

┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   mcp-server    │────▶│    McpService    │────▶│   fff-search    │
│                 │     │                  │     │ (find_files,    │
│                 │     │                  │     │  grep_files)    │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

### Backend Usage by Binary

| Binary | Search Backend | Notes |
|--------|---------------|-------|
| `terraphim-grep` | **fff-search** | Uses `FilePicker::grep()` directly with KG boosting |
| `terraphim-agent` | **ripgrep** (via middleware) | `RipgrepIndexer` spawns `rg` process |
| `terraphim-cli` | **ripgrep** (via middleware) | Same path as agent |
| `terraphim-mcp-server` | **fff-search** | `McpService::find_files()` / `grep_files()` use fff |
| `terraphim_server` | **ripgrep** (via middleware) | HTTP API → `TerraphimService::search()` → `search_haystacks()` |

### RipgrepIndexer Analysis

**File:** `crates/terraphim_middleware/src/indexer/ripgrep.rs`

The `RipgrepIndexer`:
1. Spawns the `rg` binary as a subprocess
2. Parses JSON output from ripgrep
3. Only indexes `.md` files (line 57: `ext == "md"`)
4. Has caching via `cached` macro
5. Supports extra parameters for tag filtering

**Limitations:**
- Only Markdown files are indexed (code files are ignored)
- Spawns external process (overhead)
- No frecency scoring
- No KG path scoring
- No cursor pagination

### fff-search Advantages

The `fff-search` crate (used by terraphim-grep and MCP server) provides:
1. **Unified file discovery** — walks filesystem with git-awareness
2. **Frecency scoring** — LMDB-backed usage tracking
3. **KG path scoring** — `KgPathScorer` boosts files matching KG concepts
4. **Cursor pagination** — base64-encoded offsets for large result sets
5. **Aho-Corasick matching** — O(n) multi-pattern search
6. **No external dependencies** — pure Rust, no subprocess

## Recommendations

### Immediate (This PR)

1. **✅ terraphim-cli .terraphim/ support** — DONE (added `merge_project_config()`)

### Short-term (Follow-up Issue)

2. **Replace RipgrepIndexer with fff-search in middleware**
   - Add `fff-search` dependency to `terraphim_middleware`
   - Create `FffIndexer` implementing `IndexMiddleware`
   - Support all file types (not just `.md`)
   - Wire KG path scorer for KG-boosted file discovery
   - Maintain caching layer

3. **ServiceType consolidation**
   - Consider deprecating `ServiceType::Ripgrep` in favor of `ServiceType::Fff` or similar
   - Keep `Ripgrep` as alias for backward compatibility

### Benefits of Migration

| Aspect | Ripgrep | fff-search | Impact |
|--------|---------|------------|--------|
| File types | Markdown only | All text files | +Code search |
| Scoring | None | Frecency + KG | +Relevance |
| Performance | Process spawn | In-process | +Latency |
| Pagination | None | Cursor-based | +UX |
| Caching | Memory only | LMDB persistent | +Cross-session |

## Verification Commands

```bash
# Verify terraphim-cli picks up project roles
cargo run -p terraphim-cli -- roles list

# Verify terraphim-grep uses fff-search
cargo run -p terraphim_grep --features code-search -- --role rust-engineer async

# Verify MCP server discovers project config
cargo run -p terraphim_mcp_server -- --profile desktop
```

## Appendix: Config Discovery Code Paths

### terraphim-grep
```
main.rs:306  load_project_config()
    → project::discover(None)
    → ProjectConfig::load_from_dir()
    → resolve_role_name()
        → explicit --role > selected_role > default_role > single role
```

### terraphim-agent
```
service.rs:31  TuiService::new()
    → merge_project_config()
        → project::discover(None)
        → ProjectConfig::load_from_dir()
        → ConfigBuilder::merge_with()
```

### terraphim-cli
```
service.rs:32  CliService::new()
    → merge_project_config()  [NEW]
        → project::discover(None)
        → ProjectConfig::load_from_dir()
        → ConfigBuilder::merge_with()
```

### terraphim-mcp-server
```
main.rs:182  main()
    → project::discover(None)
    → ProjectConfig::load_from_dir()
    → merge_project_into_base()
        → repair_selected_roles()
```
