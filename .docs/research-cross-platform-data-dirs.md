# Research Document: Cross-Platform Data Directory Conventions for Terraphim AI

**Status**: Draft
**Author**: Terraphim AI Research Agent
**Date**: 2026-04-17
**Reviewers**: [Pending]

## Executive Summary

This document analyses cross-platform data directory conventions across macOS, Linux, and Windows, focusing on the Rust `dirs` and `directories` crates. It maps all current Terraphim codebase usage of directory functions, compares platform-specific conventions, and provides evidence-based recommendations for where Terraphim should store its shared learning knowledge graph (KG), configuration, and application data. The key finding is that Terraphim currently uses an inconsistent mix of `dirs::data_dir()`, `dirs::config_dir()`, `dirs::home_dir()`, and `directories::ProjectDirs`, creating a risk of data fragmentation. A unified approach using `ProjectDirs` for namespaced paths, with `dirs` as a fallback, is recommended.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Learning KG is a core differentiator for Terraphim; correct storage ensures portability and user trust |
| Leverages strengths? | Yes | We already use both `dirs` and `directories` crates; this research unifies our approach |
| Meets real need? | Yes | Shared KG path must work across agents (Claude, Codex, Opencode) and platforms; current ad-hoc paths will break |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Description
Terraphim AI needs a single, predictable location to store:
1. **Configuration** (settings.toml, role configs, update history)
2. **Application Data** (SQLite databases, knowledge graphs, learnings)
3. **Shared Learning KG** (markdown learnings accessible across multiple AI agents)
4. **Cache** (temporary indices, downloaded automata)

Currently, different parts of the codebase use different directory functions, leading to inconsistent paths. For example, `terraphim_settings` uses `directories::ProjectDirs` to derive `~/.config/terraphim/` and `~/.local/share/terraphim/` on Linux, while `terraphim_agent::learnings` uses `dirs::data_dir()` directly with a manual `.join("terraphim/learnings")`, and `terraphim_router` uses `dirs::home_dir().join(".terraphim/providers")`.

### Impact
- **User confusion**: Data is scattered across multiple directories
- **Cross-agent failure**: Shared KG path may not resolve consistently
- **Platform non-compliance**: macOS and Windows have specific conventions that `~/.terraphim/` violates
- **Container/docker issues**: Hardcoded home directory assumptions break in containerised environments

### Success Criteria
- All Terraphim data stored in platform-appropriate, standard locations
- Shared learning KG path is consistent regardless of which agent creates it
- Containerised environments can override paths via environment variables
- No breaking changes to existing user data (migration path identified)

---

## Current State Analysis

### Dependency Versions

| Crate | Version | Used By | Purpose |
|-------|---------|---------|---------|
| `dirs` | 5.0 | `terraphim_agent`, `terraphim_update`, `terraphim_hooks`, `terraphim_router`, `terraphim_tinyclaw`, `terraphim_validation` | Low-level directory paths |
| `directories` | 6.0.0 | `terraphim_settings` | High-level `ProjectDirs` for namespaced paths |

> **Note**: The `directories` crate depends on `dirs-sys` internally, so both crates share the same underlying platform resolution logic. The version discrepancy (5.0 vs 6.0.0) is because `directories` 6.0.0 pulls in a newer `dirs-sys` but the `dirs` crate 5.0 still works. We should consider aligning `dirs` to 6.0 to match `directories`.

### Complete Codebase Usage Map

