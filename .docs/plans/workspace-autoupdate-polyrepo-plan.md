# Research and Implementation Plan: Workspace Recovery, Auto-Update Restoration, and Polyrepo Completion

**Status**: Draft
**Author**: AI Assistant
**Date**: 2026-06-08
**Scope**: 3 interconnected work items

---

## Part 1: Disciplined Research

### Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Unblocks v1.20.4 release and restores CI/CD pipeline |
| Leverages strengths? | Yes | Builds on existing polyrepo-publish pipeline and release infrastructure |
| Meets real need? | Yes | Workspace is broken, auto-update is disabled, 3 repos are stuck mid-publish |

**Proceed**: Yes

### Problem Statement

The Terraphim AI monorepo is in a partially-migrated state after the Gitea #1910 polyrepo split. Three critical issues block the next release:

1. **Workspace Build Broken**: `cargo check --workspace` fails because 8 crate directories were partially moved to polyrepo splits but are still matched by the `crates/*` workspace glob. These directories no longer contain `Cargo.toml`.

2. **Auto-Update Disabled**: `terraphim_server` has `check-update` / `update` CLI flags that exit with "Auto-update feature temporarily disabled". The `terraphim_update` crate was moved out of the workspace and is only available in a Git worktree.

3. **Polyrepo Publish Incomplete**: 3 of 6 split repos are stuck:
   - `terraphim-clients`: GitHub CI passes, but `cargo publish` fails due to `publish = ["terraphim"]` restriction
   - `terraphim-agents`: GitHub CI fails with compilation error (`terraphim_spawner::SpawnContext` API mismatch)
   - `terraphim-kg-agents`: Blocked behind terraphim-agents

### Current State Analysis

#### Workspace Structure

```
crates/                          # 24 directories total
├── haystack_atlassian           # OK
├── haystack_discourse           # OK
├── terraphim_agent              # EMPTY (no Cargo.toml)
├── terraphim_agent_application  # OK but depends on moved crates
├── terraphim_agent_evolution    # EMPTY
├── terraphim_automata           # EMPTY
├── terraphim_automata_py        # OK
├── terraphim_build_args         # OK
├── terraphim_cli                # EMPTY
├── terraphim_dsm                # OK
├── terraphim_gitea_runner       # OK
├── terraphim_github_runner      # OK but depends on moved crates
├── terraphim_github_runner_server # OK but depends on moved crates
├── terraphim_merge_coordinator  # OK
├── terraphim_multi_agent        # EMPTY
├── terraphim_orchestrator       # EMPTY
├── terraphim_rlm                # OK but depends on moved crates
├── terraphim_rolegraph_py       # OK
├── terraphim_service            # EMPTY
├── terraphim_settings           # EMPTY
├── terraphim_symphony           # OK
├── terraphim_tinyclaw           # OK
├── terraphim_validation         # OK
└── terraphim_workspace          # OK
```

#### Crates with Dependencies on Moved Crates

| Crate | Depends On | Resolution Strategy |
|-------|------------|---------------------|
| `terraphim_agent_application` | terraphim_agent_supervisor, terraphim_agent_messaging, terraphim_agent_registry | Already excluded from workspace |
| `terraphim_automata_py` | terraphim_automata | Already excluded |
| `terraphim_github_runner` | terraphim_multi_agent, terraphim_agent_evolution, terraphim_service | Already excluded |
| `terraphim_github_runner_server` | terraphim_service | Already excluded |
| `terraphim_rlm` | terraphim_service, terraphim_automata, terraphim_agent_supervisor | Already excluded |
| `terraphim_rolegraph_py` | terraphim_automata | Already excluded |

#### Auto-Update Code Locations

| Component | Location | Status |
|-----------|----------|--------|
| CLI flag definition | `terraphim_server/src/main.rs:42-47` | Active but handler is stub |
| Update handler stub | `terraphim_server/src/main.rs:68-74` | Returns error "temporarily disabled" |
| Original update crate | `.worktrees/pr-security-sentinel-31531dbb/crates/terraphim_update/` | Exists in worktree only |
| Patched `self_update` | `Cargo.toml:81` | Already in `[patch.crates-io]` |

