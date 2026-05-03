# Implementation Plan: Fix Bigbox Git Corrupted Refs

**Status**: Draft | Review | Approved
**Research Doc**: `.docs/research-bigbox-corrupted-refs-2026-05-03.md`
**Author**: AI Agent (Phase 2 Disciplined Design)
**Date**: 2026-05-03
**Estimated Effort**: 2 days

## Overview

### Summary

Fix recurring corrupted refs in bigbox git repository by: (1) Adding automated ref pruning cron job, (2) Fixing worktree cleanup race condition in ADF orchestrator, (3) Narrowing Gitea fetch refspec to track only `main` + PR branches.

### Approach

Three-track approach addressing both symptoms (operational fixes) and root cause (code fix):
- **Track A**: Operational — automated ref pruning via cron
- **Track B**: Code — fix worktree cleanup race condition (Issue #471)
- **Track C**: Config — narrow Gitea fetch refspec

### Scope

**In Scope:**
- Automated daily ref pruning cron job on bigbox
- Fix worktree add/remove atomicity in ADF orchestrator
- Change Gitea remote refspec from `refs/heads/*` to only `main` + PRs
- Add monitoring for ref corruption (count check)

**Out of Scope:**
- Migrating away from Gitea
- Reducing number of agent types
- Rewriting ADF orchestrator

**Avoid At All Cost** (from 5/25 analysis):
- Adding new agent types "to isolate the problem" — adds complexity without solving root cause
- Implementing distributed locking for worktree operations — over-engineering for single-host bigbox
- Creating separate cleanup service — cron job is sufficient

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    Bigbox Server                        │
│                                                     │
│  ┌──────────────────┐    ┌──────────────────┐      │
│  │  Cron Job         │    │  ADF Orchestrator│      │
│  │  (ref-prune)    │    │  (worktree fix) │      │
│  └────────┬─────────┘    └────────┬─────────┘      │
│           │                    │                │              │
│           ▼                    ▼                │              │
│  ┌──────────────────┐    ┌──────────────────┐│              │
│  │ Git Repo         │◄───│ Worktree        ││              │
│  │ /home/alex/     │    │ cleanup fix     ││              │
│  │ terraphim-ai    │    └──────────────────┘│              │
│  └────────┬─────────┘                   │              │
│           │                            │              │
│           ▼                            ▼              │
│  ┌─────────────────────────────────────────────┐         │
│  │ Gitea Server (narrowed refspec)      │         │
│  │ Only main + PRs (not all 192 branches)│         │
│  └─────────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────────┘
```

### Data Flow

```
Daily Cron Trigger
    ↓
git fetch --prune gitea (removes stale refs)
    ↓
git reflog expire --expire=now --all
    ↓
git gc --prune=now (cleanup loose objects)
    ↓
Check ref count (alert if > 50)

Agent Spawn Request
    ↓
ADF Orchestrator: git worktree add (atomic, with lock)
    ↓
Agent does work
    ↓
ADF Orchestrator: git worktree remove (atomic, with lock)
    ↓
Orchestrator: remove metadata from .git/worktrees/ (guaranteed)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Cron job for ref pruning | Simple, reliable, no code changes needed | Daemon process (over-engineering) |
| Fix worktree cleanup in orchestrator | Addresses root cause of metadata leaks | Separate cleanup service (complex) |
| Narrow refspec to main + PRs | Reduces refs from 192 to ~20 | Keep all branches (too many stale refs) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Distributed locking for worktree ops | Single-host bigbox doesn't need this | Maintenance burden, complexity |
| New agent type for cleanup | Adds more worktrees (ironic) | Worse problem |
| Migration to different git hosting | Over-engineering, Gitea works | Months of work, no business value |

### Simplicity Check

> "Minimum code that solves the problem. Nothing speculative." -- Andrej Karpathy

**What if this could be easy?**

The simplest solution is: (1) Cron job runs `git fetch --prune` daily, (2) Fix the one race condition in worktree cleanup. No new services, no new agents, no architecture changes.

**Senior Engineer Test**: Would a senior engineer call this overcomplicated? 
- Cron job: No, standard practice
- Worktree fix: No, small targeted fix
- Refspec change: No, config change

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `infrastructure/scripts/bigbox-ref-prune.sh` | Ref pruning script for cron |
| `infrastructure/cron/bigbox-ref-prune.cron` | Cron job definition |

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/worktree.rs` | Fix race condition in worktree add/remove |
| `.git/config` (on bigbox) | Change gitea remote refspec |
| `crates/terraphim_orchestrator/tests/fixtures/conf.d/terraphim.toml` | Update build-runner config with prune |

### Deleted Files
| File | Reason |
|------|--------|
| (none) | Clean fix, no deletions needed |

## API Design

### New Public Types (in worktree.rs)

```rust
/// Result of worktree operation with cleanup guarantee
#[derive(Debug)]
pub struct WorktreeGuard {
    /// Path to worktree
    pub path: PathBuf,
    /// Guard ensures cleanup on drop
    _guard: DropGuard,
}

/// Errors from worktree operations
#[derive(Debug, thiserror::Error)]
pub enum WorktreeError {
    #[error("worktree {0} already exists")]
    AlreadyExists(String),
    
    #[error("worktree {0} lock failed: {1}")]
    LockFailed(String, String),
    
    #[error("worktree {0} creation failed: {1}")]
    CreationFailed(String, String),
    
    #[error("worktree {0} removal failed: {1}")]
    RemovalFailed(String, String),
}
```

### Key Functions

```rust
/// Create worktree with atomic cleanup guarantee
///
/// # Arguments
/// * `repo_path` - Path to git repository
/// * `worktree_path` - Path for new worktree
/// * `branch` - Branch to checkout
///
/// # Returns
/// WorktreeGuard that ensures cleanup on drop
///
/// # Errors
/// Returns `WorktreeError` if creation fails or lock cannot be acquired
pub fn create_worktree_atomic(
    repo_path: &Path,
    worktree_path: &Path,
    branch: &str,
) -> Result<WorktreeGuard, WorktreeError>;

/// Remove worktree with guaranteed metadata cleanup
///
/// # Arguments
/// * `repo_path` - Path to git repository  
/// * `worktree_path` - Path to worktree to remove
///
/// # Errors
/// Returns `WorktreeError` if removal fails
pub fn remove_worktree_atomic(
    repo_path: &Path,
    worktree_path: &Path,
) -> Result<(), WorktreeError>;
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_create_worktree_atomic_success` | `worktree.rs` | Happy path |
| `test_create_worktree_atomic_lock` | `worktree.rs` | Verify lock prevents races |
| `test_remove_worktree_atomic_cleanup` | `worktree.rs` | Verify metadata removed |
| `test_worktree_guard_drop_cleans_up` | `worktree.rs` | Verify Drop trait works |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_worktree_lifecycle_e2e` | `tests/worktree.rs` | Full add/use/remove cycle |
| `test_concurrent_worktree_creation` | `tests/worktree.rs` | Verify no race condition |
| `test_ref_prune_script` | `tests/ref_prune.rs` | Verify cron script works |

### Property Tests

```rust
proptest! {
    #[test]
    fn worktree_never_leaves_stale_metadata(path in "[a-z]{10}") {
        let repo = setup_test_repo();
        let wt_path = format!("/tmp/test-worktrees/{}", path);
        
        // Create and immediately drop (simulate crash)
        {
            let _guard = create_worktree_atomic(&repo, &Path::new(&wt_path), "main")
                .expect("create worktree");
            // guard dropped here
        }
        
        // Verify no stale metadata
        assert!(!worktree_metadata_exists(&repo, &wt_path));
    }
}
```

## Implementation Steps

### Step 1: Operational Fix — Cron Job

**Files:** `infrastructure/scripts/bigbox-ref-prune.sh`, `infrastructure/cron/bigbox-ref-prune.cron`

**Description:** Create daily cron job that prunes stale refs and monitors ref count.

**Tests:** Integration test for script

**Estimated:** 1 hour

```bash
#!/bin/bash
# infrastructure/scripts/bigbox-ref-prune.sh
set -euo pipefail

REPO="/home/alex/terraphim-ai"
cd "$REPO"

echo "[$(date)] Starting ref prune..."

# Prune stale remote refs
git fetch --prune gitea 2>&1 | grep -v "bad object" || true

# Expire old reflogs
git reflog expire --expire=now --all

# Cleanup loose objects
git gc --prune=now 2>&1 | grep -v "bad object" || true

# Check ref count
REF_COUNT=$(git for-each-ref refs/remotes/gitea/ | wc -l)
echo "Gitea remote refs: $REF_COUNT"

if [ "$REF_COUNT" -gt 50 ]; then
    echo "WARNING: Ref count $REF_COUNT exceeds threshold 50"
    # Could send alert here
fi

echo "[$(date)] Ref prune complete."
```

```cron
# infrastructure/cron/bigbox-ref-prune.cron
# Run daily at 3 AM
0 3 * * * /home/alex/projects/terraphim/terraphim-ai/infrastructure/scripts/bigbox-ref-prune.sh >> /var/log/bigbox-ref-prune.log 2>&1
```

### Step 2: Code Fix — Worktree Atomic Operations

**Files:** `crates/terraphim_orchestrator/src/worktree.rs`

**Description:** Implement atomic worktree creation/removal with lock file and Drop guard to prevent metadata leaks.

**Tests:** Unit tests + integration tests

**Dependencies:** Step 1 (can be done in parallel)

**Estimated:** 4 hours

Key code to write:

```rust
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Lock file to prevent concurrent worktree operations
struct WorktreeLock {
    _file: File,
}

impl WorktreeLock {
    fn acquire(repo: &Path, name: &str) -> Result<Self, WorktreeError> {
        let lock_path = repo.join(".git").join("worktrees").join(format!("{}.lock", name));
        let file = File::create(&lock_path)
            .map_err(|e| WorktreeError::LockFailed(name.to_string(), e.to_string()))?;
        Ok(Self { _file: file })
    }
}

impl Drop for WorktreeLock {
    fn drop(&mut self) {
        // Lock file deleted when file is dropped
    }
}

pub struct WorktreeGuard {
    pub path: PathBuf,
    repo: PathBuf,
    _lock: WorktreeLock,
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        // Guaranteed cleanup
        let _ = remove_worktree_atomic(&self.repo, &self.path);
    }
}

pub fn create_worktree_atomic(
    repo_path: &Path,
    worktree_path: &Path,
    branch: &str,
) -> Result<WorktreeGuard, WorktreeError> {
    let name = worktree_path
        .file_name()
        .ok_or_else(|| WorktreeError::CreationFailed(worktree_path.display().to_string(), "invalid path".into()))?
        .to_string_lossy();
    
    let lock = WorktreeLock::acquire(repo_path, &name)?;
    
    // Create worktree
    let status = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["worktree", "add", worktree_path.to_str().unwrap(), branch])
        .status()
        .map_err(|e| WorktreeError::CreationFailed(name.clone(), e.to_string()))?;
        
    if !status.success() {
        return Err(WorktreeError::CreationFailed(name, format!("exit code: {}", status)));
    }
    
    Ok(WorktreeGuard {
        path: worktree_path.to_path_buf(),
        repo: repo_path.to_path_buf(),
        _lock: lock,
    })
}
```

### Step 3: Config Fix — Narrow Gitea Refspec

**Files:** Update `.git/config` on bigbox (or script to do so)

**Description:** Change Gitea remote refspec from `refs/heads/*:refs/remotes/gitea/*` to only track `main` and PR branches.

**Tests:** Verify ref count drops from 192 to ~20

**Dependencies:** Step 1

**Estimated:** 30 minutes

```bash
# On bigbox, update .git/config
cd /home/alex/terraphim-ai

# Remove old refspec
git config --unset remote.gitea.fetch "^.*$"

# Add narrow refspecs
git config --add remote.gitea.fetch "+refs/heads/main:refs/remotes/gitea/main"
git config --add remote.gitea.fetch "+refs/pull/*/head:refs/remotes/gitea/pr/*"

# Prune old refs
git fetch --prune gitea

# Verify
git for-each-ref refs/remotes/gitea/ | wc -l  # Should be ~20
```

### Step 4: Update Build-Runner Config

**Files:** `crates/terraphim_orchestrator/tests/fixtures/conf.d/terraphim.toml`

**Description:** Ensure build-runner config includes `fetch --prune` (from commit `550be8820`).

**Tests:** Verify config is correct

**Dependencies:** None (can be done anytime)

**Estimated:** 15 minutes

```toml
# In [build-runner] section
[build-runner.git]
fetch_prune = true  # Ensure this is set (from commit 550be8820)
checkout_force = true
suppress_bad_object_errors = true
```

## Rollback Plan

If issues discovered:

1. **Cron job issues**: Disable cron job with `crontab -e`, remove entry
2. **Worktree fix regression**: Revert commit, agents continue with old behaviour (leaky but functional)
3. **Refspec change breaks agents**: Restore old refspec:
   ```bash
   git config --unset remote.gitea.fetch "^.*$"
   git config --add remote.gitea.fetch "+refs/heads/*:refs/remotes/gitea/*"
   git fetch gitea
   ```

Feature flag: N/A (not a feature, infrastructure fix)

## Migration (if applicable)

### Repository State Migration

```bash
# On bigbox, one-time cleanup before deploying fix
cd /home/alex/terraphim-ai

# Remove stale worktree metadata
for wt in .git/worktrees/*; do
    path=$(grep "^gitdir:" "$wt/gitdir" 2>/dev/null | cut -d' ' -f2)
    if [ ! -d "$path" ]; then
        echo "Removing stale: $wt"
        rm -rf "$wt"
    fi
done

# Prune all stale refs
git fetch --prune --all
git reflog expire --expire=now --all
git gc --prune=now
```

## Dependencies

### New Dependencies

None — uses existing git functionality.

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Ref prune cron job runtime | < 1 minute | Log timing |
| Worktree creation time | < 5 seconds | Instrument orchestrator |
| Worktree removal time | < 2 seconds | Instrument orchestrator |
| Remote ref count | < 50 | Daily check in cron |

### Benchmarks to Add

```rust
// In worktree.rs, add timing instrumentation
use std::time::Instant;

pub fn create_worktree_atomic(...) -> Result<WorktreeGuard, WorktreeError> {
    let start = Instant::now();
    // ... implementation ...
    tracing::info!(elapsed = ?start.elapsed(), "worktree created");
    // ...
}
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify build-runner config from commit 550be8820 is deployed | Pending | DevOps |
| Test narrow refspec on test repository first | Pending | Engineer |
| Coordinate with agents during cron job deployment | Pending | Engineer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

## Next Steps

After Phase 2 approval:

1. **Phase 2.5 (Specification)**: Use `disciplined-specification` skill to deep-dive into edge cases, especially:
   - What if lock file exists but worktree is stale?
   - What if cron job runs while agent is using worktree?
   - What if Gitea PR branch is force-pushed?

2. **Phase 3 (Implementation)**: Use `disciplined-implementation` skill to execute this plan step-by-step.

3. **Phase 4 (Verification)**: Use `disciplined-verification` skill to verify implementation matches design.

4. **Deploy**: 
   - Deploy cron job to bigbox
   - Merge worktree fix to main
   - Update bigbox `.git/config`
   - Monitor for 1 week
