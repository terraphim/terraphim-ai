# Research Document: PR #1788 Slice 4 Agent Registry

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-05-31
**Reviewers**: Human maintainer

## Executive Summary

Slice 4 from PR #1788 proposed a project-scoped agent registry, but the original branch only partially wired it into runtime lookups. The core problem is real: current lookup code repeatedly scans `config.agents`, and several runtime maps still use bare agent names even though multi-project configs can contain duplicate agent names such as `build-runner` per project.

The safe path is to add a read-only registry as the single lookup interface first, then migrate dispatch call sites in focused steps. Full runtime identity migration for `active_agents` should be explicitly scoped because it affects restart, timeout, worktree, mention, and PR dispatch behaviour.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | We just completed the functional slices from #1788; registry is the remaining architectural slice needed to avoid duplicated lookup logic. |
| Leverages strengths? | Yes | This is a Rust orchestration/data-modelling problem with clear type and invariance boundaries. |
| Meets real need? | Yes | Current code has many `config.agents.iter().find(...)` and `active_agents` bare-name checks while multi-project mode allows project-scoped agents with the same name. |

**Proceed**: Yes - 3/3 YES.

## Problem Statement

### Description

ADF multi-project support currently stores effective agent definitions in `OrchestratorConfig.agents` and resolves agents by repeatedly scanning that vector. This works for small configs but creates inconsistent lookup semantics across webhook dispatch, polling dispatch, PR dispatch, direct dispatch, restart logic, and budget tracking.

Slice 4 should introduce a single project-aware registry that answers: "which agent definition is authoritative for `(project, name)` or legacy `name`?" It should not silently change runtime lifecycle identity until those paths are deliberately migrated.

### Impact

If unresolved, project-scoped agents with the same name can be handled inconsistently. Known sensitive cases are:

- `build-runner` is expected to exist per project.
- Webhook/poll mention dispatch resolves using `mention::resolve_mention`, while PR/push dispatch still scans `self.config.agents` directly.
- `active_agents` is keyed by bare `String`, so concurrent `build-runner` agents from different projects can collide.
- `should_skip_dispatch`, timeout handling, restart maps, and log files operate mostly by bare names.

### Success Criteria

- A registry can be built from a validated `OrchestratorConfig`.
- Registry lookups are deterministic and scoped by project where needed.
- Duplicate names are allowed across different projects but rejected within the same scope.
- At least the direct direct-scan dispatch paths for PR/push build-runner and PR reviewer lookup use the registry.
- Any remaining bare-name lifecycle keys are documented and not accidentally hidden by the registry.
- Tests prove that lookups do not cross project boundaries.

## Current State Analysis

### Existing Implementation

The config loader merges base and project-source ADF files into one `OrchestratorConfig`. Validation enforces project references and mixed-mode constraints, but it does not currently provide a runtime index. Runtime code performs one of several lookup styles:

- Direct `self.config.agents.iter().find(...)` for PR and push dispatch.
- `mention::resolve_mention(...)` for mention-driven dispatch.
- `DirectDispatchAgentIndex::from_agents(...)` for UDS direct dispatch validation.
- `agent_key(def)` for some restart counters.
- Bare `active_agents: HashMap<String, ManagedAgent>` for live agent state.

The original Slice 4 commit `b80d98de` added `agent_registry.rs` and an `AgentOrchestrator.agent_registry` field, then replaced only two `build-runner` lookups. It also bundled TLA/spec files, which were intentionally excluded from the runtime slice.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Orchestrator config | `crates/terraphim_orchestrator/src/config.rs` | Merges TOML/project sources and validates project references. |
| Agent lifecycle | `crates/terraphim_orchestrator/src/lib.rs` | Spawns, tracks, restarts, times out, and dispatches agents. |
| Mention resolution | `crates/terraphim_orchestrator/src/mention.rs` | Resolves `@adf:[project/]name` mentions from config agents. |
| Direct dispatch index | `crates/terraphim_orchestrator/src/direct_dispatch.rs` | Pre-validates UDS dispatches using sets derived from config agents. |
| Webhook parser | `crates/terraphim_orchestrator/src/webhook.rs` | Emits `WebhookDispatch` with optional detected project. |
| Original Slice 4 diff | Git commit `b80d98de` | Adds read-only registry and partial build-runner lookup usage. |

### Data Flow

Current effective config flow:

```text
TOML files + project sources
  -> OrchestratorConfig::from_file/load_and_validate
  -> OrchestratorConfig.agents Vec<AgentDefinition>
  -> direct scans / mention resolver / direct dispatch index
  -> spawn_agent / active_agents keyed by bare name
```

