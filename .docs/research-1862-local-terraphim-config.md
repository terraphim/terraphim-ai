# Research Document: Local .terraphim/ Config as First-Priority Source

**Status**: Draft
**Author**: opencode (GLM-5.1)
**Date**: 2026-05-25
**Issue**: #1862

## Executive Summary

Three CLI tools (`terraphim_grep`, `terraphim_agent`, `terraphim_mcp_server`) do not consistently discover and prioritise local `.terraphim/` project configuration. Only `terraphim_agent` uses `project::discover()`, and it only loads `.terraphim/config.json`. The other two tools have no project config discovery at all. This means the `.terraphim/` directory we created in PR #1850 (with 3 role configs, thesauri, and KG files) is silently ignored by all tools unless manually specified via CLI flags.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Every developer working in this repo expects `terraphim-grep` to just work with local config |
| Leverages strengths? | Yes | `project::discover()` already exists -- we are extending a proven pattern |
| Meets real need? | Yes | Without this, the 3-role KG config is dead weight in `.terraphim/` |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
When a project has a `.terraphim/` directory with role configs, thesauri, and KG files, CLI tools should automatically discover and use them. Currently, each tool requires explicit CLI flags (`--thesaurus`, `--role-config`, `--profile`) to find this config.

### Impact
- Developers must remember CLI flags for each tool
- Project-level KG and thesaurus customisation is effectively unused
- The `.terraphim/` config from PR #1850 provides no value without manual wiring

### Success Criteria
Running any CLI tool from within a project that has `.terraphim/` should auto-discover:
1. Thesaurus for the current role
2. Role config (LLM settings, etc.)
3. KG path for learning and search boosting

## Current State Analysis

### Existing Implementation

| Component | Location | Purpose |
|-----------|----------|---------|
| `project::discover()` | `terraphim_config/src/project.rs:30-48` | Walks upward from CWD to find `.terraphim/` directory |
| `ProjectConfig` | `project.rs:14-20` | Holds `global_shortcut` + `roles` HashMap (JSON only) |
| `ProjectConfig::from_file()` | `project.rs:23-28` | Loads from `.terraphim/config.json` |
| `ConfigBuilder::with_project()` | `lib.rs:874-884` | Calls discover + merge |
| `merge_project_config()` | `agent/service.rs:216-236` | Agent's project config merge |
| `find_default_thesaurus()` | `grep/main.rs:215-241` | Searches `.`, `../docs/src`, `../../docs/src` for `*_thesaurus.json` |
| `build_llm_for_role()` | `grep/main.rs:135-154` | `--role-config` JSON or env vars |

### Config Priority Order (Current)

**terraphim_agent** (best case -- has project discovery):
1. `--config <path>` (JSON)
2. `role_config` in `settings.toml` (bootstrap-then-persist)
3. Persistence layer (SQLite)
4. Embedded defaults
5. `.terraphim/config.json` merged on top (lowest effective priority)

**terraphim_grep** (no project discovery):
1. `--role-config <path>` for LLM client
2. Environment variables (OPENROUTER_*, OLLAMA_*)
3. `--thesaurus <path>` or `find_default_thesaurus()` heuristic

**terraphim_mcp_server** (no project discovery):
1. `--profile desktop/server` (hardcoded ConfigBuilder profiles)

### Data Flow

```
.terraphim/
  config.toml          -- NOT read by any tool
  role-*.json           -- NOT read by any tool
  thesaurus-*.json      -- NOT read by any tool
  kg/                   -- NOT discovered by any tool (only via role.kg path)

vs. what tools actually look for:
  .terraphim/config.json -- Only checked by terraphim_agent
  *_thesaurus.json       -- Only checked by terraphim_grep (in CWD, not .terraphim/)
```

## Constraints

### Technical Constraints
1. **Backward compatibility**: CLI flags must still work and must override auto-discovered config
2. **No new dependencies**: Use existing `serde_json`, `terraphim_config`, `terraphim_settings`
3. **Feature gates**: `terraphim_grep` changes gated behind `llm` and `code-search` features
4. **Config format**: Project config is JSON (TOML `config.toml` is documentation only)

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Config discovery latency | < 10ms | N/A (not attempted) |
| Backward compat | 100% | N/A |
| Test coverage | New paths tested | 0 tests for auto-discovery |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| `.terraphim/` auto-discovery | Without this, local config is dead weight | PR #1850 config unused |
| CLI flags override auto-discovery | Developers need escape hatches | All 3 tools have existing flags |
| Single discovery implementation | DRY -- one `project.rs` serves all tools | `project::discover()` already exists |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| TOML config support | `config.toml` is documentation; all tooling uses JSON |
| Config file watching | Not needed for CLI tools |
| `terraphim_server` changes | Server uses persistence layer, not project config |
| Global `~/.terraphim/` config merge | Already handled by DeviceSettings/persistence |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_config::project` | Core discovery mechanism; needs extension | Low -- already has discover() |
| `terraphim_settings::DeviceSettings` | Agent's config loading | Low -- already used |
| `terraphim_automata::load_thesaurus` | Thesaurus loading in grep | None -- just path resolution |
| `terraphim_grep::HybridSearcher` | Needs thesaurus + role config | Low |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing CLI workflows | Low | High | CLI flags always override; tests |
| Project discovery false positives | Low | Medium | Only activates if `role-*.json` exists |
| Performance hit from filesystem walk | Low | Low | Walk stops at first `.terraphim/` found |

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `.terraphim/role-*.json` follows `terraphim_config::Role` schema | Created in PR #1850 | Parse errors | Yes |
| `.terraphim/thesaurus-*.json` follows terraphim_automata format | Created in PR #1850 | Load failures | Yes |
| Role name matches filename pattern `role-<name>.json` | Convention from PR #1850 | Discovery misses | Yes |
| `project::discover()` is cheap enough for every CLI invocation | Single upward directory walk | Latency | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Extend `ProjectConfig` to load role-*.json individually | Minimal change, additive | **Chosen** -- builds on existing pattern |
| Require `.terraphim/config.json` as single entry point | Forces config consolidation | Rejected -- we already have per-role files |
| Add a new `.terraphim/manifest.json` index | Extra file to maintain | Rejected -- filesystem scanning is sufficient |

## Research Findings

### Key Insights
1. **`project::discover()` already works** -- it finds `.terraphim/` correctly. The gap is what happens after discovery.
2. **`ProjectConfig::from_file()` only loads `config.json`** -- we need to extend it to scan for `role-*.json` and `thesaurus-*.json`.
3. **`terraphim_grep` is the simplest fix** -- just extend `find_default_thesaurus()` and add role config discovery.
4. **`terraphim_agent` already merges project config** -- just needs the merge to understand individual role files.
5. **`terraphim_mcp_server` needs the most work** -- it doesn't use DeviceSettings or project discovery at all.

### Recommended Implementation Order
1. Extend `ProjectConfig` to auto-discover role-*.json files from `.terraphim/`
2. Fix `terraphim_grep` (highest impact, simplest change)
3. Fix `terraphim_agent` (extend existing merge_project_config)
4. Fix `terraphim_mcp_server` (add project discovery)

## Next Steps

Awaiting human approval to proceed to Phase 2 (Design).
