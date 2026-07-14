# Terraphim AI Agent Configuration Planning Manifest

Status: planning only. This document proposes a Terraphim AI agent configuration and crate/dependency manifest. It does not implement or activate any agent configuration.

## Source Comparison

The requested Odilo path `~/projects/zestic-ai/odilo` was not present. The active Odilo repository used for comparison was `/home/alex/projects/zestic-at/odilo`.

Odilo patterns worth adopting:

- `manifest.yaml` is the canonical component registry for agent routing, ownership, dependencies, deployment targets, secrets, policies, and compatible skills.
- Root `AGENTS.md` defines an on-ramp: read manifest, component `.agent` contract, policies, global boundaries, build docs, and file summaries before changing code.
- `.agent/BOUNDARIES.md` defines repo-wide no-touch paths and command constraints.
- `.agent/POLICIES.md` maps component policies to merge/push gates such as `production-code-pairing`, `auto-mode-ok`, and `human-review-only`.
- `.agents/skills/` stores project-local skills so dispatched agents get reproducible behaviour independent of global skill drift.

Terraphim-specific patterns already present:

- `.terraphim/orchestrator.toml.bigbox` is the deployed ADF-style root config and includes `conf.d/*.toml`.
- `scripts/adf-setup/agents/*.toml` contains version-controlled agent templates.
- `crates/terraphim_orchestrator/orchestrator.example.toml` documents the larger fleet shape, routing, pre-checks, and Gitea posting conventions.
- Gitea is the task authority for `terraphim/terraphim-ai` at `https://git.terraphim.cloud`.

## Proposed Configuration Shape

Create a Terraphim equivalent of the Odilo manifest rather than adding more ad hoc agent TOML:

- `manifest.yaml`: canonical component and dependency registry for this repository.
- `.agent/BOUNDARIES.md`: Terraphim-specific global boundaries, including no `.env` edits, no machine-local Cargo overrides committed, and no changes to extracted polyrepo ownership without an issue/ADR.
- `.agent/POLICIES.md`: policy definitions aligned with Gitea PageRank workflow and ADF merge gates.
- `.agents/README.md`: project-local skill inventory and provenance.
- `.agents/skills/`: only if reproducible project-local skill pinning is required; otherwise reference existing global skills.
- `.terraphim/adf.toml`: project source config already referenced by `.terraphim/orchestrator.toml.bigbox`; should become the project-specific manifest-to-ADF bridge if not already present.
- `conf.d/*.toml`: generated or hand-curated concrete agent definitions after the manifest is approved.

No implementation should happen until the manifest schema, component list, and policy mapping are approved.

## Proposed Agent Policies

- `production-code-pairing`: human approval required; use for server runtime, Gitea runner, orchestrator, merge coordination, registry publishing, and deployment-sensitive crates.
- `auto-mode-ok`: CI-gated autonomous implementation allowed; use for tooling crates with low blast radius and strong test coverage.
- `human-review-only`: agents may analyse and propose changes but must not merge; use for governance docs, manifest/policy files, release/versioning decisions, private registry configuration, and security boundaries.
- `event-only`: agents can only be dispatched by orchestrator events; use for build runner, PR reviewer, merge coordinator, and webhook/status agents.

## Proposed Agent Fleet

The fleet should be narrower than the generic example and biased towards Terraphim's actual risks: polyrepo registry boundaries, Gitea automation, Rust workspace quality, and ADF reliability.

