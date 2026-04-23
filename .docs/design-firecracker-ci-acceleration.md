# Design & Implementation Plan: Firecracker CI Acceleration for Rust Builds

## 1. Summary of Target Behavior

Transform the current CI pipeline to leverage Firecracker microVMs for 2-5x faster Rust compilation and 3-10x faster test execution by:

- Using pre-warmed microVMs with cached Cargo registries and pre-built dependencies
- Executing builds and tests inside isolated Firecracker VMs via `fcctl-web` API
- Supporting parallel compilation across multiple VMs with matrix strategies
- Persisting `target/` directories across builds using VM snapshots or shared volumes
- Eliminating self-hosted runner contention and cold-start penalties

**Success Metrics:**
- Cargo build: < 30 seconds (from current 2-5 minutes)
- Cargo test: < 60 seconds (from current 5-10 minutes)
- VM boot time: < 1 second
- Cache hit rate: > 80% for Cargo registry

## 2. Key Invariants and Acceptance Criteria

| ID | Criterion | Testable? | Verification Method |
|----|------------|----------|---------------------|
| AC1 | Firecracker VM boots in < 1 second | Yes | Time `fcctl-web` API response |
| AC2 | Cargo registry cached in VM (no re-download) | Yes | Check `~/.cargo/registry` exists |
| AC3 | `cargo build --workspace` completes < 30s | Yes | Time CI step execution |
| AC4 | `cargo test --workspace` completes < 60s | Yes | Time CI step execution |
| AC5 | Parallel builds across 3+ VMs succeed | Yes | Matrix strategy in CI |
| AC6 | No regression in build output or test results | Yes | Compare outputs with baseline |
| AC7 | VM cleanup after build (no residual state) | Yes | Check VM destroyed after use |
| AC8 | Secure isolation (no cross-VM access) | Yes | Security audit of fcctl-web |

**Invariants:**
- I1: VMs are ephemeral - each build gets a clean environment or restored snapshot
- I2: Cargo registry cache is read-only to prevent corruption
- I3: Build artifacts are copied out before VM destruction
- I4: No credentials or secrets persist in VM snapshots

## 3. High-Level Design and Boundaries

### System Architecture

```
GitHub Actions Workflow
         │
         ▼
    fcctl-web API (host)
         │
         ├──► VM-1 (Rust build cache) ──► cargo build
         ├──► VM-2 (clean) ──────────────► cargo test
         └──► VM-3 (parallel build) ───────► cargo clippy
```

### Components

| Component | Responsibility | Location | Changes |
|-----------|----------------|----------|---------|
| **fcctl-web** | Manage Firecracker VMs, execute commands | `scratchpad/firecracker-rust/fcctl-web` | Extend API for build artifacts |
| **CI Workflow** | Orchestrate builds in Firecracker VMs | `.github/workflows/ci-firecracker.yml` | Create new workflow |
| **VM Template** | Pre-configured Rust build VM | `infrastructure/vm-templates/rust-build.json` | New file |
| **Cache Manager** | Maintain Cargo registry cache | `scripts/fc-cache-sync.sh` | New file |

### Boundaries

**Inside scope:**
- Firecracker VM creation and management via fcctl-web
- Caching Cargo registry and dependencies
- Parallel build execution
- Artifact collection from VMs

**Outside scope:**
- Replacing self-hosted runners for non-Rust tasks
- Docker builds (use existing `ci-native.yml`)
- Frontend builds (use existing workflow)

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|-------|------|--------------|
| `.github/workflows/ci-firecracker.yml` | Create | - | New CI workflow using Firecracker | fcctl-web API |
| `infrastructure/vm-templates/rust-build.json` | Create | - | VM definition with Rust toolchain | Firecracker |
| `scripts/fc-cache-sync.sh` | Create | - | Sync Cargo cache to VM | rsync, virtio-fs |
| `scripts/ci-firecracker-build.sh` | Create | - | Build orchestration script | fcctl-web client |
| `crates/terraphim_github_runner/src/fcctl_client.rs` | Create | - | Rust client for fcctl-web API | reqwest |
| `crates/terraphim_github_runner/Cargo.toml` | Modify | Basic runner | Add fcctl-web client deps | - |
| `.github/workflows/ci-pr.yml` | Modify | Uses self-hosted runners | Add Firecracker option | ci-firecracker.yml |

## 5. Step-by-Step Implementation Sequence

1. **Create VM template with Rust toolchain** - Define base VM with Rust 1.87.0, Cargo, common targets - Deployable: No (infrastructure setup)

2. **Extend fcctl-web API for build artifacts** - Add endpoints: `/api/vms/{id}/upload`, `/api/vms/{id}/download` - Deployable: Yes (backward compatible)

3. **Create cache sync script** - Sync `~/.cargo/registry` to VM via virtio-9p or rsync - Deployable: Yes (can test manually)

4. **Implement fcctl-web Rust client** - Type-safe API client in `terraphim_github_runner` - Deployable: Yes (library only)

