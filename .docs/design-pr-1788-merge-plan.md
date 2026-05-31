# Implementation Plan: PR #1788 Split Merge

**Status**: Draft
**Research Doc**: `.docs/research-pr-1788-merge.md`
**Author**: OpenCode
**Date**: 2026-05-31
**Estimated Effort**: 2-3 focused sessions
**Target PR**: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1788

## Overview

### Summary

PR #1788 should not be merged directly. This plan decomposes it into focused, independently reviewable slices, starting with the actual titled feature: integrating `.terraphim/skills/` with native Claude/OpenCode project skill directories for `adf --local run`.

### Approach

Create new branches from current `main` and extract only the relevant hunks from `gitea/task/1768-local-skills-discovery`. Do not rebase the entire PR. Each slice gets its own verification and validation evidence before merge.

### Scope

**In Scope for Slice 1 (merge first):**
- Add explicit project root tracking to `ProjectAdfConfig`.
- Add `ProjectAdfConfig::project_root()`.
- Add `ProjectAdfConfig::skills_dir()`.
- Add `AgentConfig::skill_dir_name()` for Claude/OpenCode.
- Add local skill symlink integration to `adf --local run` before spawning.
- Preserve live output streaming behaviour.
- Add/retain focused tests for idempotency, unsupported CLIs, directory filtering, and existing skill coexistence.

**Out of Scope for Slice 1:**
- Spawner captured output buffer.
- Timeout output posting to Gitea.
- Agent registry.
- Webhook group alias dispatch.
- Provider probe timeout changes.
- Worktree fail-closed changes.
- Reconcile-loop timing instrumentation.
- TLA specs.
- `.terraphim/learnings/` files.

**Important**: Out of scope does not mean abandoned. Every excluded functional change must either land through a separate focused PR or be explicitly rejected with a Gitea comment explaining why. See **Excluded Changes Landing Plan** below.

**Avoid At All Cost**:
- Rebasing all 173 files into `main`.
- Merging generated `.terraphim/learnings/` artefacts.
- Changing foreground `adf --local run` output semantics.
- Posting raw agent output to Gitea without redaction.
- Bundling registry/webhook/provider/worktree changes into the skills PR.

## Architecture

### Component Diagram

```text
adf --local run
    |
    v
ProjectAdfConfig::discover_and_load(cwd)
    |
    +-- project_root() ----------------------+
    |                                        |
    +-- skills_dir() -> .terraphim/skills    |
                                             v
AgentDefinition.cli_tool -> AgentConfig::skill_dir_name()
                                             |
                                             v
project_root/.claude/skills or .opencode/skill
                                             |
                                             v
symlink project skill dirs, skip existing paths
                                             |
                                             v
existing AgentSpawner live output path
```

### Data Flow

```text
Input cwd
  -> discover `.terraphim/adf.toml`
  -> load config with explicit `project_root`
  -> convert local ADF config to `Project` + `AgentDefinition`
  -> if `.terraphim/skills/` exists and CLI supports native skill dir:
       create native skill dir
       symlink skill directories without overwriting
  -> spawn local agent exactly as main does today
  -> stream output live exactly as main does today
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Extract from current `main`, not rebase full #1788 | Avoid unrelated P1 regressions and generated artefacts | Full rebase of 173 files. |
| Symlink directories only | Skill conventions treat each skill as a directory | Linking files, copying recursively. |
| Skip existing destination path | Prevent overwriting user/global skills | Delete/replace existing skills. |
| No-op unsupported CLI tools | Keeps local run robust for Codex/echo/tests | Fail if `.terraphim/skills/` exists. |
| Preserve live output streaming | Foreground CLI must remain diagnosable | Post-exit replay from captured buffer. |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Full #1788 merge | Contains multiple unrelated P1 risks | Runtime regressions, security exposure, hard rollback. |
| Fix all #1788 findings in-place | Still leaves huge review surface | High conflict and regression risk. |
| Copy skills instead of symlink | Creates stale duplicates | Users edit one copy while CLI reads another. |
| Add timeout output posting in same PR | Not required for skills | Data exposure and policy uncertainty. |
| Add agent registry in same PR | Separate architectural boundary | Hard to isolate defects. |

### Simplicity Check

The simplest solution is to make project-local skills visible to the CLI tools that already know how to load project skill directories. That requires only three conceptual changes: know the project root, map CLI tool to native skill directory, and create idempotent symlinks before spawning.

**Senior Engineer Test**: A senior engineer would reject the full PR as a bundle and extract the small local-skills feature first. The proposed design does that.

**Nothing Speculative Checklist:**
- [x] No features the user did not request in Slice 1.
- [x] No new registry abstraction.
- [x] No output buffering redesign.
- [x] No timeout posting policy.
- [x] No generated artefacts.

## File Changes

### New Files

None for Slice 1.

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/project_adf.rs` | Add `root: PathBuf`, `project_root()`, `skills_dir()`, and `load_from_path(adf_path, project_root)`. Update discovery conversion and tests. |
| `crates/terraphim_orchestrator/src/bin/adf.rs` | Add `integrate_local_skills()` and call it before local agent spawn. Preserve current live output subscription and printing. |
| `crates/terraphim_spawner/src/config.rs` | Add `AgentConfig::skill_dir_name(cli_command) -> Option<&'static str>` and tests. |

