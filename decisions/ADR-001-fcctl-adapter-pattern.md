# ADR: fcctl-core Adapter Pattern for terraphim_rlm

## Status

**Accepted** - 2026-03-19

## Context

The Resource Lifecycle Manager (RLM) in terraphim-ai requires Firecracker microVM support for secure code execution. The fcctl-core crate provides VM management capabilities, but its API differs from terraphim_firecracker's traits and types.

**Problem**: Direct integration would couple RLM to fcctl-core's internal types, making future migrations difficult and complicating testing.

**Current Pain Point**: FirecrackerExecutor previously had no abstraction layer, making it impossible to swap VM backends or mock for tests.

## Decision

We will implement an adapter pattern (`FcctlVmManagerAdapter`) that:

1. Wraps fcctl-core's VmManager
2. Implements terraphim_firecracker's VmManager trait
3. Translates between domain-specific types (VmRequirements) and fcctl-core types (VmConfig)
4. Enforces ULID-based VM IDs at the boundary
5. Preserves error chains with #[source] annotation

## Consequences

### Benefits

- **Decoupling**: RLM depends on traits, not concrete fcctl-core types
- **Testability**: Can mock VmManager trait for unit tests
- **Migration Path**: Can swap fcctl-core for alternative implementations
- **Type Safety**: VmRequirements enforces valid configurations at compile time
- **Consistency**: ULID enforcement ensures uniform ID format across ecosystem

### Tradeoffs

- **Complexity**: Additional abstraction layer (424 lines)
- **Overhead**: ~0.3ms per config translation (acceptable for VM allocation)
- **Maintenance**: Must update adapter when fcctl-core API changes

## Implementation

### Adapter Structure

```rust
pub struct FcctlVmManagerAdapter {
    inner: Arc<Mutex<FcctlVmManager>>,
    firecracker_bin: PathBuf,
    socket_base_path: PathBuf,
    kernel_path: PathBuf,
    rootfs_path: PathBuf,
}
```

### Translation Layer

- `VmRequirements` -> `FcctlVmConfig` (domain to fcctl-core)
- `FcctlVmState` -> `Vm` (fcctl-core to domain)
- `fcctl_core::Error` -> `FcctlAdapterError` (with source preservation)

### Locking Strategy

- `tokio::sync::Mutex` for VmManager (held across await points)
- `tokio::sync::RwLock` for pool and session state
- Never use parking_lot in async code (deadlock risk)

## Alternatives Considered

### 1. Direct Integration

**Rejected**: Would tightly couple RLM to fcctl-core types.

### 2. Generic Traits Only

**Rejected**: Would require changes to fcctl-core which is external.

### 3. Full Abstraction Layer

**Rejected**: Overkill for current needs; adapter pattern provides right balance.

## Related Decisions

- [PR #426](https://github.com/terraphim/terraphim-ai/pull/426) - Implementation
- [terraphim-ai/DEPLOYMENT_SUMMARY_PR426.md](../terraphim-ai/DEPLOYMENT_SUMMARY_PR426.md) - Deployment details

## References

- Adapter Pattern (GoF Design Patterns)
- [fcctl_adapter.rs](../terraphim-ai/crates/terraphim_rlm/src/executor/fcctl_adapter.rs)
