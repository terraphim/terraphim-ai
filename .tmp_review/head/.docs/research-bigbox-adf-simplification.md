# Research Document: Bigbox ADF Orchestrator Simplification

## 1. Problem Restatement and Scope

The bigbox ADF deployment currently concentrates too many responsibilities in `/opt/ai-dark-factory/orchestrator.toml` and `/opt/ai-dark-factory/conf.d/*.toml`. Fleet runtime settings, project identity, repository paths, agent schedules, large task prompts, skill metadata, Gitea routing, and project workflow policy are all configured in production-local TOML. This makes project changes operationally heavy, hard to review, and prone to drift.

The problem is not that ADF cannot run agents. The problem is that project-specific behaviour is encoded as production daemon configuration rather than versioned project configuration. Recent work added `.terraphim/adf.toml` discovery and local validation, which creates an opportunity to make bigbox ADF a thin fleet runtime that loads project-owned configuration from each repository.

IN scope:

- Define how bigbox should discover and load project-local `.terraphim/adf.toml` files.
- Preserve global fleet controls in `/opt/ai-dark-factory`.
- Move project-specific agent definitions, schedules, tasks, and PR dispatch mappings into repositories.
- Treat project-scoped `skill_chain` as metadata only, relying on native Claude/opencode project skill loading.
- Validate project-scoped agent identity and avoid duplicate flat-name collisions.
- Provide a safe migration path for `terraphim-ai` without deleting production config prematurely.

OUT of scope:

- Replacing Gitea task tracking.
- Rewriting provider routing, budget tracking, or Quickwit logging.
- Changing Claude/opencode CLI semantics.
- Implementing live hot reload in the first increment.
- Migrating every project in one change.

## 2. User & Business Outcomes

Expected outcomes:

- Project ADF behaviour is versioned, reviewed, and shipped with the repository that owns it.
- Bigbox production config becomes smaller and easier to audit.
- Operators can validate project config with `adf --local --check` before daemon rollout.
- Agent prompt sizes shrink because ADF no longer inlines project skill documents.
- Duplicate agent names like `merge-coordinator` can coexist across projects when scoped by project id.
- Swarm changes require normal branch/PR workflow instead of manual `/opt` edits.

Business value:

- Less deployment friction for agent improvements.
- Lower risk of production-only drift.
- Better auditability for autonomous implementation agents.
- Faster recovery and rollback because project config changes are ordinary git changes.

## 3. System Elements and Dependencies

| Element | Location | Role | Dependencies |
|---------|----------|------|--------------|
| Global ADF config | `/opt/ai-dark-factory/orchestrator.toml` | Fleet runtime config | systemd env, global provider settings, include files |
| Global project conf.d | `/opt/ai-dark-factory/conf.d/*.toml` | Current project and agent definitions | `OrchestratorConfig::from_file`, include merge |
| Project ADF config | `.terraphim/adf.toml` | Repo-owned project agents and PR dispatch | `ProjectAdfConfig::discover_and_load` |
| Project ADF conversion | `crates/terraphim_orchestrator/src/project_adf.rs` | Converts local config to `Project` and `AgentDefinition` | `AgentLayer`, `Project`, `AgentDefinition` |
| ADF CLI local mode | `crates/terraphim_orchestrator/src/bin/adf.rs` | Validates and runs local project agents | `ProjectAdfConfig`, `AgentSpawner` |
| Orchestrator config loader | `crates/terraphim_orchestrator/src/config.rs` | Loads global TOML and includes | merge logic, validation |
| Agent prompt assembly | `crates/terraphim_orchestrator/src/lib.rs` | Builds task prompt before spawn | persona, skill_chain, learning, evolution |
| Project runtime routing | `crates/terraphim_orchestrator/src/lib.rs` and related modules | Chooses working directory and env | `Project.id`, `AgentDefinition.project` |
| Gitea workflow | Gitea issue comments and `gtr` | Task trigger and implementation tracking | `GITEA_URL`, `GITEA_TOKEN` |
| Native CLI skills | Claude/opencode project mechanisms | Load project skill instructions | working directory and CLI config |

