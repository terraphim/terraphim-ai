# Research Document: Robot Search Output Contract Regression (#905)

**Status**: Approved
**Date**: 2026-04-26
**Issue**: #905

## Executive Summary

Robot-mode search (`--robot search`) fails when rolegraphs are not pre-populated in
`state.roles`. The auto-route function returns a synthesised "Default" role with zero
candidates, which doesn't exist in config, causing the search to error out instead of
producing a JSON error envelope.

## Problem Statement

### Description
When running `terraphim-agent --robot search <query>` without pre-warmed rolegraphs,
the command fails with exit code 1 and plain-text error on stderr:
```
Error: Config error: Role `Default` not found in config
```
No JSON output is produced, violating the RobotResponse contract.

### Impact
- Robot mode (used by AI agents) silently breaks when rolegraphs aren't loaded
- No machine-readable error output -- agents cannot parse the failure
- The 5 regression tests in `robot_search_output_regression_tests.rs` pass only
  because they use the default config which has a "Terraphim Engineer" role that
  gets loaded during service init

### Success Criteria
1. `--robot search` always produces valid JSON (success envelope or error envelope)
2. Auto-route never returns a non-existent role
3. When no roles have rolegraphs, auto-route falls back to the config's first role
4. All 5 existing regression tests continue to pass

## Current State Analysis

### Root Cause Chain

1. `service.rs:resolve_or_auto_route()` (line 269) calls `auto_select_role()`
   WITHOUT ensuring rolegraphs are loaded
2. `auto_route.rs:auto_select_role()` iterates `state.roles` (the rolegraph map)
3. If `state.roles` is empty (no rolegraphs built), `scored` is empty
4. Degenerate path (line 149-155) returns `RoleName::from("Default")` with 0 candidates
5. "Default" doesn't exist in the user's config -> search fails
6. Error goes through `classify_error()` -> eprintln + process::exit(1)
7. No JSON envelope is emitted

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `resolve_or_auto_route` | `service.rs:269` | Orchestrates role resolution |
| `auto_select_role` | `auto_route.rs:113` | Scores and picks role |
| Degenerate fallback | `auto_route.rs:149-155` | Returns "Default" when 0 candidates |
| Zero-match fallback | `auto_route.rs:172-184` | Returns "Default" or first scored |
| `classify_error` in main | `main.rs:1493-1513` | Exits with code, plain text to stderr |
| Config default selected_role | `config/lib.rs:858` | Hardcoded `RoleName::new("Default")` |

### Data Flow (broken path)
```
search command -> resolve_or_auto_route(None, query)
  -> state.roles is empty (no rolegraphs)
  -> auto_select_role returns { role: "Default", candidates: [] }
  -> search tries "Default" -> "Role `Default` not found in config"
  -> classify_error -> eprintln("Error: ...") -> exit(1)
  -> stdout is empty, no JSON envelope
```

## Constraints

- No new external dependencies
- Must not break the existing 5 regression tests
- Must not regress #892 exit code contract
- Robot mode errors should be machine-readable (JSON envelope)
- Must handle both: empty state.roles AND single-role configs

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Changing auto_route fallback breaks multi-role configs | Low | High | Test with multi-role config |
| JSON error envelope conflicts with classify_error path | Medium | High | Emit JSON before process::exit |
| Rolegraph population timing varies by config | Medium | Medium | Use config.roles as fallback source |

### Assumptions

| Assumption | Basis | Risk if Wrong |
|------------|-------|---------------|
| `config.roles` is always populated even when `state.roles` is empty | Config loaded at init | If config also empty, must handle gracefully |
| The degenerate "Default" fallback was meant as a placeholder | Comment says "synthesised" | Might be relied upon somewhere |

## Recommendations

Two fixes needed:

**Fix A (auto_route fallback)**: When `scored` is empty (no rolegraphs), fall back to
the first role in `config.roles` rather than synthesising "Default". This fixes the
root cause.

**Fix B (robot-mode error envelope)**: When `classify_error` fires and robot mode is
active, emit a JSON error envelope to stdout before exiting. This ensures the contract
is upheld even for errors we haven't anticipated.
