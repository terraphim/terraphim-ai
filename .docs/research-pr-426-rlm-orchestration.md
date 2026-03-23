# Research Document: RLM Orchestration with MCP Tools (PR #426)

**Status**: Draft
**Author**: Alex Mikhalev
**Date**: 2026-03-11
**Reviewers**: 

## Executive Summary

PR #426 aims to integrate RLM orchestration with MCP tools using Firecracker VMs. The current state reveals a critical dependency issue: `terraphim_rlm` requires `fcctl-core` which is located in `scratchpad/firecracker-rust/fcctl-core` but the `Cargo.toml` points to a non-existent path `../../../firecracker-rust/fcctl-core`. Additionally, `terraphim_firecracker` implements some pool management but lacks integration with the full VM lifecycle and snapshot management required by `terraphim_rlm`. Key missing features include `ExecutionEnvironment` trait implementation, `VmManager` integration, `SnapshotManager` integration, pre-warmed pool, OverlayFS, network audit, LLM bridge, and output streaming.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Solving isolated code execution is critical for AI coding assistant safety and capability. |
| Leverages strengths? | Yes | Team has Rust expertise and existing `terraphim_firecracker` crate. |
| Meets real need? | Yes | PR #426 is blocked, and GitHub issues #664-670 document missing features. |

**Proceed**: Yes - at least 2/3 YES

## Problem Statement

### Description
Enable RLM (Recursive Language Model) to execute code in isolated Firecracker VMs with sub-500ms allocation, snapshot support, and MCP tool integration.

### Impact
Without this, RLM cannot safely execute untrusted code, limiting its utility as an AI coding assistant.

### Success Criteria
`terraphim_rlm` builds and passes tests using `fcctl-core` for VM management, and `terraphim_firecracker` provides pre-warmed VM pools.

## Current State Analysis

### Existing Implementation
1. `terraphim_rlm`: Defines `FirecrackerExecutor` using `fcctl-core` types (`VmManager`, `SnapshotManager`) but cannot build due to missing dependency. The crate is excluded from the workspace (listed in `exclude` array in root `Cargo.toml`). User reported 108 tests passing, but this likely refers to a previous state or different configuration.
2. `terraphim_firecracker`: Implements `VmPoolManager`, `Sub2SecondOptimizer`, etc., with 54 passing tests. However, `ensure_pool` in `FirecrackerExecutor` is unimplemented (TODO).
3. `fcctl-core`: Exists in `scratchpad/firecracker-rust/fcctl-core` with functional `VmManager` and `SnapshotManager` implementations, but is not integrated into the main workspace.

### Code Locations
| Component | Location | Purpose |
|-----------|----------|---------|
| `terraphim_rlm` | `crates/terraphim_rlm/src/executor/firecracker.rs` | Firecracker execution backend |
| `terraphim_firecracker` | `terraphim_firecracker/src/lib.rs` | VM pool management |
| `fcctl-core` | `scratchpad/firecracker-rust/fcctl-core` | VM and snapshot management |

### Data Flow
1. RLM receives code execution request.
2. `FirecrackerExecutor` checks for available VM from pool.
3. If no VM, allocate from pool (not implemented).
4. Execute code via SSH on VM.
5. Create/restore snapshots using `fcctl-core`.

### Integration Points
- `terraphim_rlm` -> `fcctl-core` (VM lifecycle, snapshots)
- `terraphim_rlm` -> `terraphim_firecracker` (pool management)
- `fcctl-core` -> Firecracker binary (VM execution)

## Constraints

### Technical Constraints
- KVM support required (`/dev/kvm` must exist).
- Firecracker binary installed at `/usr/bin/firecracker`.
- Linux host required.