5. **Create CI workflow for Firecracker builds** - Orchestrate VM creation → build → test → artifact collection - Deployable: Yes (new workflow, doesn't affect existing)

6. **Add parallel build matrix to workflow** - Test multiple crates in parallel VMs - Deployable: Yes (feature flag via workflow_dispatch)

7. **Update ci-pr.yml to use Firecracker option** - Feature flag to switch between self-hosted and Firecracker - Deployable: Yes (feature flag, default: current behavior)

8. **Benchmark and optimize** - Compare build times, tune cache, optimize VM template - Deployable: Yes (gradual rollout)

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location | Test Description |
|---------------------|-----------|---------------|-----------------|
| AC1: VM boot < 1s | Integration | `crates/terraphim_github_runner/tests/vm_boot_test.rs` | Time `curl http://fcctl-web:8080/api/vms` response |
| AC2: Registry cached | Integration | `scripts/tests/test_cache_sync.sh` | Verify `~/.cargo/registry` in VM |
| AC3: Build < 30s | Benchmark | `.github/workflows/benchmark-build.yml` | Time `cargo build --workspace` in VM |
| AC4: Test < 60s | Benchmark | `.github/workflows/benchmark-build.yml` | Time `cargo test --workspace` in VM |
| AC5: Parallel builds | Integration | `.github/workflows/ci-firecracker.yml` | Matrix strategy with 3 VMs |
| AC6: No regression | Comparison | `scripts/compare-build-outputs.sh` | Diff outputs from old vs new CI |
| AC7: VM cleanup | Integration | `crates/terraphim_github_runner/tests/vm_cleanup_test.rs` | Verify VM destroyed after build |
| AC8: Secure isolation | Security audit | `scripts/audit-vm-isolation.sh` | Check no cross-VM network access |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| **R1: fcctl-web API instability** (from Phase 1 research: VM execution tests marked experimental) | Add retry logic, health checks, fallback to self-hosted runners | API may need redesign |
| **R2: Cache corruption across builds** | Mount registry as read-only, use copy-on-write for target/ | Registry version conflicts |
| **R3: VM resource exhaustion** (from CLAUDE.md: memory at 99.5%) | Resource limits per VM, monitor with `fcctl-web /health`, auto-cleanup | Host memory pressure |
| **R4: Artifact transfer slow** (copying target/ from VM) | Use virtio-fs shared mounts, incremental artifact collection | Network bottleneck on large artifacts |
| **R5: Firecracker not available on GitHub Actions** | Use self-hosted runner with Firecracker installed, or nested virt | GitHub-hosted runners don't support KVM |
| **R6: Secret exposure in VMs** | Never inject secrets into VM, use GitHub Actions secrets after VM finishes | Human error in workflow design |

**Complexity Assessment:**
- **High**: Multi-component coordination (fcctl-web, VMs, CI workflow, cache management)
- **Medium**: API client implementation, VM template creation
- **Low**: Shell scripts for cache sync, benchmark workflow

## 8. Open Questions / Decisions for Human Review

1. **Q1: Should we use virtio-9p (file sharing) or rsync over SSH for cache sync?**
   - A: virtio-9p is faster but requires host-side setup
   - B: rsync is simpler but slower

2. **Q2: Where should fcctl-web run?**
   - A: On self-hosted runner (current setup)
   - B: On dedicated server with more resources
   - C: Inside another Firecracker VM (nested)

3. **Q3: Should we maintain persistent VMs or create/destroy per build?**
   - A: Persistent = faster (no boot), but state leakage risk
   - B: Ephemeral = clean builds, but slower (VM creation time)

4. **Q4: How to handle the existing `ci-pr.yml` failure (self-hosted runner issues)?**
   - A: Replace entirely with Firecracker workflow
   - B: Keep as fallback, use Firecracker when available
   - C: Fix self-hosted runners (may be out of scope)

5. **Q5: Should we create a new GitHub Runner (custom) that spawns Firecracker VMs?**
   - This would integrate seamlessly with existing `runs-on: [self-hosted]`
   - Requires implementing GitHub Runner protocol in Rust

**Recommendation for Q4:** Option B - Use Firecracker when fcctl-web is available, fall back to self-hosted runners. This provides immediate speed improvements while maintaining compatibility.

## 9. Estimated Impact

| Metric | Current | Target | Improvement |
|--------|--------|--------|-------------|
| Cargo build time | 2-5 min | < 30s | 4-10x faster |
| Cargo test time | 5-10 min | < 60s | 5-10x faster |
| VM boot time | N/A | < 1s | Firecracker advantage |
| Cache hit rate | 0% (cold starts) | > 80% | Massive savings |
| Parallel jobs | 1 (runner contention) | 3-5 (VMs) | 3-5x throughput |

## 10. References

- Current Firecracker setup: `crates/terraphim_github_runner/FIRECRACKER_FIX.md`
- fcctl-web service: `crates/terraphim_github_runner/src/`
- Current CI workflow: `.github/workflows/ci-pr.yml`
- VM execution tests: `.github/workflows/vm-execution-tests.yml`
- Firecracker documentation: https://firecracker-microvm.github.io/
