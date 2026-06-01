# Research Document: PR #1788 Merge Strategy

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-05-31
**Reviewers**: Human maintainer
**PR**: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/1788
**Last reviewed commit**: `b0b3d93`

## Executive Summary

PR #1788 is titled as project-local `.terraphim/skills/` integration, but the branch actually bundles many unrelated ADF/orchestrator changes across 173 files and roughly 12k added lines. The project-local skills feature is useful and has a coherent core, but the PR is not safe to merge as-is because it includes runtime regressions, generated local learning artefacts, timeout-output exposure risk, registry/routing changes, webhook dispatch changes, provider probe changes, TLA specs, and broad documentation additions.

The merge strategy should be decomposition, not direct rebase. The safe path is to extract the small skills feature into a new focused branch, discard generated artefacts, and handle each unrelated architectural change as its own independently reviewed PR.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Project-local skills are a high-leverage developer workflow improvement for `adf --local run`. |
| Leverages strengths? | Yes | The codebase already has `ProjectAdfConfig`, `AgentConfig`, and CLI spawn infrastructure that can support this with small changes. |
| Meets real need? | Yes | The PR exists to make `.terraphim/skills/` usable by native Claude/OpenCode project skill loaders instead of prompt-injecting project-scoped skill chains. |

**Proceed**: Yes, but only by splitting the PR. Direct merge is not acceptable.

## Problem Statement

### Description

Project-local ADF configurations can declare skill chains, but project-local skills under `.terraphim/skills/` are not currently integrated with native CLI skill paths such as `.claude/skills/` and `.opencode/skill`. PR #1788 attempts to solve this while also adding several unrelated orchestration features.

### Impact

Without a focused local-skills integration, project-specific skills either need manual copying into CLI-specific directories or must be injected into prompts, which duplicates CLI-native skill discovery and can inflate task payloads. However, merging the full PR would create broad operational risk across the orchestrator, webhook dispatch, output capture, and worktree isolation.

### Success Criteria

- `adf --local run <agent>` discovers `.terraphim/adf.toml` and resolves the correct project root.
- If `.terraphim/skills/` exists, each skill directory is linked into the native CLI project skill directory for supported tools.
- Supported tools initially remain limited to Claude and OpenCode.
- The operation is idempotent and does not overwrite existing skills.
- Local foreground output remains live-streamed while the child process runs.
- No generated `.terraphim/learnings/` artefacts are committed.
- No unrelated orchestrator registry, webhook, timeout, provider probe, or TLA changes are merged as part of the skills feature.

## Current State Analysis

### Existing Implementation

Current `main` already contains ADF local config discovery and local agent spawning. PR #1788 adds explicit project root tracking and a `skills_dir()` helper, then uses that helper in `adf --local run` to symlink project skills into CLI-native directories.

The PR also modifies unrelated areas:

- Agent registry and project-scoped lookup
- Webhook group alias dispatch
- Provider probe timeout behaviour
- Worktree creation fail-closed behaviour
- Reconcile-loop timing instrumentation
- Output capture retention and timeout issue posting
- TLA specification files
- Generated `.terraphim/learnings/*.md` files

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Local ADF discovery | `crates/terraphim_orchestrator/src/project_adf.rs` | Loads `.terraphim/adf.toml` and converts it into project/agent definitions. |
| Local ADF CLI | `crates/terraphim_orchestrator/src/bin/adf.rs` | Implements `adf --local check` and `adf --local run`. |
| CLI skill path mapping | `crates/terraphim_spawner/src/config.rs` | PR adds CLI-specific native skill path mapping. |
| Foreground output streaming | `crates/terraphim_orchestrator/src/bin/adf.rs` | Current behaviour streams output live; PR changes this to post-exit replay. |
| Output capture | `crates/terraphim_spawner/src/output.rs` | PR adds bounded in-memory event capture. |
| Timeout output posting | `crates/terraphim_orchestrator/src/lib.rs` | PR posts captured timeout output to Gitea issue comments. |
| Registry/routing | `crates/terraphim_orchestrator/src/agent_registry.rs`, `src/lib.rs` | PR adds registry-backed project agent lookup. |
| Webhook alias dispatch | `crates/terraphim_orchestrator/src/webhook.rs` | PR adds group alias expansion for `@adf:<prefix>`. |

