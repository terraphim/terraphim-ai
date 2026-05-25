use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectDiscoveryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Not a directory: {0}")]
    NotDirectory(PathBuf),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub global_shortcut: Option<String>,
    #[serde(default)]
    pub roles: std::collections::HashMap<String, crate::Role>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            global_shortcut: None,
            roles: std::collections::HashMap::new(),
        }
    }
}

impl ProjectConfig {
    pub fn from_file(path: &Path) -> Result<Self, ProjectDiscoveryError> {
        let content = std::fs::read_to_string(path)?;
        let config: ProjectConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Load a ProjectConfig by scanning a `.terraphim/` directory.
    ///
    /// If `config.json` exists, loads it first (backward compat).
    /// Then scans for `role-*.json` files and merges them in.
    /// Role name is derived from filename: `role-devops.json` -> `"devops"`.
    pub fn load_from_dir(dir: &Path) -> Result<Self, ProjectDiscoveryError> {
        let mut config = Self::default();

        let config_json = dir.join("config.json");
        if config_json.is_file() {
            config = Self::from_file(&config_json)?;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(config),
        };

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("role-") && name.ends_with(".json") {
                let role_name = name
                    .trim_start_matches("role-")
                    .trim_end_matches(".json")
                    .to_string();
                let content = std::fs::read_to_string(entry.path())?;
                let role: crate::Role = serde_json::from_str(&content)?;
                config.roles.insert(role_name, role);
            }
        }

        Ok(config)
    }
}

/// Find the thesaurus file for a given role inside a `.terraphim/` directory.
///
/// Looks for `thesaurus-<role_name>.json`.
pub fn discover_thesaurus(dir: &Path, role_name: &str) -> Option<PathBuf> {
    let filename = format!("thesaurus-{}.json", role_name);
    let path = dir.join(&filename);
    if path.is_file() { Some(path) } else { None }
}

/// Find the KG directory within `.terraphim/`.
///
/// Looks for `.terraphim/kg/` (top-level) or `.terraphim/kg/<role_name>/` (role-specific).
pub fn discover_kg_path(dir: &Path, role_name: Option<&str>) -> Option<PathBuf> {
    if let Some(name) = role_name {
        let role_kg = dir.join("kg").join(name);
        if role_kg.is_dir() {
            return Some(role_kg);
        }
    }
    let kg_dir = dir.join("kg");
    if kg_dir.is_dir() { Some(kg_dir) } else { None }
}

