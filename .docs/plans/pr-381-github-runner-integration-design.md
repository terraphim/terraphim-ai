# Design & Implementation Plan: PR #381 GitHub Runner Integration Merge

**Status:** Ready for Review
**PR:** #381 - feat: Add DevOps/CI-CD role configuration and GitHub runner integration
**Date:** 2025-12-31

---

## 1. Summary of Target Behavior

After merging PR #381, the system will:

1. **Support self-hosted Terraphim-based GitHub runners** with Firecracker VM orchestration
2. **Provide DevOps/CI-CD role configurations** for specialized knowledge graph queries
3. **Enable sub-200ms command execution** via pre-warmed VM pools
4. **Maintain all recent CI fixes** (Tauri builds, cross-compilation, signing)

---

## 2. Key Invariants and Acceptance Criteria

### Invariants

| Invariant | Guarantee |
|-----------|-----------|
| I1 | Recent CI fixes (Tauri v1, cross-compilation, signing) are preserved |
| I2 | All existing workflows continue to function |
| I3 | New DevOps role config doesn't break existing roles |
| I4 | Claude log analyzer maintains backward compatibility |

### Acceptance Criteria

| ID | Criterion | Testable |
|----|-----------|----------|
| AC1 | CI workflows pass after merge | Yes - CI runs |
| AC2 | DevOps role config loads successfully | Yes - server startup |
| AC3 | Tauri desktop builds work on Ubuntu 22.04 | Yes - release workflow |
| AC4 | Cross-compilation with feature flags works | Yes - musl/ARM builds |
| AC5 | macOS signing and notarization works | Yes - release workflow |

---

## 3. High-Level Design and Boundaries

### Components to Merge (Keep from PR #381)

| Component | Purpose | Risk |
|-----------|---------|------|
| `devops_cicd_config.json` | New role configuration | Low - additive |
| `.docs/github-runner-ci-integration.md` | Integration documentation | Low - docs only |
| Claude log analyzer updates | Enhanced analysis | Low - isolated crate |
| Blog posts/marketing | Content | None |
| Test data files | Test fixtures | Low |

### Components to Carefully Merge (Review for Conflicts)

| Component | PR #381 Changes | Main Changes | Resolution Strategy |
|-----------|-----------------|--------------|---------------------|
| `ci-native.yml` | Whitespace cleanup | Tauri/webkit fixes | Take main's version |
| `ci-pr.yml` | Minor updates | Recent fixes | Take main, cherry-pick non-conflicting |
| `release.yml` | Slack->Discord | Signing fixes | Merge both changes |
| `ci-main.yml` | Updates | Recent fixes | Take main, cherry-pick runner integration |

### Components to Skip (Already Superseded)

| Component | Reason |
|-----------|--------|
| `floor_char_boundary` changes | Already in main with MSRV-compatible version |
| Old lesson-learned entries | Main has more recent entries |

---

## 4. File/Module-Level Change Plan

### Phase 1: Resolve Conflicts (Take Main's Version)

| File | Action | Conflict Type | Resolution |
|------|--------|---------------|------------|
| `crates/terraphim_middleware/src/indexer/ripgrep.rs` | Resolve | floor_char_boundary | Keep main's version |
| `terraphim_server/src/lib.rs` | Resolve | floor_char_boundary | Keep main's version |
| `crates/terraphim_settings/test_settings/settings.toml` | Resolve | Config values | Review and merge |
| `lessons-learned.md` | Resolve | Appended content | Keep both (main + PR additions) |

### Phase 2: Workflow Files (Selective Merge)

| File | Action | Strategy |
|------|--------|----------|
| `.github/workflows/ci-native.yml` | Skip PR changes | Main has critical Tauri fixes |
| `.github/workflows/ci-pr.yml` | Skip PR changes | Main has recent fixes |
| `.github/workflows/ci-main.yml` | Skip PR changes | Main has recent fixes |
| `.github/workflows/ci-optimized-main.yml` | Skip PR changes | Keep main |
| `.github/workflows/release.yml` | Manual merge | Keep main's signing fixes, add Discord notification |
| `.github/workflows/deploy.yml` | Review | May contain useful changes |
| `.github/workflows/test-ci.yml` | Accept PR | New file for testing |
| `.github/workflows/test-firecracker-runner.yml` | Accept PR | New file for runner testing |

### Phase 3: New Files (Accept All)

