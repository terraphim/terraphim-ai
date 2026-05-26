# Implementation Plan: Local .terraphim/ Config as First-Priority Source

**Status**: Draft
**Research Doc**: `.docs/research-1862-local-terraphim-config.md`
**Author**: opencode (GLM-5.1)
**Date**: 2026-05-25
**Issue**: #1862
**Estimated Effort**: 4-6 hours

## Overview

### Summary
Extend `ProjectConfig` to auto-discover individual `role-*.json` and `thesaurus-*.json` files from `.terraphim/`, then wire this discovery into all three CLI tools.

### Approach
Extend the existing `project::discover()` + `ProjectConfig` pattern rather than creating new infrastructure. Add filesystem scanning for role/thesaurus files, then consume in each tool.

### Scope
**In Scope:**
- Extend `ProjectConfig` with `load_from_dir()` to scan `role-*.json` files
- Add `discover_thesaurus()` to find `thesaurus-*.json` by role name
- Wire discovery into `terraphim_grep`, `terraphim_agent`, `terraphim_mcp_server`
- Tests for new discovery functions

**Out of Scope:**
- TOML config support
- Config file watching
- `terraphim_server` changes
- Global `~/.terraphim/` config merge

**Avoid At All Cost:**
- Adding new config file formats or manifest files
- Changing existing CLI flag behaviour (flags must still work)
- Modifying DeviceSettings persistence
- Creating new crates

## Architecture

### Component Diagram
```
.terraphim/                    terraphim_config::project
  role-*.json  ----discover---> ProjectConfig::load_from_dir()
  thesaurus-*.json                         |
  kg/                                      +---> terraphim_grep (thesaurus + role + kg)
                                          +---> terraphim_agent (role merge)
                                          +---> terraphim_mcp_server (full config)
```

### Data Flow
```
CLI invocation
  -> project::discover(None)  -- walk up from CWD
  -> found .terraphim/
  -> ProjectConfig::load_from_dir(path)  -- scan role-*.json, thesaurus-*.json
  -> merge with CLI flag overrides
  -> run tool with merged config
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Scan filesystem for role-*.json | Simple, no manifest needed | Require config.json with role list |
| Role name from filename | `role-devops.json` -> "devops" role | Embed role name in JSON content |
| Thesaurus matched by role name | `thesaurus-devops.json` matches "devops" | Single thesaurus for all roles |
| CLI flags override project config | Escape hatch for developers | Project config always wins |

### Simplicity Check

**What if this could be easy?** It IS easy. We just need:
1. A function that lists `role-*.json` files and parses them
2. A function that finds `thesaurus-<role>.json` for a given role
3. Calling these functions in main.rs of each tool

No new types, no new crates, no new formats. Just extending what exists.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_config/src/project.rs` | Add `load_from_dir()`, `discover_thesaurus_for_role()`, `discover_kg_path()` |
| `crates/terraphim_grep/src/main.rs` | Add `.terraphim/` discovery to `find_default_thesaurus()`, role config, kg path |
| `crates/terraphim_agent/src/service.rs` | Extend `merge_project_config()` to load individual role files |
| `crates/terraphim_mcp_server/src/main.rs` | Add project config discovery before hardcoded profile |

### No New Files Needed

## API Design

### New Public Functions in `project.rs`

```rust
/// Load a ProjectConfig by scanning .terraphim/ for role-*.json files.
///
/// If config.json exists, loads it first (backward compat).
/// Then scans for role-*.json files and merges them in.
/// Role name is derived from filename: role-devops.json -> "devops"
pub fn load_from_dir(dir: &Path) -> Result<ProjectConfig, ProjectDiscoveryError>

/// Find the thesaurus file for a given role in .terraphim/
///
/// Looks for .terraphim/thesaurus-<role>.json
/// Returns None if not found
pub fn discover_thesaurus(dir: &Path, role_name: &str) -> Option<PathBuf>

/// Find the KG directory within .terraphim/
///
/// Looks for .terraphim/kg/
/// Returns None if not found
pub fn discover_kg_path(dir: &Path) -> Option<PathBuf>
```

### No New Types Needed
`ProjectConfig` already has `roles: HashMap<String, Role>` -- we just populate it from individual files.

## Test Strategy