| Agent | Dispatch | Policy | Purpose | Template Basis |
| --- | --- | --- | --- | --- |
| `build-runner` | push/PR event | `event-only` | deterministic `fmt`, `clippy`, and tests via `rch` | `scripts/adf-setup/agents/build-runner.toml` |
| `pr-reviewer` | PR event | `event-only` | bounded structural PR review and `adf:gate-result` output | `scripts/adf-setup/agents/pr-reviewer.toml` |
| `merge-coordinator` | PR event/comment | `production-code-pairing` | coordinate required checks and issue closure | `scripts/adf-setup/agents/merge-coordinator.toml` |
| `repo-steward` | scheduled | `human-review-only` | detect stale branches, duplicate issues, dependency drift, and documentation drift | `scripts/adf-setup/agents/repo-steward.toml` |
| `runtime-guardian` | scheduled/event | `production-code-pairing` | monitor orchestrator/systemd/Quickwit/webhook health | `scripts/adf-setup/agents/runtime-guardian.toml` |
| `registry-steward` | scheduled/manual | `human-review-only` | audit private `terraphim` registry versions, local-vs-registry duplication, and publish readiness | new proposed template |
| `gitea-runner-steward` | scheduled/manual | `production-code-pairing` | maintain native Gitea runner and Gitea API integration | new proposed template |
| `product-development` | scheduled | `human-review-only` | PageRank prioritisation only; no code, no branches, no PRs | `scripts/adf-setup/agents/product-development.toml` |

Recommended first implementation slice after approval:

1. Add `manifest.yaml`, `.agent/BOUNDARIES.md`, and `.agent/POLICIES.md` only.
2. Add `registry-steward.toml` and `gitea-runner-steward.toml` as disabled templates.
3. Wire enabled agents only after validating the manifest with a small schema/test.

## Cargo Registry And Dependency Sources

Cargo registry configuration is in `.cargo/config.toml`:

- crates.io uses sparse protocol.
- private registry `terraphim` points at `sparse+https://git.terraphim.cloud/api/packages/terraphim/cargo/`.
- reads and publishes require `CARGO_REGISTRIES_TERRAPHIM_TOKEN="Bearer $GITEA_TOKEN"`.

Resolved dependency source counts from `cargo metadata --format-version 1`:

| Source | Count | Notes |
| --- | ---: | --- |
| crates.io | 930 | normal third-party transitive dependency set |
| local/path | 17 | active workspace members in this checkout |
| `terraphim` private registry | 14 | extracted Terraphim crates consumed from Gitea Cargo registry |
| git | 2 | patched dependencies from GitHub |

Private `terraphim` registry packages currently resolved:

- `terraphim-markdown-parser` 1.20.2
- `terraphim_agent_evolution` 1.20.2
- `terraphim_automata` 1.20.2
- `terraphim_config` 1.20.2
- `terraphim_mcp_search` 0.1.0
- `terraphim_orchestrator` 1.20.2
- `terraphim_persistence` 1.20.2
- `terraphim_rolegraph` 1.20.2
- `terraphim_router` 1.20.2
- `terraphim_service` 1.20.5
- `terraphim_settings` 1.20.2
- `terraphim_spawner` 1.21.0
- `terraphim_tracker` 1.20.2
- `terraphim_types` 1.20.2

Git dependencies currently resolved:

- `rustls-webpki` 0.103.12 from `https://github.com/rustls/webpki.git`, tag `v/0.103.12`.
- `self_update` 0.42.0 from `https://github.com/AlexMikhalev/self_update.git`, branch `update-zipsign-api-v0.2`.

## Local Cargo Manifest Inventory

Active workspace members:

| Manifest | Package | Category |
| --- | --- | --- |
| `terraphim_server/Cargo.toml` | `terraphim_server` | server/runtime |
| `terraphim_firecracker/Cargo.toml` | `terraphim-firecracker` | sandbox/runtime |
| `terraphim_ai_nodejs/Cargo.toml` | `terraphim_ai_nodejs` | Node integration |
| `crates/terraphim_dsm/Cargo.toml` | `terraphim_dsm` | architecture analysis |
| `crates/terraphim_eval_check/Cargo.toml` | `terraphim_eval_check` | evaluation tooling |
| `crates/terraphim_github_runner/Cargo.toml` | `terraphim_github_runner` | CI runner |
| `crates/terraphim_lsp/Cargo.toml` | `terraphim_lsp` | LSP tooling |
| `crates/terraphim_mcp_search/Cargo.toml` | `terraphim_mcp_search` | MCP/search |
| `crates/terraphim_merge_coordinator/Cargo.toml` | `terraphim_merge_coordinator` | merge automation |
| `crates/terraphim_rlm/Cargo.toml` | `terraphim_rlm` | recursive language model/sandbox |
| `crates/terraphim_sessions/Cargo.toml` | `terraphim_sessions` | session search/history |
| `crates/terraphim_spawner/Cargo.toml` | `terraphim_spawner` | process spawning |
| `crates/terraphim_tinyclaw/Cargo.toml` | `terraphim_tinyclaw` | agent/proxy runtime |
| `crates/terraphim_update/Cargo.toml` | `terraphim_update` | updater |
| `crates/terraphim_validation/Cargo.toml` | `terraphim_validation` | validation tooling |
| `crates/terraphim_weather_report/Cargo.toml` | `terraphim_weather_report` | weather/reporting |
| `crates/terraphim_workspace/Cargo.toml` | `terraphim_workspace` | workspace tooling |

