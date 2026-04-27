# Design & Implementation Plan: Stable Release for Terraphim AI

## 1. Summary of Target Behavior

After implementation, Terraphim AI will have a stable release (v1.17.0 or v1.16.38) that:
- Compiles successfully (zlob issue resolved)
- Passes all workspace tests
- Has no known critical security vulnerabilities
- Includes a clear decision on ADF orchestrator inclusion
- Has stabilised CI pipeline

## 2. Key Invariants and Acceptance Criteria

| Invariant | Acceptance Criteria | Test/Verification |
|-----------|---------------------|-------------------|
| Compilation | `cargo build --workspace` succeeds | Manual/CI |
| Tests Pass | `cargo test --workspace` passes > 95% | CI |
| Security | No unaddressed RUSTSEC advisories | `cargo audit` |
| Documentation | CHANGELOG.md updated with release notes | Manual review |
| Version Consistency | All crates at same version | `cargo check` |

## 3. High-Level Design and Boundaries

### Components
1. **zlob Fix**: Resolve compilation blocker
2. **Test Stabilisation**: Ensure >95% test pass rate
3. **Security Verification**: Confirm all patches applied
4. **Release Decision**: Include/exclude ADF orchestrator
5. **Version Bump**: Update workspace version

### Boundaries
- **Inside**: Workspace crates, CI configuration, documentation
- **Outside**: Experimental crates (already excluded), firecracker crates (private deps)

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `Cargo.toml` | Modify | zlob dependency failing | zlob fixed or pinned | None |
| `CHANGELOG.md` | Modify | v1.14.0 last entry | Add v1.17.0/v1.16.38 entry | All changes |
| `.github/workflows/` | Modify | 151 CI fixes applied | Stable CI config | None |
| `crates/terraphim_symphony/` | Decision | Included in workspace | Exclude if unstable | Workspace config |

## 5. Step-by-Step Implementation Sequence

1. **[CRITICAL] Fix zlob compilation**
   - Try: Update zlob to compatible version
   - Try: Pin Zig version in CI/environment
   - Fallback: Exclude zlob-dependent crates temporarily
   - **Deployable**: No (blocks everything)
   - **Estimated time**: 1-4 hours
   - **Rollback**: Revert dependency changes

2. **[HIGH] Run full test suite**
   - `cargo test --workspace`
   - Document failing tests
   - Fix or skip (with justification)
   - **Deployable**: Yes, if >95% pass
   - **Estimated time**: 2-4 hours
   - **Rollback**: Revert test skip annotations

3. **[HIGH] Security audit**
   - `cargo audit`
   - Verify RUSTSEC patches applied
   - **Deployable**: Yes
   - **Estimated time**: 15 minutes
   - **Rollback**: Add advisories to ignore list (documented)

4. **[MEDIUM] ADF inclusion decision**
   - If tests pass with ADF: Include
   - If ADF causes >5% test failures: Exclude from workspace
   - **Deployable**: Yes, either way
   - **Estimated time**: 30 minutes
   - **Rollback**: Revert workspace config change

5. **[MEDIUM] Version bump and release notes**
   - Update `workspace.package.version`
   - Update CHANGELOG.md
   - Create git tag
   - **Deployable**: Yes
   - **Estimated time**: 30 minutes
   - **Rollback**: Delete tag, revert version bump commit

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| Compilation succeeds | Integration | `cargo build --workspace` |
| Tests > 95% pass | Integration | `cargo test --workspace` |
| No security advisories | Security | `cargo audit` |
| Version consistent | Unit | `cargo check` |
| CHANGELOG updated | Manual | Document review |

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| zlob unfixable | Exclude zlob-dependent crates | Medium - May lose features |
| ADF instability | Exclude from release | Low - Can release without |
| CI still flaky | Use stable runner, disable experimental jobs | Low |
| Security patches incomplete | `cargo audit` verification | Low |

## 8. Open Questions / Decisions for Human Review

1. **Release version**: v1.17.0 (minor) or v1.16.38 (patch)?
   - *Decision criteria*: If ADF included -> v1.17.0. If only fixes -> v1.16.38.

2. **ADF orchestrator**: Include or exclude?
   - *Decision criteria*: Include if `cargo test -p terraphim_symphony` passes > 90%.

3. **zlob approach**: Update, pin, or exclude?
   - *Decision criteria*: Try update first (least disruptive). Pin if update fails. Exclude only as last resort.

4. **Minimum test threshold**: 95% or higher?
   - *Decision criteria*: 95% for patch release, 98% for minor release.

## 9. Post-Release Monitoring

| Check | Frequency | Action if Failed |
|-------|-----------|------------------|
| CI pipeline | 48 hours | Revert release tag, investigate |
| Test pass rate | Weekly | Patch release if < 95% |
| Security advisories | Weekly | Emergency patch if critical |
| User-reported issues | Daily | Triage and schedule fixes |