### Deleted or Excluded Files

| File/Path | Reason |
|-----------|--------|
| `.terraphim/learnings/*.md` | Generated runtime artefacts; must not merge. |
| `crates/terraphim_orchestrator/src/agent_registry.rs` | Separate registry slice. |
| `crates/terraphim_orchestrator/tla/*` | Separate formal-spec slice. |
| PR #1788 docs unrelated to local skills | Separate documentation or discard. |

## API Design

### Public/Internal Types

```rust
pub struct ProjectAdfConfig {
    pub project_id: String,
    pub name: Option<String>,
    pub agents: Vec<TomlAdfAgent>,
    pub pr_dispatch: Option<PrDispatchConfig>,
    pub discovered_path: PathBuf,
    root: PathBuf,
}
```

### Functions

```rust
impl ProjectAdfConfig {
    pub fn project_root(&self) -> &Path;

    pub fn skills_dir(&self) -> Option<PathBuf>;

    pub fn load_from_path(
        adf_path: &Path,
        project_root: &Path,
    ) -> Result<Self, OrchestratorError>;
}
```

```rust
impl AgentConfig {
    pub fn skill_dir_name(cli_command: &str) -> Option<&'static str>;
}
```

```rust
fn integrate_local_skills(
    working_dir: &Path,
    cli_tool: &str,
    skills_dir: &Path,
) -> Result<usize, String>;
```

### Error Handling

