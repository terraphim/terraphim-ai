# Research Document: Fix SearchResultDoc Compilation Errors in Integration Tests

## 1. Problem Restatement and Scope

**Problem:** The integration test file `kg_ranking_integration_test.rs` fails to compile due to undefined type `SearchResultDoc`. This type is used in two locations (lines 454 and 604) but was never defined in the codebase.

**IN Scope:**
- Fix compilation errors in `crates/terraphim_agent/tests/kg_ranking_integration_test.rs`
- Verify and fix any similar issues in `cross_mode_consistency_test.rs`
- Ensure all test files compile and pass

**OUT of Scope:**
- Implementing actual CLI mode functionality (already disabled in tests)
- Adding new features or functionality
- Refactoring test logic beyond compilation fixes

## 2. User & Business Outcomes

**Visible Changes:**
- Integration tests compile successfully
- Test suite runs without compilation errors
- CI/CD pipeline passes test compilation phase

**Business Value:**
- Prevents CI failures blocking merges
- Maintains code quality standards
- Enables proper testing of KG ranking functionality

## 3. System Elements and Dependencies

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| `kg_ranking_integration_test.rs` | `crates/terraphim_agent/tests/` | Integration test for KG ranking | `terraphim_agent::client::ApiClient`, `terraphim_types` |
| `cross_mode_consistency_test.rs` | `crates/terraphim_agent/tests/` | Cross-mode consistency tests | Same dependencies + `NormalizedResult` (local struct) |
| `repl_integration_tests.rs` | `crates/terraphim_agent/tests/` | REPL command tests | `terraphim_agent::repl::commands` |
| `Document` type | `terraphim_types::lib.rs` | Core document type | Used in search results |

## 4. Constraints and Their Implications

**Constraint 1: No SearchResultDoc type exists**
- **Implication:** Cannot use this type; must either define it or use existing types
- **Why it matters:** Prevents compilation, blocking all test execution

**Constraint 2: CLI mode is disabled in tests (commented out)**
- **Implication:** The `SearchResultDoc` usage is in dead code paths
- **Why it matters:** Can safely comment out/remove these lines without affecting test functionality

**Constraint 3: Existing pattern in cross_mode_consistency_test.rs**
- **Implication:** Uses locally-defined `NormalizedResult` struct for cross-mode normalization
- **Why it matters:** Provides a pattern for consistency

**Constraint 4: Test files must compile without warnings**
- **Implication:** Cannot leave unused variables or dead code
- **Why it matters:** CI/CD pipeline enforces clean builds

## 5. Risks, Unknowns, and Assumptions

**ASSUMPTION 1:** The `SearchResultDoc` type was intended to be a placeholder for future CLI functionality that was never implemented.
- **Risk Level:** Low
- **De-risking:** Code comments confirm CLI mode is disabled intentionally

**ASSUMPTION 2:** The test logic doesn't actually need CLI comparison for current test goals.
- **Risk Level:** Medium
- **De-risking:** Tests are marked as server-mode only; CLI paths are commented out

**UNKNOWN 1:** Whether there are other undefined types in the test files.
- **Risk Level:** Medium
- **De-risking:** Need to compile both test files to verify

**RISK 1:** Removing the lines might reduce test coverage.
- **Mitigation:** The CLI mode tests are already disabled; this is just cleanup of dead code

## 6. Context Complexity vs. Simplicity Opportunities

**Complexity Sources:**
- Multiple test files with interrelated functionality
- Disabled code paths mixed with active code
- Type definitions scattered across crates

**Simplification Strategies:**
1. **Comment out dead code** rather than defining unused types
2. **Use existing `NormalizedResult` pattern** from cross_mode_consistency_test.rs if needed
3. **Focus on compilation only** - don't refactor test logic

## 7. Questions for Human Reviewer

1. **Should CLI mode functionality be fully removed from these tests, or just commented out?**
   - *Why it matters:* Affects whether we delete vs. comment the SearchResultDoc lines

2. **Is there a long-term plan to enable CLI mode testing that requires SearchResultDoc to be defined?**
   - *Why it matters:* If yes, we should define the type properly instead of commenting out

3. **Should we align all test files to use a common result type (like NormalizedResult)?**
   - *Why it matters:* Affects consistency across test suite

4. **Are there other test files with similar disabled CLI mode code that needs cleanup?**
   - *Why it matters:* Scope of similar fixes needed

5. **What's the priority: quick compilation fix vs. proper type definition and test refactoring?**
   - *Why it matters:* Determines whether we do minimal fix or comprehensive solution
