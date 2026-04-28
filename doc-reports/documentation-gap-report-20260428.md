# Documentation Gap Report

**Generated:** 2026-04-28T04:47:15Z
**Agent:** documentation-generator (Ferrox)
**Project:** terraphim-ai

## Executive Summary

Systematic scan of 40+ crates reveals documentation coverage is strong in core orchestration and agent crates but has significant gaps in supporting crates, haystack providers, and utility modules.

| Metric | Count |
|--------|-------|
| Total crates scanned | ~45 |
| Crates with module docs | ~18 |
| Crates missing module docs | 27 |
| Main.rs files missing module docs | 13 |

## Critical Gaps (Core Infrastructure)

### Missing Module-Level Documentation (lib.rs)

The following crates lack module documentation in their lib.rs:

#### Core Types and Services
- `crates/terraphim_types/src/lib.rs` -- Foundational document/score types
- `crates/terraphim_service/src/lib.rs` -- LLM service abstractions
- `crates/terraphim_settings/src/lib.rs` -- Device and user settings
- `crates/terraphim_config/src/lib.rs` -- Configuration management
- `crates/terraphim_persistence/src/lib.rs` -- Persistence traits

#### Agent Infrastructure
- `crates/terraphim_agent/src/lib.rs` -- Main agent crate (robot mode, CLI)
- `crates/terraphim_mcp_server/src/lib.rs` -- MCP server implementation
- `crates/terraphim_middleware/src/lib.rs` -- Middleware components

#### Haystack Providers (Search Integrations)
- `crates/haystack_core/src/lib.rs` -- Provider trait
- `crates/haystack_atlassian/src/lib.rs` -- Confluence/Jira
- `crates/haystack_discourse/src/lib.rs` -- Discourse forum
- `crates/haystack_grepapp/src/lib.rs` -- Grep.app search
- `crates/haystack_jmap/src/lib.rs` -- JMAP protocol

#### Knowledge Graph
- `crates/terraphim_rolegraph/src/lib.rs` -- Role graph core
- `crates/terraphim_atomic_client/src/lib.rs` -- Atomic Data client

#### Utilities and Tools
- `crates/terraphim_file_search/src/lib.rs` -- File system search
- `crates/terraphim-markdown-parser/src/lib.rs` -- Markdown parsing
- `crates/terraphim_build_args/src/lib.rs` -- Build argument parsing
- `crates/terraphim_ccusage/src/lib.rs` -- Code completion usage
- `crates/terraphim_usage/src/lib.rs` -- Usage tracking
- `crates/terraphim_onepassword_cli/src/lib.rs` -- 1Password CLI
- `crates/terraphim_kg_linter/src/lib.rs` -- KG linting
- `crates/terraphim_lsp/src/lib.rs` -- LSP implementation

#### Bindings and FFI
- `crates/terraphim_automata_py/src/lib.rs` -- Python bindings
- `crates/terraphim_automata/wasm/src/lib.rs` -- WASM bindings
- `crates/terraphim_rolegraph_py/src/lib.rs` -- Python rolegraph
- `crates/terraphim_automata/node/terraphim-automata-node-rs/src/lib.rs` -- Node.js bindings

### Missing Module-Level Documentation (main.rs)

Binary crates without documentation:
- `crates/haystack_atlassian/src/main.rs`
- `crates/haystack_discourse/src/main.rs`
- `crates/haystack_jmap/src/main.rs`
- `crates/terraphim_agent/src/main.rs`
- `crates/terraphim_atomic_client/src/main.rs`
- `crates/terraphim_build_args/src/bin/main.rs`
- `crates/terraphim_config/src/bin/main.rs`
- `crates/terraphim_github_runner_server/src/main.rs`
- `crates/terraphim_kg_linter/src/main.rs`
- `crates/terraphim-markdown-parser/src/main.rs`
- `crates/terraphim-session-analyzer/src/main.rs`
- `crates/terraphim_tinyclaw/src/main.rs`

## Partial Gaps (Some Items Undocumented)

### Public Items Missing Doc Comments

- `terraphim_settings/src/lib.rs:27` -- `pub type DeviceSettingsResult`
- `terraphim_service/src/lib.rs:17` -- `pub use auto_route::...` re-exports
- `terraphim_service/src/llm.rs:32` -- `pub trait LlmClient`
- `terraphim_persistence/src/conversation.rs:10` -- `pub trait ConversationPersistence`
- `terraphim_persistence/src/lib.rs:197` -- `pub trait Persistable`

## Well-Documented Crates (Exemplars)

These crates demonstrate best practices and should be used as templates:

- `terraphim_orchestrator` -- Comprehensive module and item docs
- `terraphim_agent::robot` -- Schema docs with examples
- `terraphim_orchestrator::pr_dispatch` -- Design rationale in docs
- `terraphim_orchestrator::project_control` -- Clear operational docs

## Recommendations

### Priority 1 (Core Infrastructure)
1. Add module docs to `terraphim_types`, `terraphim_service`, `terraphim_config`
2. Document `terraphim_agent` public API surface
3. Document persistence traits in `terraphim_persistence`

### Priority 2 (Search Providers)
4. Add module docs to all `haystack_*` crates
5. Document provider trait in `haystack_core`

### Priority 3 (Knowledge Graph)
6. Document `terraphim_rolegraph` public API
7. Document `terraphim_atomic_client`

### Priority 4 (Utilities and Bindings)
8. Add docs to utility crates (`build_args`, `ccusage`, `usage`)
9. Document Python/WASM/Node bindings entry points

## CHANGELOG Status

CHANGELOG.md is current through v1.17.0 (2026-04-27). Recent additions properly documented:
- ADF Orchestrator Phase 2 PR fan-out
- Agent templates (Phase 2b-2e)
- Robot mode improvements
- Documentation reports

No CHANGELOG updates required at this time.
