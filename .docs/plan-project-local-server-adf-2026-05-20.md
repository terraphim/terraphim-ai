# Disciplined Plan: Project-Local and Server ADF from `.terraphim`

Date: 2026-05-20 10:10 BST
Status: implementation-ready plan
Scope: disciplined research and design for implementation; no runtime code changes in this document.
Tracking issue: `terraphim/terraphim-ai#1764`

## Phase 1 Research

### 1. Problem Restatement and Scope

Terraphim already has two related but separate configuration paths:

- project search/role configuration, discovered from `.terraphim/config.json`; and
- ADF orchestrator configuration, loaded from an explicit TOML file such as `/opt/ai-dark-factory/orchestrator.toml`.

The missing capability is a repository-owned ADF declaration that can run the same agent definitions in two contexts:

- locally, from a developer checkout, for foreground checks and one-shot agents; and
- on the server, as part of the existing multi-project ADF fleet.

The problem is not simply "add another config file". The system must avoid creating a second ADF runtime model, because provider allow-listing, project-agent validation, PR dispatch, Gitea output, worktree handling, learning, evolution, and security redaction already live in `terraphim_orchestrator::config::OrchestratorConfig` and its runtime consumers.

#### In Scope

- Define a project-owned `.terraphim/adf.toml` schema.
- Load `.terraphim/adf.toml` using existing upward `.terraphim` discovery.
- Compile project ADF config into existing `OrchestratorConfig` structures.
- Add `adf --local --check` as the first implementation slice.
- Plan local one-shot execution and server import as later slices.
- Preserve existing `adf CONFIG` and `adf --check CONFIG` behaviour.
- Reuse existing provider validation and secret redaction rules.

#### Out of Scope

- Replacing `/opt/ai-dark-factory/orchestrator.toml`.
- Replacing existing `include = ["conf.d/*.toml"]` support.
- Implementing auto-merge policy changes in the first slice.
- Enabling arbitrary project-defined shell commands before sandbox/worktree hardening is complete.
- Storing secrets in `.terraphim/adf.toml`.

### 2. User and Business Outcomes

- Developers can run `adf --local --check` from any subdirectory inside a repository and see the resolved project ADF agents, models, Gitea target, and project root.
- Developers can later run one configured project agent locally without starting the server orchestrator.
- The ADF server can later import the same project declaration into its multi-project fleet.
- Project owners can version agent definitions with the repository they protect.
- The fleet keeps one validation model for provider safety and project-agent references.
- PR-gate configuration becomes project-owned and easier to diagnose, reducing repeated `[ADF] PR gate remediation` issues.

### 3. System Elements and Dependencies

| Element | Location | Current Responsibility | Relevant Dependencies |
|---|---|---|---|
| Project discovery | `crates/terraphim_config/src/project.rs` | Finds `.terraphim/` upward from a start directory; loads role config from `config.json`. | Used by `ConfigBuilder::with_project()`. Currently JSON role config only. |
| Project role config | `.terraphim/config.json` via `ProjectConfig` | Overrides Terraphim roles and global shortcut. | `terraphim_config::Role`; not ADF-aware. |
| Orchestrator config | `crates/terraphim_orchestrator/src/config.rs` | Defines `OrchestratorConfig`, `Project`, `AgentDefinition`, `GiteaOutputConfig`, `PrDispatchConfig`, validation, include expansion. | Existing production ADF runtime and tests. |
| ADF CLI | `crates/terraphim_orchestrator/src/bin/adf.rs` | Parses `adf [CONFIG]`, `adf --check CONFIG`, help/version; prints routing table. | Calls `OrchestratorConfig::from_file()` and `validate()`. |
| Agent runtime | `crates/terraphim_orchestrator/src/lib.rs` | Spawns agents, injects `ADF_PROJECT_ID`, `ADF_WORKING_DIR`, `GITEA_OWNER`, `GITEA_REPO`, handles server loops. | Depends on `OrchestratorConfig`, `AgentSpawner`, Gitea output, worktree handling. |
| PR dispatch | `PrDispatchConfig`, `PrDispatchEntry` in `config.rs`; runtime in `pr_dispatch`, `pr_poller`, webhook paths | Maps PR-open events to agents and commit-status contexts. | `PrDispatchEntry` currently requires `name` and `context`. |
| Worktree lifecycle | `worktree_guard`, `scope`, `docs/design/adf-worktree-lifecycle-design.md` | Guards and sweeps ADF worktrees. | Current hardening issues/PRs: `#1567`, `#1739`. |
| Security checks | `warn_if_world_readable`, manual `Debug` impls for secret-bearing config | Warns on sensitive file permissions and redacts tokens/secrets. | Must cover any new token-bearing project ADF structs. |
| ADF tests | `crates/terraphim_orchestrator/tests/adf_check_tests.rs`, `tests/fixtures/multi_project/*` | Validates `adf --check` with inline and include-based multi-project TOML. | New project-local fixtures should follow this style. |