### Business Constraints
- PR #426 blocked on missing `fcctl-core` dependency.
- Need to unblock development to proceed with RLM features.

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| VM Allocation Time | < 500ms | Not implemented |
| Boot Time | < 2s | Achieved in `terraphim_firecracker` |
| Snapshot Restore Time | < 1s | Not measured |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Fix `fcctl-core` dependency | Blocker for building `terraphim_rlm` | `terraphim_rlm/Cargo.toml` points to non-existent path |
| Integrate VM pool with VmManager | Required for pre-warmed VM allocation | `ensure_pool` TODO in `FirecrackerExecutor` |
| Implement snapshot support | Required for state versioning | `create_snapshot` uses `fcctl-core` SnapshotManager |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Network audit implementation | Tracked in GitHub issue #667 |
| OverlayFS implementation | Tracked in GitHub issue #668 |
| LLM bridge implementation | Tracked in GitHub issue #669 |
| Output streaming | Tracked in GitHub issue #670 |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_firecracker` | Provides pool management | Medium - incomplete integration |
| `fcctl-core` | Provides VM/snapshot management | High - missing from workspace |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `firecracker` | Latest | High - binary must be installed | None |
| `tokio` | 1.0 | Low - well established | None |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `fcctl-core` incomplete | Medium | High | Review code, implement missing features |
| Integration complexity | High | Medium | Design clear interfaces, incremental integration |
| KVM not available | Low | High | Document requirements, provide fallback |

### Open Questions
1. Is `fcctl-core` production-ready? - Review code and tests
2. How does `terraphim_firecracker` integrate with `fcctl-core`? - Design interface
3. What is the migration path for existing VMs? - Plan rollout strategy

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `fcctl-core` provides necessary interfaces | Referenced in `terraphim_rlm` | Need to implement traits ourselves | No |
| `terraphim_firecracker` can adapt to `fcctl-core` | Both manage Firecracker VMs | Major refactoring required | No |
| SSH executor works with VM IPs | Standard Firecracker networking | Network configuration issues | No |

### Multiple Interpretations Considered
| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Move `fcctl-core` to workspace | Fixes dependency, enables build | Chosen - standard practice |
| Implement `fcctl-core` features in `terraphim_rlm` | Duplicates code, maintenance burden | Rejected - violates DRY |
| Use Docker instead of Firecracker | Less isolation, simpler setup | Rejected - security requirements |

## Research Findings

### Key Insights
1. `fcctl-core` exists but is in `scratchpad/` directory, not integrated into workspace.
2. `terraphim_rlm` already uses `fcctl-core` types but cannot build due to path issue.
3. `terraphim_firecracker` has 54 passing tests but lacks full integration with `fcctl-core`.

### Relevant Prior Art
- `firecracker-rust` project: Provides Rust bindings for Firecracker API.
- `fcctl` CLI: Command-line tool for Firecracker VM management.

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Review `fcctl-core` code | Assess completeness and stability | 4 hours |
| Design `terraphim_firecracker` integration | Define interface between pool manager and VmManager | 2 hours |
| Test `terraphim_rlm` build | Verify dependency fix | 1 hour |

## Recommendations

### Proceed/No-Proceed
**Proceed** with research findings. The dependency issue is clear and fixable.

### Scope Recommendations
1. **Fix `fcctl-core` path**: Update `terraphim_rlm/Cargo.toml` to use `../../scratchpad/firecracker-rust/fcctl-core` instead of `../../../firecracker-rust/fcctl-core`.
2. **Move `fcctl-core` to workspace** (Optional): Move `fcctl-core` from `scratchpad/` to `crates/` and add to workspace for better integration.
3. Implement `ensure_pool` in `FirecrackerExecutor` using `terraphim_firecracker`.
4. Complete snapshot management integration.

### Risk Mitigation Recommendations
1. Review `fcctl-core` tests to assess stability.
2. Start with minimal integration, expand incrementally.
3. Document KVM requirements clearly.

## Next Steps

If approved:
1. Move `fcctl-core` to `crates/fcctl-core` ✓ (COMPLETED)
2. Update `terraphim_rlm/Cargo.toml` ✓ (COMPLETED)
3. Run `cargo test -p terraphim_rlm` ✓ (COMPLETED - 108 tests passed)
4. Implement missing features based on GitHub issues #664-670 (IN PROGRESS)

## Appendix

### Reference Materials
- PR #426: RLM orchestration with MCP tools
- GitHub issues #664-670: Missing features
- `terraphim_rlm/src/executor/firecracker.rs`: Current implementation
- `scratchpad/firecracker-rust/fcctl-core`: VM management library

### Code Snippets
```rust
// From terraphim_rlm/src/executor/firecracker.rs
use fcctl_core::firecracker::models::SnapshotType;
use fcctl_core::snapshot::SnapshotManager;
use fcctl_core::vm::VmManager;
```
