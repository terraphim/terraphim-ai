# Implementation Plan: Close FFF Epic #222

**Status**: Approved
**Research Doc**: `.docs/research-fff-epic-222-closure.md`
**Author**: opencode (disciplined-design)
**Date**: 2026-04-17
**Estimated Effort**: 4-6 hours

## Overview

### Summary
Complete the 4 remaining items to close FFF Epic #222: add `terraphim_multi_grep` MCP tool, wire SharedFrecency persistence, add cursor pagination, and remove fff-mcp sidecar.

### Approach
Follow the exact pattern established by `terraphim_find_files` and `terraphim_grep`. Three workstreams can execute in parallel since they touch different parts of the MCP server.

### Scope
**In Scope:**
1. Add `terraphim_multi_grep` MCP tool (OR-pattern grep)
2. Wire `SharedFrecency` with configurable LMDB path
3. Add cursor-based pagination to all three tools
4. Remove fff-mcp sidecar from bigbox
5. Close stale sub-issues #225, #226

**Out of Scope:**
- Upstream ExternalScorer trait PR to fff.nvim
- KG content scoring (path-only scoring stays)
- Query completion tracking

**Avoid At All Cost:**
- Refactoring existing find_files/grep implementations (they work)
- Adding configuration file format changes (use env vars/CLI args)
- Abstracting MCP tool registration (YAGNI -- 3 tools is fine)

## Architecture

### Data Flow
```
MCP Client
  -> terraphim_find_files  (fuzzy match + KG boost)
  -> terraphim_grep         (content search + KG file ordering + cursor)
  -> terraphim_multi_grep   (multi-pattern OR search + KG file ordering + cursor)
  -> SharedFrecency (LMDB)  (persistent access frequency across sessions)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use `multi_grep_search` directly from fff-core | Already public, no fork needed | Wrapping in our own trait |
| CursorStore as HashMap<String, usize> | Simple, matches fff-mcp pattern | Redis/external store |
| SharedFrecency via configurable env var | `FFF_FRECENCY_PATH` -- zero config change | New config section in TOML |
| Pagination as offset-based | Matches fff-core's `file_offset` in GrepSearchOptions | Keyset pagination |

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_mcp_server/src/lib.rs` | Add multi_grep tool, frecency wiring, cursor store, pagination |
| `crates/terraphim_mcp_server/Cargo.toml` | No changes needed (fff-search already a dep) |

### No new files. No deleted files (sidecar removal is ops, not code).

## Implementation Steps

### Workstream A: terraphim_multi_grep (Parallel -- 1-2h)

**Step A1: Add multi_grep method on McpService**

File: `crates/terraphim_mcp_server/src/lib.rs`
Location: After `grep_files` method (~line 1336)

```rust
pub async fn multi_grep_files(
    &self,
    patterns: Vec<String>,
    path: Option<String>,
    constraints: Option<String>,
    limit: Option<usize>,
    cursor: Option<String>,
    output_mode: Option<String>,
) -> Result<CallToolResult, ErrorData> {
    let base_path = path.unwrap_or_else(|| ".".to_string());
    let max_results = limit.unwrap_or(50);
    let files_only = output_mode.as_deref() == Some("files");

    // Same FilePicker init as grep_files
    let mut picker = FilePicker::new(FilePickerOptions { ... })?;
    picker.collect_files()?;
    let mut files = picker.get_files().to_vec();

    // KG sort (same as grep_files)
    if let Some(scorer) = &self.kg_scorer {
        files.sort_by(|a, b| scorer.score(b).cmp(&scorer.score(a)));
    }

    // Parse constraints
    let patterns_refs: Vec<&str> = patterns.iter().map(|s| s.as_str()).collect();
    let options = GrepSearchOptions { file_offset: cursor_offset, page_limit: max_results, ... };

    let result = multi_grep_search(&files, &patterns_refs, &constraints_parsed, &options, &budget, None);

    // Format output (same pattern as grep_files)
    // Return with next_cursor
}
```

**Step A2: Register tool in get_info()**

Location: After `terraphim_grep` tool entry (~line 1749)

