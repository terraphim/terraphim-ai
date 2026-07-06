# Implementation Plan: Session Analyzer JSONL Parser Fix (#3081)

**Status**: Draft
**Research Doc**: [research.md](./research.md)
**Author**: opencode session
**Date**: 2026-07-06
**Estimated Effort**: 2-3 hours

## Overview

### Summary

Add a pre-filter to `terraphim_session_analyzer::parser::SessionParser::from_file()` that skips known Claude Code metadata JSONL entry types before attempting full deserialization. This eliminates ~2003 of ~2430 WARN lines without changing any downstream code.

### Approach

Pre-filter by entry type using a lightweight JSON peek. Only entry types that carry conversational message content (`user`, `assistant`) are deserialized into `SessionEntry`. Known metadata types are skipped silently at debug level.

### Scope

**In Scope:**
- Pre-filter in `parser.rs::from_file()`
- Skip set constant for known metadata types
- Test fixtures for each metadata type
- Test verifying zero WARN for metadata-only files

**Out of Scope:**
- Investigating the ~427 `assistant` parse failures (separate issue)
- Changing `SessionEntry` struct fields
- Modifying downstream consumers
- The `terraphim_sessions` crate's own parser (different code)

**Avoid At All Cost:**
- Making `message` or `uuid` optional on `SessionEntry` (ripples through entire crate)
- Adding new structs for metadata entry types (they carry no searchable content)
- Adding abstraction layers or trait-based filtering (premature complexity)

## Architecture

### Data Flow (After Fix)

```
Claude Code JSONL file
    |
    v
parser.rs:from_file() -- reads line by line
    |
    v
serde_json::from_str::<EntryTypePeek>(&line)   <-- NEW: lightweight type extraction
    |
    +-- type in SKIP_SET? ---> debug!("Skipping metadata entry type: {ty}")
    |                               continue
    |
    +-- type not in SKIP_SET?
            |
            v
    serde_json::from_str::<SessionEntry>(&line)
        |                    |
        | Ok(entry)          | Err(e)
        v                    v
    entries.push(entry)   warn!("Failed to parse...")  <-- still warns for genuine errors
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Pre-filter by `"type"` field before full deserialize | Single-location change, zero downstream impact | Making fields Option<> (ripples through codebase) |
| Use `EntryTypePeek` struct for lightweight parse | Cheaper than full `serde_json::Value` parse, type-safe | Regex matching on raw string (fragile) |
| Skip at debug level, not silent | Traceability for debugging; can be turned on with RUST_LOG | Silent skip (loses all visibility) |
| Constant `HashSet<&str>` for skip types | Compile-time known, O(1) lookup | Match statement (verbose for 12 types) |

### Simplicity Check

**What if this could be easy?**

The simplest possible fix: a `const SKIP_TYPES: &[&str]` array and a `.contains()` check after a one-field JSON peek. That's 15 lines of code in one function. No struct changes. No new modules. No new dependencies.

**Senior Engineer Test**: This is a filter clause, not an architecture change. Passes.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `src/parser.rs` | Add `SKIP_ENTRY_TYPES` constant; add `EntryTypePeek` struct; add pre-filter in `from_file()` loop |

### New Files

| File | Purpose |
|------|---------|
| `tests/fixtures/metadata-entries.jsonl` | Test fixture: one line per metadata entry type |
| `tests/skip_metadata.rs` | Integration test: zero WARN for metadata-only file |

No new modules. No new dependencies. No struct changes.

## API Design

### New Types (internal, not exported)

```rust
/// Lightweight struct for peeking at the entry type before full deserialization.
/// Only extracts the "type" field -- costs negligible overhead vs full SessionEntry parse.
#[derive(Deserialize)]
struct EntryTypePeek {
    #[serde(rename = "type")]
    entry_type: String,
}

