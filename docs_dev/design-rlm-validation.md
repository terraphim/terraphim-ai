# Design & Implementation Plan: Validate terraphim_rlm with All Examples

## 1. Summary of Target Behavior

After implementation, the terraphim_rlm crate will have comprehensive validation of all examples found in:
- Rustdoc documentation examples (currently marked `ignore`)
- Unit tests in each module
- MCP tool examples (when feature enabled)

The validation will:
1. Confirm all rustdoc examples compile and run correctly (with mocks where needed)
2. Verify all existing unit tests pass
3. Create explicit validation tests for each documented example
4. Produce a validation report showing pass/fail status for each example

## 2. Key Invariants and Acceptance Criteria

### Invariants
- All validation must work without real Firecracker VMs (use MockExecutor)
- Validation must work with `cargo test --package terraphim_rlm`
- Feature-gated code must be validated with appropriate feature flags
- Tests must clean up after themselves (no leftover state)

### Acceptance Criteria

| ID | Criterion | Testable? | Priority |
|----|-----------|-----------|----------|
| AC1 | All rustdoc examples can be extracted and compiled | Yes | High |
| AC2 | All rustdoc examples run to completion with MockExecutor | Yes | High |
| AC3 | All existing unit tests pass | Yes | High |
| AC4 | MCP tool examples validate correctly (when `mcp` feature enabled) | Yes | Medium |
| AC5 | Validation report generated with clear pass/fail status | Yes | Medium |
| AC6 | No real VMs required for basic validation | Yes | High |
| AC7 | Feature flag combinations tested (default, full, minimal) | Yes | Low |

## 3. High-Level Design and Boundaries

### Validation Architecture

```
Validation Layer 1: Doc Tests
  └── Extract rustdoc examples from source
  └── Convert to runnable doctests OR create explicit test functions
  └── Run with `cargo test --doc`

Validation Layer 2: Unit Test Audit
  └── Inventory all #[test] and #[tokio::test] functions
  └── Categorise by module and feature gate
  └── Run with `cargo test --package terraphim_rlm`

Validation Layer 3: Example Validation Tests (NEW)
  └── Create tests/examples_validation.rs
  └── Each rustdoc example becomes a test function
  └── Use MockExecutor for execution tests
  └── Mock LLM bridge for query tests

Validation Layer 4: Feature-Gated Validation
  └── Run with --features full
  └── Run with --features mcp
  └── Run with --no-default-features
```

### Boundaries

**Changes INSIDE terraphim_rlm crate:**
- `tests/` directory - new file `examples_validation.rs`
- Potentially modify rustdoc examples to remove `ignore` (if they can run with mocks)

**Changes OUTSIDE crate:**
- `docs_dev/validation-report.md` - output report (in project docs, not crate)

**NOT modifying:**
- Existing test code (only auditing)
- Production source code (unless bugs found)
- Feature gate configurations

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `tests/examples_validation.rs` | **CREATE** | N/A | New validation tests | Uses MockExecutor, TerraphimRlm |
| `src/lib.rs` | **MODIFY** | Examples marked `ignore` | Some examples maybe `no_run` or runnable | None |
| `src/rlm.rs` | **MODIFY** | Examples marked `rust,ignore` | Convert to runnable or create test refs | None |
| `docs_dev/validation-report.md` | **CREATE** | N/A | Validation results report | None |
| `docs_dev/task_plan.md` | **MODIFY** | Phase 1-2 plan | Add Phase 3 (implementation) | None |

### Mock Infrastructure Required

Since real Firecracker VMs aren't available in CI, create a comprehensive MockExecutor:

```rust
// In tests/common/mod.rs or tests/examples_validation.rs
struct ValidationMockExecutor {
    capabilities: Vec<Capability>,
    // Track calls for assertions
}
```

## 5. Step-by-Step Implementation Sequence

### Step 1: Audit Existing Examples
**Purpose:** Create complete inventory of all examples
**Deployable state?** N/A (research)
1. Grep all `.rs` files for ````rust` and `# Example` patterns
2. Extract each example with its context (module, feature gate)
3. Categorise: documentation example, unit test, integration test
4. Save inventory to `docs_dev/findings.md` (UPDATE existing)

