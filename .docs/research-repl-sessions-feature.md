# Research Document: repl-sessions Feature Flag Fix

**Status**: Approved
**Author**: Claude
**Date**: 2026-01-12
**Branch**: feat/terraphim-rlm-experimental

## Executive Summary

The `repl-sessions` feature is used throughout `terraphim_agent` crate code but not declared in Cargo.toml, causing compiler warnings. The feature was intentionally commented out because `terraphim_sessions` dependency is not published to crates.io yet.

## Problem Statement

### Description
Compiler warnings appear during builds due to `#[cfg(feature = "repl-sessions")]` annotations referencing an undeclared feature:

```
unexpected `cfg` condition value: `repl-sessions`
expected values for `feature` are: `default`, `repl`, `repl-chat`, `repl-custom`,
`repl-file`, `repl-full`, `repl-interactive`, `repl-mcp`, and `repl-web`
```

### Impact
- CI/CD builds show warnings (potential future -Werror failures)
- Developer confusion about feature availability
- IDE diagnostics cluttered with warnings

### Success Criteria
- No compiler warnings about `repl-sessions` feature
- Feature remains non-functional until `terraphim_sessions` is published
- Clear documentation about feature status

## Current State Analysis

### Existing Implementation

**Cargo.toml** (lines 25-26):
```toml
# NOTE: repl-sessions disabled for crates.io publishing (terraphim_sessions not published yet)
# repl-sessions = ["repl", "dep:terraphim_sessions"]  # Session history search
```

**Code using the feature**:

| File | Lines | Purpose |
|------|-------|---------|
| `commands.rs` | 89-92 | `Sessions` variant in `ReplCommand` enum |
| `commands.rs` | 136-155 | `SessionsSubcommand` enum definition |
| `commands.rs` | 1035, 1261, 1317, 1361 | Session command parsing |
| `handler.rs` | 3 | Import for sessions module |
| `handler.rs` | 317 | Handle sessions commands |
| `handler.rs` | 1661 | Sessions handler implementation |

### Dependencies

**Internal**:
- `terraphim_sessions` (path: `../terraphim_sessions`, NOT published to crates.io)
- `claude-log-analyzer` (published as v1.4.10 on crates.io)

**External**: None specific to this feature.

## Constraints

### Technical Constraints
- Cannot publish `terraphim_agent` with dependency on unpublished `terraphim_sessions`
- Feature flag must be declared to silence warnings
- Code must compile with and without feature enabled

### Business Constraints
- `terraphim_sessions` requires `claude-log-analyzer` which IS published
- Publishing `terraphim_sessions` would unblock full feature

## Solution Analysis

### Option 1: Declare Empty Feature (Recommended)
Add `repl-sessions` feature without the dependency, keeping dependency commented:

```toml
# Session search (dependency not published to crates.io yet)
repl-sessions = ["repl"]  # Placeholder - enable terraphim_sessions when published
# When terraphim_sessions is published, change to:
# repl-sessions = ["repl", "dep:terraphim_sessions"]
```

**Pros**:
- Silences all warnings
- Zero runtime impact (feature-gated code won't compile)
- Documents intended future behavior
- No changes to published crate API

**Cons**:
- Feature exists but doesn't do anything until dependency added

### Option 2: Remove Feature from Code
Remove all `#[cfg(feature = "repl-sessions")]` annotations.

**Pros**:
- No feature complexity

**Cons**:
- Loses all session search code
- Would need to re-add when feature is ready
- Not recommended - code is valuable

### Option 3: Publish terraphim_sessions
Publish the dependency crate to crates.io.

**Pros**:
- Fully enables feature
- Cleanest solution

**Cons**:
- Requires crate review/preparation
- Out of scope for this fix
- terraphim_sessions uses path dependencies itself

## Recommendation

**Proceed with Option 1** - Declare the feature as a placeholder without the dependency. This:
1. Silences compiler warnings immediately
2. Preserves all session search code for future use
3. Documents the feature status clearly
4. Requires minimal changes

## Next Steps

1. Add `repl-sessions = ["repl"]` to Cargo.toml features
2. Update comments to explain placeholder status
3. Run `cargo check` to verify warnings resolved
4. Format and commit