### 4. Constraints and Implications

| Constraint | Why It Matters | Implication for Good Design |
|---|---|---|
| Single validation path | Existing provider safety and project-reference checks are already in `OrchestratorConfig::validate()`. | Project ADF must compile into `OrchestratorConfig` and call `validate()`. |
| Subscription-only provider policy | Config already rejects banned provider prefixes. | Do not introduce local-only provider parsing that bypasses `validate_model_provider`. |
| Secrets cannot be committed | `.terraphim` is project-owned and likely git-tracked. | Only allow `${VAR}` placeholders; keep local secrets in env or `adf.local.toml` with permission checks. |
| Local and server semantics differ | Local mode should not auto-merge or run as a daemon by default. | Add explicit local/server capability flags and keep local first slice check-only. |
| Current PR gate fragility | Issues show missing statuses, empty success, and remediation issue storms. | Design status contexts explicitly and make PR dispatch mapping exact. |
| Backward compatibility | Existing server deployments use explicit TOML and include fragments. | Preserve `adf CONFIG`; add `.terraphim` as opt-in/local discovery path. |
| Worktree safety | Existing ADF worktree leaks have caused serious operational incidents. | Do not enable broad local execution until worktree root and cleanup semantics are explicit. |
| Rust 2024 workspace | Code must fit existing crate/module style and integration tests. | Keep the loader small and colocated with orchestrator config, avoiding new dependencies. |

### 5. Risks, Unknowns, and Assumptions

#### Assumptions

- `adf.toml` should be TOML-only for the first slice because `OrchestratorConfig` is TOML and ADF already uses TOML.
- Local mode should be check-only first, then one-shot execution, then optional watch mode later.
- Server import should ignore `adf.local.toml`.
- Project ADF should not extend `terraphim_config::ProjectConfig` directly in the first slice because ADF fields map more naturally to `terraphim_orchestrator` types.

#### Unknowns

- Whether local mode should ever post commit statuses by default.
- Whether server import should read project configs from checked-out repos, Gitea raw API, or both.
- Whether a project can override server-defined agents with the same name, or whether conflicts should fail validation.
- Whether local foreground execution can reuse the full `AgentOrchestrator` cleanly or needs a narrower `run_one_agent` API.

#### Risks and De-Risking

| Risk | De-Risking Step |
|---|---|
| Config drift between project ADF and server ADF | Compile to `OrchestratorConfig`; test conversion. |
| Secret leakage via new config structs | Manual `Debug` for token-bearing structs; regression tests. |
| Incorrect status context mapping | Require explicit `context` in project PR dispatch and test conversion. |
| Local runner mutates repository unexpectedly | Start with `--local --check`; require explicit `--agent` later. |
| Server import creates duplicate project ids | Fail validation on conflict before startup. |
| Banned providers bypass checks | Add banned-provider fixture for `.terraphim/adf.toml`. |

### 6. Simplicity Opportunities