- Missing `.terraphim/skills/` is not an error.
- Unsupported CLI tool is not an error; return `Ok(0)`.
- Failure to create the native skill dir or symlink is a warning for `adf --local run`, not a spawn blocker for Slice 1.
- Existing destination path is skipped.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_skill_dir_name` | `terraphim_spawner/src/config.rs` | Verify Claude/OpenCode path mapping and unsupported tool no-op. |
| `skills_dir_returns_none_when_absent` | `project_adf.rs` | Verify no skills path when directory absent. |
| `skills_dir_returns_path_when_present` | `project_adf.rs` | Verify project root-based skill path discovery. |
| `integrate_local_skills_creates_symlinks_for_claude` | `adf.rs` | Verify Claude native path. |
| `integrate_local_skills_creates_symlinks_for_opencode` | `adf.rs` | Verify OpenCode native path. |
| `integrate_local_skills_is_idempotent` | `adf.rs` | Verify repeated run skips existing links. |
| `integrate_local_skills_noop_for_unknown_cli` | `adf.rs` | Verify unsupported CLI no-op. |
| `integrate_local_skills_skips_files_in_skills_dir` | `adf.rs` | Verify only directories are linked. |
| `integrate_local_skills_coexists_with_existing_skills` | `adf.rs` | Verify existing native skills are untouched. |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| Existing `adf --local check` tests | `crates/terraphim_orchestrator/tests` or inline tests | Ensure root changes do not break local config validation. |
| Existing local run tests, if present | `crates/terraphim_orchestrator` | Ensure live output behaviour remains unchanged. |

### Verification Commands

```bash
cargo test -p terraphim_spawner test_skill_dir_name
cargo test -p terraphim_orchestrator integrate_local_skills
cargo test -p terraphim_orchestrator skills_dir
cargo check -p terraphim_orchestrator
cargo check -p terraphim_spawner
cargo fmt --all -- --check
cargo clippy -p terraphim_orchestrator -- -D warnings
cargo clippy -p terraphim_spawner -- -D warnings
ubs crates/terraphim_orchestrator/src/bin/adf.rs crates/terraphim_orchestrator/src/project_adf.rs crates/terraphim_spawner/src/config.rs
```

## Implementation Steps

### Step 1: Prepare Focused Branch

**Files:** N/A
**Description:** Create a new branch from current `main` for Slice 1.
**Tests:** N/A
**Estimated:** 10 minutes

```bash
git checkout main
git fetch origin main gitea main
git checkout -b task/1768-local-skills-focused
```

### Step 2: Extract Project Root and Skills Discovery

**Files:** `crates/terraphim_orchestrator/src/project_adf.rs`
**Description:** Add explicit root storage and skills discovery helper. Update conversion to use `project_root()`.
**Tests:** `cargo test -p terraphim_orchestrator skills_dir`
**Estimated:** 30 minutes

### Step 3: Extract CLI Skill Directory Mapping

**Files:** `crates/terraphim_spawner/src/config.rs`
**Description:** Add `skill_dir_name()` and focused tests.
**Tests:** `cargo test -p terraphim_spawner test_skill_dir_name`
**Estimated:** 20 minutes

### Step 4: Extract Local Skill Integration Without Output Changes

**Files:** `crates/terraphim_orchestrator/src/bin/adf.rs`
**Description:** Add `integrate_local_skills()` and call before spawn, but keep existing live output subscriber logic from `main`.
**Tests:** `cargo test -p terraphim_orchestrator integrate_local_skills`
**Dependencies:** Steps 2-3
**Estimated:** 45 minutes

### Step 5: Disciplined Verification

**Files:** changed files only
**Description:** Run UBS first, then targeted tests, formatting, clippy, and compile checks.
**Tests:** Commands listed in Test Strategy.
**Dependencies:** Steps 2-4
**Estimated:** 45 minutes

### Step 6: Disciplined Validation

**Files:** N/A
**Description:** Validate user-visible workflow: local ADF project with `.terraphim/skills/` and a simple supported CLI tool mapping creates symlinks idempotently. For unsupported tools, validate no-op behaviour.
**Tests:** Unit tests plus manual/local command if safe.
**Dependencies:** Step 5
**Estimated:** 30 minutes

### Step 7: Merge and Close/Supersede #1788

**Files:** Gitea PR state only
**Description:** Open focused PR, merge after checks, comment on #1788 that it is superseded for local skills and must be split for remaining work.
**Tests:** Remote sync verification.
**Dependencies:** Steps 5-6
**Estimated:** 20 minutes

## Rollback Plan

If the focused local-skills PR causes issues:

1. Revert its merge commit only.
2. Push revert to both `origin` and `gitea`.
3. Leave #1788 open or closed with split-work issues, but do not restore the bundled branch.

Because Slice 1 touches only local ADF run behaviour and CLI skill path mapping, rollback should be low-risk and isolated.

## Follow-Up Slice Plan

| Slice | Source from #1788 | Action |
|-------|-------------------|--------|
| Slice 2: Output capture | `terraphim_spawner/src/output.rs`, `AgentHandle::captured_output_events` | Redesign with `VecDeque`, redaction policy, and separate security review. |
| Slice 3: Timeout reporting | `poll_wall_timeouts` changes | Only after output redaction exists; add caps and tests. |
| Slice 4: Agent registry | `agent_registry.rs`, registry-backed lookups | Separate architecture review and tests for duplicate/project-scoped agents. |
| Slice 5: Webhook aliases | `webhook.rs` group alias dispatch | Separate dispatch semantics review; add collision and auth tests. |
| Slice 6: Worktree fail-closed | `create_agent_worktree` changes | Separate safety PR with regression tests. |
| Slice 7: Provider probe timeout | `provider_probe.rs` | Separate ops/performance decision; validate 15s is sufficient. |
| Slice 8: TLA/docs | `tla/*`, `.docs/*` | Merge only curated docs/specs, no generated artefacts. |

## Excluded Changes Landing Plan

The following table is the authoritative disposition for every meaningful change excluded from Slice 1. A change may not be silently dropped. Before closing #1788, each row must be represented by a merged PR, an open replacement PR, or an explicit rejection comment on #1788.

| Follow-up PR | Change Area | Files from #1788 | Landing Requirement | Acceptance Criteria |
|--------------|-------------|------------------|---------------------|---------------------|
| PR-A | Local skills integration | `project_adf.rs`, `bin/adf.rs`, `terraphim_spawner/src/config.rs` | **Land first** as Slice 1. | Skills are symlinked idempotently for Claude/OpenCode; unsupported CLIs no-op; live output streaming preserved; no unrelated files. |
| PR-B | Agent output capture buffer | `terraphim_spawner/src/output.rs`, `terraphim_spawner/src/lib.rs` | Land only after security review of captured data lifecycle. | Uses `VecDeque` or equivalent O(1) ring buffer; bounded by count and preferably bytes; no automatic remote posting; tests cover overflow and mention events. |
| PR-C | Timeout reporting to Gitea | `terraphim_orchestrator/src/lib.rs` timeout paths | Land only after PR-B and redaction policy. | Redacts/sanitises output before posting; caps posted lines/bytes; preserves exit code context; tests prove secrets/patterns are not posted. |
| PR-D | Project-scoped agent registry | `agent_registry.rs`, `lib.rs` registry lookups, related tests | Dedicated architecture PR. | Duplicate detection tested; legacy and project scopes tested; no behaviour change for existing single-project configs; build-runner lookup regression tests pass. |
| PR-E | Webhook group alias dispatch | `webhook.rs` | Dedicated dispatch semantics PR. | Alias matching cannot shadow exact agent names; project derivation tested; concurrency/rate limits still enforced; unauthorised dispatch behaviour unchanged. |
| PR-F | Worktree fail-closed behaviour | `lib.rs`, `worktree_guard` call sites if needed | Dedicated safety PR. | Mutating model-backed agents never fall back to shared checkout on worktree failure; read-only/local command tests remain unaffected; regression tests cover failure path. |
| PR-G | Provider probe timeout reduction | `provider_probe.rs` | Dedicated operations PR with evidence. | Timeout value justified by measured provider behaviour; log/error text matches actual timeout; tests updated; no false unhealthy marking for slow but valid providers. |
| PR-H | Reconcile-loop instrumentation | `lib.rs` timed-step macro and reconcile changes | Dedicated observability PR. | No duplicate `tick_count` or `last_tick_time` update; periodic tick-based tasks preserve cadence; tests cover tick increment exactly once. |
| PR-I | TLA and formal specs | `crates/terraphim_orchestrator/tla/*` | Dedicated documentation/spec PR. | Specs are curated, referenced from docs, and do not depend on unmerged implementation changes. |
| PR-J | Curated research/design docs | `.docs/*` from #1788 | Dedicated documentation PR or selective merge into each functional PR. | Only relevant docs land with their slice; stale/duplicate planning artefacts are excluded. |
| Reject/Do not land | Generated learning artefacts | `.terraphim/learnings/*.md` | **Do not land**. Add rejection comment and ensure ignored. | No learning artefacts in any replacement PR; if useful, manually summarise into `.docs/` with review. |

### Follow-Up PR Ordering

1. PR-A: Local skills integration.
2. PR-B: Output capture buffer.
3. PR-C: Timeout reporting, dependent on PR-B.
4. PR-D: Agent registry.
5. PR-E: Webhook group alias dispatch, after PR-D if it relies on registry semantics.
6. PR-F: Worktree fail-closed behaviour.
7. PR-G: Provider probe timeout reduction.
8. PR-H: Reconcile-loop instrumentation.
9. PR-I and PR-J: Curated docs/specs, after or alongside their matching implementation slices.

### Closure Rule for #1788

#1788 can be closed only after:

- PR-A is merged or opened as a focused replacement.
- PR-B through PR-J are either opened as separate PRs/issues or explicitly rejected in a comment.
- `.terraphim/learnings/*.md` is explicitly rejected and absent from all replacement PRs.
- The #1788 closing comment links to all replacement PRs/issues so no useful work is lost.

## Dependencies

### New Dependencies

None for Slice 1.

### Dependency Updates

None for Slice 1.

## Performance Considerations

Slice 1 performs a directory scan and creates symlinks once before local agent spawn. Expected cost is negligible relative to agent runtime. The implementation skips non-directory entries and existing links. No hot-path service runtime is affected.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm Windows support requirement | Pending | Human maintainer |
| Decide whether unsupported CLI with `.terraphim/skills/` should warn | Pending | Human maintainer |
| Decide whether #1788 should be closed after Slice 1 or kept as umbrella | Pending | Human maintainer |

## Approval

- [ ] Research document reviewed
- [ ] Split strategy approved
- [ ] Slice 1 scope approved
- [ ] Test strategy approved
- [ ] Human approval received before implementation

## Merge Readiness Gate for Slice 1

- [ ] No `.terraphim/learnings/` files in diff.
- [ ] No `agent_registry.rs` in diff.
- [ ] No `OutputCapture` buffering changes in diff.
- [ ] No timeout output posting changes in diff.
- [ ] No webhook alias changes in diff.
- [ ] `adf --local run` still streams live output.
- [ ] UBS passes on changed source files.
- [ ] Targeted tests pass.
- [ ] `cargo check -p terraphim_orchestrator` passes.
- [ ] `cargo check -p terraphim_spawner` passes.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] Remotes are synced after merge.