### Data Flow: Intended Local Skills Feature

```text
cwd -> discover .terraphim/adf.toml -> ProjectAdfConfig(project_root)
    -> skills_dir = project_root/.terraphim/skills
    -> agent cli_tool -> AgentConfig::skill_dir_name(cli_tool)
    -> create project_root/.claude/skills or project_root/.opencode/skills
    -> symlink each skill directory
    -> spawn local agent with native CLI skill discovery available
```

### Data Flow: Risky Bundled Timeout Output Change

```text
agent stdout/stderr -> OutputCapture captured_events
    -> wall-clock timeout -> collect captured lines
    -> append timeout summary
    -> post_agent_output_for_project(... Gitea issue comment ...)
```

This flow widens exposure because raw agent output can include sensitive operational context or accidental secrets.

## Constraints

### Technical Constraints

- The repository is Rust-first and uses Cargo workspace checks.
- `terraphim_symphony` is excluded from the workspace and needs separate checks if touched.
- The PR branch is stale (`mergeable: false`) and has merge base `16678f3` while current `main` is `6d9a71fb`.
- The local-skills feature requires Unix symlinks as currently implemented; Windows support is not handled.
- The implementation must preserve foreground output streaming for `adf --local run`.

### Business Constraints

- Reduce the 29-open-PR backlog without merging high-risk bundles.
- Keep both GitHub and Gitea remotes in sync after successful merges.
- Avoid committing generated local artefacts or operational learnings.

### Non-Functional Requirements

| Requirement | Target | Current PR State |
|-------------|--------|-------------------|
| Merge blast radius | One feature per PR | Fails: 173 files and multiple unrelated features. |
| Runtime correctness | No behaviour regressions | Fails: duplicate tick update and non-streaming local output. |
| Data exposure | No new unredacted remote posting | Fails: timeout output posted to Gitea unredacted. |
| Reviewability | Small, testable chunks | Fails: ~12k added lines. |

## Vital Few (Essentialism)

### Essential Constraints

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Split local skills from unrelated orchestration work | Keeps merge small and reversible | Structural review found multiple unrelated P1 risks. |
| Preserve live output streaming | Foreground `adf --local run` must remain diagnosable | PR currently delays all output until process exit. |
| Remove generated `.terraphim/learnings/` | Prevents local artefact leakage and review noise | PR includes 100+ learning files. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Agent registry merge | Separate architectural change; not required for local skills. |
| Webhook group aliases | Separate dispatch semantics change; needs dedicated tests/review. |
| Provider probe timeout reduction | Operational tuning unrelated to local skills. |
| Timeout output posting | Security/observability change needing redaction design. |
| Worktree fail-closed behaviour | Important but independent runtime safety change. |
| TLA specs | Useful but unrelated to project-local skills. |
| Generated learnings | Local runtime artefacts, not source. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `ProjectAdfConfig` | Must retain project root for skills lookup | Low if isolated. |
| `AgentConfig::cli_name` | Used to map CLI command to skill dir | Low. |
| `adf --local run` | Integration point for symlink setup | Medium because it affects local UX. |
| `AgentSpawner` output streaming | Must remain unchanged for focused skills PR | Medium if accidentally modified. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Unix symlinks | OS API | Medium: Windows unsupported | Copy directories or platform abstraction in later PR. |
| Claude/OpenCode native skill loading | CLI convention | Medium: path conventions may change | Keep mapping small and tested. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Direct rebase pulls unrelated P1 bugs into main | High | High | Do not direct rebase; cherry-pick focused changes only. |
| Symlink integration overwrites existing user skills | Low | High | Skip existing path/symlink; test idempotency. |
| Broken local output UX | High in current PR | Medium | Preserve live subscriber; do not merge output replay change. |
| Sensitive output posted to remote issues | Medium in current PR | High | Exclude timeout posting from skills PR; design redaction separately. |
| Generated artefacts leak local context | High | High | Remove `.terraphim/learnings/` from merge scope. |

### Open Questions