#### Polyrepo Publish Status

| Repo | GitHub URL | Gitea Sync | Gitea CI | GitHub CI | crates.io | Blocker |
|------|-----------|------------|----------|-----------|-----------|---------|
| terraphim-core | Created | Yes | Pass | Pass | Done | None |
| terraphim-config-persistence | Created | Yes | Pass | Pass | Done | None |
| terraphim-service | Created | Yes | Pass | Pass | Done | None |
| terraphim-agents | Created | Yes | Pass | **FAIL** | Not reached | `terraphim_spawner` API mismatch |
| terraphim-kg-agents | Created | Yes | Pass | **FAIL** | Not reached | Depends on terraphim-agents |
| terraphim-clients | Created | Yes | Pass | Pass | **FAIL** | `publish = ["terraphim"]` restriction |

### Constraints

#### Technical Constraints
- Workspace uses `members = ["crates/*", ...]` glob which matches all subdirectories
- Cannot remove empty directories entirely because some contain runtime config/settings files
- `terraphim_update` depends on `self_update` which is already patched in workspace
- Polyrepo splits must maintain topological dependency order for crates.io publishing
- GitHub repos have branch protection requiring PR merge-back

#### Business Constraints
- Must not break the 3 already-published repos
- Must maintain dual-remote sync (GitHub + Gitea)
- Release v1.20.4 is blocked until workspace builds

### Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| terraphim-agents API mismatch requires version bump + republish of terraphim-service | High | High | Plan explicit version bump step |
| crates.io API rate limiting during retry | Medium | Medium | Use authenticated API checks, add delays |
| terraphim_update restoration may have dependency conflicts | Medium | Medium | Test build after restoration |
| Branch protection prevents direct push of Cargo.toml fixes | Low | Medium | Use PR workflow |
| Empty crate directories contain untracked files that should not be deleted | Medium | Low | Audit before removal |

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong |
|------------|-------|---------------|
| Empty crate directories can be safely excluded from workspace | They contain only residual config/settings, not source | Cargo.toml already gone, so no build value |
| terraphim_update from worktree is the latest good version | It's the only surviving copy in the repo | May need updates to match current APIs |
| terraphim-agents compilation error is caused by stale `terraphim_spawner` on crates.io | Error message says method missing in published crate | May be a code bug in terraphim-agents itself |
| All 6 polyrepo splits should be published to crates.io | Pipeline was designed for this | Product decision may limit scope |

### Multiple Interpretations Considered

| Option | Implications | Chosen/Rejected |
|--------|--------------|-----------------|
| **A**: Exclude empty directories from workspace | Simplest, preserves residual files | **Chosen** for item 1 |
| **B**: Delete empty directories entirely | Cleaner but risk losing untracked files | Rejected - audit needed first |
| **C**: Restore moved crates to monorepo | Reverses polyrepo split | Rejected - contradicts #1910 strategy |
| **D**: Publish terraphim_update to crates.io | Clean separation, but adds publish step | Rejected for now - restore to workspace first |
| **E**: Copy terraphim_update back into workspace | Fastest path to re-enable auto-update | **Chosen** for item 2 |
| **F**: Bump terraphim_spawner minor version and republish | Correct semver path | **Chosen** for item 3 |
| **G**: Patch terraphim-agents to avoid `with_stderr_log` | Works around issue but hides API drift | Rejected - fix root cause |

### Key Insights

1. The workspace break is a **configuration issue**, not a code issue. The fix is purely in `Cargo.toml` exclusions.
2. Auto-update is a **dependency restoration** problem, not a logic rewrite. The crate exists; it just needs to be reintegrated.
3. The polyrepo blockers are **dependency ordering / API compatibility** issues, not pipeline issues. The pipeline itself is now robust.
4. All three items can be worked in parallel once the research is approved, but item 3 (polyrepo) has a dependency on item 1 (workspace) for local verification.

