# Implementation Plan: #925 listen --server flag error

**Status**: Approved
**Research Doc**: `.docs/research-925-listen-server-flag.md`
**Date**: 2026-04-26

## Step 1: Fix exit code
**File:** `crates/terraphim_agent/src/main.rs`
**Change:** Line 1276: `std::process::exit(1)` -> `std::process::exit(ExitCode::ErrorUsage as i32)` (or just `exit(2)`)
**Test:** Build passes, manual test confirms exit code 2.

## Step 2: Add integration test
**File:** New `crates/terraphim_agent/tests/exit_codes_integration_test.rs`
**Test:** Spawn `terraphim-agent listen --server some-id`, assert stderr contains error message and exit code is 2.
