# Implementation Plan: Fix Cross-Mode Consistency Test Failures

**Status**: Draft
**Research Doc**: `.docs/research-cross-mode-consistency-tests.md`
**Author**: Claude (Disciplined Design)
**Date**: 2026-02-05
**Estimated Effort**: 1-2 hours

## Overview

### Summary
Fix the three failing cross-mode consistency integration tests by replacing the heavyweight production config with a lightweight test config that uses pre-built automata and small fixture data.

### Approach
Create a new test-specific config file that:
1. Uses pre-built automata (`fixtures/term_to_id.json`) instead of building at runtime
2. Uses small fixture haystack (`fixtures/haystack/` with 18 files) instead of production docs
3. Includes the exact roles tested: "Terraphim Engineer", "Default", "Quickwit Logs"
4. Starts in <10 seconds instead of >60 seconds

### Scope
**In Scope:**
- Create new lightweight test config JSON file
- Update test to use new config path
- Reduce health check timeout (faster failure detection)
- Verify all 3 tests pass

**Out of Scope:**
- Refactoring server startup architecture
- Modifying production config files
- Adding new test fixtures or data
- Parallel test execution optimization

**Avoid At All Cost** (from 5/25 analysis):
- Server architecture changes (bind before init)
- Adding more complex test infrastructure
- Creating test-only server binaries
- Mock servers or test doubles

## Architecture

### Component Diagram
```
[Test] --> [New Config: cross_mode_test_config.json]
              |
              +-> roles: Terraphim Engineer, Default, Quickwit Logs
              +-> automata: fixtures/term_to_id.json (PRE-BUILT)
              +-> haystack: fixtures/haystack/ (18 files)
              |
              v
        [Server starts in <10s]
              |
              v
        [Health check succeeds]
              |
              v
        [Tests execute and pass]
```

### Data Flow
```
Test starts
    |
    v
spawn server with --config terraphim_server/fixtures/cross_mode_test_config.json
    |
    v
Server loads pre-built automata (instant)
    |
    v
Server indexes 18 haystack files (fast)
    |
    v
Server binds to port (~5s total)
    |
    v
Health check succeeds
    |
    v
Test runs search operations
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| New JSON config file | Minimal code change (1 line in test) | Programmatic config (more code changes) |
| Use existing fixtures/ directory | Already has data, proven to work | Create new test fixtures (more work) |
| Pre-built automata | Eliminates 50+ seconds of thesaurus building | Build small thesaurus at runtime |
| Keep same role names as tests expect | Zero test logic changes needed | Rename roles in tests |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Programmatic ConfigBuilder in test | More code to maintain | Divergence from JSON config pattern |
| Increase timeout to 120s | Treats symptom, CI still slow | Masks real performance issues |
| Mock server | Tests wouldn't validate real behavior | False confidence in test results |
| Lazy server binding | Major architectural change | Scope creep, regression risk |

### Simplicity Check

> "Minimum code that solves the problem. Nothing speculative."

**What if this could be easy?**

This IS easy: Change one config file path in the test (line 87) and create a lightweight config JSON. The server code doesn't change at all. The test logic doesn't change. Only the config changes.

**Senior Engineer Test**: A senior engineer would ask "why not just use a smaller config?" That's exactly what we're doing.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `terraphim_server/fixtures/cross_mode_test_config.json` | Lightweight test config with pre-built automata |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_agent/tests/cross_mode_consistency_test.rs` | Change config path (1 line), reduce timeout (1 line) |

### Deleted Files

None.

## API Design

No API changes. This is purely a test configuration fix.

## Test Strategy