## 4. Constraints and Their Implications

| Constraint | Why It Matters | Implication |
|------------|----------------|-------------|
| Secrets must not enter repo-local TOML | Project config is committed to git | Tokens stay in systemd/env/global config only |
| Bigbox daemon must stay deployable during migration | ADF is operational infrastructure | Loader must be additive before production config is removed |
| Existing `conf.d` includes must keep working | Other projects depend on current behaviour | Project source loading must not break legacy includes |
| Duplicate names already exist | `merge-coordinator` appears in multiple projects | Validation and runtime identity need project scoping |
| Project skills should be native CLI concerns | ADF prompt injection caused large prompts | Do not add a second ADF skill package manager |
| Agents need correct working directory | Native skill loading and git commands are cwd-sensitive | Project-local agents must run at repo root, not `.terraphim/` |
| Bigbox validation must catch bad configs before dispatch | A bad project config should not silently break a project | `adf --check` should report project source load failures clearly |
| Rollback must be simple | Production daemon config changes are risky | Keep old `conf.d` path until project-source path is proven |

## 5. Risks, Unknowns, and Assumptions

### Risks

| Risk | Severity | De-risking Step |
|------|----------|-----------------|
| Project source loader changes global validation semantics | High | Add focused config tests covering legacy-only, project-source-only, and mixed modes |
| Duplicate agent names still collide in scheduler/status/output paths | High | Audit all agent-keyed maps and logs for project scoping before migrating duplicate names |
| Bigbox agents do not see project-local config if repo is stale | Medium | Document deployment sequence: pull repo before daemon check/restart |
| Project-local config cannot express required production Gitea settings | Medium | Keep Gitea token and owner/repo defaults in global project source metadata or env |
| Native CLI project skills differ between Claude and opencode | Medium | Validate with controlled `adf --local --agent` smoke tests before real swarm cutover |
| Removing prompt-injected skills changes agent behaviour | Medium | First migrate implementation swarm only and compare prompt size + output quality |

### Unknowns

- Whether every agent-keyed structure already uses project id where needed.
- Whether global `pr_dispatch_per_project` can cleanly consume project-local `pr_dispatch` entries.
- Whether production bigbox should fail the whole daemon on one invalid project source or disable only that project.
- Whether `adf --check` should print the project source origin for each agent in the routing table.

### Assumptions

- ASSUMPTION: `.terraphim/adf.toml` is the intended repo-local project configuration format.
- ASSUMPTION: Bigbox repositories are present under stable paths such as `/data/projects/terraphim/terraphim-ai`.
- ASSUMPTION: Claude and opencode project skill loading works when the spawned process cwd is the repo root.
- ASSUMPTION: Existing `conf.d` include loading must remain supported for at least one migration cycle.

## 6. Context Complexity vs. Simplicity Opportunities

Complexity sources:

- Global config mixes fleet controls with project-specific behaviour.
- ADF currently has legacy global agents and project-scoped agents.
- Agent names are treated as globally unique in some areas, while the operational model is becoming project-scoped.
- Skills exist in multiple places: Gitea cache, global paths, user paths, and now native project paths.

Simplification opportunities:

- Introduce a single `project_sources` registry in global config and load repo-owned project config from there.
- Keep ADF responsible for orchestration only; leave project skill instruction loading to Claude/opencode.
- Migrate project by project, with old global config retained until each repo-local config is validated.

## 7. Questions for Human Reviewer

1. Should an invalid project source fail the whole daemon startup, or should ADF disable only that project and continue running other projects?
2. Should `project_sources` include Gitea owner/repo metadata, or should that remain inside each project `.terraphim/adf.toml` without tokens?
3. Should project-local configs be reloaded only on daemon restart for the first increment?
4. Should the routing table include config origin paths to make production validation auditable?
5. Which Terraphim agent should be migrated first after implementation-swarm-A/B: PR reviewers or merge-coordinator?
6. Should duplicate agent names be allowed only across different projects, or should globally scoped agents still reserve names?
7. Should the first implementation include a controlled `echo` local-agent smoke test fixture in addition to config validation tests?