#### 1. `terraphim_settings` (uses `directories::ProjectDirs`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/lib.rs:157` | `ProjectDirs::from("com", "aks", "terraphim").data_dir()` | Linux: `~/.local/share/terraphim/`<br>macOS: `~/Library/Application Support/com.aks.terraphim/`<br>Windows: `%APPDATA%\terraphim\` | Default data path for SQLite storage |
| `src/lib.rs:192` | `ProjectDirs::from("com", "aks", "terraphim").config_dir()` | Linux: `~/.config/terraphim/`<br>macOS: `~/Library/Application Support/com.aks.terraphim/`<br>Windows: `%APPDATA%\terraphim\` | Config directory for settings.toml |

**Key code**:
```rust
let data_dir = if let Some(proj_dirs) = ProjectDirs::from("com", "aks", "terraphim") {
    proj_dirs.data_dir().to_string_lossy().to_string()
} else if let Ok(home) = std::env::var("HOME") {
    format!("{}/.terraphim", home)
} else {
    "/tmp/terraphim_embedded".to_string()
};
```

**Analysis**: This is the most correct approach. `ProjectDirs` provides OS-appropriate namespacing via the reverse-domain qualifier (`com.aks.terraphim`). The fallback chain (`ProjectDirs` → `$HOME/.terraphim` → `/tmp`) is sensible but the `/tmp` fallback is dangerous (data loss on reboot).

#### 2. `terraphim_agent::learnings` (uses `dirs::data_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/learnings/mod.rs:85` | `dirs::data_dir()` + manual join | Linux: `~/.local/share/terraphim/learnings`<br>macOS: `~/Library/Application Support/terraphim/learnings`<br>Windows: `%APPDATA%\terraphim\learnings` | Global learning capture directory |

**Key code**:
```rust
let global_dir = dirs::data_dir()
    .unwrap_or_else(|| PathBuf::from("~/.local/share"))
    .join("terraphim")
    .join("learnings");
