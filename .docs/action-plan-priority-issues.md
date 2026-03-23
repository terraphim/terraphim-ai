# Priority Issues Action Plan

## Executive Summary

**Goal**: Unblock development and deliver ready features in 3 days  
**Approach**: Fix critical blockers first, merge ready PRs, defer complex work  
**Risk**: Low - surgical fixes, well-tested PRs

---

## Priority Matrix

### P0: CRITICAL (Do First)

| Item | Issue/PR | Effort | Action |
|------|----------|--------|--------|
| Build fails on clean clone | #491 | 1 hr | Feature-gate fcctl-core in terraphim_rlm |
| Test utils compilation | NEW | 1 hr | Add build.rs for Rust version detection |
| Auto-update 404 | #462 | 1 hr | Normalize asset names (underscore→hyphen) |

### P1: HIGH VALUE (Do Next)

| Item | PR | Effort | Action |
|------|-----|--------|--------|
| Agent integration tests | #516 | 2 hrs | Review, test, merge |
| CLI onboarding wizard | #492 | 2 hrs | Review, test, merge |
| Validation framework hooks | #443 | 2 hrs | Review, test, merge |

### P2: STRATEGIC (Defer)

| Item | Issue | Why Deferred |
|------|-------|--------------|
| 1Password audit | #503 | Can batch with other security work |
| Agent search output | #499 | Nice-to-have, not blocking |
| CodeGraph | #490 | Needs design spike, separate epic |
| Performance issues | #432, #434-438 | Batch separately, not blocking |

### DEFERRED

| Item | Issue/PR | Reason |
|------|----------|--------|
| GPUI desktop | #461 | 68K lines, "DON'T MERGE" label |
| npm/PyPI publish | #315, #318 | Release process, not blocking |
| MCP Aggregation | #278-281 | Complex, lower priority |

---

## Day-by-Day Execution Plan

### Day 1: Unblock Build

**Morning (2 hours):**
```bash
# 1. Fix terraphim_rlm (30 min)
# Edit: crates/terraphim_rlm/Cargo.toml
# Make fcctl-core optional

# 2. Fix terraphim_test_utils (1 hour)  
# Create: crates/terraphim_test_utils/build.rs
# Detect Rust 1.92+ and set cfg flag

# 3. Verify on clean clone (30 min)
cd /tmp && rm -rf terraphim-ai-test
git clone https://github.com/terraphim/terraphim-ai.git terraphim-ai-test
cd terraphim-ai-test
cargo build --workspace
```

**Afternoon (2 hours):**
```bash
# 4. Fix auto-update asset naming (1 hour)
# Edit: crates/terraphim_update/src/lib.rs
# Add normalize_bin_name() function

# 5. Verify against GitHub releases (30 min)
curl -s https://api.github.com/repos/terraphim/terraphim-ai/releases/latest | \
  jq '.assets[].name'

# 6. Run tests (30 min)
cargo test -p terraphim_update
cargo test -p terraphim_test_utils
```

### Day 2: Merge PRs

**Morning (3 hours):**
```bash
# PR #516 - Agent integration tests
git checkout main && git pull
git fetch origin pull/516/head:pr-516
git checkout pr-516
cargo test -p terraphim_agent
cargo clippy -p terraphim_agent --tests
git checkout main && git merge --squash pr-516
git commit -m "test(agent): add integration tests for cross-mode consistency (#516)"
git push origin main
gh issue close 516  # if applicable
```

**Afternoon (3 hours):**
```bash
# PR #492 - CLI onboarding wizard
git fetch origin pull/492/head:pr-492
git checkout pr-492
cargo test -p terraphim_agent
cargo build -p terraphim_agent --release
./target/release/terraphim-agent setup --list-templates  # Manual verification
git checkout main && git merge --squash pr-492  
git commit -m "feat(agent): add CLI onboarding wizard (#492)"
git push origin main
gh issue close 493
```

### Day 3: Validation Framework + Verification

