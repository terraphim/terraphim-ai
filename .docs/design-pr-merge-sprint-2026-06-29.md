# Implementation Plan: PR Merge Sprint 2026-06-29

**Status**: Draft
**Research Doc**: `.docs/research-pr-merge-sprint-2026-06-29.md`
**Author**: AI Agent
**Date**: 2026-06-29
**Estimated Effort**: 2-3 hours (mostly waiting on CI)

## Overview

### Summary
Merge ~50 PRs from the open backlog, fixing fmt/clippy blockers on main first, then merging in dependency-aware order starting with CI infrastructure gates, followed by quick fixes, test additions, and finally feature PRs.

### Approach
1. Fix fmt/clippy on main (direct push, unblocks all CI gates)
2. Merge CI gate PRs (builds guardrails for future merges)
3. Tier-based merge: simple -> moderate -> complex
4. Resolve conflicts on 2 PRs
5. Close stale/noise ADF issues

### Scope

**In Scope:**
- Fix fmt violations on main (2 files)
- Fix clippy warnings on main (1 file, 3 lines)
- Merge ~40 Tier 1-3 PRs
- Merge ~10 Tier 5 CI gate PRs
- Handle 2 conflicting PRs (#2970, #2969)
- Push to both remotes
- Close duplicate ADF monitor-noise issues

**Out of Scope:**
- New feature development
- Merging Tier 6 feature PRs without review
- Full workspace test suite run (would take hours)
- Fixing all 253 ready issues

**Avoid At All Cost:**
- Merging PRs that reference wrong issues
- Bypassing CI checks
- Rebasing PRs that would introduce conflicts with others
- Creating new branches to fix PR conflicts manually (let agents fix their own)

## Architecture

### Merge Dependency Graph
```
[Fix fmt+clippy on main]
         |
    [CI Gate PRs] ───── Tier 5
         |
    [Tier 1: Quick Fixes] ───── 1-line changes, no deps
         |
    [Tier 2: Confident Merges] ── already mergeable=True
         |
    [Tier 3: Test Additions] ─── single-crate tests
         |
    [Fix #2970, #2969 conflicts]
         |
    [Tier 6: Feature PRs] ────── requires review
```

### Key Design Decisions
| Decision | Rationale |
|----------|-----------|
| Fix main first, not via PR | fmt/clippy fixes are 5 lines total; a PR would need its own CI gate which is circular |
| Merge CI gate PRs early | Without gates, merging other PRs is blind; gates prevent regression |
| Merge oldest/lowest PR # first | Minimises merge conflict probability |
| Don't rebase conflicting PRs | Original author should rebase; we close with comment asking for rebase |

## File Changes

### Step 1: Fix fmt on main
**Files:** `crates/terraphim_rlm/src/mcp_tools.rs`, `crates/terraphim_tinyclaw/src/main.rs`
**Description:** Run `cargo fmt` on the 2 unformatted files
**Verification:** `cargo fmt --all -- --check` exits 0

### Step 2: Fix clippy on main
**Files:** `crates/terraphim_rlm/src/mcp_tools.rs`
**Description:** Fix 3 clippy warnings: doc_lazy_continuation (2x indent), redundant_closure (replace closure with fn)
**Verification:** `cargo clippy --workspace` has 0 warnings

### Step 3: Merge CI Gate PRs (Tier 5)
| PR | Title |
|----|-------|
| #2955 | cargo audit CI gate |
| #2954 | CI rust-clippy + rust-compile jobs |
| #2942 | workspace test execution gate |
| #2939 | compile gate ci |
| #2960 | host-runners workspace |

### Step 4: Merge Tier 1 Quick Fixes
| PR | Title | Lines |
|----|-------|-------|
| #3018 | fmt gate (3 crates) | ~5 |
| #3012 | clippy mcp_tools | ~3 |
| #2968 | remove dangling meta_coordinator | 1 |
| #2966 | remove assert!(true) | 1 |

### Step 4b: Merge Tier 2 Confident PRs
| PR | Title |
|----|-------|
| #3032 | Gitea health check source profile |
| #3031 | auto-merge gate log enhancement |
| #3026 | auto-merge agent allowlist |
| #3011 | rlm set_var safety comment |
| #3007 | Ed25519 key confirmation |
| #3002 | worktree disk dedup |
| #3001 | flaky repro profile |
| #3000 | per-test timeout |
| #2993 | OnceLock redaction |
| #2963 | flaky port fix |
| #2948 | ADF meta-coordinator health check |
| #2943 | D002+D004 clippy |
| #2938 | clippy merge_coordinator/weather_report |

### Step 4c: Merge Tier 3 Test PRs
| PR | Title |
|----|-------|
| #2985 | github_runner unit tests |
| #2984 | PrFile deserialization + tests |
| #2979 | relocate stranded specs |
| #2977 | weather_report tests |
| #2976 | tinyclaw core tests |
| #2974 | remove orphaned agent source |
| #2962 | validation unit tests |
| #2957 | gitea_runner tests |
| #2952 | validation unit tests (dup?) |
| #2946 | spawner validator tests |
| #2926 | workspace archive tests |
| #2925 | RLM strictness tests |
| #2923 | tinyclaw tests |
| #2918 | Atlassian email redact |
| #2917 | OpenRouter runtime fallback |
| #2915 | Homebrew placeholder cleanup |
| #2913 | set_var data race fix |
| #2912 | KG fixture on-disk tests |
| #2910 | workspace test compile blocker |
| #2908 | tinyclaw safety comment |
| #2903 | DSM unit tests |
| #2902 | KG validation executors |

## Merge Procedure

For each PR, the following sequence:

```bash
# 1. Fetch the PR branch
git fetch gitea pull/PRNUM/head:pr-PRNUM

# 2. Fast-forward check
git merge-base --is-ancestor origin/main pr-PRNUM && echo "Fast-forward" || echo "Needs rebase"

# 3. If fast-forward: merge directly
git checkout main
git merge pr-PRNUM --no-edit
git push origin main

# 4. If merge conflict: skip PR, note for later

# 5. Verify sync
git push gitea main
git diff origin/main gitea/main --stat
```

## Test Strategy

### Per-Step Verification
| Step | Verification |
|------|-------------|
| 1. Fix fmt | `cargo fmt --all -- --check` exits 0 |
| 2. Fix clippy | `cargo clippy --workspace` exits 0 |
| 3-4. CI gate PRs | Verify CI workflow files valid, no syntax errors |
| 4b-c. Tier 2-3 | `cargo check --workspace` after each batch |
| 5. Conflicting PRs | Skip; comment asking author to rebase |
| After all merges | `git diff origin/main gitea/main --stat` empty |

## Rollback Plan

If merge introduces breakage:
1. `git revert` the problematic merge commit
2. Push revert to both remotes
3. Investigate root cause before re-attempting

## Implementation Steps

### Step 1: Fix fmt and clippy on main (10 min)
**Files:** `crates/terraphim_rlm/src/mcp_tools.rs`, `crates/terraphim_tinyclaw/src/main.rs`
**Actions:**
1. Run `cargo fmt` to fix formatting
2. Fix 3 clippy warnings in mcp_tools.rs
3. Verify: `cargo fmt --all -- --check && cargo clippy --workspace`
4. Commit: `fix: fmt and clippy on main`

### Step 2: Merge CI Gate PRs (15 min)
**Targets:** #2955, #2954, #2942, #2939, #2960
**Actions:** Fetch each, verify fast-forward, merge, verify build

### Step 3: Merge Tier 1 Quick Fixes (10 min)
**Targets:** #3018, #3012, #2968, #2966

### Step 4: Merge Tier 2 Confident PRs (20 min)
**Targets:** 13 PRs listed above

### Step 5: Merge Tier 3 Test PRs (30 min)
**Targets:** 20+ PRs listed above

### Step 6: Handle Conflicts (10 min)
**Targets:** #2970, #2969
**Action:** Comment asking author to rebase on latest main

### Step 7: Final Verification and Push (5 min)
**Actions:**
1. `git diff origin/main gitea/main --stat` - verify empty
2. Close stale ADF monitor-noise issues

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Why are 3 new branches empty? | Pending investigation | Agent |
| Should ADF duplicate issues be batch-closed? | Yes, post-merge | Agent |
| Are Tier 6 feature PRs worth merging? | Deferred | Human review |

## Approval

- [ ] Technical review complete
- [ ] Merge order approved
- [ ] Human approval received
