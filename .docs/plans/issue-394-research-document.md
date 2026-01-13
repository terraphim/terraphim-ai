# Research Document: Issue #394 - terraphim-agent replace: case preservation and over-replacement issues

## 1. Problem Restatement and Scope

### Problem Statement
The `terraphim-agent replace` command has two issues:

1. **Case not preserved from heading**: When replacing text using knowledge graph patterns, the output is lowercase instead of preserving the case from the markdown heading.
   - Input: `"Claude Code"`
   - Expected: `"Terraphim AI"` (matching heading case)
   - Actual: `"terraphim ai"` (lowercase)

2. **Over-replacement in URLs and compound terms**: The replacement engine is too aggressive and modifies text within URLs and versioned identifiers.
   - Input: `"[Claude Code](https://claude.ai/claude-code)"`
   - Actual: `"[terraphim ai](https://terraphim ai.com/terraphim ai-code)"`
   - Expected: URLs should not be modified (or only display text)

### IN Scope
- Text replacement case preservation
- Word boundary detection for replacement
- URL and special context exclusion
- Knowledge graph markdown file parsing
- Thesaurus building and storage
- Aho-Corasick automata replacement logic

### OUT of Scope
- Changes to markdown file format (synonyms:: syntax)
- UI/frontend changes
- Performance optimization beyond current levels
- Changes to other terraphim-agent commands

---

## 2. User and Business Outcomes

### User-Visible Changes After Fix
1. Replacements will preserve the case from the markdown heading
2. URLs will not be corrupted by replacement
3. Compound terms with version numbers (e.g., "Claude Opus 4.5") will be handled correctly
4. JSON output in `--json` mode will include case-preserved replacements

### Business Impact
- Improved developer experience with knowledge graph-based text replacement
- Reliable commit message attribution replacement (prepare-commit-msg hook)
- Correct handling of markdown links and documentation

---

## 3. System Elements and Dependencies

### Component Map

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| `NormalizedTermValue` | `crates/terraphim_types/src/lib.rs:203-228` | Stores normalized (lowercase) term values | None |
| `NormalizedTerm` | `crates/terraphim_types/src/lib.rs:235-253` | Holds id, value (lowercase), and optional URL | `NormalizedTermValue` |
| `Concept` | `crates/terraphim_types/src/lib.rs:283-310` | Higher-level concept from filename | `NormalizedTermValue` |
| `concept_from_path()` | `crates/terraphim_automata/src/builder.rs:181-186` | Extracts concept from markdown filename | `Concept` |
| `index_inner()` | `crates/terraphim_automata/src/builder.rs:107-178` | Builds thesaurus from ripgrep messages | `Thesaurus`, `NormalizedTerm` |
| `replace_matches()` | `crates/terraphim_automata/src/matcher.rs:50-79` | Performs Aho-Corasick text replacement | `Thesaurus`, `AhoCorasick` |
| `ReplacementService` | `crates/terraphim_hooks/src/replacement.rs:67-110` | Wrapper service for hooks | `replace_matches()` |
| `Replace` command | `crates/terraphim_agent/src/main.rs:169-183` | CLI command handler | `ReplacementService` |

### Data Flow
```
Markdown File (docs/src/kg/Terraphim AI.md)
    |
    v
ripgrep --json (finds synonyms:: lines)
    |
    v
concept_from_path() -> extracts "Terraphim AI" from filename
    |
    v
Concept::from(String) -> NormalizedTermValue::new() -> LOWERCASES to "terraphim ai"
    |
    v
Thesaurus.insert(synonym, NormalizedTerm{value: "terraphim ai"})
    |
    v
replace_matches() -> AhoCorasick(case_insensitive=true)
    |
    v
Output: lowercase replacement value
```

### Root Cause Location

**Issue 1 (Case Loss)**: `crates/terraphim_types/src/lib.rs:214-216`
```rust
pub fn new(term: String) -> Self {
    let value = term.trim().to_lowercase();  // <-- CASE LOST HERE
    Self(value)
}
```