Local manifests present but not active workspace members:

- `crates/terraphim_orchestrator/Cargo.toml`: excluded from this workspace; Bigbox builds orchestrator from `/home/alex/projects/terraphim/terraphim-agents` per `AGENTS.md`.
- `crates/terraphim_gitea_runner/Cargo.toml`: native Gitea runner source exists but is excluded from workspace builds.
- `crates/terraphim_github_runner_server/Cargo.toml`: host runner server, excluded.
- `crates/terraphim_agent_application/Cargo.toml`: experimental, excluded.
- `crates/terraphim_automata_py/Cargo.toml` and `crates/terraphim_rolegraph_py/Cargo.toml`: Python bindings, excluded.
- `crates/haystack_atlassian/Cargo.toml` and `crates/haystack_discourse/Cargo.toml`: future haystack providers, excluded.
- `crates/terraphim_symphony/Cargo.toml`: separate build path.
- `browser_extensions/TerraphimAIParseExtension/wasm/Cargo.toml`: browser extension WASM build.
- `vs-code-terraphim-it/rust-lib/Cargo.toml`: VS Code integration Rust library.
- `lab/parking-lot/terraphim-grep/Cargo.toml`: parked lab source.

## Direct Private Registry References By Manifest

These manifests directly reference the `terraphim` private registry and need registry-aware agent handling:

- Root `Cargo.toml`: `[patch.crates-io] terraphim_service = { version = "1.20.5", registry = "terraphim" }`.
- `crates/haystack_atlassian/Cargo.toml`: `haystack_core`, `terraphim_types`.
- `crates/haystack_discourse/Cargo.toml`: `haystack_core`, `terraphim_types`.
- `crates/terraphim_agent_application/Cargo.toml`: agent supervisor, messaging, registry, KG orchestration, task decomposition, goal alignment, and `terraphim_types`.
- `crates/terraphim_automata_py/Cargo.toml`: `terraphim_automata`, `terraphim_types`.
- `crates/terraphim_github_runner/Cargo.toml`: optional `terraphim_agent_evolution`.
- `crates/terraphim_github_runner_server/Cargo.toml`: `terraphim_service`, `terraphim_config`, `terraphim_test_utils`.
- `crates/terraphim_lsp/Cargo.toml`: `terraphim_automata`, `terraphim_types`, `terraphim_rolegraph`.
- `crates/terraphim_rlm/Cargo.toml`: optional `terraphim_mcp_search`.
- `crates/terraphim_rolegraph_py/Cargo.toml`: `terraphim_rolegraph`, `terraphim_types`, `terraphim_automata`.
- `crates/terraphim_sessions/Cargo.toml`: session analyser, markdown parser, automata, rolegraph, and types.
- `crates/terraphim_spawner/Cargo.toml`: `terraphim_types`.
- `crates/terraphim_symphony/Cargo.toml`: `terraphim_tracker`.
- `crates/terraphim_tinyclaw/Cargo.toml`: `terraphim_mcp_search`.
- `crates/terraphim_validation/Cargo.toml`: `terraphim_config`, `terraphim_types`.
- `crates/terraphim_weather_report/Cargo.toml`: `terraphim_automata`, `terraphim_orchestrator`, `terraphim_types`.

## Gitea-Specific Sources And Risks

Gitea-relevant local sources:

