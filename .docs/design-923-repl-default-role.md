# Implementation Plan: #923 REPL defaults to non-existent Default role

**Status**: Approved
**Research Doc**: `.docs/research-923-repl-default-role.md`
**Date**: 2026-04-26

## Step 1: Fix current_role initialisation in ReplHandler::new()
**File:** `crates/terraphim_agent/src/repl/handler.rs`
**Change:** Remove hardcoded `"Default"`. Instead, call `service.get_selected_role().await` to get the actual role. Since `new()` isn't async, we need to either:
- Make the constructor set it from the service's config_state directly, OR
- Add a line in `run()` before `show_welcome()` that reads the role

Simplest: In `run()`, before `show_welcome()`, set `self.current_role = service.get_selected_role().await.to_string()`.

## Step 2: Verify
- Build passes
- REPL starts with correct role name
