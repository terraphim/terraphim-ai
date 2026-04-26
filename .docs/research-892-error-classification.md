# Research Document: #892 Error classification mapping in main.rs

**Status**: Approved
**Date**: 2026-04-26

## Executive Summary

The F1.2 exit-code contract is fully implemented on branch `task/860-f1-2-exit-codes` but was never merged to main. The implementation includes `classify_error()`, `--fail-on-empty` flag, exit code help text, and integration tests. Cherry-picking the valuable commits is the correct approach.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Exit code contract is critical for robot mode |
| Leverages strengths? | Yes | Existing ExitCode enum, classify_error on 860 branch |
| Meets real need? | Yes | #892 acceptance criteria are clear and validated |

## Problem Statement

Main.rs always exits with code 0 or 1. The ExitCode enum exists (`robot/exit_codes.rs`) but is unused in the main error path. Orchestrators and CI need structured exit codes to distinguish error types.

## Existing Implementation (on `task/860-f1-2-exit-codes` branch)

### Components

| Component | Location | Status |
|-----------|----------|--------|
| `classify_error()` function | `main.rs:1227-1287` | On 860 branch, not main |
| `classify_error_tests` module | `main.rs:1291-1408` | On 860 branch, not main |
| `--fail-on-empty` flag on Search | `main.rs:721,1748,1896` | On 860 branch, not main |
| Exit code help text (`after_long_help`) | `main.rs:660-676` | On 860 branch, not main |
| `exit_codes.rs` integration tests | `tests/exit_codes.rs` | On 860 branch, not main |
| `no_kg_config.json` test fixture | `tests/no_kg_config.json` | On 860 branch, not main |
| `listen` mode typed exit code | `main.rs:d6e6c5fa9` | On 860 branch, not main |

### Key Commits (value, no agent noise)

| Commit | Description |
|--------|-------------|
| `d6e6c5fa9` | Use typed ExitCode in listen-mode guard + from_code(1) arm |
| `fdbba88b1` | Tighten auth heuristic in classify_error |
| `dfaa60c14` | Add exit code table to --help |
| `709d9d63c` | cargo fmt to test formatting |
| `1177d2fed` | Align exit code assertions with F1.2 contract |
| `57c959c66` | Align integration_tests.rs assertions |
| `eb389bdec` | Tolerate timeout on server roles select |
| `d8d26b187` | Allow exit code 3 in full_feature_matrix test |

### Conflict with #920/#922 fix

My earlier fix for #920/#922 changed "Knowledge graph not configured" from a hard error to returning an empty thesaurus (success path). The 860 branch's `classify_error` maps that same message to exit code 3 (ErrorIndexMissing).

**Resolution**: The 860 branch behaviour is correct for the F1.2 spec -- when a role has no KG, commands SHOULD return exit code 3 (ErrorIndexMissing), not silently succeed with empty results. My #920/#922 fix needs adjustment: graceful degradation should only apply when the user has a role with KG configured but the file is temporarily missing, NOT when the role explicitly has `kg: null`.

## Constraints

- No new dependencies
- Must not break existing tests (228 lib tests)
- Must integrate with existing ExitCode enum
- Must handle #920/#922 conflict carefully

## Recommendations

Cherry-pick the valuable commits from `task/860-f1-2-exit-codes` onto main, resolving the #920/#922 conflict:
1. `classify_error()` + unit tests
2. `--fail-on-empty` flag on Search
3. Exit code help text
4. `exit_codes.rs` integration tests + fixture
5. Typed exit code in listen guard
6. Adjust #920/#922 fix to return exit code 3 for `kg: null` roles instead of empty thesaurus