```

**Analysis**: Uses `dirs::data_dir()` directly instead of `ProjectDirs`. This means:
- On macOS, the path lacks the bundle identifier namespace (`com.aks.terraphim`)
- On Windows, it uses `%APPDATA%` (roaming) rather than `%LOCALAPPDATA%` (local)
- The fallback `"~/.local/share"` is a literal string, not expanded; if `dirs::data_dir()` returns `None`, this creates a directory literally named `~` in the current working directory

#### 3. `terraphim_agent::REPL handler` (uses `dirs::data_local_dir()` and `dirs::home_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/repl/handler.rs:133` | `dirs::home_dir()` | `~/.terraphim_history` | REPL history file |
| `src/repl/handler.rs:1733` | `dirs::data_local_dir()` | Linux: `~/.local/share/terraphim/backups`<br>macOS: `~/Library/Application Support/terraphim/backups`<br>Windows: `%LOCALAPPDATA%\terraphim\backups` | Backup directory |
| `src/repl/handler.rs:1763` | `dirs::data_local_dir()` | Same as above | Backup directory (duplicate) |

**Analysis**: The REPL history is stored directly in `~/.terraphim_history` (Unix convention, no platform abstraction). The backup uses `data_local_dir()` which is correct for non-roaming data. However, it also manually joins `terraphim` without the bundle namespace.

#### 4. `terraphim_agent::main.rs` (uses `dirs::cache_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/main.rs:975` | `dirs::cache_dir()` | Linux: `~/.cache/terraphim`<br>macOS: `~/Library/Caches/terraphim`<br>Windows: `%LOCALAPPDATA%\terraphim\cache` | Cache directory |

**Analysis**: Correct use of `cache_dir()` for ephemeral data. The manual `terraphim` subfolder lacks namespacing on macOS/Windows.

#### 5. `terraphim_agent::onboarding::validation` (uses `dirs::home_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/onboarding/validation.rs:147` | `dirs::home_dir()` | `~/.terraphim` | Onboarding validation path |
| `src/onboarding/validation.rs:151` | `dirs::home_dir()` | `~/.terraphim` | Onboarding validation path |

**Analysis**: Uses `~/.terraphim` directly. This is the old Unix convention and violates macOS/Windows guidelines.

#### 6. `terraphim_agent::learnings::install` (uses `dirs::config_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/learnings/install.rs:43` | `dirs::config_dir()` + `join("claude")` | Linux: `~/.config/claude`<br>macOS: `~/Library/Application Support/claude`<br>Windows: `%APPDATA%\claude` | Claude Code config directory |
| `src/learnings/install.rs:44` | `dirs::config_dir()` + `join("codex")` | Linux: `~/.config/codex`<br>macOS: `~/Library/Application Support/codex`<br>Windows: `%APPDATA%\codex` | Codex config directory |
| `src/learnings/install.rs:45` | `dirs::config_dir()` + `join("opencode")` | Linux: `~/.config/opencode`<br>macOS: `~/Library/Application Support/opencode`<br>Windows: `%APPDATA%\opencode` | Opencode config directory |

**Analysis**: These are correct for discovering OTHER tools' config directories. Claude Code actually uses `~/.claude/` (not `dirs::config_dir()`), so this is an area for improvement.

#### 7. `terraphim_hooks::discovery` (uses `dirs::home_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/discovery.rs:47` | `dirs::home_dir()` | `~/.cargo/bin/terraphim-agent` | Cargo binary discovery |

**Analysis**: Correct use for cargo home directory check.

#### 8. `terraphim_router::registry` (uses `dirs::home_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/registry.rs:426` | `dirs::home_dir()` | `~/.terraphim/providers/` | Provider registry persistence |

**Analysis**: Uses `~/.terraphim/providers/` directly. This is ad-hoc and platform non-compliant.

#### 9. `terraphim_sessions::connector::native` (uses `dirs::home_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/connector/native.rs:49` | `dirs::home_dir()` | `~/.claude/projects` | Claude Code session import |

**Analysis**: Claude Code actually stores sessions at `~/.claude/projects/` on all platforms (verified). This is correct because Claude Code itself does not follow platform conventions and uses a hardcoded Unix-style path.

#### 10. `terraphim_tinyclaw::commands` (uses `dirs::config_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/commands/mod.rs:122` | `dirs::config_dir()` + `join("terraphim/commands")` | Linux: `~/.config/terraphim/commands`<br>macOS: `~/Library/Application Support/terraphim/commands`<br>Windows: `%APPDATA%\terraphim\commands` | Markdown command search path |

**Analysis**: Correct use of `config_dir()` for user-defined commands. Lacks bundle namespace on macOS.

#### 11. `terraphim_tinyclaw::tools::voice_transcribe` (uses `dirs::data_local_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/tools/voice_transcribe.rs:39` | `dirs::data_local_dir()` | Platform-specific local data dir | Voice transcription data |

#### 12. `terraphim_update::state` (uses `dirs::config_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/state.rs:16` | `dirs::config_dir()` + `join("terraphim/update_history.json")` | Linux: `~/.config/terraphim/update_history.json`<br>macOS: `~/Library/Application Support/terraphim/update_history.json`<br>Windows: `%APPDATA%\terraphim\update_history.json` | Update history persistence |

**Analysis**: Update history is arguably data, not config. However, since it is a small JSON file and is user-specific, `config_dir()` is acceptable. `data_local_dir()` would also be correct.

#### 13. `terraphim_validation::testing::desktop_ui` (uses `dirs::data_dir()`)

| Location | Function | Path Constructed | Purpose |
|----------|----------|------------------|---------|
| `src/testing/desktop_ui/utils.rs:203` | `dirs::data_dir()` | Platform-specific data dir | Desktop UI test paths |
| `src/testing/desktop_ui/utils.rs:209` | `dirs::data_dir()` fallback `%APPDATA%` | Windows fallback | Windows test path |
| `src/testing/desktop_ui/utils.rs:214` | `dirs::data_dir()` fallback `~/.local/share` | Linux fallback | Linux test path |

---

## Platform Conventions Deep Dive

### The `dirs` Crate: What It Provides

The `dirs` crate (v6.0.0, docs at https://docs.rs/dirs) provides 18 low-level functions that return `Option<PathBuf>` for platform-standard directories. It is a minimal API with no namespacing -- you get the raw platform directory and must construct your app's subdirectory manually.

#### Key Functions for Terraphim

| Function | Linux | macOS | Windows | Use Case |
|----------|-------|-------|---------|----------|
| `dirs::home_dir()` | `$HOME` | `$HOME` | `{FOLDERID_Profile}` | User's home directory |
| `dirs::config_dir()` | `$XDG_CONFIG_HOME` or `~/.config` | `~/Library/Application Support` | `%APPDATA%` | Configuration files |
| `dirs::config_local_dir()` | `$XDG_CONFIG_HOME` or `~/.config` | `~/Library/Application Support` | `%LOCALAPPDATA%` | Local-only config |
| `dirs::data_dir()` | `$XDG_DATA_HOME` or `~/.local/share` | `~/Library/Application Support` | `%APPDATA%` | User data (roaming) |
| `dirs::data_local_dir()` | `$XDG_DATA_HOME` or `~/.local/share` | `~/Library/Application Support` | `%LOCALAPPDATA%` | User data (local) |
| `dirs::cache_dir()` | `$XDG_CACHE_HOME` or `~/.cache` | `~/Library/Caches` | `%LOCALAPPDATA%\cache` | Cache/temporary data |
| `dirs::preference_dir()` | N/A | `~/Library/Preferences` | N/A | macOS preferences only |

#### Critical Differences: `data_dir()` vs `data_local_dir()`

On **Linux** and **macOS**, these return the **same path**. The distinction only matters on **Windows**:

- `dirs::data_dir()` → `%APPDATA%` (Roaming AppData)
  - Synced across domain-joined machines via roaming profiles
  - Use for: small config files, user preferences, bookmarks
  - **Do NOT use for**: large databases, cache files, machine-specific data

- `dirs::data_local_dir()` → `%LOCALAPPDATA%` (Local AppData)
  - NOT synced across machines
  - Use for: large databases, SQLite files, machine-specific indices, cached downloads
  - This is where most modern Windows apps store their data

**Recommendation for Terraphim**: Use `data_local_dir()` for SQLite databases, knowledge graphs, and learnings. Use `config_dir()` for settings.toml and small JSON files.

#### The `directories::ProjectDirs` Alternative

The `directories` crate (which depends on `dirs`) provides `ProjectDirs::from(qualifier, organisation, application)`. This is the **recommended approach** for applications because it:

1. Automatically namespaces your app with a reverse-domain qualifier
2. Constructs all standard paths for your app in one call
3. Follows platform conventions precisely:

| Method | Linux | macOS | Windows |
|--------|-------|-------|---------|
| `project_dirs.config_dir()` | `~/.config/terraphim` | `~/Library/Application Support/com.aks.terraphim` | `%APPDATA%\terraphim\terraphim\config` |
| `project_dirs.data_dir()` | `~/.local/share/terraphim` | `~/Library/Application Support/com.aks.terraphim` | `%APPDATA%\terraphim\terraphim\data` |
| `project_dirs.cache_dir()` | `~/.cache/terraphim` | `~/Library/Caches/com.aks.terraphim` | `%LOCALAPPDATA%\terraphim\terraphim\cache` |

> **Note**: The `directories` crate v6.0.0 produces the paths above. The Windows paths include the organisation and application name as subdirectories, which is the standard Microsoft convention.

### Platform-Specific Convention Details

#### macOS

Apple's [File System Programming Guide](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/MacOSXDirectories/MacOSXDirectories.html) specifies:

- `~/Library/Application Support/` -- "Contains all app-specific data and support files... By convention, all of these items should be put in a subdirectory whose name matches the bundle identifier of the app."
- `~/Library/Preferences/` -- "Contains the user's preferences. You should never create files in this directory yourself. To get or set preference values, you should always use the `NSUserDefaults` class."
- `~/Library/Caches/` -- "Contains cached data that can be regenerated as needed."

**Key insight**: macOS does NOT distinguish between "config" and "data" at the filesystem level like Linux does. Both go in `Application Support/`. The `dirs::config_dir()` and `dirs::data_dir()` both return `~/Library/Application Support/` on macOS. Using `ProjectDirs` adds the bundle identifier (`com.aks.terraphim`) which is the Apple-recommended namespacing.

#### Linux (XDG Base Directory Specification)

The [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) defines:

- `$XDG_CONFIG_HOME` (default `~/.config`) -- User-specific configuration files
- `$XDG_DATA_HOME` (default `~/.local/share`) -- User-specific data files
- `$XDG_CACHE_HOME` (default `~/.cache`) -- Non-essential cached data
- `$XDG_STATE_HOME` (default `~/.local/state`) -- State data that should persist between restarts but is not user configuration (logs, history)

**Key insight**: Linux makes a clean separation between config and data. Users can set `XDG_CONFIG_HOME` and `XDG_DATA_HOME` to different locations (e.g., config on a synced dotfiles repo, data on local SSD). Hardcoding paths or conflating config/data breaks this separation.

#### Windows

Windows uses the [Known Folder API](https://msdn.microsoft.com/en-us/library/windows/desktop/bb776911(v=vs.85).aspx):

- `%APPDATA%` (`{FOLDERID_RoamingAppData}`) -- Roaming application data
- `%LOCALAPPDATA%` (`{FOLDERID_LocalAppData}`) -- Local (non-roaming) application data
- `%PROGRAMDATA%` (`{FOLDERID_ProgramData}`) -- System-wide application data (all users)

**Key insight**: Windows is the only platform where `data_dir()` vs `data_local_dir()` matters. For Terraphim's SQLite databases (potentially large), `data_local_dir()` is strongly preferred to avoid syncing multi-gigabyte files across domain networks.

---

## How Other Tools Handle This

### Claude Code (Anthropic)

- **Sessions**: `~/.claude/projects/` (hardcoded Unix-style path on ALL platforms)
- **Config**: `~/.claude/CLAUDE.md` (global instructions)
- **Approach**: Ignores platform conventions entirely; uses a dot-directory in the user's home folder
- **Verdict**: Simple but non-compliant. Works because Claude Code is primarily a developer tool used on Unix-like systems.

### Aider

- **Config**: `~/.aider.conf.yml` or `.aider.conf.yml` in project root
- **Chat history**: `.aider.chat.history.md` in project directory
- **Input history**: `.aider.input.history` in project directory
- **Approach**: Project-local by default, with optional global config in home directory
- **Verdict**: Good for project-specific tools. Not suitable for Terraphim's cross-project shared KG.

### Cargo / Rust Toolchain

- **Config**: `~/.cargo/config.toml`
- **Binaries**: `~/.cargo/bin/`
- **Approach**: Uses `~/.cargo/` universally, following the Unix dot-directory convention even on Windows
- **Verdict**: Acceptable for developer tools, but not ideal for user-facing applications.

### VS Code / Cline

- **macOS**: `~/Library/Application Support/Code/User/globalStorage/`
- **Linux**: `~/.config/Code/User/globalStorage/`
- **Windows**: `%APPDATA%\Code\User\globalStorage\`
- **Approach**: Uses Electron's `app.getPath('userData')` which follows platform conventions
- **Verdict**: The gold standard for cross-platform applications.

---

## Recommendations

### 1. Unified Directory Strategy

**Adopt `directories::ProjectDirs` as the primary mechanism** for all Terraphim path resolution. This provides:
- Proper namespacing on all platforms
- Consistent path construction
- No manual string concatenation

**Use `dirs` crate only for**:
- Discovering OTHER tools' directories (e.g., `~/.claude/`, `~/.cargo/bin/`)
- When you specifically need the raw platform directory without app namespacing

### 2. Recommended Path Assignments

| Data Type | Primary Location | Fallback | Rationale |
|-----------|------------------|----------|-----------|
| **Settings / Config** | `ProjectDirs::config_dir()` | `dirs::config_dir().join("terraphim")` | Small files, user-editable |
| **SQLite Database** | `ProjectDirs::data_local_dir()` | `dirs::data_local_dir().join("terraphim")` | Large, local-only, not roaming |
| **Knowledge Graphs** | `ProjectDirs::data_local_dir().join("kg")` | Same | Large data, built from source |
| **Shared Learnings** | `ProjectDirs::data_local_dir().join("shared-learnings")` | Same | Markdown files, cross-agent |
| **Cache** | `ProjectDirs::cache_dir()` | `dirs::cache_dir().join("terraphim")` | Regenerable data |
| **Update History** | `ProjectDirs::data_local_dir().join("update_history.json")` | Same | Small but machine-specific |
| **REPL History** | `ProjectDirs::data_local_dir().join("repl_history.txt")` | Same | User-specific state |
| **Provider Registry** | `ProjectDirs::data_local_dir().join("providers")` | Same | Plugin data |

### 3. Shared Learning KG Path Recommendation

The shared learning KG should be stored in:

```rust
use directories::ProjectDirs;
use std::path::PathBuf;

