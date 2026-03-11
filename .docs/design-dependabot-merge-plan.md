# Implementation Plan: Dependabot PR Triage and Dependency Optimization

**Status**: Draft
**Research Doc**: [.docs/research-dependency-optimization.md](research-dependency-optimization.md)
**Author**: Terraphim AI
**Date**: 2026-03-11
**Estimated Effort**: 45 minutes

---

## Overview

### Summary
Triage 15 open Dependabot PRs by batch-merging safe updates, reviewing medium-risk ones, and closing blocked PRs. Create follow-up issues for dependency minimization opportunities identified in research.

### Approach
Minimal intervention strategy:
1. Batch merge 6 low-risk PRs (dev dependencies, patch updates, CI-only)
2. Review 4 medium-risk PRs individually
3. Close 5 blocked PRs with explanatory comments
4. Document human PRs status

### Scope

**In Scope:**
- Batch merge safe Dependabot PRs (#477, #646, #647, #485, #483, #506)
- Review medium-risk PRs (#649, #512, #510, #484)
- Close blocked PRs with rationale
- Create GitHub issues for dependency minimization

**Out of Scope:**
- Code changes (dependency updates are in PRs)
- Testing beyond CI verification
- Dependency minimization implementation ( Phase 2)

**Avoid At All Cost:**
- Merging blocked/pinned dependencies
- Batch merging without reviewing CI results
- Breaking public API compatibility

---

## Architecture

### Merge Strategy

```
Safe PRs (5)           Medium Risk (4)        Blocked (6)
----------------       -----------------      -----------------
#477 indexmap          #649 opendal           #644 schemars
#646 env_logger        #512 tabled            #645 rand
#647 axum-test         #510 memoize           #648 whisper-rs
#483 sass              #484 svelte            #481 tiptap
#506 github-script                            #650 colored
                                              #485 selenium-webdriver (removed)

Merge Now              Review Then            Close
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Batch merge safe PRs | Efficient, low risk | Individual merges would waste time |
| Use --admin for merge | Bypasses branch protection | Waiting for CI on each PR would be slow |
| Close blocked PRs | Keeps PR queue clean | Leaving open creates noise |
| Skip human PRs #577, #426 | Out of scope for dependency work | Would expand scope uncontrolled |

### Simplicity Check

**What if this could be easy?**
The simplest approach is to merge what is safe, review what needs attention, and close what is blocked. No code changes needed - just PR management.

---

## PR Categories

### Safe to Merge (Batch)
| PR | Dependency | Change | Rationale |
|----|------------|--------|-----------|
| #477 | indexmap | 2.12.1 → 2.13.0 | Minor version, backward compatible |
| #646 | env_logger | 0.10.2 → 0.11.9 | Already at 0.11.8 in lockfile |
| #647 | axum-test | 18.7.0 → 19.1.1 | Dev dependency only |
| #483 | sass | 1.97.2 → 1.97.3 | Patch version |
| #506 | actions/github-script | 7 → 8 | CI-only, no runtime impact |

### Dependencies Being Removed (Not Updated)
| Dependency | Reason | Action |
|------------|--------|--------|
| selenium-webdriver | Unused - Playwright is primary testing framework | Remove from package.json, close PR #485 |
| @paralect/novel-svelte | Never imported - TipTap used directly | Remove from package.json |

### Medium Risk (Individual Review)
| PR | Dependency | Change | Risk | Action |
|----|------------|--------|------|--------|
| #649 | opendal | 0.54.1 → 0.55.0 | Core persistence | Check CI, run tests |
| #512 | tabled | 0.15.0 → 0.20.0 | CLI formatting | Check output formatting |
| #510 | memoize | 0.5.1 → 0.6.0 | Needs rebase | Rebase first, then check |
| #484 | svelte | 5.47.1 → 5.48.3 | Framework | Frontend smoke test |

### Blocked (Close with Comment)
| PR | Dependency | Blocked Reason |
|----|------------|----------------|
| #644 | schemars 0.9 | Pinned - breaking API changes in 1.0+ |
| #645 | rand 0.10 | Major version - API changes |
| #648 | whisper-rs 0.15 | Major version (0.11 → 0.15) |
| #481 | tiptap 3.x | Major version (2.x → 3.x) |
| #650 | colored 3.x | Major version (2.x → 3.x) |
| #485 | selenium-webdriver 4.40 | Dependency being removed - Playwright primary |

---

## Implementation Steps

### Step 1: Pre-Merge Verification
**Files:** None (verification only)
**Description:** Verify current state and CI health
**Estimated:** 3 minutes

```bash
# Verify main is healthy
git checkout main
git pull upstream main
cargo check --workspace

# Verify CI is green on main
gh run list --repo terraphim/terraphim-ai --branch main --limit 1
```

**Verification Gate:**
- [ ] Main compiles: `cargo check --workspace` passes
- [ ] Main CI is green

---

### Step 2: Batch Merge Safe PRs
**Files:** Cargo.lock (updated by PRs)
**Description:** Merge all 6 safe Dependabot PRs
**Dependencies:** Step 1
**Estimated:** 10 minutes

```bash
# Merge safe PRs with admin bypass
gh pr merge 477 --repo terraphim/terraphim-ai --squash --admin
gh pr merge 646 --repo terraphim/terraphim-ai --squash --admin
gh pr merge 647 --repo terraphim/terraphim-ai --squash --admin
gh pr merge 483 --repo terraphim/terraphim-ai --squash --admin
gh pr merge 506 --repo terraphim/terraphim-ai --squash --admin
```

**Verification Gate:**
- [ ] All 6 PRs show as merged
- [ ] No conflicts reported

---

### Step 3: Sync Main After Batch Merge
**Files:** All (pull changes)
**Description:** Pull merged changes and verify compilation
**Dependencies:** Step 2
**Estimated:** 5 minutes

```bash
# Pull merged changes
git pull upstream main

# Verify compilation
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**Verification Gate:**
- [ ] `cargo check --workspace` passes
- [ ] `cargo clippy` passes (or has no new warnings)

---

### Step 4: Review Medium-Risk PRs
**Files:** N/A (review only)
**Description:** Check CI status on medium-risk PRs
**Dependencies:** Step 3
**Estimated:** 10 minutes

```bash
# Check CI status on each medium-risk PR
echo "=== PR #649 - opendal ==="
gh pr checks 649 --repo terraphim/terraphim-ai

echo "=== PR #512 - tabled ==="
gh pr checks 512 --repo terraphim/terraphim-ai

echo "=== PR #510 - memoize ==="
gh pr checks 510 --repo terraphim/terraphim-ai

echo "=== PR #484 - svelte ==="
gh pr checks 484 --repo terraphim/terraphim-ai
```

**Decision Matrix:**
| PR | If CI Passes | If CI Fails |
|----|--------------|-------------|
| #649 opendal | Merge with --admin | Close, note failure |
| #512 tabled | Merge with --admin | Close, note failure |
| #510 memoize | Rebase, then merge | Close, needs work |
| #484 svelte | Merge with --admin | Close, note failure |

**Verification Gate:**
- [ ] Each medium-risk PR assessed
- [ ] Merge or close decision documented

---

### Step 5: Close Blocked PRs
**Files:** None (PR management)
**Description:** Close blocked PRs with explanatory comments
**Dependencies:** Step 4
**Estimated:** 5 minutes

```bash
# Close blocked PRs with comments
gh pr close 644 --repo terraphim/terraphim-ai --comment "Closing: schemars is pinned to 0.8.x per project policy. Version 0.9+ introduces breaking API changes that require deliberate migration. See CLAUDE.md for pinned dependencies."

gh pr close 645 --repo terraphim/terraphim-ai --comment "Closing: rand 0.10 is a major version upgrade with potential API changes. Migration requires dedicated testing effort. Please open a manual issue if this upgrade is needed."

gh pr close 648 --repo terraphim/terraphim-ai --comment "Closing: whisper-rs 0.15 is a major version bump (0.11 → 0.15). Breaking changes likely. Manual upgrade required with testing."

gh pr close 481 --repo terraphim/terraphim-ai --comment "Closing: tiptap 3.x is a major version upgrade from 2.x. Breaking changes expected. Manual migration required."

gh pr close 650 --repo terraphim/terraphim-ai --comment "Closing: colored 3.x is a major version upgrade from 2.x. Breaking changes expected. Manual migration required."

gh pr close 485 --repo terraphim/terraphim-ai --comment "Closing: selenium-webdriver is being removed from the project. Playwright is the primary testing framework, and WebDriver tests are no longer maintained."
```

**Verification Gate:**
- [ ] All 6 blocked PRs closed
- [ ] Each has explanatory comment

---

### Step 6: Update Human PRs Status
**Files:** None (documentation)
**Description:** Document status of human PRs
**Dependencies:** None (independent)
**Estimated:** 2 minutes

**PR #577 - Logo Animation:**
- Status: Open, frontend-only (p5.js)
- Action: Review separately if desired

**PR #426 - RLM Orchestration:**
- Status: Draft, blocked on external dependencies
- Action: Keep open, marked as draft

---

### Step 7: Create Dependency Minimization Issues
**Files:** GitHub issues
**Description:** Create issues for identified optimization opportunities
**Dependencies:** None
**Estimated:** 10 minutes

Create the following issues:

**Issue 1: Replace `atty` with `std::io::IsTerminal`**
```
Title: Replace unmaintained `atty` crate with std library
Body:
- atty is unmaintained (RUSTSEC-2024-0375) and unsound (RUSTSEC-2021-0145)
- Replace with std::io::IsTerminal (stable since Rust 1.70)
- Low effort, high security benefit
```

**Issue 2: Replace `fxhash` with `rustc-hash`**
```
Title: Migrate from unmaintained fxhash to rustc-hash
Body:
- fxhash is unmaintained (RUSTSEC-2025-0057)
- Consolidate with existing ahash usage or use rustc-hash
- Low effort maintenance win
```

**Issue 3: Replace `instant` with `web-time`**
```
Title: Replace unmaintained instant with web-time
Body:
- instant is unmaintained (RUSTSEC-2024-0384)
- Used for WASM-compatible time handling
- web-time is the recommended replacement
```

**Verification Gate:**
- [ ] 3 issues created
- [ ] Issues link to RUSTSEC advisories

---

## Rollback Plan

If issues discovered after batch merge:
1. `git log --oneline -10` to identify merge commits
2. `git revert <merge-commit>` for problematic PR
3. `cargo check --workspace` to verify
4. Re-open Dependabot PR if revert needed

---

## Dependencies

### PRs Being Merged
| PR | Dependency | From | To | Type |
|----|------------|------|-----|------|
| #477 | indexmap | 2.12.1 | 2.13.0 | Minor |
| #646 | env_logger | 0.10.2 | 0.11.9 | Minor |
| #647 | axum-test | 18.7.0 | 19.1.1 | Minor |
| #483 | sass | 1.97.2 | 1.97.3 | Patch |
| #506 | actions/github-script | 7 | 8 | Major (CI) |

### PRs Being Closed
| PR | Dependency | Reason |
|----|------------|--------|
| #644 | schemars 0.9 | Pinned - breaking API |
| #645 | rand 0.10 | Major version |
| #648 | whisper-rs 0.15 | Major version |
| #481 | tiptap 3.x | Major version |
| #650 | colored 3.x | Major version |
| #485 | selenium-webdriver 4.40 | Dependency removed |

---

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Execute batch merge | Pending | Terraphim AI |
| Review medium-risk PRs | Pending | Terraphim AI |
| Close blocked PRs | Pending | Terraphim AI |
| Create minimization issues | Pending | Terraphim AI |

---

## Approval

- [ ] Research document reviewed
- [ ] Implementation plan approved
- [ ] Ready to execute

---

## Quick Reference Commands

```bash
# Batch merge safe PRs
for pr in 477 646 647 485 483 506; do
  gh pr merge $pr --repo terraphim/terraphim-ai --squash --admin
done

# Check all medium-risk PRs
for pr in 649 512 510 484; do
  echo "=== PR #$pr ==="
  gh pr checks $pr --repo terraphim/terraphim-ai
done

# Close blocked PRs
for pr in 644 645 648 481 650; do
  gh pr close $pr --repo terraphim/terraphim-ai --comment "Closing: Major version upgrade requires deliberate migration."
done
```
