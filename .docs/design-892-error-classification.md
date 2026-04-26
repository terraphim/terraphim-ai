# Implementation Plan: #892 Error classification mapping in main.rs

**Status**: Approved
**Research Doc**: `.docs/research-892-error-classification.md`
**Date**: 2026-04-26
**Estimated Effort**: 3-4 hours

## Overview

Cherry-pick the F1.2 exit-code implementation from `task/860-f1-2-exit-codes` branch, resolving the conflict with the #920/#922 graceful degradation fix.

## Architecture

```
main() -> run_offline_command() / run_server_command()
              |
              +--> Result<(), anyhow::Error>
                       |
                       +--> Err(e) -> classify_error(&e) -> ExitCode
                                                      -> std::process::exit(code)
```

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_agent/tests/exit_codes.rs` | Integration tests for exit codes |
| `crates/terraphim_agent/tests/no_kg_config.json` | Test fixture for no-KG role |

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/main.rs` | Add classify_error, --fail-on-empty, exit code help, wire into main flow |
| `crates/terraphim_service/src/lib.rs` | Adjust #920/#922 fix to only degrade for missing files, not `kg: null` |

## Implementation Steps

### Step 1: Add classify_error() function + unit tests
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Add the `classify_error` function and its unit test module from the 860 branch.
**Tests:** 15+ unit tests covering all exit code categories.

### Step 2: Add --fail-on-empty flag to Search command
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Add `fail_on_empty: bool` to Search variant, handle in offline and server search paths.

### Step 3: Add exit code table to --help
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Add `after_long_help` to Cli struct with F1.2 exit code documentation.

### Step 4: Wire classify_error into main error path
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** In the catch-all `Some(command)` arm, call classify_error on Err and exit with the mapped code.

### Step 5: Fix #920/#922 conflict -- differentiate kg:null from missing thesaurus file
**Files:** `crates/terraphim_service/src/lib.rs`
**Description:** When `role.kg` is explicitly `None` (user configured no KG), return an error that classify_error maps to exit code 3. When `role.kg` is `Some` but the file is missing, return empty thesaurus (graceful degradation).

### Step 6: Add integration tests and fixture
**Files:** `crates/terraphim_agent/tests/exit_codes.rs`, `no_kg_config.json`
**Description:** Integration tests using assert_cmd to test each exit code.

### Step 7: Verify
**Tests:** `cargo test -p terraphim_agent --lib` + `cargo test -p terraphim_agent --test exit_codes`

## Rollback Plan

Each step is independently revertable.

## Dependencies

No new dependencies. `assert_cmd` is already a dev-dependency in the 860 branch's Cargo.toml -- need to verify it's already on main.
