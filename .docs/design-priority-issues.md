# Implementation Plan: Priority Issues & PRs Resolution

**Status**: Draft  
**Research Doc**: [research-priority-issues.md](research-priority-issues.md)  
**Author**: Terraphim AI Assistant  
**Date**: 2026-02-04  
**Estimated Effort**: 3 days (P0 + P1 items)

## Overview

### Summary
This plan addresses the critical blockers (#491, #462, test utils compilation) and merges ready PRs (#516, #492, #443, #413) to unblock development and deliver immediate value. It follows a phased approach: unblock first, then deliver.

### Approach
**Essentialism-Driven**: Address only the vital few items that block all other work. Complex features (GPUI, CodeGraph) are deferred to separate epics. Performance issues are batched.

### Scope

**In Scope:**
1. Fix clean clone build failure (#491)
2. Fix auto-update 404 error (#462)
3. Fix test utils compilation (new issue)
4. Review/merge PR #516 (agent integration tests)
5. Review/merge PR #492 (CLI onboarding wizard)
6. Review/merge PR #443 (validation framework - LLM hooks)
7. Review/merge PR #413 (validation framework - base)

**Out of Scope:**
- GPUI desktop migration (#461) - Separate epic
- CodeGraph implementation (#490) - Requires design spike
- Performance optimizations (#432, #434-438) - Batch separately
- npm/PyPI publishing (#315, #318) - Release process
- MCP Aggregation phases (#278-281) - Lower priority

**Avoid At All Cost:**
- Refactoring beyond immediate fix scope
- Adding new dependencies
- Perfecting instead of shipping
- Scope creep from "while we're here"

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Priority Fixes                            │
├─────────────────────────────────────────────────────────────┤
│  P0: Build System                                            │
│  ├── terraphim_rlm: Feature-gate fcctl-core                  │
│  └── terraphim_test_utils: Fix unsafe env ops               │
│                                                              │
│  P0: Auto-Update                                              │
│  └── terraphim_update: Fix asset name normalization         │
│                                                              │
│  P1: Ready PRs                                                │
│  ├── PR #516: Agent integration tests                        │
│  ├── PR #492: CLI onboarding wizard                          │
│  └── PR #443/#413: Validation framework                      │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow (Auto-Update Fix)

```
[User runs update] 
  → [check_update()]
    → [normalize_asset_name(bin_name)]  ← FIX: underscore→hyphen
    → [GitHub API: get_latest_release()]
    → [compare versions]
    → [download asset]
      → [Asset name matches release artifact]  ← FIX: consistent naming
    → [verify + install]
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Feature-gate fcctl-core in terraphim_rlm | Minimal change, unblocks build immediately | Git submodule (complex), Remove crate (loses work) |
| Fix test utils with unsafe blocks + cfg flag | Already attempted, just needs build.rs | Remove crate (breaks tests), Use serial_test (adds dep) |
| Normalize asset names at lookup time | Fixes without changing release process | Change release asset naming (requires CI changes) |
| Merge PRs sequentially (not parallel) | Easier to bisect if issues | Parallel merge (faster but risky) |
| Skip #461 (GPUI) entirely | 68K lines, labeled "DON'T MERGE" | Attempt review (too large) |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Refactor RLM for clean architecture | Not essential for unblocking | 1+ week delay, no user value |
| Rewrite auto-updater | Current design works, just naming bug | Over-engineering, risk of new bugs |
| Add feature flags for all optional deps | Scope creep | Maintenance burden |
| Review GPUI in this batch | 68K lines, separate concerns | Blocks all other work for weeks |
| Fix performance issues now | Not blocking, can batch | Context switching, delay delivery |

### Simplicity Check

**What if this could be easy?**

- Build fix: Add `optional = true` to one dependency line
- Test fix: Wrap unsafe calls in `unsafe {}` blocks with `#[cfg]`  
- Auto-update: Normalize `terraphim_agent` → `terraphim-agent` before lookup
- PRs: Review, rebase if needed, merge

**Senior Engineer Test**: Would a senior engineer call this overcomplicated? 
**Answer**: No. These are minimal, surgical fixes.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

---

## File Changes

### New Files (from PRs - verify)

| File | Purpose | Source PR |
|------|---------|-----------|
| `crates/terraphim_agent/src/onboarding/*.rs` | CLI wizard modules | #492 |
| `crates/terraphim_agent/tests/*_test.rs` | Integration tests | #516 |
| `.docs/*-validation*.md` | V-model documentation | #443, #413 |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_rlm/Cargo.toml` | Make fcctl-core optional |
| `crates/terraphim_rlm/src/lib.rs` | Gate fcctl-core imports behind feature |
| `crates/terraphim_test_utils/Cargo.toml` | Add build.rs for feature flag |
| `crates/terraphim_test_utils/src/lib.rs` | Fix unsafe env operations |
| `crates/terraphim_update/src/lib.rs` | Normalize asset names |
| `crates/terraphim_update/src/downloader.rs` | Consistent asset naming |

### Deleted Files
None

---

## API Design

### No New Public APIs

This plan focuses on fixes and merging existing PRs. No new public APIs are introduced.

### Modified Internal APIs

```rust
// terraphim_update/src/lib.rs
// Asset name normalization (internal)
fn normalize_asset_name(name: &str) -> String {
    name.replace('_', "-")
}

// terraphim_test_utils/src/lib.rs  
// Already has correct API, just fix implementation
pub fn set_env_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    #[cfg(rust_has_unsafe_env_setters)]
    unsafe { std::env::set_var(key, value) }
    
    #[cfg(not(rust_has_unsafe_env_setters))]
    std::env::set_var(key, value)
}
```

---

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_normalize_asset_name` | `terraphim_update/src/lib.rs` | Verify underscore→hyphen |
| `test_set_env_var_safe` | `terraphim_test_utils/src/lib.rs` | Env var setting works |
| `test_set_env_var_unsafe` | `terraphim_test_utils/src/lib.rs` | Unsafe path works |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_clean_clone_build` | CI workflow | Fresh checkout builds |
| `test_auto_update_check` | `terraphim_update/tests/` | Mock GitHub API |
| `test_agent_cross_mode` | From PR #516 | Cross-mode consistency |
| `test_onboarding_wizard` | From PR #492 | Template application |

### Verification Steps

```bash
# Test 1: Clean clone build
cd /tmp
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --workspace  # Should succeed

# Test 2: Auto-update naming
cargo test -p terraphim_update test_normalize_asset_name

# Test 3: PR #516 integration tests
cargo test -p terraphim_agent --test integration  # From PR

# Test 4: PR #492 wizard
cargo test -p terraphim_agent test_onboarding  # From PR
```

---

## Implementation Steps

### Phase 1: Unblock Build (Day 1)

#### Step 1.1: Fix terraphim_rlm Build
**Files:** `crates/terraphim_rlm/Cargo.toml`, `crates/terraphim_rlm/src/lib.rs`
**Description:** Make fcctl-core optional dependency
**Tests:** `cargo build -p terraphim_rlm` succeeds
**Estimated:** 30 minutes

```toml
# Cargo.toml change
[dependencies]
fcctl-core = { path = "../../../firecracker-rust/fcctl-core", optional = true }

[features]
default = []
firecracker = ["fcctl-core"]
```

```rust
// lib.rs change
#[cfg(feature = "firecracker")]
use fcctl_core::VmManager;
```

#### Step 1.2: Fix terraphim_test_utils
**Files:** `crates/terraphim_test_utils/build.rs` (new), `Cargo.toml`
**Description:** Add build.rs to detect Rust version, fix already present in lib.rs
**Tests:** `cargo build -p terraphim_test_utils` succeeds
**Dependencies:** Step 1.1
**Estimated:** 1 hour

```rust
// build.rs
use std::process::Command;

fn main() {
    let output = Command::new("rustc")
        .args(["--version"])
        .output()
        .expect("rustc not found");
    
    let version = String::from_utf8_lossy(&output.stdout);
    // Parse version, set cfg flag for 1.92+
    if is_rust_1_92_or_later(&version) {
        println!("cargo:rustc-cfg=rust_has_unsafe_env_setters");
    }
}
```

#### Step 1.3: Verify Clean Build
**Description:** Test on fresh clone
**Tests:** Full workspace build succeeds
**Dependencies:** Step 1.2
**Estimated:** 30 minutes

**Command:**
```bash
cd /tmp && rm -rf terraphim-ai-test
git clone https://github.com/terraphim/terraphim-ai.git terraphim-ai-test
cd terraphim-ai-test
cargo build --workspace 2>&1 | tail -20
```

### Phase 2: Fix Auto-Update (Day 1)

#### Step 2.1: Asset Name Normalization
**Files:** `crates/terraphim_update/src/lib.rs`, `src/downloader.rs`
**Description:** Normalize binary names (underscore to hyphen) before asset lookup
**Tests:** Unit test for normalization function
**Estimated:** 1 hour

```rust
// lib.rs
/// Normalize binary name for GitHub asset matching
/// Converts underscores to hyphens for consistent naming
fn normalize_bin_name(name: &str) -> String {
    name.replace('_', "-")
}

// In check_update():
let bin_name_for_asset = normalize_bin_name(&bin_name);
```

#### Step 2.2: Verify Against GitHub Releases
**Description:** Check actual release asset names
**Tests:** Mock test with real naming pattern
**Dependencies:** Step 2.1
**Estimated:** 30 minutes

**Verification:**
```bash
# Check actual GitHub releases
curl -s https://api.github.com/repos/terraphim/terraphim-ai/releases/latest | \
  jq '.assets[].name'
```

### Phase 3: Review Ready PRs (Day 2-3)

#### Step 3.1: Review PR #516 (Agent Integration Tests)
**Files:** All files in PR
**Description:** Review cross-mode consistency and KG ranking tests
**Review Checklist:**
- [ ] Tests are meaningful (not tautological)
- [ ] No breaking changes
- [ ] Documentation updated
- [ ] CI will pass
**Estimated:** 2 hours

**Merge Command:**
```bash
gh pr checkout 516
cargo test -p terraphim_agent  # Run tests locally
cargo clippy -p terraphim_agent --tests  # Check lint
gh pr merge 516 --squash
```

#### Step 3.2: Review PR #492 (CLI Onboarding Wizard)
**Files:** All files in PR
**Description:** Review 6 templates, interactive prompts, validation
**Review Checklist:**
- [ ] 6 templates work correctly
- [ ] Interactive mode handles errors gracefully
- [ ] Non-interactive flags work
- [ ] No breaking changes to existing CLI
**Estimated:** 2 hours

**Merge Command:**
```bash
gh pr checkout 492
cargo test -p terraphim_agent --features onboarding  # Run tests
cargo build -p terraphim_agent --release
./target/release/terraphim-agent setup --list-templates  # Manual test
gh pr merge 492 --squash
```

#### Step 3.3: Review PR #443 (Validation Framework - LLM Hooks)
**Files:** All files in PR
**Description:** Review runtime validation hook integration
**Review Checklist:**
- [ ] HookManager properly integrated
- [ ] All 9 LLM calls use hooks
- [ ] Error handling is graceful
- [ ] Documentation is comprehensive
**Estimated:** 2 hours

**Merge Command:**
```bash
gh pr checkout 443
cargo test -p terraphim_multi_agent  # Run tests
cargo test --workspace --features validation  # Full test
cargo clippy -p terraphim_multi_agent --features validation
gh pr merge 443 --squash
```

#### Step 3.4: Review PR #413 (Validation Framework - Base)
**Files:** All files in PR  
**Description:** Review base validation framework (if not merged with #443)
**Review Checklist:**
- [ ] Basic validation crate structure
- [ ] CLI integration
- [ ] Documentation
**Estimated:** 1 hour

**Note:** If #443 already includes #413 changes, skip this step.

### Phase 4: Post-Merge Verification (Day 3)

#### Step 4.1: Full Workspace Test
**Description:** Ensure all merged changes work together
**Tests:** Full test suite passes
**Dependencies:** All PRs merged
**Estimated:** 1 hour

```bash
cargo test --workspace --all-features
cargo build --workspace --release
```

#### Step 4.2: Update GitHub Issues
**Description:** Close resolved issues
**Estimated:** 30 minutes

```bash
gh issue close 491 --comment "Fixed by feature-gating fcctl-core dependency"
gh issue close 462 --comment "Fixed by normalizing asset names in updater"
```

---

## Rollback Plan

If issues discovered post-merge:

1. **Build still fails:**
   - Revert fcctl-core feature gate
   - Alternative: Exclude terraphim_rlm from workspace temporarily
   
2. **Auto-update still broken:**
   - Check actual GitHub release asset names
   - May need CI/release process change instead
   
3. **PR merge causes issues:**
   - Each PR is squashed, easy to revert
   - `git revert <merge-commit>`
   - Re-open original issue with details

**Feature Flags:**
- `terraphim_rlm/firecracker` - Controls fcctl-core dependency
- Validation framework already uses feature gates

---

## Dependencies

### No New Dependencies

This plan introduces no new crate dependencies.

### Modified Dependencies

| Crate | Change | Reason |
|-------|--------|--------|
| fcctl-core | Make optional | Unblock clean builds |
| self_update | No change | Use existing |

---

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Clean clone build time | < 5 min | `cargo build --workspace` |
| Auto-update check | < 2s | Network + comparison |
| Test execution | < 60s | Full workspace test |

### No Performance Regressions Expected

All changes are:
- Build system fixes (no runtime impact)
- String normalization (negligible overhead)
- PR merges (already tested by authors)

---

## Open Items

| Item | Status | Owner | Notes |
|------|--------|-------|-------|
| Verify GitHub release asset naming | Pending | @AlexMikhalev | Check actual release page |
| Confirm PR #413 vs #443 overlap | Pending | PR author | May be duplicate |
| Test environment for clean build | Pending | CI | Need fresh container |
| CodeGraph epic creation | Deferred | Future | Out of scope |

---

## Risks and Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| PR #516 has merge conflicts | Medium | Low | Rebase before review |
| PR tests fail on merge | Low | Medium | Run tests before merging |
| Asset naming more complex than expected | Low | Low | Check releases API first |
| fcctl-core feature gate breaks RLM | Low | Medium | Test RLM with feature enabled |
| GPUI PR (#461) blocks other merges | Medium | Low | Keep separate, don't wait |

---

## Success Criteria

### Phase 1 Success
- [ ] `cargo build --workspace` succeeds on clean clone
- [ ] `cargo test -p terraphim_test_utils` passes
- [ ] No compilation errors in any crate

### Phase 2 Success
- [ ] Auto-update unit tests pass
- [ ] Asset naming correctly handles underscore/hyphen
- [ ] Mock update check works

### Phase 3 Success
- [ ] PR #516 merged, tests passing
- [ ] PR #492 merged, wizard functional
- [ ] PR #443 merged, validation hooks active
- [ ] Issues #491 and #462 closed

### Overall Success
- [ ] Clean clone → build in < 5 minutes
- [ ] Auto-update works for all platforms
- [ ] All ready PRs merged
- [ ] No regressions introduced
- [ ] Issues #491, #462 closed

---

## Approval

**Required Approvals:**
- [ ] Technical review (implementation approach)
- [ ] Test strategy review
- [ ] Risk assessment
- [ ] Human approval from @AlexMikhalev

**Sign-off Criteria:**
- Plan is clear enough for anyone to implement
- Risks are understood and mitigated
- Scope is essential only (no scope creep)
- Timeline is realistic

---

## Appendix

### GitHub CLI Commands Reference

```bash
# Check issue details
gh issue view <number>

# Check PR details  
gh pr view <number>

# Checkout PR for testing
gh pr checkout <number>

# Merge PR
gh pr merge <number> --squash

# Close issue with comment
gh issue close <number> --comment "Fixed by..."

# List open issues
gh issue list --state open

# List open PRs
gh pr list --state open
```

### Test Commands Reference

```bash
# Build specific crate
cargo build -p <crate_name>

# Test specific crate
cargo test -p <crate_name>

# Test with all features
cargo test -p <crate_name> --all-features

# Clippy checks
cargo clippy -p <crate_name> --tests

# Full workspace
cargo build --workspace
cargo test --workspace
```

### Related Documents
- [Research Document](research-priority-issues.md)
- [PR #516](https://github.com/terraphim/terraphim-ai/pull/516)
- [PR #492](https://github.com/terraphim/terraphim-ai/pull/492)
- [PR #443](https://github.com/terraphim/terraphim-ai/pull/443)
- [PR #413](https://github.com/terraphim/terraphim-ai/pull/413)
- [Issue #491](https://github.com/terraphim/terraphim-ai/issues/491)
- [Issue #462](https://github.com/terraphim/terraphim-ai/issues/462)