| File | Action | Purpose |
|------|--------|---------|
| `terraphim_server/default/devops_cicd_config.json` | Create | DevOps role config |
| `.docs/github-runner-ci-integration.md` | Create | Integration docs |
| `.docs/*.md` (other docs) | Create | Design/research docs |
| `blog-posts/*.md` | Create | Marketing content |
| `blog/*.md` | Create | Social media drafts |
| `crates/claude-log-analyzer/tests/test_data/*.jsonl` | Create | Test fixtures |

### Phase 4: Modified Files (Careful Review)

| File | Action | Notes |
|------|--------|-------|
| `Cargo.lock` | Auto-merge | Let git handle |
| `crates/claude-log-analyzer/Cargo.toml` | Review | Check for version bumps |
| `crates/claude-log-analyzer/src/*.rs` | Accept | Isolated crate |
| `Earthfile` | Review | May have runner integration |
| `.gitignore` | Accept | Additional ignores |
| `HANDOVER-2025-01-31.md` | Accept | Historical document |

---

## 5. Step-by-Step Implementation Sequence

### Step 1: Abort Current Merge State
- Purpose: Clean slate
- Deployable: N/A

### Step 2: Resolve Code Conflicts
- Resolve `ripgrep.rs` - take main's `floor_char_boundary`
- Resolve `lib.rs` - take main's version
- Resolve `settings.toml` - review and merge
- Resolve `lessons-learned.md` - append PR content after main
- Deployable: Yes

### Step 3: Skip Workflow Conflicts
- `git checkout --ours` for workflow files that main has fixed
- Keep recent CI fixes intact
- Deployable: Yes

### Step 4: Accept New Files
- Stage all new documentation files
- Stage DevOps role config
- Stage test fixtures
- Stage blog content
- Deployable: Yes

### Step 5: Review and Test
- Run `cargo check --workspace`
- Run `cargo clippy -p terraphim_server`
- Verify DevOps config loads
- Deployable: Yes

### Step 6: Commit and Push
- Single merge commit
- Push to PR branch
- Wait for CI
- Deployable: Yes (after CI passes)

---

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Verification |
|---------------------|-----------|--------------|
| AC1: CI passes | Integration | GitHub Actions |
| AC2: DevOps config loads | Manual | `cargo run -- --config devops_cicd_config.json` |
| AC3: Tauri builds | CI | Release workflow on test tag |
| AC4: Cross-compilation | CI | Matrix build jobs |
| AC5: macOS signing | CI | Release workflow |

---

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Workflow conflicts override recent fixes | Skip PR's workflow changes, keep main | Low |
| DevOps config breaks existing roles | Config is additive, separate file | None |
| Claude log analyzer changes break tests | Run tests before merge | Low |
| Merge commit too large to review | Step-by-step approach with clear phases | Low |

---

## 8. Open Questions / Decisions for Human Review

| Question | Options | Recommendation |
|----------|---------|----------------|
| Keep PR's workflow changes? | Yes / No / Selective | **No** - main has critical fixes |
| Discord vs Slack notifications? | Discord / Slack / Both | **Discord** - from PR |
| Include blog posts? | Yes / No | **Yes** - marketing value |
| Include test-firecracker-runner.yml? | Yes / No | **Yes** - useful for runner testing |

---

## 9. Recommended Merge Command Sequence

```bash
# Step 1: Reset merge state
git merge --abort

# Step 2: Start fresh merge
git merge origin/main --no-commit

# Step 3: Resolve conflicts by keeping main's versions
git checkout --theirs crates/terraphim_middleware/src/indexer/ripgrep.rs
git checkout --theirs terraphim_server/src/lib.rs

# Step 4: Manually resolve settings.toml and lessons-learned.md

# Step 5: Skip workflow changes (keep main's fixes)
git checkout --theirs .github/workflows/ci-native.yml
git checkout --theirs .github/workflows/ci-pr.yml
git checkout --theirs .github/workflows/ci-main.yml
git checkout --theirs .github/workflows/ci-optimized-main.yml

# Step 6: Stage resolved files
git add -A

# Step 7: Commit merge
git commit -m "Merge branch 'main' into feat/github-runner-ci-integration"

# Step 8: Push and verify CI
git push origin feat/github-runner-ci-integration
```

---

**Do you approve this plan as-is, or would you like to adjust any part?**