Proposed registry flow:

```text
Validated OrchestratorConfig
  -> AgentRegistry::from_config(&config)
  -> lookup_project(project, name) / lookup_legacy(name) / lookup(project, name)
  -> dispatch paths clone AgentDefinition from RegisteredAgent
```

### Integration Points

- `AgentOrchestrator::new` can build the registry once from the effective config.
- PR dispatch helpers can use the registry without changing the spawner API.
- Push dispatch can use the registry without changing webhook payloads.
- Mention dispatch can initially keep `mention::resolve_mention`; later it can delegate to the registry.
- Direct dispatch can eventually build `DirectDispatchAgentIndex` from the registry instead of the config vector.

## Constraints

### Technical Constraints

- No runtime code should be written during this research/design task.
- The registry must be read-only over already merged config; it must not load TOML or mutate `OrchestratorConfig`.
- It must preserve legacy mode where no `[[projects]]` exist and agents have `project = None`.
- It must preserve multi-project validation rules currently enforced by `OrchestratorConfig::validate`.
- It must not introduce generated `.terraphim/learnings/*.md` artefacts or TLA files in this runtime slice.

### Business Constraints

- The work is a backlog-cleanup slice from #1788 and should remain independently reviewable.
- It should reduce future regression risk without destabilising already-merged Slices 1-3 and 5-7.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Lookup determinism | O(log n) or O(1) by `(scope, name)` with explicit semantics | Multiple vector scans and helper functions. |
| Scope safety | Same name in different projects resolves distinctly | Partial: config can hold it, lifecycle state still collides by bare name. |
| Backwards compatibility | Legacy configs continue resolving bare names | Current legacy path works; registry must preserve it. |

## Vital Few (Essentialism)

### Essential Constraints

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Registry is read-only and built from validated config | Avoids creating a second config source of truth | Original Slice 4 comment explicitly described this intent. |
| Project scope is part of lookup identity | Prevents `build-runner` and reviewer lookup crossing repos | Multi-project configs can define identical agent names per project. |
| Lifecycle identity migration is explicit | Bare `active_agents` keys are broader than lookup and affect many paths | `active_agents: HashMap<String, ManagedAgent>` is used in spawn, restart, timeout, stop, and mention paths. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| TLA/spec files | Tracked separately in issue #1924; they should not be bundled with runtime code. |
| Generated learning artefacts | Not source code and previously rejected from Slice 8. |
| Hot-reloadable registry | Config is currently loaded into orchestrator state; hot reload would be a separate feature. |
| Full `active_agents` key migration in the first PR | High blast radius; needs its own implementation step or second PR after lookup registry lands. |
| Source path attribution for each agent | Useful later, but merged config currently does not preserve precise source paths for every agent. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `OrchestratorConfig::validate` | Ensures project references and mixed-mode rules before registry build | Medium if callers use `AgentOrchestrator::new` with unvalidated configs. |
| `AgentDefinition` clone semantics | Registry entries will initially clone definitions | Low; current code already clones definitions at dispatch boundaries. |
| `mention::resolve_mention` | Existing mention semantics must be preserved | Medium; replacing it too early risks mention regressions. |
| `active_agents` bare keys | Runtime lifecycle collision risk remains until migrated | High if registry lookup is mistaken for complete runtime scoping. |
| `DirectDispatchAgentIndex` | Similar concept already exists for UDS validation | Low; can eventually be backed by registry methods. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Standard collections | Rust stdlib | Low | `HashMap` or `BTreeMap`; choose deterministic `BTreeMap` for predictable tests. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Registry is added but not used | Medium | Medium | Require at least PR/push dispatch paths to use it before merge. |
| Bare `active_agents` collision remains | High | High | Document as out-of-scope for first registry PR and add follow-up issue/step. |
| Duplicate agents within one project are silently accepted | Medium | Medium | Registry build should reject duplicate `(scope, name)` keys. |
| Legacy configs regress | Low | High | Add tests for `AgentKey::legacy` and legacy lookup. |
| Mention resolution changes unexpectedly | Medium | High | Do not replace mention resolver until dedicated tests cover qualified/unqualified/persona cases. |

### Open Questions

