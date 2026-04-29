# Implementation Plan: PR Review P1/P2 Fix Batch

**Status**: Draft
**Research Doc**: `.docs/research-pr-review-fixes.md`
**Author**: CTO Executive (session lead)
**Date**: 2026-04-29
**Estimated Effort**: 1 hour

## Overview

### Summary

Fix 3 P1 and 1 P2 finding from structural PR review in a single coordinated commit: eliminate double provider health recording, defer string join allocation, narrow exit classifier patterns, and bound retry_counts growth.

### Approach

Gate the quota detection block behind the RateLimit exit class check to eliminate unnecessary joins and create a clear sequencing contract with the D-3 block. Narrow the "resets at"/"resets in" patterns in the exit classifier to require a following digit or time indicator. Add TTL-based cleanup for retry_counts.

### Scope

**In Scope:**
- Gate quota detection behind `ExitClass::RateLimit`
- Skip D-3 `record_failure` when quota block already recorded
- Narrow "resets at"/"resets in" to "resets at [0-9]" / "resets in [0-9]"
- Add retry TTL cleanup in `reconcile_tick`

**Out of Scope:**
- Refactoring parse_stderr_for_limit_errors to take `&[String]` (API change, separate PR)
- D-3 block fix for non-quota exits when `def.provider` is None (separate issue)
- Error signatures triple-recording fix (separate issue)

**Avoid At All Cost:**
- Changing public API of output_parser or telemetry functions
- Adding new crate dependencies
- Creating new files

## Architecture

### Fix 1: Gate quota block + skip D-3

```
BEFORE:
  quota detection block (always runs, joins strings)
  D-3 block (always runs, may double-record)

AFTER:
  if record.exit_class == RateLimit:
      quota detection block (join needed only for secondary check)
      flag: quota_recorded = true
  D-3 block:
      if quota_recorded && same provider: skip record_failure
      else: run as before
```

### Fix 2: Narrow patterns

```
BEFORE (agent_run_record.rs):
  "resets at"     -- matches "system resets at startup"
  "resets in"     -- matches "cache resets in background"

AFTER (agent_run_record.rs):
  "resets at " + digit requirement handled by parse_reset_time at detection layer
  "resets in " + digit requirement handled by parse_reset_time at detection layer
  
  REMOVE "resets at" and "resets in" from EXIT_CLASS_PATTERNS entirely.
  These patterns remain in output_parser.rs and telemetry.rs where they are
  only called after a RateLimit suspicion already exists.
```

### Fix 3: TTL cleanup for retry_counts

```
reconcile_tick():
    provider_rate_limits.clean_expired()
    retry_counts.retain(|_, ts| ts.elapsed() < Duration::from_secs(3600))
    
Change retry_counts from HashMap<String, u32> to HashMap<String, (u32, Instant)>
```

## File Changes

| File | Changes |
|------|---------|
| `lib.rs` | Gate quota block, add `quota_recorded` flag, skip D-3 on overlap, change retry_counts to `(u32, Instant)`, add TTL cleanup |
| `agent_run_record.rs` | Remove "resets at" and "resets in" from ratelimit patterns |

## Implementation Steps

### Step 1: Remove broad patterns from exit classifier

**File**: `agent_run_record.rs`
**Description**: Remove `"resets at"` and `"resets in"` from the ratelimit PatternDef patterns array. These patterns remain in `output_parser.rs` and `telemetry.rs` (where they are only called after suspicion exists). Update any tests that relied on these patterns in the exit classifier.
**Tests**: `cargo test -p terraphim_orchestrator --lib -- classify_quota`
**Estimated**: 5 min

### Step 2: Gate quota detection + fix D-3 overlap

**File**: `lib.rs`
**Description**:
1. Move the `is_quota_exit` check to only run when `record.exit_class == ExitClass::RateLimit`. This eliminates the join for 99% of exits (only RateLimit exits need it).
2. Introduce a `let mut quota_provider_recorded: Option<&str> = None;` variable before the quota block.
3. When the quota block records a provider, set `quota_provider_recorded = Some(provider_key)`.
4. In the D-3 block, skip `record_failure` when `quota_provider_recorded` matches `def.provider`.
5. Keep the D-3 `record_success` path untouched (success should always record).
6. Keep the error_signatures classify_lines block untouched (it runs on different signals).