### Step 2: Create Mock Infrastructure for Validation
**Purpose:** Enable testing without real VMs
**Deployable state?** Yes (tests compile but don't run yet)
1. Create `tests/common/mod.rs` with `ValidationMockExecutor`
2. Implement `ExecutionEnvironment` trait with appropriate mocking
3. Add mock for LLM bridge (if testing `query()` examples)
4. Ensure mocks can simulate both success and failure cases

### Step 3: Create Examples Validation Test File
**Purpose:** Explicit tests for each documented example
**Deployable state?** Yes (tests run with mocks)
1. Create `tests/examples_validation.rs`
2. For each rustdoc example in `lib.rs`, `rlm.rs`, `executor/trait.rs`:
   - Create a test function that exercises the same code
   - Use MockExecutor instead of real VMs
   - Assert expected behavior
3. Run `cargo test --package terraphim_rlm` to verify

### Step 4: Convert or Reference Documentation Examples
**Purpose:** Make rustdoc examples validateable
**Deployable state?** Yes (doctests run)
**Feature flag needed?** No (uses mocks)

Options (choose one based on human feedback):
- **Option A:** Remove `ignore` from examples, add mock setup in doctest
- **Option B:** Keep `ignore`, reference examples in validation tests (already done in Step 3)
- **Option C:** Change `ignore` to `no_run` where examples compile but can't run

Recommendation: **Option B** - keeps docs clean, validation tests are explicit.

### Step 5: Run Validation with Different Feature Flags
**Purpose:** Ensure all feature combinations work
**Deployable state?** Yes
1. `cargo test --package terraphim_rlm --no-default-features`
2. `cargo test --package terraphim_rlm` (default features)
3. `cargo test --package terraphim_rlm --features full`
4. `cargo test --package terraphim_rlm --features mcp` (if MCP tools validation added)

### Step 6: Generate Validation Report
**Purpose:** Document what was validated
**Deployable state?** N/A (documentation)
1. Create `docs_dev/validation-report.md`
2. List all examples with: location, type, validation method, status
3. Include feature-gated examples with appropriate flags
4. Document any examples that couldn't be validated (with reason)

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| AC1: Doc examples compile | Doc test or unit test | `tests/examples_validation.rs` |
| AC2: Doc examples run | Unit test with mocks | `tests/examples_validation.rs` |
| AC3: Unit tests pass | Existing `#[test]` functions | Each module's `mod tests` |
| AC4: MCP tools validate | Unit test with mocks | `tests/examples_validation.rs` (behind `#[cfg(feature = "mcp")]`) |
| AC5: Report generated | Manual verification | `docs_dev/validation-report.md` |
| AC6: No VMs required | CI check | `cargo test` exits 0 without Firecracker |
| AC7: Feature combos | Multiple test runs | CI matrix |

### Test Categories in `examples_validation.rs`

```rust
// Category 1: Basic API examples (from lib.rs)
mod api_examples {
    // Test: Create TerraphimRlm with default config
    // Test: Create session, execute code, destroy session
}

// Category 2: RLM examples (from rlm.rs)
mod rlm_examples {
    // Test: Full query loop (with mocked LLM)
    // Test: Snapshot create/restore
    // Test: Context variables
}

// Category 3: Executor trait examples (from executor/trait.rs)
mod executor_examples {
    // Test: execute_code, execute_command signatures
}

// Category 4: MCP tools (from mcp_tools.rs, feature-gated)
#[cfg(feature = "mcp")]
mod mcp_examples {
    // Test: rlm_code, rlm_bash, rlm_query tools
}
```

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|-------------|----------------|
| **MockExecutor doesn't catch real bugs** | Use mocks only for examples; integration tests separately | Low - examples are documentation, not integration |
| **LLM bridge mocking is complex** | Create simple mock that returns predefined responses | Medium - may not catch LLM parsing bugs |
| **Feature-gated code not compiled in CI** | Run multiple test commands with different flags | Low - process documented |
| **Some examples need real VMs** | Clearly document which examples need real infrastructure | Low - use `#[ignore]` for these |
| **Examples change over time** | Add CI check to fail if example count changes | Low - validation report shows drift |

### Complexity Sources
1. **Multiple feature flags** - mitigated by testing key combinations only
2. **Async everywhere** - mitigated by using `#[tokio::test]`
3. **Mock complexity** - mitigated by reusing MockExecutor pattern from existing tests

## 8. Open Questions / Decisions for Human Review

1. **Which Option for documentation examples?**
   - Option A: Remove `ignore`, make runnable with mocks in doctest
   - Option B: Keep `ignore`, create separate validation tests (RECOMMENDED)
   - Option C: Use `no_run` for compile-only checks

2. **Should we mock the LLM bridge?** The `query()` method needs an LLM service. Should we:
   - Create a full mock that simulates LLM responses
   - Skip `query()` example validation (mark as `#[ignore]`)
   - Only test the API signature, not the full loop

3. **How comprehensive should MCP tool validation be?** Given it's feature-gated:
   - Full validation with mocks
   - Basic smoke test only
   - Skip for now, add later

4. **Should validation be part of `cargo test`?** Or a separate command?
   - Integrate into existing test suite
   - Create a separate `cargo test --test examples_validation`
   - Both: integration + separate report

5. **What to do with examples that need real VMs?** (Firecracker, Docker, E2B)
   - Clearly mark as requiring infrastructure (document only)
   - Create `#[ignore]` tests that could run with real VMs
   - Skip entirely from validation

6. **Should we add `#[should_panic]` tests?** Some examples may show error handling:
   - Yes, add explicit error case tests
   - No, only test happy path in validation
   - Only if the example demonstrates error handling

## Next Steps After Approval

1. Update `docs_dev/task_plan.md` with Phase 3 (Implementation)
2. Execute Step 1: Audit existing examples (update findings.md)
3. Execute Step 2: Create mock infrastructure
4. Execute Step 3: Create examples_validation.rs
5. Execute Step 4-6: Convert, test features, generate report
6. Run quality evaluation on this design document
