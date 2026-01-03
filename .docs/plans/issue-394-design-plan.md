# Design and Implementation Plan: Issue #394 - Case Preservation and URL Protection

## 1. Summary of Target Behavior

After implementation, the `terraphim-agent replace` command will:

1. **Preserve case from markdown headings**: Replacements will output text matching the case from the markdown file heading (e.g., "Terraphim AI" not "terraphim ai")

2. **Protect URLs from replacement**: Text within URL patterns will not be modified, including:
   - Markdown links: `[display](url)` - only display text is replaceable
   - Plain URLs: `https://example.com/path` - never modified
   - Email addresses: `user@domain.com` - never modified

3. **Maintain backward compatibility**:
   - Existing thesauri without display values will use normalized (lowercase) value as fallback
   - Case-insensitive matching behavior is preserved
   - JSON output format remains compatible

---

## 2. Key Invariants and Acceptance Criteria

### Invariants

| Invariant | Guarantee |
|-----------|-----------|
| **I1**: Case-insensitive matching | "Claude", "claude", "CLAUDE" all match the same thesaurus entry |
| **I2**: Thesaurus key uniqueness | Normalized lowercase keys ensure no duplicates |
| **I3**: URL integrity | URLs (http/https/mailto) are never corrupted by replacement |
| **I4**: Serialization compatibility | Thesauri with `display_value: None` deserialize correctly |
| **I5**: WASM compatibility | All changes compile for `wasm32-unknown-unknown` target |

### Acceptance Criteria

| ID | Criterion | Testable Assertion |
|----|-----------|-------------------|
| **AC1** | Case preserved from heading | `echo "Claude Code" \| replace` outputs "Terraphim AI" (capital T, A, I) |
| **AC2** | Plain URLs unchanged | `https://claude.ai/code` remains unchanged |
| **AC3** | Markdown link URLs protected | `[text](https://x.com)` - only "text" is replaced, URL unchanged |
| **AC4** | Email addresses protected | `user@claude.ai` remains unchanged |
| **AC5** | Existing thesauri work | Thesauri without `display_value` use `value` as fallback |
| **AC6** | JSON output has case-preserved values | `--json` output shows "Terraphim AI" not "terraphim ai" |
| **AC7** | Replacement count accurate | `HookResult.replacements` counts only actual text changes |

---

## 3. High-Level Design and Boundaries

### Conceptual Solution

```
                         Knowledge Graph File
                                |
                                v
                    +------------------------+
                    |   Thesaurus Builder    |
                    |  (extract heading case)|
                    +------------------------+
                                |
                                v
                    +------------------------+
                    |    NormalizedTerm      |
                    |  value: lowercase      |
                    |  display_value: Option |  <-- NEW FIELD
                    +------------------------+
                                |
                                v
                    +------------------------+
                    |   URL Protector        |  <-- NEW COMPONENT
                    | (identify & mask URLs) |
                    +------------------------+
                                |
                                v
                    +------------------------+
                    |   Aho-Corasick Match   |
                    | (case-insensitive)     |
                    +------------------------+
                                |
                                v
                    +------------------------+
                    |   Replacement Output   |
                    | (use display_value)    |
                    +------------------------+
```

### Component Boundaries

| Component | Changes | New Responsibility |
|-----------|---------|-------------------|
| `NormalizedTerm` | Modify | Add `display_value: Option<String>` field |
| `NormalizedTermValue` | No change | Remains lowercase for key lookups |
| `ThesaurusBuilder` (Logseq) | Modify | Extract original case from filename for display_value |
| `replace_matches()` | Modify | Use display_value for output, integrate URL protection |
| `UrlProtector` | **NEW** | Identify and mask URL patterns before replacement |
| `ReplacementService` | Minimal change | Pass through updated behavior |

### Avoiding Complection

1. **Separation of matching vs output**: Keep `value` for matching (lowercase), `display_value` for output
2. **URL protection as pre-processing**: Handle URL masking before Aho-Corasick, not inside it
3. **Optional field**: `display_value: Option<String>` allows graceful fallback to `value`

---