### Unit Tests
N/A - This is a test infrastructure fix.

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_cross_mode_consistency` | `cross_mode_consistency_test.rs` | Verify search consistency across modes |
| `test_mode_specific_verification` | `cross_mode_consistency_test.rs` | Verify mode-specific behavior |
| `test_role_consistency_across_modes` | `cross_mode_consistency_test.rs` | Verify role behavior across modes |

### Verification

After implementation:
1. Run `cargo test -p terraphim_agent --test cross_mode_consistency_test`
2. All 3 tests should pass
3. Total test time should be <60 seconds (vs timeout before)

## Implementation Steps

### Step 1: Create Lightweight Test Config
**Files:** `terraphim_server/fixtures/cross_mode_test_config.json`
**Description:** Create new config with pre-built automata and small fixtures
**Tests:** Server starts successfully with new config
**Estimated:** 30 minutes

Config structure:
```json
{
  "id": "CrossModeTest",
  "global_shortcut": "Ctrl+Shift+T",
  "roles": {
    "Terraphim Engineer": {
      "shortname": "TerraEng",
      "name": "Terraphim Engineer",
      "relevance_function": "terraphim-graph",
      "kg": {
        "automata_path": "terraphim_server/fixtures/term_to_id.json"
      },
      "haystacks": [{"location": "terraphim_server/fixtures/haystack", "service": "Ripgrep"}]
    },
    "Default": {
      "shortname": "Default",
      "name": "Default",
      "relevance_function": "title-scorer",
      "haystacks": [{"location": "terraphim_server/fixtures/haystack", "service": "Ripgrep"}]
    },
    "Quickwit Logs": {
      "shortname": "QuickwitLogs",
      "name": "Quickwit Logs",
      "relevance_function": "bm25",
      "haystacks": [{"location": "terraphim_server/fixtures/haystack", "service": "Ripgrep"}]
    }
  },
  "default_role": "Terraphim Engineer",
  "selected_role": "Terraphim Engineer"
}
```

### Step 2: Update Test Config Path
**Files:** `crates/terraphim_agent/tests/cross_mode_consistency_test.rs`
**Description:** Change config path from production to test config
**Tests:** Server starts within 15 seconds
**Dependencies:** Step 1
**Estimated:** 10 minutes

Change line 87:
```rust
// Before
"terraphim_server/default/terraphim_engineer_config.json"
// After
"terraphim_server/fixtures/cross_mode_test_config.json"
```

### Step 3: Reduce Health Check Timeout
**Files:** `crates/terraphim_agent/tests/cross_mode_consistency_test.rs`
**Description:** Reduce timeout from 60s to 15s for faster failure detection
**Tests:** Tests complete in reasonable time
**Dependencies:** Step 2
**Estimated:** 5 minutes

Change line 99:
```rust
// Before
for attempt in 1..=60 {
// After
for attempt in 1..=15 {
```

### Step 4: Run Tests and Verify
**Files:** None
**Description:** Execute tests, verify all pass
**Tests:** All 3 tests pass in <60s total
**Dependencies:** Step 3
**Estimated:** 15 minutes

```bash
cargo test -p terraphim_agent --test cross_mode_consistency_test -- --nocapture
```

## Rollback Plan

If issues discovered:
1. Revert config path change in test file
2. Delete new test config file
3. Tests will fail as before (known state)

No feature flags needed - this is test infrastructure only.

## Dependencies

### New Dependencies
None.

### Dependency Updates
None.

## Performance Considerations

### Expected Performance

| Metric | Before | After |
|--------|--------|-------|
| Server startup | >60s (timeout) | <10s |
| Health check timeout | 60s | 15s |
| Total test time | FAIL (180s timeout x3) | <60s (all 3 tests) |

### Benchmarks
No new benchmarks needed. Success metric: tests pass.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify fixtures/term_to_id.json compatible | Pending | Implementation |
| Confirm role names match test expectations | Done | "Terraphim Engineer", "Default", "Quickwit Logs" |

## Approval Checklist

- [x] All file changes listed
- [x] Test strategy complete
- [x] Steps sequenced with dependencies
- [x] Simplicity check passed
- [x] Eliminated options documented
- [ ] Human approval received
