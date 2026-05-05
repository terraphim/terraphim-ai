# Documentation Gap Report

**Generated:** 2026-05-05T00:00:00Z
**Agent:** documentation-generator (Ferrox)
**Command:** `cargo rustdoc -p <crate> --lib -- -D missing-docs`

## Executive Summary

| Metric | Count |
|--------|-------|
| Total missing documentation warnings | **1,058** |
| Crates scanned | 12 |
| Crates with zero warnings | 1 (terraphim_hooks) |
| Worst offender | terraphim_orchestrator (445 warnings) |

## Per-Crate Breakdown

| Crate | Missing Docs | Severity |
|-------|--------------|----------|
| terraphim_orchestrator | 445 | Critical |
| terraphim_server | 138 | High |
| terraphim_service | 114 | High |
| terraphim_agent | 99 | High |
| terraphim_types | 98 | High |
| terraphim_config | 38 | Medium |
| terraphim_middleware | 40 | Medium |
| terraphim_persistence | 30 | Medium |
| terraphim_router | 28 | Medium |
| terraphim_rolegraph | 22 | Medium |
| haystack_core | 4 | Low |
| terraphim_hooks | 0 | Clean |

## Critical Gaps (terraphim_orchestrator)

The orchestrator crate has the highest documentation debt. Key un-documented items:

- Crate-level documentation (`src/lib.rs:1`)
- Core modules: `adf_commands`, `agent_run_record`, `config`, `control_plane`
- Event structs in `control_plane/events.rs`
- Policy and routing modules in `control_plane/`

### Sample Locations

```
crates/terraphim_orchestrator/src/lib.rs:33-64       -- core re-exports
crates/terraphim_orchestrator/src/adf_commands.rs:14  -- command enum variants
crates/terraphim_orchestrator/src/config.rs:14-390    -- configuration structs
crates/terraphim_orchestrator/src/control_plane/events.rs:213-218  -- event fields
crates/terraphim_orchestrator/src/control_plane/routing.rs:17-19   -- routing types
crates/terraphim_orchestrator/src/agent_run_record.rs:560-566      -- record fields
```

## High-Priority Gaps (terraphim_server)

The server crate has 138 missing docs. Key areas:

- Crate-level documentation (`src/lib.rs:1`)
- API structs and handlers (`src/api.rs`)
- Webhook payload structs with many undocumented fields

### Sample Locations

```
terraphim_server/src/lib.rs:1          -- crate docs
terraphim_server/src/lib.rs:139        -- core struct
terraphim_server/src/api.rs:66-236     -- API types
terraphim_server/src/api.rs:1841-1911  -- webhook payload structs
```

## High-Priority Gaps (terraphim_service)

The service crate has 114 missing docs. Key areas:

- Crate-level documentation
- Multiple modules without module docs
- Core enums and structs

## High-Priority Gaps (terraphim_agent)

The agent crate has 99 missing docs. Key areas:

- Robot module (`src/robot/`) -- budget, docs, formatter submodules
- Service struct fields
- Core types in `lib.rs`

## Recommendations

1. **Immediate (this sprint):** Document `terraphim_orchestrator` crate-level and public API surface
2. **Short-term (next sprint):** Address `terraphim_server` API structs and `terraphim_service` modules
3. **Medium-term:** Enforce `#![warn(missing_docs)]` in CI to prevent regression
4. **Long-term:** Consider `#![deny(missing_docs)]` for new crates

## CHANGELOG Update

Updated CHANGELOG.md with:
- New documentation gap report entry for 2026-05-05
- Recent commits: orchestrator event loop refactor, DSM KG refactor, build-runner dedup
- Version bump to 1.17.1
- OpenCode + Terraphim experiment results
