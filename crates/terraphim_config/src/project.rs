//! Project-level configuration discovery for `.terraphim/` directories.
//!
//! Terraphim supports project-level configuration through a `.terraphim/`
//! directory containing a `.terraphim/` subdirectory. The discovered config
//! is layered on top of the global config, with project roles taking precedence.
//!
//! Discovery algorithm:
//! - Start at `cwd` and walk up the directory tree
//! - Stop at the first directory that contains a `.terraphim/` subdirectory
//! - **Closest** `.terraphim/` to `cwd` wins (inner-most takes precedence)
//!
//! ## Directory structure
//! ```text
//! project-root/
//! └── .terraphim/
//!     ├── config.json    # Project config (optional)
//!     ├── roles/         # Split role files (optional)
//!     └── kg/            # Markdown KG sources (optional)
//! ```
//!
//! ## Behaviour
//! - **No `.terraphim/` found**: returns `Ok(None)` — not an error
//! - **`.terraphim/` exists but no `config.json`**: returns empty project
//! - **`config.json` parse error**: returns `ProjectDiscoveryError`

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Config, RoleName};

/// Maximum number of ancestor directories to walk when searching for `.terraphim/`.
const MAX_ANCESTOR_WALK: usize = 64;

/// Value loaded from a discovered `.terraphim/` project directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Absolute path to the `.terraphim/` directory (project root)
    pub root: PathBuf,
    /// Optionally present `.terraphim/roles/` directory with split role files
    pub roles_dir: Option<PathBuf>,
    /// Optionally present `.terraphim/kg/` directory with markdown KG sources
    pub kg_dir: Option<PathBuf>,
    /// Loaded project configuration (may be minimal if no config.json)
    pub config: Config,
}

impl ProjectConfig {
    /// Load a project config from an existing `.terraphim/` directory.
    ///
    /// - If `config.json` exists and is valid → parse it as [`Config`]
    /// - If `config.json` exists but is malformed → error
    /// - If `config.json` is absent → return minimal project with empty roles
    pub fn load(terraphim_dir: &Path) -> Result<Self, ProjectDiscoveryError> {
        let root = terraphim_dir.to_path_buf();

        let roles_dir = root.join("roles");
        let roles_dir = roles_dir.exists().then_some(roles_dir);

        let kg_dir = root.join("kg");
        let kg_dir = kg_dir.exists().then_some(kg_dir);

        let config_path = root.join("config.json");
        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path).map_err(|e| {
                ProjectDiscoveryError::Io(root.join("config.json"), e)
            })?;
            serde_json::from_str::<Config>(&content).map_err(|e| {
                ProjectDiscoveryError::MalformedConfig(root.join("config.json"), e)
            })?
        } else {
            Config::empty()
        };

        Ok(Self {
            root,
            roles_dir,
            kg_dir,
            config,
        })
    }
}

/// Errors that can occur during project config discovery.
#[derive(Error, Debug)]
pub enum ProjectDiscoveryError {
    #[error("malformed .terraphim/config.json at '{0}': {1}")]
    MalformedConfig(PathBuf, serde_json::Error),
    #[error("I/O error accessing '{0}': {1}")]
    Io(PathBuf, std::io::Error),
}

