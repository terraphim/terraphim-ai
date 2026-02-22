# Implementation Plan: repl-sessions Feature Flag Fix

**Status**: Draft
**Research Doc**: `.docs/research-repl-sessions-feature.md`
**Author**: Claude
**Date**: 2026-01-12
**Estimated Effort**: 15 minutes

## Overview

### Summary
Add `repl-sessions` feature declaration to `terraphim_agent/Cargo.toml` to silence compiler warnings about undeclared cfg condition.

### Approach
Declare `repl-sessions` as a placeholder feature that depends only on `repl`. The actual `terraphim_sessions` dependency remains commented out until published to crates.io.

### Scope
**In Scope:**
- Add `repl-sessions` feature to Cargo.toml
- Update comments explaining placeholder status

**Out of Scope:**
- Publishing `terraphim_sessions` crate
- Enabling session functionality
- Modifying any Rust source code

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_agent/Cargo.toml` | Add `repl-sessions` feature declaration |

## Implementation Steps

### Step 1: Add repl-sessions Feature
**File:** `crates/terraphim_agent/Cargo.toml`
**Description:** Declare placeholder feature to silence warnings

**Current** (lines 24-26):
```toml
repl-web = ["repl"]   # Web operations and configuration
# NOTE: repl-sessions disabled for crates.io publishing (terraphim_sessions not published yet)
# repl-sessions = ["repl", "dep:terraphim_sessions"]  # Session history search
```

**Change to:**
```toml
repl-web = ["repl"]   # Web operations and configuration
# Session search - placeholder feature (terraphim_sessions not published to crates.io yet)
# When terraphim_sessions is published, change to: repl-sessions = ["repl", "dep:terraphim_sessions"]
repl-sessions = ["repl"]
```

### Step 2: Verify Fix
**Command:** `cargo check -p terraphim_agent --features repl-full 2>&1 | grep -c "repl-sessions"`
**Expected:** 0 (no warnings about repl-sessions)

### Step 3: Format and Commit
**Commands:**
```bash
cargo fmt -p terraphim_agent
git add crates/terraphim_agent/Cargo.toml
git commit -m "fix(agent): add repl-sessions placeholder feature to silence warnings"
```

## Test Strategy

### Verification Tests
| Test | Command | Expected |
|------|---------|----------|
| No warnings | `cargo check -p terraphim_agent 2>&1 \| grep "repl-sessions"` | No output |
| Build succeeds | `cargo build -p terraphim_agent` | Exit 0 |
| Feature gating works | `cargo check -p terraphim_agent --features repl-sessions` | Exit 0 |

## Rollback Plan

Remove the `repl-sessions = ["repl"]` line. Warnings will return but build will still succeed.

## Approval Checklist

- [x] Single file change identified
- [x] Change is minimal and safe
- [x] No Rust source code modified
- [x] Verification commands defined
- [ ] Human approval received
