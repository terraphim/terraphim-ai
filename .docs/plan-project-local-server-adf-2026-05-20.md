# Plan: Project-Local and Server ADF from `.terraphim`

Date: 2026-05-20 10:01 BST
Status: planning
Scope: design and implementation plan only; no runtime changes in this document.

## Goal

Create a new ADF mode where a repository can define its own agent fleet under `.terraphim/`, and the same definition can run either:

- locally, for developer-triggered project agents; or
- on the ADF server, as part of the long-running multi-project fleet.

The core design principle is to extend the existing project-discovery and orchestrator configuration paths rather than create a second ADF configuration system.

## Current Evidence

### Gitea Issue Signals

Recent ADF-related issue and PR signals show the main pressure points:

- `#1674` asks for project-level `.terraphim/` config discovery and is the direct prerequisite for project-local ADF.
- `#1714`, `#1715`, `#1761`, and `#1760` show PR gate fragility around `adf/build`, `adf/pr-reviewer`, status posting, and auto-merge confidence.
- `#1754` through `#1762` are repeated `[ADF] PR gate remediation` issues, indicating the server ADF needs better dedupe and per-project gate semantics.
- `#1709` and PR `#1491` point to sandbox/executor hardening needed for agents that run code locally or on the server.
- PR `#1739` adds worktree ownership manifest protection, which should be a required part of any local/server worktree lifecycle.
- PR `#1514` adds strict ADF permission checks; project-local config must support the same validation path.
- `#1675`, `#1718`, `#1664`, and `#1655` show recurring secret-redaction risks in config, webhook, RLM, and ADF output paths.

### Codebase Signals

- `terraphim_config::project::discover()` already walks upward to find `.terraphim/` and returns its canonical path.
- `ProjectConfig` currently supports only `global_shortcut` and role overrides loaded from `.terraphim/config.json`.
- `OrchestratorConfig` already supports multi-project ADF via `[[projects]]`, per-project `working_dir`, per-project Gitea settings, include fragments, per-project PR dispatch, provider budgets, learning, evolution, and validation.
- `adf --check CONFIG` currently validates and prints the routing table for an explicit TOML config path.
- Orchestrator runtime already injects `ADF_PROJECT_ID`, `ADF_WORKING_DIR`, `GITEA_OWNER`, and `GITEA_REPO` into agent spawn contexts.

## Proposed Concept

Introduce **Project ADF**: a `.terraphim/adf.toml` file that is valid both as a developer-local ADF entrypoint and as a server-side project fragment.

The file is not a replacement for `OrchestratorConfig`. It is a project-scoped layer that compiles into the existing orchestrator model.

## Proposed `.terraphim` Layout

```text
.terraphim/
├── config.json             # existing Terraphim role/search config
├── adf.toml                # project-local ADF declaration
├── agents/                 # optional agent prompt/task templates
├── skills/                 # optional project-local skills
├── personas/               # optional project-local personas
└── adf.local.toml          # ignored/local overrides, optional
```

`adf.local.toml` should be ignored by git and used only for developer-local paths, tokens, and machine-specific limits. Secrets must still be supplied through environment variables or `op` injection, not stored directly.

## Proposed `adf.toml` Shape

```toml
[adf]
id = "terraphim-ai"
mode = "hybrid"              # local | server | hybrid
working_dir = "."            # resolved relative to repository root
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
post_status_context_prefix = "adf"

[[adf.agents]]
name = "build-runner"
layer = "Safety"
model = "sonnet"
run_policy = "on-pr-open"
local_command = "cargo test --workspace"
server_command = "cargo test --workspace"

[[adf.agents]]
name = "pr-reviewer"
layer = "Review"
model = "zai-coding-plan/glm-5.1"
run_policy = "on-pr-open"

[adf.pr_dispatch]
agents_on_pr_open = [
  { name = "build-runner", required = true },
  { name = "pr-reviewer", required = true },
]
```

This shape is intentionally project-centred and smaller than full `OrchestratorConfig`, but every field should map deterministically into the existing orchestrator TOML structures.

## Configuration Resolution

Resolution order should be explicit:

1. CLI `--config PATH`, if provided.
2. `.terraphim/adf.toml`, discovered upward from the current directory.
3. Existing `/opt/ai-dark-factory/orchestrator.toml` server default.

For project-local mode, `adf` should accept:

```text
adf --local
adf --local --check
adf --local --agent build-runner
adf --local --issue 1234
adf --local --pr 42
```

For server mode, existing `adf CONFIG` continues to work, but the server can include project fragments generated from each repository's `.terraphim/adf.toml`.

## Compilation Model

Add a small conversion layer:

```text
.terraphim/adf.toml
    -> ProjectAdfConfig
    -> OrchestratorConfig / IncludeFragment-compatible structs
    -> existing validate()
    -> existing AgentOrchestrator
```

The compiler should:

- canonicalise the project root via `terraphim_config::project::discover()`;
- set `Project.id` from `[adf].id`;
- set `Project.working_dir` to the repository root unless overridden;
- map `[adf.gitea]` to `GiteaOutputConfig`;
- map `[[adf.agents]]` to `AgentDefinition` with `project = adf.id`;
- map `[adf.pr_dispatch]` to `pr_dispatch_per_project[adf.id]`;
- resolve `.terraphim/skills` and `.terraphim/personas` relative to project root;
- preserve the same provider allow-list checks already used by `OrchestratorConfig::validate()`.

## Local Runtime Semantics

Local ADF should be a bounded, foreground-first runner:

- runs from the project repository root;
- uses a local worktree root under `.worktrees/adf-local` by default;
- limits concurrency aggressively, default `1` or `2`;
- does not auto-merge;
- may post Gitea comments/statuses only when `GITEA_TOKEN` is present;
- prints a routing table and resolved config on `--check`;
- fails closed on invalid config, banned providers, unsafe paths, and world-readable sensitive local overrides.

