# Documentation Gap Report

**Date:** 2026-05-05
**Agent:** documentation-generator (Ferrox)
**Workspace:** terraphim-ai v1.17.1

## Summary

| Metric | Count |
|--------|-------|
| Crates scanned | 12 |
| Crates with gaps | 11 |
| Clean crates | 1 (terraphim_hooks) |
| Total missing doc items | **1056** |

## Per-Crate Breakdown

| Crate | Missing Docs | Severity |
|-------|--------------|----------|
| terraphim_orchestrator | 445 | Critical |
| terraphim_service | 114 | High |
| terraphim_agent | 99 | High |
| terraphim_types | 98 | High |
| terraphim_server | 138 | High |
| terraphim_middleware | 40 | Medium |
| terraphim_config | 38 | Medium |
| terraphim_persistence | 30 | Medium |
| terraphim_router | 28 | Medium |
| terraphim_rolegraph | 22 | Low |
| haystack_core | 4 | Low |
| terraphim_hooks | 0 | Clean |

## Notable Gaps by Crate

### terraphim_orchestrator (445 gaps)
- Missing docs on flow executor steps, handoff context, and agent templates
- Unclosed HTML tags in rustdoc comments causing warnings
- Template variable documentation incomplete

### terraphim_service (114 gaps)
- Missing docs on error constructors (`system`, `system_with_source`)
- Broken intra-doc link to `kg:term` in search documentation
- Missing module-level documentation for search and graph modules

### terraphim_agent (99 gaps)
- `ReplHandler` struct and methods undocumented
- REPL command variants missing documentation
- Robot mode output formatting undocumented

### terraphim_types (98 gaps)
- Review-related structs (`ReviewResult`, `ReviewFinding`) fields undocumented
- Confidence scoring fields lack documentation

### terraphim_server (138 gaps)
- WebSocket helper functions undocumented
- Workflow notification functions missing docs
- Health check and stats functions lack documentation

## rustdoc Warnings (Non-missing-docs)

| Crate | Warning Type | Count |
|-------|--------------|-------|
| terraphim_middleware | Bare URLs | 5 |
| terraphim_orchestrator | Unclosed HTML tags | 14 |
| terraphim_service | Broken intra-doc link | 1 |

## Recommendations

1. **terraphim_orchestrator**: Prioritise -- this crate has the highest gap count and is critical for ADF operations
2. **terraphim_service** and **terraphim_server**: Second priority -- these are user-facing API crates
3. **terraphim_agent**: Third priority -- CLI is primary user interface
4. Enable `#![warn(missing_docs)]` at crate root to prevent regression
5. Fix broken intra-doc links and bare URLs to improve docs.rs rendering

## CHANGELOG Updated

Recent commits (since 2026-04-29) documented in [Unreleased] section:
- Streaming output log drain for build-runner
- GITEA_URL injection into agent spawn context
- ADF CI pipeline degradation fixes
- Integration test stabilisation
- Service-dependent tests marked `#[ignore]`

---

Theme-ID: doc-gap
