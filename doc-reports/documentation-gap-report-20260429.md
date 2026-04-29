## Documentation Gap Report

**Generated:** 2026-04-29T$(date -u +%H:%M:%SZ)
**Agent:** documentation-generator (Ferrox)
**Previous Report:** 2026-04-28

### Summary

| Metric | 2026-04-28 | 2026-04-29 | Delta |
|--------|------------|------------|-------|
| Crates scanned | ~45 | ~45 | -- |
| Crates with module docs | ~18 | ~22 | +4 |
| Crates missing module docs | 27 | 23 | -4 |
| rustdoc warnings | ~60 | 43 | -17 |

### Module Documentation Status

23 crates still lack `//!` module documentation in `lib.rs`:

**Core Types and Services:**
- `crates/terraphim_service/src/lib.rs` -- LLM service abstractions
- `crates/terraphim_settings/src/lib.rs` -- Device and user settings
- `crates/terraphim_config/src/lib.rs` -- Configuration management
- `crates/terraphim_persistence/src/lib.rs` -- Persistence traits

**Agent Infrastructure:**
- `crates/terraphim_agent/src/lib.rs` -- Main agent crate
- `crates/terraphim_mcp_server/src/lib.rs` -- MCP server
- `crates/terraphim_middleware/src/lib.rs` -- Middleware components

**Haystack Providers:**
- `crates/haystack_core/src/lib.rs` -- Provider trait
- `crates/haystack_atlassian/src/lib.rs` -- Confluence/Jira
- `crates/haystack_discourse/src/lib.rs` -- Discourse forum
- `crates/haystack_grepapp/src/lib.rs` -- Grep.app search
- `crates/haystack_jmap/src/lib.rs` -- JMAP protocol

**Knowledge Graph:**
- `crates/terraphim_rolegraph/src/lib.rs` -- Role graph core
- `crates/terraphim_atomic_client/src/lib.rs` -- Atomic Data client

**Utilities and Tools:**
- `crates/terraphim_file_search/src/lib.rs`
- `crates/terraphim-markdown-parser/src/lib.rs`
- `crates/terraphim_build_args/src/lib.rs`
- `crates/terraphim_ccusage/src/lib.rs`
- `crates/terraphim_usage/src/lib.rs`
- `crates/terraphim_onepassword_cli/src/lib.rs`
- `crates/terraphim_kg_linter/src/lib.rs`
- `crates/terraphim_lsp/src/lib.rs`

**Bindings and FFI:**
- `crates/terraphim_automata_py/src/lib.rs`
- `crates/terraphim_rolegraph_py/src/lib.rs`

### rustdoc Warnings by Crate

| Crate | Warnings | Categories |
|-------|----------|------------|
| `terraphim_orchestrator` | 14 | Unresolved links, private item refs, unclosed HTML |
| `terraphim_middleware` | 5 | Unresolved links, non-hyperlink URLs, unclosed HTML |
| `terraphim_types` | 3 | Unresolved links, non-hyperlink URLs |
| `terraphim_persistence` | 2 | Unresolved links |
| `terraphim_rolegraph` | 2 | Unresolved links |
| `terraphim_tracker` | 2 | Unresolved links, non-hyperlink URLs |
| `terraphim_service` | 1 | Unresolved link |
| `terraphim_router` | 1 | Unresolved link |
| `terraphim_file_search` | 1 | Non-hyperlink URL |
| `terraphim_tinyclaw` | 1 | Non-hyperlink URL |

### CHANGELOG Status

CHANGELOG.md updated: added compliance audit entry (Refs #1071).

### Recommendations

**Priority 1:** Fix rustdoc warnings in `terraphim_orchestrator` (14 warnings)
**Priority 2:** Add module docs to core infrastructure (`terraphim_service`, `terraphim_config`, `terraphim_persistence`)
**Priority 3:** Add module docs to haystack providers (all `haystack_*` crates)
**Priority 4:** Resolve remaining rustdoc unresolved links across workspace

Theme-ID: doc-gap

@adf:reviewer please action this finding.