## 4. File/Module-Level Change Plan

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `crates/terraphim_types/src/lib.rs` | Modify | `NormalizedTerm { id, value, url }` | Add `display_value: Option<String>` field | None |
| `crates/terraphim_automata/src/builder.rs` | Modify | `concept_from_path()` returns concept with lowercase value | Store original case in `display_value` | `NormalizedTerm` |
| `crates/terraphim_automata/src/matcher.rs` | Modify | `replace_matches()` uses `value.to_string()` | Use `display_value.unwrap_or(value)` for output | `NormalizedTerm`, new `url_protector` module |
| `crates/terraphim_automata/src/url_protector.rs` | **Create** | N/A | URL pattern detection and masking | `regex` crate (already in deps) |
| `crates/terraphim_automata/src/lib.rs` | Modify | Exports existing modules | Add `pub mod url_protector` | New module |
| `crates/terraphim_hooks/src/replacement.rs` | Minimal | Calls `replace_matches()` | No API change, behavior updated | `replace_matches()` |
| `crates/terraphim_agent/tests/replace_feature_tests.rs` | Modify | Tests lowercase output | Add tests for case preservation and URL protection | Test fixtures |

### Detailed Changes

#### `NormalizedTerm` (terraphim_types)
```
Before:
  struct NormalizedTerm {
      id: u64,
      value: NormalizedTermValue,  // lowercase
      url: Option<String>,
  }

After:
  struct NormalizedTerm {
      id: u64,
      value: NormalizedTermValue,           // lowercase (for matching)
      display_value: Option<String>,        // original case (for output) <-- NEW
      url: Option<String>,
  }
```

#### `concept_from_path()` (terraphim_automata/builder.rs)
```
Before:
  - Extracts filename stem
  - Creates Concept with NormalizedTermValue (lowercase)

After:
  - Extracts filename stem as original_case: String
  - Creates Concept with NormalizedTermValue (lowercase) for matching
  - Stores original_case in display_value
```

#### `url_protector.rs` (NEW)
```
Purpose:
  - Define URL pattern regex (http, https, mailto)
  - Function to identify URL spans in text
  - Function to mask URLs with placeholders
  - Function to restore URLs after replacement
```

#### `replace_matches()` (terraphim_automata/matcher.rs)
```
Before:
  - Build Aho-Corasick from patterns
  - Replace all matches with value.to_string()

After:
  - Mask URLs in input text
  - Build Aho-Corasick from patterns
  - Replace all matches with display_value.unwrap_or(value.to_string())
  - Restore masked URLs
```

---

## 5. Step-by-Step Implementation Sequence

Each step keeps the system deployable and tests passing.

### Step 1: Add `display_value` field to `NormalizedTerm`
- **Purpose**: Enable storage of original case
- **Deployable**: Yes, field is `Option<String>` with default `None`
- **Changes**: `crates/terraphim_types/src/lib.rs`
- **Tests**: Existing tests pass (field is optional)

### Step 2: Update `NormalizedTerm::new()` to accept display_value
- **Purpose**: Allow callers to provide display value
- **Deployable**: Yes, backward compatible
- **Changes**: Add `with_display_value()` builder method
- **Tests**: Add unit test for new method

### Step 3: Update `index_inner()` to store original case
- **Purpose**: Capture heading case during thesaurus build
- **Deployable**: Yes
- **Changes**: `crates/terraphim_automata/src/builder.rs`
- **Tests**: Add test that verifies display_value is set

### Step 4: Create `url_protector` module
- **Purpose**: URL pattern detection and masking
- **Deployable**: Yes, no integration yet
- **Changes**: Create `crates/terraphim_automata/src/url_protector.rs`
- **Tests**: Unit tests for URL detection

### Step 5: Update `replace_matches()` to use display_value
- **Purpose**: Output case-preserved text
- **Deployable**: Yes
- **Changes**: `crates/terraphim_automata/src/matcher.rs`
- **Tests**: Update existing tests, add case preservation tests

### Step 6: Integrate URL protection into `replace_matches()`
- **Purpose**: Prevent URL corruption
- **Deployable**: Yes
- **Changes**: `crates/terraphim_automata/src/matcher.rs`
- **Tests**: Add URL protection tests

### Step 7: Update integration tests
- **Purpose**: Validate end-to-end behavior
- **Deployable**: Yes
- **Changes**: `crates/terraphim_agent/tests/replace_feature_tests.rs`
- **Tests**: Add tests from issue examples

### Step 8: Verify WASM compatibility
- **Purpose**: Ensure no WASM breakage
- **Deployable**: Yes
- **Changes**: Run `./scripts/build-wasm.sh`
- **Tests**: WASM test suite passes

---

## 6. Testing and Verification Strategy