- Treat `.terraphim/adf.toml` as a compiler input, not a runtime model.
- Use existing `terraphim_config::project::discover()` instead of adding another upward search implementation.
- Implement the first slice as `adf --local --check`, avoiding runtime execution until schema and validation are stable.

### 7. Human Review Questions

1. Should local mode post Gitea commit statuses only with an explicit `--post-status` flag?
2. Should server import fail on duplicate project ids, or should central server TOML override project `.terraphim/adf.toml`?
3. Should project ADF support `adf.local.toml` in the first slice, or defer it until local execution exists?
4. Should project ADF agent commands be allowed in the schema now, or deferred until sandboxing/worktree hardening is complete?
5. Should server import read `.terraphim/adf.toml` only from local checkouts, or also through the Gitea raw API?

## Phase 2 Design and Detailed Implementation Plan

### 1. Summary of Target Behaviour

After implementation, `adf --local --check` should:

- discover `.terraphim/` upward from the current directory;
- load `.terraphim/adf.toml`;
- resolve project-relative paths against the repository root;
- compile the project ADF config into an `OrchestratorConfig` containing exactly one `Project` plus its project-scoped agents;
- run the existing orchestrator validation path;
- print the existing routing table plus the resolved project root and config path;
- fail with exit code `1` on parse/validation errors and `2` on CLI usage errors.

Existing behaviour remains unchanged:

- `adf CONFIG` runs an explicit orchestrator config;
- `adf --check CONFIG` validates an explicit config;
- `/opt/ai-dark-factory/orchestrator.toml` remains the default when neither `--local` nor a config path is supplied.

### 2. Key Invariants and Acceptance Criteria

#### Invariants

- Project ADF config never bypasses `OrchestratorConfig::validate()`.
- All generated `AgentDefinition.project` values equal the generated `Project.id`.
- Generated `PrDispatchEntry` values include explicit `context` fields.
- Paths in `.terraphim/adf.toml` are resolved relative to the repository root and must not escape the repository unless explicitly marked as absolute by the user.
- Token fields are redacted in debug output.
- Server import must never load `adf.local.toml`.

#### Acceptance Criteria

| ID | Criterion |
|---|---|
| AC1 | Running `adf --local --check` inside a fixture repository with `.terraphim/adf.toml` exits `0` and prints project root, config path, and routing table rows. |
| AC2 | Running `adf --local --check` outside any `.terraphim` tree exits `1` with a clear discovery error. |
| AC3 | A project ADF agent using `google/gemini-2` fails validation via the existing banned-provider path. |
| AC4 | Converted agents have `project = <adf.id>` and `working_dir_for_agent()` resolves to the project root. |
| AC5 | Converted PR dispatch entries preserve explicit `context` values such as `adf/build`. |
| AC6 | Debug output for project Gitea config redacts token values. |
| AC7 | Existing `adf --check CONFIG` integration tests still pass unchanged. |

### 3. High-Level Design and Boundaries

Add a new orchestrator module, `project_adf`, responsible for project-local ADF loading and conversion.

```text
terraphim_config::project::discover()
    -> .terraphim path
    -> project root
    -> ProjectAdfConfig::from_file(.terraphim/adf.toml)
    -> ProjectAdfConfig::to_orchestrator_config(project_root)
    -> OrchestratorConfig::validate()
    -> existing CLI check table
```

This keeps responsibilities separated:

- `terraphim_config::project` remains the shared `.terraphim` discovery mechanism.
- `terraphim_orchestrator::project_adf` owns ADF-specific schema and conversion.
- `terraphim_orchestrator::config` remains the authoritative runtime config and validation layer.
- `crates/terraphim_orchestrator/src/bin/adf.rs` owns CLI mode selection and output.

### 4. Project ADF Schema for First Slice

Use a TOML shape that maps directly to existing structs and avoids unsupported fields.