/// Stops at the first directory containing a `.terraphim/` subdirectory.
///
/// Walks from `cwd` up to the filesystem root (capped at 64 levels) and returns
/// the innermost `.terraphim/` found.
///
/// Returns `Ok(None)` if no `.terraphim/` exists in the ancestry chain.
pub fn discover(cwd: &Path) -> Result<Option<ProjectConfig>, ProjectDiscoveryError> {
    let mut ancestor = cwd.to_path_buf();

    for _ in 0..MAX_ANCESTOR_WALK {
        let terraphim_dir = ancestor.join(".terraphim");

        if terraphim_dir.is_dir() {
            let project = ProjectConfig::load(&terraphim_dir)?;
            return Ok(Some(project));
        }

        if !ancestor.pop() {
            break;
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_project(dir: &Path, with_config: bool, with_roles: bool, with_kg: bool) {
        let terraphim = dir.join(".terraphim");
        fs::create_dir_all(&terraphim).unwrap();

        if with_kg {
            fs::create_dir_all(terraphim.join("kg")).unwrap();
            fs::write(
                terraphim.join("kg").join("sample.md"),
                "synonyms:: test\ndescription: A test entry\n",
            )
            .unwrap();
        }

        if with_roles {
            fs::create_dir_all(terraphim.join("roles")).unwrap();
        }

        if with_config {
            let config_json = serde_json::json!({
                "id": "Server",
                "roles": {
                    "testrole": {
                        "name": "testrole",
                        "shortname": "tr",
                        "relevance_function": "title-scorer",
                        "terraphim_it": false,
                        "theme": "default",
                        "haystacks": [],
                        "llm_enabled": false
                    }
                },
                "default_role": "testrole",
                "selected_role": "testrole"
            });
            fs::write(
                terraphim.join("config.json"),
                serde_json::to_string_pretty(&config_json).unwrap(),
            )
            .unwrap();
        }
    }

    #[test]
    fn discover_returns_none_when_no_terraphim_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let result = discover(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn discover_finds_project_at_cwd() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = std::fs::canonicalize(tmp.path()).unwrap();
        create_project(&cwd, true, false, false);
        let result = discover(&cwd).unwrap();
        let project = result.expect("expected project config");
        assert_eq!(project.root, cwd.join(".terraphim"));
        assert!(project
            .config
            .roles
            .contains_key(&crate::RoleName::new("testrole")));
    }

    #[test]
    fn discover_finds_project_n_levels_up() {
        let root = tempfile::tempdir().unwrap();
        let root_canonical = std::fs::canonicalize(root.path()).unwrap();
        create_project(&root_canonical, true, false, false);

        let inner = root_canonical.join("src").join("deep");
        fs::create_dir_all(&inner).unwrap();
        let inner_canonical = std::fs::canonicalize(&inner).unwrap();

        let result = discover(&inner_canonical).unwrap();
        let project = result.expect("expected project config");
        assert_eq!(project.root, root_canonical.join(".terraphim"));
        assert!(project
            .config
            .roles
            .contains_key(&crate::RoleName::new("testrole")));
    }

    #[test]
    fn discover_empty_project_when_no_config_json() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = std::fs::canonicalize(tmp.path()).unwrap();
        create_project(&cwd, false, true, true);
        let result = discover(&cwd).unwrap();
        let project = result.expect("expected project");
        assert!(project.config.roles.is_empty());
        assert!(project.roles_dir.is_some());
        assert!(project.kg_dir.is_some());
    }

    #[test]
    fn discover_with_roles_and_kg_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = std::fs::canonicalize(tmp.path()).unwrap();
        create_project(&cwd, false, true, true);
        let result = discover(&cwd).unwrap();
        let project = result.expect("expected project");
        assert!(project.roles_dir.is_some());
        assert!(project.kg_dir.is_some());
    }

    #[test]
    fn discover_innermost_project_takes_precedence() {
        let root = tempfile::tempdir().unwrap();
        let root_canonical = std::fs::canonicalize(root.path()).unwrap();
        create_project(&root_canonical, true, false, false);

        let inner = root_canonical.join("inner-project");
        fs::create_dir_all(&inner).unwrap();
        let inner_canonical = std::fs::canonicalize(&inner).unwrap();
        create_project(&inner_canonical, true, false, false);

        let result = discover(&inner_canonical).unwrap();
        let project = result.expect("expected project");
        assert_eq!(
            project.root,
            inner_canonical.join(".terraphim"),
            "inner-most .terraphim/ should be found first"
        );
    }

    #[test]
    fn discover_malformed_config_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = std::fs::canonicalize(tmp.path()).unwrap();
        let terraphim = cwd.join(".terraphim");
        fs::create_dir_all(&terraphim).unwrap();
        fs::write(terraphim.join("config.json"), "not valid json {{{").unwrap();

        let result = discover(&cwd);
        assert!(result.is_err());
    }

    #[test]
    fn project_load_empty_config_when_no_config_json() {
        let tmp = tempfile::tempdir().unwrap();
        let terraphim = tmp.path().join(".terraphim");
        fs::create_dir_all(&terraphim).unwrap();

        let project = ProjectConfig::load(&terraphim).unwrap();
        assert!(project.config.roles.is_empty());
        assert!(project.roles_dir.is_none());
        assert!(project.kg_dir.is_none());
    }
}