---

## Part 2: Disciplined Design

### Implementation Plan Overview

**Approach**: Three parallel workstreams with one shared verification gate.

**In Scope**:
- Fix `Cargo.toml` workspace exclusions
- Restore `terraphim_update` crate and re-enable server auto-update
- Complete terraphim-clients publish
- Fix terraphim-agents compilation and complete its publish
- Complete terraphim-kg-agents after terraphim-agents

**Out of Scope**:
- New auto-update features (just restoration)
- Desktop app auto-update (separate Tauri updater)
- Version bumping beyond what's needed to resolve API mismatch
- Refactoring polyrepo publish pipeline (it works)

**Avoid At All Cost**:
- Reversing the polyrepo split
- Major refactoring of terraphim_update architecture
- Manual crates.io uploads outside the pipeline
- Force-pushing to protected branches

---

### Item 1: Fix Workspace Build

#### Architecture
No new components. Pure workspace configuration change.

#### File Changes

**Modified**: `Cargo.toml`
- Add 8 empty directories to the `exclude` list:
  - `crates/terraphim_agent`
  - `crates/terraphim_agent_evolution`
  - `crates/terraphim_automata`
  - `crates/terraphim_cli`
  - `crates/terraphim_multi_agent`
  - `crates/terraphim_orchestrator`
  - `crates/terraphim_service`
  - `crates/terraphim_settings`

**No new or deleted files**

#### API Design
None.

#### Test Strategy

| Test | Command | Purpose |
|------|---------|---------|
| Workspace check | `cargo check --workspace` | Confirms workspace loads |
| Server check | `cargo check -p terraphim_server` | Confirms default member builds |
| Remote check | `rch exec -- cargo check --workspace` | Confirms build on bigbox |

#### Implementation Steps

**Step 1.1**: Audit empty directories
- Inspect each of the 8 directories to confirm no `Cargo.toml` exists
- Confirm no untracked source files that need preserving
- Estimated: 15 minutes

**Step 1.2**: Update `Cargo.toml`
- Add the 8 paths to `workspace.exclude`
- Group them under a comment like `# Empty directories after polyrepo extraction (Gitea #1910)`
- Estimated: 15 minutes

**Step 1.3**: Verify
- Run `cargo check --workspace`
- Run `cargo check -p terraphim_server`
- Run `cargo fmt` and `cargo clippy` on workspace
- Estimated: 30 minutes

---

### Item 2: Re-enable Auto-Update

#### Architecture

```
terraphim_server/src/main.rs
  -> imports terraphim_update
  -> calls check_for_updates() or update_binary()
     -> terraphim_update crate
        -> self_update (patched)
           -> GitHub Releases API
```

#### File Changes

**New Files** (restored from worktree):
| File | Purpose |
|------|---------|
| `crates/terraphim_update/Cargo.toml` | Crate manifest |
| `crates/terraphim_update/src/lib.rs` | Public API |
| `crates/terraphim_update/src/config.rs` | Update config types |
| `crates/terraphim_update/src/state.rs` | Update history persistence |
| `crates/terraphim_update/src/platform.rs` | Platform detection |
| `crates/terraphim_update/src/downloader.rs` | Download logic |
| `crates/terraphim_update/src/signature.rs` | Signature verification |
| `crates/terraphim_update/src/notification.rs` | User notifications |
| `crates/terraphim_update/src/scheduler.rs` | Update scheduling |
| `crates/terraphim_update/src/rollback.rs` | Rollback logic |
| `crates/terraphim_update/tests/signature_test.rs` | Signature tests |

**Modified Files**:
| File | Changes |
|------|---------|
| `Cargo.toml` | Add `crates/terraphim_update` to `members` (or let `crates/*` pick it up) |
| `terraphim_server/Cargo.toml` | Add `terraphim_update` dependency |
| `terraphim_server/src/main.rs` | Uncomment imports and implement `handle_update_commands` |

