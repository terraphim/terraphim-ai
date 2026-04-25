# Re-plan: terraphim_rlm Full Configuration

## Status Update (Post-Merge)

**Completed:**
- Pulled latest changes from GitHub main (7 new commits)
- Merged upstream changes with our validation work
- Committed all validation work (54e12fac)
- All 129 tests pass (108 unit + 13 e2e + 7 doc + 1 ignored)

**Current Limitations:**
1. VM execution returns stub responses (no real VM allocation)
2. Firecracker binary at `/usr/local/bin/firecracker` (v1.10.1)
3. No VM kernel/rootfs images configured
4. VM pool initialization not implemented

## New Findings from Merged Code

The upstream changes include:
- CI-firecracker workflow improvements
- VM-cargo-probe job for testing in Firecracker VMs
- fcctl-bridge service for VM communication
- rust-build job with sccache + SeaweedFS

This suggests the project already has infrastructure for:
1. Building Rust code inside Firecracker VMs
2. Running tests in isolated VM environments
3. Caching build artifacts across VM runs

## Revised Plan

### Phase 1: Leverage Existing Infrastructure (Priority: High)

Instead of manually configuring VMs, we should leverage the existing CI infrastructure:

1. **Study the CI workflow** (`.github/workflows/ci-firecracker.yml`)
   - Understand how VMs are created in CI
   - Identify VM image locations and configurations
   - Learn how tests are executed inside VMs

2. **Use fcctl-bridge service**
   - Check if fcctl-bridge is running on bigbox
   - Use it for VM management instead of direct Firecracker API
   - This provides a higher-level interface

3. **Study VM image build process**
   - `infrastructure/firecracker-rust-ci/` contains VM build scripts
   - `build.sh` and `chroot.sh` create VM images
   - These images include Rust toolchain and sccache

### Phase 2: Configure Local VM Pool (Priority: High)

1. **Build or download VM image**
   - Use existing build scripts to create a VM image
   - Or download pre-built image if available
   - Image needs: kernel (vmlinux), rootfs (ext4)

2. **Configure VM paths in FirecrackerExecutor**
   - Update kernel_path and rootfs_path to actual images
   - Ensure images are accessible to the test runner

3. **Initialize VM pool**
   - Implement pool initialization in `ensure_pool()`
   - Pre-warm VMs for fast allocation
   - Configure pool size (min=1, max=2 for testing)

### Phase 3: Real VM Execution (Priority: High)

1. **Verify VM creation works**
   - Test `get_or_allocate_vm()` creates real VMs
   - Verify VMs boot successfully
   - Check SSH connectivity to VMs

2. **Test code execution in VMs**
   - Execute Python code in real VM
   - Execute bash commands in real VM
   - Verify output is from actual execution

3. **Test snapshots with real VMs**
   - Create snapshot of running VM
   - Restore snapshot
   - Verify state is preserved

### Phase 4: Integration with E2E Tests (Priority: Medium)

1. **Update e2e tests to verify real execution**
   - Assert that output contains actual command results
   - Test that Python code produces real output
   - Verify snapshots work end-to-end

2. **Add VM lifecycle tests**
   - Test VM creation and destruction
   - Test VM pool scaling
   - Test VM health checks

### Phase 5: Documentation and Reporting (Priority: Medium)

1. **Document VM configuration steps**
   - How to build VM images
   - How to configure FirecrackerExecutor
   - How to run tests with real VMs

2. **Update validation report**
   - Mark VM execution as working
   - Document any limitations
   - Provide troubleshooting guide

## Immediate Next Steps

1. **Check fcctl-bridge status**
   ```bash
   systemctl status fcctl-bridge
   # or
   ps aux | grep fcctl-bridge
   ```

2. **Study CI workflow**
   ```bash
   cat .github/workflows/ci-firecracker.yml
   ```

3. **Check for pre-built VM images**
   ```bash
   ls /var/lib/fcctl/ 2>/dev/null || echo "No fcctl data"
   find / -name "*.ext4" -o -name "vmlinux*" 2>/dev/null | grep -v firecracker/build
   ```

4. **Review VM build scripts**
   ```bash
   cat infrastructure/firecracker-rust-ci/build.sh
   cat infrastructure/firecracker-rust-ci/chroot.sh
   ```

## Decision Points

### Option A: Use fcctl-bridge (Recommended)
- Pros: Higher-level API, already running, handles VM lifecycle
- Cons: Requires understanding bridge API, may need configuration

### Option B: Direct Firecracker API
- Pros: Full control, no dependencies on services
- Cons: Complex VM setup, networking, image management

### Option C: Use CI VM images
- Pros: Pre-configured, tested in CI
- Cons: May need adaptation for local use

## Recommendation

**Proceed with Option A (fcctl-bridge)** because:
1. It's already part of the project infrastructure
2. The CI workflow uses it successfully
3. It abstracts VM complexity
4. It's designed for this exact use case

**Steps:**
1. Verify fcctl-bridge is running
2. Study its API and configuration
3. Configure FirecrackerExecutor to use bridge
4. Test VM creation through bridge
5. Update e2e tests for real execution

## Files to Review

- `.github/workflows/ci-firecracker.yml` - CI workflow
- `infrastructure/firecracker-rust-ci/build.sh` - VM build script
- `infrastructure/firecracker-rust-ci/chroot.sh` - VM chroot setup
- `crates/terraphim_rlm/src/executor/firecracker.rs` - Our changes
- `crates/terraphim_rlm/tests/e2e_validation.rs` - E2E tests
