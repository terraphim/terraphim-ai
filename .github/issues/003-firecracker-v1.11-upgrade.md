---
title: "MODERATE: Firecracker v1.11.0 API upgrade with snapshot breaking change"
labels: ["priority/P1", "type/breaking-change", "component/vm", "vendor/firecracker"]
assignees: []
milestone: ""
---

## Summary

**Echo reports moderate drift** in the Firecracker VM API integration. Latest release includes breaking snapshot format changes requiring regeneration.

## Current State

- **Integration:** `terraphim_firecracker` crate
- **Current API:** v1.10.x (estimated)
- **Upstream:** v1.11.0 (released 2026-03-18)
- **Drift:** 1 major version behind

## Breaking Changes

### 1. Snapshot Format v5.0.0 (BREAKING)

- **Change:** Removed fields from snapshot format
  - `max_connections` - removed
  - `max_pending_resets` - removed
- **Impact:** Snapshot version bumped to 5.0.0
- **Consequence:** Existing snapshots incompatible
- **Action Required:** Regenerate all snapshots

### 2. seccompiler Implementation

- **Change:** Migrated to `libseccomp`
- **Impact:** BPF code generation changed
- **Consequence:** Smaller, more optimized seccomp filters
- **Action Required:** Test VM creation with new seccompiler

## Non-Breaking Changes

### 3. ARM Physical Counter Reset

- **Change:** Reset `CNTPCT_EL0` on VM startup (kernel 6.4+)
- **Impact:** ARM guests no longer see host physical counter
- **Benefit:** Better isolation for ARM microVMs

### 4. AMD Genoa Support

- **Change:** Added as supported and tested platform
- **Impact:** Broader hardware compatibility

### 5. Swagger Definition Fix

- **Change:** `CpuConfig` definition includes aarch64-specific fields
- **Impact:** Better API documentation

### 6. IovDeque Page Size Fix

- **Change:** Works with any host page size
- **Impact:** virtio-net device works on non-4K kernels

### 7. PATCH /machine-config Relaxation

- **Change:** `mem_size_mib` and `track_dirty_pages` now optional
- **Impact:** Can omit fields in PATCH requests

### 8. Watchdog Fix

- **Change:** Fixed softlockup warning during GDB debugging
- **Impact:** Better debugging experience

### 9. Balloon Device UFFD

- **Change:** `remove` UFFD messages sent on balloon inflation
- **Impact:** Proper UFFD handling for memory ballooning

### 10. Jailer Integer Fix

- **Change:** Fixed integer underflow in `--parent-cpu-time-us`
- **Impact:** Development builds no longer crash

### 11. SIGHUP Fix

- **Change:** Fixed intermittent SIGHUP with `--new-pid-ns`
- **Impact:** More reliable jailer operation

### 12. AMD CPUID Fix

- **Change:** No longer overwrites CPUID leaf 0x80000000
- **Impact:** Guests can discover more CPUID leaves on AMD

### 13. KVM_CREATE_VM Reliability

- **Change:** Retry on EINTR
- **Impact:** Better reliability on heavily loaded hosts

### 14. Debug Build Seccomp

- **Change:** Empty seccomp policy for debug builds
- **Impact:** Avoids crashes from Rust 1.80.0 debug assertions

## Affected Crates

- [ ] `terraphim_firecracker` - Firecracker API client
- [ ] `terraphim_github_runner` - VM management for GitHub Actions

## Reproduction

```bash
# Check Firecracker version in use
firecracker --version

# Check snapshot compatibility
# (Will fail with v1.11 if using pre-v5 snapshots)
```

## Proposed Migration Plan

1. **Phase 1: API Client Update**
   - [ ] Create `feat/firecracker-v1.11-migration` branch
   - [ ] Review API client for snapshot v5.0 fields
   - [ ] Update snapshot creation code
   - [ ] Update snapshot loading code

2. **Phase 2: Snapshot Audit**
   - [ ] Inventory all existing snapshots
   - [ ] Document snapshot usage in CI/CD
   - [ ] Plan snapshot regeneration

3. **Phase 3: Testing**
   - [ ] Test VM creation with new seccompiler
   - [ ] Test ARM microVMs (if applicable)
   - [ ] Test AMD Genoa (if available)
   - [ ] Test memory ballooning
   - [ ] Test jailer with `--new-pid-ns`

4. **Phase 4: Snapshot Regeneration**
   - [ ] Regenerate all snapshots
   - [ ] Update CI/CD pipelines
   - [ ] Document new snapshot format

5. **Phase 5: Deployment**
   - [ ] Update production Firecracker binary
   - [ ] Deploy new snapshots
   - [ ] Monitor VM creation reliability

## References

- [Firecracker v1.11.0 Release](https://github.com/firecracker-microvm/firecracker/releases/tag/v1.11.0)
- [Firecracker CHANGELOG](https://github.com/firecracker-microvm/firecracker/blob/main/CHANGELOG.md)
- [Firecracker Snapshot Documentation](https://github.com/firecracker-microvm/firecracker/blob/main/docs/snapshotting.md)

## Dependencies

- Independent of other vendor upgrades
- Can be done in parallel with genai/rmcp work

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Snapshot regeneration fails | HIGH | LOW | Test in staging first |
| seccompiler issues | MEDIUM | LOW | Debug builds have empty policy |
| ARM counter issues | LOW | LOW | Only affects kernel 6.4+ |
| CI/CD disruption | MEDIUM | MEDIUM | Coordinate with team |

## Verification

```bash
# Test VM creation
cargo test -p terraphim_firecracker

# Test GitHub runner integration
cargo test -p terraphim_github_runner

# Verify snapshot version
# (Check snapshot metadata after creation)
```

## Rollback Plan

If issues occur:
1. Revert to Firecracker v1.10.x binary
2. Restore old snapshots from backup
3. Revert API client changes

---

**Echo's Assessment:** Snapshot format breaking change requires coordinated regeneration. VM abstraction layer drift moderate. Can proceed in parallel with other upgrades.
