# Design & Implementation Plan: Validate terraphim_rlm with All Examples (End-to-End, No Mocks)

## 1. Summary of Target Behavior

After implementation, the terraphim_rlm crate will have comprehensive end-to-end validation of all examples found in:
- Rustdoc documentation examples (currently marked `ignore` -- will be converted to run with real infrastructure)
- Unit tests in each module (will be augmented with e2e tests)
- MCP tool examples (when feature enabled, using real MCP server)

The validation will:
1. Confirm all rustdoc examples compile and run correctly with **real** infrastructure (Firecracker/Docker VMs, LLM service, MCP server)
2. Verify all existing unit tests pass
3. Create explicit end-to-end validation tests for each documented example using live services
4. Produce a validation report showing pass/fail status for each example with real execution traces

## 2. Key Invariants and Acceptance Criteria

### Invariants
- All validation must use **real** infrastructure (no mocks, no stubs)
- Validation must work with `cargo test --package terraphim_rlm`
- Feature-gated code must be validated with appropriate feature flags
- Tests must clean up after themselves (no leftover VMs, snapshots, or sessions)
- Private repository access must use `gh` CLI with proper authentication

### Acceptance Criteria

| ID | Criterion | Testable? | Priority |
|----|-----------|-----------|----------|
| AC1 | All rustdoc examples compile and run with real Docker/Firecracker backend | Yes | High |
| AC2 | All rustdoc examples execute successfully with live LLM service | Yes | High |
| AC3 | All existing unit tests pass | Yes | High |
| AC4 | MCP tool examples validate correctly with real MCP server (when `mcp` feature enabled) | Yes | Medium |
| AC5 | Validation report generated with clear pass/fail status and execution traces | Yes | Medium |
| AC6 | Private repository (firecracker-rust) accessible via `gh` CLI | Yes | High |
| AC7 | Feature flag combinations tested (default, full, minimal) | Yes | Low |
| AC8 | Resource cleanup verified (no leftover VMs, snapshots) | Yes | High |

## 3. High-Level Design and Boundaries

### Validation Architecture (End-to-End)

```
Validation Layer 1: Doc Tests (Real Infrastructure)
  └── Extract rustdoc examples from source
  └── Convert to runnable doctests with real backend setup
  └── Run with `cargo test --doc`

Validation Layer 2: Unit Test Audit
  └── Inventory all #[test] and #[tokio::test] functions
  └── Categorise by module and feature gate
  └── Run with `cargo test --package terraphim_rlm`

Validation Layer 3: End-to-End Validation Tests (NEW)
  └── Create tests/e2e_validation.rs
  └── Each rustdoc example becomes an e2e test function
  └── Use REAL ExecutionEnvironment (Docker/Firecracker)
  └── Use REAL LLM bridge for query tests
  └── Use REAL MCP server for MCP tool tests

Validation Layer 4: Feature-Gated Validation
  └── Run with --features full
  └── Run with --features mcp
  └── Run with --no-default-features
```

### Boundaries

**Changes INSIDE terraphim_rlm crate:**
- `tests/e2e_validation.rs` - new file for end-to-end validation tests
- Potentially modify rustdoc examples to remove `ignore` (if they can run with real infrastructure)

**Changes OUTSIDE crate:**
- `docs_dev/validation-report-e2e.md` - output report (in project docs, not crate)
- Git configuration for private repo access via `gh` CLI

**NOT modifying:**
- Existing test code (only augmenting with e2e tests)
- Production source code (unless bugs found)
- Feature gate configurations

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `tests/e2e_validation.rs` | **CREATE** | N/A | New e2e validation tests | Uses real TerraphimRlm, Docker/Firecracker |
| `src/lib.rs` | **MODIFY** | Examples marked `ignore` | Some examples maybe `no_run` or runnable with real infra | None |
| `src/rlm.rs` | **MODIFY** | Examples marked `rust,ignore` | Convert to runnable or create test refs | None |
| `docs_dev/validation-report-e2e.md` | **CREATE** | N/A | Validation results report | None |
| `docs_dev/task_plan.md` | **MODIFY** | Phase 1-2 plan | Add Phase 3 (implementation) | None |

