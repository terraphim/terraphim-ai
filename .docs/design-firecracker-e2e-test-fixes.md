# Design & Implementation Plan: Firecracker E2E Test Fixes

## 1. Summary of Target Behavior

After implementation:
- E2E tests execute successfully using `bionic-test` VM type (verified working)
- Tests create VMs, execute commands, and verify results
- Commands execute in <200ms inside VMs
- VMs are cleaned up after test execution to prevent stale VM accumulation
- Test failures provide clear error messages indicating root cause

## 2. Key Invariants and Acceptance Criteria

### Invariants
| ID | Invariant | Verification |
|----|-----------|--------------|
| INV-1 | Default VM type must have valid images | Test startup validates VM type |
| INV-2 | VM commands execute within timeout | 5-second timeout per command |
| INV-3 | Test cleanup prevents VM accumulation | Cleanup runs in teardown |

### Acceptance Criteria
| ID | Criterion | Testable |
|----|-----------|----------|
| AC-1 | E2E test passes with bionic-test VM type | Run test with `--ignored` flag |
| AC-2 | All 3 test commands execute with exit_code=0 | Assert exit codes in test |
| AC-3 | LearningCoordinator records >= 3 successes | Assert stats after execution |
| AC-4 | Test VM is deleted after test completion | Verify VM count after test |
| AC-5 | Boot wait reduced from 10s to 3s (VM boots in 0.2s) | Test timing assertion |

## 3. High-Level Design and Boundaries

### Components Affected

```
┌─────────────────────────────────────────────────────────────┐
│                    E2E Test Flow                            │
├─────────────────────────────────────────────────────────────┤
│  1. Test Setup                                              │
│     └─> Validate fcctl-web health                           │
│     └─> Create VM with bionic-test type ← CHANGE           │
│     └─> Wait 3s for boot ← CHANGE (was 10s)                │
│                                                             │
│  2. Test Execution                                          │
│     └─> Execute commands via VmCommandExecutor              │
│     └─> Record results in LearningCoordinator               │
│                                                             │
│  3. Test Teardown ← NEW                                     │
│     └─> Delete test VM                                      │
│     └─> Verify cleanup                                      │
└─────────────────────────────────────────────────────────────┘
```

### Boundaries
- **Changes inside** `terraphim_github_runner` crate only
- **No changes** to fcctl-web (external)
- **No changes** to VmCommandExecutor (working correctly)
- **Minimal changes** to SessionManagerConfig default

## 4. File/Module-Level Change Plan

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `src/session/manager.rs:98` | Modify | `default_vm_type: "focal-optimized"` | `default_vm_type: "bionic-test"` | None |
| `tests/end_to_end_test.rs:137,162` | Modify | `sleep(10)` wait | `sleep(3)` wait | None |
| `tests/end_to_end_test.rs:~365` | Add | No cleanup | Add VM deletion in teardown | reqwest client |

### Detailed Changes

**File 1: `src/session/manager.rs`**
- Line 98: Change default VM type string
- Responsibility: Provide working default for all session consumers
- Side-effects: Any code using `SessionManagerConfig::default()` gets correct VM type

**File 2: `tests/end_to_end_test.rs`**
- Lines 137, 162: Reduce boot wait from 10s to 3s
- After line 362: Add cleanup section to delete test VM
- Responsibility: Test now self-cleans after execution

## 5. Step-by-Step Implementation Sequence

### Step 1: Change Default VM Type
**Purpose**: Fix root cause - incorrect default VM type
**File**: `src/session/manager.rs`
**Change**: Line 98: `"focal-optimized"` → `"bionic-test"`
**Deployable**: Yes (backwards compatible - just changes default)
**Feature flag**: No

### Step 2: Reduce Boot Wait Time
**Purpose**: Optimize test speed (VMs boot in 0.2s, not 10s)
**File**: `tests/end_to_end_test.rs`
**Change**: Lines 137, 162: `Duration::from_secs(10)` → `Duration::from_secs(3)`
**Deployable**: Yes (test-only change)
**Feature flag**: No

### Step 3: Add Test Cleanup
**Purpose**: Prevent stale VM accumulation (150 VM limit)
**File**: `tests/end_to_end_test.rs`
**Change**: Add cleanup block after assertions to delete test VM
**Deployable**: Yes (test-only change)
**Feature flag**: No

### Step 4: Run and Verify E2E Test
**Purpose**: Validate all changes work together
**Command**: `cargo test -p terraphim_github_runner end_to_end_real_firecracker_vm -- --ignored --nocapture`
**Expected**: All 3 commands execute successfully, cleanup completes

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Verification Method |
|---------------------|-----------|---------------------|
| AC-1: E2E passes | E2E | Run `end_to_end_real_firecracker_vm` test |
| AC-2: Commands succeed | E2E | Assert `all_success == true`, `executed_count == 3` |
| AC-3: Learning records | E2E | Assert `learning_stats.total_successes >= 3` |
| AC-4: VM cleanup | E2E | Query `/api/vms` after test, verify test VM deleted |
| AC-5: Fast boot wait | E2E | Test completes in <30s total (was ~60s) |

### Test Execution Plan
```bash
# 1. Ensure fcctl-web is running
curl http://127.0.0.1:8080/health

# 2. Set auth token
export FIRECRACKER_AUTH_TOKEN="<valid_jwt>"

# 3. Run E2E test
cargo test -p terraphim_github_runner end_to_end_real_firecracker_vm -- --ignored --nocapture

# 4. Verify no leaked VMs (optional manual check)
curl -H "Authorization: Bearer $JWT" http://127.0.0.1:8080/api/vms | jq '.vms | length'
```

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| focal-optimized needed later | Document in CLAUDE.md that bionic-test is preferred | Low - can add focal images if needed |
| fcctl-web unavailable | Test already checks health, fails fast | Low - expected for ignored test |
| JWT expiration | Test uses env var, user controls token | Low - standard practice |
| VM cleanup fails | Add error handling, log warning but don't fail test | Low - minor resource leak |
| 3s boot wait insufficient | bionic-test boots in 0.2s, 3s is 15x margin | Very Low |

## 8. Open Questions / Decisions for Human Review

1. **Cleanup on failure**: Should we clean up VM even if test assertions fail?
   - **Recommendation**: Yes, use `defer`-style cleanup pattern

2. **Stale VM batch cleanup**: Should we add a cleanup of ALL user VMs at test start?
   - **Recommendation**: No, could interfere with other running tests

3. **Documentation update**: Should we update `END_TO_END_PROOF.md` with new test instructions?
   - **Recommendation**: Yes, after implementation verified

---

## Implementation Checklist

- [ ] Step 1: Change `SessionManagerConfig::default()` VM type to `bionic-test`
- [ ] Step 2: Reduce boot wait from 10s to 3s in test
- [ ] Step 3: Add VM cleanup in test teardown
- [ ] Step 4: Run E2E test and verify all criteria pass
- [ ] Step 5: Commit changes with clear message

---

**Do you approve this plan as-is, or would you like to adjust any part?**
