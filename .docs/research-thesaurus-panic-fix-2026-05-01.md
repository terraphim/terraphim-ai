# Research Document: Fix Thesaurus Panic (Gitea #1121)

**Status**: Approved
**Date**: 2026-05-01

## Executive Summary

`build_thesaurus_from_haystack()` at `thesaurus/mod.rs:53` panics when `default_role` does not exist in the `roles` map. This occurs after onboarding wizard creates a role (e.g. "AI Engineer") that differs from the compiled-in `default_role` ("Terraphim Engineer"). The fix requires defensive fallback logic instead of direct map indexing.

## Problem Statement

### Description
Line 51-54 of `crates/terraphim_middleware/src/thesaurus/mod.rs`:
```rust
let role: &mut Role = &mut roles
    .get(&role_name)
    .unwrap_or(&roles[&default_role])  // PANICS
    .to_owned();
```

Two failure modes:
1. `default_role` not in `roles` map (the observed bug) -> index out of bounds panic
2. `role_name` is empty string (when `search_query.role` is `None`) AND `default_role` not in map -> same panic

### Impact
- Every `terraphim-agent search` call panics after onboarding wizard creates a different role than the compiled-in default
- REPL `/search` command panics
- CLI `terraphim-agent search "query"` panics

### Success Criteria
- Search never panics regardless of `default_role` / `roles` mismatch
- Returns a clear error when no valid role can be resolved
- Existing tests continue to pass

## Current State Analysis

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| Panic site | `crates/terraphim_middleware/src/thesaurus/mod.rs:51-54` | Builds thesaurus from haystack |
| Caller | `crates/terraphim_service/src/lib.rs:136` | `TerraphimService::build_thesaurus()` |
| Trigger path | `crates/terraphim_service/src/lib.rs:1965` | `TerraphimGraph` search branch |
| Safe role lookup | `crates/terraphim_service/src/lib.rs:1267-1281` | `get_search_role()` returns `Result` |
| Config persistence | `~/.terraphim/sqlite/terraphim.db` | Stores `embedded_config.json` |

### Call Chain
```
CLI: terraphim-agent search "query"
  -> TerraphimService::search_with_query()  [agent/service.rs:334]
    -> TerraphimService::search()            [service/lib.rs:1428]
      -> get_search_role()                   [line 1431, returns Result - SAFE]
      -> role.relevance_function == TerraphimGraph
        -> self.build_thesaurus()            [line 1965]
          -> build_thesaurus_from_haystack() [middleware/thesaurus/mod.rs:39]
            -> PANIC at line 53              [BUG]
```

### Key Insight
`get_search_role()` (line 1274) already validates that the role exists. But `build_thesaurus_from_haystack()` clones the entire config and does its OWN role lookup independently using `search_query.role` (which can differ from what `get_search_role` resolved). The two lookups are not coordinated.

## Constraints

### Vital Constraints (Max 3)
| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Must not panic | Core reliability requirement | Observed crash in production |
| Must maintain backward compat | Existing callers depend on this function signature | Only one production caller |
| Must return proper error | Caller expects `Result<(), Error>` | Function returns `Result<()>` |

### Eliminated from Scope
| Item | Why Eliminated |
|------|---------------|
| Config validation on load (separate concern) | Out of scope for this fix |
| Persistence path confusion (`~/.terraphim` vs `/tmp`) | Separate issue, documented in #1121 |
| Onboarding wizard role naming | Separate concern |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Existing callers expect panic-free path already | Low | Low | Caller already unwraps Result |
| Empty role_name fallback changes search behaviour | Low | Medium | Match existing `get_search_role` pattern |

## Research Findings

1. Only ONE panic site in production code for this pattern
2. The safe pattern already exists in the same codebase: `get_search_role()` at line 1274 uses `Option` + `Err`
3. `build_thesaurus_from_haystack` takes `&SearchQuery` which has `role: Option<RoleName>` -- it should resolve this using the same logic as `get_search_role`
4. The `update_thesaurus` helper (line 83-103) already handles errors gracefully with `match`
5. No direct tests for `build_thesaurus_from_haystack` -- but the caller chain has integration coverage through search

## Recommendations

Proceed with fix. The change is minimal (3-5 lines), self-contained, and follows an existing pattern in the same crate.
