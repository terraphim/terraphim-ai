# terraphim_mcp_search

MCP (Model Context Protocol) tool and Terraphim skill indexing & discovery,
using `terraphim_automata`'s Aho-Corasick pattern matching for sub-100-entry
searches under 70ms.

## What this crate gives you

| Item | When to use |
|------|-------------|
| [`McpToolIndex`] | You want a persistent, mutable, large-corpus tool index with save/load. |
| [`mcp_search_tools`] | You have a slice of tools and want a one-shot search without managing state. |
| [`SkillEntry`] + [`mcp_search_skills`] | You want to search Terraphim skills (TinyClaw JSON, etc.) with the same engine. |

## Why this crate exists

Extracted from `terraphim_agent::mcp_tool_index` (sibling `terraphim-agents`
polyrepo, version 1.20.x) on **2026-06-28**, as part of the Mega-MCP / Tool
Search adaptation plan for terraphim-ai. `mcp_search_tools` and
`mcp_search_skills` were added the same day as the first consumers of the
extracted index.

The extraction of `McpToolIndex` is **source-verbatim** to preserve the
existing API contract and all 9 unit tests + 7 doctests unchanged.

The new search helpers are stateless conveniences over `McpToolIndex` —
they build an ephemeral index per call. For repeated searches over a stable
corpus, build a `McpToolIndex` directly.

## Status

- **Here in `terraphim-ai`:** functional, tested, clippy-clean, ready for new
  consumers in this workspace.
- **In `terraphim_agent` (sibling repo):** the original `mcp_tool_index` module
  is **still present and unchanged** — see Gitea issue
  [`terraphim-agents#64`](https://git.terraphim.cloud/terraphim/terraphim-agents/issues/64)
  for the dependency-swap migration plan.

## Usage

```rust
use terraphim_mcp_search::{
    McpToolIndex, mcp_search_tools, mcp_search_skills, SkillEntry,
};
use terraphim_types::McpToolEntry;
use std::path::PathBuf;

// State-managed: persistent, mutable index
let mut index = McpToolIndex::new(PathBuf::from("/tmp/mcp-tools.json"));
index.add_tool(McpToolEntry::new("search_files", "Search files", "filesystem"));
index.add_tool(McpToolEntry::new("read_file",    "Read file contents", "filesystem"));
let hits = index.search("search");
// hits[0].name == "search_files"

// Stateless: one-shot slice search
let tools = index.tools().to_vec();
let hits = mcp_search_tools("search", &tools);
// hits is Vec<McpToolEntry> (owned)

// Skill search via the same engine
let skills = vec![
    SkillEntry::new("code-review", "Automated code review")
        .with_tags(vec!["review".into()]),
    SkillEntry::new("deploy",      "Deploy to staging"),
];
let skill_hits = mcp_search_skills("review", &skills);
// skill_hits[0].name == "code-review"
```

## Performance NFR

- **Search latency**: < 70 ms for 100 tools (`McpToolIndex::test_discovery_latency_benchmark`,
  release-mode only).
- **Stateless helpers**: ephemeral index in `/tmp/terraphim-mcp-search-ephemeral.json`,
  never persisted.
- **Memory**: proportional to corpus size.

## Migration plan (the "C" of "A/B/C")

| Step | Where | Who | Status |
|------|-------|-----|--------|
| 1. Create standalone crate | `terraphim-ai` workspace | done 2026-06-28 | ✅ |
| 2. Add stateless search helpers | `terraphim-ai` workspace | done 2026-06-28 | ✅ |
| 3. File Gitea issue in `terraphim-agents` | `terraphim-agents` repo | done 2026-06-28 | ✅ |
| 4. Update `terraphim_agent` Cargo.toml to depend on `terraphim_mcp_search` | `terraphim-agents` polyrepo | TBD | ⏳ |
| 5. Re-export `terraphim_agent::mcp_tool_index` as a deprecation shim pointing at this crate | `terraphim-agents` polyrepo | TBD | ⏳ |
| 6. Eventually remove the legacy module | `terraphim-agents` polyrepo | TBD | ⏳ |

Steps 4-6 belong in the `terraphim-agents` polyrepo — **do not** execute them
from this workspace. They're out of scope per the surgeon pre-cut check.

## Dependencies

- `terraphim_automata = "1.20.2"` (from terraphim registry)
- `terraphim_types = "1.20.2"` (from terraphim registry, for `McpToolEntry`)
- `serde`, `serde_json`, `tempfile` (dev)