**Code sketch** (exact changes):
```rust
// Replace the current quota detection block with:
let mut quota_provider_recorded: Option<String> = None;
if record.exit_class == ExitClass::RateLimit {
    let stderr_text = stderr_lines.join("\n");
    let is_quota_exit = control_plane::output_parser::parse_stderr_for_limit_errors(&stderr_text).is_some()
        || control_plane::telemetry::is_subscription_limit_error(&stderr_text);
    // (exit_class already confirmed RateLimit, secondary check is belt-and-suspenders)
    let _ = is_quota_exit; // acknowledged but the block always runs for RateLimit
    {
        let effective_provider = def.provider.as_deref()
            .or_else(|| routed_model.as_deref().and_then(|m| provider_budget::provider_key_for_model(m)));
        if let Some(provider_key) = effective_provider {
            warn!(...);
            self.provider_health.record_failure(provider_key);
            if let Some(tracker) = self.provider_budget_tracker.as_ref() {
                tracker.force_exhaust(provider_key);
            }
            let quota_line = stderr_lines.iter().chain(stdout_lines.iter())
                .find(|l| l.to_lowercase().contains("resets "))
                .map(|s| s.as_str())
                .unwrap_or("");
            if let Some(reset_time) = parse_reset_time(quota_line) {
                self.provider_rate_limits.block_until(provider_key, reset_time);
            }
            quota_provider_recorded = Some(provider_key.to_string());
        }
    }
}
```

Then in D-3 block, wrap the `record_failure` calls:
```rust
// D-3: Feed exit classification into provider health circuit breaker
if let Some(ref provider) = def.provider {
    let already_recorded_by_quota = quota_provider_recorded.as_deref() == Some(provider);
    match record.exit_class {
        ExitClass::ModelError => {
            if !already_recorded_by_quota {
                self.provider_health.record_failure(provider);
            }
        }
        ExitClass::RateLimit => {
            if !already_recorded_by_quota {
                self.provider_health.record_failure(provider);
            }
        }
        ExitClass::Success | ExitClass::EmptySuccess => {
            self.provider_health.record_success(provider);
        }
        _ => {}
    }
    // error_signatures block unchanged
```
**Tests**: `cargo test -p terraphim_orchestrator --lib`
**Dependencies**: Step 1
**Estimated**: 20 min

### Step 3: TTL cleanup for retry_counts

**File**: `lib.rs`
**Description**:
1. Change `retry_counts: HashMap<String, u32>` to `retry_counts: HashMap<String, (u32, Instant)>`.
2. Where retry count is read/written in the KG fallback respawn block:
   - `let retry_count = self.retry_counts.entry(name.clone()).or_insert((0, Instant::now()));`
   - Access as `retry_count.0` and `retry_count.1`
   - On increment: `retry_count.0 += 1; retry_count.1 = Instant::now();`
3. Add TTL cleanup in `reconcile_tick`:
   - `self.retry_counts.retain(|_, (count, ts)| *count < 3 || ts.elapsed() < Duration::from_secs(3600));`
   Wait -- actually the simpler approach: always retain entries less than 1 hour old, regardless of count. This way the retry count resets after 1 hour of no retries:
   - `self.retry_counts.retain(|_, (_, ts)| ts.elapsed() < Duration::from_secs(3600));`
4. Update the `retry_counts.remove` call in the max-retries branch (still correct -- remove when exhausted).
**Tests**: `cargo test -p terraphim_orchestrator --lib -- rate_limit_window`
**Dependencies**: Step 2
**Estimated**: 15 min

### Step 4: Update tests and verify

**Files**: `agent_run_record.rs`, `lib.rs`
**Description**:
1. Fix `classify_quota_resets_at` test -- if "resets at" is removed from patterns, this test should either use a more specific string or test at the output_parser level instead.
2. Run full suite: `cargo test -p terraphim_orchestrator --lib`
3. Run clippy: `cargo clippy -p terraphim_orchestrator -- -D warnings`
**Tests**: 566+ tests must pass
**Dependencies**: Step 3
**Estimated**: 10 min

## Rollback Plan

Each step is independently revertible. If Step 2 breaks D-3 behaviour, revert to the double-recording (which is the current state). The double-recording is a conservative error (records too much, not too little).

## Performance Considerations

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| String allocation per exit | 2 joins (stderr + output) | 0 for non-RateLimit, 1 for RateLimit | ~99% reduction |
| record_failure calls per quota exit | 2-3 | 1 | 50-66% reduction |
| retry_counts map size | Unbounded growth | Bounded by 1h TTL | Bounded |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