**Issue 2 (Over-replacement)**: `crates/terraphim_automata/src/matcher.rs:66-69`
```rust
let ac = AhoCorasick::builder()
    .match_kind(MatchKind::LeftmostLongest)
    .ascii_case_insensitive(true)
    .build(patterns)?;  // <-- NO WORD BOUNDARY DETECTION
```

---

## 4. Constraints and Their Implications

### Technical Constraints

| Constraint | Why It Matters | Impact on Solution |
|------------|----------------|-------------------|
| `NormalizedTermValue` used for HashMap keys | Case-insensitive lookup required for matching | Must keep lowercase for keys, add separate field for display |
| Aho-Corasick is optimized for speed | Regex word boundaries would slow matching | Need efficient boundary check approach |
| Thesaurus serialization format | Backward compatibility with persisted data | Migration strategy needed for existing thesauri |
| WASM compatibility | `terraphim_automata` supports WASM target | Solution must not introduce WASM-incompatible deps |

### Business Constraints

| Constraint | Why It Matters | Impact on Solution |
|------------|----------------|-------------------|
| Existing KG file format | Users have existing markdown files with `synonyms::` | Cannot change file format significantly |
| Hook integration | Used in prepare-commit-msg hook | Must maintain fail-open semantics |
| CLI backwards compatibility | Users depend on current output format | New behavior should be opt-in or gradual |

### Performance Constraints

| Constraint | Why It Matters | Impact on Solution |
|------------|----------------|-------------------|
| Replacement in hot path | Used in real-time commit hooks | Word boundary checks must be O(1) per match |
| Large thesauri support | Some roles have hundreds of terms | Cannot add excessive per-term overhead |

---

## 5. Risks, Unknowns, and Assumptions

### UNKNOWNS

1. **U1**: What is the intended behavior for partial matches within compound terms?
   - Should "Claude Opus" match when the synonym is "Claude"?
   - Should only exact word boundaries trigger replacement?

2. **U2**: Should URLs be completely excluded, or should display text be replaceable?
   - `[Claude](url)` - replace "Claude" in display text?
   - What about plain URLs without markdown syntax?

3. **U3**: Should the heading case or the filename case be the source of truth?
   - File: `Terraphim AI.md`, Heading: `# Terraphim AI`
   - Currently uses filename; should we parse the actual heading?

4. **U4**: How should existing persisted thesauri be migrated?
   - Add new `display_value` field to `NormalizedTerm`?
   - Regenerate all thesauri from source files?

### ASSUMPTIONS

- **A1**: Users expect case to match the markdown heading exactly
- **A2**: URLs should never be modified by replacement (standard behavior)
- **A3**: Word boundary matching is acceptable (not substring matching)
- **A4**: Backward compatibility with existing JSON output is not required for new `--json` format
- **A5**: The heading text in markdown files matches the filename

### RISKS

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **R1**: Changing `NormalizedTermValue` breaks serialization | Medium | High | Add new field, don't modify existing |
| **R2**: Word boundary detection misses valid cases | Medium | Medium | Configurable boundary mode |
| **R3**: Performance regression from boundary checks | Low | Medium | Benchmark before/after |
| **R4**: Existing tests fail with new behavior | High | Low | Update tests, add new test cases |
| **R5**: WASM build breaks | Low | High | Test WASM build in CI |

### De-risking Suggestions

1. **For R1**: Add `display_value: Option<String>` to `NormalizedTerm` for case-preserved output
2. **For R2**: Implement configurable modes: `--boundary=word|none` flag
3. **For R3**: Run benchmarks with `cargo bench` before and after changes
4. **For R4**: Add test cases from issue examples before implementing
5. **For R5**: Ensure changes are tested with `wasm32-unknown-unknown` target

---

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity

1. **Dual-purpose `NormalizedTermValue`**: Used for both lookup (case-insensitive) and display (case-preserved)
2. **Multiple replacement contexts**: Plain text, markdown, URLs, code blocks
3. **Thesaurus lifecycle**: Built from markdown, serialized, loaded, used for matching
4. **Multiple consumers**: CLI, hooks, MCP server, TUI all use the same core

### Simplification Strategies

#### Strategy 1: Separate Display Value
Add `display_value: String` to `NormalizedTerm` struct to store the original case:
- Lookup key remains lowercase for case-insensitive matching
- Display value used for replacement output
- Minimal change to existing logic

#### Strategy 2: Context-Aware Replacement
Implement a pre-processing step to identify and protect special contexts:
- Parse markdown for URLs `[text](url)` and protect the URL portion
- Use regex to identify URL patterns and skip them
- Apply replacement only to unprotected text

#### Strategy 3: Word Boundary Mode (Optional Enhancement)
Add boundary detection after Aho-Corasick matching:
- Keep fast substring matching
- Post-filter matches that don't have word boundaries
- Configurable via CLI flag

### Recommended Approach
Implement Strategy 1 + Strategy 2 as the core fix:
1. Add `display_value` field for case preservation
2. Parse and protect URLs before replacement
3. Consider Strategy 3 as a future enhancement

---

## 7. Questions for Human Reviewer

1. **Case Source**: Should the display case come from the markdown heading text or the filename? (Currently: filename)

2. **URL Handling**: For markdown links `[display](url)`, should we:
   a) Replace only the display text?
   b) Replace nothing in the entire link?
   c) Make this configurable?

3. **Word Boundaries**: Should replacement require word boundaries by default?
   - "Claude" in "ClaudeCode" - should this match?
   - "npm" in "npm-scripts" - should this match?

4. **Backward Compatibility**: Is it acceptable to change the default output behavior, or should case preservation be opt-in via flag?

5. **Compound Terms**: For "Claude Opus 4.5", if the synonym is just "Claude", should:
   a) The whole compound be preserved?
   b) Only "Claude" be replaced?
   c) Nothing be replaced (no word boundary match)?

6. **Migration**: For existing persisted thesauri, should we:
   a) Require manual regeneration?
   b) Auto-migrate on first load?
   c) Support both old and new formats?

7. **Performance Budget**: Is there an acceptable performance regression threshold for the boundary checking feature? (e.g., <10% slower)

---

## Appendix: Affected Files Summary

### Must Modify
- `crates/terraphim_types/src/lib.rs` - Add `display_value` to `NormalizedTerm`
- `crates/terraphim_automata/src/builder.rs` - Store original case in `display_value`
- `crates/terraphim_automata/src/matcher.rs` - Use `display_value` for replacement output, add URL protection

### Should Modify
- `crates/terraphim_hooks/src/replacement.rs` - Update to use case-preserved values
- `crates/terraphim_agent/tests/replace_feature_tests.rs` - Add new test cases

### Consider Modifying
- `crates/terraphim_persistence/src/thesaurus.rs` - Handle migration
- `crates/terraphim_agent/src/main.rs` - Add `--boundary` flag option

---

## Appendix: Test Cases from Issue

### Case Preservation Test
```bash
# Knowledge graph: docs/src/kg/Terraphim AI.md
# Contents: # Terraphim AI
#           synonyms:: Claude Code, Claude

echo "Claude Code" | terraphim-agent replace
# Expected: Terraphim AI
# Actual:   terraphim ai
```

### URL Over-replacement Test
```bash
echo "Generated with [Claude Code](https://claude.ai/claude-code)" | terraphim-agent replace
# Expected: Generated with [Terraphim AI](https://claude.ai/claude-code)
# Actual:   Generated with [terraphim ai](https://terraphim ai.com/terraphim ai-code)
```

### Compound Term Test
```bash
echo "Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>" | terraphim-agent replace
# Expected: Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>  (no change, compound term)
# Actual:   Co-Authored-By: terraphim ai 4.5 <noreply@anthropic.com>
```
