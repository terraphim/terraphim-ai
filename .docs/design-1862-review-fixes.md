# Implementation Plan: Fix PR #1865 Review Findings

**Status**: Draft
**Research Doc**: `.docs/research-1862-review-fixes.md`
**Author**: opencode (gpt-5.5)
**Date**: 2026-05-25
**Estimated Effort**: 2-3 hours
**Issue**: #1862
**PR**: #1865

## Overview

### Summary

Fix the four findings from the structural review of PR #1865 while preserving the PR's original goal: local `.terraphim/` project configuration should be discovered automatically and should sit below explicit CLI flags but above global/default configuration.

### Approach

Make `terraphim_config` available to `terraphim_grep` outside the `llm` feature, add deterministic project role resolution, repair selected/default roles after project merges, and restore `config.json`-only merge behaviour in the agent.

### Scope

**In Scope:**
- Fix `terraphim_grep` non-LLM code-search compile failure.
- Add deterministic role resolution for no-flag grep project config.
- Ensure MCP project config merge preserves or repairs selected/default role validity.
- Ensure agent merges project config when only `global_shortcut` exists.
- Add tests for role resolution and merge conditions.

**Out of Scope:**
- TOML `.terraphim/config.toml` support.
- Large refactor of `ConfigBuilder`.
- Runtime config reload/watch behaviour.
- Changing role JSON schema.

**Avoid At All Cost:**
- Silently picking the first role in a multi-role project without an explicit/default role.
- Hiding invalid project config parse errors where they affect selected behaviour.
- Adding new config formats or manifest files.
- Breaking existing CLI flag override behaviour.

## Architecture

### Component Diagram

```text
terraphim_config::project
  load_from_dir()
  resolve_role_name(explicit?)
  ensure_selected_role_exists(config)
        |
        +--> terraphim_grep: role -> thesaurus -> role config/env fallback
        +--> terraphim_agent: merge project config if roles OR global_shortcut
        +--> terraphim_mcp_server: merge into base profile, repair selected/default
```

### Data Flow

```text
CLI args
  -> explicit --role? yes: use it
  -> project config? yes:
       config default/selected role exists? use it
       exactly one role file? use that role
       multiple roles and no default? fail with actionable error
  -> no project role: use legacy "default"
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Make `terraphim_config` non-optional for `terraphim_grep` | Project config discovery is not LLM-specific and must work in code-search-only builds | Gate project discovery behind `llm`, which would violate local config first priority for non-LLM grep |
| Add shared role resolution helper in `project.rs` | Avoid duplicating default/single/ambiguous role logic in each CLI | Inline logic in grep only |
| Repair selected/default only when invalid | Preserves user intent while avoiding missing selected role | Always override selected/default with project first role |
| Merge agent project config when global shortcut is present | Restores previous schema behaviour | Roles-only merge condition |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| New manifest file listing default role | More files to maintain | Configuration drift |
| Silent first-role fallback for multi-role projects | Nondeterministic from directory iteration | Wrong role/thesaurus selected |
| Making MCP use only project config with no base | Loses fallback profile settings | Missing selected/default role |

### Simplicity Check

The simplest design is to add two small shared helpers and use them from callers:

- `ProjectConfig::resolve_role_name(explicit_role)` for deterministic role slug selection.
- `repair_selected_role(config)` or equivalent local helper for config validity after merge.

This keeps fixes local and avoids speculative abstractions.

**Nothing Speculative Checklist:**
- [x] No features the user didn't request
- [x] No abstractions for unknown future config sources
- [x] No new config format
- [x] No premature optimisation

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_config/src/project.rs` | Add role resolution helper and tests. Possibly add helper for whether config is non-empty. |
| `crates/terraphim_grep/Cargo.toml` | Move `terraphim_config` from optional `llm` dependency to normal dependency, or add a non-LLM feature dependency path. |
| `crates/terraphim_grep/src/main.rs` | Use project role resolution before thesaurus lookup; update role-config path lookup to resolved role. |
| `crates/terraphim_agent/src/service.rs` | Merge project config when roles are present OR `global_shortcut` exists. |
| `crates/terraphim_mcp_server/src/main.rs` | Merge project config into base profile and repair selected/default role if invalid. |

### New Files

None.

## API Design

### `ProjectConfig::is_empty()`

```rust
impl ProjectConfig {
    pub fn is_empty(&self) -> bool {
        self.global_shortcut.is_none() && self.roles.is_empty()
    }
}
```

### `ProjectConfig::resolve_role_name()`

```rust
impl ProjectConfig {
    /// Resolve an effective project role key.
    ///
    /// Priority:
    /// 1. Explicit CLI role if provided.
    /// 2. `selected_role` or `default_role` from config.json if it exists in project roles.
    /// 3. The only role if exactly one `role-*.json` exists.
    /// 4. None when no project role applies.
    /// 5. Error when multiple roles exist and no default/explicit role resolves.
    pub fn resolve_role_name(
        &self,
        explicit_role: Option<&str>,
    ) -> Result<Option<String>, ProjectDiscoveryError>;
}
```

### `ProjectDiscoveryError::AmbiguousRole`

```rust
#[error("multiple project roles found ({available:?}); pass --role or set selected/default role")]
AmbiguousRole { available: Vec<String> },
```

