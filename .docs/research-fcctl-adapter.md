# Research Document: fcctl-core to terraphim_firecracker Adapter

## 1. Problem Restatement and Scope

We need to bridge fcctl-core's concrete `VmManager` struct with terraphim_firecracker's `VmManager` trait to enable full VM pool functionality in terraphim_rlm. Currently, there's a TODO stub preventing actual VM execution.

**IN Scope:**
- Design and implement an adapter pattern to bridge the type mismatch
- Enable terraphim_rlm to use fcctl-core's VmManager through terraphim_firecracker's pool
- Preserve all pool features (pre-warming, sub-500ms allocation, VM reuse)
- Ensure async compatibility between the two systems

**OUT of Scope:**
- Modifying fcctl-core's API
- Modifying terraphim_firecracker's trait definition
- Adding new pooling features to either crate
- Production deployment configuration

## 2. User & Business Outcomes

**User Outcomes:**
- Sub-500ms VM allocation for responsive AI agent execution
- Pre-warmed VMs ready before requests arrive
- Efficient VM reuse without boot overhead
- Reliable pool health with background maintenance

**Business Outcomes:**
- Production-ready RLM orchestration with VM pooling
- Latency guarantees for AI workloads
- Resource efficiency through VM reuse
- Foundation for scalable multi-tenant AI execution

## 3. System Elements and Dependencies

| Component | Location | Role | Dependencies |
|-----------|----------|------|--------------|
| fcctl-core VmManager | `firecracker-rust/fcctl-core/src/vm/manager.rs` | Concrete VM lifecycle manager | Firecracker binary, KVM |
| terraphim_firecracker VmManager trait | `terraphim-ai/crates/terraphim_firecracker/src/vm/mod.rs` | Trait for pool compatibility | async-trait |
| VmPoolManager | `terraphim-ai/crates/terraphim_firecracker/src/pool.rs` | Pool management with pre-warming | VmManager trait, Sub2SecondOptimizer |
| FirecrackerExecutor | `terraphim-ai/crates/terraphim_rlm/src/executor/firecracker.rs` | RLM's VM execution backend | VmPoolManager |
| Adapter (NEW) | `terraphim-ai/crates/terraphim_rlm/src/executor/` | Bridge between struct and trait | fcctl-core, terraphim_firecracker |

**Type Mismatch:**
- **fcctl-core**: `VmManager` is a concrete struct with direct implementation
- **terraphim_firecracker**: Expects `Arc<dyn VmManager>` trait object for pool

## 4. Constraints and Their Implications

**Technical Constraints:**

1. **Async Runtime Compatibility**
   - fcctl-core uses standard async/await
   - terraphim_firecracker uses async-trait
   - **Implication**: Adapter must be compatible with both

2. **Send + Sync Requirements**
   - Trait requires Send + Sync for Arc sharing
   - fcctl-core's VmManager must be wrapped appropriately
   - **Implication**: May need Mutex/RwLock wrapping

3. **Error Type Compatibility**
   - fcctl-core returns fcctl_core::Error
   - Trait expects terraphim_firecracker::Error
   - **Implication**: Error conversion/translation layer needed

4. **VM Configuration Differences**
   - fcctl-core uses VmConfig struct
   - Trait uses VmRequirements
   - **Implication**: Configuration mapping required

**Performance Constraints:**

5. **Sub-500ms Allocation Guarantee**
   - Pool must maintain latency SLA
   - Adapter must not add significant overhead
   - **Implication**: Minimal wrapper overhead, no blocking operations

6. **Resource Efficiency**
   - VM reuse critical for performance
   - Adapter must preserve pool's VM lifecycle management
   - **Implication**: Direct passthrough preferred over complex translation

## 5. Risks, Unknowns, and Assumptions

**Critical Risks:**

1. **Trait Method Mismatch (HIGH)**
   - fcctl-core's VmManager methods may not match trait exactly
   - **Risk**: Some trait methods cannot be implemented
   - **De-risk**: Compare method signatures before implementation

2. **State Management Complexity (HIGH)**
   - fcctl-core maintains internal state (running_vms HashMap)
   - Pool may expect different state model
   - **Risk**: Double-booking or state inconsistency
   - **De-risk**: Careful state synchronization design

3. **Error Handling Compatibility (MEDIUM)**
   - Different error types between crates
   - **Risk**: Error information loss or incorrect mapping
   - **De-risk**: Comprehensive error variant mapping

**Unknowns:**

4. **VM Identifier Format**
   - fcctl-core may use different VM ID format than pool expects
   - **Unknown**: UUID vs ULID vs custom format

5. **Configuration Translation**
   - How VmConfig maps to VmRequirements
   - **Unknown**: Full compatibility or subset only

**Assumptions:**

- fcctl-core's VmManager can be wrapped without significant overhead
- async-trait compatibility layer won't impact performance
- Error types have sufficient overlap for meaningful translation
- VM lifecycle (create/start/stop/delete) maps cleanly between systems

## 6. Context Complexity vs. Simplicity Opportunities

**Complexity Sources:**

1. **Two-crate integration** - Different error types, config types, async models
2. **State duplication risk** - Both layers maintain VM state
3. **Trait/struct impedance mismatch** - Object-oriented vs concrete types
4. **Performance sensitivity** - Adapter must not degrade sub-500ms SLA

**Simplification Strategies:**

1. **Minimal Adapter Pattern**
   - Thin wrapper, not complex translation layer
   - Direct method passthrough where possible
   - Avoid state duplication by delegating to fcctl-core

2. **Clear Ownership Model**
   - fcctl-core owns VM lifecycle
   - Pool owns scheduling and warming
   - Adapter is transparent bridge

3. **Error Type Aggregation**
   - Create adapter-specific errors that wrap both
   - Preserve original error information
   - Let caller decide handling strategy

## 7. Questions for Human Reviewer

1. **Method Compatibility**: Should we verify all trait methods are implementable with fcctl-core's API before starting implementation?

2. **Error Handling Strategy**: Should adapter errors preserve both fcctl-core and trait error information, or translate to a unified error type?

3. **VM ID Format**: What VM identifier format does the pool expect (UUID, ULID, string)? fcctl-core may use a different format.

4. **State Ownership**: Should the adapter maintain any state, or purely delegate to fcctl-core's VmManager?

5. **Configuration Translation**: How should we handle VmConfig to VmRequirements mapping - exhaustive translation or minimal viable mapping?

6. **Performance Testing**: Should we benchmark the adapter to ensure sub-500ms allocation is preserved?

7. **Fallback Strategy**: If some trait methods cannot be implemented, should we stub them (return error) or fail compilation?

8. **Testing Strategy**: Should we create integration tests that verify both fcctl-core and the adapter work together?

9. **Future Extensibility**: Should the adapter be designed to potentially support other VM backends in future?

10. **Documentation**: Should we document the adapter pattern for other developers who might need similar integrations?