pub fn shared_learning_kg_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "aks", "terraphim")
        .map(|pd| pd.data_local_dir().join("shared-learnings"))
}
```

**Why `data_local_dir()` and not `data_dir()`?**
- Learnings are potentially large (markdown files with embedded context)
- They are machine-specific (captured from local sessions)
- On Windows, we do NOT want them synced via roaming profiles
- On Linux/macOS, `data_local_dir()` and `data_dir()` resolve to the same path anyway

**Why `ProjectDirs` and not manual `dirs::data_local_dir().join("terraphim")`?**
- On macOS, Apple recommends the bundle identifier namespace (`com.aks.terraphim`)
- On Windows, `ProjectDirs` creates the proper subdirectory structure
- Consistent with `terraphim_settings` which already uses `ProjectDirs`

**Environment variable override**:
```rust
pub fn shared_learning_kg_path() -> PathBuf {
    std::env::var("TERRAPHIM_SHARED_KG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            ProjectDirs::from("com", "aks", "terraphim")
                .map(|pd| pd.data_local_dir().join("shared-learnings"))
                .unwrap_or_else(|| {
                    dirs::data_local_dir()
                        .unwrap_or_else(|| PathBuf::from("/tmp"))
                        .join("terraphim")
                        .join("shared-learnings")
                })
        })
}
```

### 4. Containerised Environment Handling

In Docker/containerised environments, the `dirs` crate relies on:
- `$HOME` environment variable
- On Linux: `$XDG_DATA_HOME`, `$XDG_CONFIG_HOME`

**Best practices for containers**:

```dockerfile
# Set a known home directory
ENV HOME=/home/terraphim
ENV XDG_DATA_HOME=/home/terraphim/.local/share
ENV XDG_CONFIG_HOME=/home/terraphim/.config

