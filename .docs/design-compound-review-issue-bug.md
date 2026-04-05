# Design & Implementation Plan: Compound Review Issue Serialisation Fix

## 1. Summary of Target Behavior

When compound review files a Gitea issue for a finding, the issue title and body must contain properly escaped, human-readable text. Raw JSON in `finding.finding` should be sanitised before being embedded in markdown content.

## 2. Key Invariants and Acceptance Criteria

| Criterion | Test |
|-----------|------|
| Issue titles contain no raw JSON syntax (`{`, `}`, `[`, `]`, `"`) in visible position | Manual inspection of created issues |
| Issue body renders as proper markdown | Gitea web UI renders correctly |
| Dedup search still works with sanitised keywords | Query existing issues with partial cleaned text |
| No regressions in non-JSON findings | Existing compound review runs still work |

## 3. High-Level Design and Boundaries

**Solution:** Add a sanitisation layer in `file_finding_issue` that:
1. Detects if finding text contains JSON-like patterns
2. Strips or escapes JSON syntax characters for title (strict)
3. Escapes for markdown in body (allow backticks to render as code)

**Components:**
- `terraphim_orchestrator::lib.rs`: Add sanitisation in `file_finding_issue`
- No new files or external dependencies

**Boundaries:**
- Change limited to issue filing code only
- No changes to review agent output parsing
- No changes to Gitea API client

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After |
|-------------|--------|--------|-------|
| `crates/terraphim_orchestrator/src/lib.rs` | Modify | `file_finding_issue` embeds raw finding text | Sanitise finding text before embedding |

**Specific changes:**
1. Add helper function `sanitise_for_title(input: &str) -> String` - strips JSON syntax, limits length
2. Add helper function `sanitise_for_body(input: &str) -> String` - escapes markdown special chars
3. Apply sanitisation at lines 1349-1354 (title) and 1374 (body)

## 5. Step-by-Step Implementation Sequence

1. **Add sanitisation functions** - Create two helper functions in lib.rs for title/body sanitisation
2. **Apply to title** - Update title construction at line 1346-1354 to use title sanitiser
3. **Apply to body** - Update body construction at line 1374 to use body sanitiser
4. **Test manually** - Run compound review and verify created issues are readable

Each step is a small, reversible change. No feature flags needed - this is a bug fix.

## 6. Testing & Verification Strategy

| Acceptance Criterion | Test Type | Test Location |
|---------------------|-----------|---------------|
| Title has no JSON syntax | Manual | Run compound review, inspect issue #277+ |
| Body renders as markdown | Manual | View issue in Gitea web UI |
| Dedup still works | Integration | Run twice, second should skip duplicate |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Escaping breaks dedup | Use original text for dedup keyword, cleaned for display | Low - dedup will still match on key terms |
| Some findings legitimately contain JSON | Strip only obvious JSON syntax, preserve actual finding text | Low - if finding is about JSON, summary should still be readable |
| Body escaping incomplete | Use markdown escaping for common chars (`` ` ``, `*`, `_`, etc.) | Low - can iterate if new issues found |

## 8. Open Questions / Decisions for Human Reviewer

1. Should we strip JSON syntax entirely or escape it? (Strip chosen for readability)
2. Should dedup use original or cleaned text? (Original to ensure matches)
3. Is there a preferred character limit for title? (80 chars - current limit)