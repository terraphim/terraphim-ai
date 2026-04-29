# Documentation Gap Report

**Generated:** 2026-04-29T10:47:08Z
**Agent:** documentation-generator (Ferrox)
**Run:** Recurring scan on issue #1046

## Summary

Systematic scan of 24 workspace crates reveals **~1,773 missing documentation items**.

| Metric | Count |
|--------|-------|
| Crates scanned | 24 |
| Total missing docs | ~1,773 |
| Crates with zero missing docs | 3 |
| Most impacted crate | terraphim_validation (443 missing) |

## Missing Documentation by Crate

| Crate | Missing Docs | Severity |
|-------|-------------|----------|
| terraphim_validation | 443 | Critical |
| terraphim_orchestrator | 430 | Critical |
| terraphim_server | 138 | High |
| terraphim_service | 114 | High |
| terraphim_tinyclaw | 104 | High |
| terraphim_agent | 99 | High |
| terraphim_types | 98 | High |
| terraphim_automata | 87 | High |
| terraphim_usage | 82 | Medium |
| terraphim_middleware | 40 | Medium |
| terraphim_kg_orchestration | 40 | Medium |
| terraphim_config | 38 | Medium |
| terraphim_tracker | 35 | Medium |
| terraphim_persistence | 30 | Medium |
| terraphim_ccusage | 29 | Medium |
| terraphim_router | 28 | Medium |
| terraphim_rolegraph | 22 | Medium |
| terraphim_sessions | 13 | Low |
| terraphim_workspace | 11 | Low |
| terraphim_settings | 8 | Low |
| terraphim_mcp_server | 8 | Low |
| terraphim_file_search | 3 | Low |
| terraphim_cli | 0 | Complete |
| terraphim_hooks | 0 | Complete |
| terraphim_markdown_parser | 0 | Complete |

## Detailed Findings

### terraphim_validation (443 missing)
- **Types:** Enum variants (quality gate states, severity levels)
- **Structs:** Validation rule definitions, report structures
- **Functions:** Quality gate evaluators, rule engines
- **Impact:** Zero API docs for quality gate system -- consumers cannot discover validation APIs

### terraphim_orchestrator (430 missing)
- **Modules:** All submodules undocumented
- **Structs:** Agent templates, webhook handlers, task dispatchers
- **Fields:** Configuration fields on orchestrator state
- **Impact:** ADF orchestrator is entirely opaque -- no contributor can understand the agent lifecycle

### terraphim_server (138 missing)
- **Modules:** Route modules, middleware modules
- **Structs:** Server configuration, state containers
- **Impact:** Main server binary lacks module-level docs

### terraphim_service (114 missing)
- **Modules:** Service submodules
- **Types:** Core service enums and structs
- **Impact:** Core business logic undocumented

### terraphim_tinyclaw (104 missing)
- **Structs:** Claw engine state, rule definitions
- **Functions:** Matching engine, scoring functions
- **Impact:** TinyClaw rule engine has no discoverable API

### terraphim_agent (99 missing)
- **Modules:** Agent subsystems
- **Types:** Agent state, event types
- **Impact:** Primary agent crate lacks entry-point documentation

### terraphim_types (98 missing)
- **Modules:** Type submodules
- **Structs:** Shared data structures
- **Fields:** Public struct fields
- **Impact:** Foundation types used across workspace are undocumented

## Recommendations

1. **Immediate (P0):** Add module-level docs (`//!`) to `terraphim_validation` and `terraphim_orchestrator` root lib.rs files
2. **Short-term (P1):** Document all public structs and enums in `terraphim_server`, `terraphim_service`, `terraphim_agent`, `terraphim_types`
3. **Medium-term (P2):** Complete docs for `terraphim_tinyclaw`, `terraphim_automata`, `terraphim_usage`
4. **Process:** Enable `#![warn(missing_docs)]` in all crate lib.rs files to prevent regression
5. **CI:** Add `cargo rustdoc -- -D missing-docs` gate to CI once per-crate thresholds are met

## API Reference Snippets

### terraphim_types
```rust
use terraphim_types::{Article, IndexedArticle, SearchQuery};

// SearchQuery is the primary input type for all search operations
let query = SearchQuery::new("rust async");

// Articles are the core document type
let article = Article::default();
```

### terraphim_service
```rust
use terraphim_service::TerraphimService;

// Main service orchestrator
let service = TerraphimService::new(config).await?;
let results = service.search(query).await?;
```

### terraphim_config
```rust
use terraphim_config::Config;

// Load configuration from default paths
let config = Config::load().await?;
```

---

**Theme-ID:** doc-gap
