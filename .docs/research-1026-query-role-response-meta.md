# Research Document: Add query and role fields to robot-mode JSON envelope ResponseMeta

**Status**: Approved
**Author**: opencode (glm-5.1)
**Date**: 2026-04-27
**Issue**: #1026

## Executive Summary

Robot-mode JSON responses lack context about which query and role produced the results. The `ResponseMeta` struct needs `query: Option<String>` and `role: Option<String>` fields so ADF agents and orchestrator can correlate search results with the inputs that generated them.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Robot-mode is the primary interface for ADF agents |
| Leverages strengths? | Yes | Extends existing ResponseMeta builder pattern |
| Meets real need? | Yes | ADF agents cannot correlate results to inputs without these fields |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
When ADF agents receive robot-mode JSON search responses, the envelope contains `meta.command = "search"` but no information about which query string or role produced the results. This makes the response opaque -- agents see results but cannot programmatically identify the context.

### Impact
ADF agents (pr-reviewer, spec-validator, compliance-watchdog, test-guardian) that use robot-mode search cannot correlate responses to their request parameters.

### Success Criteria
- `ResponseMeta` includes optional `query` and `role` fields
- Search commands populate both fields
- Fields are absent from JSON when `None` (non-search commands)
- All existing robot-mode tests continue to pass

## Current State Analysis

### Existing Implementation
`ResponseMeta` in `crates/terraphim_agent/src/robot/schema.rs:47-65` has:
- `command: String`
- `elapsed_ms: u64`
- `timestamp: DateTime<Utc>`
- `version: String`
- `auto_corrected: Option<AutoCorrection>`
- `pagination: Option<Pagination>`
- `token_budget: Option<TokenBudget>`

Builder pattern: `new()`, `with_elapsed()`, `with_auto_correction()`, `with_pagination()`, `with_token_budget()`

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| ResponseMeta struct | `crates/terraphim_agent/src/robot/schema.rs:47` | Metadata for robot-mode JSON envelope |
| ResponseMeta impl | `crates/terraphim_agent/src/robot/schema.rs:67` | Builder methods |
| Search (direct) | `crates/terraphim_agent/src/main.rs:1745` | `Command::Search` handler, has `query` and `role` args |
| Search (server API) | `crates/terraphim_agent/src/main.rs:3700` | Server API search handler, has `query` and `role_name` |
| Meta construction (direct) | `crates/terraphim_agent/src/main.rs:1873` | `ResponseMeta::new("search").with_elapsed(...)` |
| Meta construction (server) | `crates/terraphim_agent/src/main.rs:3825` | `ResponseMeta::new("search").with_elapsed(...)` |
| RobotResponse struct | `crates/terraphim_agent/src/robot/schema.rs:33` | Generic wrapper with `meta` field |
| Robot-mode tests | `crates/terraphim_agent/src/robot/schema.rs:340+` | Existing unit tests for schema |

### Data Flow
```
CLI args (query, role) -> Command::Search
  -> resolve_or_auto_route(role, query) -> (role_name, auto_route)
  -> search_with_query / search_with_role
  -> build SearchResultsData
  -> ResponseMeta::new("search").with_elapsed(...)
  -> RobotResponse::success(data, meta)
  -> JSON output to stdout
```

### Available Variables at Meta Construction Sites

**Site 1 (direct, line 1873):**
- `query`: `String` from CLI arg (available from `Command::Search { query, .. }`)
- `role_name`: `RoleName` from `resolve_or_auto_route()`

**Site 2 (server API, line 3825):**
- `query`: `String` from CLI arg (available from `Command::Search { query, .. }`)
- `role_name`: `RoleName` resolved via API

Both sites have the data readily available -- no additional lookups needed.

## Constraints

### Technical Constraints
- Must use `Option<String>` with `#[serde(skip_serializing_if = "Option::is_none")]` to preserve backward compatibility
- Must follow existing builder pattern (`with_*` methods)
- Fields go between `version` and `auto_corrected` in struct (matches old PR #847 layout)
- `new()` must initialise both to `None` (non-breaking for all other callers)

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| JSON size overhead | 0 bytes when None | N/A |
| Build time impact | Negligible | N/A |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Backward compatible JSON | Existing consumers must not break | `skip_serializing_if = "Option::is_none"` |
| Populate at both search sites | Direct and API paths both produce JSON | Two call sites in main.rs |
| Follow builder pattern | Consistency with existing codebase | 4 existing `with_*` methods |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Adding query/role to error responses | Not needed -- errors already include error messages with context |
| Adding query/role to REPL responses | REPL is human-facing, not robot-mode |
| Changing SearchResultsData struct | Already has `concepts_matched` (tracked separately in PR #969) |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Break existing JSON consumers | Low | High | `skip_serializing_if` means fields absent when None |
| Miss a call site | Low | Low | Only 2 search ResponseMeta constructions exist |

### Assumptions
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Only 2 ResponseMeta::new("search") sites | grep confirmed 2 locations | Missing a site | Yes |
| `query` and `role_name` are always available at both sites | Code reads confirm | Would need fallback | Yes |
| Old PR #847 approach is correct | Matches issue #1026 requirements | Wrong field placement | Yes |

## Recommendations

### Proceed/No-Proceed
**Proceed** -- the change is ~20 lines, well-understood, with prior art from PR #847.

### Scope
Minimal: add two fields, two builder methods, populate at two call sites, add two tests.

## Next Steps
Phase 2: Design implementation plan
