# Design & Implementation Plan: Fix UTF-8 Byte Boundary Panic in KG Link Logging

## 1. Summary of Target Behaviour

The code at `crates/terraphim_service/src/lib.rs:953-958` logs a snippet of content around a Knowledge Graph link marker (`](kg:`). The current implementation uses byte arithmetic (`saturating_sub(50)`, `+100`) which can land on a UTF-8 character boundary, causing `&processed_content[start..end]` to panic at runtime.

**Desired behaviour**: The logging statement should safely extract a UTF-8 string snippet centred on the `](kg:` marker, with approximately 50 characters before and 100 characters after, without panicking regardless of the character encoding of the content.

## 2. Key Invariants and Acceptance Criteria

### Invariants
- The extracted snippet must always be a valid UTF-8 string
- The snippet boundaries must fall on valid `char` boundaries
- If the marker is within 50 bytes of the string start, the snippet starts at byte 0
- If the marker is within 100 bytes of the string end, the snippet ends at the string's end
- The snippet should contain the marker and surrounding context when possible

### Acceptance Criteria
| ID | Criterion | Test Type |
|----|-----------|-----------|
| AC1 | Snippet extraction does not panic with ASCII-only content | Unit |
| AC2 | Snippet extraction does not panic with multi-byte UTF-8 content (e.g., CJK, emoji, typographic quotes) | Unit |
| AC3 | When marker is near start (<50 chars), snippet starts at position 0 | Unit |
| AC4 | When marker is near end (<100 chars), snippet ends at string length | Unit |
| AC5 | Snippet contains the marker with context on both sides | Unit |
| AC6 | The original panic site (`lib.rs:953-958`) is replaced with the safe implementation | Integration |

## 3. High-Level Design and Boundaries

### Option Analysis

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A: Local helper `snippet_around()`** | Create a dedicated function in `terraphim_service` that extracts a snippet around a marker using `char_indices()` | Self-contained; clear intent; easily testable; no cross-crate dependencies | Adds a new utility to maintain |
| **B: Use `floor_char_boundary` polyfill** | Import the existing polyfill from `terraphim_server` into `terraphim_service` | DRY; leverages existing tested code | Cross-crate import within same workspace; the polyfill is low-level (single boundary) and doesn't directly solve "snippet around marker" |
| **C: Inline `char_indices` fix** | Directly fix the call site with proper char boundary handling | No new functions; straightforward | Verbose; not reusable; pollutes the call site |

### Recommended Option: **A (Local helper)**

The `snippet_around` function is the best fit because:
1. The existing utilities (`truncate_snippet`, `floor_char_boundary`) solve different problems (truncation from start, single boundary finding)
2. Extracting a centred snippet with asymmetric before/after padding is a distinct operation
3. A testable helper keeps the call site clean and documents intent
4. The function is small and unlikely to need further maintenance

### Scope Decision: Additional Bugs

| Location | Pattern | Include in this PR? |
|----------|---------|---------------------|
| `lib.rs:1383` (`text[..start]`) | Word boundary check; `start` comes from `is_word_boundary` callers | **No** - separate issue; requires understanding boundary determination logic |
| `lib.rs:1393` (`text[end..]`) | Same as above | **No** - same reason |
| `summarization_manager.rs:258,261,268,273` | Direct slicing at `max_length` | **No** - separate issue; different context (description extraction) |

**Rationale**: The primary bug (lib.rs:953-958) is a clear, isolated panic. The additional bugs require deeper analysis of their contexts (word boundary semantics, description extraction requirements) and should be addressed in separate issues to keep this PR focused.

## 4. File/Module Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/terraphim_service/src/lib.rs` | Modify | Unsafe byte arithmetic at lines 953-959 | Call to new `snippet_around()` helper | None (local helper) |
| `crates/terraphim_service/src/lib.rs` | Add | - | New private helper function `snippet_around(s: &str, marker: &str, before: usize, after: usize) -> String` | None |
| `crates/terraphim_service/src/lib.rs` | Add | - | Unit tests for `snippet_around` covering ASCII, multi-byte UTF-8, edge cases | None |

### New Helper Function Signature