# Create the user and directories
RUN useradd -m -d /home/terraphim terraphim
```

**For read-only containers** (common in serverless):
```rust
pub fn get_data_path() -> PathBuf {
    std::env::var("TERRAPHIM_DATA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| shared_learning_kg_path())
}
```

The `TERRAPHIM_DATA_PATH` env var should be documented as the primary override mechanism for containers, Kubernetes ConfigMaps, and CI/CD pipelines.

### 5. WSL Considerations

Windows Subsystem for Linux has special behaviour:
- `$HOME` in WSL is the Linux home (`/home/username`)
- Windows paths are accessible via `/mnt/c/`
- `dirs` crate in WSL returns Linux paths, NOT Windows paths

**Implication**: If a user runs Terraphim in both Windows native and WSL, they will have TWO separate data directories. This is generally correct because:
- SQLite databases may not be portable across Windows/Linux builds
- File paths stored in the database will differ

**If cross-WSL sharing is desired**, document that users should set `TERRAPHIM_DATA_PATH` to a shared location (e.g., `/mnt/c/Users/username/AppData/Local/terraphim`).

### 6. Migration Path from Current Paths

To avoid breaking existing users, implement a migration helper:

```rust
pub fn migrate_old_paths() -> std::io::Result<()> {
    let old_home_dot_dir = dirs::home_dir()
        .map(|h| h.join(".terraphim"));

    let new_project_dir = ProjectDirs::from("com", "aks", "terraphim")
        .map(|pd| pd.data_local_dir().to_path_buf());

    if let (Some(old), Some(new)) = (old_home_dot_dir, new_project_dir) {
        if old.exists() && !new.exists() {
            log::info!("Migrating old data from {:?} to {:?}", old, new);
            std::fs::create_dir_all(new.parent().unwrap())?;
            // Use std::fs::rename for atomic move (same filesystem)
            // Or copy for cross-filesystem
        }
    }

    Ok(())
}
```

---

## Code Examples

### Example 1: Correct Cross-Platform Path Resolution

```rust
use directories::ProjectDirs;
use std::path::PathBuf;

pub struct TerraphimPaths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
    pub shared_learnings: PathBuf,
}