### Unit Tests (in project.rs)
| Test | Purpose |
|------|---------|
| `test_load_from_dir_reads_role_files` | Scans role-devops.json, role-rust-engineer.json |
| `test_load_from_dir_merges_with_config_json` | config.json + role-*.json both present |
| `test_load_from_dir_empty_is_ok` | No role files -> empty config |
| `test_discover_thesaurus_found` | Returns thesaurus-devops.json for "devops" |
| `test_discover_thesaurus_not_found` | Returns None for unknown role |
| `test_discover_kg_path_found` | Returns .terraphim/kg/ |
| `test_discover_kg_path_not_found` | Returns None |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_grep_uses_project_thesaurus` | terraphim_grep tests | End-to-end: grep finds thesaurus from .terraphim/ |
| `test_agent_merges_project_roles` | terraphim_agent tests | Agent loads role-*.json from project |

## Implementation Steps

### Step 1: Extend ProjectConfig (project.rs)
**Files:** `crates/terraphim_config/src/project.rs`
**Description:** Add `load_from_dir()`, `discover_thesaurus()`, `discover_kg_path()`
**Tests:** 7 unit tests above
**Estimated:** 1.5 hours

Key implementation:
```rust
pub fn load_from_dir(dir: &Path) -> Result<Self, ProjectDiscoveryError> {
    let mut config = Self {
        global_shortcut: None,
        roles: std::collections::HashMap::new(),
    };
    // Backward compat: load config.json if present
    let config_json = dir.join("config.json");
    if config_json.is_file() {
        config = Self::from_file(&config_json)?;
    }
    // Scan for role-*.json files
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("role-") && name.ends_with(".json") {
            let role_name = name.trim_start_matches("role-")
                .trim_end_matches(".json")
                .to_string();
            let role: Role = serde_json::from_str(&std::fs::read_to_string(entry.path())?)?;
            config.roles.insert(role_name, role);
        }
    }
    Ok(config)
}
```

### Step 2: Wire terraphim_grep (main.rs)
**Files:** `crates/terraphim_grep/src/main.rs`
**Description:** Extend `find_default_thesaurus()` to search `.terraphim/`, add role/kg discovery
**Tests:** Existing tests still pass; manual verification
**Dependencies:** Step 1
**Estimated:** 1 hour

Changes to `find_default_thesaurus()`:
```rust
fn find_default_thesaurus() -> Option<PathBuf> {
    // Priority 1: .terraphim/ project config
    if let Ok(Some(dir)) = terraphim_config::project::discover(None) {
        // Use role name to find matching thesaurus
        if let Some(path) = terraphim_config::project::discover_thesaurus(&dir, &role_name) {
            return Some(path);
        }
    }
    // Priority 2: filesystem heuristic (existing code)
    // ...
}
```

Also add project discovery for `--role-config` and `--kg-path` defaults.

### Step 3: Wire terraphim_agent (service.rs)
**Files:** `crates/terraphim_agent/src/service.rs`
**Description:** Extend `merge_project_config()` to use `ProjectConfig::load_from_dir()`
**Tests:** Existing tests still pass
**Dependencies:** Step 1
**Estimated:** 0.5 hours

Change `merge_project_config()`:
```rust
fn merge_project_config(config: &mut Config) {
    if let Ok(Some(path)) = terraphim_config::project::discover(None) {
        // Try load_from_dir first (scans role-*.json)
        let project_config = terraphim_config::project::ProjectConfig::load_from_dir(&path)
            .unwrap_or_else(|_| {
                // Fallback to config.json only
                let config_path = path.join("config.json");
                if config_path.is_file() {
                    terraphim_config::project::ProjectConfig::from_file(&config_path)
                        .unwrap_or_default()
                } else {
                    Default::default()
                }
            });
        if !project_config.roles.is_empty() {
            log::info!("Merging {} project roles from '{}'",
                project_config.roles.len(), path.display());
            let builder = ConfigBuilder::from_config(
                config.clone(), DeviceSettings::new(), PathBuf::new(),
            );
            *config = builder.merge_with(&project_config)
                .build().unwrap_or_else(|_| config.clone());
        }
    }
}
```

### Step 4: Wire terraphim_mcp_server (main.rs)
**Files:** `crates/terraphim_mcp_server/src/main.rs`
**Description:** Add project config discovery before hardcoded profiles
**Tests:** Existing tests still pass
**Dependencies:** Step 1
**Estimated:** 0.5 hours

```rust
// Before profile selection, try project config
if let Ok(Some(project_dir)) = terraphim_config::project::discover(None) {
    if let Ok(project_config) = ProjectConfig::load_from_dir(&project_dir) {
        if !project_config.roles.is_empty() {
            // Build Config from project roles instead of hardcoded profile
            // ...
        }
    }
}
// Fall back to --profile flag
```

### Step 5: Build, Test, Clippy
**Estimated:** 0.5 hours

## Rollback Plan
Each step is additive. If any step breaks, revert the single commit for that step. No database migrations, no breaking changes.

## Open Items
None -- research phase resolved all questions.