```toml
[adf]
id = "terraphim-ai"
mode = "hybrid" # local | server | hybrid
working_dir = "."
skill_data_dir = ".terraphim/skills"
persona_data_dir = ".terraphim/personas"

[adf.gitea]
base_url = "https://git.terraphim.cloud"
owner = "terraphim"
repo = "terraphim-ai"
token = "${GITEA_TOKEN}"

[adf.local]
enabled = true
max_concurrent_agents = 2
worktree_root = ".worktrees/adf-local"
allowed_agents = ["build-runner", "pr-reviewer", "security-sentinel"]

[adf.server]
enabled = true
max_concurrent_agents = 4
schedule_offset_minutes = 0

[[adf.agents]]
name = "build-runner"
layer = "Safety"
cli_tool = "claude"
task = "Run the project build and tests. Report failures with exact commands."
model = "sonnet"
event_only = true

[[adf.agents]]
name = "pr-reviewer"
layer = "Core"
cli_tool = "claude"
task = "Review the pull request for correctness, security, and regression risk."
model = "zai-coding-plan/glm-5.1"
event_only = true

[adf.pr_dispatch]
agents_on_pr_open = [
  { name = "build-runner", context = "adf/build" },
  { name = "pr-reviewer", context = "adf/pr-reviewer" },
]
```

Notes:

- `layer` values must match existing `AgentLayer`: `Safety`, `Core`, or `Growth`.
- `cli_tool` and `task` are required because `AgentDefinition` requires them.
- `PrDispatchEntry` requires `name` and `context`; no `required` field exists today.
- `local_command` and `server_command` are not part of the first slice because no safe command execution contract exists yet.

### 5. File and Module Change Plan

| File/Module | Action | Before | After | Dependencies |
|---|---|---|---|---|
| `crates/terraphim_orchestrator/src/project_adf.rs` | Create | No project ADF schema. | Defines `ProjectAdfConfig`, nested structs, loader, path resolution, conversion to `OrchestratorConfig`. | `terraphim_config::project::discover`, `config.rs` types. |
| `crates/terraphim_orchestrator/src/lib.rs` | Modify | Does not expose project ADF module. | `pub mod project_adf;` and optional re-exports for tests/CLI. | New module. |
| `crates/terraphim_orchestrator/src/bin/adf.rs` | Modify | Supports explicit config, `--check`, help, version. | Adds `--local`, `--config`, optional local check path, discovery errors, enriched check output. | `project_adf` module. |
| `crates/terraphim_orchestrator/src/config.rs` | Minimal modify only if needed | `OrchestratorConfig` has no convenience constructor for generated config. | Prefer no changes; if needed, add small helper constructors only. | Avoid expanding validation surface. |
| `crates/terraphim_orchestrator/tests/project_adf_config_tests.rs` | Create | No tests for `.terraphim/adf.toml`. | Unit/integration tests for parse, conversion, redaction, banned provider, path resolution. | Test fixtures. |
| `crates/terraphim_orchestrator/tests/adf_check_tests.rs` | Modify | Tests explicit config only. | Adds `adf --local --check` fixture test while preserving existing tests. | CLI parser changes. |
| `crates/terraphim_orchestrator/tests/fixtures/project_adf/*` | Create | No project ADF fixtures. | Fixture repos with `.terraphim/adf.toml` for valid/minimal/full/invalid-banned/no-terraphim. | Tests. |
| `.docs/plan-project-local-server-adf-2026-05-20.md` | Maintain | Current plan. | Updated as implementation contract. | This change. |
| `docs/operations/` or `.docs/` schema reference | Later | No user-facing schema page. | Add after first slice if implementation accepted. | Stable schema. |

### 6. Step-by-Step Implementation Sequence

1. Create `project_adf` module skeleton.
   Purpose: isolate schema and conversion logic from CLI and runtime.
   Deployable state: yes, unused module with tests can compile.