### Real Infrastructure Required

Since we are NOT using mocks, the validation requires:

```
1. Docker daemon (for Docker backend)
   OR
2. KVM + Firecracker (for Firecracker backend)
   
3. LLM service endpoint (OpenAI/Anthropic/local)
   
4. MCP server (for MCP tool validation)
   
5. gh CLI authenticated for terraphim/firecracker-rust
```

## 5. Step-by-Step Implementation Sequence

### Step 1: Audit Existing Examples
**Purpose:** Create complete inventory of all examples
**Deployable state?** N/A (research)
1. Grep all `.rs` files for ````rust` and `# Example` patterns
2. Extract each example with its context (module, feature gate)
3. Categorise: documentation example, unit test, integration test
4. Identify which examples need real infrastructure vs pure API
5. Save inventory to `docs_dev/findings.md` (UPDATE existing)

### Step 2: Set Up Real Infrastructure Access
**Purpose:** Enable testing with real dependencies
**Deployable state?** Yes (environment configured)
1. Verify `gh` CLI authentication: `gh auth status`
2. Configure git to use gh token for private repos:
   ```bash
   gh auth setup-git
   # OR
   git config --global url."https://$(gh auth token)@github.com/".insteadOf "https://github.com/"
   ```
3. Verify access to terraphim/firecracker-rust: `gh repo view terraphim/firecracker-rust`
4. Check Docker availability: `docker ps`
5. Check KVM availability: `ls /dev/kvm`
6. Configure LLM service endpoint (environment variable or config file)
7. Start MCP server if needed (or verify it's running)

### Step 3: Create End-to-End Validation Test File
**Purpose:** Explicit e2e tests for each documented example
**Deployable state?** Yes (tests run with real infrastructure)
1. Create `tests/e2e_validation.rs`
2. For each rustdoc example in `lib.rs`, `rlm.rs`, `executor/trait.rs`:
   - Create a test function that exercises the same code
   - Use REAL ExecutionEnvironment (Docker or Firecracker)
   - Assert expected behavior
3. For query() examples:
   - Use REAL LLM bridge with live service
   - Assert on termination reason or result structure
4. For MCP tool examples:
   - Connect to REAL MCP server
   - Execute tools and verify responses
5. Run `cargo test --package terraphim_rlm --test e2e_validation` to verify

### Step 4: Convert or Reference Documentation Examples
**Purpose:** Make rustdoc examples validateable with real infrastructure
**Deployable state?** Yes (doctests run)
**Feature flag needed?** No (uses real infrastructure)

Options (choose one based on infrastructure availability):
- **Option A:** Remove `ignore` from examples, add real backend setup in doctest
- **Option B:** Keep `ignore`, reference examples in e2e tests (already done in Step 3)
- **Option C:** Change `ignore` to `no_run` where examples compile but can't run without infrastructure

Recommendation: **Option A** for examples that can run with Docker (portable), **Option B** for Firecracker-specific examples.

### Step 5: Run Validation with Different Feature Flags
**Purpose:** Ensure all feature combinations work with real dependencies
**Deployable state?** Yes
1. `cargo test --package terraphim_rlm --no-default-features`
2. `cargo test --package terraphim_rlm` (default features)
3. `cargo test --package terraphim_rlm --features full`
4. `cargo test --package terraphim_rlm --features mcp` (if MCP server available)
5. `cargo test --package terraphim_rlm --features kg-validation` (if KG populated)

### Step 6: Generate Validation Report
**Purpose:** Document what was validated with real infrastructure
**Deployable state?** N/A (documentation)
1. Create `docs_dev/validation-report-e2e.md`
2. List all examples with: location, type, validation method, status, execution trace
3. Include feature-gated examples with appropriate flags
4. Document any examples that couldn't be validated (with reason)
5. Document infrastructure requirements for future validation runs

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| AC1: Doc examples compile and run | Doc test or e2e test | `tests/e2e_validation.rs` |
| AC2: Doc examples execute with LLM | E2E test with live service | `tests/e2e_validation.rs` |
| AC3: Unit tests pass | Existing `#[test]` functions | Each module's `mod tests` |
| AC4: MCP tools validate | E2E test with real MCP server | `tests/e2e_validation.rs` (behind `#[cfg(feature = "mcp")]`)
| AC5: Report generated | Manual verification | `docs_dev/validation-report-e2e.md` |
| AC6: Private repo access | CI check | `gh repo view terraphim/firecracker-rust` |
| AC7: Feature combos | Multiple test runs | CI matrix |
| AC8: Resource cleanup | Post-test verification | Cleanup assertions in e2e tests |

### Test Categories in `e2e_validation.rs`

```rust
// Category 1: Basic API examples (from lib.rs)
mod api_examples {
    // Test: Create TerraphimRlm with default config (real Docker backend)
    // Test: Create session, execute code, destroy session
}

// Category 2: RLM examples (from rlm.rs)
mod rlm_examples {
    // Test: Full query loop (with real LLM service)
    // Test: Snapshot create/restore (with real VM)
    // Test: Context variables
}

// Category 3: Executor trait examples (from executor/trait.rs)
mod executor_examples {
    // Test: execute_code, execute_command with real backend
}

// Category 4: MCP tools (from mcp_tools.rs, feature-gated)
#[cfg(feature = "mcp")]
mod mcp_examples {
    // Test: rlm_code, rlm_bash, rlm_query tools with real MCP server
}
```

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|-------------|----------------|
| **Firecracker not available on host** | Use Docker backend as fallback; document requirements | Low - Docker is widely available |
| **LLM service unavailable or rate-limited** | Use small models; add retries; cache responses | Medium - depends on service availability |
| **Private repo access fails** | Verify gh auth; use SSH keys; document setup | Low - gh CLI handles auth |
| **Examples have side effects (create snapshots)** | Clean up after tests; use temporary directories | Low - cleanup code in test teardown |
| **Budget exhaustion with real LLM** | Test with low budgets; monitor token usage | Medium - real LLM costs money |
| **MCP server not running** | Start MCP server in test setup; skip if unavailable | Low - optional feature |
| **Feature-gated code not compiled in CI** | Run multiple test commands with different flags | Low - process documented |

### Complexity Sources
1. **Real infrastructure dependencies** - mitigated by Docker fallback and clear setup docs
2. **Multiple feature flags** - mitigated by testing key combinations only
3. **Async everywhere** - mitigated by using `#[tokio::test]`
4. **Private repository access** - mitigated by gh CLI automation

## 8. Open Questions / Decisions for Human Review

1. **Which backend for primary validation?**
   - Docker (portable, no KVM needed)
   - Firecracker (full isolation, needs KVM)
   - Both: Docker for CI, Firecracker for bare metal

2. **Which LLM service for validation?**
   - OpenAI (gpt-3.5-turbo for cost efficiency)
   - Anthropic (Claude for quality)
   - Local model (no API costs, but setup complexity)

3. **How to handle MCP server?**
   - Start MCP server as part of test setup
   - Assume MCP server is already running
   - Skip MCP validation if server unavailable

4. **Should validation be part of `cargo test`?**
   - Integrate into existing test suite (slow but comprehensive)
   - Create separate `cargo test --test e2e_validation`
   - Both: quick unit tests + comprehensive e2e tests

5. **What to do with examples that need Firecracker specifically?**
   - Mark with `#[ignore]` for optional Firecracker testing
   - Document Firecracker requirements
   - Use Docker equivalent where possible

6. **Budget for LLM tokens during validation?**
   - Set a token limit per test
   - Use small models (gpt-3.5-turbo)
   - Cache LLM responses for deterministic tests

## Next Steps After Approval

1. Update `docs_dev/task_plan.md` with Phase 3 (Implementation)
2. Execute Step 1: Audit existing examples (update findings.md)
3. Execute Step 2: Set up real infrastructure access
4. Execute Step 3: Create e2e_validation.rs
5. Execute Step 4-6: Convert examples, test features, generate report
6. Run quality evaluation on this design document