- `crates/terraphim_gitea_runner`: native Gitea Actions runner; present but excluded from workspace builds.
- `crates/terraphim_github_runner`: shared runner stack with Gitea payload support comments.
- `crates/terraphim_merge_coordinator`: merge automation and PR coordination.
- `crates/terraphim_orchestrator`: ADF dispatch, Gitea skill loader, PR gate, webhook, commit-status, and mention handling source; excluded in this workspace.
- `.cargo/config.toml`: private Gitea Cargo registry configuration.
- `.terraphim/orchestrator.toml.bigbox`: Gitea tracker configuration for `terraphim/terraphim-ai`.

Agent planning implications:

- Any agent touching registry configuration, `Cargo.lock`, or `Cargo.toml` entries with `registry = "terraphim"` should use `production-code-pairing` or `human-review-only` until registry drift checks are automated.
- The duplicated names `terraphim_mcp_search` and `terraphim_spawner` appear as both local workspace packages and private registry packages in the resolved graph. A `registry-steward` should detect and explain these cases before any version bump.
- `terraphim_gitea_runner` is present locally but excluded; a `gitea-runner-steward` should not assume it is covered by `cargo test --workspace`.

## Proposed Manifest Components

Initial `manifest.yaml` components should be coarse, not one component per crate:

| Component | Source Paths | Policy | Notes |
| --- | --- | --- | --- |
| `server-runtime` | `terraphim_server`, `terraphim_ai_nodejs` | `production-code-pairing` | user-facing runtime and integration surface |
| `sandbox-runtime` | `terraphim_firecracker`, `crates/terraphim_rlm` | `production-code-pairing` | sandbox/security sensitive |
| `adf-orchestration` | `crates/terraphim_orchestrator`, `.terraphim`, `scripts/adf-setup` | `production-code-pairing` | runtime deployed from agents repo; do not commit Bigbox-only overrides |
| `gitea-automation` | `crates/terraphim_gitea_runner`, `crates/terraphim_github_runner`, `crates/terraphim_merge_coordinator` | `production-code-pairing` | CI and merge status authority |
| `registry-and-polyrepo` | `Cargo.toml`, `Cargo.lock`, `.cargo/config.toml`, registry-consuming manifests | `human-review-only` | private registry and extracted crate boundary |
| `search-and-kg` | `crates/terraphim_mcp_search`, `crates/terraphim_lsp`, `crates/terraphim_sessions`, `.terraphim/kg` | `auto-mode-ok` with registry guard | core search and KG tooling |
| `validation-and-quality` | `crates/terraphim_validation`, `crates/terraphim_eval_check`, `crates/terraphim_dsm` | `auto-mode-ok` | quality tooling, with coverage evidence required |
| `desktop-and-extensions` | `desktop`, `browser_extensions`, `vs-code-terraphim-it` | `production-code-pairing` | separate build/test commands |
| `experimental-and-parked` | `lab/parking-lot`, `crates/terraphim_agent_application`, `crates/terraphim_symphony` | `human-review-only` | avoid accidental activation |
| `documentation-and-governance` | `.docs`, `docs`, `AGENTS.md`, manifest/policy files | `human-review-only` | governance artefacts |

## Validation Plan Before Implementation

Before implementing any config changes:

1. Confirm whether Terraphim should copy Odilo's exact `manifest.yaml` schema or use a Terraphim-specific schema.
2. Confirm whether project-local `.agents/skills` should be vendored, or whether global skills remain authoritative.
3. Add a manifest validator test that checks source paths exist, policies are defined, and registry-sensitive components are not `auto-mode-ok`.
4. Run `cargo metadata --format-version 1` in CI to generate the dependency source inventory and fail on unexpected source drift.
5. Add a specific check for local-vs-registry duplicate crate names.
6. Only then generate or update `conf.d/*.toml` agent configs.

## Non-Goals For This Planning Pass

- Do not edit `.terraphim/orchestrator.toml.bigbox`.
- Do not add or enable any `conf.d/*.toml` agent.
- Do not change `Cargo.toml`, `Cargo.lock`, `.cargo/config.toml`, or registry dependencies.
- Do not create Gitea issues or PRs until the manifest plan is reviewed.
- Do not commit this planning artefact unless explicitly requested.
