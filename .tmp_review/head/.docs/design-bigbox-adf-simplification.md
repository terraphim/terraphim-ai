# Design & Implementation Plan: Bigbox ADF Project-Source Loader

## 1. Summary of Target Behavior

ADF on bigbox will support a global `project_sources` registry. Each enabled project source points to a repository root and a project-local `.terraphim/adf.toml`. At startup and during `adf --check`, the orchestrator will load the global config, preserve existing `include` behaviour, then load project-local ADF configs from registered sources and merge their converted `Project`, `AgentDefinition`, and PR dispatch entries into the runtime config.

The first implementation is additive. Existing `/opt/ai-dark-factory/conf.d/*.toml` project configs continue to work. No production config is deleted in this step.

Project-scoped agents keep `skill_chain` as metadata. ADF does not inline project skill documents into prompts. Claude/opencode load project skills natively from the project working directory.

## 2. Key Invariants and Acceptance Criteria

Invariants:

- Secrets are not required in `.terraphim/adf.toml`.
- Legacy global `include` config continues to load and validate as before.
- A project-local agent always has `project = Some(project_id)` after conversion.
- Project-local agents run with working directory equal to the repository root.
- Duplicate agent names are allowed only when their project ids differ.
- `skill_chain` for project-scoped agents is not prompt-injected by ADF.
- A malformed enabled project source produces a clear validation error.

Acceptance criteria:

| ID | Criterion |
|----|-----------|
| AC1 | `OrchestratorConfig::from_file` can load a global config containing `[[project_sources]]`. |
| AC2 | Enabled project sources append converted project-local agents to `config.agents`. |
| AC3 | Existing configs without `project_sources` behave unchanged. |
| AC4 | `adf --check /path/to/orchestrator.toml` prints project-local agents in the routing table. |
| AC5 | `adf --local --check` still validates a repo-local `.terraphim/adf.toml`. |
| AC6 | Duplicate names across different projects validate successfully; duplicates within the same project fail. |
| AC7 | Project-scoped `skill_chain` produces no injected skill content. |
| AC8 | A missing or malformed enabled project config reports the project id and path. |

## 3. High-Level Design and Boundaries

Add a new optional global config section:

```toml
[[project_sources]]
id = "terraphim-ai"
root = "/data/projects/terraphim/terraphim-ai"
config = ".terraphim/adf.toml"
enabled = true
```

Boundary decisions:

- `config.rs` owns parsing, env substitution, loading, merging, and validation of global project sources.
- `project_adf.rs` owns parsing and conversion of one project-local ADF config.
- `bin/adf.rs` keeps local-mode behaviour and should not duplicate global loader logic.
- `lib.rs` keeps runtime prompt and spawn behaviour; project-scoped skill-chain injection remains disabled.
- No daemon hot reload is added in this increment.
- No production `conf.d` file is removed in this increment.

Merge rules:

- Load global config and includes first.
- Load enabled project sources second.
- Append converted `Project` entries to `projects`.
- Append converted `AgentDefinition` entries to `agents`.
- Merge project-local PR dispatch into `pr_dispatch_per_project[project_id]`.
- If a disabled project source is present, skip it entirely.

Validation rules:

- Project source ids must be unique.
- Project source config path must stay under the configured root unless absolute paths are explicitly allowed later.
- Project-local config `project_id` must equal source `id`.
- Agent identity uniqueness should be checked as `(project_id, agent_name)` for project-scoped agents and `(None, agent_name)` for global agents.

## 4. File/Module-Level Change Plan

| File/Module | Action | Before | After | Dependencies |
|-------------|--------|--------|-------|--------------|
| `crates/terraphim_orchestrator/src/config.rs` | Modify | No `project_sources` field | Add `ProjectSourceConfig`, parse/env substitution/load/merge/validate | `ProjectAdfConfig` |
| `crates/terraphim_orchestrator/src/project_adf.rs` | Modify | Can discover from cwd and convert local config | Add load-from-explicit-path helper and project id consistency support | `OrchestratorError` |
| `crates/terraphim_orchestrator/src/bin/adf.rs` | Minimal modify if needed | Local check uses direct discovery | Keep local mode unchanged unless helper extraction avoids duplication | `ProjectAdfConfig` |
| `crates/terraphim_orchestrator/src/lib.rs` | Verify/no major change | Project-scoped skills already skip prompt injection | Ensure tests cover project-scoped no-injection | `AgentDefinition.project` |
| `crates/terraphim_orchestrator/tests/project_source_tests.rs` | Create | No project-source integration tests | Cover AC1, AC2, AC3, AC6, AC8 | temp dirs, config loading |
| `.terraphim/adf.toml` | Use as fixture/source | Repo-local config exists | Used by manual `adf --local --check` proof | ADF CLI |
| `.docs/*bigbox-adf-simplification*.md` | Create | No formal plan | Research/design/quality gates for implementation | Disciplined workflow |

