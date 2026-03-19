# Design & Implementation Plan: fcctl-core to terraphim_firecracker Adapter

## 1. Summary of Target Behavior

After implementation, the terraphim_rlm crate will be able to:

- Use fcctl-core's concrete `VmManager` through terraphim_firecracker's `VmManager` trait
- Maintain sub-500ms VM allocation via the existing pool architecture
- Execute actual Python/bash code in Firecracker VMs (not stub responses)
- Preserve all pool features: pre-warming, VM reuse, background maintenance
- Handle errors from both systems with full context preservation

The adapter acts as a transparent bridge: fcctl-core owns VM lifecycle, the pool owns scheduling, and the adapter translates between them without adding overhead.

## 2. Key Invariants and Acceptance Criteria

### Functional Invariants
- [ ] Adapter implements `terraphim_firecracker::vm::VmManager` trait fully
- [ ] All trait methods delegate to fcctl-core's VmManager
- [ ] VM lifecycle operations (create/start/stop/delete) work end-to-end
- [ ] Error propagation preserves fcctl-core error details
- [ ] Configuration translation (VmRequirements -> VmConfig) is lossless

### Performance Invariants
- [ ] Adapter adds < 1ms overhead per operation
- [ ] Sub-500ms allocation guarantee maintained
- [ ] No blocking operations in async path
- [ ] VM pool pre-warming works correctly

### Compatibility Invariants
- [ ] Works with existing terraphim_rlm code without modification
- [ ] Compatible with tokio async runtime
- [ ] Send + Sync safe for Arc sharing
- [ ] Error types convert appropriately

### Testing Invariants
- [ ] Unit tests for adapter methods
- [ ] Integration test verifying VM creation through adapter
- [ ] Performance benchmark showing < 1ms overhead
- [ ] Error handling tests for both success and failure paths

## 3. High-Level Design and Boundaries

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        terraphim_rlm                                 │
├─────────────────────────────────────────────────────────────────────┤
│  FirecrackerExecutor                                                 │
│  ├─ Uses VmPoolManager (existing)                                   │
│  └─ Configured with adapter-based VmManager                         │
├─────────────────────────────────────────────────────────────────────┤
│  terraphim_firecracker                                               │
│  ├─ VmPoolManager (unchanged)                                       │
│  └─ VmManager trait (unchanged)                                     │
├─────────────────────────────────────────────────────────────────────┤
│  ADAPTER (NEW) - FcctlVmManagerAdapter                              │
│  ├─ Implements VmManager trait                                      │
│  ├─ Wraps fcctl_core::vm::VmManager                                 │
│  ├─ Translates: VmRequirements -> VmConfig                          │
│  ├─ Translates: fcctl_core::Error -> terraphim_firecracker::Error   │
│  └─ Delegates all operations to inner VmManager                     │
├─────────────────────────────────────────────────────────────────────┤
│  fcctl-core                                                          │
│  └─ VmManager (concrete struct) - owns VM lifecycle                 │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Boundaries

| Component | Responsibility | Boundary |
|-----------|---------------|----------|
| FcctlVmManagerAdapter | Bridge trait/struct mismatch | Translates types, delegates calls |
| fcctl-core VmManager | VM lifecycle management | Creates/starts/stops VMs |
| VmPoolManager | Pool scheduling and warming | Uses adapter as trait object |
| FirecrackerExecutor | RLM integration | Unchanged, uses pool via adapter |

### Data Flow

```
User Request
    ↓
FirecrackerExecutor
    ↓
VmPoolManager (gets VM from pool or creates new)
    ↓
FcctlVmManagerAdapter (trait implementation)
    ↓ [Translates VmRequirements → VmConfig]
fcctl-core VmManager (creates actual VM)
    ↓
Firecracker VM (executes code)
```

## 4. File/Module-Level Change Plan

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `src/executor/fcctl_adapter.rs` | Create | - | Adapter implementation | fcctl-core, terraphim_firecracker |
| `src/executor/mod.rs` | Modify | Only declares firecracker module | Also declares fcctl_adapter module | fcctl_adapter |
| `src/executor/firecracker.rs` | Modify | TODO stub at line 211 | Uses adapter to create VmPoolManager | fcctl_adapter |
| `Cargo.toml` | Modify | fcctl-core dependency | Ensure all features available | - |

### Detailed Changes

**1. Create `src/executor/fcctl_adapter.rs`**
- Define `FcctlVmManagerAdapter` struct wrapping `fcctl_core::vm::VmManager`
- Implement `terraphim_firecracker::vm::VmManager` trait
- Implement error conversion
- Implement configuration translation
- Add unit tests

**2. Modify `src/executor/mod.rs`**
- Add `pub mod fcctl_adapter;`
- Update `create_executor()` to use adapter

**3. Modify `src/executor/firecracker.rs`**
- Replace TODO stub with actual VmPoolManager creation
- Instantiate adapter with fcctl-core VmManager
- Pass adapter to VmPoolManager::new()

