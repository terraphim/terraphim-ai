# Research Document: #923 REPL defaults to non-existent Default role

**Status**: Approved
**Date**: 2026-04-26

## Problem

The REPL's `ReplHandler::new()` hardcodes `current_role: "Default".to_string()` at `handler.rs:49`. This role doesn't exist in the user's config, causing search failures.

## Root Cause

Two issues:
1. `handler.rs:49` -- hardcodes `"Default"` instead of reading from service
2. `terraphim_config/src/lib.rs:857-858` -- `Config::empty()` defaults `selected_role` to `"Default"` which may not match any configured role

## Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| ReplHandler::new() | `repl/handler.rs:45-53` | Hardcodes "Default" |
| ReplHandler::run() | `repl/handler.rs:67` | Shows welcome with wrong role |
| Config::empty() | `terraphim_config/src/lib.rs:852-860` | Defaults selected_role to "Default" |
| get_selected_role() | `service.rs:196-199` | Returns config.selected_role |

## Fix Approach

In `ReplHandler::new()`, read the actual selected role from the service after construction. This ensures the REPL starts with the correct role that exists in config.

The simplest fix: in `new()`, after construction, immediately call `service.get_selected_role().await` and use that value. But `new()` isn't async. Instead, initialize in `run()` before showing the welcome.

Better approach: Make `run()` set `current_role` from the service before displaying welcome.
