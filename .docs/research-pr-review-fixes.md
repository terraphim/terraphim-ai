# Research Document: PR Review P1 Fix Batch

**Status**: Draft
**Author**: CTO Executive (session lead)
**Date**: 2026-04-29
**Source**: Structural PR review of `ci/sentrux-quality-gate` branch

## Executive Summary

The structural PR review identified 4 findings (3 P1, 1 P2) in the quota-to-fallback v2 implementation. This research analyses each finding's root cause, blast radius, and interaction with surrounding code to produce a single coordinated fix plan.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | PR review findings block merge |
| Leverages strengths? | Yes | Exact code already understood from v2 implementation |
| Meets real need? | Yes | Correctness and runtime bugs |

**Proceed**: Yes (3/3)

## Problem Statement

Four issues found in structural review:

1. **P1: Double provider health recording** -- quota detection block + D-3 block both call `record_failure` for the same provider on the same exit
2. **P1: Unnecessary string join on every exit** -- `stderr_lines.join("\n")` + `output_lines.join("\n")` allocated even for non-quota exits
3. **P1: Overly broad "resets at"/"resets in" patterns** -- matches unrelated text containing "resets"
4. **P2: Unbounded retry_counts growth** -- map entries never cleaned on successful retry

## Current State Analysis

### P1-1: Double Recording

**Location**: `lib.rs:5590-5640` (quota block) and `lib.rs:5644-5648` (D-3 block)

Both blocks execute sequentially in the same for loop iteration. When:
- `record.exit_class == ExitClass::RateLimit`
- `def.provider` is `Some("claude-code")`
- `provider_key_for_model(routed_model)` also resolves to `"claude-code"`

Then `record_failure("claude-code")` is called twice. The circuit breaker threshold (default 3 consecutive failures) is halved for quota exits.

**Interaction with error_signatures**: The D-3 block also runs `error_signatures::classify_lines` which may call `record_failure` a *third* time if stderr matches a Throttle pattern. Triple recording is possible.

### P1-2: String Join Allocation

**Location**: `lib.rs:5592-5593`

```rust
let stderr_text = stderr_lines.join("\n");
let output_text = output_lines.join("\n");
```

These lines run on **every agent exit** (552 test iterations, thousands of production exits per day). The join allocates a new String even when `record.exit_class != RateLimit` and neither function is needed.

**API constraint**: `parse_stderr_for_limit_errors(stderr: &str)` takes a single `&str`, not `&[String]`. Same for `is_subscription_limit_error(error: &str) -> bool`. The join is required by their signatures.

### P1-3: Broad Pattern Matching

**Location**: `agent_run_record.rs:265-267` (EXIT_CLASS_PATTERNS)

The patterns `"resets at"` and `"resets in"` in the `ratelimit` PatternDef match any agent output containing those phrases. The Aho-Corasick matcher is case-insensitive and matches substrings.

**Counter-argument**: The exit classifier uses match count. A single "resets at" match contributes 1 vote to RateLimit. If the stderr also has "error[E0433]" (compilation error, 1 vote), RateLimit wins only if there are more rate-limit votes. The risk is real but bounded -- a single spurious match with no other RateLimit patterns and exit_code=1 would classify as RateLimit.

### P2-4: Unbounded retry_counts

**Location**: `lib.rs:252` (field), `lib.rs:5785-5808` (usage)

Entry added on retry, removed only when `*retry_count >= 3`. If agent retries once and succeeds, the entry persists. After weeks of orchestrator runtime with many agent names, the HashMap grows. Each entry is small (String + u32) but the growth is unbounded.

## Constraints

### Vital Few (Max 3)

| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Must not change public API of parse_stderr_for_limit_errors/is_subscription_limit_error | Used by 6+ callers across codebase | grep verification |
| Must not break 566 existing tests | Regression risk | cargo test output |
| Fix must be atomic -- all 4 in one commit | Coordinated review | PR review context |

### Technical Constraints
- `parse_stderr_for_limit_errors` takes `&str`, not `&[String]` -- join is needed for its API
- Aho-Corasick patterns are plain strings, no regex support in PatternDef
- `ProviderHealthMap::record_failure` is idempotent per call (always increments counter)

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Fixing double-record changes D-3 behaviour for non-quota exits | Low | Medium | Only gate D-3 when quota block already recorded |
| Narrowing patterns misses real quota errors | Medium | High | Keep broad patterns in output_parser/telemetry, only narrow in exit classifier |
| Removing join changes semantics of quota detection | Low | Medium | Keep join but gate behind RateLimit check first |

## Research Findings

### Key Insight 1: P1-1 and P1-2 have a shared fix
If we gate the entire quota detection block behind `record.exit_class == ExitClass::RateLimit` FIRST (cheap check), we avoid the join for non-RateLimit exits AND we know the D-3 block will also fire. We can then skip D-3's `record_failure` when the quota block already handled it.

### Key Insight 2: P1-3 is a pattern precision problem
The exit classifier is the wrong place for "resets at"/"resets in" because those phrases are too generic. The `parse_stderr_for_limit_errors` and `is_subscription_limit_error` functions are line-scanners that only fire when called -- they should keep the broad patterns. The exit classifier should only have the precise quota phrases.

### Key Insight 3: P2-4 needs a TTL, not a removal
The cleanest fix for retry_counts is to piggyback on `clean_expired()` in `reconcile_tick()`. Add a timestamp to retry entries and prune entries older than 1 hour.

## Recommendations

Proceed with a coordinated 4-fix implementation in a single commit.