/// Claude Code JSONL entry types that carry metadata, not conversational messages.
/// These are skipped during parsing to avoid false deserialization failures.
const SKIP_ENTRY_TYPES: &[&str] = &[
    "last-prompt",
    "mode",
    "permission-mode",
    "ai-title",
    "file-history-snapshot",
    "queue-operation",
    "agent-name",
    "pr-link",
    "tool_reference",
    "text",
    "attachment",
    "system",
];
```

No changes to public API. No new public types. No new error variants.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_skip_metadata_entry_types` | `parser.rs` mod tests | Verify each metadata type is skipped without WARN |
| `test_user_entry_still_parsed` | `parser.rs` mod tests | Verify `user` entries are still parsed |
| `test_assistant_entry_still_parsed` | `parser.rs` mod tests | Verify `assistant` entries are still parsed |
| `test_unknown_type_still_warns` | `parser.rs` mod tests | Verify genuinely unknown types still produce WARN |
| `test_entry_type_peek_extracts_type` | `parser.rs` mod tests | Verify the peek struct works correctly |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_metadata_only_file_no_warnings` | `tests/skip_metadata.rs` | Full file of metadata entries produces zero entries and zero warnings |

### Test Fixtures

`tests/fixtures/metadata-entries.jsonl` -- one real JSONL line per metadata type, extracted from actual Claude Code session files:

```jsonl
{"type":"last-prompt","lastPrompt":"echo hello","leafUuid":"...","sessionId":"..."}
{"type":"mode","mode":"normal","sessionId":"..."}
{"type":"permission-mode","permissionMode":"auto","sessionId":"..."}
{"type":"ai-title","title":"...","sessionId":"..."}
{"type":"file-history-snapshot","...}
{"type":"queue-operation","...}
{"type":"agent-name","...}
{"type":"pr-link","...}
{"type":"tool_reference","...}
{"type":"text","...}
{"type":"attachment","...,"uuid":"..."}
{"type":"system","...}
```

## Implementation Steps

### Step 1: Add skip set and peek struct

**Files:** `src/parser.rs` (top of file, after imports)
**Description:** Define `SKIP_ENTRY_TYPES` constant and `EntryTypePeek` struct
**Tests:** `test_entry_type_peek_extracts_type`
**Estimated:** 15 minutes

### Step 2: Add pre-filter in from_file()

**Files:** `src/parser.rs`, function `from_file()` at line 41-42
**Description:** Before `serde_json::from_str::<SessionEntry>`, peek at the type field. If in skip set, log at debug and continue.
**Tests:** `test_skip_metadata_entry_types`, `test_user_entry_still_parsed`, `test_assistant_entry_still_parsed`, `test_unknown_type_still_warns`
**Dependencies:** Step 1
**Estimated:** 30 minutes

**Key code:**

```rust
// Inside the line-processing loop, before the SessionEntry deserialize:
match serde_json::from_str::<EntryTypePeek>(&line) {
    Ok(peek) if SKIP_ENTRY_TYPES.contains(&peek.entry_type.as_str()) => {
        debug!("Skipping metadata entry of type: {}", peek.entry_type);
        continue;
    }
    Ok(_) => { /* fall through to full deserialize */ }
    Err(_) => { /* fall through -- will produce WARN as before */ }
}

// Existing deserialize logic unchanged:
match serde_json::from_str::<SessionEntry>(&line) {
    Ok(entry) => { ... }
    Err(e) => { warn!(...); }
}
```

### Step 3: Create test fixtures

**Files:** `tests/fixtures/metadata-entries.jsonl`
**Description:** Extract one real JSONL line per metadata type from actual Claude Code session files
**Tests:** Used by Step 4 integration test
**Dependencies:** None
**Estimated:** 20 minutes

### Step 4: Integration test

**Files:** `tests/skip_metadata.rs`
**Description:** Parse the metadata-only fixture file and assert zero entries, zero warnings
**Tests:** `test_metadata_only_file_no_warnings`
**Dependencies:** Steps 1-3
**Estimated:** 20 minutes

### Step 5: Verify with live data

**Description:** Run `terraphim-agent sessions search "test"` and confirm WARN count drops from ~2430 to ~427 (only the `assistant` failures remain)
**Dependencies:** Steps 1-4, cargo build + install
**Estimated:** 15 minutes

## Rollback Plan

If issues discovered:
1. Remove the `EntryTypePeek` struct and skip check -- the `from_file()` function reverts to direct deserialize
2. No data migration needed -- the parser is stateless

No feature flag needed -- the change is a strict improvement (fewer false warnings, same behaviour for valid entries).

## Dependencies

### New Dependencies

None. Uses only `serde` and `serde_json` which are already dependencies.

### Version Bump

Publish as `terraphim-session-analyzer` `1.20.6` (patch bump from 1.20.3/1.20.5).

Update consumer in `terraphim-clients/crates/terraphim_sessions/Cargo.toml`:
```toml
terraphim-session-analyzer = { version = "1.20.6", optional = true }
```

## Crate Source Location

The crate was extracted from `terraphim-ai` in commit `aa7ba99e8`. To apply the fix:

1. Restore from git history: `git show aa7ba99e8^:crates/terraphim-session-analyzer/`
2. Or work in the publishing repo (if separate)
3. Apply fix, bump version, publish to registry
4. Update consumer version in terraphim-clients

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm crate publishing repo (terraphim-ai history vs separate repo) | Pending | Investigation needed |
| Investigate 427 `assistant` failures (may be separate issue) | Deferred | Future issue if pre-filter doesn't resolve them |
