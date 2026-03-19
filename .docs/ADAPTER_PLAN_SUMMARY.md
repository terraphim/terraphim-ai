# fcctl-core Adapter Implementation Plan Summary

## Status: READY FOR IMPLEMENTATION

Both Phase 1 (Research) and Phase 2 (Design) have passed quality evaluation.

---

## Documents Created

| Document | Type | Quality Score | Status |
|----------|------|---------------|--------|
| research-fcctl-adapter.md | Phase 1 Research | 4.3/5.0 | ✅ APPROVED |
| design-fcctl-adapter.md | Phase 2 Design | 4.6/5.0 | ✅ APPROVED |
| quality-evaluation-fcctl-research.md | Quality Gate | N/A | ✅ PASSED |
| quality-evaluation-fcctl-design.md | Quality Gate | N/A | ✅ PASSED |

---

## Problem Summary

Bridge fcctl-core's concrete `VmManager` struct with terraphim_firecracker's `VmManager` trait to enable full VM pool functionality.

**Type Mismatch:**
- fcctl-core provides: Concrete `VmManager` struct
- terraphim_firecracker expects: `Arc<dyn VmManager>` trait object

**Solution:** Adapter pattern - thin wrapper implementing the trait using fcctl-core's struct

---

## Key Design Decisions

### Architecture
```
FirecrackerExecutor -> VmPoolManager -> FcctlVmManagerAdapter -> fcctl-core VmManager -> Firecracker VM
```

### Value of Pool Architecture (Preserved)
- Pre-warmed VMs (20-30x faster burst handling)
- Sub-500ms allocation guarantee
- VM reuse without boot overhead
- Background maintenance

### Implementation Plan

**Phase 1: Adapter Structure** (3 steps)
- Create adapter.rs with struct definition
- Implement trait scaffolding
- Configuration translation

**Phase 2: Method Implementation** (5 steps)
- Implement create_vm(), start_vm(), stop_vm(), delete_vm()
- Implement remaining trait methods

**Phase 3: Integration** (3 steps)
- Update executor/mod.rs
- Replace TODO stub in firecracker.rs
- Verify compilation

**Phase 4: Testing** (3 steps)
- Unit tests for adapter
- Integration test
- Performance benchmark

**Phase 5: Verification** (2 steps)
- Full test suite
- End-to-end test with Firecracker

**Total: 16 steps across 5 phases**

---

## Critical Invariants

- ✅ Adapter implements VmManager trait fully
- ✅ All operations delegate to fcctl-core
- ✅ Error propagation preserves context
- ✅ Configuration translation is lossless
- ✅ Adapter overhead < 1ms per operation
- ✅ Sub-500ms allocation guarantee maintained

---

## Open Questions for You

1. **VM ID Format**: fcctl-core uses string IDs. Enforce ULID or pass through?

2. **Configuration Mapping**: VmRequirements may have extra fields. Options:
   - A) Extend fcctl-core's VmConfig
   - B) Store extra fields separately
   - C) Only support common subset

3. **Error Strategy**: 
   - A) Create unified error type
   - B) Map to closest trait error variant
   - C) Preserve fcctl-core errors as source

4. **Pool Configuration**: What PoolConfig values? (pool size, min/max VMs)

---

## Files to Create/Modify

| File | Action | Lines |
|------|--------|-------|
| `src/executor/fcctl_adapter.rs` | Create | ~300 |
| `src/executor/mod.rs` | Modify | +5 |
| `src/executor/firecracker.rs` | Modify | Replace TODO |

---

## Next Step: Implementation

Ready to proceed with Phase 3 (Implementation) on bigbox.

**Estimated time**: 4-6 hours for all 16 steps

Shall I proceed with implementation?