2. Define schema structs.
   Include `ProjectAdfConfig`, `ProjectAdfRoot`, `ProjectAdfGitea`, `ProjectAdfLocal`, `ProjectAdfServer`, `ProjectAdfAgent`, and `ProjectAdfPrDispatch`.
   Use `#[serde(deny_unknown_fields)]` where safe to catch typos early.
   Deployable state: yes.

3. Implement token redaction.
   Do not derive `Debug` for token-bearing structs; implement redacted `Debug` for project Gitea config.
   Deployable state: yes.

4. Implement `ProjectAdfConfig::from_file(path)`.
   Read TOML, expand `${VAR}` consistently with orchestrator config, parse, and return structured errors.
   Deployable state: yes.

5. Implement `ProjectAdfConfig::discover_and_load(start_dir)`.
   Use `terraphim_config::project::discover(start_dir)`; expect `adf.toml` inside discovered `.terraphim`; return both config path and project root.
   Deployable state: yes.

6. Implement path resolution helper.
   Resolve relative paths against project root. Canonicalise when paths exist. Reject `..` escapes for project-local managed paths such as `worktree_root`, `skill_data_dir`, and `persona_data_dir` unless a future explicit escape flag is added.
   Deployable state: yes.

7. Implement conversion to `OrchestratorConfig`.
   Build a minimal config with one `Project`, generated agents, `pr_dispatch_per_project`, default `NightwatchConfig`, default `CompoundReviewConfig`, and inherited `gitea` only if configured.
   Deployable state: yes.

8. Call existing validation after conversion.
   Ensure generated config uses `AgentDefinition.project = Some(project_id)` and no global agents are mixed in.
   Deployable state: yes.

9. Extend CLI parser.
   Add `Cli::LocalCheck { start_dir: Option<PathBuf> }`. Support `adf --local --check`, and optionally `adf --local --check PATH` where `PATH` is a start directory for tests.
   Deployable state: yes.

10. Extend check output.
    Print `PROJECT_ROOT` and `PROJECT_CONFIG` before the existing routing table. Keep the existing table format unchanged so old tests remain stable.
    Deployable state: yes.

11. Add fixtures.
    Create minimal valid project, full valid project, banned provider project, and no-config project fixture directories.
    Deployable state: test-only.

12. Add tests.
    Cover parse, discovery, conversion, provider rejection, redaction, and CLI `--local --check`.
    Deployable state: yes.

13. Update Gitea issue `#1764` with implementation notes and test evidence.
    Deployable state: process-only.

### 7. Testing and Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---|---|---|
| AC1 local check succeeds and prints metadata | Integration | `crates/terraphim_orchestrator/tests/adf_check_tests.rs` |
| AC2 no `.terraphim` discovery fails clearly | Integration | `crates/terraphim_orchestrator/tests/adf_check_tests.rs` |
| AC3 banned provider rejected | Unit/integration | `crates/terraphim_orchestrator/tests/project_adf_config_tests.rs` |
| AC4 project id and working dir conversion correct | Unit | `crates/terraphim_orchestrator/tests/project_adf_config_tests.rs` |
| AC5 PR dispatch context preserved | Unit | `crates/terraphim_orchestrator/tests/project_adf_config_tests.rs` |
| AC6 token redaction | Unit | `crates/terraphim_orchestrator/tests/project_adf_config_tests.rs` |
| AC7 existing explicit config checks unchanged | Existing integration | `crates/terraphim_orchestrator/tests/adf_check_tests.rs` |

Manual verification after implementation:

```bash
cargo test -p terraphim_orchestrator project_adf
cargo test -p terraphim_orchestrator --test adf_check_tests
cargo run -p terraphim_orchestrator --bin adf -- --local --check
```

Coverage note: after code implementation, run the project's coverage command if available. If no project-wide coverage command is configured, record that the changed crate is covered by targeted unit and integration tests.

### 8. Risk and Complexity Review

