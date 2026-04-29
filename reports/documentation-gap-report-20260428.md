# Documentation Gap Report -- $(date +%Y-%m-%d)

## Summary

| Crate | Missing Docs | Priority |
|-------|--------------|----------|
| terraphim_types | 15+ | High |
| terraphim_config | 14+ | High |
| terraphim_service | 12+ | High |
| terraphim_agent | 10+ | High |
| terraphim_middleware | 10+ | Medium |
| terraphim_persistence | 10+ | Medium |
| terraphim_rolegraph | 15+ | High |
| terraphim_router | 15+ | Medium |
| haystack_core | 1+ | Medium |

## Critical Gaps (Public API Surface)

### terraphim_types
- `MedicalNodeType` -- no module-level docs; central to role graph
- `HgncGene` / `HgncNormalizer` -- missing rustdoc on all pub items
- `MedicalNodeMetadata` -- undocumented struct and methods

### terraphim_config
- `LlmRouterConfig` / `RouterMode` / `RouterStrategy` -- undocumented routing abstractions
- `Role` -- missing docs on builder methods
- `Haystack` -- undocumented constructor and setters
- `KnowledgeGraph` -- missing docs

### terraphim_service
- `ContextConfig` / `ContextManager` -- core context API, largely undocumented
- `OpenRouterService` -- public constructor and methods lack docs
- `OpenRouterError` -- undocumented error variants

### terraphim_agent (robot module)
- `RobotResponse<T>` -- undocumented generic response wrapper
- `ResponseMeta` -- builder methods lack docs
- `RobotConfig` / `RobotFormatter` -- re-exported without module docs

### terraphim_rolegraph
- `SymbolicEmbedding` -- core type, missing docs on fields and methods
- `SymbolicEmbeddingIndex` -- build / query / cache methods undocumented
- `MedicalRoleGraph` -- public API surface largely undocumented

### terraphim_router
- `RouterMetrics` -- all metrics methods undocumented
- `Timer` -- utility struct lacks docs

## CHANGELOG Status

CHANGELOG.md is current as of commit 0541f2c59.

## Recommendations

1. Prioritise terraphim_types and terraphim_rolegraph -- these are foundational crates consumed by downstream modules.
2. Add module-level documentation (`//!`) to every crate `lib.rs` that lacks it.
3. Run `cargo rustdoc -p <crate> --lib -- -D missing-docs` in CI to prevent regressions.
