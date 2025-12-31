# Design & Implementation Plan: Fix All CI Workflow Failures

## 1. Summary of Target Behavior

After implementation:
1. **Query parser** correctly treats mixed-case keywords ("oR", "Or", "AND", etc.) as concepts, not boolean operators
2. **Earthly CI/CD** includes `terraphim_ai_nodejs` in the build and passes all checks
3. **CI Optimized** workflow runs successfully with all lint/format checks passing

## 2. Key Invariants and Acceptance Criteria

### Invariants
- Query parser MUST only recognize lowercase keywords: "and", "or", "not"
- All workspace members in Cargo.toml MUST be copied in Earthfile
- CI workflows MUST pass without manual intervention

### Acceptance Criteria
| Criterion | Verification Method |
|-----------|-------------------|
| "oR" is parsed as concept, not OR keyword | Proptest passes consistently |
| "AND" is parsed as concept, not AND keyword | Unit test |
| Earthly `+lint-and-format` target passes | `earthly +lint-and-format` |
| CI PR Validation workflow passes | GitHub Actions check |
| CI Optimized Main workflow passes | GitHub Actions check |

## 3. High-Level Design and Boundaries

### Component Changes

**Query Parser (crates/claude-log-analyzer/src/kg/query.rs)**
- Change from case-insensitive to case-sensitive keyword matching
- Only exact lowercase "and", "or", "not" are treated as operators
- All other variations ("AND", "Or", "NOT") become concepts

**Earthfile**
- Add `terraphim_ai_nodejs` to COPY commands at lines 120 and 162
- Ensure all workspace members are synchronized

### No Changes Required
- CI Optimized workflow file itself (failure was downstream of Earthly)
- Rate limiting configuration (already fixed)

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/claude-log-analyzer/src/kg/query.rs:69-76` | Modify | Case-insensitive keyword matching via `to_lowercase()` | Case-sensitive exact match | None |
| `Earthfile:120` | Modify | Missing `terraphim_ai_nodejs` | Include `terraphim_ai_nodejs` in COPY | None |
| `Earthfile:162` | Modify | Missing `terraphim_ai_nodejs` | Include `terraphim_ai_nodejs` in COPY | None |

## 5. Step-by-Step Implementation Sequence

### Step 1: Fix Query Parser Keyword Matching
**Purpose**: Make keyword matching case-sensitive so only lowercase keywords are operators
**Deployable state**: Yes - backwards compatible change, stricter parsing

Change `word_to_token()` function:
```rust
// Before (line 70):
match word.to_lowercase().as_str() {

// After:
match word {
```

This ensures:
- "and" → Token::And (operator)
- "AND" → Token::Concept("AND") (not operator)
- "oR" → Token::Concept("oR") (not operator)

### Step 2: Add Regression Test
**Purpose**: Prevent future regressions with explicit test cases
**Deployable state**: Yes

Add test for mixed-case keywords being treated as concepts.

### Step 3: Update Earthfile COPY Commands
**Purpose**: Include all workspace members in build
**Deployable state**: Yes

Modify lines 120 and 162 to include `terraphim_ai_nodejs`:
```
COPY --keep-ts --dir terraphim_server terraphim_firecracker terraphim_ai_nodejs desktop default crates ./
```

### Step 4: Verify CI Passes
**Purpose**: Confirm all fixes work together
**Deployable state**: Yes

Run local tests and push to trigger CI.

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| Mixed-case keywords are concepts | Unit | `query.rs::tests::test_mixed_case_keywords` |
| Proptest passes | Property | `query.rs::tests::test_boolean_expression_parsing` |
| Earthly build succeeds | Integration | `earthly +lint-and-format` |
| CI workflows pass | E2E | GitHub Actions |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Breaking existing queries using uppercase keywords | This is intentional - uppercase should be concepts | Low - existing queries were likely incorrect |
| Earthfile change breaks other targets | Only affects COPY, not build logic | Low |
| Proptest still fails with other shrunk cases | Case-sensitive matching addresses root cause | Low |

## 8. Open Questions / Decisions for Human Review

None - the fix is straightforward:
1. Case-sensitive keyword matching is the correct behavior
2. All workspace members should be in Earthfile

---

**Do you approve this plan as-is, or would you like to adjust any part?**
