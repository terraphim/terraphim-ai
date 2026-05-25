# Research Document: PR #1865 Review Fixes

**Status**: Draft
**Author**: opencode (gpt-5.5)
**Date**: 2026-05-25
**Reviewers**: PR #1865 structural review
**Issue**: #1862
**PR**: #1865

## Executive Summary

The structured review of PR #1865 found four issues that prevent local `.terraphim/` configuration from being a reliable first-priority source. Three are P1 correctness defects: a non-LLM build break in `terraphim_grep`, invalid selected/default role handling in `terraphim_mcp_server`, and missing no-flag role resolution in `terraphim_grep`. One P2 regression causes `terraphim_agent` to ignore `config.json` project settings when no role files are present.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | The current PR cannot safely merge while a supported feature combination fails to compile. |
| Leverages strengths? | Yes | The fix is local to existing config seams: `project.rs`, grep main, agent service, MCP main. |
| Meets real need? | Yes | The original requirement was local config first priority across CLI tools; the review proves gaps remain. |

**Proceed**: Yes - 3/3 YES.

## Problem Statement

### Description

PR #1865 introduced project-local config discovery but left four defects:

1. `terraphim_grep` references `terraphim_config` even when the `llm` feature is disabled.
2. `terraphim_mcp_server` merges project roles into a fresh empty config, preserving `selected_role = Default` even when `Default` is absent.
3. `terraphim_grep` defaults role resolution to `default`, so no-flag invocation misses `.terraphim/role-*.json` and `.terraphim/thesaurus-*.json` unless the role is explicitly passed.
4. `terraphim_agent` only merges project config when roles are present, regressing support for `config.json` containing only `global_shortcut`.

### Impact

- `cargo check -p terraphim_grep --no-default-features --features code-search` fails.
- MCP server can silently ignore project KG scoring because its selected role does not exist in the merged config.
- Grep still requires `--role` for projects with per-role `.terraphim/` configs, contrary to the advertised no-flag discovery behaviour.
- Existing `.terraphim/config.json` global settings can be ignored.

### Success Criteria

- `terraphim_grep` builds with `--no-default-features --features code-search`.
- Grep resolves a project role without `--role` when the project has one role or a config default.
- MCP config merging preserves or repairs selected/default role validity.
- Agent project merge preserves `config.json` settings even when no roles exist.

## Current State Analysis

### Existing Implementation

| Component | Location | Current Behaviour |
|-----------|----------|-------------------|
| Project directory scan | `crates/terraphim_config/src/project.rs` | Scans `role-*.json`, offers `discover_thesaurus()` and `discover_kg_path()`. |
| Grep thesaurus discovery | `crates/terraphim_grep/src/main.rs:135-166` | Calls `terraphim_config::project::discover_thesaurus()` unconditionally in a crate where `terraphim_config` is optional under `llm`. |
| Grep role resolution | `crates/terraphim_grep/src/main.rs:293` | Defaults missing `--role` to `default`. |
| MCP project merge | `crates/terraphim_mcp_server/src/main.rs:148-150` | Builds `base`, then ignores it by merging project roles into `ConfigBuilder::new()`. |
| Agent project merge | `crates/terraphim_agent/src/service.rs:229` | Merges only if project roles are non-empty. |

### Data Flow

Current grep path:

```text
args.role missing -> "default"
  -> find_default_thesaurus("default")
  -> looks for .terraphim/thesaurus-default.json
  -> misses role-devops.json / thesaurus-devops.json
```

Current MCP path:

```text
build_profile_config() -> base with valid selected_role
  -> ConfigBuilder::new() -> selected_role Default
  -> merge project roles only
  -> selected_role can reference absent Default
```

## Constraints

### Technical Constraints

- CLI flags must remain the highest priority.
- `terraphim_grep` must continue supporting `--no-default-features --features code-search` unless the crate deliberately changes supported feature combinations.
- The fix should stay additive and avoid new config formats.
- Tests must not use mocks.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Compile coverage | `terraphim_grep` non-LLM code-search builds | Fails |
| Role validity | selected/default roles always exist after project merge | Not guaranteed |
| No-flag usability | single/default project role discovered automatically | Not implemented |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Preserve supported feature builds | Compile failures block merge | Structural review P1 |
| Resolve project role before path lookup | Enables no-flag local config | Structural review P1 |
| Ensure selected/default role validity | Prevents silent MCP KG disablement | Structural review P1 |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| TOML project config support | Not part of the review findings. |
| Full config precedence redesign | The defects are local and can be fixed without redesign. |
| File watchers or dynamic reload | CLI startup discovery only is sufficient. |
| New manifest file | Existing `role-*.json` and `config.json` are enough. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_config::project` | Needs role resolution helper or callers need local logic | Low |
| `terraphim_grep` Cargo features | Determines whether project discovery can compile without `llm` | Medium |
| `ConfigBuilder::merge_with()` | Merges roles but does not repair selected/default role | Medium |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Multi-role project with no default is ambiguous | Medium | Medium | Return explicit error asking for `--role`, or select only when exactly one role exists. |
| Making `terraphim_config` non-optional increases dependencies for non-LLM grep | Medium | Low | Prefer if config discovery is non-LLM functionality; otherwise gate discovery. |
| Repairing selected/default incorrectly changes user intent | Low | Medium | Only repair when selected/default role is absent. |

### Open Questions

None requiring stakeholder input. The review findings are precise enough to design fixes.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `--no-default-features --features code-search` is a supported grep build | Existing feature layout and review compile command | Could over-prioritise feature compatibility | Yes - compile failure reproduced |
| `role-*.json` filename slug is a valid CLI role key | PR #1865 implementation derives keys this way | Role mismatch if JSON `name` differs | Partially |
| MCP should use project roles when present | PR description states `.terraphim/` overrides profile | Profile may be expected as base fallback | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Make `terraphim_config` non-optional in grep | Project config works in non-LLM builds | Candidate fix; simplest for local config first priority |
| Gate project config behind `llm` in grep | Restores compile only | Rejected unless dependency weight is unacceptable, because config discovery is not LLM-specific |
| In multi-role projects, silently choose first role | Convenient but nondeterministic | Rejected; should use config default or explicit `--role` |
| In multi-role projects, require default/explicit role | Deterministic behaviour | Chosen |

## Research Findings

### Key Insights

1. The compile failure is caused by a mismatch between an optional dependency and non-gated usage, not by Rust code syntax.
2. `ProjectConfig` needs one more concept: resolving the effective role slug from explicit input, project config defaults, or single-role fallback.
3. `ConfigBuilder::merge_with()` only merges roles and global shortcut; it does not repair selected/default roles, so callers that build from empty configs are responsible for validity.
4. The agent P2 regression is a one-line merge condition problem.

### Technical Spikes Needed

None. The fixes are direct.

## Recommendations

### Proceed/No-Proceed

Proceed to design. All review findings are actionable and local.

### Scope Recommendations

- Add a shared `ProjectConfig::resolve_role_name(explicit: Option<&str>) -> Result<Option<String>, ProjectDiscoveryError>` or equivalent helper.
- Make `terraphim_config` available to `terraphim_grep` outside `llm`, or gate project discovery correctly.
- Repair config selected/default role after project role merge where necessary.

### Risk Mitigation Recommendations

- Add tests for role resolution: explicit role, config default, single role, ambiguous multi-role.
- Add compile verification for grep non-LLM feature combination.
- Add MCP merge test or extracted helper test to prove selected/default role validity.

## Next Steps

1. Produce a design plan with exact file/function changes.
2. Add PR comment linking the research and design documents.