#### API Design

Restore the existing public API:

```rust
// crates/terraphim_update/src/lib.rs
pub use config::UpdaterConfig;
pub use state::{load_update_history, save_update_history};

/// Check for available updates
pub async fn check_for_updates() -> Result<UpdateStatus, UpdateError>;

/// Download and install latest update
pub async fn update_binary() -> Result<UpdateStatus, UpdateError>;
```

#### Test Strategy

| Test | Location | Purpose |
|------|----------|---------|
| Unit tests | `crates/terraphim_update/tests/` | Signature verification, config parsing |
| Build test | `cargo check -p terraphim_update` | Crate compiles |
| Server integration | `cargo check -p terraphim_server` | Server links against update crate |
| CLI smoke test | `cargo run --bin terraphim_server -- --check-update` | Verifies CLI path (may fail on network, but should not panic) |

#### Implementation Steps

**Step 2.1**: Copy terraphim_update from worktree
- Copy `.worktrees/pr-security-sentinel-31531dbb/crates/terraphim_update/` to `crates/terraphim_update/`
- Estimated: 15 minutes

**Step 2.2**: Adapt Cargo.toml
- Verify `crates/terraphim_update/Cargo.toml` uses workspace dependencies where appropriate
- Update `terraphim_server/Cargo.toml` to add `terraphim_update = { path = "../crates/terraphim_update" }`
- Estimated: 30 minutes

**Step 2.3**: Restore server integration
- In `terraphim_server/src/main.rs`:
  - Uncomment `use terraphim_update::{check_for_updates, update_binary};`
  - Replace stub `handle_update_commands` with real implementation
- Estimated: 30 minutes

**Step 2.4**: Verify
- `cargo check -p terraphim_update`
- `cargo check -p terraphim_server`
- Run existing tests for update crate
- Estimated: 30 minutes

#### Rollback Plan
If restoration fails:
1. Revert `terraphim_server/src/main.rs` to stub
2. Remove `terraphim_update` dependency from `terraphim_server/Cargo.toml`
3. Delete `crates/terraphim_update/`

---

### Item 3: Complete Polyrepo Publish

#### Architecture
Use the existing `polyrepo-publish.sh` pipeline. No pipeline changes needed.

```
For each blocked repo:
  -> Fix source issue
  -> Push to Gitea publish/github-mirror
  -> Wait for Gitea CI
  -> Rewrite Cargo.toml
  -> Push to GitHub
  -> Wait for GitHub CI
  -> Merge back
  -> Dispatch publish-crates.yml
  -> Wait for crates.io
```

#### File Changes

**Modified**: `scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh`
- Already fixed: `publish = ["terraphim"]` stripping
- Already fixed: cargo rewrite ordering
- Already fixed: idempotency
- Already fixed: auto-secret setting
- Already fixed: fetch before push

**Modified**: Source code in polyrepo split repos
- `terraphim-service`: Bump `terraphim_spawner` version if needed
- `terraphim-agents`: May need version constraint update for `terraphim_spawner`

#### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Bump `terraphim_spawner` to 1.20.3+ and republish | The current published version lacks `with_stderr_log`; Gitea main has it. The simplest fix is to ensure the published version reflects Gitea main. |
| Force re-publish of terraphim-service crates if needed | Use the idempotent publish loop to push any version bumps. |
| Retry terraphim-clients after script fix | The `publish = ["terraphim"]` fix is already in the script; just re-run. |
| Wait for terraphim-agents before running terraphim-kg-agents | Topological dependency order must be preserved. |

#### Test Strategy

| Test | Command | Purpose |
|------|---------|---------|
| Dry-run | `POLYREPO_DRY_RUN=1 bash polyrepo-publish.sh full <repo>` | Pipeline still works |
| Gitea CI | Poll commit statuses | Gate 1 passes |
| GitHub CI | `gh run list` | Gate 2 passes |
| crates.io | `curl https://crates.io/api/v1/crates/<name>/<version>` | Crate published |