## 5. Step-by-Step Implementation Sequence

1. Add `ProjectSourceConfig` to global `OrchestratorConfig` with `id`, `root`, `config`, and `enabled` fields. Purpose: parse the registry without changing runtime behaviour. Deployable: yes.
2. Add env substitution for `project_sources`. Purpose: allow paths or flags to use existing env expansion. Deployable: yes.
3. Add explicit project-local config loading from a known root/path. Purpose: avoid cwd-dependent discovery in daemon config loading. Deployable: yes.
4. Merge enabled project-source configs after global include merge. Purpose: make project-local agents visible to normal runtime and `adf --check`. Deployable: yes.
5. Add project id consistency validation. Purpose: prevent a source id from accidentally loading a different project's config. Deployable: yes.
6. Adjust duplicate-name validation to use project-scoped identity where necessary. Purpose: unblock same agent names across projects while preserving within-project uniqueness. Deployable: yes.
7. Merge project-local PR dispatch entries into `pr_dispatch_per_project`. Purpose: let PR/event dispatch use repo-local agent mappings. Deployable: yes.
8. Add tests for legacy compatibility, project-source loading, duplicate scoping, disabled sources, and malformed source errors. Purpose: protect migration path. Deployable: yes.
9. Run verification: `cargo fmt`, `cargo test -p terraphim_orchestrator project_source`, `cargo check -p terraphim_orchestrator`, `adf --local --check`, and full pre-commit hook before commit. Deployable: yes.
10. Trigger a controlled bigbox migration follow-up only after the loader lands. Purpose: avoid changing production `/opt` config before the loader is proven. Deployable: yes.

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---------------------|-----------|---------------|
| AC1 | Unit/integration config load | `crates/terraphim_orchestrator/tests/project_source_tests.rs` |
| AC2 | Integration config merge | `crates/terraphim_orchestrator/tests/project_source_tests.rs` |
| AC3 | Regression existing fixture load | existing config tests plus new legacy fixture assertion |
| AC4 | CLI smoke/manual proof | `adf --check <temp-global-config>` |
| AC5 | CLI smoke/manual proof | `target/debug/adf --local --check` from repo root and subdirectory |
| AC6 | Validation tests | `crates/terraphim_orchestrator/tests/project_source_tests.rs` |
| AC7 | Unit regression | existing `test_load_skill_chain_content_skips_project_scoped_agents` |
| AC8 | Error-path integration test | `crates/terraphim_orchestrator/tests/project_source_tests.rs` |

Required manual proof before production cutover:

- Run `adf --local --check` in `/data/projects/terraphim/terraphim-ai`.
- Run global `adf --check /opt/ai-dark-factory/orchestrator.toml` with `GITEA_TOKEN` exported.
- Confirm routing table shows project-local agents and no duplicate-name error.
- Trigger one controlled implementation swarm only after the loader is deployed.

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| Loader breaks legacy includes | Add regression tests for configs without `project_sources` | Low |
| Duplicate names are still used as map keys elsewhere | Scope validation first, then audit runtime maps before migrating duplicate agents | Medium |
| Invalid project source breaks all projects | First implementation can fail startup clearly; later improvement can isolate failures | Medium |
| Project-local Gitea settings are incomplete | Keep global env and project source metadata for secrets; avoid tokens in repo | Low |
| Agents behave differently without prompt-injected skills | Project-scoped no-injection is already committed; prove with local and controlled bigbox runs | Medium |
| Production migration is attempted too early | Implementation issue explicitly limits scope to additive loader | Low |

## 8. Open Questions / Decisions for Human Review

1. For the first implementation, should one invalid enabled project source fail whole-daemon startup? Recommendation: yes, fail clearly for now.
2. Should `project_sources.config` allow absolute paths? Recommendation: no for first increment; keep it relative to `root`.
3. Should project-local configs declare Gitea owner/repo? Recommendation: not in first increment unless needed; keep secrets and production routing global.
4. Should hot reload be implemented now? Recommendation: no, restart/reload only.
5. Should migration of `/opt/ai-dark-factory/conf.d/terraphim.toml` happen in this issue? Recommendation: no, create a follow-up after loader validation.