1. Should full lifecycle keys (`active_agents`, restart maps, cooldowns, logs) be migrated in the same implementation PR or split after registry lookup lands?
2. Should duplicate `(project, agent)` validation live in `OrchestratorConfig::validate`, `AgentRegistry::from_config`, or both?
3. Should `AgentRegistry` be public API (`pub use`) immediately, or remain crate-private until externally needed?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Project-scoped duplicate names are intended | Existing multi-project project-source tests and build-runner pattern | Registry could reject valid fleets | Partially verified by tests and config patterns. |
| Registry should not own loading/merging | Original Slice 4 module comment and current config architecture | Duplicated source-of-truth risk | Yes, from `b80d98de` diff. |
| First useful wiring targets PR/push build-runner and PR reviewer lookups | These are direct `config.agents.iter().find(...)` sites with project constraints | Other paths remain inconsistent longer | Yes, code inspected. |
| Active runtime state is not fully solved by lookup registry | `active_agents` uses bare `String` keys in many places | False confidence if omitted from docs | Yes, code inspected. |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Minimal registry only | Add module and tests, use for build-runner lookups only | Good first PR but insufficient if presented as complete runtime identity. |
| Registry plus all dispatch lookup migration | More coherent lookup semantics with moderate risk | Recommended if kept to lookup-only call sites. |
| Registry plus full lifecycle key migration | Solves collisions end-to-end | Rejected for first implementation due to high blast radius. |
| No registry, just helper functions | Less code | Rejected because multiple scopes and call sites benefit from a typed key and central invariants. |

## Research Findings

### Key Insights

1. The original Slice 4 registry code is conceptually sound as a read-only index but incomplete as a runtime integration.
2. `active_agents` bare-name keys are the largest unresolved design risk; they should not be hidden behind registry language.
3. `DirectDispatchAgentIndex` already models bare vs project-qualified names and can either remain separate or be implemented from registry output later.
4. `mention::resolve_mention` currently contains important semantics for project hints and should not be replaced casually.
5. `OrchestratorConfig::validate` checks project references but does not currently centralise duplicate `(scope, name)` enforcement.

### Relevant Prior Art

- Commit `b80d98de`: initial `AgentRegistry`, `AgentScope`, `AgentKey`, `RegisteredAgent` implementation and partial build-runner wiring.
- `DirectDispatchAgentIndex`: existing precomputed index for direct dispatch validation.
- `agent_key(def)`: tuple key for restart counters, demonstrating that project scope is already considered important in some lifecycle state.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Lifecycle key audit | Enumerate every `active_agents` and restart/cooldown/log key usage before full migration | 1-2 hours |
| Mention resolver replacement spike | Determine if `mention::resolve_mention` can delegate to registry without semantic drift | 1 hour |

## Recommendations

### Proceed/No-Proceed

Proceed with a focused implementation plan for a read-only `AgentRegistry` and lookup-only wiring. Do not claim that this fully solves runtime identity collisions until `active_agents` is migrated to a scoped key.

### Scope Recommendations

- First implementation PR: registry module, construction in `AgentOrchestrator::new`, tests, and direct PR/push lookup migration.
- Optional same PR if low-risk: a private helper `lookup_agent(project, name)` on `AgentOrchestrator` to reduce duplicate call-site code.
- Follow-up PR: lifecycle identity migration from bare `String` to `AgentKey` or a runtime-specific key type.

### Risk Mitigation Recommendations

- Add tests for same agent name across two projects resolving differently.
- Add tests for duplicate names within the same scope failing registry construction.
- Add tests proving legacy lookup still works.
- Keep TLA/spec work in issue #1924, outside runtime implementation.

## Next Steps

If approved:

1. Create `crates/terraphim_orchestrator/src/agent_registry.rs` with typed scope/key/entry/index definitions.
2. Build `AgentRegistry` in `AgentOrchestrator::new` from the already merged config.
3. Replace PR/push direct `config.agents.iter().find(...)` lookups with registry methods.
4. Add project-source integration tests for same-name agents in different projects.
5. Add a follow-up issue or design subsection for scoped runtime lifecycle keys.

## Appendix

### Reference Materials

- PR #1788 Slice 4 source commit: `b80d98de`.
- Runtime implementation slices already merged: PRs #1921, #1922, #1923.
- TLA/spec follow-up: issue #1924.

### Code Snippets

Current direct lookup pattern:

```rust
self.config.agents.iter().find(|a| {
    a.name == "build-runner" && a.project.as_deref() == Some(project.as_str())
})
```

Original Slice 4 proposed lookup pattern:

```rust
self.agent_registry.lookup_project(project.as_str(), "build-runner")
```

Existing lifecycle risk:

```rust
active_agents: HashMap<String, ManagedAgent>
```

The lifecycle key is still bare `String`, so registry lookup alone does not prevent active-agent collisions.
