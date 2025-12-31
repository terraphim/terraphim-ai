# Research Document: Firecracker E2E Test Failures

## 1. Problem Restatement and Scope

### Problem Statement
The E2E tests for the GitHub runner Firecracker integration are failing due to SSH connectivity issues when executing commands inside VMs. The errors include:
- "No route to host" when connecting via SSH
- "Identity file not accessible: No such file or directory" for SSH keys
- Command execution timing out or returning exit code 255

### IN Scope
- Firecracker VM type configuration issues
- SSH key path mismatches between VM types
- Missing VM image files (rootfs, kernel)
- E2E test code in `terraphim_github_runner`
- fcctl-web API integration

### OUT of Scope
- fcctl-web server code changes (external project)
- Network bridge configuration (working correctly)
- JWT authentication (working correctly)
- Unit tests (49 tests passing)

## 2. User & Business Outcomes

### Expected Behavior
- E2E tests should create VMs, execute commands, and verify results
- Commands should execute in <200ms inside VMs
- GitHub webhook integration should work end-to-end

### Current Behavior
- Tests fail with SSH connection errors
- Commands return exit code 255 (SSH failure)
- Tests hang waiting for VM response

## 3. System Elements and Dependencies

### Component Map

| Component | Location | Role | Status |
|-----------|----------|------|--------|
| `end_to_end_test.rs` | `crates/terraphim_github_runner/tests/` | E2E test orchestration | Failing |
| `VmCommandExecutor` | `src/workflow/vm_executor.rs` | HTTP client to fcctl-web API | Working |
| `SessionManager` | `src/session/manager.rs` | VM session lifecycle | Working |
| `SessionManagerConfig` | `src/session/manager.rs:95-105` | Default VM type config | **BUG: defaults to focal-optimized** |
| fcctl-web API | External (port 8080) | Firecracker VM management | Working |
| fcctl-images.yaml | `/home/alex/projects/terraphim/firecracker-rust/` | VM type definitions | **Misconfigured** |

### Critical File Evidence

**Working VM Type (bionic-test)**:
```
./images/test-vms/bionic/bionic.rootfs     ✅ (838MB)
./firecracker-ci-artifacts/vmlinux-5.10.225 ✅ (38MB)
./images/test-vms/bionic/keypair/fctest     ✅ (SSH key)
```

**Broken VM Type (focal-optimized)**:
```
./images/ubuntu/focal/focal.rootfs          ❌ MISSING
./images/ubuntu/focal/vmlinux-5.10          ❌ MISSING
./images/ubuntu/focal/keypair/ubuntu        ❌ MISSING
```

### API Endpoints Used
- `GET /api/vms` - List VMs (working)
- `POST /api/vms` - Create VM (working but uses wrong default type)
- `POST /api/llm/execute` - Execute command (working for bionic-test, fails for focal-optimized)

## 4. Constraints and Their Implications

### Configuration Constraint
- **Constraint**: `SessionManagerConfig::default()` uses `focal-optimized` VM type
- **Impact**: All sessions created via the test use broken VM type
- **Solution**: Change default to `bionic-test` which has working images

### Infrastructure Constraint
- **Constraint**: fcctl-images.yaml defines multiple VM types with different file paths
- **Impact**: Only `bionic-test` has all required files present
- **Solution**: Either provision focal-optimized images OR use bionic-test

### Test Environment Constraint
- **Constraint**: E2E test is marked `#[ignore]` requiring `FIRECRACKER_AUTH_TOKEN` env var
- **Impact**: Test won't run in standard CI without explicit configuration
- **Solution**: Test infrastructure documentation needed

## 5. Risks, Unknowns, and Assumptions

### UNKNOWNS
1. Why does fcctl-images.yaml reference non-existent focal-optimized images?
2. Were the focal-optimized images ever provisioned?
3. Is focal-optimized meant to be used or is it legacy?

### ASSUMPTIONS
1. **ASSUMPTION**: bionic-test is production-ready (verified: commands execute correctly)
2. **ASSUMPTION**: fcctl-web API is stable and won't change (external dependency)
3. **ASSUMPTION**: Network bridge (fcbr0) configuration is correct (verified: bionic-test VMs route correctly)

### RISKS

| Risk | Impact | Mitigation |
|------|--------|------------|
| focal-optimized images may be needed later | Medium | Document why bionic-test is preferred |
| E2E tests depend on external fcctl-web service | High | Add health check before test execution |
| JWT token expiration during tests | Low | Already handled with fresh token generation |
| Stale VMs accumulate (150 VM limit) | Medium | Add cleanup step in test teardown |

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
1. **Multiple VM types**: 10+ VM types defined, only 2 working (bionic-test, focal-ci)
2. **External dependency**: fcctl-web is a separate project with its own configuration
3. **Historical artifacts**: focal-optimized config exists but images were never provisioned

### Simplification Strategies

1. **Single VM Type for Tests**:
   - Change `SessionManagerConfig::default()` to use `bionic-test`
   - Remove reference to focal-optimized from test code
   - **Effort**: Low (one line change)

2. **VM Type Validation**:
   - Add validation in test setup to verify VM type images exist
   - Fail fast with clear error if images missing
   - **Effort**: Medium (add validation logic)

3. **Test Cleanup**:
   - Add VM cleanup in test teardown to prevent stale VM accumulation
   - **Effort**: Low (add cleanup call)

## 7. Questions for Human Reviewer

1. **Should focal-optimized images be provisioned?** The images don't exist but the config references them. Is this intentional or oversight?

2. **Is bionic-test the preferred VM type for production?** It uses CI kernel (5.10.225) which is well-tested.

3. **Should the E2E test be added to CI pipeline?** Currently marked `#[ignore]` and requires local fcctl-web service.

4. **Should we add VM cleanup to prevent 150 VM limit issues?** Current tests don't clean up VMs after execution.

5. **Is the 10 second boot wait sufficient?** Test waits 10s but VMs boot in 0.2s. Could reduce wait time significantly.

---

## Verified Evidence

### bionic-test VM Execution (SUCCESS)
```json
{
  "vm_id": "vm-2aa3ec72",
  "exit_code": 0,
  "stdout": "fctest\n8c0bb792817a\nLinux 8c0bb792817a 5.10.225...",
  "duration_ms": 135
}
```

### focal-optimized VM Execution (FAILURE)
```json
{
  "vm_id": "vm-e2a5a1a7",
  "exit_code": 255,
  "stderr": "Warning: Identity file ./images/test-vms/focal/keypair/fctest not accessible...\nssh: connect to host 172.26.0.221 port 22: No route to host",
  "duration_ms": 3063
}
```

### Root Cause Summary
1. **Primary**: `SessionManagerConfig::default()` uses `focal-optimized` VM type which has missing images
2. **Secondary**: No validation that VM images exist before creating VMs
3. **Tertiary**: E2E test doesn't verify VM type compatibility
