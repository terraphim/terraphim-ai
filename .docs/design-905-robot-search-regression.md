# Design Document: Robot Search Output Contract Regression (#905)

**Status**: Draft
**Date**: 2026-04-26
**Research Doc**: `.docs/research-905-robot-search-regression.md`

## Overview

Two targeted fixes:
1. **Fix A**: Auto-route falls back to first config role instead of synthesising "Default"
2. **Fix B**: Robot-mode errors emit JSON envelope to stdout

## Scope

**In Scope:**
- Fix auto_route degenerate fallback
- Emit JSON error envelope in robot mode when classify_error fires
- Keep all 5 existing regression tests passing

**Out of Scope:**
- Changing the RoleName default from "Default" (too wide impact)
- Rewriting the error handling architecture (#938 tracks this)
- Changing classify_error mappings

## File Changes

### Fix A: `crates/terraphim_service/src/auto_route.rs`

**Change 1**: Degenerate fallback (line 148-155) -- use `config.roles` instead of hardcoded "Default"

Current:
```rust
if scored.is_empty() {
    return AutoRouteResult {
        role: RoleName::from("Default"),
        score: 0,
        candidates: Vec::new(),
        reason: AutoRouteReason::ZeroMatchDefault,
    };
}
```

New:
```rust
if scored.is_empty() {
    let fallback = config
        .roles
        .keys()
        .min_by(|a, b| a.original.cmp(&b.original))
        .cloned()
        .unwrap_or_else(|| RoleName::from("Default"));
    return AutoRouteResult {
        role: fallback,
        score: 0,
        candidates: Vec::new(),
        reason: AutoRouteReason::ZeroMatchDefault,
    };
}
```

When `state.roles` has no rolegraphs but `config.roles` has role definitions, pick
the alphabetically first configured role. Only synthesise "Default" as absolute last
resort when config is also empty.

### Fix B: `crates/terraphim_agent/src/main.rs`

**Change 2**: In the `classify_error` error path (lines ~1493-1513), when robot mode
or machine-readable format is active, emit a JSON error envelope to stdout before
exiting.

The error path currently does:
```rust
if let Err(ref e) = result {
    let code = classify_error(e);
    eprintln!("Error: {:#}", e);
    std::process::exit(code.code().into());
}
```

New: detect robot/machine-readable mode and emit JSON envelope first. The `output`
config is not in scope at this point (it's inside the command handler). Instead, check
`cli.robot` and `cli.format` from the outer scope:

```rust
if let Err(ref e) = result {
    let code = classify_error(e);
    if cli.robot || !matches!(cli.format, OutputFormat::Human) {
        let meta = ResponseMeta::new("unknown");
        let error = RobotError::new(
            format!("E{:03}", code.code()),
            format!("{:#}", e),
        ).with_suggestion(format!("Exit code: {}", code.code()));
        let response = RobotResponse::<()>::error(vec![error], meta);
        if let Ok(json) = serde_json::to_string(&response) {
            println!("{}", json);
        }
    }
    eprintln!("Error: {:#}", e);
    std::process::exit(code.code().into());
}
```

## Test Strategy

### Existing Tests (must continue passing)
- `robot_search_output_regression_tests.rs` -- 5 tests, all pass currently

### New Test
Add a test to `tests/exit_codes.rs`:
- `robot_mode_error_emits_json_envelope` -- invoke with bad config in `--robot`
  mode, verify stdout contains valid JSON with `success: false` and `errors` array

### Manual Verification
```
./target/debug/terraphim-agent --robot search terraphim --limit 2
```
Should now produce JSON error envelope on stdout instead of nothing.

## Implementation Steps

### Step 1: Fix A -- auto_route fallback
**File**: `crates/terraphim_service/src/auto_route.rs`
**Change**: Lines 148-155, use config.roles as fallback source

### Step 2: Fix B -- robot-mode error envelope
**File**: `crates/terraphim_agent/src/main.rs`
**Change**: Lines ~1493-1513, emit JSON before exit in robot mode

### Step 3: Add integration test
**File**: `crates/terraphim_agent/tests/exit_codes.rs`
**Add**: `robot_mode_error_emits_json_envelope` test

### Step 4: Run all tests
```bash
cargo test -p terraphim_agent --test robot_search_output_regression_tests
cargo test -p terraphim_agent --test exit_codes
cargo fmt && cargo clippy
```