1. Should project-local skills be symlinked permanently, or should `adf --local run` set CLI-specific environment/config for one run only?
2. Should unsupported CLI tools silently no-op, warn, or fail when `.terraphim/skills/` exists?
3. Is Windows support required for local ADF skills in the near term?
4. Should `.terraphim/skills/` be documented as source-controlled while `.terraphim/learnings/` remains ignored?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Claude uses `.claude/skills/` and OpenCode uses `.opencode/skill` | PR tests and confirmed local skill conventions | Skills do not load | Yes. |
| Project-local skill linking can happen before spawn without changing `AgentSpawner` | PR implementation | CLI cannot discover links in time | Yes, for local run path. |
| Existing paths must not be overwritten | User data safety | Local skill loss | Yes, tested in PR. |
| `.terraphim/learnings/` should not be tracked | Existing repo cleanup pattern and generated nature | Artefact leakage | Yes. |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Merge #1788 as a full ADF platform update | Fast but high risk | Rejected: unrelated P1 defects and huge blast radius. |
| Extract only local skills support | Small, testable, aligned with title | Chosen as the first merge slice. |
| Close #1788 and recreate all work manually | Safest but may lose useful tests | Rejected: cherry-pick focused code is more efficient. |
| Fix every finding in-place on #1788 | Large rebase with broad regression risk | Rejected: still a bundled PR. |

## Research Findings

### Key Insights

1. The valuable core of #1788 is small: `ProjectAdfConfig.project_root()`, `skills_dir()`, `AgentConfig::skill_dir_name()`, and `adf --local run` symlink setup.
2. The PR includes generated `.terraphim/learnings/` files that must be excluded from any merge path.
3. The output capture/timeout changes are not required for local skills and introduce security/UX risks.
4. The branch is stale and `mergeable: false`; rebasing the entire branch is not the right strategy.
5. Several changes are individually valuable but require dedicated research/design: agent registry, webhook aliases, fail-closed worktrees, timeout output reporting.

### Relevant Prior Art

- Existing `adf --local run` command already assembles a local `OrchestratorConfig` and spawns a named agent.
- Existing `AgentConfig::cli_name()` already normalises binary names from full paths.
- Existing structural review for #1788 documented P1/P2 findings in Gitea comment `#issuecomment-31797`.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Local skills focused cherry-pick | Confirm extracted changes compile without unrelated registry/output changes | 1 hour |
| Symlink behaviour on existing destination | Confirm no overwrite and idempotency | Covered by PR tests; rerun after extraction |
| Optional Windows strategy | Decide whether to gate symlink code with Unix-only cfg | 30 minutes |

## Recommendations

### Proceed/No-Proceed

Proceed with a split merge. Do not merge #1788 directly.

### Scope Recommendations

First merge only:

- `ProjectAdfConfig` root/skills helpers
- `AgentConfig::skill_dir_name()`
- `adf --local run` symlink integration
- Focused tests for these behaviours

Defer or reject:

- Output capture and timeout issue posting
- Registry-backed dispatch changes
- Webhook group alias changes
- Provider probe timeout changes
- TLA/docs unrelated to local skills
- Generated learnings

### Risk Mitigation Recommendations

- Create a new branch from current `main`; do not rebase the whole PR branch.
- Cherry-pick or manually apply only the focused hunks.
- Preserve live output streaming exactly as on `main`.
- Run targeted tests before broader checks.
- Post a comment on #1788 explaining that it should be superseded by split PRs.

## Next Steps

If approved:

1. Create `task/1768-local-skills-focused` from current `main`.
2. Extract only the local-skills feature from #1788.
3. Run disciplined verification on changed files and affected crates.
4. Open/merge a focused replacement PR.
5. Close #1788 after replacement PRs exist or after documenting remaining slices as new issues/PRs.

## Appendix

### Structural Review Findings

- P1: Duplicate `tick_count` and `last_tick_time` update in `reconcile_tick`.
- P1: `adf --local run` no longer streams output while agent runs.
- P1: Timed-out agent output posted to Gitea without redaction.
- P1: Generated `.terraphim/learnings/*.md` artefacts committed.
- P2: Output capture uses `Vec::remove(0)` on hot path.