pub fn discover(start_dir: Option<&Path>) -> Result<Option<PathBuf>, ProjectDiscoveryError> {
    let start_dir = match start_dir {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir()?,
    };

    let mut current = Some(start_dir);

    while let Some(dir) = current {
        let terraphim_dir = dir.join(".terraphim");
        if terraphim_dir.is_dir() {
            let canonical = terraphim_dir.canonicalize()?;
            return Ok(Some(canonical));
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_dir_with_structure(base: &TempDir, structure: &[&str]) -> PathBuf {
        let base_path = base.path().to_path_buf();
        for path in structure {
            let full_path = base_path.join(path);
            if path.ends_with('/') {
                fs::create_dir_all(&full_path).unwrap();
            } else {
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(&full_path, "{}").unwrap();
            }
        }
        base_path
    }

    #[test]
    fn test_discover_finds_terraphim_dir() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(&temp, &["work/", "work/.terraphim/", "work/src/"]);
        let result = discover(Some(&base.join("work/src"))).unwrap();
        let expected = std::fs::canonicalize(base.join("work/.terraphim")).unwrap();
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_discover_not_found() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(&temp, &["src/", "src/main.rs"]);
        let result = discover(Some(&base.join("src"))).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_discover_from_current_dir() {
        let original_dir = std::env::current_dir().unwrap();
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(&temp, &[".terraphim/"]);
        std::env::set_current_dir(&base).unwrap();
        let result = discover(None).unwrap();
        let expected = std::fs::canonicalize(base.join(".terraphim")).unwrap();
        assert_eq!(result, Some(expected));
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_discover_upwards_search() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(
            &temp,
            &["project/", "project/.terraphim/", "project/src/main.rs"],
        );
        let result = discover(Some(&base.join("project/src"))).unwrap();
        let expected = std::fs::canonicalize(base.join("project/.terraphim")).unwrap();
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_discover_multiple_levels_up() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(
            &temp,
            &[
                "a/",
                "a/b/",
                "a/b/c/",
                "a/b/c/.terraphim/",
                "a/b/c/src/main.rs",
            ],
        );
        let result = discover(Some(&base.join("a/b/c/src"))).unwrap();
        let expected = std::fs::canonicalize(base.join("a/b/c/.terraphim")).unwrap();
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_project_config_from_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.json");
        let json = r#"{"global_shortcut": "Ctrl+Shift+T", "roles": {}}"#;
        fs::write(&config_path, json).unwrap();
        let config = ProjectConfig::from_file(&config_path).unwrap();
        assert_eq!(config.global_shortcut, Some("Ctrl+Shift+T".to_string()));
    }

    #[test]
    fn test_project_config_from_file_empty() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();
        let config = ProjectConfig::from_file(&config_path).unwrap();
        assert_eq!(config.global_shortcut, None);
        assert!(config.roles.is_empty());
    }

    #[test]
    fn test_discover_returns_none_for_missing() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(&temp, &["src/", "src/main.rs"]);
        let result = discover(Some(&base.join("src"))).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_discover_root_finds_terraphim() {
        let temp = TempDir::new().unwrap();
        let base = temp_dir_with_structure(&temp, &[".terraphim/"]);
        let result = discover(Some(&base)).unwrap();
        let expected = std::fs::canonicalize(base.join(".terraphim")).unwrap();
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_discover_symlink_to_real_dir() {
        let temp = TempDir::new().unwrap();
        let real = temp.path().join("real");
        let linked = temp.path().join("linked");
        fs::create_dir_all(real.join(".terraphim")).unwrap();
        fs::create_dir_all(real.join("src")).unwrap();
        std::os::unix::fs::symlink(&real, &linked).unwrap();
        let canonical = std::fs::canonicalize(real.join(".terraphim")).unwrap();
        let result = discover(Some(&linked.join("src"))).unwrap();
        assert_eq!(result, Some(canonical));
    }

    fn minimal_role_json(name: &str) -> String {
        format!(
            r#"{{"shortname":"{}","name":"{}","relevance_function":"title-scorer","terraphim_it":false,"theme":"default","haystacks":[]}}"#,
            name, name
        )
    }

    #[test]
    fn test_load_from_dir_reads_role_files() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".terraphim");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("role-devops.json"), minimal_role_json("DevOps")).unwrap();
        fs::write(
            dir.join("role-rust-engineer.json"),
            minimal_role_json("Rust Engineer"),
        )
        .unwrap();

        let config = ProjectConfig::load_from_dir(&dir).unwrap();
        assert_eq!(config.roles.len(), 2);
        assert!(config.roles.contains_key("devops"));
        assert!(config.roles.contains_key("rust-engineer"));
    }

    #[test]
    fn test_load_from_dir_merges_with_config_json() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".terraphim");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("config.json"),
            r#"{"global_shortcut":"Ctrl+T","roles":{}}"#,
        )
        .unwrap();
        fs::write(dir.join("role-devops.json"), minimal_role_json("DevOps")).unwrap();

        let config = ProjectConfig::load_from_dir(&dir).unwrap();
        assert_eq!(config.global_shortcut, Some("Ctrl+T".to_string()));
        assert_eq!(config.roles.len(), 1);
        assert!(config.roles.contains_key("devops"));
    }

    #[test]
    fn test_load_from_dir_empty_is_ok() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".terraphim");
        fs::create_dir_all(&dir).unwrap();

        let config = ProjectConfig::load_from_dir(&dir).unwrap();
        assert!(config.roles.is_empty());
    }

    #[test]
    fn test_discover_thesaurus_found() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".terraphim");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("thesaurus-devops.json"), "{}").unwrap();

        let result = discover_thesaurus(&dir, "devops");
        assert_eq!(result, Some(dir.join("thesaurus-devops.json")));
    }

    #[test]
    fn test_discover_thesaurus_not_found() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".terraphim");
        fs::create_dir_all(&dir).unwrap();

        let result = discover_thesaurus(&dir, "devops");
        assert!(result.is_none());
    }

    #[test]
    fn test_discover_kg_path_found() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".terraphim");
        fs::create_dir_all(dir.join("kg")).unwrap();

        let result = discover_kg_path(&dir, None);
        assert_eq!(result, Some(dir.join("kg")));
    }

    #[test]
    fn test_discover_kg_path_role_specific() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".terraphim");
        fs::create_dir_all(dir.join("kg").join("devops")).unwrap();

        let result = discover_kg_path(&dir, Some("devops"));
        assert_eq!(result, Some(dir.join("kg").join("devops")));
    }

    #[test]
    fn test_discover_kg_path_not_found() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".terraphim");
        fs::create_dir_all(&dir).unwrap();

        let result = discover_kg_path(&dir, None);
        assert!(result.is_none());
    }
}