**Morning (3 hours):**
```bash
# PR #443 - Validation framework LLM hooks
git fetch origin pull/443/head:pr-443
git checkout pr-443
cargo test -p terraphim_multi_agent --all-features
cargo clippy -p terraphim_multi_agent --all-features
git checkout main && git merge --squash pr-443
git commit -m "feat(validation): add runtime LLM hooks (#443)"
git push origin main
gh issue close 442
```

**Afternoon (2 hours):**
```bash
# Full verification
cargo clean
cargo build --workspace --all-features
cargo test --workspace --all-features

# Test auto-update (mock)
cargo test -p terraphim_update

# Update and close issues
gh issue close 491 --comment "Fixed by making fcctl-core optional in terraphim_rlm"
gh issue close 462 --comment "Fixed by normalizing asset names in updater"
```

---

## Quick Fixes (Copy-Paste Ready)

### Fix #1: terraphim_rlm/Cargo.toml

```toml
[dependencies]
# ... other deps ...
fcctl-core = { path = "../../../firecracker-rust/fcctl-core", optional = true }

[features]
default = []
firecracker = ["dep:fcctl-core"]
full = ["firecracker"]
```

### Fix #2: terraphim_rlm/src/lib.rs

```rust
// At top of file
#[cfg(feature = "firecracker")]
pub mod firecracker_backend;

#[cfg(feature = "firecracker")]
pub use firecracker_backend::*;

// Where fcctl-core is used
#[cfg(feature = "firecracker")]
use fcctl_core::{VmManager, SnapshotManager};
```

### Fix #3: terraphim_test_utils/build.rs (Create this file)

```rust
use std::process::Command;

fn main() {
    let output = Command::new("rustc")
        .args(["--version"])
        .output()
        .expect("rustc not found");
    
    let version = String::from_utf8_lossy(&output.stdout);
    let version_num = version.split_whitespace().nth(1).unwrap_or("0.0.0");
    
    // Check if Rust 1.92 or later (when set_var became unsafe)
    let parts: Vec<u32> = version_num.split('.')
        .take(2)
        .filter_map(|s| s.parse().ok())
        .collect();
    
    if parts.len() == 2 && (parts[0] > 1 || (parts[0] == 1 && parts[1] >= 92)) {
        println!("cargo:rustc-cfg=rust_has_unsafe_env_setters");
    }
}
```

### Fix #4: terraphim_update/src/lib.rs

```rust
/// Normalize binary name for GitHub asset matching
/// GitHub releases use hyphens, but crate names may use underscores
fn normalize_bin_name(name: &str) -> String {
    name.replace('_', "-")
}

// In check_update(), replace:
// let bin_name_for_asset = bin_name.replace('_', "-");
// with:
let bin_name_for_asset = normalize_bin_name(&bin_name);
```

---

## Verification Checklist

### Before Any PR Merge
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test -p <crate>` passes for affected crate
- [ ] `cargo clippy -p <crate> --tests` shows no new warnings
- [ ] Manual testing for CLI changes

### After Each PR Merge
- [ ] `cargo build --workspace` still succeeds
- [ ] `cargo test --workspace` passes
- [ ] GitHub issue closed with descriptive comment

### Final Verification
- [ ] Clean clone builds in < 5 minutes
- [ ] All P0 issues closed
- [ ] All P1 PRs merged
- [ ] No regressions in existing tests

---

## Risk Mitigation

| Risk | Action |
|------|--------|
| PR has conflicts | Rebase: `git rebase main` |
| Tests fail after merge | Revert: `git revert <commit>` |
| Build still fails | Check if RLM feature needs enabling |
| Auto-update still broken | Check actual GitHub release names |

---

## Success Metrics

- Day 1: Clean clone builds successfully
- Day 2: 2 PRs merged, tests passing
- Day 3: All P0/P1 items complete, issues closed
- End: Development unblocked, contributors can build

---

## Related Documents

- Full Research: [research-priority-issues.md](research-priority-issues.md)
- Full Design: [design-priority-issues.md](design-priority-issues.md)
- Research Document: `.docs/research-priority-issues.md`
- Design Document: `.docs/design-priority-issues.md`

---

## Questions?

See the full design document for:
- Detailed architecture decisions
- Complete test strategy
- Rollback procedures
- API specifications
- Performance considerations