| Risk | Mitigation | Residual Risk |
|---|---|---|
| Schema diverges from runtime config | Convert into `OrchestratorConfig`; do not build separate runtime. | Some project-friendly fields still need mapping maintenance. |
| Local execution becomes unsafe | First slice is check-only; execution requires later explicit design. | Users may expect `--local` to run agents immediately. |
| Secrets leak in debug/check output | Redacted debug implementations and tests. | Logs from later runtime execution need separate audit. |
| Project path escaping | Resolve relative paths against root and reject managed-path escapes. | Absolute paths may still be useful and need policy. |
| PR dispatch misconfiguration blocks merges | Require explicit `context`; validate referenced agents. | Required/non-required status semantics are not yet modelled. |
| Server import conflict | First slice does not implement server import; later phase must fail conflicts clearly. | Cross-repo fleet semantics remain an open design item. |

### 9. Detailed Later Phases

#### Phase 3: Local One-Shot Runner

After `--local --check` lands, add:

- `adf --local --agent NAME` for a single foreground agent.
- Optional `--issue` and `--pr` context injection.
- Local worktree root default `.worktrees/adf-local`.
- Explicit `--post-status` flag before local mode can write Gitea commit statuses.
- No auto-merge, no long-running reconcile loop.

This phase likely needs a small public method on `AgentOrchestrator` or a helper that can spawn one project-scoped agent without entering `run()`.

#### Phase 4: Server Import

Add server-side import only after project-local config is stable:

- configured list of repository working directories to scan;
- optional Gitea raw API import if local checkouts are unavailable;
- conversion into include-fragment-compatible project/agent additions;
- conflict policy: fail startup on duplicate project id by default;
- server ignores `adf.local.toml`.

#### Phase 5: PR Gate Contract

Address current issue signals:

- require non-empty success output for PR-gate agents to avoid `empty_success`;
- dedupe `[ADF] PR gate remediation` issues by `(project, pr, context, head_sha)`;
- make auto-merge confidence depend on explicit required contexts;
- keep local status posting opt-in until this contract is stable.

#### Phase 6: Worktree and Sandbox Hardening

Before enabling arbitrary project-local commands:

- integrate worktree ownership manifest protections from PR `#1739`;
- keep server root cleanup in systemd `ExecStartPre` only;
- use Docker/RLM sandbox work from `#1709` / PR `#1491` for untrusted code execution;
- document command allow-listing and path constraints.

### 10. Quality Gate Self-Evaluation

| Dimension | Score | Notes |
|---|---:|---|
| Syntactic | 4 | Terms are aligned with current structs: `AgentLayer`, `AgentDefinition`, `PrDispatchEntry`, `GiteaOutputConfig`. |
| Semantic | 4 | Plan references verified code paths and current issue signals. |
| Pragmatic | 4 | First slice is small, testable, and deployable without changing server runtime. |
| Social | 4 | Local/server responsibilities and open decisions are explicit. |
| Physical | 4 | Research and design sections are complete and structured. |
| Empirical | 4 | Tables and phased slices keep the implementation understandable. |

Verdict: GO for implementation planning, pending human decisions in section 11.

### 11. Open Decisions for Human Review

1. Should `adf.local.toml` be included in the first slice, or deferred until local execution exists?
2. Should local status posting require `--post-status`? Recommendation: yes.
3. Should duplicate server/project definitions fail startup? Recommendation: yes for project id conflicts; agent conflicts can be revisited later.
4. Should project ADF allow absolute managed paths? Recommendation: no for first slice, except `working_dir` if explicitly supplied as absolute.
5. Should the first implementation include schema documentation outside this plan? Recommendation: defer until the schema passes tests.

## Recommended Implementation Contract

Implement only the first vertical slice now:

1. Create `project_adf` schema and loader.
2. Convert `.terraphim/adf.toml` into one-project `OrchestratorConfig`.
3. Add `adf --local --check`.
4. Add fixtures and tests for discovery, parse, conversion, provider rejection, PR dispatch contexts, and redaction.

Do not implement agent execution, server import, auto-merge changes, or arbitrary command fields until the first slice is merged and validated.