## 5. Step-by-Step Implementation Sequence

### Phase 1: Adapter Structure (Deployable)

1. **Create adapter.rs with struct definition**
   - Define `FcctlVmManagerAdapter` struct
   - Add fields: inner: fcctl_core::vm::VmManager
   - Implement `new()` constructor
   - State: Compiles, no trait implementation yet

2. **Implement trait scaffolding**
   - Add `#[async_trait]` impl block
   - Stub all required methods (return errors)
   - State: Compiles, trait methods stubbed

3. **Implement configuration translation**
   - Create `vm_requirements_to_config()` function
   - Map VmRequirements fields to VmConfig
   - State: Config translation works

### Phase 2: Method Implementation (Deployable)

4. **Implement create_vm()**
   - Translate requirements to config
   - Call inner.create_vm()
   - Convert error types
   - State: VM creation works

5. **Implement start_vm()**
   - Delegate to inner.start_vm()
   - Convert error types
   - State: VM start works

6. **Implement stop_vm()**
   - Delegate to inner.stop_vm()
   - Convert error types
   - State: VM stop works

7. **Implement delete_vm()**
   - Delegate to inner.delete_vm()
   - Convert error types
   - State: VM deletion works

8. **Implement remaining trait methods**
   - list_vms(), get_vm_status(), etc.
   - State: All trait methods implemented

### Phase 3: Integration (Deployable)

9. **Update executor/mod.rs**
   - Add fcctl_adapter module declaration
   - State: Module accessible

10. **Replace TODO stub in firecracker.rs**
    - Create fcctl-core VmManager
    - Wrap in adapter
    - Create VmPoolManager with adapter
    - State: Full integration complete

11. **Verify compilation**
    - cargo check --all-targets
    - State: No errors

### Phase 4: Testing (Deployable)

12. **Write unit tests for adapter**
    - Test configuration translation
    - Test error conversion
    - Mock inner VmManager for testing
    - State: Unit tests pass

13. **Write integration test**
    - Create VM through adapter
    - Verify full flow works
    - State: Integration test passes

14. **Performance benchmark**
    - Measure adapter overhead
    - Verify < 1ms target
    - State: Performance acceptable

### Phase 5: Verification

15. **Run full test suite**
    - cargo test --all-targets
    - State: All tests pass

16. **End-to-end test with Firecracker**
    - Execute Python code in VM
    - Verify sub-500ms allocation
    - State: Production ready

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| Adapter implements trait | Unit | `fcctl_adapter.rs` - test_trait_implementation |
| Configuration translation works | Unit | `fcctl_adapter.rs` - test_config_translation |
| Error conversion preserves info | Unit | `fcctl_adapter.rs` - test_error_conversion |
| create_vm delegates correctly | Unit | `fcctl_adapter.rs` - test_create_vm_delegation |
| VM lifecycle works end-to-end | Integration | `tests/adapter_integration.rs` - test_vm_lifecycle |
| Adapter overhead < 1ms | Benchmark | `benches/adapter_overhead.rs` |
| Sub-500ms allocation maintained | E2E | `tests/e2e_firecracker.rs` - test_allocation_latency |
| Pool pre-warming works | Integration | `tests/adapter_integration.rs` - test_pool_warming |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Trait method mismatch | Verify signatures before implementation (Phase 1) | Low - addressed in research |
| Performance overhead | Benchmark in Phase 4, target < 1ms | Low - thin adapter pattern |
| Error information loss | Comprehensive error mapping with source preservation | Low - test error conversion |
| State inconsistency | Adapter is stateless, delegates to fcctl-core | Low - no state duplication |
| Configuration mismatch | Map fields explicitly, document any gaps | Medium - may need VmConfig extension |
| Async compatibility | Use async-trait, test with tokio | Low - standard patterns |
| Compilation failures | Incremental implementation, check after each step | Low - step-by-step approach |

## 8. Open Questions / Decisions for Human Review

1. **VM ID Format**: fcctl-core may use string VM IDs. Should we enforce ULID format in the adapter, or pass through as-is?

2. **Configuration Mapping**: VmRequirements may have fields not present in VmConfig. Should we:
   - A) Extend fcctl-core's VmConfig
   - B) Store extra fields separately
   - C) Only support common subset

3. **Error Strategy**: Should we:
   - A) Create unified error type wrapping both
   - B) Map fcctl-core errors to closest trait error variant
   - C) Preserve original fcctl-core errors as source

4. **Metrics/Logging**: Should the adapter add its own metrics/logging, or purely delegate to inner VmManager?

5. **Pool Configuration**: What PoolConfig values should we use (pool size, min/max VMs, etc.)?

6. **Testing Approach**: Should we mock fcctl-core in unit tests, or require actual Firecracker for adapter tests?

7. **Documentation**: Should we add architecture documentation explaining the adapter pattern for future maintainers?
