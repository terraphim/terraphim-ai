# Implementation Plan: RLM Orchestration with MCP Tools (PR #426)

**Status**: Draft
**Research Doc**: `.docs/research-pr-426-rlm-orchestration.md`
**Author**: Alex Mikhalev
**Date**: 2026-03-11
**Estimated Effort**: 2-3 days

## Overview

### Summary
Complete PR #426 by fixing the missing `fcctl-core` dependency and integrating `terraphim_firecracker` pool management with `fcctl-core` VM lifecycle. This enables RLM to execute code in isolated Firecracker VMs with pre-warmed pools and snapshot support.

### Approach
1. Move `fcctl-core` from `scratchpad/` to workspace `crates/`
2. Create adapter to bridge `fcctl_core::vm::VmManager` (struct) to `terraphim_firecracker::vm::VmManager` (trait)
3. Implement `ensure_pool` in `FirecrackerExecutor`
4. Complete snapshot integration

### Scope
**In Scope:**
- Fix `fcctl-core` dependency path
- Move `fcctl-core` to workspace
- Create `FcctlVmManagerAdapter` in `terraphim_firecracker`
- Implement `ensure_pool` using adapter
- Complete snapshot management integration

**Out of Scope:**
- Network audit logging (Issue #667)
- OverlayFS support (Issue #668)
- LLM bridge endpoint (Issue #669)
- Output streaming (Issue #670)

**Avoid At All Cost:**
- Modifying `fcctl-core` internals unnecessarily
- Rewriting `terraphim_firecracker` to remove `VmManager` trait
- Complex async logic in adapter if not needed
- Duplicate VM management code

## Architecture

### Component Diagram
```
terraphim_rlm::FirecrackerExecutor
    ├── fcctl_core::vm::VmManager (struct)
    ├── fcctl_core::snapshot::SnapshotManager (struct)
    └── terraphim_firecracker::VmPoolManager
            └── terraphim_firecracker::vm::VmManager (trait)
                    └── FcctlVmManagerAdapter (new)
                            └── fcctl_core::vm::VmManager (struct)
```

### Data Flow
1. RLM receives code execution request
2. `FirecrackerExecutor.ensure_pool()` creates `FcctlVmManagerAdapter`
3. Adapter wraps `fcctl_core::vm::VmManager` and implements `terraphim_firecracker::vm::VmManager` trait
4. `VmPoolManager` uses adapter to manage pre-warmed VMs
5. `FirecrackerExecutor` executes code via SSH on VM from pool
6. Snapshots created/restored using `fcctl_core::snapshot::SnapshotManager`

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Create adapter in `terraphim_firecracker` | Keeps adapter close to trait definition | Putting adapter in `terraphim_rlm` would create circular dependency |
| Use interior mutability (`Mutex`) in adapter | `fcctl_core::vm::VmManager` uses `&mut self`, trait uses `&self` | Using `&mut self` in trait would break `VmPoolManager` interface |
| Move `fcctl-core` to workspace | Standard practice, enables proper dependency management | Keeping in `scratchpad/` prevents proper Cargo resolution |

### Eliminated Options (Essentialism)
| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Implement `VmManager` trait directly in `fcctl-core` | Would require forking `fcctl-core` | Maintenance burden, divergence from upstream |
| Remove `VmManager` trait from `terraphim_firecracker` | Would break pool management abstraction | Loss of clean separation of concerns |
| Implement all missing features at once | Violates 5/25 rule, scope creep | Delayed delivery, integration complexity |

### Simplicity Check
**What if this could be easy?**
The simplest design is an adapter that translates between the two `VmManager` interfaces. This requires:
1. Moving `fcctl-core` to workspace (standard Cargo operation)
2. Creating a wrapper struct that implements the trait using the struct
3. Wiring it up in `ensure_pool`

**Senior Engineer Test**: This is not overcomplicated - it's necessary to bridge two incompatible interfaces.

**Nothing Speculative Checklist**:
- [x] No features user didn't request (only fixes PR #426 blockers)
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `terraphim_firecracker/src/vm/fcctl_adapter.rs` | Adapter implementing `terraphim_firecracker::vm::VmManager` using `fcctl_core::vm::VmManager` |

### Modified Files
| File | Changes |
|------|---------|
| `Cargo.toml` | Add `crates/fcctl-core` to workspace members |
| `crates/terraphim_rlm/Cargo.toml` | Update `fcctl-core` path to `../../crates/fcctl-core` |
| `terraphim_firecracker/src/vm/mod.rs` | Add `mod fcctl_adapter;` and export adapter |
| `terraphim_firecracker/Cargo.toml` | Add `fcctl-core` dependency |
| `crates/terraphim_rlm/src/executor/firecracker.rs` | Implement `ensure_pool` using adapter |

### Deleted Files
| File | Reason |
|------|--------|
| None | No deletions required |

## API Design

### Public Types (Adapter)
```rust
// In terraphim_firecracker/src/vm/fcctl_adapter.rs
use std::sync::Arc;
use tokio::sync::Mutex;

/// Adapter that implements terraphim_firecracker::vm::VmManager
/// using fcctl_core::vm::VmManager
pub struct FcctlVmManagerAdapter {
    inner: Arc<Mutex<fcctl_core::vm::VmManager>>,
}

impl FcctlVmManagerAdapter {
    /// Create new adapter from fcctl_core VmManager
    pub fn new(vm_manager: fcctl_core::vm::VmManager) -> Self {
        Self {
            inner: Arc::new(Mutex::new(vm_manager)),
        }
    }
}
```

### Public Functions
```rust
// In terraphim_rlm/src/executor/firecracker.rs
impl FirecrackerExecutor {
    pub async fn ensure_pool(&self) -> Result<Arc<VmPoolManager>, RlmError> {
        // Get fcctl_core VmManager
        let mut vm_manager_guard = self.vm_manager.lock().await;
        let vm_manager = vm_manager_guard.take()
            .ok_or_else(|| RlmError::BackendInitFailed {
                backend: "firecracker".to_string(),
                message: "VmManager not initialized".to_string(),
            })?;

        // Create adapter
        let adapter = FcctlVmManagerAdapter::new(vm_manager);
        let adapter Arc = Arc::new(adapter);

        // Create VmPoolManager with adapter
        let pool_manager = VmPoolManager::new(
            adapter Arc,
            Arc::new(Sub2SecondOptimizer::new()),
            PoolConfig::default(),
        );

        // Initialize pools
        pool_manager.initialize_pools(vec!["terraphim-minimal".to_string()]).await?;

        *self.pool_manager.write() = Some(Arc::clone(&pool_manager));
        Ok(pool_manager)
    }
}
```

### Error Types
```rust
// Existing error types in terraphim_rlm::error
// No new error types required for adapter
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_adapter_create_vm` | `terraphim_firecracker/src/vm/fcctl_adapter.rs` | Verify adapter creates VM via fcctl-core |
| `test_adapter_start_vm` | `terraphim_firecracker/src/vm/fcctl_adapter.rs` | Verify adapter starts VM |
| `test_adapter_delete_vm` | `terraphim_firecracker/src/vm/fcctl_adapter.rs` | Verify adapter deletes VM |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_ensure_pool_integration` | `crates/terraphim_rlm/src/executor/firecracker.rs` | Verify pool initialization with adapter |
| `test_vm_allocation_from_pool` | `tests/rlm_integration.rs` | Verify VM allocation from pre-warmed pool |

### Property Tests
```rust
// Not required for this adapter-focused change
```

## Implementation Steps

### Step 1: Move fcctl-core to Workspace ✓
**Files:** `Cargo.toml`
**Description:** Add `crates/fcctl-core` to workspace members
**Tests:** `cargo check --workspace`
**Estimated:** 1 hour
**Status:** COMPLETED

**Commands:**
```bash
# Move fcctl-core to crates/
mv scratchpad/firecracker-rust/fcctl-core crates/fcctl-core

# Update workspace Cargo.toml
# Add "crates/fcctl-core" to members array

# Verify build
cargo check -p fcctl-core
```

### Step 2: Update terraphim_rlm Dependency ✓
**Files:** `crates/terraphim_rlm/Cargo.toml`
**Description:** Update `fcctl-core` path to `../../crates/fcctl-core`
**Tests:** `cargo check -p terraphim_rlm`
**Dependencies:** Step 1
**Estimated:** 30 minutes
**Status:** COMPLETED

**Changes:**
```toml
# Before
fcctl-core = { path = "../../../firecracker-rust/fcctl-core" }

# After
fcctl-core = { path = "../../crates/fcctl-core" }
```

### Step 3: Create FcctlVmManagerAdapter ✓
**Files:** `terraphim_firecracker/src/vm/fcctl_adapter.rs`
**Description:** Implement adapter bridging fcctl_core VmManager to terraphim_firecracker VmManager trait
**Tests:** Unit tests for adapter methods
**Dependencies:** Step 1, 2
**Estimated:** 3 hours
**Status:** COMPLETED

**Key Implementation:**
- Wrap `fcctl_core::vm::VmManager` in `Arc<Mutex<>>`
- Implement `terraphim_firecracker::vm::VmManager` trait methods
- Translate between different method signatures and types

### Step 4: Integrate Adapter in FirecrackerExecutor ✓
**Files:** `crates/terraphim_rlm/src/executor/firecracker.rs`
**Description:** Implement `ensure_pool` using adapter
**Tests:** Integration tests
**Dependencies:** Step 3
**Estimated:** 2 hours
**Status:** COMPLETED

**Changes:**
- Modify `ensure_pool` to use `FcctlVmManagerAdapter`
- Create `VmPoolManager` with adapter
- Initialize pools

### Step 5: Complete Snapshot Integration ✓
**Files:** `crates/terraphim_rlm/src/executor/firecracker.rs`
**Description:** Ensure snapshot methods work with fcctl-core SnapshotManager
**Tests:** Unit tests for snapshot operations
**Dependencies:** Step 2
**Estimated:** 2 hours
**Status:** COMPLETED

**Verification:**
- `create_snapshot` uses `SnapshotManager::create_snapshot` ✓
- `restore_snapshot` uses `SnapshotManager::restore_snapshot` ✓
- `list_snapshots` uses `SnapshotManager::list_snapshots` ✓
- `delete_snapshot` uses `SnapshotManager::delete_snapshot` ✓

### Step 6: Run Full Test Suite ✓
**Files:** All
**Description:** Verify all changes work together
**Tests:** `cargo test --workspace --exclude terraphim_agent`
**Dependencies:** All previous steps
**Estimated:** 1 hour
**Status:** COMPLETED

**Commands:**
```bash
cargo check --workspace  # ✓ PASSED
cargo test -p fcctl-core  # ✓ PASSED (1 warning)
cargo test -p terraphim_firecracker  # ✓ PASSED (1 warning)
cargo test -p terraphim_rlm --features full  # ✓ PASSED (108 tests passed)
```

## Rollback Plan

If issues discovered:
1. Revert `terraphim_rlm/Cargo.toml` to previous path
2. Remove adapter code from `terraphim_firecracker`
3. Restore `fcctl-core` to `scratchpad/`

Feature flag: `terraphim_rlm` can be excluded from workspace if needed

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| `fcctl-core` | Workspace path | Required for VM and snapshot management |

### Dependency Updates
| Crate | From | To | Reason |
|-------|------|-----|--------|
| `terraphim_firecracker` | No fcctl-core | Add fcctl-core | Adapter requirement |

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| VM Allocation | < 500ms | Benchmark with pre-warmed pool |
| Snapshot Create | < 1s | Measure with fcctl-core |
| Snapshot Restore | < 1s | Measure with fcctl-core |

### Benchmarks to Add
```rust
// Future work: Add benchmarks for pool allocation
#[bench]
fn bench_vm_allocation(b: &mut Bencher) {
    // Benchmark VM allocation from pre-warmed pool
}
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Review fcctl-core code for completeness | Pending | Alex Mikhalev |
| Test on system with KVM | Pending | Alex Mikhalev |
| Document KVM requirements | Pending | Alex Mikhalev |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
