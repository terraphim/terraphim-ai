# Implementation Plan: CLI Bugs #920, #921, #922

**Status**: Approved
**Research Doc**: `.docs/research-920-921-922-cli-bugs.md`
**Date**: 2026-04-26
**Estimated Effort**: 2-3 hours

## Overview

Fix three CLI bugs by (1) allowing commands to work without a configured knowledge graph, (2) supporting GitHub token authentication for update checks, and (3) improving error messages.

## Scope

**In Scope:**
- Return empty thesaurus for roles without KG (#920, #922)
- Support GITHUB_TOKEN env var for update checks (#921)
- Suppress noisy startup update errors (#921)
- Improve error messages for all three bugs

**Out of Scope:**
- Refactoring the thesaurus loading pipeline
- Adding retry/backoff to update checks
- Caching update check results
- Auto-building thesaurus from KG on first use (already attempted)

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_service/src/lib.rs` | Return empty thesaurus when role has no KG |
| `crates/terraphim_update/src/lib.rs` | Support GITHUB_TOKEN, improve 403 error message |
| `crates/terraphim_agent/src/main.rs` | Downgrade startup update error to debug logging |

## Implementation Steps

### Step 1: Fix #920/#922 - Graceful empty thesaurus for roles without KG
**Files:** `crates/terraphim_service/src/lib.rs`
**Description:** In `ensure_thesaurus_loaded()`, when `role.kg` is `None`, return an empty thesaurus instead of a hard error. Also when a persisted thesaurus file is not found and there's no automata/local KG path, return empty thesaurus.
**Tests:** Existing tests should still pass (they configure KG). Add test for role without KG.
**Estimated:** 30 min

### Step 2: Fix #921 - GitHub token support and error messages
**Files:** `crates/terraphim_update/src/lib.rs`
**Description:** In `check_for_updates_auto()`, read `GITHUB_TOKEN` env var and pass it to the self_update builder via `.auth_token()`. Improve error messages for 403 to explain rate limiting.
**Tests:** Existing tests should pass. Add test for token env var.
**Estimated:** 30 min

### Step 3: Fix #921 - Suppress noisy startup update check
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** In `main()`, change the startup update check from printing to stderr to logging at debug level. This is a non-critical background check and should not produce visible output.
**Tests:** Existing tests should pass.
**Estimated:** 15 min

## Rollback Plan

Each step is independent and can be reverted individually.

## Dependencies

No new dependencies required.