This supports developer workflows such as:

```text
adf --local --agent build-runner
adf --local --pr 42
adf --local --issue 1714
```

## Server Runtime Semantics

Server ADF should continue using the existing long-running orchestrator, but with one additional source of project fragments:

- server scans configured repositories for `.terraphim/adf.toml`;
- server converts each file into project/agent fragments;
- server validates the merged fleet once before startup;
- server owns PR-gate status posting and auto-merge decisions;
- server applies pause flags and project circuit breakers per project id;
- server never reads developer-local `adf.local.toml`.

## Security Requirements

- No secrets in `.terraphim/adf.toml`; use `${VAR}` placeholders.
- Warn or fail on group/world-readable `.terraphim/adf.local.toml` and any loaded token files.
- Redact webhook URLs, API keys, Gitea tokens, RLM keys, and provider credentials from all debug output.
- Restrict local command execution to the project root or managed worktrees.
- Reject path traversal in project-relative paths.
- Reuse the provider allow-list in `terraphim_orchestrator::config`.
- Treat local mode as non-privileged; only the server systemd lifecycle may run privileged cleanup.

## Implementation Phases

### Phase 1: Research and Schema

- Define `ProjectAdfConfig` in `terraphim_orchestrator::project_adf` or `terraphim_config::project` if shared outside ADF.
- Document the `.terraphim/adf.toml` schema.
- Add JSON/TOML examples for local-only, server-only, and hybrid projects.
- Decide whether `adf.local.toml` is merged by the CLI only or by a dedicated `--local-overrides` flag.

### Phase 2: Loader and Validation

- Add `ProjectAdfConfig::discover_and_load(start_dir)` using existing `.terraphim` discovery.
- Add conversion into an `OrchestratorConfig` or include-fragment-compatible type.
- Reuse `OrchestratorConfig::validate()` for duplicate project ids, banned providers, mixed mode, and project references.
- Extend `adf --check` so `adf --local --check` prints the same routing table plus resolved project root.

### Phase 3: Local Runner

- Add CLI parsing for `--local`, `--agent`, `--issue`, and `--pr`.
- Build a single-project `AgentOrchestrator` from `.terraphim/adf.toml`.
- Add a foreground execution path for one agent without starting the full reconcile loop.
- Ensure local mode does not auto-merge or post statuses unless explicitly enabled.

### Phase 4: Server Import

- Add a server-side scanner that reads `.terraphim/adf.toml` from configured repositories.
- Convert each project file into include fragments.
- Preserve current `include = ["conf.d/*.toml"]` behaviour for existing deployments.
- Add clear conflict rules when both server TOML and project `.terraphim/adf.toml` define the same project id.

### Phase 5: PR Gate and Status Contract

- Standardise status contexts: `adf/build`, `adf/pr-reviewer`, `adf/security`, and project-qualified variants when needed.
- Require non-empty agent output for success to avoid `empty_success` cases.
- Add dedupe keys for repeated `[ADF] PR gate remediation` issues.
- Ensure auto-merge confidence can reach threshold only when required contexts are posted by the correct local/server actor.

### Phase 6: Worktree and Sandbox Hardening

- Make project-local worktrees use ownership manifests from PR `#1739` once merged.
- Require local runner cleanup on normal exit and startup sweep for residue.
- For server mode, keep root-owned cleanup in systemd `ExecStartPre` only.
- Integrate Docker/RLM sandbox constraints from `#1709` and PR `#1491` before enabling arbitrary build agents by default.

## Test Plan

- Unit test `.terraphim` discovery for nested directories and symlinks.
- Unit test `ProjectAdfConfig` parsing with minimal and full examples.
- Unit test conversion from `ProjectAdfConfig` to `OrchestratorConfig`.
- Integration test `adf --local --check` from a fixture repository with `.terraphim/adf.toml`.
- Integration test local single-agent execution with a harmless command.
- Integration test server import of two fixture repositories with distinct project ids.
- Regression test banned provider rejection in project-local config.
- Regression test secret redaction in `Debug` output.
- Regression test that `adf.local.toml` is ignored by server imports.

## Acceptance Criteria

- A developer can run `adf --local --check` anywhere inside a repository and see the resolved project root, agents, models, and Gitea target.
- A developer can run one configured local agent without a server orchestrator running.
- The server can import the same project's `.terraphim/adf.toml` and run its agents as part of the existing multi-project fleet.
- `OrchestratorConfig::validate()` remains the single validation gate for provider allow-list and project-agent references.
- No secrets are printed by `--check`, debug formatting, status comments, or failure issues.
- Repeated PR gate failures produce deduped remediation issues instead of issue storms.

## Open Decisions

- Should `.terraphim/adf.toml` be TOML only, or should JSON be supported for symmetry with `.terraphim/config.json`?
- Should local mode be able to post commit statuses by default, or require `--post-status`?
- Should local mode support long-running watch/reconcile mode, or only foreground one-shot agent runs initially?
- Should server import pull project config from the checked-out repository path, Gitea raw files, or both?
- How should conflicts be resolved when server `conf.d/<project>.toml` and project `.terraphim/adf.toml` both define the same agent?

## Recommended First Slice

Implement the smallest useful vertical slice:

1. Add `ProjectAdfConfig` and parse `.terraphim/adf.toml`.
2. Convert it into a single-project `OrchestratorConfig`.
3. Add `adf --local --check`.
4. Add tests for discovery, parsing, conversion, and banned provider validation.

This validates the configuration model without changing server runtime behaviour. Local one-agent execution and server import should follow after the schema proves stable.