impl TerraphimPaths {
    pub fn new() -> Option<Self> {
        ProjectDirs::from("com", "aks", "terraphim").map(|pd| Self {
            config: pd.config_dir().to_path_buf(),
            data: pd.data_local_dir().to_path_buf(),
            cache: pd.cache_dir().to_path_buf(),
            shared_learnings: pd.data_local_dir().join("shared-learnings"),
        })
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config)?;
        std::fs::create_dir_all(&self.data)?;
        std::fs::create_dir_all(&self.cache)?;
        std::fs::create_dir_all(&self.shared_learnings)?;
        Ok(())
    }
}
```

### Example 2: Environment-Aware Path with Fallbacks

```rust
use std::path::PathBuf;

pub fn get_shared_kg_path() -> PathBuf {
    // 1. Explicit env override (highest priority)
    if let Ok(path) = std::env::var("TERRAPHIM_SHARED_KG_PATH") {
        return PathBuf::from(path);
    }

    // 2. ProjectDirs (preferred)
    if let Some(pd) = directories::ProjectDirs::from("com", "aks", "terraphim") {
        return pd.data_local_dir().join("shared-learnings");
    }

    // 3. dirs fallback
    if let Some(data_dir) = dirs::data_local_dir() {
        return data_dir.join("terraphim").join("shared-learnings");
    }

    // 4. Final fallback (current directory)
    PathBuf::from("terraphim-shared-learnings")
}
```

### Example 3: Per-Platform Display Path (for UI)

```rust
pub fn display_path(path: &std::path::Path) -> String {
    let path_str = path.to_string_lossy();

    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if path_str.starts_with(home_str.as_ref()) {
            return path_str.replacen(home_str.as_ref(), "~", 1);
        }
    }

    path_str.to_string()
}
```

---

## Edge Cases and Risk Matrix

| Edge Case | Risk | Mitigation |
|-----------|------|------------|
| **Container with no `$HOME`** | `dirs` returns `None`; app panics or uses `/tmp` | Always use fallback chain; never unwrap `dirs` results without fallback |
| **Read-only filesystem** | `create_dir_all` fails | Catch error, log warning, allow read-only operation |
| **Windows roaming profile** | Large KG synced across network | Use `data_local_dir()` not `data_dir()` |
| **macOS sandboxed app** | `Application Support` may be redirected | `ProjectDirs` handles this correctly via system APIs |
| **WSL/Windows dual boot** | Two separate data stores | Document `TERRAPHIM_DATA_PATH` for shared location |
| **Flatpak/Snap packages** | `$HOME` is sandboxed | These tools set `XDG_DATA_HOME` appropriately; respect it |
| **Network home directory (Linux)** | `~/.local/share` may be on NFS | `data_local_dir()` returns correct path; use it |
| **Long paths on Windows** | `MAX_PATH` (260 char) limit | Enable long path support in Windows manifest; use `\\?\` prefix if needed |

---

## Open Questions

1. **Should we align `dirs` version to 6.0?** Currently `terraphim_agent` uses `dirs = "5.0"` while `terraphim_settings` uses `directories = "6.0.0"`. Should we upgrade `dirs` to 6.0 for consistency?

2. **Should we deprecate `~/.terraphim` entirely?** The onboarding validation and router registry still use it. Do we need a migration tool for existing users?

3. **Should shared learnings be in `data_dir()` or `config_dir()` on Linux?** The XDG spec is clear (data goes in `XDG_DATA_HOME`), but some users expect learnings to be in `.config/` because they are "text configs". What is the user expectation?

4. **How should the shared KG behave in CI/CD?** Should `TERRAPHIM_SHARED_KG_PATH` default to the current project's `.terraphim/shared-learnings/` when running in CI?

---

## Next Steps

1. **Create a `terraphim_paths` utility crate** that centralises all path resolution using `ProjectDirs` with fallback chains
2. **Audit all crates** to replace ad-hoc `dirs::home_dir().join(".terraphim")` usage with the centralised utility
3. **Upgrade `dirs` from 5.0 to 6.0** in `terraphim_agent` to match `directories` 6.0.0
4. **Document `TERRAPHIM_SHARED_KG_PATH`** and other env overrides in the user-facing documentation
5. **Implement migration helper** for users with existing `~/.terraphim/` data
6. **Add container test** that verifies Terraphim works correctly when `$HOME` is set to a non-standard location

---

## Appendix A: Complete Platform Path Reference

### Linux (XDG-compliant)

| Function | Env Var | Default Path | Example |
|----------|---------|--------------|---------|
| `dirs::home_dir()` | `$HOME` | `/home/<user>` | `/home/alice` |
| `dirs::config_dir()` | `$XDG_CONFIG_HOME` | `~/.config` | `/home/alice/.config` |
| `dirs::data_dir()` | `$XDG_DATA_HOME` | `~/.local/share` | `/home/alice/.local/share` |
| `dirs::data_local_dir()` | `$XDG_DATA_HOME` | `~/.local/share` | `/home/alice/.local/share` |
| `dirs::cache_dir()` | `$XDG_CACHE_HOME` | `~/.cache` | `/home/alice/.cache` |

### macOS (Standard Directories)

| Function | Path | Example |
|----------|------|---------|
| `dirs::home_dir()` | `$HOME` | `/Users/Alice` |
| `dirs::config_dir()` | `~/Library/Application Support` | `/Users/Alice/Library/Application Support` |
| `dirs::data_dir()` | `~/Library/Application Support` | `/Users/Alice/Library/Application Support` |
| `dirs::data_local_dir()` | `~/Library/Application Support` | `/Users/Alice/Library/Application Support` |
| `dirs::cache_dir()` | `~/Library/Caches` | `/Users/Alice/Library/Caches` |
| `dirs::preference_dir()` | `~/Library/Preferences` | `/Users/Alice/Library/Preferences` |

### Windows (Known Folders)

| Function | CSIDL/FOLDERID | Example |
|----------|----------------|---------|
| `dirs::home_dir()` | `FOLDERID_Profile` | `C:\Users\Alice` |
| `dirs::config_dir()` | `FOLDERID_RoamingAppData` | `C:\Users\Alice\AppData\Roaming` |
| `dirs::data_dir()` | `FOLDERID_RoamingAppData` | `C:\Users\Alice\AppData\Roaming` |
| `dirs::data_local_dir()` | `FOLDERID_LocalAppData` | `C:\Users\Alice\AppData\Local` |
| `dirs::cache_dir()` | `FOLDERID_LocalAppData` | `C:\Users\Alice\AppData\Local\cache` |

### With `ProjectDirs::from("com", "aks", "terraphim")`

| Platform | Config Dir | Data Dir | Cache Dir |
|----------|-----------|----------|-----------|
| Linux | `~/.config/terraphim` | `~/.local/share/terraphim` | `~/.cache/terraphim` |
| macOS | `~/Library/Application Support/com.aks.terraphim` | `~/Library/Application Support/com.aks.terraphim` | `~/Library/Caches/com.aks.terraphim` |
| Windows | `%APPDATA%\terraphim\terraphim\config` | `%LOCALAPPDATA%\terraphim\terraphim\data` | `%LOCALAPPDATA%\terraphim\terraphim\cache` |

> **Note**: The Windows paths from `ProjectDirs` include `\terraphim\terraphim\` because the crate uses both organisation and application name. This is the correct Windows convention.

---

## Appendix B: Relevant Source Files

| File | Lines | Usage |
|------|-------|-------|
| `crates/terraphim_settings/src/lib.rs` | 1, 156-157, 192 | `ProjectDirs::from()` for config and data |
| `crates/terraphim_agent/src/learnings/mod.rs` | 85 | `dirs::data_dir()` for learnings |
| `crates/terraphim_agent/src/repl/handler.rs` | 133, 1733, 1763 | `dirs::home_dir()`, `dirs::data_local_dir()` |
| `crates/terraphim_agent/src/main.rs` | 975 | `dirs::cache_dir()` |
| `crates/terraphim_agent/src/onboarding/validation.rs` | 147, 151 | `dirs::home_dir()` for `~/.terraphim` |
| `crates/terraphim_agent/src/learnings/install.rs` | 43-45 | `dirs::config_dir()` for agent discovery |
| `crates/terraphim_hooks/src/discovery.rs` | 47 | `dirs::home_dir()` for cargo bin |
| `crates/terraphim_router/src/registry.rs` | 426 | `dirs::home_dir()` for `~/.terraphim/providers` |
| `crates/terraphim_sessions/src/connector/native.rs` | 49 | `dirs::home_dir()` for Claude Code sessions |
| `crates/terraphim_tinyclaw/src/commands/mod.rs` | 122 | `dirs::config_dir()` for commands |
| `crates/terraphim_tinyclaw/src/tools/voice_transcribe.rs` | 39 | `dirs::data_local_dir()` |
| `crates/terraphim_update/src/state.rs` | 16 | `dirs::config_dir()` for update history |
| `crates/terraphim_validation/src/testing/desktop_ui/utils.rs` | 203, 209, 214 | `dirs::data_dir()` with platform fallbacks |
| `docs/domain-models/terraphim_config.md` | 227 | Documentation reference |
| `.docs/design-learning-kg-2026-04-17.md` | 168, 237, 251, 494 | Design doc references |

---

*End of Research Document*