### MCP selected-role repair helper

Keep private to `terraphim_mcp_server/src/main.rs` unless another caller needs it later:

```rust
fn repair_selected_roles(config: &mut terraphim_config::Config) {
    if config.roles.contains_key(&config.selected_role)
        && config.roles.contains_key(&config.default_role) {
        return;
    }
    if let Some(first_role) = config.roles.keys().next().cloned() {
        if !config.roles.contains_key(&config.selected_role) {
            config.selected_role = first_role.clone();
        }
        if !config.roles.contains_key(&config.default_role) {
            config.default_role = first_role;
        }
    }
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_project_config_is_empty_with_no_roles_or_shortcut` | `project.rs` | Validates merge condition helper. |
| `test_project_config_is_not_empty_with_shortcut_only` | `project.rs` | Prevents agent regression. |
| `test_resolve_role_prefers_explicit_role` | `project.rs` | CLI flags remain highest priority. |
| `test_resolve_role_uses_single_project_role` | `project.rs` | Enables no-flag one-role grep config. |
| `test_resolve_role_ambiguous_multi_role_without_default` | `project.rs` | Prevents nondeterministic selection. |
| `test_resolve_role_uses_config_default_when_present` | `project.rs` | Enables deterministic multi-role project config. |
| `test_repair_selected_roles_sets_missing_selected_to_existing_role` | `terraphim_mcp_server/src/main.rs` | Ensures MCP selected/default validity. |

### Build/Compile Verification

| Command | Purpose |
|---------|---------|
| `cargo check -p terraphim_grep --no-default-features --features code-search` | Proves non-LLM grep build is restored. |
| `cargo test -p terraphim_config -- project` | Project helper tests. |
| `cargo test -p terraphim_grep --features "code-search llm" --lib` | Existing grep tests. |
| `cargo clippy -p terraphim_config -p terraphim_grep -p terraphim_agent -p terraphim_mcp_server -- -D warnings` | Quality gate. |

## Implementation Steps

### Step 1: Strengthen `ProjectConfig` Helpers

**Files:** `crates/terraphim_config/src/project.rs`

**Description:**
- Add `AmbiguousRole` error variant.
- Add `ProjectConfig::is_empty()`.
- Add `ProjectConfig::resolve_role_name(explicit_role)`.
- Add tests for explicit role, single role, ambiguous role, and default/selected role.

**Dependencies:** None.

**Expected Review Finding Addressed:** P1 grep no-flag role discovery, P2 agent merge condition.

### Step 2: Fix `terraphim_grep` Feature Gate and Role Resolution

**Files:** `crates/terraphim_grep/Cargo.toml`, `crates/terraphim_grep/src/main.rs`

**Description:**
- Make `terraphim_config` available for project discovery in non-LLM builds.
- Resolve `role_name` before thesaurus lookup:
  - explicit `--role` wins;
  - project config default/single role is used when available;
  - ambiguous multi-role project returns a clear error;
  - fallback remains `default` when no project roles exist.
- Use the resolved role for both `discover_thesaurus()` and `.terraphim/role-<role>.json` lookup.

**Dependencies:** Step 1.

**Expected Review Findings Addressed:** P1 compile failure, P1 no-flag local config discovery.

### Step 3: Fix Agent Merge Condition

**Files:** `crates/terraphim_agent/src/service.rs`

**Description:**
- Replace `if !project_config.roles.is_empty()` with `if !project_config.is_empty()`.
- Keep existing merge mechanics.

**Dependencies:** Step 1.

**Expected Review Finding Addressed:** P2 `config.json` shortcut-only regression.

### Step 4: Fix MCP Project Merge Validity

**Files:** `crates/terraphim_mcp_server/src/main.rs`

**Description:**
- Merge project config into `base`, not into `ConfigBuilder::new()`.
- Either add `terraphim_settings` as a dependency and use `ConfigBuilder::from_config(base, DeviceSettings::new(), PathBuf::new())`, or manually merge roles/global shortcut into `base` to avoid a new dependency.
- Prefer manual merge to keep dependency footprint unchanged.
- Add `repair_selected_roles(&mut config)` after merge.

**Dependencies:** Step 1 optional only for `is_empty()` if used.

**Expected Review Finding Addressed:** P1 invalid selected/default role.

### Step 5: Verification and PR Update

**Files:** none beyond test updates.

**Description:**
- Run targeted tests and build commands listed above.
- Commit fixes with `Refs #1862`.
- Push branch and comment on PR #1865 summarising resolved findings.
- Request structural re-review.

## Rollback Plan

All changes are additive and local. If a fix regresses behaviour, revert the last fix commit and retain the research/design docs for a revised plan.

## Dependencies

### New Dependencies

None preferred.

### Dependency Updates

| Crate | Change | Reason |
|-------|--------|--------|
| `terraphim_grep` | `terraphim_config` from optional to normal dependency | Project config discovery is non-LLM functionality and is needed in code-search-only builds. |

## Performance Considerations

Project role resolution scans data already loaded by `load_from_dir()`. No additional directory walks beyond existing discovery are required if callers reuse the loaded project config.

## Open Items

None.

## Approval

- [x] Technical review findings mapped to fixes
- [x] Test strategy defined
- [x] No new external dependencies planned
- [ ] Human approval received