```rust
Tool {
    name: "terraphim_multi_grep".into(),
    description: "Search file contents for lines matching ANY of multiple patterns (OR logic). ...".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "patterns": { "type": "array", "items": { "type": "string" }, "description": "Patterns to match (OR logic)" },
            "path": { "type": "string", "description": "Base directory" },
            "constraints": { "type": "string", "description": "File constraints (e.g. '*.rs !test/')" },
            "limit": { "type": "integer", "description": "Max results (default 50)" },
            "cursor": { "type": "string", "description": "Pagination cursor from previous result" },
            "output_mode": { "type": "string", "enum": ["content", "files"], "description": "Output format" }
        },
        "required": ["patterns"]
    }),
}
```

**Step A3: Add match arm in call_tool()**

Location: After `"terraphim_grep"` arm (~line 2159)

```rust
"terraphim_multi_grep" => { ... }
```

### Workstream B: SharedFrecency Wiring (Parallel -- 2-3h)

**Step B1: Add frecency field and initialization**

File: `crates/terraphim_mcp_server/src/lib.rs`

McpService struct already has `frecency` (check). If not, add:
```rust
pub struct McpService {
    // ... existing fields ...
    frecency: Option<SharedFrecency>,
}
```

Constructor: initialise from env var `FFF_FRECENCY_PATH`:
```rust
let frecency = std::env::var("FFF_FRECENCY_PATH")
    .ok()
    .map(|path| {
        // Init LMDB-backed frecency at path
        SharedFrecency::new(&path) // or equivalent from fff-search API
    })
    .transpose()?;
```

**Step B2: Pass frecency to FilePicker**

In `find_files` and `grep_files` and `multi_grep_files`:
```rust
if let Some(frecency) = &self.frecency {
    picker.update_frecency_scores(frecency);
}
```

### Workstream C: Cursor Pagination (Parallel -- 1-2h)

**Step C1: Add CursorStore to McpService**

```rust
pub struct McpService {
    // ... existing ...
    cursor_store: Arc<Mutex<HashMap<String, usize>>>,
}
```

**Step C2: Add cursor handling to grep_files and multi_grep_files**

Parse `cursor` param -> lookup offset from store.
After results, if more available, generate new cursor token:
```rust
let next_cursor = if result.matches.len() > max_results {
    let token = format!("cur_{}", uuid::Uuid::new_v4());
    self.cursor_store.lock().unwrap().insert(token.clone(), offset + max_results);
    Some(token)
} else {
    None
};
```

Include `next_cursor` in the response Content.

### Workstream D: Cleanup (Sequential -- after A,B,C -- 30min)

**Step D1: Close stale Gitea issues**
- Close #225 (research) with comment "Work completed out of order during Phase 3 implementation"
- Close #226 (design) with same comment

**Step D2: Update #224**
- Comment with completion status
- Check off remaining items

**Step D3: Remove fff-mcp sidecar from bigbox**
- `ssh bigbox` -- check if fff-mcp is still running
- Check if any other tool references it
- Stop service, remove from MCP configs

**Step D4: Close epic #222**

## Test Strategy

### Unit Tests (in terraphim_mcp_server)
| Test | Purpose |
|------|---------|
| `test_multi_grep_multiple_patterns` | Verify OR logic returns files matching any pattern |
| `test_multi_grep_no_matches` | Empty result for non-existent patterns |
| `test_cursor_store_round_trip` | Store and retrieve offset |
| `test_cursor_pagination_limit` | Verify next_cursor only when more results exist |

### Integration Tests
- `cargo test -p terraphim_mcp_server` -- existing tests must still pass
- Manual: invoke `terraphim_multi_grep` with patterns `["sort_by", "sort_by_key"]` and verify results

## Execution Order (Max Parallelism)

```
Time    Workstream A        Workstream B        Workstream C        Workstream D
----    -----------        -----------        -----------        -----------
T+0     A1: multi_grep     B1: frecency init  C1: CursorStore
T+1     A2: register tool  B2: wire to picker C2: pagination
T+2     A3: call_tool arm                      (verify)
T+3     (verify + test)    (verify + test)    (verify + test)
T+4                                                                 D1-D4: cleanup
```

All three workstreams are independent -- they can be three separate commits on one branch, or three parallel branches merged sequentially.