```rust
/// Extract a UTF-8 safe snippet around the first occurrence of `marker`.
/// Returns the substring from `max(0, marker_pos - before)` to
/// `min(len, marker_pos + marker.len() + after)`, truncated to char boundaries.
///
/// If `marker` is not found, returns an empty string.
fn snippet_around(s: &str, marker: &str, before: usize, after: usize) -> String {
    // Implementation using char_indices() to find safe boundaries
}
```

## 5. Step-by-Step Implementation Sequence

1. **[Step 1: Add helper function with tests]** - Create `snippet_around()` in `crates/terraphim_service/src/lib.rs` with comprehensive unit tests (ASCII, CJK, emoji, edge cases). System remains deployable.

2. **[Step 2: Replace panic site with helper]** - Replace lines 953-959 with call to `snippet_around(processed_content, "](kg:", 50, 100)`. System remains deployable.

3. **[Step 3: Verify build and tests]** - Run `cargo build -p terraphim_service` and `cargo test -p terraphim_service`. All tests pass.

4. **[Step 4: Run ubs scanner]** - Run `ubs` on modified files to catch any additional issues.

### Feature Flags / Migrations
- No feature flags required
- No database migrations required
- No breaking changes to API/interface

## 6. Testing & Verification Strategy

### Unit Test Cases for `snippet_around`

| Test Case | Input | Expected Output |
|-----------|-------|-----------------|
| ASCII simple | `"Hello World foo"](kg:bar"` with `before=10, after=10` | `"Hello World foo"](kg:bar"` |
| ASCII truncation left | `"xyz Hello World foo"](kg:bar"` with `before=10, after=10` | `"Hello World foo"](kg:bar"` |
| ASCII truncation right | `"Hello World foo"](kg:bar xyz"` with `before=10, after=10` | `"Hello World foo"](kg:bar xyz"` |
| Multi-byte UTF-8 (CJK) | `"日本語 Hello"](kg:bar 日本語"` with `before=5, after=5` | Safe slice on char boundary, no panic |
| Multi-byte UTF-8 (emoji) | `"Hello 😂 World"](kg:bar"` with `before=10, after=10` | Safe slice containing emoji |
| Marker not found | `"Hello World"` with `before=10, after=10` | `""` |
| Empty string | `""` with `before=10, after=10` | `""` |
| Marker at start | `"](kg:bar Hello"` with `before=10, after=10` | `"](kg:bar Hello"` |
| Marker at end | `"Hello ](kg:bar"` with `before=10, after=10` | `"Hello ](kg:bar"` |

### Test Location
- Add tests as a `#[cfg(test)]` module in `crates/terraphim_service/src/lib.rs` near the helper function
- Follow existing test module patterns in the crate

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Byte vs char confusion in helper implementation | Use `char_indices()` iterator; test with multi-byte content | Low - pattern is well-understood |
| Helper used incorrectly at call site | The helper handles all edge cases internally | Low |
| Edge case: marker position lands in middle of multi-byte char | Use `chars().next()` on remainder after marker position | Low |

### Complexity Assessment
- **Cognitive complexity**: Low - the helper is ~15 lines of straightforward iteration
- **Architectural impact**: Minimal - local change within existing module
- **Testing surface**: Small - 8 test cases cover the essential behaviours

## 8. Open Questions / Decisions for Human Review

### Q1: Should the helper return an `Option<String>` or `String`?
**Recommendation**: Return `String` (empty when marker not found). This matches the logging context where empty output is acceptable.

### Q2: Should the helper include `...` ellipsis prefix/suffix when truncating?
**Recommendation**: No. The current code uses `"...{}..."` formatting in the log statement itself. The ellipsis should remain at the log formatting layer, not in the snippet extraction.

### Q3: Confirm scope exclusion of additional bugs (lib.rs:1383, lib.rs:1393, summarization_manager.rs)?
**Yes/No**: The design excludes these as separate issues. Please confirm or reject.

### Q4: Should the `floor_char_boundary` polyfill be consolidated into a shared crate?
**Recommendation**: Postpone. While there's code duplication (`terraphim_server`, and potentially `terraphim_sessions`), consolidating requires understanding cross-crate dependencies. Not in scope for this bug fix.

---

**Do you approve this plan as-is, or would you like to adjust any part (design, scope, sequencing, testing)?**