#### Implementation Steps

**Step 3.1**: Fix terraphim-service / terraphim_spawner version
- Verify `terraphim_spawner` version in `terraphim-service/crates/terraphim_spawner/Cargo.toml`
- If version hasn't been bumped since the method was added, bump patch or minor
- Commit and push to Gitea main (follows normal PR flow)
- Re-run `polyrepo-publish.sh crates-publish terraphim-service`
- Estimated: 45 minutes + CI wait time (~10 min)

**Step 3.2**: Update terraphim-agents dependency constraint
- In `terraphim-agents/crates/terraphim_orchestrator/Cargo.toml`:
  - Change `terraphim_spawner = { version = "1.0.0" }` to use a version that includes `with_stderr_log`
  - Or use `version = ">=1.20.3"` if that's when the method was introduced
- Commit to Gitea main
- Estimated: 30 minutes

**Step 3.3**: Re-run terraphim-agents pipeline
- `bash polyrepo-publish.sh full terraphim-agents`
- Wait for both CI gates
- Verify crates publish
- Estimated: 20 minutes + CI wait time (~10 min)

**Step 3.4**: Re-run terraphim-kg-agents pipeline
- `bash polyrepo-publish.sh full terraphim-kg-agents`
- Wait for both CI gates
- Verify crates publish
- Estimated: 20 minutes + CI wait time (~10 min)

**Step 3.5**: Retry terraphim-clients publish
- `bash polyrepo-publish.sh full terraphim-clients`
- The `publish = ["terraphim"]` fix should now allow publish to succeed
- Estimated: 20 minutes + CI wait time (~10 min)

#### Rollback Plan
If publish fails:
1. Check GitHub Actions logs for specific crate failure
2. Fix source issue in Gitea main
3. Re-run pipeline (idempotent - already-published crates are skipped)

---

## Cross-Item Dependencies

```
Item 1 (Workspace) ──┬── enables local verification of server build
                     │
Item 2 (Auto-update) ├── depends on Item 1 for server integration testing
                     │
Item 3 (Polyrepo)    ├── terraphim-agents must complete before terraphim-kg-agents
                     └── workspace fixes help verify nothing else broke
```

## Recommended Execution Order

1. **Item 1 first** (15-60 min) - Unblocks local builds
2. **Item 2 in parallel** with Item 3 prep (60-90 min) - Restores auto-update
3. **Item 3 sequential by dependency** (2-3 hours including CI):
   - terraphim-service version bump (if needed)
   - terraphim-agents fix + publish
   - terraphim-kg-agents publish
   - terraphim-clients retry publish

## Quality Gates

Before proceeding to implementation (Phase 3):
- [ ] This plan reviewed and approved
- [ ] 1Password session available for crates.io token
- [ ] Gitea token available for API operations
- [ ] GitHub CLI authenticated
- [ ] Workspace build verified after Item 1

After implementation:
- [ ] `cargo check --workspace` passes
- [ ] `terraphim_server --check-update` runs without "disabled" error
- [ ] All 6 polyrepo splits published to crates.io
- [ ] Both GitHub and Gitea remotes in sync
- [ ] ADR / docs updated if needed

## Open Items

| Item | Status | Owner | Blocking? |
|------|--------|-------|-----------|
| Confirm exact version bump needed for terraphim_spawner | Pending | Implementer | Yes for item 3 |
| Verify terraphim_update has no dependency on moved crates | Pending | Implementer | Yes for item 2 |
| Decide if empty crate directories should be deleted after exclusion | Pending | Human | No |

## Appendix: Relevant Code References

- Workspace config: `Cargo.toml:1-114`
- Server main: `terraphim_server/src/main.rs:68-74`
- terraphim_update worktree: `.worktrees/pr-security-sentinel-31531dbb/crates/terraphim_update/`
- Polyrepo script: `scripts/adf-setup/polyrepo-publish/polyrepo-publish.sh`
- Tracking issue: Gitea #2260