| Acceptance Criteria | Test Type | Test Location | Test Description |
|---------------------|-----------|---------------|------------------|
| AC1: Case preserved | Unit | `matcher.rs::tests` | `replace_matches()` outputs display_value |
| AC1: Case preserved | Integration | `replace_feature_tests.rs` | CLI test with "Claude Code" -> "Terraphim AI" |
| AC2: Plain URLs unchanged | Unit | `url_protector.rs::tests` | URLs detected and masked correctly |
| AC3: Markdown link URLs | Unit | `url_protector.rs::tests` | `[text](url)` - only text portion replaceable |
| AC4: Email addresses | Unit | `url_protector.rs::tests` | `user@domain.com` detected as URL |
| AC5: Backward compat | Unit | `types::tests` | `NormalizedTerm` with `None` display_value works |
| AC6: JSON output | Integration | `replace_feature_tests.rs` | `--json` output has case-preserved values |
| AC7: Replacement count | Unit | `matcher.rs::tests` | Count reflects actual changes made |
| I5: WASM compat | Build | `scripts/build-wasm.sh` | WASM build succeeds |

### New Test Cases (from Issue)

```rust
// Test case 1: Case preservation
#[test]
fn test_replace_preserves_heading_case() {
    // Create thesaurus with display_value = "Terraphim AI"
    // Input: "Claude Code"
    // Expected: "Terraphim AI" (not "terraphim ai")
}

// Test case 2: URL protection
#[test]
fn test_replace_protects_urls() {
    // Input: "Visit https://claude.ai/code for more"
    // Expected: URL unchanged, surrounding text may be replaced
}

// Test case 3: Markdown link protection
#[test]
fn test_replace_protects_markdown_link_urls() {
    // Input: "[Claude Code](https://claude.ai/claude-code)"
    // Expected: "[Terraphim AI](https://claude.ai/claude-code)"
}

// Test case 4: Email protection
#[test]
fn test_replace_protects_email_addresses() {
    // Input: "Contact noreply@anthropic.com"
    // Expected: Email unchanged
}
```

---

## 7. Risk and Complexity Review

| Risk (from Phase 1) | Mitigation in Design | Residual Risk |
|---------------------|---------------------|---------------|
| **R1**: Serialization breaks | `display_value: Option<String>` with `#[serde(default)]` | Low - serde handles missing fields |
| **R2**: Word boundary misses | URL protection is separate concern; word boundaries deferred | Medium - compound terms like "Claude Opus 4.5" still need consideration |
| **R3**: Performance regression | URL regex compiled once, reused; masking is O(n) | Low - negligible overhead |
| **R4**: Existing tests fail | Tests updated incrementally per step | Low - each step keeps tests passing |
| **R5**: WASM breaks | Step 8 explicitly verifies WASM build | Low - no new deps that break WASM |

### Complexity Assessment

| Area | Complexity | Justification |
|------|------------|---------------|
| Type changes | Low | Single optional field addition |
| Builder changes | Low | Store one additional value |
| URL protection | Medium | Regex patterns for URLs, edge cases |
| Replacement logic | Low | Simple fallback logic |
| Testing | Medium | Multiple new test cases needed |

---

## 8. Open Questions / Decisions for Human Review

### Decision 1: Compound Terms (Deferred)
The issue mentions "Claude Opus 4.5" being incorrectly modified. This design addresses:
- Case preservation (will output "Terraphim AI" with correct case)
- URL protection (will not modify URLs)

However, **word boundary matching** for compound terms is deferred. Should we:
- a) Accept this as a future enhancement?
- b) Add it to this implementation scope?

**Recommendation**: Defer to future issue. URL protection handles the most critical case.

### Decision 2: Markdown Link Display Text
For `[Claude Code](url)`, this design replaces "Claude Code" in display text. Is this the desired behavior?
- a) Yes, replace display text only
- b) No, don't replace anything in markdown links
- c) Make it configurable

**Recommendation**: Option (a) - replace display text, protect URL.

### Decision 3: Filename vs Heading as Case Source
Currently, case comes from the filename (e.g., `Terraphim AI.md`). Should we parse the actual `# Heading` from the markdown?
- a) Use filename (current approach, simpler)
- b) Parse heading from file content (more accurate)

**Recommendation**: Option (a) - use filename. Parsing headings adds complexity and the convention is filename = heading.

---

## Summary

This design plan addresses Issue #394 with minimal changes:

1. **Add `display_value` field** to `NormalizedTerm` for case preservation
2. **Create `url_protector` module** for URL pattern detection and masking
3. **Update `replace_matches()`** to use display_value and integrate URL protection

The implementation is broken into 8 incremental steps, each keeping the system deployable with passing tests.

---

**Do you approve this plan as-is, or would you like to adjust any part?**
