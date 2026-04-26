# Research Document: #925 listen --server flag error

**Status**: Approved
**Date**: 2026-04-26

## Problem

`terraphim-agent listen --server` exits with code 1 but should exit with code 2 (ERROR_USAGE per the exit code spec). The referenced integration test doesn't exist yet.

## Root Cause

`main.rs:1276` uses `std::process::exit(1)` instead of `std::process::exit(2)` (ExitCode::ErrorUsage).

## Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Listen command handler | `main.rs:1271-1293` | Offline-only, rejects --server |
| Exit code enum | `robot/exit_codes.rs:12-28` | ErrorUsage = 2 |
| Error output | `main.rs:1274-1275` | eprintln error message |

## Fix

Change `exit(1)` to `exit(2)` on line 1276. The error message and eprintln output are correct.
